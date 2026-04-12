//! E2E tests for convergio-file-transport HTTP route handlers.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use convergio_db::pool::ConnPool;
use convergio_file_transport::routes::file_transport_routes;
use convergio_file_transport::transfer;
use convergio_file_transport::types::{
    TransferDirection, TransferRequest, TransferResult, TransferStatus,
};
use tower::ServiceExt;

fn setup() -> (axum::Router, ConnPool) {
    let pool = convergio_db::pool::create_memory_pool().unwrap();
    let conn = pool.get().unwrap();
    for m in convergio_file_transport::schema::migrations() {
        conn.execute_batch(m.up).unwrap();
    }
    drop(conn);
    let app = file_transport_routes(pool.clone());
    (app, pool)
}

fn rebuild(pool: &ConnPool) -> axum::Router {
    file_transport_routes(pool.clone())
}

async fn body_json(resp: axum::http::Response<Body>) -> serde_json::Value {
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

fn get_req(uri: &str) -> Request<Body> {
    Request::builder().uri(uri).body(Body::empty()).unwrap()
}

fn sample_req() -> TransferRequest {
    TransferRequest {
        source_path: "/data/project/".into(),
        dest_path: "/backup/project/".into(),
        peer_name: "studio-mac".into(),
        ssh_target: "rob@192.168.1.50".into(),
        direction: TransferDirection::Push,
        exclude_patterns: vec![],
    }
}

fn sample_result() -> TransferResult {
    TransferResult {
        peer_name: "studio-mac".into(),
        bytes_transferred: 4096,
        files_count: 12,
        duration_ms: 350,
        status: TransferStatus::Success,
    }
}

fn seed_transfer(pool: &ConnPool) -> i64 {
    let conn = pool.get().unwrap();
    transfer::record_transfer(&conn, &sample_result(), &sample_req()).unwrap()
}

#[tokio::test]
async fn route_list_transfers_empty() {
    let (app, _) = setup();
    let resp = app
        .oneshot(get_req("/api/file-transport/transfers"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["ok"], true);
    assert_eq!(json["transfers"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn route_list_transfers_with_data() {
    let (_, pool) = setup();
    seed_transfer(&pool);
    seed_transfer(&pool);

    let app = rebuild(&pool);
    let resp = app
        .oneshot(get_req("/api/file-transport/transfers"))
        .await
        .unwrap();
    let json = body_json(resp).await;
    assert_eq!(json["ok"], true);
    assert_eq!(json["transfers"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn route_list_transfers_with_peer_filter() {
    let (_, pool) = setup();
    seed_transfer(&pool);

    let app = rebuild(&pool);
    let resp = app
        .oneshot(get_req("/api/file-transport/transfers?peer=studio-mac"))
        .await
        .unwrap();
    let json = body_json(resp).await;
    assert_eq!(json["ok"], true);
    assert_eq!(json["transfers"].as_array().unwrap().len(), 1);

    let app2 = rebuild(&pool);
    let resp2 = app2
        .oneshot(get_req("/api/file-transport/transfers?peer=nonexistent"))
        .await
        .unwrap();
    let json2 = body_json(resp2).await;
    assert_eq!(json2["transfers"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn route_list_transfers_limit() {
    let (_, pool) = setup();
    for _ in 0..5 {
        seed_transfer(&pool);
    }
    let app = rebuild(&pool);
    let resp = app
        .oneshot(get_req("/api/file-transport/transfers?limit=2"))
        .await
        .unwrap();
    let json = body_json(resp).await;
    assert_eq!(json["transfers"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn route_get_transfer_by_id() {
    let (_, pool) = setup();
    let id = seed_transfer(&pool);

    let app = rebuild(&pool);
    let uri = format!("/api/file-transport/transfers/{id}");
    let resp = app.oneshot(get_req(&uri)).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["ok"], true);
    let t = &json["transfer"];
    assert_eq!(t["peer_name"], "studio-mac");
    assert_eq!(t["direction"], "push");
    assert_eq!(t["bytes_transferred"], 4096);
}

#[tokio::test]
async fn route_get_transfer_not_found() {
    let (app, _) = setup();
    let resp = app
        .oneshot(get_req("/api/file-transport/transfers/999"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["ok"], false);
    assert_eq!(json["error"], "transfer not found");
}
