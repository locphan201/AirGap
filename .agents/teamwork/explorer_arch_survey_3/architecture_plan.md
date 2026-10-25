# AirGap — Architecture Design, Workspace Layout & Core Algorithms

**Document Version:** 1.0.0  
**Author:** explorer_arch_survey_3  
**Status:** Approved Architectural Specification  
**Target Root:** `C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\airgap`  
**Reference Documents:** `ORIGINAL_REQUEST.md`, `AGENT.md`, `spec_miner_survey_1/requirements.md`

---

## 1. Executive Summary & Architectural Principles

**AirGap** is an offline, native, GPU-rendered Markdown desktop application designed for Windows 10/11 and Linux (X11 & Wayland), built using Rust (edition 2021) and the `egui`/`eframe` GUI toolkit. 

The application is engineered around four core architectural pillars:
1. **Air-Gapped Privacy Guarantee:** Strictly zero network sockets, telemetry, background HTTP requests, or external dependencies. All data is processed locally. Hyperlinks clicked by the user are safely delegated to the default OS browser via standard OS shell invocation (`open` crate) without embedded browser runtimes or network stacks.
2. **True 1:1 Filesystem Vault:** Notes exist on disk as plain `.md` files in real directories. There is no proprietary database, SQLite layer, or hidden file lock-in. Vaults can be synchronized via Git, Syncthing, or file managers transparently.
3. **Decoupled Multi-Crate Architecture:** A strict tripartite Cargo workspace separation (`core`, `render`, `app`) where domain safety, indexing, and I/O logic are 100% independent of any graphical or windowing framework, ensuring headless testability and predictable invariants.
4. **Resilient Concurrency & Non-Blocking UI:** The GUI thread never blocks on disk I/O, regex searching, diagram compilation, or archive unpacking. Background workers communicate via lock-free `crossbeam-channel` message passing and wake the UI loop with `ctx.request_repaint()`.

---

## 2. Cargo Workspace Layout & Release Optimization

### 2.1 Workspace Directory Tree
The workspace root is located at `airgap/`:

```
airgap/
├── Cargo.toml                      # Workspace manifest & profile configuration
├── deny.toml                       # Cargo-deny security & network ban policy
├── crates/
│   ├── core/                       # Domain logic, vault model, I/O, safety, search, import/export
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── vault.rs            # Vault struct, directory management, collision resolver
│   │       ├── tree.rs             # In-memory tree model, node hierarchy, natural sorting
│   │       ├── atomic_io.rs        # Atomic write protocol (tempfile, fsync, rename)
│   │       ├── path_safety.rs      # Path sanitizer, traversal, symlink & Windows reserved names
│   │       ├── trash.rs            # Trash subsystem (OS recycle bin + .trash/ fallback)
│   │       ├── watcher.rs          # Notify debouncer & internal write suppression journal
│   │       ├── search/
│   │       │   ├── mod.rs
│   │       │   ├── quick_open.rs   # Fuzzy filename matcher (nucleo/fuzzy-matcher)
│   │       │   └── full_text.rs    # Multi-threaded streaming text search (rayon/regex)
│   │       ├── import/
│   │       │   ├── mod.rs
│   │       │   ├── folder.rs       # Directory & Obsidian importer
│   │       │   ├── zip.rs          # Zip archive importer with Zip-Slip & Zip-Bomb guards
│   │       │   └── html.rs         # HTML to Markdown converter (htmd)
│   │       ├── export/
│   │       │   ├── mod.rs
│   │       │   ├── zip.rs          # Zip backup export
│   │       │   └── html.rs         # Standalone HTML export with inline SVG & CSS
│   │       └── config.rs           # Persistent settings & state (directories crate)
│   ├── render/                     # Markdown parsing, syntax highlighting, Mermaid rendering
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── markdown.rs         # pulldown-cmark parser & GFM extensions
│   │       ├── model.rs            # Intermediate DocModel, blocks, inline spans & line mapping
│   │       ├── highlight.rs        # syntect syntax highlighter (pure-Rust regex backend)
│   │       ├── mermaid/
│   │       │   ├── mod.rs          # Mermaid pipeline dispatcher & error boundary
│   │       │   ├── types.rs        # Diagram types (Flowchart, Sequence, Class, State, ER, Pie, Gantt)
│   │       │   ├── layout.rs       # Native layout & SVG generator
│   │       │   ├── theme.rs        # Light / Dark theme styling & palette injection
│   │       │   └── rasterize.rs    # resvg + usvg + tiny-skia HiDPI rasterization
│   │       └── cache.rs            # LRU pixmap cache keyed by (content_hash, theme, scale)
│   └── app/                        # egui / eframe desktop application binary
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs             # Application entry point & crash handler
│           ├── app.rs              # AirGapApp state & top-level egui update loop
│           ├── channels.rs         # Worker threads, message passing & command dispatch
│           ├── conflict.rs         # Conflict resolution state machine & dialogs
│           ├── ui/
│           │   ├── mod.rs
│           │   ├── theme.rs        # Visual palette, spacing scale, widget styling
│           │   ├── fonts.rs        # Bundled OFL fonts loader (Sans, Mono, Fallbacks)
│           │   ├── menu_bar.rs     # Top action bar & mode selectors
│           │   ├── sidebar.rs      # Vault tree view, collapse/expand, context menu, drag-and-drop
│           │   ├── editor.rs       # Multiline text editor, layouter, line numbers, word wrap
│           │   ├── preview.rs      # DocModel renderer, tables, code blocks, images, scroll sync
│           │   ├── modals.rs       # Quick open overlay, search panel, import dialog, settings
│           │   └── status_bar.rs   # Cursor position, word count, character count, save state
│           └── recovery.rs         # Emergency buffer recovery & crash logging
├── assets/
│   ├── fonts/                      # Bundled OFL fonts (Inter, JetBrains Mono)
│   ├── icons/                      # App icon & UI SVG/PNG assets
│   └── sample_vault/               # Golden test vault containing all edge cases
└── docs/
    ├── ARCHITECTURE.md
    ├── DECISIONS.md
    └── QA.md
```

