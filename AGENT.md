# PROJECT BRIEF: "AirGap" (working name), a native, local-only Markdown note taker in Rust

You are building this from scratch. You have no prior context beyond this document. Everything below was decided with the product owner. Treat "Decisions" as fixed, "Open items" as yours to resolve and report, and "Non-goals" as out of scope.

## 1. Product summary
A lightweight, fast, beautiful, native desktop note-taking app for Linux and Windows.
- Notes are plain `.md` files in real folders on disk (no database, no lock-in).
- Markdown editing with live preview, including Mermaid diagram rendering, done natively (no web view, no browser, no JavaScript runtime).
- Local only: no network access, no accounts, no telemetry, no auto-update pings.
- Import and export of notes (the owner wrote "import and import"; interpreted as import and export).

## 2. Fixed decisions
- **Language:** Rust (stable toolchain, edition 2021 or newer).
- **Platforms:** Linux (X11 + Wayland) and Windows 10/11, both first-class. macOS is not a target but should not be gratuitously broken.
- **Native app, no web:** no Electron, Tauri, WebView, or embedded browser engine.
- **GUI toolkit:** `egui` / `eframe` (GPU-rendered). Beauty comes from a custom theme, spacing, bundled fonts, and polish. If you find a blocking limitation (see Open items), report it with evidence before switching; Slint is the fallback candidate.
- **Storage:** a user-chosen root directory ("vault"). The folder tree in the sidebar maps 1:1 to directories on disk. No database. Only a small config file lives in the OS config dir (use the `directories` crate).
- **Architecture:** Cargo workspace with separate crates so core logic is testable without a UI (see section 5).
- **Distribution:** a single small executable per OS, plus installers (see section 9).
- **Binary should be lean:** release profile with `lto = "fat"`, `codegen-units = 1`, `strip = true`, `panic = "abort"` where safe. Target idle RAM under ~150 MB and cold start under ~1 s on a mid-range machine. Report actual numbers.

## 3. Features

### 3.1 Editor
- Plain-text markdown editor (not WYSIWYG) with syntax highlighting for headings, emphasis, code spans/fences, links, lists, blockquotes, task lists, tables, and frontmatter.
- Line numbers (toggleable), word wrap (toggleable), monospace font, current-line highlight.
- Undo/redo with a sensible history, find/replace in the current note (case, whole-word, regex toggles).
- Autosave (debounced about 1 s after last keystroke, and on focus loss and exit) with a visible saved/unsaved indicator.
- Markdown conveniences: continue lists on Enter, Tab/Shift+Tab indent, auto-pair brackets/backticks, smart paste of URLs onto selected text to make `[text](url)`.
- Image paste: save clipboard images into an `assets/` folder next to the note and insert a relative link.

