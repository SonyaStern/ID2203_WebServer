use actix_web::HttpResponse;
use actix_web::web::Json;
use actix_web::post;

use crate::KeyValue;
use crate::OP_SERVER_HANDLERS;

#[post("/key-value")]
pub async fn create(kv_req: Json<KeyValue>) -> HttpResponse {
    let kv = KeyValue {
        key: String::from(&kv_req.key),
        value: kv_req.value,
    };

    let handler = OP_SERVER_HANDLERS.lock().unwrap();
    let (server, _, _) = handler.get(&1).unwrap();

    println!("Adding value: {:?} via server {}", kv, 1);
    server
        .lock()
        .unwrap()
        .append(kv.clone())
        .expect("append failed");

    let response = serde_json::to_string(&kv).unwrap();
    HttpResponse::Created()
        .content_type("application/json")
        .json(response)
}