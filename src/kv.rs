use std::collections::HashMap;
use omnipaxos_core::storage::Snapshot;

#[derive(Clone, Debug)] // Clone and Debug are required traits.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct KeyValue {
    pub key: String,
    pub value: u64,
}

#[derive(Clone, Debug)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct KVSnapshot {
    snapshotted: HashMap<String, u64>,
}

impl Snapshot<KeyValue> for KVSnapshot {
    fn create(entries: &[KeyValue]) -> Self {
        let mut snapshotted = HashMap::new();
        for e in entries {
            let KeyValue { key, value } = e;
            snapshotted.insert(key.clone(), *value);
        }
        Self { snapshotted }
    }

    fn merge(&mut self, delta: Self) {
        for (k, v) in delta.snapshotted {
            self.snapshotted.insert(k, v);
        }
    }

    fn use_snapshots() -> bool {
        true
    }
}