### 3.2 Preview
- Modes: editor only, split (default), preview only. Toggleable via shortcut and toolbar.
- Renders CommonMark + GFM: headings, emphasis, lists, task lists, tables, strikethrough, blockquotes, footnotes (nice-to-have), horizontal rules, links (open in the system browser only on explicit click; this is the only permitted outward action), images (local relative paths), fenced code blocks with syntax highlighting (`syntect` with a pure-Rust regex backend, or `tree-sitter-highlight`; pick the lighter one and justify it).
- Scroll sync between editor and preview (best effort; approximate by source line mapping).
- ` ```mermaid ` fenced blocks render as diagrams (see section 4).

### 3.3 Folder and note management
- Sidebar tree: create, rename, delete, and drag-and-drop move for notes and folders; expand/collapse state remembered; sort by name (folders first); context menu; keyboard navigation.
- Delete goes to a recycle mechanism (use the `trash` crate for the OS trash, with a fallback to a `.trash/` folder inside the vault). Always confirm destructive actions.
- Filesystem watcher (`notify` crate, debounced) so external changes (git pull, other editors) appear live. If the open note changes on disk while it has unsaved edits, show a conflict prompt (keep mine / take theirs / view diff). Never silently overwrite.
- New note creates `Untitled.md`, with a uniqueness suffix on collision, and focuses rename.
- Wiki-style `[[note]]` links: nice-to-have, post-v1.

### 3.4 Search and navigation
- Quick open (Ctrl+P): fuzzy filename match (`nucleo` or `fuzzy-matcher`).
- Global full-text search (Ctrl+Shift+F): run in a background thread, stream results, show snippets. A simple parallel scan (`ignore` + `grep-searcher`/`regex`, or `rayon`) is fine for v1; add an index only if scanning is demonstrably too slow on a 10k-note vault.
- Recent notes list, and restore the last open note and layout on launch.

### 3.5 Import
- Folder of `.md` / `.txt` files (copy into the vault, preserving structure).
- Obsidian vault (copy; keep `[[wikilinks]]` as text; skip `.obsidian/`).
- `.zip` archive of markdown (use the `zip` crate; guard against zip-slip path traversal and zip bombs by capping total and per-file size).
- HTML to Markdown (e.g. `htmd` or `html2md`; evaluate and pick one).
- Conflict policy dialog: skip / overwrite / keep both (rename with suffix). Show a preview summary (N files, N conflicts) before committing. Run in a background thread with progress and cancel.

### 3.6 Export
- Export a folder or the whole vault as a `.zip` backup or a plain folder copy.
- Export a single note to HTML (self-contained, with embedded CSS; Mermaid diagrams embedded as inline SVG; images as data URIs optional).
- PDF export: lowest priority, after v1. Suggested route: render the HTML pipeline to PDF through a pure-Rust path (e.g. `printpdf` or `typst`-based) only if it can be done without a browser. Otherwise defer and document.

### 3.7 UX and polish
- Dark and light themes plus follow-system; a carefully designed palette, consistent spacing scale, rounded corners, subtle shadows, smooth hover/selection states, and a clean typography stack. Bundle fonts (a quality sans for UI, a quality mono for the editor, and emoji/CJK fallback strategy) so the look is identical on both OSes. Check licenses (prefer OFL fonts).
- Layout: resizable sidebar, tab bar or a single-document view with a back/forward history (choose one and justify it; tabs are preferred if cheap), status bar with word/char count and save state.
- Keyboard-first: Ctrl+S, Ctrl+N, Ctrl+Shift+N (new folder), Ctrl+P, Ctrl+Shift+F, Ctrl+F, Ctrl+B/I (bold/italic), Ctrl+\ (toggle preview), F2 (rename), Del (delete), Ctrl+, (settings). Use Cmd on macOS where trivially supported.
- Settings: vault location, theme, fonts and sizes, autosave interval, line numbers, wrap, spellcheck (post-v1).
- HiDPI correct on both OSes; text must be crisp. Also test at 100%, 150%, 200% scale.
- Accessibility basics: sufficient contrast, scalable UI font size, keyboard operability. (egui's AccessKit support is a bonus.)

## 4. Mermaid: the highest-risk item (do this FIRST)
Mermaid is normally a JavaScript library that requires a browser DOM. We have excluded web tech, so rendering must be native.

**Approach:**
1. Convert Mermaid source to SVG with a native Rust implementation. A pure-Rust crate named `mermaid-rs-renderer` is believed to exist but is young; I (the previous agent) have not verified it. You must verify its existence, maintenance status, license, diagram-type coverage, and output quality. Also survey alternatives (other Rust Mermaid ports, or a minimal in-house renderer for the most common types built on a layout crate such as `layout-rs`/`dagre`-like algorithms).
2. Rasterize SVG with `resvg` + `usvg` + `tiny-skia` into an RGBA texture; upload to egui as a texture; cache by hash of (source, theme, scale) so diagrams only re-render when changed; render off the UI thread.
3. Theme-aware: diagram colors must adapt to light/dark.
4. Re-render crisply at the current HiDPI scale (rasterize at pixel size, not at 1x then scale).
5. **Graceful degradation:** if a diagram type is unsupported or fails to parse, show the source in a styled code block with a clear inline message ("Unsupported diagram type" or the parse error with line number). The app must never crash or freeze on bad Mermaid input. Wrap rendering in `catch_unwind` and a timeout/size guard.

**v1 target diagram types:** flowchart/graph, sequence, class, state, ER, pie, gantt. Others (mindmap, timeline, journey, git graph, etc.) are later.

**Gate:** deliver a spike report before building the rest: a table of diagram types vs. rendered correctly / partially / unsupported, with screenshots, a performance measurement (time per diagram), and a recommendation. If native coverage is too weak, present options to the owner: (a) accept limited coverage with graceful fallback, (b) contribute upstream, (c) write a focused in-house renderer for flowchart + sequence first. Do not silently introduce a browser or JS engine. (An embedded pure-Rust JS engine such as `boa` running Mermaid would also need a DOM shim; treat that as a last resort and report it as a decision for the owner.)

## 5. Architecture (Cargo workspace)
```
app/
  Cargo.toml                (workspace)
  crates/
    core/       vault + tree model, path safety, atomic file I/O, trash, watcher abstraction,
                search, import, export, config. NO UI deps. Heavily unit-tested.
    render/     markdown -> intermediate document model (pulldown-cmark, GFM options),
                code highlighting, mermaid -> SVG -> pixmap, texture cache. UI-agnostic
                where possible (produces pixmaps/models, not egui widgets).
    app/        egui/eframe binary: theme, sidebar, editor, preview, dialogs, settings,
                command palette/quick open, background-task plumbing.
  assets/       fonts, icons, app icon, sample vault
  docs/         ARCHITECTURE.md, DECISIONS.md, MERMAID_SPIKE.md, CHANGELOG.md
  .github/workflows/ci.yml
