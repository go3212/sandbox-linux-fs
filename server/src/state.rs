use crate::config::AppConfig;
use crate::models::file::FileMeta;
use crate::models::repo::RepoMeta;
use crate::persistence::wal::WalWriter;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, Semaphore};
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub repos: Arc<DashMap<Uuid, RepoMeta>>,
    pub files: Arc<DashMap<Uuid, DashMap<String, FileMeta>>>,
    pub wal: Arc<RwLock<WalWriter>>,
    pub config: Arc<AppConfig>,
    pub command_semaphore: Arc<Semaphore>,
    pub start_time: chrono::DateTime<chrono::Utc>,
}

impl AppState {
    pub fn new(config: AppConfig, wal: WalWriter) -> Self {
        let max_concurrent = config.max_concurrent_commands;
        Self {
            repos: Arc::new(DashMap::new()),
            files: Arc::new(DashMap::new()),
            wal: Arc::new(RwLock::new(wal)),
            config: Arc::new(config),
            command_semaphore: Arc::new(Semaphore::new(max_concurrent)),
            start_time: chrono::Utc::now(),
        }
    }
}
