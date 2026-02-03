use crate::error::AppError;
use crate::models::file::FileMeta;
use crate::persistence::wal::WalEntry;
use crate::state::AppState;
use chrono::{Duration, Utc};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

fn repo_files_dir(state: &AppState, repo_id: Uuid) -> PathBuf {
    state
        .config
        .repos_dir()
        .join(repo_id.to_string())
        .join("files")
}

pub fn resolve_file_path(state: &AppState, repo_id: Uuid, rel_path: &str) -> PathBuf {
    repo_files_dir(state, repo_id).join(rel_path)
}

pub async fn upload_file(
    state: &AppState,
    repo_id: Uuid,
    rel_path: &str,
    data: bytes::Bytes,
    ttl_seconds: Option<u64>,
) -> Result<FileMeta, AppError> {
    // Check repo exists
    let default_ttl = {
        let repo = state
            .repos
            .get(&repo_id)
            .ok_or_else(|| AppError::NotFound(format!("Repository {} not found", repo_id)))?;
        repo.default_ttl_seconds
    };

    let file_size = data.len() as u64;

    // Check size limits
    {
        let repo = state.repos.get(&repo_id).unwrap();
        if file_size > state.config.max_upload_size {
            return Err(AppError::PayloadTooLarge(format!(
                "File size {} exceeds max upload size {}",
                file_size, state.config.max_upload_size
            )));
        }

        // Check if existing file - we'll subtract its size
        let existing_size = state
            .files
            .get(&repo_id)
            .and_then(|files| files.get(rel_path).map(|f| f.size_bytes))
            .unwrap_or(0);

        let new_total = repo.current_size_bytes - existing_size + file_size;
        if new_total > repo.max_size_bytes {
            // Try eviction
            let needed = new_total - repo.max_size_bytes;
            let freed =
                crate::services::eviction_service::evict_bytes(state, repo_id, needed).await?;
            if freed < needed {
                return Err(AppError::PayloadTooLarge(format!(
                    "Repository size limit exceeded. Need {} more bytes",
                    needed - freed
                )));
            }
        }
    }

    // Compute hash
    let mut hasher = Sha256::new();
    hasher.update(&data);
    let etag = hex::encode(hasher.finalize());

    // Content type
    let content_type = mime_guess::from_path(rel_path)
        .first_or_octet_stream()
        .to_string();

    let now = Utc::now();
    let ttl = ttl_seconds.or(default_ttl);
    let expires_at = ttl.map(|s| now + Duration::seconds(s as i64));

    // Write file to disk
    let file_path = resolve_file_path(state, repo_id, rel_path);
    if let Some(parent) = file_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let mut file = tokio::fs::File::create(&file_path).await?;
    file.write_all(&data).await?;
    file.flush().await?;

    let meta = FileMeta {
        repo_id,
        path: rel_path.to_string(),
        size_bytes: file_size,
        etag: etag.clone(),
        content_type: content_type.clone(),
        created_at: now,
        updated_at: now,
        last_accessed_at: now,
        access_count: 0,
        expires_at,
    };

    // WAL
    {
        let mut wal = state.wal.write().await;
        wal.append(&WalEntry::FileCreated {
            repo_id,
            path: rel_path.to_string(),
            size_bytes: file_size,
            etag,
            content_type,
            created_at: now,
            expires_at,
        })
        .map_err(|e| AppError::Internal(format!("WAL write failed: {}", e)))?;
    }

    // Update in-memory state
    let old_size = state
        .files
        .get(&repo_id)
        .and_then(|files| files.get(rel_path).map(|f| f.size_bytes))
        .unwrap_or(0);

    let is_new = !state
        .files
        .get(&repo_id)
        .map(|f| f.contains_key(rel_path))
        .unwrap_or(false);

    state
        .files
        .entry(repo_id)
        .or_insert_with(dashmap::DashMap::new)
        .insert(rel_path.to_string(), meta.clone());

    // Update repo size
    if let Some(mut repo) = state.repos.get_mut(&repo_id) {
        repo.current_size_bytes = repo.current_size_bytes - old_size + file_size;
        if is_new {
            repo.file_count += 1;
        }
        repo.updated_at = now;
    }

    Ok(meta)
}

