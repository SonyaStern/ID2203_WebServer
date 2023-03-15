use actix_web::http::StatusCode;
use restest::{assert_body_matches, Context, path, Request};
use serde::{Deserialize, Serialize};

const CONTEXT: Context = Context::new().with_port(8000);

#[tokio::test]
async fn test_first_route() {
    let request = Request::post(path!["key-value"])
        .with_header("ContentType", "application/json")
        .with_body(KeyValue {
            key: String::from("a"),
            value: 2,
        });

    let body = CONTEXT
        .run(request)
        .await
        .expect_status(StatusCode::CREATED)
        .await;

    assert_body_matches! {
        body,
        KeyValueResponse { key: "a", value: 2,..}
    }
}
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct KeyValue {
    pub key: String,
    pub value: u64
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct KeyValueResponse {
    pub key: String,
    pub value: u64,
    pub decided_idx: u64,
}
