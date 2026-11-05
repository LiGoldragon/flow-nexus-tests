use std::fs;

#[test]
fn ordinary_projection_has_no_request_id() {
    let source = fs::read_to_string("signal-flow-nexus/ethos/generated/signal.rs").expect("ordinary generated source");
    assert!(!source.contains("RequestId"));
}

#[test]
fn every_actual_memory_root_row_is_in_owner_record_union() {
    let memory = include_str!("../flow-nexus/ethos/memory.ethos");
    let evidence = include_str!("../flow-nexus/ethos/evidence.ethos");
    // This oracle enumerates the authored Memory root, independently of the Record implementation list.
    let rows: Vec<_> = memory.lines().filter_map(|line| {
        let head = line.split(".{").next()?;
        if !line.contains(".{") { return None; }
        head.split_whitespace().last()
    }).collect();
    assert!(!rows.is_empty());
    let union = evidence.lines().find(|line| line.trim_start().starts_with("Record.[")).unwrap();
    let variants: Vec<_> = union.split("Record.[").nth(1).unwrap().split(']').next().unwrap()
        .split_whitespace().collect();
    assert_eq!(variants.len(), rows.len(), "owner union must have exactly the actual Memory row kinds");
    for row in rows {
        assert!(variants.contains(&format!("{row}.{row}").as_str()), "missing typed row {row}");
    }
    let meta = include_str!("../meta-signal-flow-nexus/ethos/signal.ethos");
    assert!(meta.contains("Mutate.MutationBatch"));
    assert!(meta.contains("Read.ReadSelector"));
    assert!(meta.contains("Vector<Record>"));
}

#[test]
fn request_vocabulary_and_declaration_blacklist_survive_generation() {
    let generated = include_str!("../flow-nexus/ethos/generated/library.rs");
    let request = generated.split("pub enum Request {").nth(1).unwrap().split('}').next().unwrap();
    for variant in ["Order(String)", "Question(String)", "Words(String)", "Result(String)", "Notice(String)"] {
        assert!(request.contains(variant), "missing request variant {variant}");
    }
    let declaration = generated.split("pub struct Declaration {").nth(1).unwrap().split('}').next().unwrap();
    assert!(declaration.contains("harness_module_name_vector: std::vec::Vec<HarnessModuleName>"));
}
