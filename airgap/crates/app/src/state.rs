use crate::theme::ThemeMode;
use airgap_core::{TreeNode, Vault, VaultError};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Instant;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveStatus {
    Saved,
    Unsaved,
    Saving,
    Error(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewMode {
    #[default]
    Split,
    EditorOnly,
    PreviewOnly,
}

impl ViewMode {
    pub fn cycle(&self) -> Self {
        match self {
            ViewMode::Split => ViewMode::EditorOnly,
            ViewMode::EditorOnly => ViewMode::PreviewOnly,
            ViewMode::PreviewOnly => ViewMode::Split,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScrollDriver {
    #[default]
    None,
    Editor,
    Preview,
}

#[derive(Debug, Clone, Default)]
pub struct ScrollSyncState {
    pub driver: ScrollDriver,
    pub target_preview_scroll: Option<f32>,
    pub target_editor_scroll: Option<f32>,
    pub editor_line: usize,
    pub editor_content_height: f32,
    pub editor_viewport_height: f32,
    pub preview_scroll_y: f32,
    pub preview_content_height: f32,
    pub preview_viewport_height: f32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextSnapshot {
    pub text: String,
    pub cursor_line: usize,
    pub cursor_col: usize,
}

pub struct UndoManager {
    pub undo_stack: Vec<TextSnapshot>,
    pub redo_stack: Vec<TextSnapshot>,
    pub last_edit: Instant,
    pub max_history: usize,
}

impl Default for UndoManager {
    fn default() -> Self {
        Self::new(200)
    }
}

impl UndoManager {
    pub fn new(max_history: usize) -> Self {
        Self {
            undo_stack: Vec::with_capacity(max_history),
            redo_stack: Vec::new(),
            last_edit: Instant::now(),
            max_history,
        }
    }

    pub fn push_snapshot(
        &mut self,
        text: &str,
        cursor_line: usize,
        cursor_col: usize,
        force: bool,
    ) {
        let snapshot = TextSnapshot {
            text: text.to_string(),
            cursor_line,
            cursor_col,
        };

        if let Some(top) = self.undo_stack.last() {
            if top.text == snapshot.text {
                return;
            }
        }

        let now = Instant::now();
        let elapsed = now.duration_since(self.last_edit).as_millis();
        self.last_edit = now;
        self.redo_stack.clear();

        if !force && elapsed < 500 && !self.undo_stack.is_empty() {
            let last_idx = self.undo_stack.len() - 1;
            self.undo_stack[last_idx] = snapshot;
        } else {
            if self.undo_stack.len() >= self.max_history {
                self.undo_stack.remove(0);
            }
            self.undo_stack.push(snapshot);
        }
    }

    pub fn undo(
        &mut self,
        current_text: &str,
        current_line: usize,
        current_col: usize,
    ) -> Option<TextSnapshot> {
        if self.undo_stack.is_empty() {
            return None;
        }

        let current_snap = TextSnapshot {
            text: current_text.to_string(),
            cursor_line: current_line,
            cursor_col: current_col,
        };

        if let Some(top) = self.undo_stack.last() {
            if top.text == current_text {
                let popped = self.undo_stack.pop().unwrap();
                self.redo_stack.push(popped);
            } else {
                self.redo_stack.push(current_snap);
            }
        } else {
            self.redo_stack.push(current_snap);
        }

        self.undo_stack.last().cloned()
    }

    pub fn redo(
        &mut self,
        current_text: &str,
        current_line: usize,
        current_col: usize,
    ) -> Option<TextSnapshot> {
        let snapshot = self.redo_stack.pop()?;
        self.undo_stack.push(TextSnapshot {
            text: current_text.to_string(),
            cursor_line: current_line,
            cursor_col: current_col,
        });
        Some(snapshot)
    }
}

#[derive(Debug, Clone, Default)]
pub struct FindReplaceState {
    pub is_open: bool,
    pub show_replace: bool,
    pub find_query: String,
    pub replace_query: String,
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub use_regex: bool,
    pub regex_error: Option<String>,
    pub matches: Vec<std::ops::Range<usize>>,
    pub active_match: usize,
}

impl FindReplaceState {
    pub fn recalculate_matches(&mut self, text: &str) {
        self.matches.clear();
        self.regex_error = None;

        let query = self.find_query.trim();
        if query.is_empty() || text.is_empty() {
            self.active_match = 0;
            return;
        }

        if self.use_regex {
            let regex_result = regex::RegexBuilder::new(query)
                .case_insensitive(!self.case_sensitive)
                .build();
            match regex_result {
                Ok(re) => {
                    for mat in re.find_iter(text) {
                        self.matches.push(mat.range());
                    }
                }
                Err(err) => {
                    self.regex_error = Some(err.to_string());
                }
            }
        } else {
            let search_target = if self.case_sensitive {
                text.to_string()
            } else {
                text.to_lowercase()
            };
            let needle = if self.case_sensitive {
                query.to_string()
            } else {
                query.to_lowercase()
            };

            let mut start_idx = 0;
            while let Some(found_idx) = search_target[start_idx..].find(&needle) {
                let match_start = start_idx + found_idx;
                let match_end = match_start + needle.len();

                let is_word_boundary = if self.whole_word {
                    let before_is_word = match_start > 0
                        && text[..match_start]
                            .chars()
                            .last()
                            .map(|c| c.is_alphanumeric() || c == '_')
                            .unwrap_or(false);
                    let after_is_word = match_end < text.len()
                        && text[match_end..]
                            .chars()
                            .next()
                            .map(|c| c.is_alphanumeric() || c == '_')
                            .unwrap_or(false);
                    !before_is_word && !after_is_word
                } else {
                    true
                };

                if is_word_boundary {
                    self.matches.push(match_start..match_end);
                }

                start_idx = match_start + 1.max(needle.len());
                if start_idx >= text.len() {
                    break;
                }
            }
        }

        if self.matches.is_empty() || self.active_match >= self.matches.len() {
            self.active_match = 0;
        }
    }

    pub fn next_match(&mut self) {
        if !self.matches.is_empty() {
            self.active_match = (self.active_match + 1) % self.matches.len();
        }
    }

    pub fn prev_match(&mut self) {
        if !self.matches.is_empty() {
            self.active_match = if self.active_match == 0 {
                self.matches.len() - 1
            } else {
                self.active_match - 1
            };
        }
    }

    pub fn replace_current(
        &mut self,
        text: &mut String,
        undo: &mut UndoManager,
        cursor_line: usize,
        cursor_col: usize,
    ) {
        if self.matches.is_empty() {
            return;
        }
        let range = self.matches[self.active_match].clone();
        if range.end <= text.len() {
            text.replace_range(range, &self.replace_query);
            undo.push_snapshot(text, cursor_line, cursor_col, true);
            self.recalculate_matches(text);
        }
    }

    pub fn replace_all(
        &mut self,
        text: &mut String,
        undo: &mut UndoManager,
        cursor_line: usize,
        cursor_col: usize,
    ) {
        if self.matches.is_empty() {
            return;
        }
        let mut sorted = self.matches.clone();
        sorted.sort_by_key(|r| std::cmp::Reverse(r.start));

        for range in sorted {
            if range.end <= text.len() {
                text.replace_range(range, &self.replace_query);
            }
        }
        undo.push_snapshot(text, cursor_line, cursor_col, true);
        self.recalculate_matches(text);
    }
}

pub struct AppState {
    pub vault: Option<Vault>,
    pub tree: Option<TreeNode>,
    pub active_note_path: Option<PathBuf>,
    pub editor_content: String,
    pub saved_content: String,
    pub last_keystroke: Option<Instant>,
    pub save_status: SaveStatus,

    pub expanded_folders: HashSet<PathBuf>,
    pub renaming_item: Option<(PathBuf, String)>,
    pub delete_confirm: Option<PathBuf>,

    pub show_vault_picker: bool,
    pub vault_picker_path: String,
    pub vault_picker_error: Option<String>,

    pub theme_mode: ThemeMode,
    pub line_numbers_enabled: bool,
    pub word_wrap_enabled: bool,

    pub cursor_line: usize,
    pub cursor_col: usize,
    pub word_count: usize,
    pub char_count: usize,

    // M2 additions
    pub view_mode: ViewMode,
    pub split_ratio: f32,
    pub scroll_sync: ScrollSyncState,
    pub undo_manager: UndoManager,
    pub find_replace: FindReplaceState,
    pub editor_char_range: Option<std::ops::Range<usize>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            vault: None,
            tree: None,
            active_note_path: None,
            editor_content: String::new(),
            saved_content: String::new(),
            last_keystroke: None,
            save_status: SaveStatus::Saved,

            expanded_folders: HashSet::new(),
            renaming_item: None,
            delete_confirm: None,

            show_vault_picker: true,
            vault_picker_path: String::new(),
            vault_picker_error: None,

            theme_mode: ThemeMode::Dark,
            line_numbers_enabled: true,
            word_wrap_enabled: true,

            cursor_line: 1,
            cursor_col: 1,
            word_count: 0,
            char_count: 0,

            view_mode: ViewMode::Split,
            split_ratio: 0.5,
            scroll_sync: ScrollSyncState::default(),
            undo_manager: UndoManager::default(),
            find_replace: FindReplaceState::default(),
            editor_char_range: None,
        }
    }
}

impl AppState {
    pub fn open_vault(&mut self, root: PathBuf) -> Result<(), VaultError> {
        let vault = Vault::new(&root);
        let tree = vault.scan_tree()?;
        self.vault = Some(vault);
        self.tree = Some(tree);
        self.show_vault_picker = false;
        self.vault_picker_error = None;
        self.active_note_path = None;
        self.editor_content.clear();
        self.saved_content.clear();
        self.save_status = SaveStatus::Saved;

        if let Some(first_note) = self.find_first_note() {
            self.select_note(first_note);
        }

        Ok(())
    }

    fn find_first_note(&self) -> Option<PathBuf> {
        fn search(node: &TreeNode) -> Option<PathBuf> {
            if node.item_type == airgap_core::VaultItemType::Note {
                return Some(node.relative_path.clone());
            }
            for child in &node.children {
                if let Some(found) = search(child) {
                    return Some(found);
                }
            }
            None
        }

        self.tree.as_ref().and_then(search)
    }

    pub fn refresh_tree(&mut self) {
        if let Some(vault) = &self.vault {
            if let Ok(tree) = vault.scan_tree() {
                self.tree = Some(tree);
            }
        }
    }

    pub fn select_note(&mut self, rel_path: PathBuf) {
        if self.is_dirty() {
            self.save_active_note();
        }

        if let Some(vault) = &self.vault {
            match vault.read_note(&rel_path) {
                Ok(content) => {
                    self.active_note_path = Some(rel_path);
                    self.editor_content = content.clone();
                    self.saved_content = content.clone();
                    self.save_status = SaveStatus::Saved;
                    self.last_keystroke = None;
                    self.cursor_line = 1;
                    self.cursor_col = 1;
                    self.undo_manager = UndoManager::new(200);
                    self.undo_manager.push_snapshot(&content, 1, 1, true);
                    self.find_replace.recalculate_matches(&content);
                    self.editor_char_range = None;
                    self.update_metrics();
                }
                Err(e) => {
                    self.save_status = SaveStatus::Error(format!("Failed to read note: {}", e));
                }
            }
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.active_note_path.is_some() && self.editor_content != self.saved_content
    }

    pub fn on_editor_modified(&mut self) {
        self.save_status = SaveStatus::Unsaved;
        self.last_keystroke = Some(Instant::now());
        self.undo_manager.push_snapshot(
            &self.editor_content,
            self.cursor_line,
            self.cursor_col,
            false,
        );
        self.find_replace.recalculate_matches(&self.editor_content);
        self.update_metrics();
    }

    pub fn save_active_note(&mut self) {
        if let (Some(vault), Some(note_path)) = (&self.vault, &self.active_note_path) {
            match vault.write_note_atomic(note_path, &self.editor_content) {
                Ok(()) => {
                    self.saved_content = self.editor_content.clone();
                    self.save_status = SaveStatus::Saved;
                    self.last_keystroke = None;
                }
                Err(e) => {
                    self.save_status = SaveStatus::Error(format!("Autosave failed: {}", e));
                }
            }
        }
    }

    pub fn check_autosave(&mut self, window_focused: bool) {
        if !self.is_dirty() {
            return;
        }

        let should_save = if !window_focused {
            true
        } else if let Some(t) = self.last_keystroke {
            t.elapsed().as_millis() >= 1000
        } else {
            false
        };

        if should_save {
            self.save_active_note();
        }
    }

    pub fn create_new_note(&mut self, parent_rel: Option<&Path>) {
        if let Some(vault) = &self.vault {
            let parent = parent_rel.unwrap_or_else(|| Path::new(""));
            if let Ok(new_rel) = vault.create_note(parent, "Untitled.md") {
                self.refresh_tree();
                self.select_note(new_rel.clone());
                self.start_rename(new_rel);
            }
        }
    }

    pub fn create_new_folder(&mut self, parent_rel: Option<&Path>, folder_name: &str) {
        if let Some(vault) = &self.vault {
            let parent = parent_rel.unwrap_or_else(|| Path::new(""));
            if let Ok(new_folder_rel) = vault.create_folder(parent, folder_name) {
                self.expanded_folders.insert(new_folder_rel);
                self.refresh_tree();
            }
        }
    }

    pub fn start_rename(&mut self, rel_path: PathBuf) {
        let current_name = rel_path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        self.renaming_item = Some((rel_path, current_name));
    }

    pub fn commit_rename(&mut self) {
        if let Some((from_rel, new_name)) = self.renaming_item.take() {
            let trimmed = new_name.trim();
            if trimmed.is_empty() {
                return;
            }

            if let Some(vault) = &self.vault {
                if let Ok(renamed_rel) = vault.rename_item(&from_rel, trimmed) {
                    if self.active_note_path.as_ref() == Some(&from_rel) {
                        self.active_note_path = Some(renamed_rel);
                    }
                    self.refresh_tree();
                }
            }
        }
    }

    pub fn cancel_rename(&mut self) {
        self.renaming_item = None;
    }

    pub fn request_delete(&mut self, rel_path: PathBuf) {
        self.delete_confirm = Some(rel_path);
    }

    pub fn confirm_delete(&mut self) {
        if let Some(path_to_delete) = self.delete_confirm.take() {
            if let Some(vault) = &self.vault {
                if vault.delete_item(&path_to_delete).is_ok() {
                    if self.active_note_path.as_ref() == Some(&path_to_delete) {
                        self.active_note_path = None;
                        self.editor_content.clear();
                        self.saved_content.clear();
                    }
                    self.refresh_tree();
                }
            }
        }
    }

    pub fn cancel_delete(&mut self) {
        self.delete_confirm = None;
    }

    pub fn toggle_folder_expanded(&mut self, rel_path: &Path) {
        if self.expanded_folders.contains(rel_path) {
            self.expanded_folders.remove(rel_path);
        } else {
            self.expanded_folders.insert(rel_path.to_path_buf());
        }
    }

    pub fn update_metrics(&mut self) {
        self.char_count = self.editor_content.chars().count();
        self.word_count = self.editor_content.split_whitespace().count();
    }

    pub fn create_sample_vault() -> Result<PathBuf, std::io::Error> {
        let base_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let sample_dir = base_dir.join("Sample_Vault");
        std::fs::create_dir_all(&sample_dir)?;

        let welcome_path = sample_dir.join("Welcome.md");
        if !welcome_path.exists() {
            let welcome_content = r#"# Welcome to AirGap 🚀

AirGap is a fast, native, local-only Markdown note-taking desktop application built with Rust and egui.

## 🔒 Security & Privacy Non-Negotiables
- **Zero Network Traffic:** 100% offline, zero telemetry, zero analytics.
- **Pure Local Filesystem:** Notes are plain `.md` files in real folders on disk.
- **Atomic Writes:** Every save writes to a temporary file, fsyncs, and atomically renames.
- **Safe Recycling:** Deleting notes uses your OS recycle bin with fallback to `.trash/`.

## ⌨️ Essential Keyboard Shortcuts
- `Ctrl + N`: Create a new note
- `Ctrl + Shift + N`: Create a new folder
- `Ctrl + S`: Force immediate save
- `Ctrl + \`: Cycle view modes (Split, Editor, Preview)
- `Ctrl + F` / `Ctrl + H`: In-note Find & Replace
- `F2`: Inline rename active item
- `Del`: Delete item with confirmation prompt

Happy note taking!
"#;
            std::fs::write(&welcome_path, welcome_content)?;
        }

        let guides_dir = sample_dir.join("Guides");
        std::fs::create_dir_all(&guides_dir)?;

        let cheatsheet_path = guides_dir.join("Markdown_Cheatsheet.md");
        if !cheatsheet_path.exists() {
            let cheatsheet_content = r#"# Markdown Cheatsheet

## Typography
You can make text **bold**, *italic*, or `inline code`.

## Lists & Tasks
- [ ] Incomplete task item
- [x] Completed task item
- Regular list item
  - Nested sub-item

## GFM Table
| Feature | Supported | Description |
| :--- | :---: | :--- |
| Tables | Yes | Formatted with alignments |
| Tasks | Yes | Interactive checkboxes |
| Highlighting | Yes | Pure-Rust Syntect |

## Code Blocks
```rust
fn main() {
    println!("Hello from AirGap!");
}
```

## Quotes
> [!NOTE]
> "Simplicity is prerequisite for reliability." — Edsger W. Dijkstra
"#;
            std::fs::write(&cheatsheet_path, cheatsheet_content)?;
        }

        Ok(sample_dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_view_mode_cycle() {
        let mut mode = ViewMode::Split;
        mode = mode.cycle();
        assert_eq!(mode, ViewMode::EditorOnly);
        mode = mode.cycle();
        assert_eq!(mode, ViewMode::PreviewOnly);
        mode = mode.cycle();
        assert_eq!(mode, ViewMode::Split);
    }

    #[test]
    fn test_undo_redo_manager() {
        let mut undo = UndoManager::new(10);
        undo.push_snapshot("first", 1, 1, true);
        undo.push_snapshot("second", 1, 7, true);

        let undone = undo.undo("second", 1, 7);
        assert_eq!(undone.unwrap().text, "first");

        let redone = undo.redo("first", 1, 1);
        assert_eq!(redone.unwrap().text, "second");
    }

    #[test]
    fn test_find_and_replace() {
        let mut text = "The quick brown fox jumps over the lazy dog. The fox is fast.".to_string();
        let mut find_state = FindReplaceState {
            find_query: "fox".to_string(),
            replace_query: "cat".to_string(),
            ..Default::default()
        };
        find_state.recalculate_matches(&text);

        assert_eq!(find_state.matches.len(), 2);

        let mut undo = UndoManager::new(10);
        find_state.replace_all(&mut text, &mut undo, 1, 1);
        assert_eq!(
            text,
            "The quick brown cat jumps over the lazy dog. The cat is fast."
        );
    }
}
