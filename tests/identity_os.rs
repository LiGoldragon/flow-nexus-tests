use flow_nexus_platform::LinuxEntropy;
use flow_nexus_runtime::identity::{CandidateMintable, CancellationObservable, MintContext};
use flow_nexus_types::{IdentityAlphabet, IdentityPolicy};
use std::sync::atomic::{AtomicBool, Ordering};
struct Cancellation { requested: AtomicBool }
impl CancellationObservable for Cancellation {
    fn is_cancelled(&self) -> bool { self.requested.load(Ordering::Relaxed) }
}
#[tokio::test]
async fn actual_os_entropy_mints_six_configured_characters() {
    let policy = IdentityPolicy { identity_alphabet: IdentityAlphabet("A7z".into()) };
    let cancellation = Cancellation { requested: AtomicBool::new(false) };
    let mut entropy = LinuxEntropy::default();
    let id = MintContext { policy: &policy, entropy: &mut entropy, cancellation: &cancellation }.mint().await.unwrap();
    assert_eq!(id.0.len(), 6);
    assert!(id.0.bytes().all(|byte| b"A7z".contains(&byte)));
}
