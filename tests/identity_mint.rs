use flow_nexus_runtime::identity::{
    Alphabetic, CancellationObservable, CandidateMintable, EntropyFailure, EntropyReadable,
    Exhaustible, IdentityCountable, MintContext, MintFailure,
};
use flow_nexus_runtime::state::RecordState;
use flow_nexus_types::evidence::Record;
use flow_nexus_types::memory::{Flow, Permaflow};
use flow_nexus_types::{FlowId, IdentityAlphabet, IdentityPolicy, State};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

#[path = "fixtures/identity.rs"]
mod fixture;
use fixture::FixtureFactory;

struct EntropyScript {
    bytes: Vec<Result<u8, EntropyFailure>>,
    index: usize,
    width: usize,
    cancel_after: Option<(Arc<AtomicBool>, usize)>,
}
trait EntropyScripted {
    fn scripted(bytes: Vec<Result<u8, EntropyFailure>>) -> Self;
    fn width(self, width: usize) -> Self;
    fn cancel_after(self, flag: Arc<AtomicBool>, reads: usize) -> Self;
}
impl EntropyScripted for EntropyScript {
    fn scripted(bytes: Vec<Result<u8, EntropyFailure>>) -> Self { Self { bytes, index: 0, width: 1, cancel_after: None } }
    fn width(mut self, width: usize) -> Self { self.width = width; self }
    fn cancel_after(mut self, flag: Arc<AtomicBool>, reads: usize) -> Self { self.cancel_after = Some((flag, reads)); self }
}
impl EntropyReadable for EntropyScript {
    fn read_entropy(&mut self, target: &mut [u8]) -> Result<usize, EntropyFailure> {
        if self.width != target.len() { return Ok(self.width); }
        let value = self.bytes.get(self.index).cloned().unwrap_or(Ok(0));
        self.index += 1;
        if let Some((flag, reads)) = &self.cancel_after { if self.index >= *reads { flag.store(true, Ordering::SeqCst); } }
        match value { Ok(byte) => { target[0] = byte; Ok(1) }, Err(error) => Err(error) }
    }
}

struct CancellationScript { flag: Arc<AtomicBool> }
impl CancellationObservable for CancellationScript { fn is_cancelled(&self) -> bool { self.flag.load(Ordering::SeqCst) } }

trait PolicyFixture { fn policy(alphabet: &str) -> Self; }
impl PolicyFixture for IdentityPolicy {
    fn policy(alphabet: &str) -> Self { Self { identity_alphabet: IdentityAlphabet(alphabet.to_owned()) } }
}

#[tokio::test]
async fn alphabet_requires_two_or_more_unique_ascii_alphanumeric_symbols() {
    assert!(IdentityPolicy::policy("a").alphabet().is_err());
    assert!(IdentityPolicy::policy("aa").alphabet().is_err());
    assert!(IdentityPolicy::policy("a_").alphabet().is_err());
    assert!(IdentityPolicy::policy("aB3").alphabet().is_ok());
    assert!(IdentityPolicy::policy("aé").alphabet().is_err());
}

#[tokio::test]
async fn six_selection_preserves_order_and_case() {
    let policy = IdentityPolicy::policy("aB3");
    let flag = Arc::new(AtomicBool::new(false));
    let cancellation = CancellationScript { flag };
    let mut entropy = EntropyScript::scripted(vec![Ok(0), Ok(1), Ok(2), Ok(0), Ok(1), Ok(2)]);
    let value = MintContext { policy: &policy, entropy: &mut entropy, cancellation: &cancellation }.mint().await.unwrap();
    assert_eq!(value, FlowId("aB3aB3".to_owned()));
}

