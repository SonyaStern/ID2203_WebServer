use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use lazy_static::lazy_static;

lazy_static! {
    // mimic multiple web server replicas
    pub static ref STORAGE_REPLICAS: Vec<Arc<Mutex<KVStore>>> = {
        let mut set = Vec::new();
        set.insert(0, Arc::new(Mutex::new(
            KVStore {
                        key_value: HashMap::new(),
                        decided_idx: 0,
                    })
        ));
         set.insert(0, Arc::new(Mutex::new(
            KVStore {
                        key_value: HashMap::new(),
                        decided_idx: 0,
                    })
        ));
         set.insert(0, Arc::new(Mutex::new(
            KVStore {
                        key_value: HashMap::new(),
                        decided_idx: 0,
                    })
        ));
        set
    };

    pub static ref TEST_STORAGE_REPLICAS: Vec<Arc<Mutex<KVStore>>> = {
        let mut set = Vec::new();
         set.insert(0, Arc::new(Mutex::new(
            KVStore {
                        key_value: HashMap::new(),
                        decided_idx: 0,
                    })
        ));
         set.insert(0, Arc::new(Mutex::new(
            KVStore {
                        key_value: HashMap::new(),
                        decided_idx: 0,
                    })
        ));
        set.insert(0, STORAGE_REPLICAS.get(0).unwrap().clone());
        set
    };
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct KVStore {
    pub key_value: HashMap<String, u64>,
    pub decided_idx: u64,
}

impl KVStore {
    pub(crate) fn get_storage(id: usize, is_failed: bool) -> Arc<Mutex<KVStore>> {
        if is_failed {
            TEST_STORAGE_REPLICAS.get(id).unwrap().clone()
        } else {
            STORAGE_REPLICAS.get(id).unwrap().clone()
        }
    }
}