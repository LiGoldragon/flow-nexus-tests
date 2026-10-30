use memory::{Memory, Schema, SchemaVersion, Table, Error};
use rkyv::{Archive, Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Archive, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
struct Reply { value: String }

const SCHEMA: Schema = Schema { version: SchemaVersion::new(1) };

#[test]
fn typed_reply_persists_across_reopen_and_rolls_back_failed_write() {
    let mut path = std::env::temp_dir();
    path.push(format!("flow_nexus_memory_{}.redb", std::process::id()));
    let _ = std::fs::remove_file(&path);
    const REPLIES: Table<&str, Reply> = Table::new("replies");
    let memory = Memory::open_with_schema(&path, &SCHEMA).unwrap();
    memory.write(|txn| REPLIES.insert(txn, "actor", &Reply { value: "ok".into() })).unwrap();
    let failed = memory.write(|txn| -> memory::Result<()> {
        REPLIES.insert(txn, "rollback", &Reply { value: "no".into() })?;
        Err(Error::SchemaVersionMismatch { expected: SchemaVersion::new(1), found: SchemaVersion::new(2) })
    });
    assert!(failed.is_err());
    assert!(memory.read(|txn| REPLIES.get(txn, "rollback")).unwrap().is_none());
    drop(memory);
    let reopened = Memory::open_with_schema(&path, &SCHEMA).unwrap();
    assert_eq!(reopened.read(|txn| REPLIES.get(txn, "actor")).unwrap(), Some(Reply { value: "ok".into() }));
    let _ = std::fs::remove_file(path);
}
