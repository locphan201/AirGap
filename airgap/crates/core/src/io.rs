use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum IoError {
    #[error("I/O failure: {0}")]
    Io(#[from] io::Error),

    #[error("Failed to determine parent directory for path: {0}")]
    MissingParentDir(String),

    #[error("UTF-8 decoding error: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineEnding {
    Lf,
    Crlf,
}

impl LineEnding {
    pub fn as_str(&self) -> &'static str {
        match self {
            LineEnding::Lf => "\n",
            LineEnding::Crlf => "\r\n",
        }
    }
}

/// Detects whether a string primarily uses CRLF or LF line endings.
pub fn detect_line_ending(content: &str) -> LineEnding {
    if content.contains("\r\n") {
        LineEnding::Crlf
    } else {
        LineEnding::Lf
    }
}

/// Normalizes text to the requested line ending.
pub fn apply_line_ending(content: &str, ending: LineEnding) -> String {
    let mut normalized = String::with_capacity(content.len());
    let mut chars = content.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\r' {
            if chars.peek() == Some(&'\n') {
                chars.next();
            }
            normalized.push_str(ending.as_str());
        } else if c == '\n' {
            normalized.push_str(ending.as_str());
        } else {
            normalized.push(c);
        }
    }
    normalized
}

/// Strips UTF-8 Byte Order Mark (BOM: \u{FEFF}) if present.
pub fn strip_bom(content: &str) -> &str {
    content.strip_prefix('\u{FEFF}').unwrap_or(content)
}

struct TempFileCleanupGuard<'a> {
    path: &'a Path,
    active: bool,
}

impl<'a> Drop for TempFileCleanupGuard<'a> {
    fn drop(&mut self) {
        if self.active {
            let _ = fs::remove_file(self.path);
        }
    }
}

/// Atomically writes content to `target_path`.
/// Writes to a temporary file in the same directory, flushes and syncs to disk (`fsync`),
/// and atomically renames the temporary file over `target_path`.
/// Preserves CRLF line endings and UTF-8 BOM if existing file has them; defaults new notes to LF.
pub fn write_atomic(target_path: &Path, content: &str) -> Result<(), IoError> {
    let parent = target_path
        .parent()
        .ok_or_else(|| IoError::MissingParentDir(target_path.to_string_lossy().into_owned()))?;

    // Create parent directory if it does not exist
    if !parent.exists() {
        fs::create_dir_all(parent)?;
    }

    // Inspect existing target file if present for CRLF and BOM preservation
    let (preserve_crlf, preserve_bom) = if target_path.exists() {
        if let Ok(mut existing_file) = File::open(target_path) {
            let mut sample = Vec::new();
            let _ = existing_file.read_to_end(&mut sample);
            let has_crlf = sample.windows(2).any(|w| w == b"\r\n");
            let has_bom = sample.starts_with(&[0xEF, 0xBB, 0xBF]);
            (has_crlf, has_bom)
        } else {
            (false, false)
        }
    } else {
        (false, false)
    };

    // If existing file had CRLF, normalize content to CRLF; otherwise default to LF
    let normalized = if preserve_crlf {
        apply_line_ending(content, LineEnding::Crlf)
    } else {
        apply_line_ending(content, LineEnding::Lf)
    };

    // Prepare final payload bytes
    let mut payload = Vec::new();
    let has_explicit_bom = content.starts_with('\u{FEFF}');
    if (preserve_bom || has_explicit_bom) && !content.is_empty() {
        payload.extend_from_slice(&[0xEF, 0xBB, 0xBF]);
    }
    let stripped_content = strip_bom(&normalized);
    payload.extend_from_slice(stripped_content.as_bytes());

    // Create temporary file in same directory to ensure same filesystem mount
    let temp_file = tempfile::Builder::new()
        .prefix(".airgap_tmp_")
        .suffix(".tmp")
        .tempfile_in(parent)?;

    let (mut file, temp_path) = temp_file.into_parts();
    let mut cleanup_guard = TempFileCleanupGuard {
        path: &temp_path,
        active: true,
    };

    // Write content, flush and sync_all (fsync)
    file.write_all(&payload)?;
    file.flush()?;
    file.sync_all()?;
    drop(file);

    // Atomically replace target file
    #[cfg(windows)]
    {
        if target_path.exists() {
            if fs::rename(&temp_path, target_path).is_err() {
                let _ = fs::remove_file(target_path);
                fs::rename(&temp_path, target_path)?;
            }
        } else {
            fs::rename(&temp_path, target_path)?;
        }
    }

    #[cfg(not(windows))]
    {
        fs::rename(&temp_path, target_path)?;
    }

    // Defuse cleanup guard since rename succeeded
    cleanup_guard.active = false;

    Ok(())
}

/// Reads a note from disk, handling UTF-8 BOM tolerance.
pub fn read_note(path: &Path) -> Result<String, IoError> {
    let mut file = File::open(path)?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;

    // Check for UTF-8 BOM bytes: 0xEF, 0xBB, 0xBF
    let slice = if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        &bytes[3..]
    } else {
        &bytes[..]
    };

    let s = String::from_utf8(slice.to_vec())?;
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_atomic_write_and_read() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.md");

        write_atomic(&file_path, "# Title\nHello AirGap").unwrap();
        assert!(file_path.exists());

        let read_back = read_note(&file_path).unwrap();
        assert_eq!(read_back, "# Title\nHello AirGap");

        // Overwrite existing atomically
        write_atomic(&file_path, "# Updated Title\nNew Content").unwrap();
        let read_updated = read_note(&file_path).unwrap();
        assert_eq!(read_updated, "# Updated Title\nNew Content");
    }

    #[test]
    fn test_bom_tolerance() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("bom.md");

        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice("BOM note content".as_bytes());
        fs::write(&file_path, bytes).unwrap();

        let content = read_note(&file_path).unwrap();
        assert_eq!(content, "BOM note content");
    }

    #[test]
    fn test_line_ending_detection_and_apply() {
        assert_eq!(detect_line_ending("line 1\r\nline 2"), LineEnding::Crlf);
        assert_eq!(detect_line_ending("line 1\nline 2"), LineEnding::Lf);

        let crlf_text = apply_line_ending("line 1\nline 2", LineEnding::Crlf);
        assert_eq!(crlf_text, "line 1\r\nline 2");

        let lf_text = apply_line_ending("line 1\r\nline 2", LineEnding::Lf);
        assert_eq!(lf_text, "line 1\nline 2");
    }
}
