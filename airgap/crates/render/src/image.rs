use std::path::{Component, Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum ImageResolutionError {
    #[error("Network image URLs are prohibited by local-only privacy: {0}")]
    NetworkBlocked(String),

    #[error("Image path escapes vault root: {0}")]
    VaultEscape(String),

    #[error("Image file does not exist on disk: {0}")]
    NotFound(String),

    #[error("Image filename or path is invalid: {0}")]
    InvalidPath(String),

    #[error("Unsupported image format: {0}")]
    UnsupportedFormat(String),
}

const SUPPORTED_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "svg", "webp", "bmp", "gif"];

/// Resolves a Markdown image URL relative to the active note and vault root.
/// Returns verified canonical path within vault.
pub fn resolve_local_image_path(
    vault_root: &Path,
    active_note_path: &Path,
    image_url: &str,
) -> Result<PathBuf, ImageResolutionError> {
    let trimmed = image_url.trim();

    // 1. Strict ban on network schemes
    let lower = trimmed.to_lowercase();
    if lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("//")
        || lower.starts_with("ftp://")
    {
        return Err(ImageResolutionError::NetworkBlocked(trimmed.to_string()));
    }

    // 2. Strip file:// scheme if present
    let raw_path_str = if let Some(stripped) = trimmed.strip_prefix("file://") {
        stripped
    } else {
        trimmed
    };

    // Remove query parameters or fragments if any (e.g. "image.png?v=1#anchor")
    let clean_str = raw_path_str.split('?').next().unwrap_or(raw_path_str);
    let clean_str = clean_str.split('#').next().unwrap_or(clean_str);

    // 3. Canonicalize vault_root
    let canon_vault = vault_root
        .canonicalize()
        .map_err(|_| ImageResolutionError::InvalidPath(vault_root.display().to_string()))?;

    // Determine base directory and canonicalize if possible
    let canon_base = if clean_str.starts_with('/') || clean_str.starts_with('\\') {
        canon_vault.clone()
    } else {
        let note_dir = if active_note_path.is_file() {
            active_note_path.parent().unwrap_or(vault_root)
        } else {
            active_note_path
        };
        note_dir
            .canonicalize()
            .unwrap_or_else(|_| canon_vault.clone())
    };

    // 4. Combine base folder with image path
    let rel_str = if clean_str.starts_with('/') || clean_str.starts_with('\\') {
        clean_str.trim_start_matches(['/', '\\'])
    } else {
        clean_str
    };

    let mut normalized = canon_base;
    for comp in Path::new(rel_str).components() {
        match comp {
            Component::ParentDir => {
                if !normalized.pop() || !normalized.starts_with(&canon_vault) {
                    return Err(ImageResolutionError::VaultEscape(clean_str.to_string()));
                }
            }
            Component::CurDir => {}
            Component::Normal(c) => normalized.push(c),
            Component::RootDir | Component::Prefix(_) => {
                normalized = PathBuf::from(comp.as_os_str());
            }
        }
    }

    if !normalized.starts_with(&canon_vault) {
        return Err(ImageResolutionError::VaultEscape(clean_str.to_string()));
    }

    // 5. Verify file exists
    if !normalized.exists() {
        return Err(ImageResolutionError::NotFound(
            normalized.display().to_string(),
        ));
    }

    // 6. Verify containment inside vault_root after symlink resolution
    let canon_file = normalized
        .canonicalize()
        .map_err(|_| ImageResolutionError::NotFound(normalized.display().to_string()))?;

    if !canon_file.starts_with(&canon_vault) {
        return Err(ImageResolutionError::VaultEscape(clean_str.to_string()));
    }

    // 7. Validate extension
    let ext = canon_file
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    if !SUPPORTED_EXTENSIONS.contains(&ext.as_str()) {
        return Err(ImageResolutionError::UnsupportedFormat(ext));
    }

    Ok(canon_file)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};

    #[test]
    fn test_network_images_blocked() {
        let vault = Path::new("C:/vault");
        let note = Path::new("C:/vault/note.md");

        assert_eq!(
            resolve_local_image_path(vault, note, "http://example.com/pic.png"),
            Err(ImageResolutionError::NetworkBlocked(
                "http://example.com/pic.png".into()
            ))
        );
        assert_eq!(
            resolve_local_image_path(vault, note, "https://example.com/pic.png"),
            Err(ImageResolutionError::NetworkBlocked(
                "https://example.com/pic.png".into()
            ))
        );
        assert_eq!(
            resolve_local_image_path(vault, note, "//cdn.example.com/pic.png"),
            Err(ImageResolutionError::NetworkBlocked(
                "//cdn.example.com/pic.png".into()
            ))
        );
        assert_eq!(
            resolve_local_image_path(vault, note, "ftp://example.com/pic.png"),
            Err(ImageResolutionError::NetworkBlocked(
                "ftp://example.com/pic.png".into()
            ))
        );
    }

    #[test]
    fn test_relative_image_resolution_success() {
        let dir = tempfile::tempdir().unwrap();
        let vault_root = dir.path();
        let assets_dir = vault_root.join("assets");
        fs::create_dir_all(&assets_dir).unwrap();

        let image_file = assets_dir.join("photo.png");
        File::create(&image_file).unwrap();

        let note_path = vault_root.join("subfolder").join("my_note.md");
        fs::create_dir_all(note_path.parent().unwrap()).unwrap();
        File::create(&note_path).unwrap();

        // 1. Vault-relative with leading slash
        let res1 = resolve_local_image_path(vault_root, &note_path, "/assets/photo.png").unwrap();
        assert_eq!(res1, image_file.canonicalize().unwrap());

        // 2. Note-relative with parent navigation
        let res2 = resolve_local_image_path(vault_root, &note_path, "../assets/photo.png").unwrap();
        assert_eq!(res2, image_file.canonicalize().unwrap());
    }

    #[test]
    fn test_vault_escape_rejection() {
        let dir = tempfile::tempdir().unwrap();
        let vault_root = dir.path().join("vault");
        let outside = dir.path().join("outside");
        fs::create_dir_all(&vault_root).unwrap();
        fs::create_dir_all(&outside).unwrap();

        let outside_img = outside.join("secret.png");
        File::create(&outside_img).unwrap();

        let note = vault_root.join("note.md");
        File::create(&note).unwrap();

        let res = resolve_local_image_path(&vault_root, &note, "../outside/secret.png");
        assert!(matches!(res, Err(ImageResolutionError::VaultEscape(_))));
    }

    #[test]
    fn test_unsupported_format_rejection() {
        let dir = tempfile::tempdir().unwrap();
        let vault_root = dir.path();
        let bad_file = vault_root.join("script.sh");
        File::create(&bad_file).unwrap();

        let note = vault_root.join("note.md");
        File::create(&note).unwrap();

        let res = resolve_local_image_path(vault_root, &note, "script.sh");
        assert!(matches!(
            res,
            Err(ImageResolutionError::UnsupportedFormat(_))
        ));
    }

    #[test]
    fn test_not_found_rejection() {
        let dir = tempfile::tempdir().unwrap();
        let vault_root = dir.path();
        let note = vault_root.join("note.md");
        File::create(&note).unwrap();

        let res = resolve_local_image_path(vault_root, &note, "missing.png");
        assert!(matches!(res, Err(ImageResolutionError::NotFound(_))));
    }
}
