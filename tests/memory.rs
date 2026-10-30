use flow_nexus_runtime::{
    memory_actor::{MemoryAccessible, MemoryCommand, MemoryPort, MemoryReply},
    state::{Mutatable, RecordState},
    store::{MemoryStore, Openable, Readable, StoreLayout, TableName, Writable},
};
use flow_nexus_types::{
    evidence::{Invariant, Mutation, MutationBatch, ReadSelector, Record, RecordKey, RequestId},
    memory::RequestAllocator,
};
use memory::{Schema, SchemaVersion};

#[test]
fn allocator_high_water_cannot_regress_or_disappear() {
    let state = RecordState::default().mutate(&MutationBatch(vec![
        Mutation::Put(Record::RequestAllocator(RequestAllocator { request_id: RequestId(8) })),
    ])).unwrap();
    assert_eq!(state.mutate(&MutationBatch(vec![
        Mutation::Put(Record::RequestAllocator(RequestAllocator { request_id: RequestId(7) })),
    ])), Err(Invariant::CounterRegression(RecordKey::RequestAllocatorSingleton)));
    assert_eq!(state.mutate(&MutationBatch(vec![Mutation::Remove(RecordKey::RequestAllocatorSingleton)])),
        Err(Invariant::AllocationStateRequired(RecordKey::RequestAllocatorSingleton)));
}

#[test]
fn duplicate_key_batch_refuses_without_touching_original() {
    let state = RecordState::default();
    let batch = MutationBatch(vec![
        Mutation::Put(Record::RequestAllocator(RequestAllocator { request_id: RequestId(8) })),
        Mutation::Remove(RecordKey::RequestAllocatorSingleton),
    ]);
    assert_eq!(state.mutate(&batch), Err(Invariant::DuplicateMutation(RecordKey::RequestAllocatorSingleton)));
    assert!(state.records.is_empty());
}

#[test]
fn actual_flow_store_reopens_and_rejects_whole_invalid_batch() {
    let path = std::env::temp_dir().join(format!("flow-v1-store-{}.redb", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let mut store = MemoryStore::open(StoreLayout {
        path: path.clone(), table_name: TableName("rows"),
        schema: Schema { version: SchemaVersion::new(1) },
    }).unwrap();
    store.mutate(&MutationBatch(vec![Mutation::Put(Record::RequestAllocator(
        RequestAllocator { request_id: RequestId(8) }))])).unwrap();
    let before = store.snapshot().unwrap();
    assert!(store.mutate(&MutationBatch(vec![
        Mutation::Put(Record::ArrivalAllocator(flow_nexus_types::memory::ArrivalAllocator {
            address: flow_nexus_types::Address { aspect: flow_nexus_types::Aspect::Mind,
                topic: flow_nexus_types::Topic("atomic".into()), layer: flow_nexus_types::Layer::Tertiary },
            arrival_ordinal: flow_nexus_types::evidence::ArrivalOrdinal(3),
        })),
        Mutation::Remove(RecordKey::RequestAllocatorSingleton),
    ])).is_err());
    assert_eq!(store.snapshot().unwrap(), before);
    drop(store);
    let reopened = MemoryStore::open(StoreLayout {
        path: path.clone(), table_name: TableName("rows"),
        schema: Schema { version: SchemaVersion::new(1) },
    }).unwrap();
    assert_eq!(reopened.snapshot().unwrap(), before);
    drop(reopened);
    std::fs::remove_file(path).unwrap();
}

#[tokio::test]
async fn actual_actor_preserves_committed_rejected_and_record_replies() {
    let path = std::env::temp_dir().join(format!("flow-v1-actor-{}.redb", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let store = MemoryStore::open(StoreLayout {
        path, table_name: TableName("rows"),
        schema: Schema { version: SchemaVersion::new(1) },
    }).unwrap();
    let port = MemoryPort::from(store);
    assert!(matches!(port.request(MemoryCommand::OwnerBatch(MutationBatch(vec![
        Mutation::Put(Record::RequestAllocator(RequestAllocator { request_id: RequestId(8) })),
    ]))).await.unwrap(), MemoryReply::Committed));
    assert!(matches!(port.request(MemoryCommand::OwnerBatch(MutationBatch(vec![
        Mutation::Remove(RecordKey::RequestAllocatorSingleton),
    ]))).await.unwrap(), MemoryReply::Rejected(Invariant::AllocationStateRequired(_))));
    let reply = port.request(MemoryCommand::Read(ReadSelector::All)).await.unwrap();
    match reply {
        MemoryReply::Records(rows) => assert_eq!(rows, vec![Record::RequestAllocator(
            RequestAllocator { request_id: RequestId(8) })]),
        other => panic!("expected typed Records, received {other:?}"),
    }
}
