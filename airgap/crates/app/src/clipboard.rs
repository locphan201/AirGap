use crate::url_safety::sanitize_external_url;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct ClipboardHandler;

impl ClipboardHandler {
    /// Attempts to read an image from the system clipboard.
    /// If an image exists, writes it as a PNG into `<note_dir>/assets/image_<timestamp>.png`
    /// and returns the relative Markdown image syntax: `![image](assets/image_<timestamp>.png)`.
    pub fn try_paste_image(vault_root: &Path, active_note_path: &Path) -> Option<String> {
        let mut clipboard = arboard::Clipboard::new().ok()?;
        let image = clipboard.get_image().ok()?;

        let note_parent = active_note_path.parent().unwrap_or_else(|| Path::new(""));
        let target_dir = vault_root.join(note_parent).join("assets");
        if std::fs::create_dir_all(&target_dir).is_err() {
            return None;
        }

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let filename = format!("image_{}.png", timestamp);
        let file_path = target_dir.join(&filename);

        // Save raw RGBA buffer to PNG
        let width = image.width as u32;
        let height = image.height as u32;
        if image::save_buffer_with_format(
            &file_path,
            &image.bytes,
            width,
            height,
            image::ExtendedColorType::Rgba8,
            image::ImageFormat::Png,
        )
        .is_err()
        {
            return None;
        }

        Some(format!("![image](assets/{})", filename))
    }

    /// If text is currently selected and the clipboard contains a valid external URL,
    /// formats the selection as `[selected_text](url)`.
    pub fn try_paste_smart_url(clipboard_text: &str, selected_text: &str) -> Option<String> {
        let trimmed_sel = selected_text.trim();
        if trimmed_sel.is_empty() {
            return None;
        }

        if let Ok(safe_url) = sanitize_external_url(clipboard_text) {
            Some(format!("[{}]({})", selected_text, safe_url))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smart_url_paste_selection() {
        let pasted =
            ClipboardHandler::try_paste_smart_url("https://example.com/docs", "My Link").unwrap();
        assert_eq!(pasted, "[My Link](https://example.com/docs)");
    }

    #[test]
    fn test_smart_url_paste_empty_selection_returns_none() {
        let pasted = ClipboardHandler::try_paste_smart_url("https://example.com/docs", "  ");
        assert!(pasted.is_none());
    }

    #[test]
    fn test_smart_url_paste_invalid_url_returns_none() {
        let pasted = ClipboardHandler::try_paste_smart_url("not a url", "My Link");
        assert!(pasted.is_none());
    }
}
