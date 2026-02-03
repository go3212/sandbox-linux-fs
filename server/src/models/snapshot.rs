use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use super::file::FileMeta;
use super::repo::RepoMeta;

pub const SNAPSHOT_VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
pub struct MetadataSnapshot {
    pub version: u32,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub repos: HashMap<Uuid, RepoMeta>,
    pub files: HashMap<Uuid, HashMap<String, FileMeta>>,
}