pub async fn download_file(
    state: &AppState,
    repo_id: Uuid,
    rel_path: &str,
) -> Result<(FileMeta, PathBuf), AppError> {
    // Check repo exists
    if !state.repos.contains_key(&repo_id) {
        return Err(AppError::NotFound(format!(
            "Repository {} not found",
            repo_id
        )));
    }

    let meta = state
        .files
        .get(&repo_id)
        .and_then(|files| files.get(rel_path).map(|f| f.clone()))
        .ok_or_else(|| AppError::NotFound(format!("File not found: {}", rel_path)))?;

    // Update access stats
    if let Some(files) = state.files.get(&repo_id) {
        if let Some(mut file) = files.get_mut(rel_path) {
            file.last_accessed_at = Utc::now();
            file.access_count += 1;
        }
    }

    let file_path = resolve_file_path(state, repo_id, rel_path);
    if !file_path.exists() {
        return Err(AppError::NotFound(format!("File not found on disk: {}", rel_path)));
    }

    Ok((meta, file_path))
}

pub async fn head_file(
    state: &AppState,
    repo_id: Uuid,
    rel_path: &str,
) -> Result<FileMeta, AppError> {
    if !state.repos.contains_key(&repo_id) {
        return Err(AppError::NotFound(format!(
            "Repository {} not found",
            repo_id
        )));
    }

    state
        .files
        .get(&repo_id)
        .and_then(|files| files.get(rel_path).map(|f| f.clone()))
        .ok_or_else(|| AppError::NotFound(format!("File not found: {}", rel_path)))
}

pub async fn delete_file(
    state: &AppState,
    repo_id: Uuid,
    rel_path: &str,
) -> Result<(), AppError> {
    if !state.repos.contains_key(&repo_id) {
        return Err(AppError::NotFound(format!(
            "Repository {} not found",
            repo_id
        )));
    }

    let file_size = state
        .files
        .get(&repo_id)
        .and_then(|files| files.get(rel_path).map(|f| f.size_bytes))
        .ok_or_else(|| AppError::NotFound(format!("File not found: {}", rel_path)))?;

    // WAL
    {
        let mut wal = state.wal.write().await;
        wal.append(&WalEntry::FileDeleted {
            repo_id,
            path: rel_path.to_string(),
        })
        .map_err(|e| AppError::Internal(format!("WAL write failed: {}", e)))?;
    }

    // Remove from memory
    if let Some(files) = state.files.get(&repo_id) {
        files.remove(rel_path);
    }

    // Update repo stats
    if let Some(mut repo) = state.repos.get_mut(&repo_id) {
        repo.current_size_bytes = repo.current_size_bytes.saturating_sub(file_size);
        repo.file_count = repo.file_count.saturating_sub(1);
        repo.updated_at = Utc::now();
    }

    // Remove from disk
    let file_path = resolve_file_path(state, repo_id, rel_path);
    if file_path.exists() {
        tokio::fs::remove_file(&file_path).await?;
        // Clean up empty parent dirs
        cleanup_empty_dirs(&repo_files_dir(state, repo_id), &file_path).await;
    }

    Ok(())
}

async fn cleanup_empty_dirs(root: &Path, file_path: &Path) {
    let mut dir = file_path.parent();
    while let Some(d) = dir {
        if d == root {
            break;
        }
        match tokio::fs::read_dir(d).await {
            Ok(mut entries) => {
                if entries.next_entry().await.ok().flatten().is_none() {
                    let _ = tokio::fs::remove_dir(d).await;
                } else {
                    break;
                }
            }
            Err(_) => break,
        }
        dir = d.parent();
    }
}

pub async fn list_files(
    state: &AppState,
    repo_id: Uuid,
    prefix: Option<String>,
    recursive: bool,
    page: u64,
    per_page: u64,
) -> Result<Vec<FileMeta>, AppError> {
    if !state.repos.contains_key(&repo_id) {
        return Err(AppError::NotFound(format!(
            "Repository {} not found",
            repo_id
        )));
    }

    let files_map = state
        .files
        .get(&repo_id)
        .ok_or_else(|| AppError::NotFound(format!("Repository {} not found", repo_id)))?;

    let mut files: Vec<FileMeta> = files_map
        .iter()
        .filter(|entry| {
            let path = entry.key();
            if let Some(ref pfx) = prefix {
                if !path.starts_with(pfx) {
                    return false;
                }
            }
            if !recursive {
                let rel = if let Some(ref pfx) = prefix {
                    path.strip_prefix(pfx).unwrap_or(path)
                } else {
                    path.as_str()
                };
                // Only include direct children (no more slashes)
                !rel.trim_start_matches('/').contains('/')
            } else {
                true
            }
        })
        .map(|entry| entry.value().clone())
        .collect();

    files.sort_by(|a, b| a.path.cmp(&b.path));

    let start = ((page - 1) * per_page) as usize;
    Ok(files.into_iter().skip(start).take(per_page as usize).collect())
}

