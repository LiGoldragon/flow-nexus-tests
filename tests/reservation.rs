#[path = "fixtures/identity.rs"]
mod fixture;
use fixture::{FixtureFactory, IdentityFixture};
use flow_nexus_runtime::{
    bootstrap::Initializable,
    memory_actor::{ActorTransportFailure, MemoryAccessible, MemoryCommand, MemoryPort, MemoryReply,
        MemoryInterruptible, MemoryRestartable, OrderedResolvable, RecoveredResolvable, ReservationControlled,
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
    fn configuration(&self) -> flow_nexus_types::memory::ConfigurationRecord;
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
        assert_eq!(store.initialize(&self.configuration()).unwrap(),
            flow_nexus_types::evidence::InitializationOutcome::Initialized);
        if next != 0 {
            store.mutate(&MutationBatch(vec![Mutation::Put(Record::RequestAllocator(
                RequestAllocator { request_id: RequestId(next) }))])).unwrap();
        }
        store
    }
    fn request(&self, id: i64, flow_id: &str) -> ReserveLaunch {
        let attempt = self.identity.launch(id, flow_id, self.identity.address.clone());
        ReserveLaunch { request_id: RequestId(id), flow_id: FlowId(flow_id.into()),
            resolved_launch: attempt.resolved_launch, creation_intent: attempt.creation_intent,
            origin_admission_id_option: None }
    }
    fn configuration(&self) -> flow_nexus_types::memory::ConfigurationRecord {
        let snapshot = self.identity.resolved(self.identity.address.clone());
        let assignment = flow_nexus_types::LayerAssignment {
            aspect: self.identity.address.aspect.clone(), layer: self.identity.address.layer.clone(),
            model_profile_name: snapshot.model_profile.model_profile_name.clone(),
        };
        flow_nexus_types::memory::ConfigurationRecord {
            configuration: flow_nexus_types::Configuration {
                metaflow_name: flow_nexus_types::MetaflowName("test".into()),
                memory_path: flow_nexus_types::MemoryPath(self.path.to_str().unwrap().into()),
                socket_configuration: flow_nexus_types::SocketConfiguration {
                    ordinary_socket_path: flow_nexus_types::OrdinarySocketPath("/ordinary".into()),
                    meta_socket_path: flow_nexus_types::MetaSocketPath("/meta".into()),
                },
                declaration_vector: vec![snapshot.declaration], layer_assignment_vector: vec![assignment],
                model_profile_vector: vec![snapshot.model_profile], harness_profile_vector: vec![snapshot.harness_profile],
                module_source_vector: vec![], identity_policy: snapshot.identity_policy,
                capsule_configuration: snapshot.capsule_configuration,
                security_configuration: flow_nexus_types::SecurityConfiguration { owner_uid: flow_nexus_types::OwnerUid(0) },
            },
            meta_configure_occurred: flow_nexus_types::evidence::MetaConfigureOccurred(false),
        }
    }

}

