# AirGap — Complete Specification & Feature Inventory

**Document Version:** 1.0.0  
**Author:** spec_miner_survey_1  
**Timestamp:** 2026-10-24T15:01:00Z  
**Source Specifications:**
- `ORIGINAL_REQUEST.md` (Authoritative User Request)
- `AGENT.md` (Product Specification & Architecture)

---

## 1. Executive Summary & Vision

**AirGap** is a high-performance, native, local-only Markdown note-taking desktop application built with Rust and `egui`/`eframe` for Windows (10/11) and Linux (X11 & Wayland). 

AirGap addresses modern privacy concerns and tool bloat by enforcing four inviolable design principles:
1. **Zero Network / Air-Gapped Privacy:** Zero network calls, zero telemetry, zero analytics, zero background update requests. All HTTP and networking client libraries are banned from the dependency tree. The only permitted outward action is delegating a user-clicked hyperlink to the default operating system browser.
2. **True Local 1:1 Storage:** Notes are stored directly as plain `.md` files in standard filesystem directories (a "vault"). There is no proprietary database, no lock-in, and no hidden metadata files that prevent standard Git workflows or external file editing.
3. **Native Non-Web Engine:** Completely free of Chromium, Electron, Tauri, WebViews, and JavaScript runtimes. Markdown parsing and Mermaid diagram rendering are 100% native in Rust, rendering to GPU-backed textures.
4. **Ironclad Data Safety:** Multi-layered data protection including atomic write routines (temp file + fsync + rename), path sanitization against directory traversal / zip-slip / symlink escapes / Windows reserved filenames, confirmation prompts on destructive actions, OS recycle bin with `.trash/` fallback, and debounced filesystem watching with interactive conflict resolution.

---

## 2. Multi-Crate Workspace Architecture

The project is structured as a Cargo workspace with clean separation of concerns, ensuring core domain logic is 100% decoupled from GUI frameworks:

```
airgap/
  Cargo.toml                (Workspace configuration)
  crates/
    core/                   (Domain models, path safety, atomic I/O, watcher, search, import/export, config)
    render/                 (Markdown AST/document model, code syntax highlighting, Mermaid->SVG->pixmap pipeline)
    app/                    (egui/eframe GUI, themes, layout, editor widgets, dialogs, async task channels)
  assets/                   (OFL bundled fonts, icons, app logo, sample vault)
  docs/                     (ARCHITECTURE.md, DECISIONS.md, MERMAID_SPIKE.md, QA.md, CHANGELOG.md)
  .github/workflows/ci.yml  (Cross-platform matrix CI for Windows and Linux)
```

### Crate Boundaries & Responsibilities

| Crate | Responsibilities | Constraints / Dependencies |
|-------|------------------|----------------------------|
| `airgap-core` | Vault directory tree model, atomic file read/write, path safety validation, trash routing, `notify` filesystem watcher abstraction, background full-text search, archive/folder import & export pipelines, app configuration storage (`directories` crate). | **Strictly NO UI dependencies**. No `egui`, no `winit`. Must be 100% unit and property testable in headless CI. |
| `airgap-render` | Markdown parsing (CommonMark + GFM via `pulldown-cmark`), code block syntax highlighting (`syntect` pure-Rust regex or `tree-sitter`), Mermaid syntax parsing & SVG generation, SVG rasterization to RGBA pixmaps via `resvg`/`usvg`/`tiny-skia`, pixmap texture caching by hash/theme/scale. | UI-agnostic where possible. Generates pixmaps and intermediate render models, not direct `egui` widgets. Graceful fallback on syntax errors. |
| `airgap-app` | Desktop GUI entry point (`eframe`/`egui`), custom dark/light/system themes, typography loading, resizable panel layout (sidebar, editor, preview, status bar), tabs / document history, dialogs (conflicts, imports, settings), background worker coordination (`crossbeam-channel` or `mpsc`). | Coordinates asynchronous background jobs off the UI thread; calls `ctx.request_repaint()` upon job completion. |

---

## 3. Features Discovered

The following table comprehensively catalogues every feature identified across `ORIGINAL_REQUEST.md` and `AGENT.md`.

