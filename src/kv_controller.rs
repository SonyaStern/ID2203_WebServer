use std::collections::HashMap;
use actix_web::HttpResponse;
use actix_web::web::Json;
use actix_web::post;
use omnipaxos_core::util::LogEntry;

use crate::{KeyValue, WAIT_DECIDED_TIMEOUT};
use crate::OP_SERVER_HANDLERS;

#[post("/key-value")]
pub async fn create(kv_req: Json<KeyValue>) -> HttpResponse {
    let kv = KeyValue {
        key: String::from(&kv_req.key),
        value: kv_req.value,
    };

    let handler = OP_SERVER_HANDLERS.lock().unwrap();
    let (server, _, _) = handler.get(&1).unwrap();

    server
        .lock()
        .unwrap()
        .append(kv.clone())
        .expect("append failed");

    let leader_pid = server
        .lock()
        .unwrap()
        .get_current_leader()
        .expect("Leader not found");

    println!("Adding value: {:?} via server {}, leader {}", kv, 1, leader_pid);

    let (leader, _, _) = handler.get(&leader_pid).unwrap();

    std::thread::sleep(WAIT_DECIDED_TIMEOUT);

    let committed_ents = leader
        .lock()
        .unwrap()
        .read_decided_suffix(0)
        .expect("Failed to read expected entries");

    let mut simple_kv_store = HashMap::new();
    for ent in committed_ents {
        match ent {
            LogEntry::Decided(kv) => {
                simple_kv_store.insert(kv.key, kv.value);
            }
            _ => {} // ignore not committed entries
        }
    }
    println!("KV store: {:?}", simple_kv_store);

    let response = serde_json::to_string(&kv).unwrap();
    HttpResponse::Created()
        .content_type("application/json")
        .json(response)
}