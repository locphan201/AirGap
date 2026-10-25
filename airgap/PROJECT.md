# Project: AirGap — Native Local-Only Markdown Note-Taking Desktop Application

## Architecture
AirGap is architected as a modular, three-crate Cargo workspace in Rust (edition 2021) adhering to strict separation of concerns, zero network access, and non-blocking asynchronous UI execution:

1. `crates/core` (`airgap-core`):
   - Pure domain logic, zero UI dependencies.
   - Modules: `vault` (directory tree model, 1:1 disk mapping), `io` (atomic writes: tempfile + fsync + rename), `path_safety` (rejection of directory traversal `..`, symlink escapes, Windows reserved names `CON`, `PRN`, `AUX`, `NUL`, `COM1-9`, `LPT1-9`, and long path handling), `trash` (OS recycle bin with fallback to `.trash/`), `watcher` (`notify` filesystem watcher with self-write suppression and debounce), `search` (quick open fuzzy match via `fuzzy-matcher` and parallel multi-threaded full-text search via `rayon`), `importer` (folder, Obsidian vault, and zip archive with zip-slip/zip-bomb guards), `exporter` (zip backup, directory copy, standalone HTML export with inline SVG), `config` (user settings and session persistence via `directories` crate).

2. `crates/render` (`airgap-render`):
   - Headless Markdown parsing and vector rendering pipeline.
   - Modules: `markdown` (CommonMark + GFM parser via `pulldown-cmark`, producing an intermediate `DocModel` with source-line mappings for scroll sync), `highlight` (syntax highlighting for code fences using `syntect` with pure-Rust `fancy-regex` backend), `mermaid` (native Rust Mermaid parser, layout engine for flowchart, sequence, class, state, ER, pie, gantt, and pure SVG generator), `raster` (rasterization from SVG to RGBA pixmaps via `resvg`/`usvg`/`tiny-skia` with premultiplied alpha), `cache` (LRU pixmap cache keyed by content hash, theme, and HiDPI scale), `fallback` (graceful syntax error diagnostics banner and formatted code block).

3. `crates/app` (`airgap-app`):
   - Desktop application binary using `egui` and `eframe`.
   - Modules: `state` (Elm-style central application state and command dispatcher), `ui` (sidebar tree, multi-pane layout, editor widget with line numbers and syntax layouter, preview pane with triple view modes, status bar), `dialogs` (conflict resolution prompt: "Keep Mine" / "Take Theirs" / "View Diff", import preview modal, delete confirmation, settings modal, quick open palette), `workers` (background thread channels via `crossbeam-channel` communicating with `ctx.request_repaint()`), `theme` (custom dark and light themes, typography loading for bundled Inter and JetBrains Mono).

---

## Feature Inventory
Every feature from the Survey phase is mapped below with its assigned milestone.

