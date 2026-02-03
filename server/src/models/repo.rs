use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoMeta {
    pub id: Uuid,
    pub name: String,
    pub max_size_bytes: u64,
    pub current_size_bytes: u64,
    pub file_count: u64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_accessed_at: DateTime<Utc>,
    pub default_ttl_seconds: Option<u64>,
    pub tags: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateRepoRequest {
    pub name: String,
    pub max_size_bytes: Option<u64>,
    pub default_ttl_seconds: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateRepoRequest {
    pub name: Option<String>,
    pub max_size_bytes: Option<u64>,
    pub default_ttl_seconds: Option<Option<u64>>,
    pub tags: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize)]
pub struct ListReposQuery {
    pub page: Option<u64>,
    pub per_page: Option<u64>,
    pub sort: Option<String>,
}