### 2.2 Root `Cargo.toml`
```toml
[workspace]
members = [
    "crates/core",
    "crates/render",
    "crates/app",
]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"
rust-version = "1.75"
license = "MIT OR Apache-2.0"
authors = ["AirGap Team"]

[profile.dev]
opt-level = 0
debug = true

[profile.release]
opt-level = 3
lto = "fat"
codegen-units = 1
strip = true
panic = "abort"

# Performance profile for development profiling
[profile.profiling]
inherits = "release"
strip = false
debug = true
```

#### Rationale for Release Profile Flags:
- **`opt-level = 3`**: Maximizes aggressive vectorization, loop unrolling, and inline optimizations.
- **`lto = "fat"`**: Full Link-Time Optimization performs cross-crate dead code elimination and inlining across `core`, `render`, `app`, and all dependencies.
- **`codegen-units = 1`**: Instructs LLVM to optimize the entire crate graph as a single compilation unit, eliminating boundary overhead at the cost of longer build times.
- **`strip = true`**: Automatically strips symbol tables and debuginfo from the final binary, shrinking output binary size from ~45 MB to ~9–14 MB.
- **`panic = "abort"`**: Removes stack unwinding tables and code generation overhead. In `render`, panic protection for external parsing (such as Mermaid syntax or SVG layout) is handled by isolating untrusted work inside worker threads or structured `Result` error boundaries.

---

## 3. `crates/core`: Vault Model, Safety & File Operations

### 3.1 Crate Manifest (`crates/core/Cargo.toml`)
```toml
[package]
name = "airgap-core"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
# Serialization & Config
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
directories = "5.0"

# Safety & Filesystem
tempfile = "3.10"
trash = "5.1"
notify = "6.1"
notify-debouncer-mini = "0.4"
walkdir = "2.5"

# Search & Concurrency
rayon = "1.10"
regex = "1.10"
fuzzy-matcher = "0.3"

# Archive & HTML Import/Export
zip = { version = "1.1", default-features = false, features = ["deflate"] }
htmd = "0.1"

# Utility & Error Handling
thiserror = "1.0"
tracing = "0.1"
chrono = "0.4"

[dev-dependencies]
proptest = "1.4"
```

### 3.2 Vault & Tree Hierarchy Data Structures
The filesystem tree is represented as an in-memory recursive graph updated incrementally via filesystem watcher events.

