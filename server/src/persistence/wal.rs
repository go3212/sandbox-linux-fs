use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum WalEntry {
    RepoCreated {
        id: Uuid,
        name: String,
        max_size_bytes: u64,
        default_ttl_seconds: Option<u64>,
        created_at: DateTime<Utc>,
    },
    RepoUpdated {
        id: Uuid,
        name: Option<String>,
        max_size_bytes: Option<u64>,
        default_ttl_seconds: Option<Option<u64>>,
        tags: Option<HashMap<String, String>>,
        updated_at: DateTime<Utc>,
    },
    RepoDeleted {
        id: Uuid,
    },
    RepoSizeChanged {
        id: Uuid,
        current_size_bytes: u64,
        file_count: u64,
    },
    FileCreated {
        repo_id: Uuid,
        path: String,
        size_bytes: u64,
        etag: String,
        content_type: String,
        created_at: DateTime<Utc>,
        expires_at: Option<DateTime<Utc>>,
    },
    FileDeleted {
        repo_id: Uuid,
        path: String,
    },
    FileMoved {
        repo_id: Uuid,
        source: String,
        destination: String,
        updated_at: DateTime<Utc>,
    },
}

pub struct WalWriter {
    dir: PathBuf,
    file: Option<std::fs::File>,
    entry_count: u64,
}

impl WalWriter {
    pub fn open(dir: &Path) -> anyhow::Result<Self> {
        std::fs::create_dir_all(dir)?;
        let wal_path = dir.join("current.wal");
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&wal_path)?;
        Ok(Self {
            dir: dir.to_path_buf(),
            file: Some(file),
            entry_count: 0,
        })
    }

    pub fn append(&mut self, entry: &WalEntry) -> anyhow::Result<()> {
        let data = bincode::serialize(entry)?;
        let len = data.len() as u32;
        if let Some(ref mut f) = self.file {
            f.write_all(&len.to_le_bytes())?;
            f.write_all(&data)?;
            f.flush()?;
            self.entry_count += 1;
        }
        Ok(())
    }

    pub fn truncate(&mut self) -> anyhow::Result<()> {
        let wal_path = self.dir.join("current.wal");
        if let Some(ref mut f) = self.file {
            drop(std::mem::replace(
                f,
                std::fs::OpenOptions::new()
                    .create(true)
                    .write(true)
                    .truncate(true)
                    .open(&wal_path)?,
            ));
            self.entry_count = 0;
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub fn flush(&mut self) -> anyhow::Result<()> {
        if let Some(ref mut f) = self.file {
            f.flush()?;
        }
        Ok(())
    }

    pub fn read_entries(dir: &Path) -> anyhow::Result<Vec<WalEntry>> {
        let wal_path = dir.join("current.wal");
        if !wal_path.exists() {
            return Ok(Vec::new());
        }
        let data = std::fs::read(&wal_path)?;
        let mut entries = Vec::new();
        let mut cursor = 0;
        while cursor + 4 <= data.len() {
            let len =
                u32::from_le_bytes(data[cursor..cursor + 4].try_into().unwrap()) as usize;
            cursor += 4;
            if cursor + len > data.len() {
                tracing::warn!("WAL truncated at entry boundary, stopping replay");
                break;
            }
            match bincode::deserialize::<WalEntry>(&data[cursor..cursor + len]) {
                Ok(entry) => entries.push(entry),
                Err(e) => {
                    tracing::warn!("WAL entry corrupt, stopping replay: {}", e);
                    break;
                }
            }
            cursor += len;
        }
        Ok(entries)
    }
}
