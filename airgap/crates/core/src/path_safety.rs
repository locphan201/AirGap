use std::path::{Component, Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum PathSafetyError {
    #[error("Path attempts directory traversal outside vault root: {0}")]
    TraversalAttempted(String),

    #[error("Filename contains Windows reserved device name: {0}")]
    ReservedName(String),

    #[error("Filename contains illegal characters: {0}")]
    IllegalCharacters(String),

    #[error("Filename cannot end with a period or space: {0}")]
    TrailingPeriodOrSpace(String),

    #[error("Filename cannot be empty")]
    EmptyFilename,

    #[error("Path is absolute and points outside vault: {0}")]
    AbsoluteOutsideVault(String),

    #[error("Canonicalization failed: {0}")]
    CanonicalizationFailed(String),
}

/// Windows reserved device names (case-insensitive, regardless of extension).
const WINDOWS_RESERVED_NAMES: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Characters strictly prohibited in cross-platform filenames (Windows/POSIX safe set).
const ILLEGAL_FILENAME_CHARS: &[char] = &['<', '>', ':', '"', '/', '\\', '|', '?', '*'];

/// Validates that a single filename or directory name adheres to cross-platform safety rules.
pub fn validate_filename(name: &str) -> Result<(), PathSafetyError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(PathSafetyError::EmptyFilename);
    }

    // Check for trailing period or space (illegal on Windows filesystem)
    if name.ends_with('.') || name.ends_with(' ') {
        return Err(PathSafetyError::TrailingPeriodOrSpace(name.to_string()));
    }

    // Check for illegal characters and ASCII control chars (0..=31)
    for c in name.chars() {
        if ILLEGAL_FILENAME_CHARS.contains(&c) || (c as u32) <= 31 {
            return Err(PathSafetyError::IllegalCharacters(format!(
                "Character '{}' (code {}) in '{}'",
                c, c as u32, name
            )));
        }
    }

    // Check Windows reserved base names (e.g. "con.md", "NUL", "com1.txt", "con.bak.md")
    // In Windows, any filename whose first segment before dot is a reserved name is reserved.
    let primary_prefix = name.split('.').next().unwrap_or(name).trim();
    let upper_prefix = primary_prefix.to_ascii_uppercase();
    for &reserved in WINDOWS_RESERVED_NAMES {
        if upper_prefix == reserved {
            return Err(PathSafetyError::ReservedName(name.to_string()));
        }
    }

    let base_stem = Path::new(name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(name);

    let upper_stem = base_stem.to_ascii_uppercase();
    for &reserved in WINDOWS_RESERVED_NAMES {
        if upper_stem == reserved {
            return Err(PathSafetyError::ReservedName(name.to_string()));
        }
    }

    Ok(())
}