```
Key design rules:
- UI thread never blocks on disk, search, import/export, or Mermaid rendering. Use worker threads/channels (`std::thread` + `crossbeam-channel`, or a small thread pool), and call `ctx.request_repaint()` on completion.
- Single source of truth for application state with explicit messages/commands (a simple Elm-like update loop is welcome) to keep egui immediate-mode code manageable.
- Preview rendering: evaluate the `egui_commonmark` crate first. If it cannot support custom blocks (Mermaid), tables, or the theme you need, write your own renderer from `pulldown-cmark` events to egui widgets. Decide and document in DECISIONS.md.
- Editor: start with `egui::TextEdit::multiline` plus a custom `layouter` for highlighting. Evaluate its limits (large files, line numbers, find highlighting, performance on 1 MB notes, selection/IME behavior). If inadequate, evaluate alternatives (`egui_code_editor`, a custom text widget, or `cosmic-text`-based editing) and report before committing. IME input (CJK) must work.
- Keep dependencies modest and audited. Run `cargo deny` / `cargo audit` in CI. Prefer pure-Rust crates so cross-platform builds stay trivial.

## 6. Data safety and security (non-negotiable)
- **Atomic writes:** write to a temp file in the same directory, `fsync`, then rename over the target (handle Windows replace semantics). Never leave a half-written note.
- **Path safety:** every path from UI/import/archives is canonicalized and verified to be inside the vault root; reject `..` escapes, absolute paths in archives, and handle symlinks deliberately (do not follow symlinks out of the vault).
- **Windows specifics:** reserved names (CON, NUL, COM1...), illegal characters, trailing dots/spaces, case-insensitive collisions, long paths (>260; use the `\\?\` handling or manifest long-path opt-in), CRLF vs LF (preserve a file's existing line endings; default LF for new files), UTF-8 with BOM tolerance.
- **Linux specifics:** non-UTF-8 filenames, permissions errors, Wayland and X11 both work, file-watcher inotify limits (fall back gracefully).
- Never delete or overwrite user data without confirmation; imports never modify the source.
- No network code at all except opening a clicked link via the OS (`open` crate). CI should assert there is no HTTP client dependency in the tree.
- Crash resilience: on panic, write a crash log to the config dir and attempt to flush unsaved buffers to a recovery file; offer recovery on next launch.

## 7. Non-goals for v1
Cloud sync, accounts, collaboration, plugins, tags/backlinks/graph view, mobile, WYSIWYG editing, spellcheck, PDF export (best-effort only), math/LaTeX (post-v1), a database or search index (unless proven necessary).

## 8. Milestones (work in this order; stop and report at each gate)
- **M0. Mermaid spike (GATE):** see section 4. Deliver `docs/MERMAID_SPIKE.md` and a minimal demo window rendering sample diagrams in light/dark. Also run a quick spike on editor feasibility (egui TextEdit with highlighting at 1 MB and a CJK IME check) and report.
- **M1. Skeleton:** workspace, CI, theme, fonts, window, vault picker, sidebar tree from disk, open/edit/save with atomic writes, autosave.
- **M2. Preview:** markdown render, code highlighting, split/preview modes, scroll sync, images, links.
- **M3. Mermaid integration:** wire the chosen renderer with caching, theming, HiDPI, error fallback.
- **M4. Management and navigation:** create/rename/move/delete with trash, watcher + conflict handling, quick open, global search, find/replace, tabs or history, shortcuts, settings.
- **M5. Import and export:** all sources and targets in section 3.5 and 3.6 (PDF deferred), progress UI, conflict dialog.
- **M6. Polish and release:** visual design pass, HiDPI testing, performance tuning, accessibility pass, installers, CI release artifacts, README with screenshots.

## 9. Packaging and CI
- GitHub Actions matrix: `ubuntu-latest` and `windows-latest`; run `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`, `cargo deny`, and release builds. Cache with `Swatinem/rust-cache`.
- Artifacts: Windows `.msi` (via `cargo-wix` or `cargo-packager`) plus a portable `.exe`; Linux AppImage plus `.deb` (via `cargo-packager`/`cargo-deb`). Include an app icon and `.desktop` file. Document the system packages needed to build on Linux (e.g. X11/Wayland/xkbcommon/GL dev packages for eframe).
- Version in `Cargo.toml`; keep a CHANGELOG.

## 10. Testing and quality bar
- Unit tests in `core` for path safety, atomic write, collision naming, import conflict policies, zip-slip rejection, and Windows reserved names. Property tests (`proptest`) for path normalization are welcome.
- Golden tests in `render`: markdown to model snapshots (`insta`), Mermaid to SVG snapshots for the supported types.
- A `sample-vault/` with varied notes (nested folders, big note, tables, every Mermaid type, images, CJK text, odd filenames) used for manual and automated checks.
- Manual QA checklist in `docs/QA.md` run on both Linux and Windows before release.
- Performance budgets: open a 1 MB note without UI stutter; vault with 10k notes lists in under 1 s; typing latency imperceptible; Mermaid render never blocks the UI.

## 11. How to work
- Do not ask the owner questions unless truly blocked; make a reasonable decision, record it in `docs/DECISIONS.md` (decision, alternatives, why), and continue.
- Verify every crate's current version, maintenance status, and license on crates.io/docs.rs before depending on it; do not trust names in this brief blindly. Prefer permissive licenses (MIT/Apache-2.0/BSD); flag anything GPL/LGPL/AGPL.
- Commit in small, logical steps with clear messages. Keep the build green.
- After each milestone, report: what was done, what was verified (with evidence such as test output, screenshots, and measurements), deviations from this brief, and risks.
- You can only test what you can actually run. State clearly which platforms you were able to verify and which are untested.

## 12. Open items for you to resolve and report
1. Mermaid renderer: coverage, quality, and licensing (gate at M0).
2. Whether egui `TextEdit` is adequate for the editor or needs a replacement.
3. Preview renderer: `egui_commonmark` vs. custom.
4. Tabs vs. single-document with history.
5. Syntax highlighter choice (`syntect` pure-Rust backend vs. `tree-sitter`), judged on binary size and speed.
6. HTML to Markdown crate choice for import.
7. Feasibility of PDF export without a browser.
8. Final app name (working name "AirGap"; check for trademark/crate-name conflicts).

## 13. Definition of done for v1
A user on Windows or Linux can install the app, choose a folder, create nested folders and notes, write Markdown with live preview, see Mermaid flowchart/sequence/class/state/ER/pie/gantt diagrams rendered natively (with graceful fallback otherwise), search, import from a folder/Obsidian vault/zip/HTML, export to zip/HTML, and trust that their files are never corrupted or lost, with no network activity and a polished, fast, themeable UI.