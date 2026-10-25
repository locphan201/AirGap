use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use thiserror::Error;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

#[derive(Error, Debug)]
pub enum ExportError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Zip write error: {0}")]
    Zip(#[from] zip::result::ZipError),
}

/// Exports a vault directory to a zip backup.
pub fn export_vault_zip(vault_root: &Path, destination_zip: &Path) -> Result<(), ExportError> {
    let file = File::create(destination_zip)?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    fn visit_dirs(
        root: &Path,
        dir: &Path,
        zip: &mut ZipWriter<File>,
        options: SimpleFileOptions,
    ) -> Result<(), ExportError> {
        if dir.is_dir() {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                let file_name = entry.file_name().to_string_lossy().into_owned();

                // Skip hidden files/folders
                if file_name.starts_with('.') {
                    continue;
                }

                if path.is_dir() {
                    visit_dirs(root, &path, zip, options)?;
                } else {
                    let rel_path = path.strip_prefix(root).unwrap_or(&path);
                    let name_str = rel_path.to_string_lossy().replace('\\', "/");
                    zip.start_file(name_str, options)?;
                    let data = fs::read(&path)?;
                    zip.write_all(&data)?;
                }
            }
        }
        Ok(())
    }

    visit_dirs(vault_root, vault_root, &mut zip, options)?;
    zip.finish()?;
    Ok(())
}

/// Generates a standalone HTML document with embedded CSS.
pub fn generate_html_document(title: &str, body_html: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <title>{}</title>
  <style>
    body {{
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
      line-height: 1.6;
      max-width: 800px;
      margin: 40px auto;
      padding: 0 20px;
      color: #1f2328;
    }}
    pre, code {{ font-family: monospace; background: #f6f8fa; border-radius: 4px; padding: 2px 4px; }}
    pre code {{ padding: 0; }}
    pre {{ padding: 16px; overflow: auto; }}
    blockquote {{ border-left: 4px solid #d0d7de; margin: 0; padding-left: 16px; color: #656d76; }}
    table {{ border-collapse: collapse; width: 100%; }}
    th, td {{ border: 1px solid #d0d7de; padding: 6px 13px; }}
  </style>
</head>
<body>
  <h1>{}</h1>
  {}
</body>
</html>"#,
        title, title, body_html
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_export_vault_zip() {
        let dir = tempdir().unwrap();
        let note1 = dir.path().join("Note1.md");
        fs::write(note1, "Content 1").unwrap();

        let zip_dest = dir.path().join("backup.zip");
        export_vault_zip(dir.path(), &zip_dest).unwrap();
        assert!(zip_dest.exists());
    }

    #[test]
    fn test_html_document_generation() {
        let html = generate_html_document("Test Note", "<p>Hello</p>");
        assert!(html.contains("<title>Test Note</title>"));
        assert!(html.contains("<p>Hello</p>"));
    }
}
