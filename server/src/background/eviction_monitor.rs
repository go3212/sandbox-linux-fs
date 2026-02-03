use crate::services::eviction_service;
use crate::state::AppState;
use std::time::Duration;
use tokio::sync::watch;

pub async fn run(state: AppState, mut shutdown: watch::Receiver<bool>) {
    let interval = Duration::from_secs(300); // 5 minutes

    loop {
        tokio::select! {
            _ = tokio::time::sleep(interval) => {}
            _ = shutdown.changed() => {
                tracing::info!("Eviction monitor shutting down");
                return;
            }
        }

        eviction_service::evict_over_limit_repos(&state).await;
    }
}
