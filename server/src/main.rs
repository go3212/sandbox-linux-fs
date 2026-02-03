use linux_fs::background;
use linux_fs::config::AppConfig;
use linux_fs::models;
use linux_fs::persistence;
use linux_fs::persistence::wal::WalWriter;
use linux_fs::routes;
use linux_fs::state::AppState;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    // Load .env if present
    let _ = dotenvy::dotenv();

    let config = AppConfig::from_env();

    // Init tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new(&config.log_level)),
        )
        .json()
        .init();

    tracing::info!("Starting linux-fs v{}", env!("CARGO_PKG_VERSION"));

    // Ensure data directories exist
    std::fs::create_dir_all(config.repos_dir()).expect("Failed to create repos dir");
    std::fs::create_dir_all(config.metadata_dir()).expect("Failed to create metadata dir");
    std::fs::create_dir_all(config.wal_dir()).expect("Failed to create WAL dir");

    // Boot recovery: load snapshot, then replay WAL
    let wal_writer =
        WalWriter::open(&config.wal_dir()).expect("Failed to open WAL");

    let state = AppState::new(config.clone(), wal_writer);

    // Load snapshot
    if let Some(snapshot) =
        persistence::snapshot::load_snapshot(&config.snapshot_path())
            .expect("Failed to load snapshot")
    {
        tracing::info!(
            repos = snapshot.repos.len(),
            "Loaded snapshot from {}",
            snapshot.timestamp
        );
        for (id, repo) in snapshot.repos {
            state.repos.insert(id, repo);
        }
        for (repo_id, files) in snapshot.files {
            let map = dashmap::DashMap::new();
            for (path, meta) in files {
                map.insert(path, meta);
            }
            state.files.insert(repo_id, map);
        }
    }

    // Replay WAL
    match persistence::wal::WalWriter::read_entries(&config.wal_dir()) {
        Ok(entries) => {
            if !entries.is_empty() {
                tracing::info!(count = entries.len(), "Replaying WAL entries");
                replay_wal_entries(&state, entries);
            }
        }
        Err(e) => {
            tracing::error!("Failed to read WAL entries: {}", e);
        }
    }

    // Reconcile with filesystem
    reconcile_filesystem(&state).await;

    // Shutdown signal
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

    // Start background tasks
    let ttl_handle = tokio::spawn(background::ttl_reaper::run(
        state.clone(),
        shutdown_rx.clone(),
    ));
    let snapshot_handle = tokio::spawn(background::snapshot_writer::run(
        state.clone(),
        shutdown_rx.clone(),
    ));
    let eviction_handle = tokio::spawn(background::eviction_monitor::run(
        state.clone(),
        shutdown_rx.clone(),
    ));

    // Build router
    let app = routes::build_router(state.clone());

    let addr = format!("{}:{}", config.host, config.port);
    tracing::info!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind");

    // Serve with graceful shutdown
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(shutdown_tx))
        .await
        .expect("Server error");

    // Wait for background tasks
    tracing::info!("Waiting for background tasks to finish");
    let _ = tokio::join!(ttl_handle, snapshot_handle, eviction_handle);

    // Final snapshot
    tracing::info!("Writing final snapshot");
    background::snapshot_writer::write_snapshot(&state).await;

    tracing::info!("Shutdown complete");
}

async fn shutdown_signal(shutdown_tx: tokio::sync::watch::Sender<bool>) {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("Shutdown signal received");
    let _ = shutdown_tx.send(true);
}