```rust
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub u64);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vault {
    pub root_path: PathBuf,
    pub name: String,
    pub tree: VaultTree,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultTree {
    pub root_folder: FolderNode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderNode {
    pub id: NodeId,
    pub name: String,
    pub rel_path: PathBuf,
    pub children: Vec<VaultNode>,
    pub is_expanded: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileNode {
    pub id: NodeId,
    pub name: String,
    pub rel_path: PathBuf,
    pub size_bytes: u64,
    pub modified: SystemTime,
    pub is_markdown: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VaultNode {
    Folder(FolderNode),
    File(FileNode),
}

impl VaultNode {
    pub fn name(&self) -> &str {
        match self {
            VaultNode::Folder(f) => &f.name,
            VaultNode::File(f) => &f.name,
        }
    }

    pub fn rel_path(&self) -> &Path {
        match self {
            VaultNode::Folder(f) => &f.rel_path,
            VaultNode::File(f) => &f.rel_path,
        }
    }
}
```

#### Natural Sorting Algorithm:
Directories must always precede files, with alphanumeric natural sorting (e.g. `2.md` appears before `10.md`):
```rust
impl FolderNode {
    pub fn sort_children(&mut self) {
        self.children.sort_by(|a, b| {
            match (a, b) {
                (VaultNode::Folder(_), VaultNode::File(_)) => std::cmp::Ordering::Less,
                (VaultNode::File(_), VaultNode::Folder(_)) => std::cmp::Ordering::Greater,
                _ => natural_lexicographical_cmp(a.name(), b.name()),
            }
        });
        for child in &mut self.children {
            if let VaultNode::Folder(ref mut sub) = child {
                sub.sort_children();
            }
        }
    }
}
```

#### Collision-Free Note Naming:
When creating `Untitled.md`, collision detection generates `Untitled 1.md`, `Untitled 2.md`, etc.:
```rust
pub fn resolve_unique_note_name(parent_dir: &Path, base_name: &str) -> String {
    let candidate = format!("{}.md", base_name);
    if !parent_dir.join(&candidate).exists() {
        return candidate;
    }
    let mut counter = 1;
    loop {
        let candidate = format!("{} {}.md", base_name, counter);
        if !parent_dir.join(&candidate).exists() {
            return candidate;
        }
        counter += 1;
    }
}
```

### 3.3 Atomic Write Protocol & Crash Safety
Atomic writes prevent file corruption or zero-byte truncation in the event of an application crash, power outage, or out-of-disk condition.

#### Protocol Sequence:
1. **Directory Anchor:** Allocate the temporary file in the **same filesystem directory** as the target file. (Cross-filesystem moves are not atomic).
2. **Line-Ending Detection:** If target exists, sample the first 4 KB to determine dominant line endings (`\r\n` vs `\n`). Preserve detected format; default new notes to `\n` (LF).
3. **Tempfile Creation:** Create a hidden, uniquely named temporary file: `.<file_stem>.tmp.<uuid>`.
4. **Buffer Write & Flush:** Write content bytes and call `flush()`.
5. **Durable Fsync:** Execute `file.sync_all()` on the underlying OS handle to commit dirty page caches to physical media.
6. **Atomic Rename / Replace:**
   - **Linux:** Standard POSIX `rename()` is atomic and replaces existing targets.
   - **Windows:** Rust's `std::fs::rename()` issues `MoveFileExW` with `MOVEFILE_REPLACE_EXISTING`.
   - **Retry Loop:** Because Windows antivirus indexers and shell monitors can hold brief shared locks, execute up to 3 retries with 25 ms exponential backoff.
7. **Cleanup on Error:** In any failure path before the rename, explicitly remove the tempfile.

```rust
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::thread;
use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum AtomicWriteError {
    #[io]
    Io(#[from] io::Error),
    #[error("Failed to replace target file after retries: {0}")]
    RenameFailed(String),
}

pub fn atomic_write_file(target_path: &Path, content: &str) -> Result<(), AtomicWriteError> {
    let parent = target_path.parent().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "Target path has no parent")
    })?;

    // Detect existing line endings
    let line_ending = detect_line_ending(target_path);
    let normalized_content = normalize_line_endings(content, line_ending);

    // Create unique temp file in same directory
    let temp_name = format!(
        ".{}.tmp.{}",
        target_path.file_name().and_then(|s| s.to_str()).unwrap_or("note"),
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    );
    let temp_path = parent.join(temp_name);

    let write_result = (|| -> io::Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)?;

        file.write_all(normalized_content.as_bytes())?;
        file.flush()?;
        file.sync_all()?; // Force durable disk write
        drop(file);

        // Atomic rename with Windows lock retry backoff
        let mut retries = 3;
        while let Err(e) = fs::rename(&temp_path, target_path) {
            retries -= 1;
            if retries == 0 {
                return Err(e);
            }
            thread::sleep(Duration::from_millis(25));
        }
        Ok(())
    })();

    if write_result.is_err() && temp_path.exists() {
        let _ = fs::remove_file(&temp_path);
    }

    write_result.map_err(AtomicWriteError::Io)
}
```

