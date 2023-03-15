use actix_web::get;
use actix_web::HttpResponse;
use actix_web::post;
use actix_web::web::{Json, Path};
use http::StatusCode;
use serde::Deserialize;

use storage::create_kv;

use crate::{KeyValue, storage};
use crate::storage::get_kv;

#[post("/key-value")]
pub async fn create(kv_req: Json<KeyValue>) -> HttpResponse {
    let kv = KeyValue {
        key: String::from(&kv_req.key),
        value: kv_req.value,
    };

    let decided_idx = create_kv(kv.clone()).await;
    println!("decided_idx: {}", decided_idx);

    let response = KeyValueResponse {
        key: kv.key,
        value: kv.value,
        decided_idx,
    };

    HttpResponse::Created()
        .content_type("application/json")
        .status(StatusCode::CREATED)
        .json(response)
}

#[get("/key-value/{key}")]
pub async fn get(key: Path<String>) -> HttpResponse {
    let response = get_kv(key.into_inner()).await;

    return if response.key.is_empty() {
        HttpResponse::NotFound()
            .content_type("application/json")
            .status(StatusCode::NOT_FOUND)
            .finish()
    } else {
        HttpResponse::Ok()
            .content_type("application/json")
            .status(StatusCode::OK)
            .json(response)
    };
}

#[derive(Clone, Debug, serde::Serialize, Deserialize)]
pub struct KeyValueResponse {
    pub key: String,
    pub value: u64,
    pub decided_idx: u64,
}