# Original User Request

## 2026-10-24T14:54:05Z

Build "AirGap", a fast, native, local-only Markdown note-taking app in Rust using egui/eframe for Windows and Linux, storing plain .md files on disk with zero network activity, live CommonMark/GFM preview, native Mermaid diagram rendering, folder management, quick open, and safe import/export.

Working directory: C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\airgap
Integrity mode: development

## Reference Material
- Detailed Product Specification & Architecture: C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\AGENT.md

## Requirements

### R1. Multi-Crate Workspace & Native GUI
Construct a Cargo workspace with distinct crates (`core`, `render`, `app`) in Rust (stable toolchain, 2021 edition or newer). The `core` crate contains all domain logic, file operations, indexing, and safety routines without any UI dependencies. The `render` crate parses Markdown (CommonMark + GFM) into document models and translates Mermaid syntax into rendered pixmaps. The `app` crate builds the desktop GUI using `egui` and `eframe`, featuring dark/light themes, custom typography, resizable panels, and non-blocking background workers.

### R2. Vault Management & Data Safety
Provide local directory vault management mapping 1:1 to disk directories with no external database. Implement atomic writes (temp file + fsync + rename), path sanitization against directory traversal / symlink escapes / Windows reserved filenames, and confirmation before deletion (with OS recycle bin via `trash` crate and `.trash/` directory fallback). Monitor vault directories using a debounced filesystem watcher (`notify`) and provide an interactive conflict resolution dialog (keep mine / take theirs / view diff) if external changes occur while unsaved edits exist.

### R3. Markdown Editor & Live Preview
Implement a high-performance Markdown editor with syntax highlighting, line numbers, word wrap, autosave (debounced ~1 s after typing, on focus loss, and on exit), and editing conveniences (list continuation, auto-pairing, smart URL paste). Provide three preview modes (editor only, split, preview only) with scroll synchronization, CommonMark + GFM rendering (tables, task lists, code blocks with syntax highlighting), and local image resolution.

### R4. Native Mermaid Diagram Rendering
Render Mermaid diagram code blocks (` ```mermaid `) natively without invoking any browser, webview, or JavaScript runtime engine. Rasterize diagrams to crisp RGBA textures at the active HiDPI scale using `resvg`/`tiny-skia` off the UI thread and cache textures by content hash and theme. Provide full coverage for flowcharts, sequence, class, state, entity-relationship, pie, and gantt diagrams. Ensure graceful error handling where malformed or unsupported diagrams render as formatted code blocks with explicit error diagnostic messages without crashing or blocking the UI.

### R5. Search, Import, Export & Release Packaging
Implement fuzzy quick open (Ctrl+P) and global full-text search across all vault notes running on a background worker. Support safe importing from folder trees, Obsidian vaults, and `.zip` archives with zip-slip / zip-bomb guards and conflict policies (skip, overwrite, keep both). Support vault backup export and single-note standalone HTML export with inline SVG diagrams. Provide release build configurations with LTO, stripping, and packaging scripts/CI workflows for Windows and Linux.

### R6. Local-Only Privacy & Integrity Constraints
The application must remain completely local. No network calls, telemetry, analytics, or background update pings are permitted. The only allowed outward action is opening a user-clicked URL in the default system browser via the OS launcher.

## Acceptance Criteria

### Correctness & Test Suite
- [ ] `cargo test --workspace` executes and passes all unit, integration, and property tests.
- [ ] `core` tests explicitly verify atomic writes, collision handling (e.g. `Untitled 1.md`), zip-slip path rejection, and Windows reserved name sanitization (CON, PRN, AUX, NUL, COM1-9, LPT1-9).
- [ ] `render` tests verify CommonMark/GFM formatting, code block highlighting, and Mermaid SVG/pixmap generation across all required diagram types (flowchart, sequence, class, state, ER, pie, gantt).
- [ ] Malformed Mermaid diagrams fall back gracefully with clear error notices and zero panics.
- [ ] Dependency tree audit (`cargo tree`) confirms absence of HTTP/network client libraries.

### Performance & Quality Standards
- [ ] Large notes (1 MB+) open and edit smoothly without UI thread lag.
- [ ] All intensive tasks (disk scans, search, imports, Mermaid rendering) run off the UI thread.
- [ ] Memory footprint remains under ~150 MB idle in release mode.
- [ ] Release binaries compile cleanly with `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --check`.


## 2026-10-24T16:16:17Z

User requested: "Pause the current teamwork session". Please pause/halt all active orchestrator tasks, subagents, and schedules immediately.

## 2026-10-25T00:13:29Z

Resume and continue the teamwork execution from where you stopped. Please drive through the remaining milestones (M2 through M6) as planned.
