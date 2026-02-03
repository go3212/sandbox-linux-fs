use crate::state::AppState;
use chrono::Utc;
use std::time::Duration;
use tokio::sync::watch;

pub async fn run(state: AppState, mut shutdown: watch::Receiver<bool>) {
    let interval = Duration::from_secs(state.config.ttl_sweep_interval_secs);

    loop {
        tokio::select! {
            _ = tokio::time::sleep(interval) => {}
            _ = shutdown.changed() => {
                tracing::info!("TTL reaper shutting down");
                return;
            }
        }

        let now = Utc::now();
        let repo_ids: Vec<uuid::Uuid> = state.repos.iter().map(|r| *r.key()).collect();

        let mut total_expired = 0u64;

        for repo_id in repo_ids {
            let expired_paths: Vec<String> = state
                .files
                .get(&repo_id)
                .map(|files| {
                    files
                        .iter()
                        .filter(|entry| {
                            entry
                                .value()
                                .expires_at
                                .map(|exp| exp <= now)
                                .unwrap_or(false)
                        })
                        .map(|entry| entry.key().clone())
                        .collect()
                })
                .unwrap_or_default();

            for path in expired_paths {
                match crate::services::file_service::delete_file(&state, repo_id, &path).await {
                    Ok(()) => {
                        total_expired += 1;
                        tracing::debug!(
                            repo_id = %repo_id,
                            path = %path,
                            "Expired file removed"
                        );
                    }
                    Err(e) => {
                        tracing::warn!(
                            repo_id = %repo_id,
                            path = %path,
                            error = %e,
                            "Failed to remove expired file"
                        );
                    }
                }
            }
        }

        if total_expired > 0 {
            tracing::info!(count = total_expired, "TTL reaper sweep completed");
        }
    }
}
