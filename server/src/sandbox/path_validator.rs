use crate::error::AppError;
use std::path::{Component, Path, PathBuf};

/// Validate and normalize a relative file path.
/// Rejects path traversal attempts and returns a clean relative path.
pub fn validate_relative_path(rel_path: &str) -> Result<String, AppError> {
    let path = Path::new(rel_path);

    // Reject empty paths
    if rel_path.is_empty() {
        return Err(AppError::BadRequest("Empty path".into()));
    }

    // Check each component
    for component in path.components() {
        match component {
            Component::ParentDir => {
                return Err(AppError::Forbidden(
                    "Path traversal not allowed".into(),
                ));
            }
            Component::RootDir | Component::Prefix(_) => {
                // Strip leading slashes, reject Windows prefixes
            }
            Component::Normal(s) => {
                let s = s.to_string_lossy();
                if s.contains('\0') {
                    return Err(AppError::BadRequest(
                        "Null bytes not allowed in path".into(),
                    ));
                }
            }
            Component::CurDir => {
                // Skip "."
            }
        }
    }

    // Build clean relative path
    let clean: PathBuf = path
        .components()
        .filter(|c| matches!(c, Component::Normal(_)))
        .collect();

    let result = clean.to_string_lossy().to_string();
    if result.is_empty() {
        return Err(AppError::BadRequest("Path resolves to empty".into()));
    }

    // Normalize to forward slashes
    Ok(result.replace('\\', "/"))
}

/// Validate that a resolved path is within the given root directory.
#[allow(dead_code)]
pub fn ensure_within_root(root: &Path, resolved: &Path) -> Result<(), AppError> {
    // Use canonical comparison if possible, fall back to starts_with
    let canon_root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let canon_resolved = resolved
        .canonicalize()
        .unwrap_or_else(|_| resolved.to_path_buf());

    if !canon_resolved.starts_with(&canon_root) {
        return Err(AppError::Forbidden(
            "Path escapes repository root".into(),
        ));
    }
    Ok(())
}
