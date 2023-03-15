use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use omnipaxos_core::storage::Snapshot;

static mut KV_STORE: Option<Arc<Mutex<KVStore>>> = None;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)] // Clone and Debug are required traits.
pub struct KeyValue {
    pub key: String,
    pub value: u64,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct KVStore {
    pub key_value: HashMap<String, u64>,
    pub decided_idx: u64,
}

impl KVStore {
    pub(crate) fn get_storage() -> Arc<Mutex<KVStore>> {
        unsafe {
            match KV_STORE {
                None => {
                    let kv_store = Arc::new(Mutex::new(KVStore {
                        key_value: HashMap::new(),
                        decided_idx: 0,
                    }));
                    KV_STORE = Some(kv_store.clone());
                    kv_store
                }
                Some(ref kv_store) =>
                    kv_store.clone(),
            }
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
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