pub async fn move_file(
    state: &AppState,
    repo_id: Uuid,
    source: &str,
    destination: &str,
) -> Result<FileMeta, AppError> {
    if !state.repos.contains_key(&repo_id) {
        return Err(AppError::NotFound(format!(
            "Repository {} not found",
            repo_id
        )));
    }

    let now = Utc::now();

    // Get source file
    let mut meta = state
        .files
        .get(&repo_id)
        .and_then(|files| files.get(source).map(|f| f.clone()))
        .ok_or_else(|| AppError::NotFound(format!("Source file not found: {}", source)))?;

    // Check destination doesn't exist
    if state
        .files
        .get(&repo_id)
        .map(|f| f.contains_key(destination))
        .unwrap_or(false)
    {
        return Err(AppError::Conflict(format!(
            "Destination already exists: {}",
            destination
        )));
    }

    // WAL
    {
        let mut wal = state.wal.write().await;
        wal.append(&WalEntry::FileMoved {
            repo_id,
            source: source.to_string(),
            destination: destination.to_string(),
            updated_at: now,
        })
        .map_err(|e| AppError::Internal(format!("WAL write failed: {}", e)))?;
    }

    // Move on disk
    let src_path = resolve_file_path(state, repo_id, source);
    let dst_path = resolve_file_path(state, repo_id, destination);
    if let Some(parent) = dst_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::rename(&src_path, &dst_path).await?;

    // Update in-memory
    if let Some(files) = state.files.get(&repo_id) {
        files.remove(source);
    }
    meta.path = destination.to_string();
    meta.updated_at = now;
    state
        .files
        .entry(repo_id)
        .or_insert_with(dashmap::DashMap::new)
        .insert(destination.to_string(), meta.clone());

    // Cleanup empty dirs
    cleanup_empty_dirs(
        &repo_files_dir(state, repo_id),
        &src_path,
    )
    .await;

    Ok(meta)
}

pub async fn copy_file(
    state: &AppState,
    repo_id: Uuid,
    source: &str,
    destination: &str,
) -> Result<FileMeta, AppError> {
    if !state.repos.contains_key(&repo_id) {
        return Err(AppError::NotFound(format!(
            "Repository {} not found",
            repo_id
        )));
    }

    let now = Utc::now();

    // Get source file
    let src_meta = state
        .files
        .get(&repo_id)
        .and_then(|files| files.get(source).map(|f| f.clone()))
        .ok_or_else(|| AppError::NotFound(format!("Source file not found: {}", source)))?;

    // Check destination doesn't exist
    if state
        .files
        .get(&repo_id)
        .map(|f| f.contains_key(destination))
        .unwrap_or(false)
    {
        return Err(AppError::Conflict(format!(
            "Destination already exists: {}",
            destination
        )));
    }

    // Check size
    {
        let repo = state.repos.get(&repo_id).unwrap();
        if repo.current_size_bytes + src_meta.size_bytes > repo.max_size_bytes {
            return Err(AppError::PayloadTooLarge(
                "Repository size limit would be exceeded by copy".into(),
            ));
        }
    }

    // Copy on disk
    let src_path = resolve_file_path(state, repo_id, source);
    let dst_path = resolve_file_path(state, repo_id, destination);
    if let Some(parent) = dst_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::copy(&src_path, &dst_path).await?;

    let meta = FileMeta {
        repo_id,
        path: destination.to_string(),
        size_bytes: src_meta.size_bytes,
        etag: src_meta.etag.clone(),
        content_type: src_meta.content_type.clone(),
        created_at: now,
        updated_at: now,
        last_accessed_at: now,
        access_count: 0,
        expires_at: src_meta.expires_at,
    };

    // WAL
    {
        let mut wal = state.wal.write().await;
        wal.append(&WalEntry::FileCreated {
            repo_id,
            path: destination.to_string(),
            size_bytes: meta.size_bytes,
            etag: meta.etag.clone(),
            content_type: meta.content_type.clone(),
            created_at: now,
            expires_at: meta.expires_at,
        })
        .map_err(|e| AppError::Internal(format!("WAL write failed: {}", e)))?;
    }

    // Update in-memory
    state
        .files
        .entry(repo_id)
        .or_insert_with(dashmap::DashMap::new)
        .insert(destination.to_string(), meta.clone());

    // Update repo size
    if let Some(mut repo) = state.repos.get_mut(&repo_id) {
        repo.current_size_bytes += meta.size_bytes;
        repo.file_count += 1;
        repo.updated_at = now;
    }

    Ok(meta)
}
