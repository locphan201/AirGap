# Progress Tracker - worker_m1_1

Last visited: 2026-10-24T16:02:10Z

## Status
Milestone M1 implementation in progress. Core storage hardening complete and passing tests. GUI & layout implemented, compiling and running test suite.

## Completed
- Created DISPATCH.md
- Created BRIEFING.md
- Implemented `crates/core/src/io.rs`:
  - Added `TempFileCleanupGuard` RAII guard ensuring no orphaned `.tmp` files under any failure path.
  - Implemented line ending preservation (detecting and preserving CRLF on existing files, defaulting to LF for new notes).
  - Implemented UTF-8 BOM tolerance on read and preservation on write.
- Implemented `crates/core/src/path_safety.rs`:
  - Added multi-extension Windows reserved names check (e.g. `con.bak.md`, `aux.backup.tar.gz`, `NUL.old.txt`).
  - Added canonical vault root boundary enforcement and symlink escape rejection.
- Implemented `crates/core/src/trash.rs`:
  - Added timestamped collision handling in `.trash/` fallback.
  - Added cross-device link copy/delete fallback for `.trash/`.
- Implemented `crates/core/src/vault.rs`:
  - Added `create_note(&self, parent_rel, name)`.
  - Added `rename_item(&self, from_rel, to_name)`.
  - Added `delete_item(&self, path_rel)`.
  - Added `TrashError` into `VaultError`.
  - Added unit test covering create_note, rename_item, and delete_item.
- Implemented `crates/app`:
  - `theme.rs`: Dark, Light, and System themes with WCAG AA contrast palettes and consistent 6px corner radius.
  - `font.rs`: Typography configuration with Proportional UI fonts and Monospace editor fonts.
  - `state.rs`: Single source of truth `AppState`, debounced ~1s autosave, blur autosave, exit autosave, natural tree scanning, inline rename, delete confirmation, word & char counts, and sample vault creator.
  - `ui/vault_picker.rs`: Interactive vault selector with folder path input, `rfd` OS directory picker, and default sample vault generator.
  - `ui/sidebar.rs`: 1:1 disk folder/note tree, fold/unfold, note selection, inline rename (`F2`), new note (`Ctrl+N`), new folder (`Ctrl+Shift+N`), context menu.
  - `ui/editor.rs`: Plain-text Markdown editor with line numbers gutter, wrap toggle, current-line and cursor metrics, keystroke tracking.
  - `ui/status_bar.rs`: Word count, char count, Ln/Col, save indicator (`● Saved` / `○ Unsaved`), theme toggle.
  - `ui/delete_modal.rs`: Safe delete confirmation dialog.
  - `app.rs`: Central `eframe::App` with resizable sidebar, autosave loop, keyboard shortcuts, and exit flush.
  - `main.rs`: Default GUI runner + benchmark / spike CLI flags.

## In Progress
- Running workspace tests, clippy, fmt, and E2E verification
