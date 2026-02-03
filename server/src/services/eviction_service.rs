use crate::error::AppError;
use crate::state::AppState;
use chrono::Utc;
use uuid::Uuid;

/// Evict files from a repo to free at least `needed_bytes`.
/// Returns the number of bytes freed.
pub async fn evict_bytes(
    state: &AppState,
    repo_id: Uuid,
    needed_bytes: u64,
) -> Result<u64, AppError> {
    let files_map = match state.files.get(&repo_id) {
        Some(f) => f,
        None => return Ok(0),
    };

    let now = Utc::now();

    // Score files: score = access_count / age_seconds (higher = more valuable)
    let mut scored: Vec<(String, f64, u64)> = files_map
        .iter()
        .map(|entry| {
            let meta = entry.value();
            let age = now
                .signed_duration_since(meta.created_at)
                .num_seconds()
                .max(1) as f64;
            let score = meta.access_count as f64 / age;
            (meta.path.clone(), score, meta.size_bytes)
        })
        .collect();

    // Sort by score ascending (evict lowest score first)
    scored.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut freed = 0u64;
    for (path, _score, size) in &scored {
        if freed >= needed_bytes {
            break;
        }
        // Delete the file
        drop(files_map);
        crate::services::file_service::delete_file(state, repo_id, path).await?;
        freed += size;
        // Re-acquire the map reference
        if state.files.get(&repo_id).is_none() {
            break;
        }
        return evict_continue(state, repo_id, needed_bytes, freed, scored).await;
    }

    Ok(freed)
}

async fn evict_continue(
    state: &AppState,
    repo_id: Uuid,
    needed_bytes: u64,
    mut freed: u64,
    scored: Vec<(String, f64, u64)>,
) -> Result<u64, AppError> {
    for (path, _score, size) in scored.iter().skip(1) {
        if freed >= needed_bytes {
            break;
        }
        if state.files.get(&repo_id).is_none() {
            break;
        }
        // Check if file still exists (may have been removed already)
        let exists = state
            .files
            .get(&repo_id)
            .map(|f| f.contains_key(path.as_str()))
            .unwrap_or(false);
        if !exists {
            continue;
        }
        crate::services::file_service::delete_file(state, repo_id, path).await?;
        freed += size;
    }
    Ok(freed)
}

/// Proactively check all repos and evict if over limit.
pub async fn evict_over_limit_repos(state: &AppState) {
    let repo_ids: Vec<Uuid> = state.repos.iter().map(|r| *r.key()).collect();

    for repo_id in repo_ids {
        let (current, max) = match state.repos.get(&repo_id) {
            Some(r) => (r.current_size_bytes, r.max_size_bytes),
            None => continue,
        };

        if current > max {
            let needed = current - max;
            match evict_bytes(state, repo_id, needed).await {
                Ok(freed) => {
                    if freed > 0 {
                        tracing::info!(
                            repo_id = %repo_id,
                            freed_bytes = freed,
                            "Proactive eviction completed"
                        );
                    }
                }
                Err(e) => {
                    tracing::error!(
                        repo_id = %repo_id,
                        error = %e,
                        "Proactive eviction failed"
                    );
                }
            }
        }
    }
}