#[test]
fn actual_store_reserves_all_three_rows_and_matches_only_exact_payload() {
    let fixture = ReservationFixture::named("atomic");
    let mut store = fixture.seeded(0);
    let request = fixture.request(0, "atomaa");
    assert_eq!(store.reserve(&request).unwrap(), ReservationOutcome::Reserved(RequestId(0)));
    let state = store.snapshot().unwrap();
    assert_eq!(state.records.len(), 4);
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
    assert!(matches!(port.submit(prepared).await, Err(ActorTransportFailure::NotAdmitted)));
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

#[tokio::test]
async fn cancelling_receive_future_preserves_committed_memory_and_actor_liveness() {
    let fixture = ReservationFixture::named("cancelReceiver");
    let port = MemoryPort::from(fixture.seeded(0));
    let submission = port.submit(port.prepare(fixture.request(0, "cancel"))).await.unwrap();
    let ticket = submission.ticket();
    let receive = submission.receive();
    drop(receive);
    assert!(matches!(ticket.resolve().await.unwrap(), MemoryReply::Resolution(ReservationResolution::Matched(_))));
    assert!(matches!(port.request(MemoryCommand::Read(ReadSelector::All)).await.unwrap(), MemoryReply::Records(rows) if rows.len() == 4));
}

#[tokio::test]
async fn cancelling_before_enqueue_stays_pending_until_old_actor_is_retired() {
    let fixture = ReservationFixture::named("cancelBeforeEnqueue");
    let port = MemoryPort::from(fixture.seeded(0));
    let prepared = port.prepare(fixture.request(0, "before"));
    let target = prepared.target();
    let enqueue = port.submit(prepared);
    drop(enqueue);
    assert!(matches!(port.probe(&target).await.unwrap(), MemoryReply::Resolution(ReservationResolution::Pending)));
    let recovered = port.restart().await.unwrap();
    assert!(matches!(recovered.resolve(&target).await.unwrap(), MemoryReply::Resolution(ReservationResolution::AbsentFinal)));
}

#[tokio::test]
async fn replacing_file_at_same_path_cannot_mint_a_recovered_capability() {
    let fixture = ReservationFixture::named("replacedStore");
    let port = MemoryPort::from(fixture.seeded(0));
    let original = fixture.path.with_extension("original.redb");
    std::fs::rename(&fixture.path, &original).unwrap();
    let mut replacement = MemoryStore::open(StoreLayout { path: fixture.path.clone(), table_name: TableName("rows"),
        schema: Schema { version: SchemaVersion::new(1) } }).unwrap();
    replacement.mutate(&MutationBatch(vec![Mutation::Put(Record::RequestAllocator(RequestAllocator { request_id: RequestId(99) }))])).unwrap();
    drop(replacement);
    assert!(matches!(port.restart().await, Err(flow_nexus_runtime::store::StoreFailure::IdentityChanged)));
    let replacement = MemoryStore::open(StoreLayout { path: fixture.path.clone(), table_name: TableName("rows"),
        schema: Schema { version: SchemaVersion::new(1) } }).unwrap();
    assert_eq!(replacement.snapshot().unwrap().records, vec![Record::RequestAllocator(RequestAllocator { request_id: RequestId(99) })]);
    let original_store = MemoryStore::open(StoreLayout { path: original, table_name: TableName("rows"),
        schema: Schema { version: SchemaVersion::new(1) } }).unwrap();
    assert!(original_store.snapshot().unwrap().records.contains(&Record::RequestAllocator(RequestAllocator { request_id: RequestId(0) })));
}

#[tokio::test(flavor = "current_thread")]
async fn cancellation_while_mailbox_is_full_has_no_ticket_or_final_absence() {
    use std::{future::Future, task::{Context, Poll, Waker}};
    let fixture = ReservationFixture::named("cancelDuringEnqueue");
    let port = MemoryPort::from(fixture.seeded(0));
    // Pinned Kameo0.22.2 default bounded capacity is64. On this current-thread executor,
    // manually polling readonly asks fills it without letting the actor run.
    let mut context = Context::from_waker(Waker::noop());
    let mut reads = Vec::new();
    for _ in 0..64 {
        let mut read = Box::pin(port.request(MemoryCommand::Read(ReadSelector::All)));
        assert!(matches!(read.as_mut().poll(&mut context), Poll::Pending));
        reads.push(read);
    }
    let prepared = port.prepare(fixture.request(0, "midway"));
    let target = prepared.target();
    let mut sending = Box::pin(port.submit(prepared));
    // submit's only await before ticket construction is mailbox send.
    assert!(matches!(sending.as_mut().poll(&mut context), Poll::Pending));
    drop(sending);
    drop(reads);
    assert!(matches!(port.probe(&target).await.unwrap(), MemoryReply::Resolution(ReservationResolution::Pending)));
    let recovered = port.restart().await.unwrap();
    assert!(matches!(recovered.resolve(&target).await.unwrap(), MemoryReply::Resolution(ReservationResolution::AbsentFinal)));
}

#[tokio::test(flavor = "current_thread")]
async fn actor_death_before_processing_reply_remains_uncertain_until_recovery() {
    use std::{future::Future, task::{Context, Poll, Waker}};
    let fixture = ReservationFixture::named("deathBeforeReply");
    let port = MemoryPort::from(fixture.seeded(0));
    let prepared = port.prepare(fixture.request(0, "deathx"));
    let target = prepared.target();
    let mut context = Context::from_waker(Waker::noop());
    let mut sending = Box::pin(port.submit(prepared));
    let submission = match sending.as_mut().poll(&mut context) {
        Poll::Ready(Ok(submission)) => submission,
        _ => panic!("empty mailbox must admit before actor execution on this current-thread executor"),
    };
    drop(sending);
    // Spawn does not synchronously poll actor tasks; interrupt marks its abort before yielding.
    port.interrupt().await;
    assert!(matches!(submission.receive().await, Err(ActorTransportFailure::MutationUncertain)));
    let recovered = port.restart().await.unwrap();
    assert!(matches!(recovered.resolve(&target).await.unwrap(), MemoryReply::Resolution(ReservationResolution::AbsentFinal)));
}

#[test]
fn bootstrap_commits_configuration_and_allocator_together_and_never_resets_existing_state() {
    use flow_nexus_types::evidence::InitializationOutcome;
    let fixture = ReservationFixture::named("bootstrap");
    let mut store = fixture.store();
    let configuration = fixture.configuration();
    assert_eq!(store.initialize(&configuration).unwrap(), InitializationOutcome::Initialized);
    assert_eq!(store.snapshot().unwrap().records.len(), 2);
    assert_eq!(store.reserve(&fixture.request(0, "bootaa")).unwrap(), ReservationOutcome::Reserved(RequestId(0)));
    let before = store.snapshot().unwrap();
    assert_eq!(store.initialize(&configuration).unwrap(), InitializationOutcome::ExistingState);
    assert_eq!(store.snapshot().unwrap(), before);
}

#[test]
fn invalid_initial_configuration_cannot_leave_an_allocator_behind() {
    use flow_nexus_types::evidence::InitializationOutcome;
    let fixture = ReservationFixture::named("invalidBootstrap");
    let mut store = fixture.store();
    let mut configuration = fixture.configuration();
    configuration.configuration.identity_policy.identity_alphabet.0 = "aa".into();
    assert_eq!(store.initialize(&configuration).unwrap(), InitializationOutcome::Refused(Invariant::InvalidAlphabet));
    assert!(store.snapshot().unwrap().records.is_empty());
}

#[test]
fn seeded_counter_alone_cannot_admit_and_stale_configuration_snapshot_cannot_reserve() {
    let fixture = ReservationFixture::named("configurationRequired");
    let mut store = fixture.store();
    store.mutate(&MutationBatch(vec![Mutation::Put(Record::RequestAllocator(RequestAllocator { request_id: RequestId(0) }))])).unwrap();
    let before = store.snapshot().unwrap();
    assert_eq!(store.reserve(&fixture.request(0, "cfgreq")).unwrap(), ReservationOutcome::ConfigurationUnavailable);
    assert_eq!(store.snapshot().unwrap(), before);
    drop(store);
    let fixture = ReservationFixture::named("staleSnapshot");
    let mut store = fixture.seeded(0);
    let before = store.snapshot().unwrap();
    let mut request = fixture.request(0, "stalea");
    request.resolved_launch.model_profile.tokens = flow_nexus_types::Tokens(99);
    assert_eq!(store.reserve(&request).unwrap(), ReservationOutcome::Refused(Invariant::SnapshotMismatch(RecordKey::LaunchAttempt(RequestId(0)))));
    assert_eq!(store.snapshot().unwrap(), before);
}

#[tokio::test]
async fn actual_actor_bootstrap_is_one_typed_transition_before_reservation() {
    let fixture = ReservationFixture::named("actorBootstrap");
    let port = MemoryPort::from(fixture.store());
    assert!(matches!(port.request(MemoryCommand::InitializeConfiguration(fixture.configuration())).await.unwrap(),
        MemoryReply::Initialization(flow_nexus_types::evidence::InitializationOutcome::Initialized)));
    let reply = port.submit(port.prepare(fixture.request(0, "bootac"))).await.unwrap().receive().await.unwrap();
    assert!(matches!(reply, MemoryReply::Reservation(ReservationOutcome::Reserved(RequestId(0)))));
}
