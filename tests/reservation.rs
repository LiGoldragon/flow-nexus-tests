#[path = "fixtures/identity.rs"]
mod fixture;
use fixture::{FixtureFactory, IdentityFixture};
use flow_nexus_runtime::{
    memory_actor::{ActorTransportFailure, MemoryAccessible, MemoryCommand, MemoryPort, MemoryReply,
        MemoryRestartable, OrderedResolvable, RecoveredResolvable, ReservationControlled,
        TargetedReservation, TrackedSubmission},
    reservation::{Reservable, ReservationResolvable},
    store::{MemoryStore, Openable, Readable, StoreLayout, TableName, Writable},
};
use flow_nexus_types::{
    evidence::{Invariant, Mutation, MutationBatch, OriginAdmissionId, ReadSelector, Record,
        RecordKey, RequestId, ReservationOutcome, ReservationResolution, ReserveLaunch},
    memory::{Flow, Permaflow, RequestAllocator},
    FlowId, State,
};
use memory::{Schema, SchemaVersion};
use std::path::PathBuf;

struct ReservationFixture { identity: IdentityFixture, path: PathBuf }
trait ReservationFixtures: Sized {
    fn named(name: &str) -> Self;
    fn store(&self) -> MemoryStore;
    fn seeded(&self, next: i64) -> MemoryStore;
    fn request(&self, id: i64, flow_id: &str) -> ReserveLaunch;
}
impl ReservationFixtures for ReservationFixture {
    fn named(name: &str) -> Self {
        Self { identity: IdentityFixture::topic(name), path: std::env::temp_dir().join(
            format!("flow-v1-reservation-{name}-{}.redb", std::process::id())) }
    }
    fn store(&self) -> MemoryStore {
        let _ = std::fs::remove_file(&self.path);
        MemoryStore::open(StoreLayout { path: self.path.clone(), table_name: TableName("rows"),
            schema: Schema { version: SchemaVersion::new(1) } }).unwrap()
    }
    fn seeded(&self, next: i64) -> MemoryStore {
        let mut store = self.store();
        store.mutate(&MutationBatch(vec![Mutation::Put(Record::RequestAllocator(
            RequestAllocator { request_id: RequestId(next) }))])).unwrap();
        store
    }
    fn request(&self, id: i64, flow_id: &str) -> ReserveLaunch {
        let attempt = self.identity.launch(id, flow_id, self.identity.address.clone());
        ReserveLaunch { request_id: RequestId(id), flow_id: FlowId(flow_id.into()),
            resolved_launch: attempt.resolved_launch, creation_intent: attempt.creation_intent,
            origin_admission_id_option: None }
    }
}

#[test]
fn actual_store_reserves_all_three_rows_and_matches_only_exact_payload() {
    let fixture = ReservationFixture::named("atomic");
    let mut store = fixture.seeded(0);
    let request = fixture.request(0, "atomaa");
    assert_eq!(store.reserve(&request).unwrap(), ReservationOutcome::Reserved(RequestId(0)));
    let state = store.snapshot().unwrap();
    assert_eq!(state.records.len(), 3);
    assert!(state.records.iter().any(|row| matches!(row, Record::RequestAllocator(v) if v.request_id == RequestId(1))));
    assert!(state.records.iter().any(|row| matches!(row, Record::LaunchAttempt(v) if v.request_id == RequestId(0) && v.flow_id == request.flow_id)));
    assert!(state.records.iter().any(|row| matches!(row, Record::AddressReservation(v) if v.request_id == RequestId(0) && v.flow_id == request.flow_id)));
    assert!(matches!(store.resolve(&request).unwrap(), ReservationResolution::Matched(_)));
    let mut other = request.clone();
    other.resolved_launch.prompt_content.first_prompt.0 = "different intended payload".into();
    assert!(matches!(store.resolve(&other).unwrap(), ReservationResolution::Conflicting(Record::LaunchAttempt(_))));
    assert_eq!(store.reserve(&request).unwrap(), ReservationOutcome::CounterAdvanced(RequestId(1)));
    assert_eq!(store.snapshot().unwrap(), state);
}

