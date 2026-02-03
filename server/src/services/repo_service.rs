use crate::error::AppError;
use crate::models::repo::{CreateRepoRequest, RepoMeta, UpdateRepoRequest};
use crate::persistence::wal::WalEntry;
use crate::state::AppState;
use chrono::Utc;
use std::collections::HashMap;
use uuid::Uuid;

pub async fn create_repo(
    state: &AppState,
    req: CreateRepoRequest,
) -> Result<RepoMeta, AppError> {
    let now = Utc::now();
    let id = Uuid::new_v4();
    let max_size = req
        .max_size_bytes
        .unwrap_or(state.config.default_max_repo_size);

    let repo = RepoMeta {
        id,
        name: req.name.clone(),
        max_size_bytes: max_size,
        current_size_bytes: 0,
        file_count: 0,
        created_at: now,
        updated_at: now,
        last_accessed_at: now,
        default_ttl_seconds: req.default_ttl_seconds,
        tags: HashMap::new(),
    };

    // WAL first
    {
        let mut wal = state.wal.write().await;
        wal.append(&WalEntry::RepoCreated {
            id,
            name: req.name,
            max_size_bytes: max_size,
            default_ttl_seconds: req.default_ttl_seconds,
            created_at: now,
        })
        .map_err(|e| AppError::Internal(format!("WAL write failed: {}", e)))?;
    }

    // Create repo directory
    let repo_dir = state.config.repos_dir().join(id.to_string()).join("files");
    tokio::fs::create_dir_all(&repo_dir)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to create repo dir: {}", e)))?;

    state.repos.insert(id, repo.clone());
    state.files.insert(id, dashmap::DashMap::new());

    Ok(repo)
}

pub async fn list_repos(
    state: &AppState,
    page: u64,
    per_page: u64,
    sort: Option<String>,
) -> Vec<RepoMeta> {
    let mut repos: Vec<RepoMeta> = state.repos.iter().map(|r| r.value().clone()).collect();

    match sort.as_deref() {
        Some("name") => repos.sort_by(|a, b| a.name.cmp(&b.name)),
        Some("created_at") => repos.sort_by(|a, b| b.created_at.cmp(&a.created_at)),
        Some("size") => repos.sort_by(|a, b| b.current_size_bytes.cmp(&a.current_size_bytes)),
        _ => repos.sort_by(|a, b| b.created_at.cmp(&a.created_at)),
    }

    let start = ((page - 1) * per_page) as usize;
    repos.into_iter().skip(start).take(per_page as usize).collect()
}

pub async fn get_repo(state: &AppState, repo_id: Uuid) -> Result<RepoMeta, AppError> {
    state
        .repos
        .get(&repo_id)
        .map(|r| {
            let mut repo = r.value().clone();
            repo.last_accessed_at = Utc::now();
            repo
        })
        .ok_or_else(|| AppError::NotFound(format!("Repository {} not found", repo_id)))
}

pub async fn update_repo(
    state: &AppState,
    repo_id: Uuid,
    req: UpdateRepoRequest,
) -> Result<RepoMeta, AppError> {
    let now = Utc::now();

    // WAL first
    {
        let mut wal = state.wal.write().await;
        wal.append(&WalEntry::RepoUpdated {
            id: repo_id,
            name: req.name.clone(),
            max_size_bytes: req.max_size_bytes,
            default_ttl_seconds: req.default_ttl_seconds,
            tags: req.tags.clone(),
            updated_at: now,
        })
        .map_err(|e| AppError::Internal(format!("WAL write failed: {}", e)))?;
    }

    let mut entry = state
        .repos
        .get_mut(&repo_id)
        .ok_or_else(|| AppError::NotFound(format!("Repository {} not found", repo_id)))?;

    let repo = entry.value_mut();
    if let Some(name) = req.name {
        repo.name = name;
    }
    if let Some(max_size) = req.max_size_bytes {
        repo.max_size_bytes = max_size;
    }
    if let Some(ttl) = req.default_ttl_seconds {
        repo.default_ttl_seconds = ttl;
    }
    if let Some(tags) = req.tags {
        repo.tags = tags;
    }
    repo.updated_at = now;

    Ok(repo.clone())
}

pub async fn delete_repo(state: &AppState, repo_id: Uuid) -> Result<(), AppError> {
    // Check exists
    if !state.repos.contains_key(&repo_id) {
        return Err(AppError::NotFound(format!(
            "Repository {} not found",
            repo_id
        )));
    }

    // WAL first
    {
        let mut wal = state.wal.write().await;
        wal.append(&WalEntry::RepoDeleted { id: repo_id })
            .map_err(|e| AppError::Internal(format!("WAL write failed: {}", e)))?;
    }

    // Remove from in-memory state
    state.repos.remove(&repo_id);
    state.files.remove(&repo_id);

    // Remove from filesystem
    let repo_dir = state.config.repos_dir().join(repo_id.to_string());
    if repo_dir.exists() {
        tokio::fs::remove_dir_all(&repo_dir)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to remove repo dir: {}", e)))?;
    }

    Ok(())
}
