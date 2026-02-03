use axum::body::Body;
use bytes::Bytes;
use http::header;
use http::Request;
use http::StatusCode;
use http_body_util::BodyExt;
use linux_fs::config::AppConfig;
use linux_fs::persistence::wal::WalWriter;
use linux_fs::routes::build_router;
use linux_fs::state::AppState;
use serde_json::{json, Value};
use tower::ServiceExt;

const TEST_API_KEY: &str = "test-api-key-12345";

fn test_config(data_dir: &str) -> AppConfig {
    AppConfig {
        api_key: TEST_API_KEY.to_string(),
        host: "127.0.0.1".to_string(),
        port: 0,
        data_dir: data_dir.to_string(),
        default_max_repo_size: 1_073_741_824,
        max_upload_size: 104_857_600,
        snapshot_interval_secs: 3600,
        ttl_sweep_interval_secs: 3600,
        command_timeout_secs: 30,
        command_max_output_bytes: 10_485_760,
        cache_max_bytes: 268_435_456,
        max_concurrent_commands: 10,
        log_level: "error".to_string(),
        cors_allowed_origins: "*".to_string(),
    }
}

fn setup() -> (AppState, tempfile::TempDir) {
    let tmp = tempfile::tempdir().expect("failed to create temp dir");
    let data_dir = tmp.path().to_str().unwrap().to_string();
    let config = test_config(&data_dir);

    std::fs::create_dir_all(config.repos_dir()).unwrap();
    std::fs::create_dir_all(config.metadata_dir()).unwrap();
    std::fs::create_dir_all(config.wal_dir()).unwrap();

    let wal = WalWriter::open(&config.wal_dir()).unwrap();
    let state = AppState::new(config, wal);
    (state, tmp)
}

fn auth_header() -> (http::HeaderName, http::HeaderValue) {
    (
        http::HeaderName::from_static("x-api-key"),
        http::HeaderValue::from_static(TEST_API_KEY),
    )
}

async fn body_to_bytes(body: Body) -> Bytes {
    body.collect().await.unwrap().to_bytes()
}

async fn body_to_json(body: Body) -> Value {
    let bytes = body_to_bytes(body).await;
    serde_json::from_slice(&bytes).unwrap()
}

// Helper: create a repo and return its UUID
async fn create_test_repo(state: &AppState, name: &str) -> uuid::Uuid {
    let app = build_router(state.clone());
    let (key, val) = auth_header();
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/repos")
        .header(key, val)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::to_string(&json!({"name": name})).unwrap(),
        ))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: Value = body_to_json(resp.into_body()).await;
    let id_str = body["data"]["id"].as_str().unwrap();
    uuid::Uuid::parse_str(id_str).unwrap()
}

// Helper: upload a file to a repo
async fn upload_test_file(state: &AppState, repo_id: uuid::Uuid, path: &str, content: &[u8]) {
    let app = build_router(state.clone());
    let (key, val) = auth_header();
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/repos/{}/files/{}", repo_id, path))
        .header(key, val)
        .body(Body::from(Bytes::from(content.to_vec())))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
}

// ==================== Health Tests ====================

#[tokio::test]
async fn test_health_returns_200() {
    let (state, _tmp) = setup();
    let app = build_router(state);

    let req = Request::builder()
        .uri("/health")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body: Value = body_to_json(resp.into_body()).await;
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn test_status_without_auth_returns_401() {
    let (state, _tmp) = setup();
    let app = build_router(state);

    let req = Request::builder()
        .uri("/api/v1/status")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_status_with_auth_returns_200() {
    let (state, _tmp) = setup();
    let app = build_router(state);

    let (key, val) = auth_header();
    let req = Request::builder()
        .uri("/api/v1/status")
        .header(key, val)
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body: Value = body_to_json(resp.into_body()).await;
    assert_eq!(body["data"]["repo_count"], 0);
    assert!(body["data"]["uptime_seconds"].is_number());
    assert!(body["data"]["version"].is_string());
}

// ==================== Repo Tests ====================

#[tokio::test]
async fn test_create_repo_returns_201() {
    let (state, _tmp) = setup();
    let app = build_router(state);

    let (key, val) = auth_header();
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/repos")
        .header(key, val)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"name":"test-repo"}"#))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body: Value = body_to_json(resp.into_body()).await;
    assert!(body["data"]["id"].is_string());
    assert_eq!(body["data"]["name"], "test-repo");
    assert!(body["error"].is_null());
}

#[tokio::test]
async fn test_create_repo_empty_name_returns_400() {
    let (state, _tmp) = setup();
    let app = build_router(state);

    let (key, val) = auth_header();
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/repos")
        .header(key, val)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"name":""}"#))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_list_repos_returns_paginated() {
    let (state, _tmp) = setup();
    let repo_id = create_test_repo(&state, "list-test").await;

    let app = build_router(state);
    let (key, val) = auth_header();
    let req = Request::builder()
        .uri("/api/v1/repos?page=1&per_page=10")
        .header(key, val)
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body: Value = body_to_json(resp.into_body()).await;
    assert_eq!(body["data"]["total"], 1);
    assert_eq!(body["data"]["repos"][0]["id"], repo_id.to_string());
}

