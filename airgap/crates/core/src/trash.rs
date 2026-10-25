use std::fs;
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TrashError {
    #[error("Failed to move to OS trash: {0}")]
    OsTrashError(String),

    #[error("Failed to move to vault fallback .trash: {0}")]
    FallbackError(#[from] std::io::Error),
}

/// Moves a file to the OS recycle bin using the `trash` crate.
/// If that fails, moves the file into a `.trash/` subdirectory within the vault.
pub fn delete_to_trash(vault_root: &Path, file_path: &Path) -> Result<(), TrashError> {
    if !file_path.exists() {
        return Ok(());
    }

    // Attempt OS trash first
    match trash::delete(file_path) {
        Ok(()) => Ok(()),
        Err(e) => {
            // Fallback to vault .trash/ directory
            let fallback_dir = vault_root.join(".trash");
            if !fallback_dir.exists() {
                fs::create_dir_all(&fallback_dir)?;
            }

            let file_name = file_path
                .file_name()
                .ok_or_else(|| TrashError::OsTrashError(e.to_string()))?;

            let mut dest = fallback_dir.join(file_name);
            if dest.exists() {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let stem = Path::new(file_name)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("item");
                let ext = Path::new(file_name).extension().and_then(|s| s.to_str());
                let timestamped = match ext {
                    Some(ext_str) => format!("{}_{}.{}", stem, now, ext_str),
                    None => format!("{}_{}", stem, now),
                };
                dest = fallback_dir.join(timestamped);
            }

            if fs::rename(file_path, &dest).is_err() {
                // Cross-device link fallback
                if file_path.is_file() {
                    fs::copy(file_path, &dest)?;
                    fs::remove_file(file_path)?;
                } else if file_path.is_dir() {
                    copy_dir_recursive(file_path, &dest)?;
                    fs::remove_dir_all(file_path)?;
                }
            }
            Ok(())
        }
    }
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let entry_type = entry.file_type()?;
        let dest_child = dst.join(entry.file_name());
        if entry_type.is_dir() {
            copy_dir_recursive(&entry.path(), &dest_child)?;
        } else {
            fs::copy(entry.path(), dest_child)?;
        }
    }
    Ok(())
}
