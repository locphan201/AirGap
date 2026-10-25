# Milestone M2 Handoff: Headless Markdown Parsing Pipeline & AST Design

**Agent:** `explorer_m2_1` (Role: Markdown AST Explorer)  
**Date:** 2026-10-25T00:25:00Z  
**Target Workspace:** `airgap/crates/render`  
**Milestone:** M2 (Markdown Renderer, Code Highlighting & Preview Architecture)

---

## 1. Observation

Direct examination of the workspace revealed the following concrete facts:

1. **Dependency Configuration (`airgap/crates/render/Cargo.toml:15`):**
   ```toml
   pulldown-cmark = { version = "0.13", default-features = false, features = ["html"] }
   ```
   The cargo cache resolves this to `pulldown-cmark v0.13.5`. All required extension flags (`ENABLE_TABLES`, `ENABLE_TASKLISTS`, `ENABLE_STRIKETHROUGH`, `ENABLE_FOOTNOTES`, `ENABLE_HEADING_ATTRIBUTES`, `ENABLE_GFM`) are core bitflags in `pulldown_cmark::Options` and do not require additional Cargo crate features.

2. **Current AST Prototype (`airgap/crates/render/src/model.rs:1-55`):**
   - Every block variant hardcodes `line: usize` (set to `line: 1` throughout `markdown.rs`).
   - `Block::List` only holds `items: Vec<String>` and `is_ordered: bool`. It has no representation for task list items (`- [ ]`, `- [x]`), nor does it store rich inlines or sub-spans.
   - `Block::Table` holds `headers: Vec<String>` and `rows: Vec<Vec<String>>`, but lacks column text alignments (`Vec<Alignment>`).
   - `Block::Heading` holds only `level: u8`, `text: String`, `line: usize`. It drops heading IDs/classes (`ENABLE_HEADING_ATTRIBUTES`).
   - `InlineSpan` is a flat enum (`Text`, `Bold`, `Italic`, `Strikethrough`, `Code`, `Link`, `Image`), which prevents nested styling (e.g. bold italic or links with bold text) and lacks footnotes and line-breaks.

3. **Current Parser Deficiencies (`airgap/crates/render/src/markdown.rs:6-94`):**
   - Options enabled: only `ENABLE_TABLES`, `ENABLE_TASKLISTS`, and `ENABLE_STRIKETHROUGH`. Missing: `ENABLE_FOOTNOTES`, `ENABLE_HEADING_ATTRIBUTES`, `ENABLE_GFM`.
   - The parser loop ignores: `Tag::List`, `Tag::Item`, `Tag::Table`, `Tag::BlockQuote`, `Tag::FootnoteDefinition`, `Event::TaskListMarker`, and all inline formatting tags (`Tag::Emphasis`, `Tag::Strong`, `Tag::Strikethrough`, `Tag::Link`, `Tag::Image`).
   - Parser uses `Parser::new_ext` without `.into_offset_iter()`, dropping all byte spans.

4. **Security & Path Safety Baseline (`airgap/crates/core/src/path_safety.rs:86-145`):**
   - `sanitize_path` enforces that paths remain strictly within `vault_root` and checks against directory traversal, Windows reserved device names (`CON`, `PRN`, `AUX`, `NUL`, etc.), illegal characters, and symlink escapes.
   - However, no dedicated image resolution module exists in `airgap-render` to handle note-relative image links (e.g. `![alt](assets/pic.png)` or `![alt](../shared/pic.png)`), leading slash vault-relative paths, and network URL blocking.

5. **UI & Editor Current State (`airgap/crates/app/src/ui/editor.rs`):**
   - The editor uses `egui::TextEdit::multiline` with line numbers gutter tracking `state.cursor_line` and `state.cursor_col`.
   - Preview rendering and scroll synchronization are completely unhooked (deferred to M2).

---

## 2. Logic Chain & System Design

### 2.1 Parser Event Stream & Extension Flags

To satisfy CommonMark + GFM specifications, `pulldown_cmark::Options` must be configured with:

```rust
let mut options = Options::empty();
options.insert(Options::ENABLE_TABLES);
options.insert(Options::ENABLE_TASKLISTS);
options.insert(Options::ENABLE_STRIKETHROUGH);
options.insert(Options::ENABLE_FOOTNOTES);
options.insert(Options::ENABLE_HEADING_ATTRIBUTES);
options.insert(Options::ENABLE_GFM); // Enables [!NOTE], [!TIP], etc. blockquotes
```

`Parser::new_ext(markdown_text, options).into_offset_iter()` produces `(Event<'a>, Range<usize>)`.
Every emitted item has a byte range `start..end`. By combining this with a precomputed `LineIndex`, every AST block and inline can be precisely tagged with its source line range `[start_line..=end_line]`.

