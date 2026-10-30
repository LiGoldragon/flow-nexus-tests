use flow_nexus_platform::{BytesRead, LinuxEntropy, OsEntropyReadable};

#[test]
fn empty_buffer_returns_zero_without_accessing_memory() {
    assert_eq!(LinuxEntropy::default().read_entropy(&mut []).unwrap(), BytesRead(0));
}
#[test]
fn actual_nonblocking_getrandom_returns_a_bounded_initialized_prefix() {
    let mut buffer = [0xa5; 256];
    let BytesRead(count) = LinuxEntropy::default().read_entropy(&mut buffer).unwrap();
    assert!((1..=buffer.len()).contains(&count));
    // No statistical/disjoint-output claim: this witnesses the syscall interface only.
    assert_eq!(&buffer[count..], &vec![0xa5; buffer.len() - count]);
}
