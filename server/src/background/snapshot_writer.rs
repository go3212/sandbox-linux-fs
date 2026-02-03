use crate::models::snapshot::{MetadataSnapshot, SNAPSHOT_VERSION};
use crate::persistence::snapshot::save_snapshot;
use crate::state::AppState;
use chrono::Utc;
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::watch;

pub async fn run(state: AppState, mut shutdown: watch::Receiver<bool>) {
    let interval = Duration::from_secs(state.config.snapshot_interval_secs);

    loop {
        tokio::select! {
            _ = tokio::time::sleep(interval) => {}
            _ = shutdown.changed() => {
                tracing::info!("Snapshot writer shutting down, writing final snapshot");
                write_snapshot(&state).await;
                return;
            }
        }

        write_snapshot(&state).await;
    }
}

pub async fn write_snapshot(state: &AppState) {
    let repos: HashMap<_, _> = state
        .repos
        .iter()
        .map(|r| (*r.key(), r.value().clone()))
        .collect();

    let files: HashMap<_, _> = state
        .files
        .iter()
        .map(|entry| {
            let inner: HashMap<String, _> = entry
                .value()
                .iter()
                .map(|f| (f.key().clone(), f.value().clone()))
                .collect();
            (*entry.key(), inner)
        })
        .collect();

    let snapshot = MetadataSnapshot {
        version: SNAPSHOT_VERSION,
        timestamp: Utc::now(),
        repos,
        files,
    };

    let snapshot_path = state.config.snapshot_path();
    if let Some(parent) = snapshot_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    match save_snapshot(&snapshot_path, &snapshot) {
        Ok(()) => {
            // Truncate WAL after successful snapshot
            let mut wal = state.wal.write().await;
            if let Err(e) = wal.truncate() {
                tracing::error!("Failed to truncate WAL: {}", e);
            }
            tracing::info!("Snapshot written successfully");
        }
        Err(e) => {
            tracing::error!("Failed to write snapshot: {}", e);
        }
    }
}