#[test]
fn missing_allocator_and_overflow_are_definite_no_commit() {
    let fixture = ReservationFixture::named("missing");
    let mut store = fixture.store();
    let before = store.snapshot().unwrap();
    assert_eq!(store.reserve(&fixture.request(0, "missaa")).unwrap(), ReservationOutcome::AllocatorUnavailable);
    assert_eq!(store.snapshot().unwrap(), before);
    drop(store);
    let fixture = ReservationFixture::named("overflow");
    let mut store = fixture.seeded(i64::MAX);
    let before = store.snapshot().unwrap();
    assert_eq!(store.reserve(&fixture.request(i64::MAX, "overaa")).unwrap(), ReservationOutcome::CounterOverflow);
    assert_eq!(store.snapshot().unwrap(), before);
}

#[test]
fn invalid_origin_rolls_back_allocator_attempt_and_reservation() {
    let fixture = ReservationFixture::named("rollback");
    let mut store = fixture.seeded(0);
    let before = store.snapshot().unwrap();
    let mut request = fixture.request(0, "rollaa");
    let origin = OriginAdmissionId(RequestId(9));
    request.origin_admission_id_option = Some(origin.clone());
    assert_eq!(store.reserve(&request).unwrap(), ReservationOutcome::Refused(Invariant::DanglingAdmission(origin)));
    assert_eq!(store.snapshot().unwrap(), before);
}

#[test]
fn definite_collision_allows_another_candidate_without_consuming_counter() {
    let fixture = ReservationFixture::named("collision");
    let mut store = fixture.seeded(0);
    let id = FlowId("oldaaa".into());
    store.mutate(&MutationBatch(vec![
        Mutation::Put(Record::Flow(Flow { flow_id: id.clone(), address: fixture.identity.address.clone(),
            session_id_option: None, state: State::Ended, flow_id_option: None, transcript_path_option: None })),
        Mutation::Put(Record::Permaflow(Permaflow { address: fixture.identity.address.clone(), flow_id_vector: vec![id.clone()] })),
    ])).unwrap();
    let before = store.snapshot().unwrap();
    assert_eq!(store.reserve(&fixture.request(0, "oldaaa")).unwrap(), ReservationOutcome::IdentityCollision(id));
    assert_eq!(store.snapshot().unwrap(), before);
    assert_eq!(store.reserve(&fixture.request(0, "newaaa")).unwrap(), ReservationOutcome::Reserved(RequestId(0)));
    // Candidate replacement is tested; no OS entropy/minting behavior is claimed here.
}

#[tokio::test]
async fn serialized_same_address_contenders_have_one_winner_and_no_counter_reuse() {
    let fixture = ReservationFixture::named("contenders");
    let port = MemoryPort::from(fixture.seeded(0));
    let first = port.prepare(fixture.request(0, "firsta"));
    let second = port.prepare(fixture.request(0, "second"));
    let (a, b) = tokio::join!(port.submit(first), port.submit(second));
    let (a, b) = tokio::join!(a.unwrap().receive(), b.unwrap().receive());
    let replies = [a.unwrap(), b.unwrap()];
    assert_eq!(replies.iter().filter(|v| matches!(v, MemoryReply::Reservation(ReservationOutcome::Reserved(RequestId(0))))).count(), 1);
    assert_eq!(replies.iter().filter(|v| matches!(v, MemoryReply::Reservation(ReservationOutcome::CounterAdvanced(RequestId(1))))).count(), 1);
    let retry = port.submit(port.prepare(fixture.request(1, "thirda"))).await.unwrap().receive().await.unwrap();
    assert!(matches!(retry, MemoryReply::Reservation(ReservationOutcome::AddressOccupied(_))));
    let reply = port.request(MemoryCommand::Read(ReadSelector::ByKey(RecordKey::RequestAllocatorSingleton))).await.unwrap();
    assert!(matches!(reply, MemoryReply::Records(v) if v == vec![Record::RequestAllocator(RequestAllocator { request_id: RequestId(1) })]));
}

