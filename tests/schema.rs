use std::fs;

#[test]
fn ordinary_projection_has_no_request_id() {
    let source = fs::read_to_string("signal-flow-nexus/ethos/generated/signal.rs").expect("ordinary generated source");
    assert!(!source.contains("RequestId"));
}

#[test]
fn manifest_declares_text_dependencies_as_optional_feature_members() {
    let ordinary = fs::read_to_string("signal-flow-nexus/Cargo.toml").expect("ordinary manifest");
    let meta = fs::read_to_string("meta-signal-flow-nexus/Cargo.toml").expect("meta manifest");
    assert!(ordinary.contains("datom-codec") && meta.contains("datom-codec"));
    assert!(ordinary.contains("datom = [\"dep:datom-codec\""));
    assert!(meta.contains("datom = [\"dep:datom-codec\""));
}