### 3.4 Path Sanitization & Security Boundary
AirGap strictly isolates all note operations to the vault boundary.

```rust
use std::path::{Component, Path, PathBuf};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PathSafetyError {
    #[error("Path is absolute; only relative paths are permitted")]
    AbsolutePath,
    #[error("Path attempts directory traversal via '..'")]
    DirectoryTraversal,
    #[error("Filename matches Windows reserved device name: {0}")]
    WindowsReservedName(String),
    #[error("Filename contains illegal characters: {0}")]
    IllegalCharacters(String),
    #[error("Path has trailing dots or spaces, which are invalid on Windows")]
    TrailingDotOrSpace,
    #[error("Resolved path escapes vault root (symlink escape)")]
    EscapesVaultRoot,
}

const WINDOWS_RESERVED_NAMES: &[&str] = &[
    "CON", "PRN", "AUX", "NUL",
    "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9",
    "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

const ILLEGAL_CHARS: &[char] = &['<', '>', ':', '"', '/', '\\', '|', '?', '*'];

pub fn sanitize_relative_path(vault_root: &Path, rel_path: &Path) -> Result<PathBuf, PathSafetyError> {
    if rel_path.is_absolute() {
        return Err(PathSafetyError::AbsolutePath);
    }

    for component in rel_path.components() {
        match component {
            Component::ParentDir => return Err(PathSafetyError::DirectoryTraversal),
            Component::RootDir | Component::Prefix(_) => return Err(PathSafetyError::AbsolutePath),
            Component::Normal(os_str) => {
                let s = os_str.to_str().ok_or_else(|| {
                    PathSafetyError::IllegalCharacters("Invalid UTF-8".to_string())
                })?;

                // Check trailing dots or spaces
                if s.ends_with('.') || s.ends_with(' ') {
                    return Err(PathSafetyError::TrailingDotOrSpace);
                }

                // Check illegal characters and control chars
                if s.chars().any(|c| ILLEGAL_CHARS.contains(&c) || c.is_control()) {
                    return Err(PathSafetyError::IllegalCharacters(s.to_string()));
                }

                // Check Windows reserved names (both full name and stem before extension)
                let stem = Path::new(s).file_stem().and_then(|st| st.to_str()).unwrap_or(s);
                let upper_stem = stem.to_ascii_uppercase();
                let upper_full = s.to_ascii_uppercase();

                if WINDOWS_RESERVED_NAMES.contains(&upper_stem.as_str()) 
                    || WINDOWS_RESERVED_NAMES.contains(&upper_full.as_str()) 
                {
                    return Err(PathSafetyError::WindowsReservedName(s.to_string()));
                }
            }
            Component::CurDir => continue,
        }
    }

    let joined = vault_root.join(rel_path);

    // Symlink escape check for existing paths
    if joined.exists() {
        let canonical = joined.canonicalize().map_err(|_| PathSafetyError::EscapesVaultRoot)?;
        let canonical_root = vault_root.canonicalize().map_err(|_| PathSafetyError::EscapesVaultRoot)?;
        if !canonical.starts_with(canonical_root) {
            return Err(PathSafetyError::EscapesVaultRoot);
        }
    }

    Ok(joined)
}
```

### 3.5 Deletion & Recycle Subsystem
Destructive operations require explicit user confirmation and route to the operating system's native trash service with an automated fallback.

```rust
use std::fs;
use std::path::{Path, PathBuf};
use chrono::Local;

#[derive(Debug)]
pub enum DeleteResult {
    MovedToOsTrash,
    MovedToVaultTrash(PathBuf),
}

pub fn safe_delete_item(vault_root: &Path, target_path: &Path) -> Result<DeleteResult, String> {
    // Attempt 1: OS Recycle Bin via `trash` crate
    if trash::delete(target_path).is_ok() {
        return Ok(DeleteResult::MovedToOsTrash);
    }

    // Attempt 2: Vault .trash/ fallback directory
    let trash_dir = vault_root.join(".trash");
    fs::create_dir_all(&trash_dir).map_err(|e| format!("Failed to create .trash directory: {}", e))?;

    let file_name = target_path.file_name().and_then(|s| s.to_str()).unwrap_or("deleted");
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let dest_name = format!("{}_{}", timestamp, file_name);
    let dest_path = trash_dir.join(dest_name);

    fs::rename(target_path, &dest_path)
        .map_err(|e| format!("Failed to move file to .trash: {}", e))?;

    Ok(DeleteResult::MovedToVaultTrash(dest_path))
}
```