#[tokio::test]
async fn rejection_interruption_and_read_shapes_are_typed() {
    let policy = IdentityPolicy::policy("abc");
    let flag = Arc::new(AtomicBool::new(false));
    let cancellation = CancellationScript { flag: flag.clone() };
    let mut rejected = EntropyScript::scripted(vec![Ok(255), Ok(0), Ok(1), Ok(2), Ok(0), Ok(1), Ok(2)]);
    assert_eq!(MintContext { policy: &policy, entropy: &mut rejected, cancellation: &cancellation }.mint().await.unwrap(), FlowId("abcabc".to_owned()));
    let mut interrupted = EntropyScript::scripted(vec![Err(EntropyFailure::Interrupted), Ok(0), Ok(1), Ok(2), Ok(0), Ok(1), Ok(2)]);
    assert!(MintContext { policy: &policy, entropy: &mut interrupted, cancellation: &cancellation }.mint().await.is_ok());
    let mut zero = EntropyScript::scripted(vec![]).width(0);
    assert_eq!(MintContext { policy: &policy, entropy: &mut zero, cancellation: &cancellation }.mint().await, Err(MintFailure::EmptyRead));
    let mut oversized = EntropyScript::scripted(vec![Ok(0)]).width(2);
    assert_eq!(MintContext { policy: &policy, entropy: &mut oversized, cancellation: &cancellation }.mint().await, Err(MintFailure::InvalidReadLength));
}

#[tokio::test]
async fn entropy_failures_and_cancellation_are_typed() {
    let policy = IdentityPolicy::policy("abc");
    let flag = Arc::new(AtomicBool::new(false));
    let cancellation = CancellationScript { flag: flag.clone() };
    let mut unavailable = EntropyScript::scripted(vec![Err(EntropyFailure::Unavailable)]);
    assert_eq!(MintContext { policy: &policy, entropy: &mut unavailable, cancellation: &cancellation }.mint().await, Err(MintFailure::Entropy(EntropyFailure::Unavailable)));
    flag.store(true, Ordering::SeqCst);
    let mut before = EntropyScript::scripted(vec![]);
    assert_eq!(MintContext { policy: &policy, entropy: &mut before, cancellation: &cancellation }.mint().await, Err(MintFailure::Cancelled));
    flag.store(false, Ordering::SeqCst);
    let during_flag = Arc::new(AtomicBool::new(false));
    let during = CancellationScript { flag: during_flag.clone() };
    let mut during_entropy = EntropyScript::scripted(vec![Ok(0), Ok(1), Ok(2)]).cancel_after(during_flag, 1);
    assert_eq!(MintContext { policy: &policy, entropy: &mut during_entropy, cancellation: &during }.mint().await, Err(MintFailure::Cancelled));
}

#[test]
fn identity_space_counts_distinct_valid_ids_and_capacity() {
    let fixture = fixture::IdentityFixture::topic("capacity");
    let policy = fixture.resolved(fixture.address.clone()).identity_policy;
    let mut records = vec![
        Record::Flow(Flow { flow_id: FlowId("aaaaaa".to_owned()), address: fixture.address.clone(), session_id_option: None, state: State::Ended, flow_id_option: None, transcript_path_option: None }),
        Record::Permaflow(Permaflow { address: fixture.address.clone(), flow_id_vector: vec![FlowId("aaaaaa".to_owned()), FlowId("bbbbbb".to_owned())] }),
        Record::LaunchAttempt(fixture.launch(1, "cccccc", fixture.address.clone())),
        Record::AddressReservation(fixture.reservation(1, "dddddd", fixture.address.clone())),
        Record::Flow(Flow { flow_id: FlowId("outside".to_owned()), address: fixture.address.clone(), session_id_option: None, state: State::Ended, flow_id_option: None, transcript_path_option: None }),
    ];
    let space = RecordState { records: std::mem::take(&mut records) }.identity_space(&policy).unwrap();
    assert_eq!(space.occupied.0, 4);
    assert_eq!(space.capacity.0, 26u64.pow(6));
    assert!(!space.exhausted());
    let binary = IdentityPolicy::policy("ab");
    let full = RecordState { records: (0..64).map(|index| Record::Flow(Flow { flow_id: FlowId(format!("{index:06b}").replace('0', "a").replace('1', "b")), address: fixture.address.clone(), session_id_option: None, state: State::Ended, flow_id_option: None, transcript_path_option: None })).collect() };
    let binary_space = full.identity_space(&binary).unwrap();
    assert_eq!(binary_space.capacity.0, 64);
    assert!(binary_space.exhausted());
    let max = IdentityPolicy::policy("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789");
    assert_eq!(max.alphabet().unwrap().len() as u64, 62);
    assert_eq!(RecordState { records: vec![] }.identity_space(&max).unwrap().capacity.0, 56_800_235_584);
}