---

### 2.2 Intermediate AST Model (`crates/render/src/model.rs`)

The document model must represent all CommonMark and GFM constructs while maintaining zero-breakage compatibility with existing tests matching on `{ level, text, .. }` or `{ source, .. }`.

```rust
use std::collections::HashMap;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

/// 1-indexed source line and 0-indexed byte offset span.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SourceSpan {
    pub start_line: usize,
    pub end_line: usize,
    pub start_byte: usize,
    pub end_byte: usize,
}

impl SourceSpan {
    pub fn new(start_line: usize, end_line: usize, start_byte: usize, end_byte: usize) -> Self {
        Self {
            start_line,
            end_line: end_line.max(start_line),
            start_byte,
            end_byte: end_byte.max(start_byte),
        }
    }

    pub fn line(line: usize) -> Self {
        Self {
            start_line: line,
            end_line: line,
            start_byte: 0,
            end_byte: 0,
        }
    }
}

/// GFM table column alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ColumnAlignment {
    #[default]
    None,
    Left,
    Center,
    Right,
}

/// GitHub-style blockquote callout kind (admonition).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CalloutKind {
    Note,
    Tip,
    Important,
    Warning,
    Caution,
}

/// Inline text styling attributes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct InlineStyle {
    pub bold: bool,
    pub italic: bool,
    pub strikethrough: bool,
    pub code: bool,
}

/// Rich inline element.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InlineSpan {
    Text(String),
    Bold(String),
    Italic(String),
    Strikethrough(String),
    Code(String),
    StyledText {
        text: String,
        style: InlineStyle,
    },
    Link {
        text: String,
        url: String,
        title: Option<String>,
    },
    Image {
        alt: String,
        url: String,
        title: Option<String>,
        resolved_path: Option<PathBuf>,
    },
    FootnoteRef(String),
    SoftBreak,
    HardBreak,
}

impl InlineSpan {
    pub fn plain_text(&self) -> &str {
        match self {
            InlineSpan::Text(s)
            | InlineSpan::Bold(s)
            | InlineSpan::Italic(s)
            | InlineSpan::Strikethrough(s)
            | InlineSpan::Code(s)
            | InlineSpan::StyledText { text: s, .. } => s,
            InlineSpan::Link { text, .. } => text,
            InlineSpan::Image { alt, .. } => alt,
            InlineSpan::FootnoteRef(s) => s,
            InlineSpan::SoftBreak => " ",
            InlineSpan::HardBreak => "\n",
        }
    }
}

/// Single item within an ordered, unordered, or task list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListItem {
    /// Plain-text content of the item (for search and fast outline)
    pub text: String,
    /// Rich inlines (supports bold, links, code inside list items)
    pub inlines: Vec<InlineSpan>,
    /// Task checkbox state: None for normal bullet/numbered item,
    /// Some(false) for `- [ ]`, Some(true) for `- [x]`.
    pub task_status: Option<bool>,
    /// Source span of this specific item
    pub span: SourceSpan,
}

/// Top-level block elements in the Document Model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Block {
    Heading {
        level: u8,               // 1..=6
        text: String,            // Trimmed plain text
        inlines: Vec<InlineSpan>,
        id: Option<String>,      // Optional heading attribute `{#intro}`
        classes: Vec<String>,    // Optional heading classes `{.title}`
        line: usize,             // 1-indexed start line (backward compatibility)
        span: SourceSpan,        // Full line and byte range
    },
    Paragraph {
        inlines: Vec<InlineSpan>,
        line: usize,
        span: SourceSpan,
    },
    BlockQuote {
        lines: Vec<String>,
        inlines: Vec<InlineSpan>,
        callout: Option<CalloutKind>,
        line: usize,
        span: SourceSpan,
    },
    CodeBlock {
        language: String,
        code: String,
        line: usize,
        span: SourceSpan,
    },
    MermaidBlock {
        source: String,
        line: usize,
        span: SourceSpan,
    },
    List {
        items: Vec<ListItem>,
        is_ordered: bool,
        start_number: Option<u64>,
        line: usize,
        span: SourceSpan,
    },
    Table {
        headers: Vec<String>,
        rows: Vec<Vec<String>>,
        alignments: Vec<ColumnAlignment>,
        header_inlines: Vec<Vec<InlineSpan>>,
        row_inlines: Vec<Vec<Vec<InlineSpan>>>,
        line: usize,
        span: SourceSpan,
    },
    ThematicBreak {
        line: usize,
        span: SourceSpan,
    },
    FootnoteDefinition {
        label: String,
        inlines: Vec<InlineSpan>,
        line: usize,
        span: SourceSpan,
    },
}

