use crate::error::AppError;
use crate::sandbox::command_whitelist;
use crate::sandbox::executor;
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct ExecRequest {
    pub command: String,
    pub args: Vec<String>,
    pub timeout_seconds: Option<u64>,
    pub max_output_bytes: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct ExecResponse {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
    pub truncated: bool,
}

pub async fn execute_command(
    state: &AppState,
    repo_id: Uuid,
    req: ExecRequest,
) -> Result<ExecResponse, AppError> {
    // Validate repo exists
    if !state.repos.contains_key(&repo_id) {
        return Err(AppError::NotFound(format!(
            "Repository {} not found",
            repo_id
        )));
    }

    // Validate command is whitelisted
    if !command_whitelist::is_allowed(&req.command) {
        return Err(AppError::Forbidden(format!(
            "Command '{}' is not allowed",
            req.command
        )));
    }

    // Validate arguments
    command_whitelist::validate_args(&req.args)?;

    let repo_root = state
        .config
        .repos_dir()
        .join(repo_id.to_string())
        .join("files");

    let timeout = req
        .timeout_seconds
        .unwrap_or(state.config.command_timeout_secs);
    let max_output = req
        .max_output_bytes
        .unwrap_or(state.config.command_max_output_bytes);

    // Acquire semaphore permit
    let _permit = state
        .command_semaphore
        .acquire()
        .await
        .map_err(|_| AppError::Internal("Command semaphore closed".into()))?;

    // Execute
    executor::run_command(&req.command, &req.args, &repo_root, timeout, max_output).await
}
