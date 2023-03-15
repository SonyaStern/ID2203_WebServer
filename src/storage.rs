use omnipaxos_core::util::LogEntry;

use crate::{KeyValue, WAIT_DECIDED_TIMEOUT};
use crate::kv::KVStore;
use crate::kv_controller::KeyValueResponse;
use crate::OP_SERVER_HANDLERS;

pub async fn get_kv(key: String) -> KeyValueResponse {
    sync_decided_kv().await;

    let kv_store = KVStore::get_storage();
    let storage = kv_store.lock().unwrap();

    let value = storage.key_value.get(key.as_str());
    let response: KeyValueResponse;
    match value {
        None => {
            response = KeyValueResponse {
                key: "".to_string(),
                value: 0,
                decided_idx: 0,
            };
        }
        Some(v) => {
            response = KeyValueResponse {
                key: key.to_string(),
                value: *v,
                decided_idx: storage.decided_idx,
            };
        }
    }
    response.clone()
}

pub async fn create_kv(kv: KeyValue) -> u64 {
    let kv_store = KVStore::get_storage();
    let storage = kv_store.lock().unwrap();
    storage.key_value.get(&kv.key);

    let handler = OP_SERVER_HANDLERS.lock().unwrap();
    let (server, _, _) = handler.get(&1).unwrap();

    let before_idx = server.lock()
        .unwrap()
        .get_decided_idx();

    server
        .lock()
        .unwrap()
        .append(kv.clone())
        .expect("append failed");

    // Check other replicas got the value
    let leader_pid = server
        .lock()
        .unwrap()
        .get_current_leader()
        .expect("Leader not found");

    println!("Adding value: {:?} via server {}, leader {}", kv, 1, leader_pid);

    let (leader, _, _) = handler.get(&leader_pid).unwrap();
    loop {
        std::thread::sleep(WAIT_DECIDED_TIMEOUT);
        let committed_ents = leader
            .lock()
            .unwrap()
            .read_decided_suffix(0)
            .expect("Failed to read expected entries");
        for (i, ent) in committed_ents.iter().enumerate() {
            match ent {
                LogEntry::Decided(kv_decided) => {
                    if kv.key == kv_decided.key {
                        return before_idx + (i as u64);
                    }
                }
                _ => {} // ignore not committed entries
            }
        }
    }
}

async fn sync_decided_kv() {
    let kv_store = KVStore::get_storage();
    let mut storage = kv_store.lock().unwrap();

    let handler = OP_SERVER_HANDLERS.lock().unwrap();
    let (server, _, _) = handler.get(&1).unwrap();

    let last_idx = server
        .lock()
        .unwrap()
        .get_decided_idx();

    if last_idx > storage.decided_idx {
        let committed_ents = server
            .lock()
            .unwrap()
            .read_decided_suffix(storage.decided_idx)
            .expect("Failed to read expected entries");
        for (i, ent) in committed_ents.iter().enumerate() {
            match ent {
                LogEntry::Decided(kv_decided) => {
                    storage.decided_idx += 1;
                    storage.key_value.insert(kv_decided.key.clone(), kv_decided.value);
                    println!("Adding value: {:?} via server {}", kv_decided.value, 1);
                }
                _ => {} // ignore not committed entries
            }
        }
    }
}