fn replay_wal_entries(state: &AppState, entries: Vec<persistence::wal::WalEntry>) {
    use persistence::wal::WalEntry;
    use std::collections::HashMap;

    for entry in entries {
        match entry {
            WalEntry::RepoCreated {
                id,
                name,
                max_size_bytes,
                default_ttl_seconds,
                created_at,
            } => {
                let repo = models::repo::RepoMeta {
                    id,
                    name,
                    max_size_bytes,
                    current_size_bytes: 0,
                    file_count: 0,
                    created_at,
                    updated_at: created_at,
                    last_accessed_at: created_at,
                    default_ttl_seconds,
                    tags: HashMap::new(),
                };
                state.repos.insert(id, repo);
                state.files.entry(id).or_insert_with(dashmap::DashMap::new);
            }
            WalEntry::RepoUpdated {
                id,
                name,
                max_size_bytes,
                default_ttl_seconds,
                tags,
                updated_at,
            } => {
                if let Some(mut repo) = state.repos.get_mut(&id) {
                    if let Some(n) = name {
                        repo.name = n;
                    }
                    if let Some(ms) = max_size_bytes {
                        repo.max_size_bytes = ms;
                    }
                    if let Some(ttl) = default_ttl_seconds {
                        repo.default_ttl_seconds = ttl;
                    }
                    if let Some(t) = tags {
                        repo.tags = t;
                    }
                    repo.updated_at = updated_at;
                }
            }
            WalEntry::RepoDeleted { id } => {
                state.repos.remove(&id);
                state.files.remove(&id);
            }
            WalEntry::RepoSizeChanged {
                id,
                current_size_bytes,
                file_count,
            } => {
                if let Some(mut repo) = state.repos.get_mut(&id) {
                    repo.current_size_bytes = current_size_bytes;
                    repo.file_count = file_count;
                }
            }
            WalEntry::FileCreated {
                repo_id,
                path,
                size_bytes,
                etag,
                content_type,
                created_at,
                expires_at,
            } => {
                let meta = models::file::FileMeta {
                    repo_id,
                    path: path.clone(),
                    size_bytes,
                    etag,
                    content_type,
                    created_at,
                    updated_at: created_at,
                    last_accessed_at: created_at,
                    access_count: 0,
                    expires_at,
                };
                state
                    .files
                    .entry(repo_id)
                    .or_insert_with(dashmap::DashMap::new)
                    .insert(path, meta);

                if let Some(mut repo) = state.repos.get_mut(&repo_id) {
                    repo.current_size_bytes += size_bytes;
                    repo.file_count += 1;
                }
            }
            WalEntry::FileDeleted { repo_id, path } => {
                if let Some(files) = state.files.get(&repo_id) {
                    if let Some((_, meta)) = files.remove(&path) {
                        if let Some(mut repo) = state.repos.get_mut(&repo_id) {
                            repo.current_size_bytes =
                                repo.current_size_bytes.saturating_sub(meta.size_bytes);
                            repo.file_count = repo.file_count.saturating_sub(1);
                        }
                    }
                }
            }
            WalEntry::FileMoved {
                repo_id,
                source,
                destination,
                updated_at,
            } => {
                if let Some(files) = state.files.get(&repo_id) {
                    if let Some((_, mut meta)) = files.remove(&source) {
                        meta.path = destination.clone();
                        meta.updated_at = updated_at;
                        files.insert(destination, meta);
                    }
                }
            }
        }
    }
}

/// Walk the filesystem and reconcile with in-memory state.
/// Remove metadata for files that don't exist on disk.
/// Recompute repo sizes.
async fn reconcile_filesystem(state: &AppState) {
    let repo_ids: Vec<uuid::Uuid> = state.repos.iter().map(|r| *r.key()).collect();

    for repo_id in repo_ids {
        let repo_dir = state
            .config
            .repos_dir()
            .join(repo_id.to_string())
            .join("files");

        if !repo_dir.exists() {
            tracing::warn!(repo_id = %repo_id, "Repo directory missing, cleaning metadata");
            state.repos.remove(&repo_id);
            state.files.remove(&repo_id);
            continue;
        }

        // Check for orphaned metadata entries
        if let Some(files) = state.files.get(&repo_id) {
            let paths: Vec<String> = files.iter().map(|f| f.key().clone()).collect();
            for path in paths {
                let file_path = repo_dir.join(&path);
                if !file_path.exists() {
                    tracing::warn!(
                        repo_id = %repo_id,
                        path = %path,
                        "Orphaned metadata entry, removing"
                    );
                    files.remove(&path);
                }
            }
        }

        // Recompute repo size
        let (total_size, file_count) = state
            .files
            .get(&repo_id)
            .map(|files| {
                let mut size = 0u64;
                let mut count = 0u64;
                for entry in files.iter() {
                    size += entry.value().size_bytes;
                    count += 1;
                }
                (size, count)
            })
            .unwrap_or((0, 0));

        if let Some(mut repo) = state.repos.get_mut(&repo_id) {
            repo.current_size_bytes = total_size;
            repo.file_count = file_count;
        }
    }
}
