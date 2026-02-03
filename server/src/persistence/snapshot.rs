use crate::models::snapshot::{MetadataSnapshot, SNAPSHOT_VERSION};
use std::path::Path;

pub fn save_snapshot(path: &Path, snapshot: &MetadataSnapshot) -> anyhow::Result<()> {
    let tmp_path = path.with_extension("bin.tmp");
    let data = bincode::serialize(snapshot)?;
    std::fs::write(&tmp_path, &data)?;
    std::fs::rename(&tmp_path, path)?;
    Ok(())
}

pub fn load_snapshot(path: &Path) -> anyhow::Result<Option<MetadataSnapshot>> {
    if !path.exists() {
        return Ok(None);
    }
    let data = std::fs::read(path)?;
    match bincode::deserialize::<MetadataSnapshot>(&data) {
        Ok(snapshot) => {
            if snapshot.version != SNAPSHOT_VERSION {
                tracing::warn!(
                    "Snapshot version mismatch: expected {}, got {}",
                    SNAPSHOT_VERSION,
                    snapshot.version
                );
                return Ok(None);
            }
            Ok(Some(snapshot))
        }
        Err(e) => {
            tracing::error!("Failed to deserialize snapshot: {}", e);
            Ok(None)
        }
    }
}
