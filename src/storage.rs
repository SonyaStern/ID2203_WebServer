use std::sync::{Arc, Mutex};

use omnipaxos_core::util::LogEntry;
use rand::Rng;

use crate::{KeyValue, SERVERS, WAIT_DECIDED_TIMEOUT};
use crate::kv::KeyValueCas;
use crate::kv_controller::KeyValueResponse;
use crate::nodes::KVStore;
use crate::nodes::STORAGE_REPLICAS;
use crate::OP_SERVER_HANDLERS;

const PEERS: u64 = SERVERS.len() as u64;

pub async fn get_kv(key: String) -> KeyValueResponse {
    let mut is_failed = false;
    if cfg!(feature="test") && rand::thread_rng().gen_range(1..10) > 7 {
        is_failed = true;
        println!("A storage was disconnected")
    }
    let replica_id = rand::thread_rng().gen_range(0..STORAGE_REPLICAS.len());
    sync_decided_kv(replica_id, is_failed).await;

    let kv_store = KVStore::get_storage(replica_id, is_failed);

    println!("Get value by replica {:?}", replica_id);
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

pub async fn cas_kv(kv: KeyValueCas) -> u64 {
    let read_response = get_kv(kv.key.clone()).await;
    if read_response.value == kv.old_value {
        let new_kv = KeyValue {
            key: kv.key,
            value: kv.new_value,
        };
        return create_kv(new_kv).await;
    } else {
        return 0;
    }
}

pub async fn create_kv(kv: KeyValue) -> u64 {
    // sync_decided_kv().await;

    let mut is_failed = false;
    if cfg!(feature="test") && rand::thread_rng().gen_range(1..10) > 7 {
        is_failed = true;
        println!("The last storage was disconnected")
    }
    let replica_id = rand::thread_rng().gen_range(0..STORAGE_REPLICAS.len());

    let kv_store = KVStore::get_storage(replica_id, is_failed);
    let storage = kv_store.lock().unwrap();
    storage.key_value.get(&kv.key);

    let handler = OP_SERVER_HANDLERS.lock().unwrap();
    let server_id = rand::thread_rng().gen_range(1..PEERS);
    let (server, _, _) = handler.get(&server_id).unwrap();

    let leader_id = server
        .lock()
        .unwrap()
        .get_current_leader()
        .expect("Failed to get leader");
    let (leader, _, _) = handler.get(&leader_id).unwrap();

    let before_idx = leader
        .lock()
        .unwrap()
        .get_decided_idx();
    println!("Before index {}", before_idx);

    leader
        .lock()
        .unwrap()
        .append(kv.clone())
        .expect("append failed");

    loop {
        std::thread::sleep(WAIT_DECIDED_TIMEOUT);
        let committed_ents = server
            .lock()
            .unwrap()
            .read_decided_suffix(before_idx)
            .expect("Failed to read expected entries");
        for (i, ent) in committed_ents.iter().enumerate() {
            match ent {
                LogEntry::Decided(kv_decided) => {
                    if kv.key == kv_decided.key {
                        let new_idx = before_idx + (i as u64) + 1;
                        println!("Adding value: {:?}, decided idx {} via server {} and replica {}",
                                 kv, new_idx, leader_id, replica_id);
                        return new_idx;
                    }
                }
                _ => {} // ignore not committed entries
            }
        }
    }
}

async fn sync_decided_kv(replica_id: usize, is_failed: bool) {
    let kv_store = KVStore::get_storage(replica_id, is_failed);
    let mut storage = kv_store.lock().unwrap();

    let handler = OP_SERVER_HANDLERS.lock().unwrap();
    let mut server_id = rand::thread_rng().gen_range(1..PEERS);
    // println!("Chosen server {}", server_id);
    let (server, _, _) = handler.get(&server_id).unwrap();

    let mut last_idx = server
        .lock()
        .unwrap()
        .get_decided_idx();
    for n in 1..(PEERS / 2 + 1) as u64 {
        let (server, _, _) = handler.get(&n).unwrap();
        let tmp_idx = server
            .lock()
            .unwrap()
            .get_decided_idx();
        if tmp_idx > last_idx {
            last_idx = tmp_idx;
            server_id = n;
        }
    }
    println!("Last index {}", last_idx);
    println!("Local index {}", storage.decided_idx);

    if last_idx > storage.decided_idx {
        let committed_ents = server
            .lock()
            .unwrap()
            .read_decided_suffix(storage.decided_idx as u64)
            .expect("Failed to read expected entries");

        for (_, ent) in committed_ents.iter().enumerate() {
            match ent {
                LogEntry::Decided(kv_decided) => {
                    storage.decided_idx += 1;
                    storage.key_value.insert(kv_decided.key.clone(), kv_decided.value);
                    println!("Adding value: {:?}, decided idx {} via server {}",
                             kv_decided.value, storage.decided_idx, server_id);
                }
                LogEntry::Snapshotted(kv_snapshotted) => {
                    for (k, v) in &kv_snapshotted.snapshot.snapshotted {
                        storage.decided_idx += 1;
                        storage.key_value.insert(k.clone(), *v);
                        println!("Adding value: {:?}, decided inx {} via server {}",
                                 v, storage.decided_idx, server_id);
                    }
                }
                _ => {} // ignore not committed entries
            }
        }
    }
}