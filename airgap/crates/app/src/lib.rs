pub mod app;
pub mod benchmarks;
pub mod clipboard;
pub mod font;
pub mod spike_window;
pub mod state;
pub mod theme;
pub mod ui;
pub mod url_safety;

#[cfg(test)]
mod tests {
    use super::benchmarks::run_all_benchmarks;

    #[test]
    fn test_benchmark_execution() {
        let report = run_all_benchmarks();
        assert_eq!(report.diagram_benchmarks.len(), 8);
        // First 7 must be rendered correctly
        for b in &report.diagram_benchmarks[0..7] {
            assert!(
                b.rendered_correctly,
                "Diagram {} should render correctly",
                b.name
            );
            assert!(b.parse_svg_ms > 0.0);
            assert!(b.rasterize_1x_ms > 0.0);
        }
        // 8th is malformed, should trigger error handling without panic
        assert!(!report.diagram_benchmarks[7].rendered_correctly);
        assert!(report.diagram_benchmarks[7].error_msg.is_some());

        // TextEdit stress test
        assert!(report.text_edit_feasibility.actual_size_bytes >= 1024 * 1024);
        assert!(report.text_edit_feasibility.is_responsive);

        // CJK test
        assert!(report.cjk_feasibility.contains_cjk_ideographs);
        assert!(report.cjk_feasibility.contains_hiragana_katakana);
        assert!(report.cjk_feasibility.contains_hangul);
    }

    #[test]
    fn test_app_state_vault_operations_and_autosave() {
        use crate::state::{AppState, SaveStatus};
        use std::path::{Path, PathBuf};
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let mut state = AppState::default();

        // 1. Open vault
        state.open_vault(dir.path().to_path_buf()).unwrap();
        assert!(state.vault.is_some());
        assert!(!state.show_vault_picker);

        // 2. Create new note
        state.create_new_note(None);
        assert_eq!(state.active_note_path, Some(PathBuf::from("Untitled.md")));
        assert_eq!(state.editor_content, "");
        assert_eq!(state.save_status, SaveStatus::Saved);

        // 3. Edit content
        state.editor_content = "# My First Note\nThis is a test of AirGap.".to_string();
        state.on_editor_modified();
        assert!(state.is_dirty());
        assert_eq!(state.save_status, SaveStatus::Unsaved);
        assert_eq!(state.word_count, 10);
        assert!(state.char_count > 0);

        // 4. Autosave on window blur
        state.check_autosave(false);
        assert!(!state.is_dirty());
        assert_eq!(state.save_status, SaveStatus::Saved);
        assert_eq!(state.saved_content, state.editor_content);

        // Verify disk content
        let on_disk = state
            .vault
            .as_ref()
            .unwrap()
            .read_note(Path::new("Untitled.md"))
            .unwrap();
        assert_eq!(on_disk, state.editor_content);

        // 5. Inline rename
        state.start_rename(PathBuf::from("Untitled.md"));
        if let Some((_, buf)) = &mut state.renaming_item {
            *buf = "Renamed_Note.md".to_string();
        }
        state.commit_rename();
        assert_eq!(
            state.active_note_path,
            Some(PathBuf::from("Renamed_Note.md"))
        );
        assert!(dir.path().join("Renamed_Note.md").exists());
        assert!(!dir.path().join("Untitled.md").exists());

        // 6. Delete confirmation flow
        state.request_delete(PathBuf::from("Renamed_Note.md"));
        assert_eq!(state.delete_confirm, Some(PathBuf::from("Renamed_Note.md")));
        state.confirm_delete();
        assert_eq!(state.delete_confirm, None);
        assert_eq!(state.active_note_path, None);
        assert!(!dir.path().join("Renamed_Note.md").exists());
    }
}