#[tokio::test]
async fn test_get_repo_returns_200() {
    let (state, _tmp) = setup();
    let repo_id = create_test_repo(&state, "get-test").await;

    let app = build_router(state);
    let (key, val) = auth_header();
    let req = Request::builder()
        .uri(format!("/api/v1/repos/{}", repo_id))
        .header(key, val)
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body: Value = body_to_json(resp.into_body()).await;
    assert_eq!(body["data"]["repo"]["name"], "get-test");
}

#[tokio::test]
async fn test_get_repo_not_found_returns_404() {
    let (state, _tmp) = setup();
    let app = build_router(state);

    let fake_id = uuid::Uuid::new_v4();
    let (key, val) = auth_header();
    let req = Request::builder()
        .uri(format!("/api/v1/repos/{}", fake_id))
        .header(key, val)
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_update_repo() {
    let (state, _tmp) = setup();
    let repo_id = create_test_repo(&state, "update-test").await;

    let app = build_router(state);
    let (key, val) = auth_header();
    let req = Request::builder()
        .method("PATCH")
        .uri(format!("/api/v1/repos/{}", repo_id))
        .header(key, val)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"name":"updated-name"}"#))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body: Value = body_to_json(resp.into_body()).await;
    assert_eq!(body["data"]["name"], "updated-name");
}

#[tokio::test]
async fn test_delete_repo_then_get_returns_404() {
    let (state, _tmp) = setup();
    let repo_id = create_test_repo(&state, "delete-test").await;

    // Delete
    let app = build_router(state.clone());
    let (key, val) = auth_header();
    let req = Request::builder()
        .method("DELETE")
        .uri(format!("/api/v1/repos/{}", repo_id))
        .header(key, val)
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // GET should now 404
    let app = build_router(state);
    let (key, val) = auth_header();
    let req = Request::builder()
        .uri(format!("/api/v1/repos/{}", repo_id))
        .header(key, val)
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// ==================== File Tests ====================

#[tokio::test]
async fn test_upload_file_returns_201() {
    let (state, _tmp) = setup();
    let repo_id = create_test_repo(&state, "file-upload").await;

    let app = build_router(state);
    let (key, val) = auth_header();
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/repos/{}/files/test.txt", repo_id))
        .header(key, val)
        .body(Body::from("hello world"))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let etag = resp.headers().get("etag");
    assert!(etag.is_some());

    let body: Value = body_to_json(resp.into_body()).await;
    assert_eq!(body["data"]["path"], "test.txt");
    assert_eq!(body["data"]["size_bytes"], 11);
}

#[tokio::test]
async fn test_download_file_returns_content() {
    let (state, _tmp) = setup();
    let repo_id = create_test_repo(&state, "file-download").await;
    upload_test_file(&state, repo_id, "hello.txt", b"hello world").await;

    let app = build_router(state);
    let (key, val) = auth_header();
    let req = Request::builder()
        .uri(format!("/api/v1/repos/{}/files/hello.txt", repo_id))
        .header(key, val)
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = body_to_bytes(resp.into_body()).await;
    assert_eq!(&bytes[..], b"hello world");
}

#[tokio::test]
async fn test_head_file_returns_headers() {
    let (state, _tmp) = setup();
    let repo_id = create_test_repo(&state, "file-head").await;
    upload_test_file(&state, repo_id, "head.txt", b"test content").await;

    let app = build_router(state);
    let (key, val) = auth_header();
    let req = Request::builder()
        .method("HEAD")
        .uri(format!("/api/v1/repos/{}/files/head.txt", repo_id))
        .header(key, val)
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert!(resp.headers().get("etag").is_some());
    assert!(resp.headers().get("content-type").is_some());
    assert_eq!(
        resp.headers().get("content-length").unwrap().to_str().unwrap(),
        "12"
    );
}

#[tokio::test]
async fn test_download_with_matching_etag_returns_304() {
    let (state, _tmp) = setup();
    let repo_id = create_test_repo(&state, "etag-test").await;
    upload_test_file(&state, repo_id, "etag.txt", b"etag content").await;

    // First download to get ETag
    let app = build_router(state.clone());
    let (key, val) = auth_header();
    let req = Request::builder()
        .uri(format!("/api/v1/repos/{}/files/etag.txt", repo_id))
        .header(key, val)
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let etag = resp.headers().get("etag").unwrap().to_str().unwrap().to_string();

    // Second request with If-None-Match
    let app = build_router(state);
    let (key, val) = auth_header();
    let req = Request::builder()
        .uri(format!("/api/v1/repos/{}/files/etag.txt", repo_id))
        .header(key, val)
        .header("If-None-Match", &etag)
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_MODIFIED);
}