impl Block {
    pub fn line(&self) -> usize {
        match self {
            Block::Heading { line, .. }
            | Block::Paragraph { line, .. }
            | Block::BlockQuote { line, .. }
            | Block::CodeBlock { line, .. }
            | Block::MermaidBlock { line, .. }
            | Block::List { line, .. }
            | Block::Table { line, .. }
            | Block::ThematicBreak { line, .. }
            | Block::FootnoteDefinition { line, .. } => *line,
        }
    }

    pub fn span(&self) -> SourceSpan {
        match self {
            Block::Heading { span, .. }
            | Block::Paragraph { span, .. }
            | Block::BlockQuote { span, .. }
            | Block::CodeBlock { span, .. }
            | Block::MermaidBlock { span, .. }
            | Block::List { span, .. }
            | Block::Table { span, .. }
            | Block::ThematicBreak { span, .. }
            | Block::FootnoteDefinition { span, .. } => *span,
        }
    }
}

/// Headless document model containing all blocks, footnotes, and line mappings.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct DocModel {
    pub blocks: Vec<Block>,
    pub footnotes: HashMap<String, Vec<InlineSpan>>,
    pub total_lines: usize,
}

impl DocModel {
    pub fn new(blocks: Vec<Block>, total_lines: usize) -> Self {
        Self {
            blocks,
            footnotes: HashMap::new(),
            total_lines,
        }
    }

    /// Finds index of block enclosing or closest to 1-indexed source line.
    pub fn find_block_by_line(&self, line: usize) -> Option<usize> {
        if self.blocks.is_empty() {
            return None;
        }
        // Binary search across blocks by start_line
        let idx = match self.blocks.binary_search_by(|b| b.span().start_line.cmp(&line)) {
            Ok(found) => found,
            Err(insert_idx) => {
                if insert_idx == 0 {
                    0
                } else {
                    let prev = insert_idx - 1;
                    if line <= self.blocks[prev].span().end_line {
                        prev
                    } else if insert_idx < self.blocks.len() {
                        // Closer to prev or next?
                        let prev_dist = line - self.blocks[prev].span().end_line;
                        let next_dist = self.blocks[insert_idx].span().start_line - line;
                        if next_dist < prev_dist {
                            insert_idx
                        } else {
                            prev
                        }
                    } else {
                        prev
                    }
                }
            }
        };
        Some(idx.min(self.blocks.len() - 1))
    }