| # | Feature | Category | Description | Milestone | Source |
|---|---------|----------|-------------|-----------|--------|
| 1 | Multi-Crate Workspace | ARCH | Workspace structure (`core`, `render`, `app`) | M1 | survey |
| 2 | Non-Blocking UI Thread | ARCH | Disk I/O, search, rasterization run off UI thread | M6 | survey |
| 3 | Elm-Style App State | ARCH | Single source of truth message-driven state | M4 | survey |
| 4 | Local 1:1 Directory Mapping | VAULT | Vault maps 1:1 to filesystem directories | M1 | survey |
| 5 | Note Creation | VAULT | Create `Untitled.md` with auto-increment suffix | M1 | survey |
| 6 | Folder Creation | VAULT | Create subdirectories in vault root/folders | M1 | survey |
| 7 | In-Place Renaming | VAULT | Rename note/folder via UI or `F2` | M1 | survey |
| 8 | Safe Deletion & Recycle Bin | VAULT | Delete with confirmation; OS trash + `.trash/` fallback | M1 | survey |
| 9 | Drag-and-Drop Reorganization | VAULT | Move notes/folders in sidebar via drag-and-drop | M4 | survey |
| 10 | Tree State Persistence | VAULT | Persist expanded/collapsed folder states | M4 | survey |
| 11 | Sidebar Sorting & Context Menu | VAULT | Folders first, alphabetical; right-click context menu | M4 | survey |
| 12 | Atomic Note Writes | SAFE | Tempfile + fsync + atomic rename | M1 | survey |
| 13 | Strict Path Sanitization | SAFE | Vault root boundary check, block `..` escapes | M1 | survey |
| 14 | Windows Reserved Filename Guard | SAFE | Reject `CON`, `PRN`, `AUX`, `NUL`, `COM1-9`, `LPT1-9` | M1 | survey |
| 15 | Windows Filename Sanitization | SAFE | Block illegal chars `<>:"/\|?*` & trailing dots/spaces | M1 | survey |
| 16 | Windows Long Path Support | SAFE | Extended path handling (`\\?\`) for paths >260 chars | M1 | survey |
| 17 | Line Ending Preservation | SAFE | Preserve existing CRLF vs LF; default LF for new | M1 | survey |
| 18 | UTF-8 BOM Tolerance | SAFE | Handle UTF-8 Byte Order Mark without artifact display | M1 | survey |
| 19 | Filesystem Watcher (`notify`) | SAFE | Debounced external disk change monitoring | M4 | survey |
| 20 | Conflict Resolution Dialog | SAFE | 3-way conflict dialog: Keep Mine / Take Theirs / Diff | M4 | survey |
| 21 | Crash Recovery Buffer | SAFE | Panic hook flushes dirty buffers to recovery file | M4 | survey |
| 22 | Syntax-Highlighted Editor | EDIT | Headings, bold, italic, code spans, links highlighted | M2 | survey |
| 23 | Line Numbers & Word Wrap | EDIT | Toggleable line numbering gutter and soft word wrap | M2 | survey |
| 24 | Current-Line Highlight | EDIT | Visual highlighting of active cursor line | M2 | survey |
| 25 | Undo / Redo History | EDIT | Undo/redo stack for editor text operations | M2 | survey |
| 26 | In-Note Find & Replace | EDIT | Find & replace toolbar (case, whole-word, regex) | M2 | survey |
| 27 | Autosave Mechanism | EDIT | Autosave ~1s after typing, on focus loss, on exit | M1 | survey |
| 28 | Saved / Unsaved Indicator | EDIT | Visual dirty indicator in tab / status bar | M1 | survey |
| 29 | List Continuation on Enter | EDIT | Enter on bullet/numbered/task list continues prefix | M2 | survey |
| 30 | Tab / Shift+Tab Indentation | EDIT | Indent / outdent line or selection | M2 | survey |
| 31 | Auto-Pairing Enclosures | EDIT | Auto-insert closing brackets, quotes, backticks | M2 | survey |
| 32 | Smart URL Paste | EDIT | Pasting URL over selected text wraps into `[text](url)` | M2 | survey |
| 33 | Image Paste from Clipboard | EDIT | Paste image writes PNG to `assets/` and inserts link | M2 | survey |
| 34 | CJK & IME Input Support | EDIT | Chinese, Japanese, Korean IME composition support | M0 | survey |
| 35 | Large Note Scalability | EDIT | 1 MB+ note editing without lag (<500ms open) | M0 | survey |
| 36 | Triple View Modes | PREV | Editor Only, Split View, Preview Only (`Ctrl+\`) | M2 | survey |
| 37 | CommonMark + GFM Rendering | PREV | Headings, lists, quotes, strikethrough, HR | M2 | survey |
| 38 | Markdown Tables | PREV | GFM tables with column alignment and padding | M2 | survey |
| 39 | Task Lists | PREV | Interactive/styled `- [ ]` and `- [x]` checkboxes | M2 | survey |
| 40 | Code Blocks with Highlighting | PREV | Fenced code blocks with `syntect` syntax colors | M2 | survey |
| 41 | Local Relative Images | PREV | Render local relative image files in preview | M2 | survey |
| 42 | Safe Hyperlink Handling | PREV | Open in OS browser on click; no in-app web requests | M2 | survey |
| 43 | Editor-Preview Scroll Sync | PREV | Approximate scroll synchronization between panes | M2 | survey |
| 44 | Native Mermaid Engine | MERM | Native Rust Mermaid parsing without browser/JS | M3 | survey |
| 45 | Flowchart / Graph Support | MERM | Flowchart/graph rendering (`graph TD`, `flowchart LR`) | M3 | survey |
| 46 | Sequence Diagram Support | MERM | Sequence diagram rendering (actors, messages, notes) | M3 | survey |
| 47 | Class Diagram Support | MERM | Class diagram rendering (classes, methods, arrows) | M3 | survey |
| 48 | State Diagram Support | MERM | State diagram rendering (states, transitions) | M3 | survey |
| 49 | ER, Pie & Gantt Support | MERM | Entity-Relationship, Pie chart, and Gantt charts | M3 | survey |
| 50 | Graceful Error Fallback | MERM | Formatted code block with error line/diagnostic | M3 | survey |
| 51 | Resvg Rasterization & Texture | MERM | SVG to RGBA pixmap via `resvg`/`tiny-skia` | M3 | survey |
| 52 | Off-Thread Background Raster | MERM | Async rasterization off UI thread via channels | M3 | survey |
| 53 | Theme-Aware Diagram Styling | MERM | Color-adapted SVG palette for Dark and Light themes | M3 | survey |
| 54 | HiDPI Crisp Rasterization | MERM | Rasterize at physical pixel scale (100%–200%) | M3 | survey |
| 55 | Content-Hashed Texture Cache | MERM | LRU cache keyed by `(hash, is_dark, scale)` | M3 | survey |
| 56 | Fuzzy Quick Open (`Ctrl+P`) | SRCH | Fast modal fuzzy search over note filenames | M4 | survey |
| 57 | Global Full-Text Search | SRCH | Parallel multi-threaded search (`Ctrl+Shift+F`) | M4 | survey |
| 58 | Recent Notes History | SRCH | MRU list of recent notes | M4 | survey |
| 59 | Layout & Session Restore | SRCH | Restore open note, tree expansion, panel sizes | M4 | survey |
| 60 | Plain Folder Import | IMP | Copy external folder of notes into vault | M5 | survey |
| 61 | Obsidian Vault Import | IMP | Import Obsidian vault (skip `.obsidian/`, keep links) | M5 | survey |
| 62 | Safe Zip Archive Import | IMP | Extract `.zip` with zip-slip & zip-bomb protection | M5 | survey |
| 63 | HTML to Markdown Import | IMP | Convert HTML files to Markdown via `htmd` | M5 | survey |
| 64 | Import Conflict Modal | IMP | Conflict preview modal: Skip, Overwrite, Keep Both | M5 | survey |
| 65 | Vault / Folder Zip Backup | EXP | Export vault/folder as compressed `.zip` archive | M5 | survey |
| 66 | Directory Copy Export | EXP | Plain filesystem copy export to external folder | M5 | survey |
| 67 | Standalone HTML Note Export | EXP | Export note to HTML with embedded CSS & inline SVG | M5 | survey |
| 68 | Dual Polished Themes | UIUX | Dark and Light themes plus follow-system | M1 | survey |
| 69 | Bundled Offline Typography | UIUX | Embedded OFL fonts (Sans, Monospace) | M1 | survey |
| 70 | Resizable Multi-Pane Layout | UIUX | Resizable sidebar, split editor/preview divider | M1 | survey |
| 71 | Tab Bar / History Stack | UIUX | Multi-tab bar or forward/back note navigation | M4 | survey |
| 72 | Status Bar Metrics | UIUX | Word count, character count, cursor line:col | M4 | survey |
| 73 | Keyboard-First Shortcuts | UIUX | Full suite of keyboard accelerators | M4 | survey |
| 74 | Settings Panel | UIUX | Preferences modal (vault path, theme, fonts, etc.) | M4 | survey |
| 75 | Zero-Network Enforcement | PRIV | Ban all HTTP/network client libraries in tree | M1 | survey |
| 76 | Lean Release Profiles | DIST | Release profile: `lto="fat"`, `strip=true`, abort | M6 | survey |
| 77 | Cross-Platform CI & Packaging | DIST | GitHub Actions matrix, installer scripts | M6 | survey |

---

## Milestones
| # | Name | Scope | Dependencies | Status |
|---|------|-------|-------------|--------|
| M0 | Mermaid & Editor Feasibility Spike (GATE) | Verify native Mermaid rendering in Rust (`mermaid-rs-renderer` 0.3.1) across all 7 types, resvg rasterization, and editor 1MB/IME limits. Deliver `docs/MERMAID_SPIKE.md` and spike demo. | None | DONE |
| M1 | Workspace Skeleton, Core Vault Storage & Atomic I/O | Initialize Cargo workspace (`core`, `render`, `app`), data structures, atomic file writes, path sanitization, Windows reserved names guard, basic egui shell, theme, fonts, sidebar tree, open/edit/save, autosave. | M0 | IN_PROGRESS |
| M2 | Markdown Renderer, Code Highlighting & Preview | `pulldown-cmark` GFM parser, intermediate `DocModel`, `syntect` code highlighting, split/preview modes, scroll sync, local image loader, safe link opener. | M1 | PLANNED |
| M3 | Native Mermaid Rendering Pipeline & Cache | Full integration of native Mermaid parser, SVG generator, `resvg`/`tiny-skia` off-thread rasterizer, LRU texture cache, HiDPI scaling, dark/light theming, and graceful error fallback widget. | M2 | PLANNED |
| M4 | Vault Management, File Watcher, Search & UI Polish | `notify` watcher with debounce and self-write suppression, 3-way conflict dialog, fuzzy quick open (`Ctrl+P`), background full-text search (`Ctrl+Shift+F`), tabs/history, status bar, settings dialog. | M3 | PLANNED |
| M5 | Import & Export Pipelines | Safe folder, Obsidian, and `.zip` import (zip-slip & zip-bomb guards), HTML-to-MD import, conflict preview dialog, zip backup export, standalone HTML export with inline SVGs. | M4 | PLANNED |
| M6 | E2E Testing, Release Packaging & Final Hardening | Pass 100% of E2E test suite (Tiers 1-4), adversarial test hardening (Tier 5), `cargo test --workspace`, `cargo clippy -- -D warnings`, `cargo fmt --check`, dependency tree audit, memory <150MB audit, packaging workflows. | M5, TEST_READY | PLANNED |

---

## Interface Contracts

### `airgap-core` ↔ `airgap-app`
- **Vault Operations**:
  ```rust
  pub struct Vault { root: PathBuf }
  impl Vault {
      pub fn open(path: &Path) -> Result<Self, VaultError>;
      pub fn scan_tree(&self) -> Result<TreeNode, VaultError>;
      pub fn read_note(&self, relative_path: &Path) -> Result<NoteContent, VaultError>;
      pub fn write_note_atomic(&self, relative_path: &Path, content: &str) -> Result<(), VaultError>;
      pub fn create_note(&self, parent_dir: &Path, name: &str) -> Result<PathBuf, VaultError>;
      pub fn create_folder(&self, parent_dir: &Path, name: &str) -> Result<PathBuf, VaultError>;
      pub fn rename_item(&self, from: &Path, to_name: &str) -> Result<PathBuf, VaultError>;
      pub fn delete_item(&self, path: &Path) -> Result<(), VaultError>; // OS trash + .trash/ fallback
  }
  ```
- **Path Safety**:
  ```rust
  pub fn sanitize_path(vault_root: &Path, untrusted: &Path) -> Result<PathBuf, PathSafetyError>;
  pub fn validate_filename(name: &str) -> Result<(), PathSafetyError>; // Checks Windows reserved names, illegal chars, trailing dots/spaces
  ```
- **Search & Navigation**:
  ```rust
  pub fn quick_open_search(entries: &[PathBuf], query: &str) -> Vec<QuickOpenMatch>;
  pub fn global_full_text_search(vault_root: &Path, query: &str, sender: Sender<SearchResult>, cancel_flag: Arc<AtomicBool>);
  ```
- **Import / Export**:
  ```rust
  pub fn import_folder(source: &Path, target_vault: &Path, policy: ConflictPolicy) -> Result<ImportSummary, ImportError>;
  pub fn import_zip(archive_path: &Path, target_vault: &Path, policy: ConflictPolicy) -> Result<ImportSummary, ImportError>; // Zip-slip & Zip-bomb safe
  pub fn export_vault_zip(vault_root: &Path, destination_zip: &Path) -> Result<(), ExportError>;
  pub fn export_note_html(note_ast: &DocModel, title: &str, destination_html: &Path) -> Result<(), ExportError>;
  ```

### `airgap-render` ↔ `airgap-app`
- **Markdown & Code Rendering**:
  ```rust
  pub fn parse_markdown(markdown_text: &str) -> DocModel;
  pub fn highlight_code(code: &str, language: &str, is_dark: bool) -> Vec<ColoredSpan>;
  ```
- **Native Mermaid Pipeline**:
  ```rust
  pub struct DiagramRenderRequest {
      pub source: String,
      pub is_dark: bool,
      pub scale: f32,
      pub id: u64,
  }
  pub enum DiagramRenderResult {
      Success { id: u64, pixmap: Vec<u8>, width: u32, height: u32, scale: f32 },
      Error { id: u64, error_message: String, source: String },
  }
  pub fn render_mermaid_to_svg(source: &str, is_dark: bool) -> Result<String, MermaidError>;
  pub fn rasterize_svg_to_pixmap(svg_data: &str, scale: f32) -> Result<tiny_skia::Pixmap, RasterError>;
  ```

---

## Code Layout
```
airgap/
├── Cargo.toml                      # Workspace manifest
├── deny.toml                       # Zero-network dependency enforcement
├── crates/
│   ├── core/                       # airgap-core (Domain, storage, safety, search, import/export)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── vault.rs            # Directory tree model & 1:1 disk mapping
│   │       ├── io.rs               # Atomic write protocol (tempfile + fsync + rename)
│   │       ├── path_safety.rs      # Path traversal, Windows reserved names & character sanitization
│   │       ├── trash.rs            # OS recycle bin with .trash/ fallback
│   │       ├── watcher.rs          # notify debouncer & self-write journal
│   │       ├── search.rs           # Fuzzy quick open & background multi-threaded search
│   │       ├── importer.rs         # Folder, Obsidian, zip import with safety bounds
│   │       ├── exporter.rs         # Zip backup & standalone HTML export
│   │       └── config.rs           # Settings & session persistence
│   ├── render/                     # airgap-render (Headless Markdown & Mermaid pipeline)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── model.rs            # DocModel AST intermediate representation
│   │       ├── markdown.rs         # pulldown-cmark parser & GFM options
│   │       ├── highlight.rs        # syntect pure-Rust syntax highlighting
│   │       ├── mermaid.rs          # Native Mermaid diagram parser & SVG generator
│   │       ├── raster.rs           # resvg/usvg/tiny-skia SVG to RGBA pixmap converter
│   │       ├── cache.rs            # LRU content-hash diagram pixmap cache
│   │       └── fallback.rs         # Formatted code block diagnostic widget
│   └── app/                        # airgap-app (Desktop executable with egui/eframe)
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs             # Application entry point
│           ├── app.rs              # Central eframe::App implementation & update loop
│           ├── state.rs            # Application state model & action messages
│           ├── theme.rs            # Custom Dark & Light palettes, spacing, styling
│           ├── font.rs             # OFL bundled typography loader (Inter, JetBrains Mono)
│           ├── channels.rs         # Background worker channel definitions
│           ├── ui/
│           │   ├── mod.rs
│           │   ├── sidebar.rs      # Vault tree sidebar & context menu
│           │   ├── editor.rs       # Syntax-highlighted editor with line numbers & layouter
│           │   ├── preview.rs      # Preview pane with scroll sync & diagram widgets
│           │   ├── status_bar.rs   # Word/character counts, cursor position, save status
│           │   └── dialogs/
│           │       ├── conflict.rs # 3-way conflict resolution modal
│           │       ├── quick_open.rs # Ctrl+P fuzzy quick open modal
│           │       ├── search_modal.rs # Ctrl+Shift+F global search overlay
│           │       ├── import_modal.rs # Import summary & conflict policy modal
│           │       └── settings.rs # User preferences modal
│           └── crash.rs            # Panic hook & crash recovery buffer flusher
├── assets/                         # Bundled fonts, icons, app icon, sample vault
├── docs/                           # ARCHITECTURE.md, DECISIONS.md, MERMAID_SPIKE.md, QA.md, CHANGELOG.md
├── tests/                          # E2E integration test suite (Tiers 1-4)
└── .github/workflows/ci.yml        # Windows & Linux cross-platform CI matrix
```