#[tokio::test]
async fn test_delete_file_returns_204() {
    let (state, _tmp) = setup();
    let repo_id = create_test_repo(&state, "file-delete").await;
    upload_test_file(&state, repo_id, "del.txt", b"delete me").await;

    let app = build_router(state);
    let (key, val) = auth_header();
    let req = Request::builder()
        .method("DELETE")
        .uri(format!("/api/v1/repos/{}/files/del.txt", repo_id))
        .header(key, val)
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn test_list_files() {
    let (state, _tmp) = setup();
    let repo_id = create_test_repo(&state, "file-list").await;
    upload_test_file(&state, repo_id, "a.txt", b"aaa").await;
    upload_test_file(&state, repo_id, "b.txt", b"bbb").await;

    let app = build_router(state);
    let (key, val) = auth_header();
    let req = Request::builder()
        .uri(format!("/api/v1/repos/{}/files", repo_id))
        .header(key, val)
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body: Value = body_to_json(resp.into_body()).await;
    let files = body["data"]["files"].as_array().unwrap();
    assert_eq!(files.len(), 2);
}

#[tokio::test]
async fn test_move_file() {
    let (state, _tmp) = setup();
    let repo_id = create_test_repo(&state, "file-move").await;
    upload_test_file(&state, repo_id, "src.txt", b"move me").await;

    // Move the file
    let app = build_router(state.clone());
    let (key, val) = auth_header();
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/repos/{}/files-move", repo_id))
        .header(key, val)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"source":"src.txt","destination":"dst.txt"}"#))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body: Value = body_to_json(resp.into_body()).await;
    assert_eq!(body["data"]["path"], "dst.txt");

    // Source should be gone
    let app = build_router(state.clone());
    let (key, val) = auth_header();
    let req = Request::builder()
        .uri(format!("/api/v1/repos/{}/files/src.txt", repo_id))
        .header(key, val)
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    // Destination should exist
    let app = build_router(state);
    let (key, val) = auth_header();
    let req = Request::builder()
        .uri(format!("/api/v1/repos/{}/files/dst.txt", repo_id))
        .header(key, val)
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_copy_file() {
    let (state, _tmp) = setup();
    let repo_id = create_test_repo(&state, "file-copy").await;
    upload_test_file(&state, repo_id, "original.txt", b"copy me").await;

    // Copy the file
    let app = build_router(state.clone());
    let (key, val) = auth_header();
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/repos/{}/files-copy", repo_id))
        .header(key, val)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            r#"{"source":"original.txt","destination":"copy.txt"}"#,
        ))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Both should exist
    let app = build_router(state.clone());
    let (key, val) = auth_header();
    let req = Request::builder()
        .uri(format!("/api/v1/repos/{}/files/original.txt", repo_id))
        .header(key, val)
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let app = build_router(state);
    let (key, val) = auth_header();
    let req = Request::builder()
        .uri(format!("/api/v1/repos/{}/files/copy.txt", repo_id))
        .header(key, val)
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

// ==================== Shell Tests ====================

#[tokio::test]
async fn test_exec_allowed_command() {
    let (state, _tmp) = setup();
    let repo_id = create_test_repo(&state, "exec-test").await;

    let app = build_router(state);
    let (key, val) = auth_header();
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/repos/{}/exec", repo_id))
        .header(key, val)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::to_string(&json!({
                "command": "ls",
                "args": []
            }))
            .unwrap(),
        ))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body: Value = body_to_json(resp.into_body()).await;
    assert!(body["data"]["exit_code"].is_number());
    assert!(body["data"]["stdout"].is_string());
    assert!(body["data"]["stderr"].is_string());
    assert!(body["data"]["duration_ms"].is_number());
}

#[tokio::test]
async fn test_exec_disallowed_command_returns_403() {
    let (state, _tmp) = setup();
    let repo_id = create_test_repo(&state, "exec-forbidden").await;

    let app = build_router(state);
    let (key, val) = auth_header();
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/repos/{}/exec", repo_id))
        .header(key, val)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::to_string(&json!({
                "command": "rm",
                "args": ["-rf", "/"]
            }))
            .unwrap(),
        ))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

// ==================== Archive Tests ====================

#[tokio::test]
async fn test_create_archive() {
    let (state, _tmp) = setup();
    let repo_id = create_test_repo(&state, "archive-test").await;
    upload_test_file(&state, repo_id, "archive-file.txt", b"archive me").await;

    let app = build_router(state);
    let (key, val) = auth_header();
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/repos/{}/archive", repo_id))
        .header(key, val)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"format":"tar.gz"}"#))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        resp.headers().get("content-type").unwrap().to_str().unwrap(),
        "application/gzip"
    );

    let bytes = body_to_bytes(resp.into_body()).await;
    // gzip magic number
    assert!(bytes.len() > 2);
    assert_eq!(bytes[0], 0x1f);
    assert_eq!(bytes[1], 0x8b);
}