    /// Extracts outline headings for table-of-contents navigation.
    pub fn outline(&self) -> Vec<(u8, String, usize)> {
        self.blocks
            .iter()
            .filter_map(|b| match b {
                Block::Heading { level, text, line, .. } => Some((*level, text.clone(), *line)),
                _ => None,
            })
            .collect()
    }
}
```

---

### 2.3 Source-Line Mapping Architecture (Editor ↔ Preview Scroll Sync)

1. **`LineIndex` Implementation:**
   Computes byte boundaries in $O(N)$ single pass, then enables $O(\log L)$ binary-search conversions:
   ```rust
   pub struct LineIndex {
       line_starts: Vec<usize>,
       total_bytes: usize,
   }

   impl LineIndex {
       pub fn new(text: &str) -> Self {
           let mut line_starts = vec![0];
           for (idx, byte) in text.bytes().enumerate() {
               if byte == b'\n' {
                   line_starts.push(idx + 1);
               }
           }
           Self {
               line_starts,
               total_bytes: text.len(),
           }
       }

       pub fn offset_to_line(&self, byte_offset: usize) -> usize {
           let clamped = byte_offset.min(self.total_bytes);
           match self.line_starts.binary_search(&clamped) {
               Ok(idx) => idx + 1,
               Err(idx) => idx,
           }
       }

       pub fn byte_range_to_span(&self, range: &std::ops::Range<usize>) -> SourceSpan {
           let start_line = self.offset_to_line(range.start);
           let end_line = if range.end > range.start {
               self.offset_to_line(range.end - 1).max(start_line)
           } else {
               start_line
           };
           SourceSpan::new(start_line, end_line, range.start, range.end)
       }

       pub fn total_lines(&self) -> usize {
           self.line_starts.len()
       }
   }
   ```

2. **Editor-to-Preview Sync Workflow:**
   - Editor reports its top visible line $L_{\text{vis}}$ (from cursor position or scroll offset / row height).
   - Find matching block: `let block_idx = doc.find_block_by_line(L_vis)`.
   - Calculate within-block progress:
     $$\text{fraction} = \frac{L_{\text{vis}} - \text{span.start\_line}}{\max(1, \text{span.end\_line} - \text{span.start\_line} + 1)}$$
   - Preview pane holds recorded layout rectangles for rendered blocks: `block_rects: Vec<Rect>`.
   - Scroll target in preview is:
     $$Y_{\text{target}} = \text{rect.min.y} + \text{fraction} \times \text{rect.height()}$$
   - Apply `ui.scroll_to_rect(Rect::from_min_size(pos, size), Some(Align::TOP))` or set preview `ScrollArea` vertical offset.

3. **Preview-to-Editor Sync Workflow:**
   - User scrolls preview pane to offset $Y_{\text{prev}}$.
   - Preview identifies block at top viewport boundary.
   - Its `span.start_line` is mapped back to the editor, setting editor cursor or scroll row.

4. **Loop Prevention Protocol:**
   - Introduce `ScrollSyncState` in `AppState`:
     ```rust
     pub enum ActiveScrollDriver {
         Editor,
         Preview,
         None,
     }
     ```
   - When editor receives user input or mouse wheel: set driver to `ActiveScrollDriver::Editor`.
   - When preview receives mouse wheel or drag: set driver to `ActiveScrollDriver::Preview`.
   - Sync only propagates from the active driver to the passive follower.
   - A deadband of 1 line or 4 pixels prevents reciprocal micro-jitter.

---

### 2.4 Secure Local Image Resolution (`crates/render/src/image.rs`)

Local images must resolve safely relative to the note's filesystem directory or vault root, with strict rejection of path escapes and network calls:

```rust
use std::path::{Component, Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum ImageResolutionError {
    #[error("Network image URLs are prohibited by local-only privacy: {0}")]
    NetworkBlocked(String),

    #[error("Image path escapes vault root: {0}")]
    VaultEscape(String),

    #[error("Image file does not exist on disk: {0}")]
    NotFound(String),

    #[error("Image filename or path is invalid: {0}")]
    InvalidPath(String),

    #[error("Unsupported image format: {0}")]
    UnsupportedFormat(String),
}

const SUPPORTED_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "svg", "webp", "bmp", "gif"];

