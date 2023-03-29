use actix_web::get;
use actix_web::HttpResponse;
use actix_web::post;
use actix_web::web::{Json, Path};
use http::StatusCode;
use serde::Deserialize;

use storage::create_kv;

use crate::{KeyValue, storage};
use crate::kv::KeyValueCas;
use crate::storage::{cas_kv, get_kv};

#[post("/key-value")]
pub async fn create(kv_req: Json<KeyValue>) -> HttpResponse {
    let kv = KeyValue {
        key: String::from(&kv_req.key),
        value: kv_req.value,
    };

    let decided_idx = create_kv(kv.clone()).await;
    println!("decided_idx: {:?}", decided_idx);

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

#[post("/key-value/cas")]
pub async fn cas(kv_req: Json<KeyValueCas>) -> HttpResponse {
    let kv = KeyValueCas {
        key: String::from(&kv_req.key),
        old_value: kv_req.old_value,
        new_value: kv_req.new_value
    };

    let decided_idx = cas_kv(kv.clone()).await;
    println!("CAS decided_idx: {:?}", decided_idx);

    if decided_idx == 0 {
        HttpResponse::BadRequest()
            .content_type("application/json")
            .status(StatusCode::BAD_REQUEST)
            .json("Could not CAS KV since the old value was different")
    } else {
        let response = KeyValueResponse {
            key: kv.key,
            value: kv.old_value,
            decided_idx,
        };
        HttpResponse::Ok()
            .content_type("application/json")
            .status(StatusCode::OK)
            .json(response)
    }
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