#[tokio::test]
async fn delayed_original_keeps_absence_pending_until_ordered_processing() {
    let fixture = ReservationFixture::named("delayed");
    let port = MemoryPort::from(fixture.seeded(0));
    let prepared = port.prepare(fixture.request(0, "delayx"));
    let target = prepared.target();
    // This original dispatch still exists and can execute later; it has not been polled/enqueued.
    let original = port.submit(prepared);
    assert!(matches!(port.probe(&target).await.unwrap(), MemoryReply::Resolution(ReservationResolution::Pending)));
    let ticket = original.await.unwrap().discard();
    assert!(matches!(ticket.resolve().await.unwrap(), MemoryReply::Resolution(ReservationResolution::Matched(_))));
    assert!(matches!(port.probe(&target).await.unwrap(), MemoryReply::Resolution(ReservationResolution::Matched(_))));
}

#[tokio::test]
async fn discarded_rejection_reply_has_final_absence_only_after_its_barrier() {
    let fixture = ReservationFixture::named("beforeCommit");
    let port = MemoryPort::from(fixture.seeded(0));
    let mut request = fixture.request(0, "reject");
    request.origin_admission_id_option = Some(OriginAdmissionId(RequestId(9)));
    let prepared = port.prepare(request);
    let target = prepared.target();
    assert!(matches!(port.probe(&target).await.unwrap(), MemoryReply::Resolution(ReservationResolution::Pending)));
    let ticket = port.submit(prepared).await.unwrap().discard();
    assert!(matches!(ticket.resolve().await.unwrap(), MemoryReply::Resolution(ReservationResolution::AbsentFinal)));
    // Plain reads still cannot mint finality, even after the original happened to finish.
    assert!(matches!(port.probe(&target).await.unwrap(), MemoryReply::Resolution(ReservationResolution::Pending)));
}

#[tokio::test]
async fn discarded_commit_reply_recovers_after_actor_restart_with_persistent_allocator() {
    let fixture = ReservationFixture::named("afterCommit");
    let port = MemoryPort::from(fixture.seeded(0));
    let request = fixture.request(0, "commix");
    let prepared = port.prepare(request.clone());
    let target = prepared.target();
    let ticket = port.submit(prepared).await.unwrap().discard();
    assert!(matches!(ticket.resolve().await.unwrap(), MemoryReply::Resolution(ReservationResolution::Matched(_))));
    let recovered = port.restart().await.unwrap();
    assert!(matches!(recovered.resolve(&target).await.unwrap(), MemoryReply::Resolution(ReservationResolution::Matched(_))));
    let reply = recovered.as_ref().request(MemoryCommand::Read(ReadSelector::ByKey(RecordKey::RequestAllocatorSingleton))).await.unwrap();
    assert!(matches!(reply, MemoryReply::Records(v) if v == vec![Record::RequestAllocator(RequestAllocator { request_id: RequestId(1) })]));
    let wrong_actor_target = recovered.as_ref().prepare(request).target();
    assert!(matches!(recovered.resolve(&wrong_actor_target).await, Err(ActorTransportFailure::WrongActor)));
}

#[tokio::test]
async fn restart_proves_delayed_old_dispatch_cannot_commit_after_final_absence() {
    let fixture = ReservationFixture::named("restartAbsent");
    let port = MemoryPort::from(fixture.seeded(0));
    let request = fixture.request(0, "absent");
    let prepared = port.prepare(request.clone());
    let target = prepared.target();
    assert!(matches!(port.probe(&target).await.unwrap(), MemoryReply::Resolution(ReservationResolution::Pending)));
    let recovered = port.restart().await.unwrap();
    assert!(matches!(recovered.resolve(&target).await.unwrap(), MemoryReply::Resolution(ReservationResolution::AbsentFinal)));
    assert!(matches!(port.submit(prepared).await, Err(ActorTransportFailure::MutationUncertain)));
    assert!(matches!(recovered.resolve(&target).await.unwrap(), MemoryReply::Resolution(ReservationResolution::AbsentFinal)));
    let old_prepared = port.prepare(request);
    assert!(matches!(recovered.as_ref().submit(old_prepared).await, Err(ActorTransportFailure::WrongActor)));
}

#[test]
fn actual_memory_kernel_refuses_a_second_writer_for_the_same_store() {
    let fixture = ReservationFixture::named("exclusive");
    let _first_owner = fixture.seeded(0);
    let second = MemoryStore::open(StoreLayout { path: fixture.path.clone(), table_name: TableName("rows"),
        schema: Schema { version: SchemaVersion::new(1) } });
    assert!(second.is_err(), "same Memory store must not acquire another independent writer");
}
