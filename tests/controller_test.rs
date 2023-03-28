use actix_web::http::StatusCode;
use futures::{FutureExt, stream, StreamExt};
use restest::{assert_body_matches, Context, path, Request};
use serde::{Deserialize, Serialize};

const CONTEXT: Context = Context::new().with_port(8000);
const CONCURRENT_REQUESTS: usize = 3;

#[tokio::test]
async fn test_create() {
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
        KeyValueResponse { key: "a", value: 2, decided_idx: 1}
    }
}

#[tokio::test]
async fn test_multiple_create() {
    let mut reqs: Vec<Request<KeyValue>> = Vec::new();

    let request1 = Request::post(path!["key-value"])
        .with_header("ContentType", "application/json")
        .with_body(KeyValue {
            key: String::from("a"),
            value: 1,
        });
    let request2 = Request::post(path!["key-value"])
        .with_header("ContentType", "application/json")
        .with_body(KeyValue {
            key: String::from("a"),
            value: 2,
        });

    let request3 = Request::post(path!["key-value"])
        .with_header("ContentType", "application/json")
        .with_body(KeyValue {
            key: String::from("a"),
            value: 3,
        });
    reqs.insert(0, request1);
    reqs.insert(0, request2);
    reqs.insert(0, request3);

    let mut i = -1;
    let bodies = stream::iter(reqs)
        .map(|req| {
            let context = &CONTEXT;
            i += 1;
            tokio::spawn(async move {
                let body = CONTEXT
                    .run(req)
                    .await
                    .expect_status(StatusCode::CREATED)
                    .await;

                assert_body_matches! {
        body,
        KeyValueResponse { key: "a", decided_idx: i, ..}
    }
            })
        })
        .buffer_unordered(CONCURRENT_REQUESTS);

    bodies.for_each(|b| async {
        match b {
            Ok(_b) => println!("Successful"),
            Err(_e) => println!("Exception")
        }
    }).await;
}

#[tokio::test]
async fn test_get() {
    let request = Request::post(path!["key-value"])
        .with_header("ContentType", "application/json")
        .with_body(KeyValue {
            key: String::from("a"),
            value: 10,
        });

    let body = CONTEXT
        .run(request)
        .await
        .expect_status(StatusCode::CREATED)
        .await;

    assert_body_matches! {
        body,
        KeyValueResponse { key: "a", value: 10,..}
    }

    let request = Request::get(path!["key-value/a"])
        .with_header("ContentType", "application/json")
        .with_body("");

    let body = CONTEXT
        .run(request)
        .await
        .expect_status(StatusCode::OK)
        .await;

    assert_body_matches! {
        body,
        KeyValueResponse { key: "a", value: 10,..}
    }
}


#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct KeyValue {
    pub key: String,
    pub value: u64,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct KeyValueResponse {
    pub key: String,
    pub value: u64,
    pub decided_idx: u64,
}
