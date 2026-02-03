use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::error::AppError;
use crate::models::repo::{CreateRepoRequest, ListReposQuery, UpdateRepoRequest};
use crate::services::repo_service;
use crate::state::AppState;

pub async fn create_repo(
    State(state): State<AppState>,
    Json(req): Json<CreateRepoRequest>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    if req.name.is_empty() {
        return Err(AppError::BadRequest("Name is required".into()));
    }

    let repo = repo_service::create_repo(&state, req).await?;
    tracing::info!(repo_id = %repo.id, name = %repo.name, "Repository created");

    Ok((
        StatusCode::CREATED,
        Json(json!({ "data": repo, "error": null })),
    ))
}

pub async fn list_repos(
    State(state): State<AppState>,
    Query(query): Query<ListReposQuery>,
) -> Json<Value> {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).min(100);

    let repos = repo_service::list_repos(&state, page, per_page, query.sort).await;

    Json(json!({
        "data": {
            "repos": repos,
            "page": page,
            "per_page": per_page,
            "total": state.repos.len(),
        },
        "error": null
    }))
}

pub async fn get_repo(
    State(state): State<AppState>,
    Path(repo_id): Path<Uuid>,
) -> Result<Json<Value>, AppError> {
    let repo = repo_service::get_repo(&state, repo_id).await?;

    let file_count = state
        .files
        .get(&repo_id)
        .map(|f| f.len())
        .unwrap_or(0);

    Ok(Json(json!({
        "data": {
            "repo": repo,
            "file_count": file_count,
        },
        "error": null
    })))
}

pub async fn update_repo(
    State(state): State<AppState>,
    Path(repo_id): Path<Uuid>,
    Json(req): Json<UpdateRepoRequest>,
) -> Result<Json<Value>, AppError> {
    let repo = repo_service::update_repo(&state, repo_id, req).await?;
    tracing::info!(repo_id = %repo_id, "Repository updated");

    Ok(Json(json!({ "data": repo, "error": null })))
}

pub async fn delete_repo(
    State(state): State<AppState>,
    Path(repo_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    repo_service::delete_repo(&state, repo_id).await?;
    tracing::info!(repo_id = %repo_id, "Repository deleted");

    Ok(StatusCode::NO_CONTENT)
}