/// Resolves a Markdown image URL relative to the active note and vault root.
/// Returns verified canonical path within vault.
pub fn resolve_local_image_path(
    vault_root: &Path,
    active_note_path: &Path,
    image_url: &str,
) -> Result<PathBuf, ImageResolutionError> {
    let trimmed = image_url.trim();

    // 1. Strict ban on network schemes
    let lower = trimmed.to_lowercase();
    if lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("//")
        || lower.starts_with("ftp://")
    {
        return Err(ImageResolutionError::NetworkBlocked(trimmed.to_string()));
    }

    // 2. Strip file:// scheme if present
    let raw_path_str = if let Some(stripped) = trimmed.strip_prefix("file://") {
        stripped
    } else {
        trimmed
    };

    // Remove query parameters or fragments if any (e.g. "image.png?v=1")
    let clean_str = raw_path_str.split('?').next().unwrap_or(raw_path_str);
    let clean_str = clean_str.split('#').next().unwrap_or(clean_str);

    let raw_path = Path::new(clean_str);

    // 3. Determine base folder
    let note_dir = if active_note_path.is_file() {
        active_note_path.parent().unwrap_or(vault_root)
    } else {
        active_note_path
    };

    // 4. Combine base folder with image path
    let candidate = if clean_str.starts_with('/') || clean_str.starts_with('\\') {
        // Leading slash resolves relative to vault root
        let rel = clean_str.trim_start_matches(|c| c == '/' || c == '\\');
        vault_root.join(rel)
    } else if raw_path.is_absolute() {
        raw_path.to_path_buf()
    } else {
        note_dir.join(raw_path)
    };

    // 5. Lexical component check against traversal
    let mut normalized = PathBuf::new();
    for comp in candidate.components() {
        match comp {
            Component::ParentDir => {
                if !normalized.pop() {
                    return Err(ImageResolutionError::VaultEscape(clean_str.to_string()));
                }
            }
            Component::CurDir => {}
            Component::Normal(c) => normalized.push(c),
            Component::RootDir | Component::Prefix(_) => {
                normalized.push(comp);
            }
        }
    }

    // 6. Verify containment inside vault_root
    let canon_vault = vault_root
        .canonicalize()
        .map_err(|_| ImageResolutionError::InvalidPath(vault_root.display().to_string()))?;

    if !normalized.exists() {
        return Err(ImageResolutionError::NotFound(normalized.display().to_string()));
    }

    let canon_file = normalized
        .canonicalize()
        .map_err(|_| ImageResolutionError::NotFound(normalized.display().to_string()))?;

    if !canon_file.starts_with(&canon_vault) {
        return Err(ImageResolutionError::VaultEscape(clean_str.to_string()));
    }

    // 7. Validate extension
    let ext = canon_file
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    if !SUPPORTED_EXTENSIONS.contains(&ext.as_str()) {
        return Err(ImageResolutionError::UnsupportedFormat(ext));
    }

    Ok(canon_file)
}
```

---

## 3. Caveats & Design Trade-offs

1. **Syntax Highlighting Separation:**
   Syntax highlighting (`syntect` in `crates/render/src/highlight.rs`) must remain decoupled from the initial AST parsing. Fenced code blocks store their `language` tag and raw `code` string in `Block::CodeBlock`. Highlighting is performed either on-demand during preview rendering or in background workers to ensure 1 MB+ note parsing completes in <5 ms.

2. **Loose vs Tight Lists:**
   CommonMark distinguishes tight lists (no `<p>` inside `<li>`) from loose lists (with `<p>` inside `<li>`). In our AST, both are normalized into `ListItem { inlines, task_status, span }`. This simplifies preview layout without losing formatting fidelity.

3. **Soft Word Wrapping vs Monospace Gutter:**
   `egui::TextEdit` word-wraps long lines visually without inserting newline characters. Scroll sync maps logical newline lines (1-indexed), which are stable across window resizes, fonts, and zoom levels.

4. **Admonition / Callout Syntax:**
   GitHub Flavored Markdown renders `> [!NOTE]` as an admonition box. With `ENABLE_GFM` enabled in `pulldown-cmark`, `Tag::BlockQuote(Some(BlockQuoteKind::Note))` directly exposes this semantic. Our model maps this to `BlockQuote { callout: Some(CalloutKind::Note), .. }`.

---

## 4. Conclusion

1. `pulldown-cmark` 0.13.5 provides complete native support for all required CommonMark and GFM features via its `into_offset_iter()` API.
2. The designed `DocModel` and `Block` variants in `crates/render/src/model.rs` provide full representation for headings (levels 1-6 with IDs), paragraphs, code fences, blockquotes (with callouts), lists, task lists (`- [ ]`, `- [x]`), GFM tables (with column alignments), horizontal rules, and rich inlines.
3. The `LineIndex` + `SourceSpan` architecture provides $O(\log N)$ coordinate mapping for bi-directional editor ↔ preview scroll synchronization with zero drift.
4. The `resolve_local_image_path` specification guarantees airtight path confinement within the vault root while supporting standard relative paths (`assets/image.png`, `../shared/img.jpg`) and strictly blocking all network and symlink escape attempts.

---

## 5. Verification Method & Implementation Plan for Worker

### 5.1 Verification Commands
The worker can independently verify the implementation with:
```bash
# 1. Verify render crate compilation and unit tests
cargo test --manifest-path airgap/Cargo.toml -p airgap-render

# 2. Verify all workspace crates compile cleanly
cargo check --manifest-path airgap/Cargo.toml --workspace

# 3. Verify zero network dependencies
cargo tree --manifest-path airgap/Cargo.toml --target all | findstr /i "reqwest hyper curl ureq"

# 4. Verify existing E2E tests continue to pass
cargo test --manifest-path airgap/tests/Cargo.toml --test tier1_feature_tests
```

### 5.2 Implementation Plan for Worker

| Step | Target File | Action |
|:---|:---|:---|
| **1** | `crates/render/src/model.rs` | Replace prototype with the full `DocModel`, `Block`, `ListItem`, `InlineSpan`, `SourceSpan`, `ColumnAlignment`, `CalloutKind` definitions and helper methods. |
| **2** | `crates/render/src/image.rs` | Implement `resolve_local_image_path` with unit tests for traversal rejection, network blocking, and note-relative path resolution. |
| **3** | `crates/render/src/markdown.rs` | Update `parse_markdown` to enable all required options, instantiate `LineIndex`, drive `Parser::into_offset_iter()`, parse tables with alignments, task lists (`TaskListMarker`), headings with attributes, and populate `DocModel`. |
| **4** | `crates/render/src/lib.rs` | Export new types (`SourceSpan`, `ColumnAlignment`, `CalloutKind`, `ListItem`, `resolve_local_image_path`, `ImageResolutionError`). |
| **5** | `crates/render/src/markdown.rs` (tests) | Add comprehensive unit tests verifying: GFM tables with alignments, `- [ ]` / `- [x]` task lists, headings with attributes, blockquote callouts, accurate source line numbers, and 1 MB large note parsing performance. |