### 3.6 Filesystem Watcher & Self-Write Suppression
External updates from Git, CLI, or other editors are detected via `notify-debouncer-mini`. To prevent infinite loop ping-pong when AirGap saves a file, internal writes are tracked in a `WriteJournal`:

```rust
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Clone, Default)]
pub struct WriteJournal {
    entries: Arc<Mutex<HashMap<PathBuf, Instant>>>,
}

impl WriteJournal {
    pub fn mark_internal_write(&self, path: &Path) {
        let mut map = self.entries.lock().unwrap();
        map.insert(path.to_path_buf(), Instant::now());
    }

    pub fn is_suppressed(&self, path: &Path) -> bool {
        let mut map = self.entries.lock().unwrap();
        // Prune older than 2 seconds
        let now = Instant::now();
        map.retain(|_, time| now.duration_since(*time) < Duration::from_secs(2));

        if let Some(time) = map.remove(path) {
            now.duration_since(time) < Duration::from_millis(1500)
        } else {
            false
        }
    }
}
```

### 3.7 Background Search Engines
1. **Quick Open (Ctrl+P):**
   - Utilizes `fuzzy-matcher` (`SkimMatcherV2`) over cached in-memory vault file paths.
   - Evaluates scores based on prefix bonus, boundary matches, and path depth.
   - Returns top 25 matches with match highlight ranges in < 5 ms for 10,000 notes.

2. **Global Full-Text Search (Ctrl+Shift+F):**
   - Uses `rayon` to parallelize scans across all `.md` files in the vault.
   - Streams matches in chunks over a `crossbeam_channel::Sender<SearchResultChunk>`.
   - Supports plain text, case sensitivity toggle, whole word, and regex search.

```rust
use std::ops::Range;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct SearchHit {
    pub path: PathBuf,
    pub line_number: usize,
    pub line_text: String,
    pub match_range: Range<usize>,
}
```

### 3.8 Safe Import & Export Subsystems
- **Zip-Slip Guard:** Every archive entry is inspected: `entry.enclosed_name()` ensures no path escapes outside the extraction root.
- **Zip-Bomb Guard:**
  - Max entry count cap: 10,000 files.
  - Max single file uncompressed size: 50 MB.
  - Max total uncompressed volume: 500 MB.
- **Conflict Policies:** `Skip`, `Overwrite`, and `KeepBoth` (appends ` (1)` suffix).
- **HTML Export:** Generates standalone single-file HTML documents with embedded modern CSS styling and inline vector `<svg>` Mermaid diagrams.

---

## 4. `crates/render`: Markdown, Syntax Highlighting & Mermaid Pipeline

### 4.1 Crate Manifest (`crates/render/Cargo.toml`)
```toml
[package]
name = "airgap-render"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
# Markdown & Highlighting
pulldown-cmark = { version = "0.10", default-features = false, features = ["simd"] }
syntect = { version = "5.2", default-features = false, features = ["default-syntaxes", "default-themes", "fancy-regex"] }

# SVG & Software Rasterization
resvg = "0.43"
usvg = "0.43"
tiny-skia = "0.11"
image = { version = "0.25", default-features = false, features = ["png", "jpeg", "webp"] }

# Core integration & caching
airgap-core = { path = "../core" }
lru = "0.12"
thiserror = "1.0"
tracing = "0.1"

[dev-dependencies]
insta = "1.38"
```

### 4.2 Intermediate Document Model (`DocModel`)
To decouple markdown parsing from immediate-mode GUI rendering and enable source-line scroll sync, `pulldown-cmark` events are converted into a structured `DocModel`:

```rust
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct DocModel {
    pub blocks: Vec<DocBlock>,
    pub source_line_map: Vec<SourceLineMapping>,
}

#[derive(Debug, Clone)]
pub struct SourceLineMapping {
    pub source_line: usize,
    pub block_index: usize,
}

#[derive(Debug, Clone)]
pub enum DocBlock {
    Heading { level: u8, text: Vec<InlineSpan>, line: usize },
    Paragraph { spans: Vec<InlineSpan>, line: usize },
    BlockQuote { blocks: Vec<DocBlock>, line: usize },
    CodeBlock { lang: Option<String>, code: String, highlighted: Vec<StyledLine>, line: usize },
    MermaidBlock { code: String, line: usize },
    Table { header: Vec<Vec<InlineSpan>>, rows: Vec<Vec<Vec<InlineSpan>>>, line: usize },
    TaskList { items: Vec<TaskItem>, line: usize },
    Rule { line: usize },
}

#[derive(Debug, Clone)]
pub struct TaskItem {
    pub checked: bool,
    pub spans: Vec<InlineSpan>,
}

#[derive(Debug, Clone)]
pub enum InlineSpan {
    Text(String),
    Bold(String),
    Italic(String),
    Strikethrough(String),
    Code(String),
    Link { label: String, url: String },
    Image { alt: String, src: String, resolved_path: Option<PathBuf> },
}

#[derive(Debug, Clone)]
pub struct StyledLine {
    pub spans: Vec<StyledSpan>,
}

#[derive(Debug, Clone)]
pub struct StyledSpan {
    pub text: String,
    pub color_rgba: [u8; 4],
    pub bold: bool,
    pub italic: bool,
}
```

### 4.3 Syntax Highlighting Architecture
- **Pure-Rust Guarantee:** By using `syntect` with `default-features = false` and enabling `fancy-regex`, the entire C `oniguruma` dependency is eliminated. This ensures effortless cross-compilation on Windows and Linux without C compiler toolchain headaches.
- **Shared Syntax Set:** Pre-compiled syntaxes and themes are loaded once at launch in an `Arc<SyntaxSet>` and `Arc<ThemeSet>`, shared between editor and preview.

### 4.4 Native Mermaid SVG Generation & Rasterization Pipeline

```
┌────────────────────────┐
│ ```mermaid Code Block  │
└───────────┬────────────┘
            │
            ▼
┌────────────────────────┐      Syntax Error / Unsupported
│ Mermaid Parser & Layout├──────────────────────────────────► ┌─────────────────────────┐
│ (Flowchart, Sequence,  │                                     │ Styled Error Fallback   │
│  Class, State, ER, etc)│                                     │ Card with Line Diagnostics
└───────────┬────────────┘                                     └─────────────────────────┘
            │ Valid SVG Markup
            ▼
┌────────────────────────┐
│ Theme Palette Injector │ (Adjusts colors for Dark / Light mode)
└───────────┬────────────┘
            │ Styled SVG
            ▼
┌────────────────────────┐
│  usvg / resvg Pipeline │ (Off-thread rasterization at active HiDPI scale)
└───────────┬────────────┘
            │
            ▼
┌────────────────────────┐
│ tiny_skia::Pixmap RGBA │
└───────────┬────────────┘
            │
            ▼
┌────────────────────────┐
│ LRU Content-Hash Cache ├──────────────────────────────────► ┌─────────────────────────┐
│ (hash, theme, scale)   │                                     │ Upload to egui Texture  │
└────────────────────────┘                                     └─────────────────────────┘
```

#### Diagram Cache Key:
```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DiagramCacheKey {
    pub content_hash: u64,
    pub is_dark: bool,
    pub scale_bits: u32,
}

pub struct RenderedDiagram {
    pub width: u32,
    pub height: u32,
    pub rgba_pixels: Vec<u8>,
}
```

#### Graceful Degradation:
If a Mermaid diagram fails parsing or uses unsupported syntax, the app **never crashes**. It renders a formatted error container displaying the line number, error description, and the formatted source code.

---

## 5. `crates/app`: GUI Application, Concurrency & State Machine

### 5.1 Crate Manifest (`crates/app/Cargo.toml`)
```toml
[package]
name = "airgap"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
# Workspace crates
airgap-core = { path = "../core" }
airgap-render = { path = "../render" }

# GUI & Windowing
eframe = { version = "0.27", default-features = false, features = ["default_fonts", "wgpu", "wayland", "x11"] }
egui = "0.27"

# Concurrency & System Integration
crossbeam-channel = "0.5"
open = "5.1"
arboard = "3.3"
rfd = "0.14"

# Logging & Error
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

### 5.2 Concurrency Architecture & Message Passing
All heavy operations run on background threads. Communication uses two lock-free `crossbeam-channel` queues:

```
┌─────────────────────────────────────────────────────────────┐
│                       egui UI Thread                        │
│                                                             │
│   eframe::App::update(&mut self, ctx: &egui::Context, ...)   │
│     1. Drain worker_rx.try_iter()                           │
│     2. Check autosave debounce timer                        │
│     3. Render layout (Sidebar, Editor, Preview, Modals)     │
└───────────────▲─────────────────────────────┬───────────────┘
                │                             │
    AppEvent    │                             │ WorkerCommand
(ctx.request_   │                             │
  repaint())    │                             │
                │                             ▼
┌───────────────┴─────────────────────────────────────────────┐
│                  Background Worker Threads                  │
│                                                             │
│  - File Watcher Worker (notify debouncer)                   │
│  - Full-Text Search Worker (rayon parallel stream)          │
│  - Mermaid Rasterizer Worker (resvg / tiny-skia)            │
│  - Safe Import / Export Worker (Zip / Obsidian / HTML)      │
│  - Autosave Worker (disk I/O)                               │
└─────────────────────────────────────────────────────────────┘
```

#### Message Types:
```rust
use std::path::PathBuf;
use airgap_core::search::SearchHit;
use airgap_render::mermaid::{DiagramCacheKey, RenderedDiagram};

pub enum WorkerCommand {
    StartSearch { query: String, is_regex: bool, case_sensitive: bool },
    CancelSearch,
    RenderDiagram { key: DiagramCacheKey, source: String, theme_dark: bool, scale: f32 },
    ExecuteImport { source_zip: PathBuf, conflict_policy: airgap_core::import::ConflictPolicy },
}

pub enum AppEvent {
    SearchHits(Vec<SearchHit>),
    SearchFinished,
    DiagramRendered { key: DiagramCacheKey, result: Result<RenderedDiagram, String> },
    ExternalFileChanged(PathBuf),
    ImportProgress { processed: usize, total: usize },
    ImportFinished { success_count: usize, error_count: usize },
}
```

When a worker posts an `AppEvent`, it immediately invokes `ctx.request_repaint()`, ensuring zero-latency updates without continuous polling.

### 5.3 Conflict Resolution State Machine
External filesystem modifications (e.g. `git checkout` or an external editor) occurring while unsaved in-memory edits exist trigger the conflict resolution protocol:

```
                      ┌──────────────┐
                      │    Clean     │
                      └──────┬───────┘
                             │
                  User types │ Autosave (~1s) /
                  keystroke  │ Ctrl+S
                             ▼
                      ┌──────────────┐
                      │    Dirty     │◄───────────────────┐
                      └──────┬───────┘                    │
                             │                            │
             External file   │                            │
             changed on disk │                            │
                             ▼                            │
                 ┌───────────────────────┐                │
                 │   Conflict Prompt     │                │
                 │  (Autosave Suspended) │                │
                 └───────────┬───────────┘                │
                             │                            │
              User Resolution Decision                    │
                             │                            │
         ┌───────────────────┼───────────────────┐        │
         ▼                   ▼                   ▼        │
   [ Keep Mine ]      [ Take Theirs ]      [ View Diff ] ─┘
   (Atomic overwrite  (Reload disk file,   (Side-by-side
    disk with buffer)  discard edits)       visual diff)
         │                   │
         └─────────► ┌───────┴──────┐
                     │    Clean     │
                     └──────────────┘
```

#### Conflict State Data:
```rust
#[derive(Debug, Clone)]
pub enum DocumentState {
    Clean {
        disk_mtime: std::time::SystemTime,
        disk_hash: [u8; 32],
    },
    Dirty {
        base_mtime: std::time::SystemTime,
        base_hash: [u8; 32],
        last_keystroke: std::time::Instant,
    },
    Conflict {
        base_mtime: std::time::SystemTime,
        base_hash: [u8; 32],
        disk_content: String,
        disk_mtime: std::time::SystemTime,
    },
}
```
*Rule:* While in `DocumentState::Conflict`, **autosave is suspended** to guarantee no automatic overwrite can destroy user or external edits.

