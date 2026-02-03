use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use bytes::Bytes;
use serde_json::{json, Value};
use tokio_util::io::ReaderStream;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::file::{CopyFileRequest, ListFilesQuery, MoveFileRequest};
use crate::sandbox::path_validator;
use crate::services::file_service;
use crate::state::AppState;

pub async fn upload_file(
    State(state): State<AppState>,
    Path((repo_id, file_path)): Path<(Uuid, String)>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<(StatusCode, HeaderMap, Json<Value>), AppError> {
    let rel_path = path_validator::validate_relative_path(&file_path)?;

    let ttl: Option<u64> = headers
        .get("X-File-TTL")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse().ok());

    let meta = file_service::upload_file(&state, repo_id, &rel_path, body, ttl).await?;

    tracing::info!(
        repo_id = %repo_id,
        path = %rel_path,
        size = meta.size_bytes,
        "File uploaded"
    );

    let mut resp_headers = HeaderMap::new();
    resp_headers.insert("ETag", format!("\"{}\"", meta.etag).parse().unwrap());

    Ok((
        StatusCode::CREATED,
        resp_headers,
        Json(json!({ "data": meta, "error": null })),
    ))
}

pub async fn download_file(
    State(state): State<AppState>,
    Path((repo_id, file_path)): Path<(Uuid, String)>,
    headers: HeaderMap,
) -> Result<axum::response::Response, AppError> {
    let rel_path = path_validator::validate_relative_path(&file_path)?;

    let (meta, disk_path) = file_service::download_file(&state, repo_id, &rel_path).await?;

    // Check If-None-Match
    if let Some(inm) = headers.get("If-None-Match").and_then(|v| v.to_str().ok()) {
        let etag_val = format!("\"{}\"", meta.etag);
        if inm == etag_val || inm == meta.etag {
            return Ok(axum::response::Response::builder()
                .status(StatusCode::NOT_MODIFIED)
                .body(Body::empty())
                .unwrap());
        }
    }

    let file = tokio::fs::File::open(&disk_path).await?;
    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    let response = axum::response::Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", &meta.content_type)
        .header("Content-Length", meta.size_bytes.to_string())
        .header("ETag", format!("\"{}\"", meta.etag))
        .header("Cache-Control", "no-cache")
        .header(
            "Last-Modified",
            meta.updated_at.format("%a, %d %b %Y %H:%M:%S GMT").to_string(),
        )
        .body(body)
        .unwrap();

    Ok(response)
}

pub async fn head_file(
    State(state): State<AppState>,
    Path((repo_id, file_path)): Path<(Uuid, String)>,
) -> Result<axum::response::Response, AppError> {
    let rel_path = path_validator::validate_relative_path(&file_path)?;
    let meta = file_service::head_file(&state, repo_id, &rel_path).await?;

    let response = axum::response::Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", &meta.content_type)
        .header("Content-Length", meta.size_bytes.to_string())
        .header("ETag", format!("\"{}\"", meta.etag))
        .header("Cache-Control", "no-cache")
        .header(
            "Last-Modified",
            meta.updated_at.format("%a, %d %b %Y %H:%M:%S GMT").to_string(),
        )
        .body(Body::empty())
        .unwrap();

    Ok(response)
}

pub async fn delete_file(
    State(state): State<AppState>,
    Path((repo_id, file_path)): Path<(Uuid, String)>,
) -> Result<StatusCode, AppError> {
    let rel_path = path_validator::validate_relative_path(&file_path)?;
    file_service::delete_file(&state, repo_id, &rel_path).await?;
    tracing::info!(repo_id = %repo_id, path = %rel_path, "File deleted");
    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_files(
    State(state): State<AppState>,
    Path(repo_id): Path<Uuid>,
    Query(query): Query<ListFilesQuery>,
) -> Result<Json<Value>, AppError> {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(100).min(1000);
    let recursive = query.recursive.unwrap_or(true);

    let files =
        file_service::list_files(&state, repo_id, query.prefix, recursive, page, per_page)
            .await?;

    Ok(Json(json!({
        "data": {
            "files": files,
            "page": page,
            "per_page": per_page,
        },
        "error": null
    })))
}

pub async fn move_file(
    State(state): State<AppState>,
    Path(repo_id): Path<Uuid>,
    Json(req): Json<MoveFileRequest>,
) -> Result<Json<Value>, AppError> {
    let source = path_validator::validate_relative_path(&req.source)?;
    let destination = path_validator::validate_relative_path(&req.destination)?;

    let meta = file_service::move_file(&state, repo_id, &source, &destination).await?;
    tracing::info!(
        repo_id = %repo_id,
        source = %source,
        destination = %destination,
        "File moved"
    );

    Ok(Json(json!({ "data": meta, "error": null })))
}

pub async fn copy_file(
    State(state): State<AppState>,
    Path(repo_id): Path<Uuid>,
    Json(req): Json<CopyFileRequest>,
) -> Result<Json<Value>, AppError> {
    let source = path_validator::validate_relative_path(&req.source)?;
    let destination = path_validator::validate_relative_path(&req.destination)?;

    let meta = file_service::copy_file(&state, repo_id, &source, &destination).await?;
    tracing::info!(
        repo_id = %repo_id,
        source = %source,
        destination = %destination,
        "File copied"
    );

    Ok(Json(json!({ "data": meta, "error": null })))
}