/// Sanitizes and validates an untrusted relative or sub-path against a known vault root.
/// Returns the verified absolute path within the vault.
pub fn sanitize_path(vault_root: &Path, untrusted: &Path) -> Result<PathBuf, PathSafetyError> {
    // 1. Check for directory traversal components
    for comp in untrusted.components() {
        match comp {
            Component::ParentDir => {
                return Err(PathSafetyError::TraversalAttempted(
                    untrusted.to_string_lossy().into_owned(),
                ));
            }
            Component::Normal(name) => {
                if let Some(s) = name.to_str() {
                    validate_filename(s)?;
                }
            }
            Component::RootDir | Component::Prefix(_) => {
                // If it's absolute, it must start with the vault root
                if !untrusted.starts_with(vault_root) {
                    return Err(PathSafetyError::AbsoluteOutsideVault(
                        untrusted.to_string_lossy().into_owned(),
                    ));
                }
            }
            Component::CurDir => {}
        }
    }

    let combined = if untrusted.is_absolute() {
        untrusted.to_path_buf()
    } else {
        vault_root.join(untrusted)
    };

    // 2. Canonical vault root boundary enforcement and symlink escape rejection
    if vault_root.exists() {
        if let Ok(canon_vault) = vault_root.canonicalize() {
            let mut check_path = combined.as_path();
            while check_path != vault_root {
                if let Some(parent) = check_path.parent() {
                    if check_path.exists() {
                        if let Ok(canon) = check_path.canonicalize() {
                            if !canon.starts_with(&canon_vault) {
                                return Err(PathSafetyError::TraversalAttempted(format!(
                                    "Path or symlink target escapes vault boundary: {}",
                                    check_path.display()
                                )));
                            }
                        }
                        if check_path.is_symlink() {
                            if let Ok(target) = std::fs::read_link(check_path) {
                                let resolved = if target.is_absolute() {
                                    target
                                } else {
                                    parent.join(target)
                                };
                                if let Ok(canon_target) = resolved.canonicalize() {
                                    if !canon_target.starts_with(&canon_vault) {
                                        return Err(PathSafetyError::TraversalAttempted(format!(
                                            "Symlink target escapes vault: {}",
                                            check_path.display()
                                        )));
                                    }
                                }
                            }
                        }
                    }
                    check_path = parent;
                } else {
                    break;
                }
            }
        }
    }

    Ok(combined)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reserved_names_rejected() {
        assert!(matches!(
            validate_filename("con.md"),
            Err(PathSafetyError::ReservedName(_))
        ));
        assert!(matches!(
            validate_filename("CON"),
            Err(PathSafetyError::ReservedName(_))
        ));
        assert!(matches!(
            validate_filename("aux.txt"),
            Err(PathSafetyError::ReservedName(_))
        ));
        assert!(matches!(
            validate_filename("nul"),
            Err(PathSafetyError::ReservedName(_))
        ));
        assert!(matches!(
            validate_filename("com1.markdown"),
            Err(PathSafetyError::ReservedName(_))
        ));
        assert!(matches!(
            validate_filename("LPT9.md"),
            Err(PathSafetyError::ReservedName(_))
        ));
        assert!(matches!(
            validate_filename("con.bak.md"),
            Err(PathSafetyError::ReservedName(_))
        ));
        assert!(matches!(
            validate_filename("aux.backup.tar.gz"),
            Err(PathSafetyError::ReservedName(_))
        ));
        assert!(matches!(
            validate_filename("NUL.old.txt"),
            Err(PathSafetyError::ReservedName(_))
        ));
        assert!(validate_filename("context.md").is_ok());
        assert!(validate_filename("auxiliary.txt").is_ok());
    }

    #[test]
    fn test_illegal_characters_rejected() {
        assert!(matches!(
            validate_filename("note<1>.md"),
            Err(PathSafetyError::IllegalCharacters(_))
        ));
        assert!(matches!(
            validate_filename("note:colon.md"),
            Err(PathSafetyError::IllegalCharacters(_))
        ));
        assert!(matches!(
            validate_filename("file|pipe.md"),
            Err(PathSafetyError::IllegalCharacters(_))
        ));
        assert!(matches!(
            validate_filename("hello?world.md"),
            Err(PathSafetyError::IllegalCharacters(_))
        ));
        assert!(matches!(
            validate_filename("star*.md"),
            Err(PathSafetyError::IllegalCharacters(_))
        ));
        assert!(matches!(
            validate_filename("quote\".md"),
            Err(PathSafetyError::IllegalCharacters(_))
        ));
    }

    #[test]
    fn test_trailing_periods_and_spaces() {
        assert!(matches!(
            validate_filename("test. "),
            Err(PathSafetyError::TrailingPeriodOrSpace(_))
        ));
        assert!(matches!(
            validate_filename("test."),
            Err(PathSafetyError::TrailingPeriodOrSpace(_))
        ));
        assert!(matches!(
            validate_filename("note "),
            Err(PathSafetyError::TrailingPeriodOrSpace(_))
        ));
        assert!(validate_filename("test.md").is_ok());
    }

    #[test]
    fn test_traversal_sanitization() {
        let vault = Path::new("C:\\vault");
        assert!(matches!(
            sanitize_path(vault, Path::new("../escaped.md")),
            Err(PathSafetyError::TraversalAttempted(_))
        ));
        assert!(matches!(
            sanitize_path(vault, Path::new("sub/../../escaped.md")),
            Err(PathSafetyError::TraversalAttempted(_))
        ));
        assert!(sanitize_path(vault, Path::new("sub/note.md")).is_ok());
    }
}
