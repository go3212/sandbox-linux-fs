use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use flate2::write::GzEncoder;
use flate2::Compression;
use serde::Deserialize;
use uuid::Uuid;

use crate::error::AppError;
use crate::sandbox::path_validator;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct ArchiveRequest {
    pub path: Option<String>,
    pub format: Option<String>,
}

pub async fn create_archive(
    State(state): State<AppState>,
    Path(repo_id): Path<Uuid>,
    Json(req): Json<ArchiveRequest>,
) -> Result<axum::response::Response, AppError> {
    if !state.repos.contains_key(&repo_id) {
        return Err(AppError::NotFound(format!(
            "Repository {} not found",
            repo_id
        )));
    }

    let format = req.format.unwrap_or_else(|| "tar.gz".into());
    if format != "tar.gz" {
        return Err(AppError::BadRequest(
            "Only tar.gz format is currently supported".into(),
        ));
    }

    let base_dir = state
        .config
        .repos_dir()
        .join(repo_id.to_string())
        .join("files");

    let archive_root = if let Some(ref subpath) = req.path {
        let clean = path_validator::validate_relative_path(subpath)?;
        base_dir.join(clean)
    } else {
        base_dir.clone()
    };

    if !archive_root.exists() {
        return Err(AppError::NotFound("Archive path not found".into()));
    }

    // Build tar.gz in memory (for simplicity; could be streamed for very large repos)
    let data = tokio::task::spawn_blocking(move || -> anyhow::Result<Vec<u8>> {
        let buf = Vec::new();
        let encoder = GzEncoder::new(buf, Compression::default());
        let mut tar_builder = tar::Builder::new(encoder);

        if archive_root.is_dir() {
            tar_builder.append_dir_all(".", &archive_root)?;
        } else {
            let name = archive_root
                .file_name()
                .unwrap_or_default()
                .to_string_lossy();
            tar_builder.append_path_with_name(&archive_root, name.as_ref())?;
        }

        let encoder = tar_builder.into_inner()?;
        let buf = encoder.finish()?;
        Ok(buf)
    })
    .await
    .map_err(|e| AppError::Internal(format!("Archive task failed: {}", e)))?
    .map_err(|e| AppError::Internal(format!("Archive creation failed: {}", e)))?;

    let response = axum::response::Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/gzip")
        .header("Content-Length", data.len().to_string())
        .header(
            "Content-Disposition",
            format!("attachment; filename=\"{}.tar.gz\"", repo_id),
        )
        .body(Body::from(data))
        .unwrap();

    Ok(response)
}