### 5.4 Editor & Layouter Architecture
- **Text Widget:** `egui::TextEdit::multiline` coupled with a custom `syntax_layouter`.
- **Syntax Highlighting:** Tokenizes the visible text buffer using `syntect` into colored `egui::text::LayoutJob` runs.
- **Line Numbers:** Dedicated gutter widget rendered to the left, calculated from editor scroll offset and font row heights.
- **Conveniences:**
  - Enter continues lists (`- `, `* `, `1. `, `- [ ] `).
  - Tab / Shift+Tab indents and unindents selections.
  - Auto-pairing brackets and backticks.
  - Smart URL paste: Pasting a URL with text selected transforms into `[selected_text](url)`.
  - Image paste: Pasting a bitmap image from clipboard auto-saves to `assets/` and inserts `![image](assets/...)`.

### 5.5 Live Preview & Scroll Synchronization
- Modes: `EditorOnly`, `Split` (default), `PreviewOnly` (toggled via `Ctrl+\`).
- Scroll Synchronization:
  - The editor computes cursor row progress: `ratio = current_line / total_lines`.
  - The preview uses `DocModel.source_line_map` to locate the corresponding `DocBlock` and scrolls the preview container to that target block smoothly.

---

## 6. Privacy & Dependency Audit (Zero-Network Enforcement)

### 6.1 Transitive Dependency Audit
AirGap enforces a complete, verified ban on network crates.

| Crate Selected | Purpose | Network Sockets? | Transitive Audit Status |
|----------------|---------|------------------|-------------------------|
| `eframe` / `egui` | Desktop GUI & windowing | NO | Pure graphics (winit + wgpu/glow). No web/networking features enabled. |
| `pulldown-cmark` | Markdown parsing | NO | Zero network dependencies. |
| `syntect` | Syntax highlighting | NO | Configured with `fancy-regex`. Pure Rust, no external C or network calls. |
| `resvg` / `tiny-skia` | SVG rasterization | NO | Software rendering. Zero network capability. |
| `notify` | Filesystem monitoring | NO | OS file kernel hooks (inotify, ReadDirectoryChangesW). |
| `trash` | OS recycle bin | NO | OS shell API invocation. |
| `crossbeam-channel` | Thread communication | NO | In-memory atomic ring-buffers. |
| `rayon` | Parallel search | NO | In-memory work-stealing threadpool. |
| `zip` | Archive handling | NO | Local file decompression. Default features stripped. |
| `open` | Open links in OS browser | NO | Invokes OS shell (`ShellExecuteW` / `xdg-open`). Contains zero network sockets. |

### 6.2 `deny.toml` Network Ban Configuration
To mathematically enforce this policy during CI:

```toml
[bans]
multiple-versions = "warn"
deny = [
    { name = "reqwest" },
    { name = "curl" },
    { name = "hyper" },
    { name = "ureq" },
    { name = "surf" },
    { name = "tungstenite" },
    { name = "tokio", features = ["net"] },
    { name = "native-tls" },
    { name = "openssl" },
    { name = "sentry" },
]

[licenses]
unlicensed = "deny"
allow = [
    "MIT",
    "Apache-2.0",
    "BSD-3-Clause",
    "ISC",
    "CC0-1.0",
    "OFL-1.1",
]
```

---

## 7. Verification Strategy & Benchmarks

### 7.1 Automated Test Suite Structure
- `crates/core/tests/`:
  - `atomic_write_tests.rs`: Tests power-loss simulation and ensures zero-byte files are impossible.
  - `path_safety_tests.rs`: Tests traversal attacks (`../../`), symlink escapes, and Windows reserved names (`CON`, `PRN`, `AUX`, `NUL`, `COM1-9`, `aux.md`).
  - `zip_slip_tests.rs`: Tests malicious zip archives with path traversal payloads.
  - `zip_bomb_tests.rs`: Tests rejection of oversized archives and high compression ratios.
- `crates/render/tests/`:
  - `mermaid_tests.rs`: Tests rendering for flowchart, sequence, class, state, ER, pie, and gantt charts.
  - `fallback_tests.rs`: Tests that malformed Mermaid blocks cleanly generate diagnostic fallback boxes without crashing.
  - `golden_tests.rs`: `insta` snapshot tests for AST outputs.

### 7.2 Performance Budgets & Verification Targets
- **Cold Boot Time:** < 1.0 s on standard SSD.
- **Idle Memory Footprint:** < 150 MB RAM in release mode.
- **Large Note Scalability:** 1 MB Markdown note (~20,000 lines) opens in < 400 ms with smooth 60 FPS scrolling and editing.
- **Mermaid Render Latency:** Off-thread render completes in < 50 ms for typical diagrams; zero UI thread stutter.
