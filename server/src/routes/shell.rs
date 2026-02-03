use axum::extract::{Path, State};
use axum::Json;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::error::AppError;
use crate::services::shell_service::{self, ExecRequest};
use crate::state::AppState;

pub async fn exec_command(
    State(state): State<AppState>,
    Path(repo_id): Path<Uuid>,
    Json(req): Json<ExecRequest>,
) -> Result<Json<Value>, AppError> {
    tracing::info!(
        repo_id = %repo_id,
        command = %req.command,
        args = ?req.args,
        "Executing command"
    );

    let response = shell_service::execute_command(&state, repo_id, req).await?;

    Ok(Json(json!({
        "data": response,
        "error": null
    })))
}
