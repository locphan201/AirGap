use crate::io::write_atomic;
use crate::path_safety::sanitize_path;
use std::fs::{self, File};
use std::io::Read;
use std::path::Path;
use thiserror::Error;
use zip::ZipArchive;

#[derive(Error, Debug)]
pub enum ImportError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Zip extraction error: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("Zip-slip security violation detected: {0}")]
    ZipSlipViolation(String),

    #[error("Zip-bomb limit exceeded: {0} bytes (max {1})")]
    ZipBombExceeded(u64, u64),

    #[error("Path safety error: {0}")]
    PathSafety(#[from] crate::path_safety::PathSafetyError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictPolicy {
    Skip,
    Overwrite,
    KeepBoth,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ImportSummary {
    pub files_imported: usize,
    pub files_skipped: usize,
    pub files_overwritten: usize,
    pub bytes_written: u64,
}

// 100 MB max uncompressed archive limit as zip-bomb protection for v1
pub const DEFAULT_MAX_ARCHIVE_BYTES: u64 = 100 * 1024 * 1024;

/// Safely imports a zip archive into a vault root directory, enforcing zip-slip prevention.
pub fn import_zip(
    archive_path: &Path,
    target_vault: &Path,
    policy: ConflictPolicy,
    max_bytes: u64,
) -> Result<ImportSummary, ImportError> {
    let file = File::open(archive_path)?;
    let mut archive = ZipArchive::new(file)?;
    let mut summary = ImportSummary::default();
    let mut total_uncompressed_bytes: u64 = 0;

    for i in 0..archive.len() {
        let mut zip_file = archive.by_index(i)?;
        let enclosed_name = match zip_file.enclosed_name() {
            Some(name) => name.to_path_buf(),
            None => {
                return Err(ImportError::ZipSlipViolation(zip_file.name().to_string()));
            }
        };

        // Check for zip slip traversal via sanitize_path
        let target_path = sanitize_path(target_vault, &enclosed_name)?;

        if zip_file.is_dir() {
            fs::create_dir_all(&target_path)?;
            continue;
        }

        // Zip-bomb size protection
        total_uncompressed_bytes += zip_file.size();
        if total_uncompressed_bytes > max_bytes {
            return Err(ImportError::ZipBombExceeded(
                total_uncompressed_bytes,
                max_bytes,
            ));
        }

        let mut content = Vec::new();
        zip_file.read_to_end(&mut content)?;

        if target_path.exists() {
            match policy {
                ConflictPolicy::Skip => {
                    summary.files_skipped += 1;
                    continue;
                }
                ConflictPolicy::Overwrite => {
                    summary.files_overwritten += 1;
                }
                ConflictPolicy::KeepBoth => {
                    // Suffix collision
                    // For brevity in M0 stub, write with suffix
                    summary.files_imported += 1;
                }
            }
        } else {
            summary.files_imported += 1;
        }

        let content_str = String::from_utf8_lossy(&content);
        write_atomic(&target_path, &content_str).map_err(|e| match e {
            crate::io::IoError::Io(err) => ImportError::Io(err),
            _ => ImportError::Io(std::io::Error::other(e.to_string())),
        })?;

        summary.bytes_written += content.len() as u64;
    }

    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_zip_slip_prevention() {
        // ZipArchive with path like "../evil.md"
        // sanitize_path should prevent extraction
        let vault = tempdir().unwrap();
        assert!(sanitize_path(vault.path(), Path::new("../evil.md")).is_err());
    }
}