| # | Category | Feature | Description | Inputs | Outputs | Error Behavior | Discovered Via | Priority | Verification Criteria |
|---|----------|---------|-------------|--------|---------|----------------|----------------|----------|-----------------------|
| 1 | ARCH | Multi-Crate Workspace | Cargo workspace partitioning core domain logic, render pipeline, and desktop GUI | Cargo configuration, Rust stable 2021+ | Compilable workspace with `core`, `render`, `app` | Compilation failure if UI leaked into `core` | R1, Sec 2, Sec 5 | P0 (Must) | `cargo check --workspace` passes; `core` has zero UI crate dependencies. |
| 2 | ARCH | Non-Blocking UI Thread | All disk I/O, full-text searches, import/export, and Mermaid rasterization executed on background worker threads | User actions, worker requests | Repaint event (`request_repaint()`), async results | Logged worker errors, failure notifications | R1, Sec 5, Sec 10 | P0 (Must) | UI maintains 60+ FPS while opening 1 MB file, rasterizing diagrams, or searching 10k notes. |
| 3 | ARCH | Elm-Style App State | Single source of truth application state driven by explicit messages/commands | UI interactions, background worker messages | State transitions, UI render pass | Invariant assertions, predictable state | Sec 5 | P1 (Should) | Deterministic state updates, no split-brain state between sidebar and editor. |
| 4 | VAULT | Local 1:1 Directory Mapping | Vault tree maps directly to real filesystem directory hierarchy without database | Directory path selection | Hierarchical tree of folders and `.md` files | Error dialog if root path inaccessible or unreadable | R2, Sec 1, Sec 2, Sec 3.3 | P0 (Must) | Files created in OS file manager appear in sidebar tree; changes reflect 1:1 on disk. |
| 5 | VAULT | Note Creation | Create new note defaulting to `Untitled.md` with auto-incrementing collision suffix | "New Note" action (`Ctrl+N`), active directory | Created file on disk, tree node added, focused for rename | Error notice if directory read-only | Sec 3.3 | P0 (Must) | Pressing `Ctrl+N` creates `Untitled.md`; next creates `Untitled 1.md`; rename field focused. |
| 6 | VAULT | Folder Creation | Create new subdirectory inside vault root or selected folder | "New Folder" action (`Ctrl+Shift+N`), active folder | Created folder on disk, tree node added, focused for rename | Error notice if invalid path or permissions error | Sec 3.3 | P0 (Must) | New folder created immediately on disk; appears in sidebar. |
| 7 | VAULT | In-Place Renaming | Rename note or folder via inline UI or keyboard shortcut (`F2`) | New filename string, target path | Renamed path on disk, updated tree node, updated open tab | Validation error prompt on collision or invalid characters | Sec 3.3 | P0 (Must) | Renaming file on disk updates tab title; rejects invalid characters with warning. |
| 8 | VAULT | Safe Deletion & Recycle Bin | Delete note/folder with confirmation dialog; routes to OS trash or `.trash/` fallback | "Delete" action (`Del`), target item | File moved to OS recycle bin via `trash` crate, or `.trash/` folder | Aborts if user cancels confirmation dialog; fallback on trash error | R2, Sec 3.3 | P0 (Must) | File is not permanently unlinked; moved to OS trash or `.trash/`; dialog prevents accidental deletion. |
| 9 | VAULT | Drag-and-Drop Reorganization | Drag notes and folders in sidebar tree to move them into different directories | Mouse drag-drop event between tree nodes | File/folder moved on disk via safe atomic rename | Rejects drop if target is invalid or self/descendant | Sec 3.3 | P1 (Should) | Moving file updates disk location and preserves note contents. |
| 10 | VAULT | Tree State Persistence | Remember expanded and collapsed state of folders across app restarts | Sidebar fold/unfold toggles, app shutdown | Saved folder expansion state in config file | Fallback to collapsed or root-expanded on missing config | Sec 3.3, Sec 4 | P1 (Should) | Restarting app preserves previously opened folders in sidebar. |
| 11 | VAULT | Sidebar Sorting & Context Menu | Sort folders first, then alphabetical; right-click context menu (New, Rename, Delete, Reveal in OS) | Right-click mouse event, directory contents | Context popup menu, sorted tree rendering | N/A | Sec 3.3 | P1 (Should) | Folders display at top; files alphabetical below; context menu allows full CRUD operations. |
| 12 | SAFE | Atomic Note Writes | Write note to temp file in same directory, `fsync`, and atomic rename over target | Editor content buffer, target path | Durably written `.md` file on disk | Temp file cleaned up if write or fsync fails; original file untouched | R2, Sec 6 | P0 (Must) | Unit tests verify process killed mid-write leaves target file unmodified and intact. |
| 13 | SAFE | Strict Path Sanitization | Canonicalize paths and enforce strict containment within vault root; block traversal | File paths from UI, imports, or watcher | Verified `PathBuf` within vault boundary | Returns `Err(PathSafetyError::TraversalAttempted)` | R2, Sec 6 | P0 (Must) | Rejects paths containing `..`, absolute outside paths, or escaped symlinks. |
| 14 | SAFE | Windows Reserved Filename Guard | Sanitize and reject reserved device names `CON`, `PRN`, `AUX`, `NUL`, `COM1-9`, `LPT1-9` | Input filename string on creation/rename/import | Safe sanitized name or validation rejection | Explicit error message returned to user | R2, Sec 6, Sec 10 | P0 (Must) | Attempting to create `con.md`, `aux.txt`, or `NUL` rejected with actionable error. |
| 15 | SAFE | Windows Filename Sanitization | Reject illegal characters (`<`, `>`, `:`, `"`, `/`, `\`, `|`, `?`, `*`) and trailing dots/spaces | Input filename string | Sanitized name or error prompt | Blocks invalid character input in rename field | Sec 6 | P0 (Must) | Trailing spaces and dots stripped or rejected; illegal characters blocked. |
| 16 | SAFE | Windows Long Path Support | Transparently support file paths exceeding 260 characters on Windows | Deeply nested file path | Extended-length path (`\\?\`) or manifest longPathAware | Returns descriptive error if OS path limit hard fails | Sec 6 | P1 (Should) | Note nested 300 characters deep opens and saves without `ERROR_PATH_NOT_FOUND`. |
| 17 | SAFE | Line Ending Preservation | Detect and preserve existing CRLF vs LF line endings; default new files to LF | File content buffer, existing file metadata | Preserved line endings in saved file | Normalizes mixed line endings cleanly | Sec 6 | P1 (Should) | File created on Windows with CRLF retains CRLF after editing and saving. |
| 18 | SAFE | UTF-8 BOM Tolerance | Seamlessly handle UTF-8 files with Byte Order Mark (`\u{FEFF}`) without artifact display | Raw file bytes with BOM | Clean text buffer; BOM preserved or handled safely | Non-UTF-8 files report graceful decode error | Sec 6 | P1 (Should) | Note saved with BOM displays text cleanly without leading `ï»¿` artifacts. |
| 19 | SAFE | Filesystem Watcher (`notify`) | Debounced background filesystem monitoring for external modifications | Filesystem events from OS | UI updates reflecting external additions, renames, deletes | Gracefully handles inotify limit exhaustion on Linux | R2, Sec 3.3, Sec 6 | P0 (Must) | External git pull or CLI file edit updates sidebar tree and editor content. |
| 20 | SAFE | Conflict Resolution Dialog | Interactive conflict resolution if active note modified externally with unsaved edits | External change event, dirty buffer state | User choice: "Keep Mine", "Take Theirs", "View Diff" | Never silently clobbers user edits or external changes | R2, Sec 3.3 | P0 (Must) | Editing file while modifying on disk triggers 3-way dialog; user edits preserved. |
| 21 | SAFE | Crash Recovery Buffer | On panic, flush dirty editor buffers to recovery location in config directory | Application panic hook trigger | Recovery dump file on disk; recovery dialog on next boot | Logs error details to `crash.log` in config directory | Sec 6 | P1 (Should) | Intentional crash flushes unsaved text; next launch prompts to restore draft. |
| 22 | EDIT | Syntax-Highlighted Editor | Plain-text Markdown editor with syntax highlighting for headings, emphasis, code, links, tables | Markdown text content | Rendered text with token formatting & colors | Graceful fallback to unstyled text on parsing glitch | R3, Sec 3.1 | P0 (Must) | Headings, bold, italic, inline code, code fences, and links highlighted in editor. |
| 23 | EDIT | Line Numbers & Word Wrap | Toggleable line numbering gutter and soft word wrap in editor pane | View settings toggles | Formatted editor viewport with gutter and wrapped lines | N/A | R3, Sec 3.1 | P0 (Must) | Toggling line numbers shows/hides gutter; toggling wrap wraps long paragraphs. |
| 24 | EDIT | Current-Line Highlight | Subtle background highlighting of the active cursor line | Cursor position change | Highlighted line rectangle behind text | N/A | Sec 3.1 | P1 (Should) | Active line visually differentiated from surrounding text. |
| 25 | EDIT | Undo / Redo History | Full undo/redo history stack for editing operations | `Ctrl+Z`, `Ctrl+Y` / `Ctrl+Shift+Z` | Restored editor buffer state | N/A | Sec 3.1 | P0 (Must) | Keystrokes, deletions, and pastes reversibly undone and redone. |
| 26 | EDIT | In-Note Find & Replace | Find and replace toolbar with match case, whole word, and regex search | Search query, replace query, match options | Highlighted matches, next/previous jump, replace action | Reports invalid regex syntax without crashing | Sec 3.1 | P0 (Must) | Searching string highlights matches; replace updates instances accurately. |
| 27 | EDIT | Autosave Mechanism | Autosaves note ~1s after last keystroke, on window focus loss, and on app exit | Keystroke idle timer, focus loss, exit signal | Atomic file write to disk; updates saved status | Retains unsaved indicator if write fails | R3, Sec 3.1 | P0 (Must) | Pausing typing for 1s writes file to disk; switching windows triggers immediate save. |
| 28 | EDIT | Saved / Unsaved Visual Indicator | UI status indicator in tab header or status bar showing dirty vs clean buffer state | Editor buffer modification | "Saved" vs "Unsaved" / dot indicator | N/A | Sec 3.1 | P0 (Must) | Typing shows dot/indicator; autosave clears indicator to "Saved". |
| 29 | EDIT | List Continuation on Enter | Pressing Enter inside a bullet, numbered, or task list automatically creates next item | Enter key press on list line | New list line with matching prefix (`- `, `* `, `1. `, `- [ ] `) | Pressing Enter on empty list item removes prefix | Sec 3.1 | P1 (Should) | Enter on `- item` generates `- `; Enter on empty `- ` exits list. |
| 30 | EDIT | Tab / Shift+Tab Indentation | Indent or outdent active line or multi-line selection by tab width | Tab or Shift+Tab key press | Indented / outdented text block | Clamps at column 0 on outdent | Sec 3.1 | P1 (Should) | Tab indents selected lines; Shift+Tab unindents. |
| 31 | EDIT | Auto-Pairing Enclosures | Automatically insert closing bracket, parenthesis, quote, or backtick | Typing `(`, `[`, `{`, `"`, `` ` `` | Paired character inserted with cursor placed between | Overtyping closing character advances cursor | Sec 3.1 | P1 (Should) | Typing `[` produces `[]` with cursor inside; typing backtick inside produces ```` ````. |
| 32 | EDIT | Smart URL Paste | Pasting a URL with text selected automatically wraps selection into `[text](url)` | Clipboard paste with URL format and active selection | Formatted Markdown link string | Standard paste if no text selected or clipboard is not URL | Sec 3.1 | P1 (Should) | Selecting word "Rust" and pasting `https://rust-lang.org` produces `[Rust](https://rust-lang.org)`. |
| 33 | EDIT | Image Paste from Clipboard | Pasting bitmap image from clipboard saves PNG to `assets/` and inserts relative link | Clipboard image paste (`Ctrl+V`) | PNG written to `./assets/image_<timestamp>.png`, link inserted | Error notification if clipboard image format unsupported | Sec 3.1 | P1 (Should) | Copying screenshot and pasting in editor saves image to `assets/` and inserts `![image](assets/...)`. |
| 34 | EDIT | CJK & IME Input Support | Full support for complex East Asian character input methods (Chinese, Japanese, Korean) | IME composition events | Composed unicode characters inserted into buffer | IME composition window rendered at cursor | Sec 5, Sec 10, Sec 12 | P0 (Must) | User can input Japanese/Chinese characters via system IME without dropped keystrokes. |
| 35 | EDIT | Large Note Scalability | Open and edit 1 MB+ plain-text Markdown notes without UI thread stutter or memory lag | 1 MB+ `.md` file | Smooth editing experience, low latency typing | Graceful degradation of heavy syntax features if needed | Sec 5, Sec 10 | P0 (Must) | 1 MB note (approx. 20,000 lines) opens in <500ms; typing latency imperceptible. |
| 36 | PREV | Triple View Modes | Toggle between Editor Only, Split View (default), and Preview Only | View mode button or shortcut (`Ctrl+\`) | Viewport layout repartitioning | N/A | R3, Sec 3.2 | P0 (Must) | `Ctrl+\` cycles or toggles between editor, split, and preview views. |
| 37 | PREV | CommonMark + GFM Rendering | High-fidelity rendering of headings, bold, italic, strikethrough, lists, blockquotes, HR | Markdown AST from `pulldown-cmark` | Formatted egui layout elements with styled typography | Render errors logged; fallback to raw text | R3, Sec 3.2 | P0 (Must) | All standard CommonMark and GFM constructs render with correct typography and styling. |
| 38 | PREV | Markdown Tables | Render GFM tables with column alignment (left, center, right), headers, cell padding | Table Markdown AST | Grid/table layout with borders and aligned cells | Gracefully handles uneven rows/columns | R3, Sec 3.2 | P0 (Must) | Markdown tables render as neat tabular grids with appropriate column alignments. |
| 39 | PREV | Task Lists | Render interactive or styled GFM task checkboxes (`- [ ]`, `- [x]`) | Task list Markdown AST | Checkbox UI elements reflecting completion state | N/A | R3, Sec 3.2 | P0 (Must) | Checked and unchecked task items render with distinctive visual boxes. |
| 40 | PREV | Code Blocks with Highlighting | Render fenced code blocks with language syntax highlighting (`syntect` or `tree-sitter`) | Fenced code block with language tag | Formatted code box with colored text and background | Falls back to plain monospace if language unknown | R3, Sec 3.2, Sec 12 | P0 (Must) | Rust, Python, JSON, and Bash code blocks render with correct syntax colors. |
| 41 | PREV | Local Relative Images | Resolve and display local relative image files (PNG, JPEG, WebP, SVG) | Relative image path (`./assets/pic.png`) | Loaded pixmap rendered in preview pane | Displays broken image placeholder if file missing | R3, Sec 3.2 | P0 (Must) | Images saved relative to note render crisply in preview pane. |
| 42 | PREV | Safe Hyperlink Handling | Links open in default OS browser on explicit click; absolute prohibition on webview navigation | Link click event | Invocation of `open::that(url)` via OS launcher | Error notification if URL format invalid | R3, R6, Sec 3.2, Sec 6 | P0 (Must) | Clicking external link opens system browser; no network request made inside app. |
| 43 | PREV | Editor-Preview Scroll Sync | Approximate synchronized scrolling between editor and preview panes | Scroll position changes in editor or preview | Proportional or source-line scroll adjustment | Best-effort synchronization without jumpiness | R3, Sec 3.2 | P1 (Should) | Scrolling editor keeps corresponding preview section visible in split mode. |
| 44 | MERM | Native Mermaid Engine | Render ` ```mermaid ` code blocks natively without browser, webview, or JS runtime | Mermaid diagram code string | SVG AST / vector graph representation | Graceful error fallback box (see Feature 50) | R4, Sec 4 | P0 (Must) | Zero browser/webview dependencies; diagrams compiled directly in Rust. |
| 45 | MERM | Flowchart / Graph Support | Parse and render Mermaid flowcharts (`graph TD`, `flowchart LR`, subgraphs, node shapes) | Flowchart Mermaid syntax | Rendered vector flowchart | Syntax error box on malformed graph | R4, Sec 4 | P0 (Must) | Renders standard flowcharts with boxes, rounded nodes, arrows, and labels. |
| 46 | MERM | Sequence Diagram Support | Parse and render Mermaid sequence diagrams (actors, messages, activation, loops) | Sequence diagram Mermaid syntax | Rendered sequence diagram | Syntax error box on malformed sequence | R4, Sec 4 | P0 (Must) | Renders participants, lifelines, solid/dashed arrows, and note boxes. |
| 47 | MERM | Class Diagram Support | Parse and render Mermaid class diagrams (classes, methods, attributes, relations) | Class diagram Mermaid syntax | Rendered class diagram | Syntax error box on malformed class | R4, Sec 4 | P0 (Must) | Renders class boxes with member lists and inheritance/composition arrows. |
| 48 | MERM | State Diagram Support | Parse and render Mermaid state diagrams (`stateDiagram`, `stateDiagram-v2`) | State diagram Mermaid syntax | Rendered state machine diagram | Syntax error box on malformed state | R4, Sec 4 | P0 (Must) | Renders initial/final states, state boxes, transitions, and fork/join bars. |
| 49 | MERM | ER, Pie & Gantt Support | Parse and render Entity-Relationship (`erDiagram`), Pie (`pie`), and Gantt (`gantt`) charts | ER, Pie, or Gantt Mermaid syntax | Rendered diagram/chart | Graceful fallback on unsupported keywords | R4, Sec 4 | P0 (Must) | Renders ER entities with cardinality, pie chart slices with percentages, Gantt tasks. |
| 50 | MERM | Graceful Error Fallback | Malformed or unsupported diagrams render as styled code blocks with line/error diagnostics | Invalid Mermaid syntax | Formatted code box with inline error diagnostic banner | Application never crashes or freezes | R4, Sec 4, Sec 10 | P0 (Must) | Broken syntax displays formatted code with "Parse error on line X: [reason]". |
| 51 | MERM | Resvg Rasterization & Texture Upload | Rasterize generated SVG via `resvg` + `usvg` + `tiny-skia` into RGBA texture for egui | SVG markup string, scale factor | `egui::TextureHandle` uploaded to GPU | Returns error texture if rasterization fails | R4, Sec 4 | P0 (Must) | SVG rendered into clean RGBA pixmap; displayed in preview pane. |
| 52 | MERM | Off-Thread Background Rasterization | Background worker thread executes Mermaid parsing and rasterization | Diagram source string, theme, scale | Completed texture sent via channel; `request_repaint()` | Rendering timeouts capped; never hangs UI thread | R4, Sec 4, Sec 5 | P0 (Must) | Complex diagrams render asynchronously without blocking typing or scrolling. |
| 53 | MERM | Theme-Aware Diagram Styling | Adapt diagram palette dynamically to match active light or dark theme | Active theme enum (`Dark` vs `Light`) | Color-adjusted SVG palette (strokes, fills, text) | Default high-contrast fallback colors | Sec 4 | P0 (Must) | In dark theme, background is dark, nodes dark gray, text white; in light theme, inverted. |
| 54 | MERM | HiDPI Crisp Rasterization | Rasterize diagrams at current device scale factor (100%, 125%, 150%, 200%) | Device pixel ratio / zoom factor | Full-resolution pixmap matched to physical pixels | N/A | R4, Sec 4, Sec 10 | P0 (Must) | Diagram on 200% 4K screen is tack-sharp; no blurry 1x upscaled bitmap artifacts. |
| 55 | MERM | Content-Hashed Texture Cache | Cache rasterized textures keyed by `(hash(source), theme, scale)` | Diagram cache key tuple | Cached texture handle | Evicts stale textures when LRU limit reached | R4, Sec 4 | P0 (Must) | Unchanged diagrams render instantly from cache without re-parsing or re-rasterizing. |
| 56 | SRCH | Fuzzy Quick Open (`Ctrl+P`) | Instant modal overlay for fuzzy filename and path matching across the vault | Query string (`Ctrl+P`), note file paths | Ranked list of matching notes; Enter to open | Empty list state if no matches | R5, Sec 3.4 | P0 (Must) | Typing "arch" matches `docs/ARCHITECTURE.md`; opens file immediately on Enter. |
| 57 | SRCH | Background Global Full-Text Search | Asynchronous multi-threaded search across all notes in vault (`Ctrl+Shift+F`) | Search query, vault file tree | Streamed matching results with file path, line, snippet | Error notice if search thread panicked | R5, Sec 3.4, Sec 5 | P0 (Must) | Searches 10k notes in parallel off UI thread; streams results with contextual snippets. |
| 58 | SRCH | Recent Notes History | Track and display list of recently opened notes for quick access | File open events | MRU list of recent note paths in UI / menu | Prunes deleted or moved files automatically | Sec 3.4 | P1 (Should) | Reopening app or pressing recent files shows previously active notes. |
| 59 | SRCH | Layout & Session State Restore | Restore last active note, expanded folders, and panel split sizes on startup | Session state file in OS config dir | Restored workspace layout | Fallback to default layout if session corrupt | Sec 3.4, Sec 4 | P1 (Should) | Closing and reopening app returns user directly to previous note and view state. |
| 60 | IMP | Plain Folder Import | Copy external directory of `.md` and `.txt` files into vault preserving folder hierarchy | Source folder path, destination vault folder | Imported files and subdirectories on disk | Previews conflicts; never modifies source folder | R5, Sec 3.5 | P0 (Must) | Folder structure preserved; non-text assets handled cleanly. |
| 61 | IMP | Obsidian Vault Import | Import Obsidian notes, preserving directory tree and `[[wikilinks]]` as text, skipping `.obsidian/` | Source Obsidian root path | Cleaned note tree imported into vault | Skips internal Obsidian cache and config files | R5, Sec 3.5 | P0 (Must) | Imports notes cleanly; `.obsidian` directory ignored completely. |
| 62 | IMP | Safe Zip Archive Import | Extract `.zip` archive of notes with zip-slip path traversal and zip-bomb protections | Archive file path, destination directory | Extracted notes in vault | Aborts extraction on malicious entry or oversize | R5, Sec 3.5, Sec 6 | P0 (Must) | Unit tests verify zip containing `../../evil.sh` or 100:1 compression ratio rejected. |
| 63 | IMP | HTML to Markdown Import | Convert external `.html` documents to formatted Markdown via pure-Rust engine (`htmd`) | HTML file path or content | Generated `.md` file in vault | Graceful conversion fallback on malformed HTML tags | Sec 3.5, Sec 12 | P1 (Should) | HTML tables, headings, links, and bold text convert to standard Markdown. |
| 64 | IMP | Import Conflict Resolution Modal | Preview summary (N files, N conflicts) and policy selection: Skip, Overwrite, Keep Both | Conflict policy selection | Resolved file actions with rename suffix on "Keep Both" | User cancel leaves vault completely untouched | Sec 3.5 | P0 (Must) | Dialog displays conflict count before committing; "Keep Both" appends suffix `(1)`. |
| 65 | EXP | Vault / Folder Zip Backup | Export entire vault or selected folder as compressed `.zip` backup archive | Source vault folder, target `.zip` path | Output `.zip` file containing all notes & assets | Error notification if output path unwritable | R5, Sec 3.6 | P0 (Must) | Produces valid zip archive verifiable by standard unzip tools. |
| 66 | EXP | Vault / Folder Directory Copy | Export vault or folder as plain filesystem copy to external directory | Source folder, target directory | Exact replica directory tree | Error notification on permissions failure | Sec 3.6 | P1 (Should) | Creates clean folder copy preserving all files and subdirectories. |
| 67 | EXP | Standalone HTML Note Export | Export single note to self-contained HTML with embedded CSS and inline SVG diagrams | Active note AST, output file path | Standalone `.html` file with embedded styling & SVGs | Error notification if write fails | R5, Sec 3.6 | P0 (Must) | Exported HTML opens in any browser with identical formatting and sharp vector diagrams. |
| 68 | UIUX | Dual Polished Themes | Meticulously designed Dark and Light themes plus "Follow System" automatic mode | System theme signals or user selection | Consistent UI colors, contrast ratios, and spacing | Fallback to Dark theme if system theme unreadable | R1, Sec 2, Sec 3.7 | P0 (Must) | Clean visual distinction between light and dark; WCAG AA contrast compliance. |
| 69 | UIUX | Bundled Offline Typography | High-quality bundled OFL fonts for UI (Sans) and Editor (Monospace) with CJK fallbacks | Embedded font binary assets in app | Consistent, identical typography on Linux & Windows | Fallback to system font if custom font load fails | Sec 2, Sec 3.7 | P0 (Must) | App displays identical crisp fonts regardless of OS installed fonts. |
| 70 | UIUX | Resizable Multi-Pane Layout | Resizable sidebar, split editor/preview divider, and responsive panel sizing | User mouse drag on panel dividers | Dynamically resized panels with persisted widths | Clamps minimum pane sizes to prevent UI collapse | R1, Sec 2, Sec 3.7 | P0 (Must) | Dragging sidebar divider smoothly resizes sidebar; panel sizes remembered. |
| 71 | UIUX | Tab Bar or Document History | Multi-tab document bar or single-document view with forward/back navigation history | Note switch actions, tab close buttons | Open note tabs or navigated history stack | Prompt to save before closing dirty tab if autosave off | Sec 3.7, Sec 12 | P1 (Should) | Allows switching between multiple open notes quickly without losing scroll state. |
| 72 | UIUX | Status Bar Metrics | Bottom status bar displaying word count, character count, cursor line:col, and save state | Active note buffer, cursor position | Live updating stats in bottom bar | N/A | Sec 3.7 | P1 (Should) | Typing updates word and character counts in real time. |
| 73 | UIUX | Keyboard-First Shortcuts | Comprehensive keyboard shortcuts (Ctrl+S, N, Shift+N, P, Shift+F, F, B, I, \, F2, Del, ,) | Keyboard event dispatch | Associated command execution | Shortcut hints visible in menus and tooltips | Sec 3.7 | P0 (Must) | All primary workflows operable entirely via keyboard without mouse. |
| 74 | UIUX | Settings Panel | In-app preferences: vault path, theme, font families/sizes, autosave interval, wrap, line numbers | User settings modifications | Serialized JSON/TOML config in OS config directory | Graceful fallback to default values on invalid input | Sec 3.7 | P1 (Should) | Modifying font size or theme in settings applies live to application immediately. |
| 75 | PRIV | Zero-Network Enforcement | Strict architectural and dependency ban on network calls, telemetry, and background pings | Crate dependencies, network calls | Zero network sockets opened | Automated CI audit fails if network crate detected | R6, Sec 1, Sec 6, Sec 10 | P0 (Must) | `cargo tree` audit proves absence of `reqwest`, `hyper`, `curl`, `ureq`; zero socket opens. |
| 76 | DIST | Lean Release Profiles | Release compilation with `lto = "fat"`, `codegen-units = 1`, `strip = true`, `panic = "abort"` | Cargo release profile configuration | Compact native executable (<25 MB) | Compilation error if release flags conflict | Sec 2, Sec 9 | P0 (Must) | Release binary optimized, stripped, cold boots in <1s, idle RAM <150 MB. |
| 77 | DIST | Cross-Platform CI & Packaging | GitHub Actions CI for Windows and Linux; generates `.exe` / `.msi` and AppImage / `.deb` | Git push / tag event, CI runners | Tested artifacts, installers, release packages | CI build fails if formatting, clippy, or tests fail | Sec 9, Sec 10 | P0 (Must) | Matrix CI runs clippy, fmt, tests, and builds installers on Windows and Linux. |

---

## 4. Edge Cases & Boundary Conditions

| # | Feature / Area | Edge Case Input / Condition | Expected / Observed Behavior | Mitigation & Implementation Strategy |
|---|----------------|-----------------------------|------------------------------|--------------------------------------|
| 1 | Safe I/O | Windows reserved names (`CON`, `PRN`, `AUX`, `NUL`, `COM1-9`, `LPT1-9`) | Reject creation, rename, or import of filenames matching reserved words in any case (e.g. `aux.md`, `NUL.txt`, `com1`) | Validation regex/lookup in `core::path_safety`. Reject with user-friendly error `ReservedWindowsName`. |
| 2 | Safe I/O | Windows illegal characters (`< > : " / \ | ? *`) in note/folder names | Attempting to create or rename item with illegal characters fails immediately | Validate filename input against illegal character set before touching filesystem; block keypress or show error tooltip. |
| 3 | Safe I/O | Windows trailing dots and spaces (e.g. `note. `, `folder...`) | Windows filesystem automatically strips trailing dots/spaces, causing path mismatch bugs | Trim or reject trailing dots and spaces during path validation prior to file creation. |
| 4 | Safe I/O | Path length exceeding 260 characters on Windows (MAX_PATH limit) | Standard Windows file APIs fail with `PathNotFound` or `FilenameTooLong` | Utilize UNC extended path prefix (`\\?\`) for all internal Windows path operations, or enable `longPathAware` in application manifest. |
| 5 | Safe I/O | Symlinks pointing outside vault root directory | Symlink in vault targets `/etc/passwd` or `C:\Windows` | Canonicalize path targets; verify resolved canonical path is prefixed by canonical vault root. Reject if outside. |
| 6 | Safe I/O | Circular symlinks within vault directory | Recursive folder scanner enters infinite loop | Maintain a `HashSet<PathBuf>` of visited canonical directories during recursive scans; break on cycles. |
| 7 | Safe I/O | Case-insensitive name collisions (e.g. `Note.md` vs `note.md`) | On Windows (case-insensitive NTFS), overwrites existing note; on Linux (case-sensitive ext4), creates two distinct notes | Enforce case-insensitive uniqueness check in vault model to ensure vaults remain portable between Windows and Linux. |
| 8 | Safe I/O | Line ending preservation (CRLF vs LF) | Opening file with Windows CRLF and saving must not convert all lines to LF (polluting git diffs) | Detect dominant line ending upon file load (`\r\n` vs `\n`). Re-serialize using the detected line ending. Default new notes to LF. |
| 9 | Safe I/O | UTF-8 Byte Order Mark (BOM: `0xEF, 0xBB, 0xBF`) | Note contains UTF-8 BOM at start; editor might display `ï»¿` or crash JSON/frontmatter parsers | Strip BOM during in-memory text decoding; preserve BOM flag in note metadata to write it back if desired. |
| 10 | Safe I/O | Non-UTF-8 or binary files in vault folder | User drops `.pdf`, `.exe`, or legacy ISO-8859 text file into vault | Check UTF-8 validity on read. If invalid, display read-only warning: "Binary or non-UTF-8 file cannot be opened as text". |
| 11 | Safe I/O | Power outage / process kill mid-save | Partial write leaves corrupted or 0-byte note on disk | Atomic write pattern: write to `temp_file` in same folder, execute `fsync()`, then atomically rename over target path. |
| 12 | Safe I/O | Read-only file or permission denied | User attempts to save note with read-only file permissions | Catch write error, preserve unsaved changes in memory buffer, display non-modal error banner: "Permission denied saving file". |
| 13 | Safe I/O | Linux inotify descriptor exhaustion | Large vault (>10k directories) exceeds system `fs.inotify.max_user_watches` | Handle `notify` watcher failure gracefully; fall back to manual poll or display warning rather than crashing. |
| 14 | Safe I/O | External file deletion while open in editor | File is deleted outside app (e.g. `rm note.md` or git branch switch) | Watcher detects deletion; mark note as "(Deleted on disk)" in editor tab; prompt user whether to recreate or close tab. |
| 15 | Safe I/O | External modification with unsaved edits | Active note edited externally while editor has unsaved memory buffer | Display 3-way Conflict Dialog: "Keep Mine" (overwrite disk), "Take Theirs" (reload disk), or "View Diff". Never silently overwrite. |
| 16 | Editor | Opening 1 MB+ plain-text note (e.g. 25,000 lines) | UI thread freezes if syntax highlighter parses entire document on every frame | Use viewport-based line virtualization; only highlight visible lines or memoize token spans incrementally. |
| 17 | Editor | Empty (0-byte) note | Creating or opening an empty note | Correctly initializes empty text buffer and empty preview without indexing panics or out-of-bounds slicing. |
| 18 | Editor | CJK IME composition at bottom of long document | Input composition window positioned incorrectly or off-screen | Query cursor coordinate transformation from `egui` to ensure system IME composition candidate box anchors at cursor. |
| 19 | Editor | Smart URL paste onto multiline selection | User selects multiple lines and pastes URL | Detect multiline selection; avoid creating invalid broken Markdown link syntax. Wrap entire selection or paste raw URL. |
| 20 | Editor | Image paste when `assets/` folder does not exist | Pasting clipboard screenshot when `assets/` subdirectory is missing | Auto-create `./assets/` directory relative to current note if not present; generate timestamped filename `image_YYYYMMDD_HHMMSS.png`. |
| 21 | Preview | Relative image path with spaces or URL encoding | `![alt](assets/my%20image.png)` or `![alt](assets/my image.png)` | Decode percent-encoded paths before resolving against filesystem path. Verify file existence before loading pixmap. |
| 22 | Preview | Broken or missing relative image | Note references `./assets/missing.png` which does not exist | Display stylized missing image placeholder box with filename rather than panicking or breaking layout. |
| 23 | Preview | Table with mismatched columns (e.g. 3 headers, 5 data cells) | Malformed Markdown table | Pad missing cells with empty content or truncate extra cells gracefully to maintain neat table grid alignment. |
| 24 | Mermaid | Malformed Mermaid syntax (unclosed brackets, syntax errors) | User types invalid diagram syntax (e.g. `graph TD; A-->`) | Intercept parser error; render styled code block containing source with inline error diagnostic banner indicating line/column. Zero panic. |
| 25 | Mermaid | Unsupported diagram type (e.g. `gitGraph`, `mindmap`, `timeline`) | User pastes post-v1 or experimental Mermaid diagram type | Gracefully render formatted code block with banner: "Unsupported diagram type: gitGraph (v1 supports flowchart, sequence, class, state, ER, pie, gantt)". |
| 26 | Mermaid | Giant Mermaid diagram (1,000 nodes) causing OOM / timeout | Massive diagram could consume gigabytes of RAM during layout or rasterization | Wrap layout/rasterization in timeout and node count guard; wrap execution in `std::panic::catch_unwind`; fail with error banner if timeout exceeded. |
| 27 | Mermaid | High-DPI monitor switch (100% to 200%) | Moving application window from standard 1080p monitor to 4K display | Monitor scale change; re-render cached textures at new physical pixel resolution so diagram remains crisp without blur. |
| 28 | Mermaid | Rapid typing inside Mermaid code block | User types continuously inside diagram fence, triggering dozens of re-renders per second | Debounce diagram re-render requests (~300ms); discard outdated background render jobs when new source hash arrives. |
| 29 | Import | Zip-slip path traversal attempt in `.zip` archive | Zip archive contains entries with `../../../../evil.exe` or `C:\Windows\System32\cmd.exe` | Inspect every zip entry path; sanitize and verify canonicalized destination is strictly within import target folder. Reject on violation. |
| 30 | Import | Zip-bomb archive (decompression expansion attack) | Zip archive with 10 KB compressed size expanding into 50 GB | Cap maximum decompressed file size (e.g. 50 MB per file) and total uncompressed archive size (e.g. 500 MB). Abort if cap exceeded. |
| 31 | Import | Malformed or invalid HTML file import | HTML file contains unclosed tags, binary garbage, or script tags | Use robust pure-Rust HTML parser (`htmd`/`html2md`); strip `<script>`, `<style>`, and `<iframe>` tags; convert semantic tags to Markdown. |
| 32 | Search | Regex denial of service (ReDoS) in find/replace or search | Catastrophic backtracking regex input (e.g. `(a+)+$`) | Pure-Rust `regex` crate uses linear time finite automata, immune to catastrophic backtracking. Capped match time / result limits. |
| 33 | Search | Massive vault with 100,000 files | Global search or tree loading causes memory ballooning or UI freeze | Stream search results asynchronously in chunks via channels; virtualize sidebar tree rendering to only layout visible nodes. |
| 34 | Network | Dependency tree pollution with HTTP clients | Accidental dependency addition pulls in `reqwest` or `hyper` | Continuous CI assertion running `cargo tree -i reqwest -i hyper -i ureq -i curl` to fail the build if any HTTP crate is found. |

---

## 5. Non-Functional Requirements & Performance Budgets

### NFR-1. Performance & Responsiveness Budgets
1. **Cold Startup Time:** Cold launch to interactive UI in under **1.0 second** on mid-range hardware (NVMe SSD, 4-core CPU).
2. **Idle Memory Footprint:** Memory usage under **150 MB RAM** while idle in release mode.
3. **Large Note Capability:** Plain-text Markdown notes up to **1 MB (~25,000 lines)** open and edit smoothly with no typing latency or UI thread frame drops.
4. **Vault Scaling:** Vaults containing **10,000 notes** list and populate in under **1.0 second**; global search across 10,000 notes completes in seconds via background thread streaming.
5. **UI Rendering Frame Rate:** UI thread maintains smooth 60 FPS rendering during continuous scrolling, text selection, and window resizing.

### NFR-2. Build & Binary Footprint
1. **Release Compilation Configuration:**
   ```toml
   [profile.release]
   lto = "fat"
   codegen-units = 1
   strip = true
   panic = "abort"
   opt-level = 3
   ```
2. **Binary Size:** Compact standalone executable under **25 MB** (including bundled OFL fonts and syntax definitions).
3. **Pure-Rust Priority:** Maximize pure-Rust dependencies to eliminate C-library runtime linking dependencies (avoiding OpenSSL, fontconfig, or GTK runtime version mismatches on Linux).

### NFR-3. Privacy & Security Non-Negotiables
1. **Zero Outward Network Traffic:** The application process must never bind listening ports, establish outbound TCP/UDP connections, or make HTTP/HTTPS calls.
2. **Local Isolation:** All settings, notes, caches, and crash logs reside entirely within the vault folder and standard OS configuration directories (`directories` crate).
3. **External Link Guard:** External hyperlinks only open via the OS shell launcher (`open::that(...)`) upon explicit user mouse click.

### NFR-4. Cross-Platform Compatibility
1. **Windows 10 & 11:** First-class native GUI support; standard Windows path semantics, UNC extended path handling, OS recycle bin integration.
2. **Linux (X11 & Wayland):** First-class native GUI support via `eframe` winit backend; proper font rendering, freedesktop trash specification support, standard desktop entry files.
3. **macOS:** Not a formal target for release packaging, but code must maintain portable standards without breaking compilation (`Cmd` key mappings provided alongside `Ctrl`).

---

## 6. Open Technical Decisions & Spikes (from AGENT.md §12)

The specification identifies 8 open technical items that require evaluation and resolution:

| Item # | Open Technical Topic | Context / Options | Recommendation / Decision Criteria |
|--------|----------------------|-------------------|-------------------------------------|
| **Item 1** | **Mermaid Renderer (GATE M0)** | Native pure-Rust Mermaid rendering without browser DOM. Options: existing crates (`mermaid-rs-renderer`), porting layout algorithms (`layout-rs`), or minimal in-house renderer for core 7 types. | Complete spike report (`MERMAID_SPIKE.md`). Verify license (prefer MIT/Apache-2.0), coverage of flowchart, sequence, class, state, ER, pie, gantt. Ensure graceful fallback on unsupported types. |
| **Item 2** | **Editor Text Widget** | `egui::TextEdit::multiline` with custom layouter vs dedicated code editor (`egui_code_editor` or `cosmic-text`). | Benchmark `egui::TextEdit` with custom syntax highlighter at 1 MB note size. Verify CJK IME composition behavior. If IME or 1MB performance stutters, evaluate `egui_code_editor` or custom rope-backed widget. |
| **Item 3** | **Preview Renderer Architecture** | `egui_commonmark` crate vs custom `pulldown-cmark` to `egui` widget renderer. | Test `egui_commonmark` for custom code block interception (` ```mermaid `) and custom table styling. If extensible, adopt; otherwise build lightweight custom renderer mapping `pulldown-cmark` events directly to `egui::Ui` widgets. |
| **Item 4** | **Document View Model** | Multi-document tab bar vs single-document view with forward/back history. | Tab bar preferred by product owner if implementation overhead is reasonable. Provide multi-tab bar with close buttons and Ctrl+Tab / Ctrl+W shortcuts. |
| **Item 5** | **Syntax Highlighter Engine** | `syntect` (pure-Rust regex backend) vs `tree-sitter-highlight`. | Evaluate binary size impact and compilation time. `syntect` with pure-Rust `fancy-regex` is mature, self-contained, and provides standard Sublime text themes without C compiler toolchain dependencies. |
| **Item 6** | **HTML to Markdown Import Engine** | `htmd` vs `html2md` crates. | Compare maintenance status, test suite, and handling of complex HTML tables/formatting. `htmd` is modern and active; verify license and pure-Rust nature. |
| **Item 7** | **PDF Export Feasibility** | Defer post-v1 vs pure-Rust PDF pipeline (`printpdf` or `typst`). | Explicitly defer post-v1 per product specification. Focus v1 strictly on high-quality standalone HTML export with embedded SVG diagrams. |
| **Item 8** | **Application Name & Branding** | Working name "AirGap"; potential crate name conflicts on crates.io. | Keep "AirGap" as application title; name workspace crates `airgap-core`, `airgap-render`, `airgap-app` or use private workspace naming (`publish = false`). |

---

## 7. Non-Goals for v1 (Explicitly Out of Scope)

Per `AGENT.md §7`, the following features are strictly **non-goals** for v1 and must not be implemented:
1. Cloud synchronization (Dropbox, Google Drive, OneDrive, or custom sync servers).
2. User accounts, login, authentication, or passwords.
3. Multi-user real-time collaboration.
4. Third-party plugin or extension runtime architecture.
5. Bidirectional backlinks graph visualization / knowledge graph view.
6. Mobile applications (iOS, Android).
7. Full WYSIWYG rich-text editing (AirGap is a plain-text Markdown editor with live preview).
8. Integrated spellchecking dictionary (deferred to post-v1).
9. Math / LaTeX formula rendering (KaTeX/MathJax; deferred to post-v1).
10. Embedded relational or document database (SQLite, RocksDB; plain files on disk only).

---

## 8. Definition of Done & Acceptance Criteria

### Correctness & Automated Test Suite
- [ ] `cargo test --workspace` executes and passes all unit, integration, and property tests.
- [ ] `core` tests explicitly verify:
  - Atomic writes (temp file + fsync + rename).
  - Path canonicalization and traversal rejection (`..`, symlinks escaping root).
  - Windows reserved name sanitization (`CON`, `PRN`, `AUX`, `NUL`, `COM1-9`, `LPT1-9`).
  - Collision suffix handling (`Untitled.md` -> `Untitled 1.md`).
  - Zip-slip rejection in archive extraction.
  - Zip-bomb decompression caps.
  - Line ending preservation (CRLF and LF).
- [ ] `render` tests explicitly verify:
  - CommonMark and GFM document model parsing.
  - Table parsing and cell alignment.
  - Code block syntax highlighting.
  - Mermaid diagram generation for flowchart, sequence, class, state, ER, pie, and gantt.
  - Malformed Mermaid syntax handling with diagnostic errors and zero panics.
- [ ] Dependency tree audit (`cargo tree`) confirms complete absence of HTTP/network client libraries (`reqwest`, `hyper`, `curl`, `ureq`).

### Performance & Quality Standards
- [ ] Large notes (1 MB+) open and edit smoothly without UI thread lag.
- [ ] All intensive tasks (disk scans, search, imports, Mermaid rendering) run asynchronously off the UI thread.
- [ ] Idle memory footprint remains under ~150 MB in release mode.
- [ ] Clean compilation with `cargo clippy --workspace --all-targets -- -D warnings`.
- [ ] Clean code formatting with `cargo fmt --check`.
- [ ] Cross-platform CI matrix passes on both `windows-latest` and `ubuntu-latest`.

---
*Report compiled and certified by `spec_miner_survey_1`.*
