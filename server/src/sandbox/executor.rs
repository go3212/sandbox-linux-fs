use crate::error::AppError;
use crate::services::shell_service::ExecResponse;
use std::path::Path;
use std::time::Instant;
use tokio::process::Command;

pub async fn run_command(
    command: &str,
    args: &[String],
    working_dir: &Path,
    timeout_secs: u64,
    max_output_bytes: usize,
) -> Result<ExecResponse, AppError> {
    let start = Instant::now();

    let mut cmd = Command::new(command);
    cmd.args(args)
        .current_dir(working_dir)
        .env_clear()
        .env("PATH", "/usr/bin:/bin:/usr/local/bin")
        .env("HOME", "/tmp")
        .env("LC_ALL", "C.UTF-8")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    let child = cmd
        .spawn()
        .map_err(|e| AppError::Internal(format!("Failed to spawn command: {}", e)))?;

    // Wait with timeout
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(timeout_secs),
        child.wait_with_output(),
    )
    .await;

    let duration_ms = start.elapsed().as_millis() as u64;

    match result {
        Ok(Ok(output)) => {
            let mut truncated = false;

            let stdout = if output.stdout.len() > max_output_bytes {
                truncated = true;
                String::from_utf8_lossy(&output.stdout[..max_output_bytes]).to_string()
            } else {
                String::from_utf8_lossy(&output.stdout).to_string()
            };

            let stderr = if output.stderr.len() > max_output_bytes {
                truncated = true;
                String::from_utf8_lossy(&output.stderr[..max_output_bytes]).to_string()
            } else {
                String::from_utf8_lossy(&output.stderr).to_string()
            };

            Ok(ExecResponse {
                exit_code: output.status.code().unwrap_or(-1),
                stdout,
                stderr,
                duration_ms,
                truncated,
            })
        }
        Ok(Err(e)) => Err(AppError::Internal(format!("Command execution failed: {}", e))),
        Err(_) => {
            // Timeout
            Ok(ExecResponse {
                exit_code: -1,
                stdout: String::new(),
                stderr: "Command timed out".to_string(),
                duration_ms,
                truncated: false,
            })
        }
    }
}
