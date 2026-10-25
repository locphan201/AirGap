# Milestone M2 Handoff: Desktop GUI Preview & Editor Enhancements

**Agent:** `explorer_m2_3` (Role: Preview UI Explorer)  
**Working Directory:** `C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\.agents\teamwork\explorer_m2_3`  
**Target Crates:** `airgap/crates/app`, `airgap/crates/render`  
**Date:** 2026-10-25  

---

## 1. Observation

### 1.1 Existing Codebase & Workspace State
- **Workspace Cargo Manifest (`airgap/Cargo.toml`)**:
  Defines workspace members `crates/core`, `crates/render`, `crates/app`.
  Strict profile settings: `lto = "fat"`, `codegen-units = 1`, `strip = true`, `panic = "abort"`.
- **App Dependencies (`airgap/crates/app/Cargo.toml`, lines 8–15)**:
  ```toml
  [dependencies]
  airgap-core = { path = "../core" }
  airgap-render = { path = "../render" }
  egui = "0.31"
  eframe = { version = "0.31", default-features = false, features = ["default_fonts", "glow"] }
  crossbeam-channel = "0.5"
  rfd = "0.15"
  ```
  Notice: The `open` crate is **missing** from `crates/app/Cargo.toml`.
- **Render Dependencies (`airgap/crates/render/Cargo.toml`, lines 8–18)**:
  `pulldown-cmark = "0.13"`, `syntect = "5"`, `mermaid-rs-renderer = "0.3.1"`, `resvg = "0.45"`, `tiny-skia = "0.11"`.
- **Cargo Dependency Graph Audit (`airgap/Cargo.lock`)**:
  - `arboard = "3.7.0"` is already present in `Cargo.lock` (line 143) as a dependency of `egui_winit`.
  - `image = "0.25.10"` (with `png = "0.18.1"`) is already present in `Cargo.lock` (line 1483) as a dependency of `arboard`.
  - Zero HTTP/network client libraries: `deny.toml` strictly bans `reqwest`, `hyper`, `curl`, `ureq`, `surf`, `isahc`, `attohttpc`, `tungstenite`.
- **Existing App State (`airgap/crates/app/src/state.rs`, lines 15–40)**:
  Holds `vault`, `tree`, `active_note_path`, `editor_content`, `saved_content`, `save_status`, `line_numbers_enabled`, `word_wrap_enabled`, `cursor_line`, `cursor_col`, `word_count`, `char_count`.
  Missing: `view_mode`, `split_ratio`, `find_replace`, `undo_stack`, `scroll_sync` state.
- **Existing Central Panel Layout (`airgap/crates/app/src/app.rs`, lines 101–105)**:
  ```rust
  // Central editor pane
  egui::CentralPanel::default().show(ctx, |ui| {
      render_editor(ui, &mut self.state);
  });
  ```
  Currently renders only the editor without view modes or preview pane.
- **Existing Editor Implementation (`airgap/crates/app/src/ui/editor.rs`, lines 117–135)**:
  The gutter currently generates a single multiline string `gutter_text` rendered as an `egui::Label` alongside `egui::TextEdit::multiline`. When word wrap is enabled, visual rows desync from logical line numbers. Active line highlight is absent.
- **Intermediate AST (`airgap/crates/render/src/model.rs`, lines 13–49)**:
  Every AST block (`Heading`, `Paragraph`, `BlockQuote`, `CodeBlock`, `MermaidBlock`, `List`, `Table`, `ThematicBreak`) has an explicit `line: usize` field designed for source-line mapping.
- **Test Suite Execution**:
  Executed `cargo test --manifest-path airgap/Cargo.toml`:
  - `airgap-app`: 2 passed; 0 failed (`test_app_state_vault_operations_and_autosave`, `test_benchmark_execution`).
  - `adversarial_editor_stress`: 1 passed; 0 failed (`test_1mb_textedit_rapid_mutations_and_frame_times`).
  - `airgap-core`: 15 passed; 0 failed.
  - `adversarial_atomic_io`: 6 passed; 0 failed.
  - `challenger_m1_adversarial`: 17 passed; 0 failed.
  - `airgap-render`: 13 passed; 0 failed.
  - `adversarial_hidpi_stress`: 2 passed; 0 failed.
  Total: 56 tests passed, 0 failures.

---

## 2. Logic Chain

### 2.1 Triple View Modes Architecture
1. **Requirement**: `AGENT.md` section 3.2 and `PROJECT.md` Feature #36 mandate three view modes: `EditorOnly`, `Split` (50/50 resizable split, default), and `PreviewOnly`, toggleable via `Ctrl+\` and toolbar buttons.
2. **State & Transition**:
   - Define enum `ViewMode`:
     ```rust
     #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
     pub enum ViewMode {
         #[default]
         Split,
         EditorOnly,
         PreviewOnly,
     }
     ```
   - Cycle order: `Split -> PreviewOnly -> EditorOnly -> Split`.
   - Store `view_mode: ViewMode` and `split_ratio: f32` (default 0.5, range `[0.15, 0.85]`) in `AppState`.
3. **Resizable Split Implementation in egui**:
   - In `CentralPanel`, measure total available width $W$.
   - When in `Split` mode:
     - Left pane width: $W_{left} = ((W - 6.0) \times \text{split\_ratio}).\text{clamp}(150.0, W - 150.0)$.
     - Draggable separator rect: width 6.0 px, height $H$.
     - Right pane width: $W_{right} = W - 6.0 - W_{left}$.
   - Interaction:
     - Allocate divider with `Sense::drag()`.
     - On drag: adjust `split_ratio += delta_x / (W - 6.0)`.
     - On hover/drag: set cursor to `egui::CursorIcon::ResizeHorizontal` and highlight divider stroke with accent color (`visuals.widgets.active.bg_fill`).
     - On double-click: reset `split_ratio = 0.5`.
   - In `EditorOnly`: editor occupies full $W$.
   - In `PreviewOnly`: preview occupies full $W$.

### 2.2 Approximate Editor-Preview Scroll Synchronization
1. **Core Problem**: Markdown editor text has fixed line heights, whereas rendered preview contains variable-height elements (headings, code blocks, tables, diagrams). Proportional scrolling ($Y_{prev} = Y_{edit} \times H_{prev} / H_{edit}$) produces severe drift.
2. **AST Source Line Anchor Mapping**:
   - `DocModel` blocks carry source `line: usize`.
   - During preview rendering in `crates/app/src/ui/preview.rs`:
     - Render each block within the preview `ScrollArea`.
     - Record `(block.line, block_rect.min.y - scroll_area_origin_y)` into an ephemeral index `Vec<(usize, f32)>`.
3. **Discrete vs Continuous Synchronization**:
   - **Discrete (Cursor Navigation / Line Jump)**:
     When the editor cursor changes line to $L$ (or when user clicks a line):
     Find the first AST block with `block.line >= L`.
     Call `block_response.scroll_to_me(Some(egui::Align::Center))` or target preview scroll offset.
   - **Continuous (Mouse Wheel Scrolling)**:
     Editor top visible line is $L_{top} \approx 1 + \lfloor Y_{editor\_scroll} / H_{line} \rfloor$.
     Find bounding checkpoints $(L_a, Y_a)$ and $(L_b, Y_b)$ in the preview index such that $L_a \le L_{top} \le L_b$.
     Interpolate: $Y_{target} = Y_a + \frac{L_{top} - L_a}{L_b - L_a}(Y_b - Y_a)$.
     Pass `vertical_scroll_offset(Y_target)` to the preview `ScrollArea`.
4. **Feedback Loop Prevention**:
   - Maintain `ScrollSyncDriver`:
     `enum ScrollSyncDriver { None, EditorDriving, PreviewDriving }`.
   - Set driver based on which pane received recent pointer hover or wheel events (`editor_ui.rect_contains_pointer()` vs `preview_ui.rect_contains_pointer()`).
   - Only apply programmatic scroll offset to the *non-driving* pane.

### 2.3 Safe External Hyperlink Handling
1. **Zero-Network Invariant**:
   `AGENT.md` section 6 and `PROJECT.md` Feature #75 ban all network client libraries. The only permissible outward action is opening a user-clicked URL in the default system browser via the OS launcher.
2. **Dependency**:
   Use `open = "5.3"`. It invokes `ShellExecuteW` on Windows, `xdg-open` on Linux, and `open` on macOS. It does not bundle or link any HTTP libraries.
3. **URL Sanitization & Scheme Whitelist**:
   Passing arbitrary URLs to `open::that` without validation is dangerous (could trigger `file://`, local binaries, shell commands, or protocol exploits).
   Strict validation rules:
   - Trim whitespace. Max length: 2048 chars.
   - Allowed schemes: ONLY `http://`, `https://`, and `mailto:`.
   - Block `file://`, `ftp://`, `data:`, `javascript:`, `powershell:`, `cmd:`, or relative filesystem paths.
   - For web URLs, verify valid domain structure.
4. **Non-Blocking Launch**:
   Execute `open::that_detached(&url)` in a spawned worker thread (`std::thread::spawn`) to prevent UI frame stutter if the OS browser launch takes 300–500 ms.
5. **Preview Widget**:
   Render links with underline, hover tooltip (`"↗ Open in browser: <url>"`), hand cursor (`CursorIcon::PointingHand`), and click dispatch to the safe opener.

### 2.4 Editor Interactive Features

#### A. Line Numbers Gutter & Active Line Highlight
- Current code bug: Soft-wrapped lines cause line number height mismatches.
- Fix:
  - Width: `(total_lines.to_string().len().max(3) as f32 * char_width) + 16.0`.
  - Use `output.galley`: For each line in `editor_content`, obtain `pos_from_char_offset(line_start_char_idx)`.
  - Draw line numbers at the exact vertical offset of each logical line.
  - Active line highlight:
    From `output.cursor_range.primary`, find active line vertical extents `[y_min, y_max]`.
    Paint background rect across editor width with subtle fill:
    `Color32::from_rgba_unmultiplied(255, 255, 255, 10)` in Dark theme, `Color32::from_rgba_unmultiplied(0, 0, 0, 10)` in Light theme.
    Render active line number in gutter with `ui.visuals().strong_text_color()`.

#### B. Explicit Undo/Redo Stack
- Problem: Modifying `state.editor_content` outside `egui::TextEdit` clears or desynchronizes egui's internal undoer.
- Fix:
  - Introduce `UndoStack`:
    Holds `Vec<EditorSnapshot>` for undo and redo with capacity 200.
    `EditorSnapshot { text: String, cursor: usize }`.
  - Coalesce rapid typing bursts within 500 ms.
  - Create immediate hard snapshots on discrete actions: Enter, Tab, auto-pair, paste, and find/replace.
  - Intercept `Ctrl+Z` (undo) and `Ctrl+Y` / `Ctrl+Shift+Z` (redo) in `handle_keyboard_shortcuts`.

#### C. In-Note Find & Replace Bar (`Ctrl+F` / `Ctrl+H`)
- State `FindReplaceState`:
  - `is_open: bool`, `show_replace: bool`, `find_query: String`, `replace_query: String`.
  - `case_sensitive: bool`, `whole_word: bool`, `use_regex: bool`, `regex_error: Option<String>`.
  - `matches: Vec<Range<usize>>`, `current_match_index: usize`.
- Shortcuts:
  - `Ctrl+F`: open find bar, focus search input.
  - `Ctrl+H`: open find bar with replace row expanded.
  - `Escape`: close find bar, return focus to editor.
  - `Enter` / `F3`: next match.
  - `Shift+Enter` / `Shift+F3`: previous match.
- Replacement:
  - "Replace Current": replaces `matches[current_match_index]`, records undo snapshot.
  - "Replace All": replaces all matches in reverse index order, records single undo snapshot.
- Match highlighting: Highlight all matches in editor layouter in amber/yellow (`Color32::from_rgba_unmultiplied(234, 179, 8, 80)`), active match in bright orange with border.

#### D. Auto-Pairing of Enclosures
- Enclosures: `()`, `[]`, `{}`, `""`, ````.
- Selection wrap: If text is selected and enclosure key is typed, wrap selection: `format!("{}{}{}", open, selected, close)`.
- Center cursor: If no selection, insert `format!("{}{}", open, close)` and position cursor between them.
- Overtype / Skip-over: If cursor is immediately before a closing character and user types that character, advance cursor past it without inserting duplicate.
- Balanced Backspace: If cursor is between empty pair (e.g. `(|)`), pressing Backspace deletes both characters.

#### E. List Continuation on Enter
- Intercept Enter key:
  - Regex check preceding line prefix:
    - Task list: `^(\s*)-\s\[([ xX])\]\s+(.*)$`
      If text is empty: delete prefix (exit task list).
      If text exists: next line gets `${indent}- [ ] `.
    - Bullet list: `^(\s*)([-*+])\s+(.*)$`
      If text is empty: delete prefix (exit bullet list).
      If text exists: next line gets `${indent}${bullet} `.
    - Numbered list: `^(\s*)(\d+)\.\s+(.*)$`
      If text is empty: delete prefix (exit numbered list).
      If text exists: next line gets `${indent}${N+1}. `.
    - Blockquote: `^(\s*>+\s*)(.*)$`
      If text is empty: delete prefix.
      If text exists: next line gets `${indent}> `.

#### F. Indent / Outdent with Tab / Shift+Tab
- Monospace 2-space indentation:
  - Without selection: `Tab` inserts 2 spaces; `Shift+Tab` removes up to 2 leading spaces before cursor.
  - With multiline selection: `Tab` prepends 2 spaces to every intersecting line; `Shift+Tab` removes up to 2 leading spaces from every intersecting line. Selection range adjusts accordingly.

#### G. Smart URL Paste
- On paste (`Ctrl+V`):
  - Check clipboard string: starts with `http://`, `https://`, or `mailto:`, contains no whitespace or newlines.
  - If editor has active selection `[start, end]`:
    Replace selection with `format!("[{}]({})", selected_text, clipboard_url)`.
    Position cursor after closing `)`.

#### H. Image Paste from Clipboard
- On paste (`Ctrl+V`):
  - Query `arboard::Clipboard::new()?.get_image()`.
  - If image data is present:
    - Ensure active note is selected.
    - Path resolution: note directory + `assets/` (e.g. `Notes/assets/`).
    - Create `assets/` directory if missing.
    - Filename generation: `image_<timestamp>_<uuid/hash>.png`.
    - Encode raw RGBA bytes to PNG file using `image::codecs::png::PngEncoder` or `RgbaImage::save`.
    - Relative markdown snippet: `format!("![image](assets/{})", filename)`.
    - Insert snippet at cursor, push undo snapshot, trigger `state.on_editor_modified()`.

---

## 3. Caveats
1. **Mermaid Rendering Phase Split**: Full off-thread rasterization and LRU texture caching of Mermaid diagrams belongs to Milestone M3. In Milestone M2, `preview.rs` renders Mermaid blocks as styled code blocks with diagram headers or utilizes the preliminary spike rasterizer where available, without blocking the UI thread.
2. **OS Browser Variance**: Opening external links relies on the host OS having a configured default browser and handlers for `http(s)://` and `mailto:`. On headless Linux systems without `xdg-open`, `open::that_detached` gracefully logs an error without crashing.
3. **Clipboard Backend on Wayland/Linux**: `arboard` requires `x11rb` or Wayland clipboard libraries (which are bundled and linked). Testing clipboard image paste in headless CI environments requires mock data or image injection tests.

---

## 4. Conclusion & Recommended Types and Signatures

### 4.1 Dependency Addition
Add `open` to `airgap/crates/app/Cargo.toml`:
```toml
[dependencies]
open = "5.3"
```
Audit confirmation: `open = "5.3"` is MIT licensed and contains zero HTTP client libraries, fully complying with `deny.toml`.

### 4.2 Recommended Structs & Signatures

#### View Mode & Layout (`airgap/crates/app/src/state.rs` & `app.rs`)
```rust
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
```

#### Safe Link Opener (`airgap/crates/app/src/ui/preview.rs` or `url_safety.rs`)
```rust
pub fn sanitize_external_url(raw: &str) -> Result<String, &'static str> {
    let trimmed = raw.trim();
    if trimmed.len() > 2048 {
        return Err("URL exceeds 2048 characters");
    }
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") {
        let host_start = if lower.starts_with("https://") { 8 } else { 7 };
        if trimmed[host_start..].trim().is_empty() || trimmed[host_start..].starts_with('/') {
            return Err("Invalid or empty web URL host");
        }
        Ok(trimmed.to_string())
    } else if lower.starts_with("mailto:") {
        if !trimmed[7..].contains('@') {
            return Err("Invalid mailto address");
        }
        Ok(trimmed.to_string())
    } else {
        Err("Blocked unsafe URL scheme")
    }
}

pub fn open_external_url_safely(raw_url: &str) -> Result<(), &'static str> {
    let safe_url = sanitize_external_url(raw_url)?;
    std::thread::spawn(move || {
        let _ = open::that_detached(&safe_url);
    });
    Ok(())
}
```

#### Undo / Redo Manager (`airgap/crates/app/src/state.rs`)
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextSnapshot {
    pub text: String,
    pub cursor_char_index: usize,
}

pub struct UndoManager {
    undo_stack: Vec<TextSnapshot>,
    redo_stack: Vec<TextSnapshot>,
    last_edit: std::time::Instant,
    max_history: usize,
}

impl UndoManager {
    pub fn new(max_history: usize) -> Self {
        Self {
            undo_stack: Vec::with_capacity(max_history),
            redo_stack: Vec::new(),
            last_edit: std::time::Instant::now(),
            max_history,
        }
    }

    pub fn push_snapshot(&mut self, text: &str, cursor: usize, force: bool);
    pub fn undo(&mut self, current_text: &str, current_cursor: usize) -> Option<TextSnapshot>;
    pub fn redo(&mut self, current_text: &str, current_cursor: usize) -> Option<TextSnapshot>;
}
```

#### In-Note Find & Replace (`airgap/crates/app/src/state.rs` & `ui/editor.rs`)
```rust
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
    pub fn recalculate_matches(&mut self, text: &str);
    pub fn next_match(&mut self);
    pub fn prev_match(&mut self);
    pub fn replace_current(&mut self, text: &mut String, undo: &mut UndoManager);
    pub fn replace_all(&mut self, text: &mut String, undo: &mut UndoManager);
}
```

#### Scroll Synchronization State
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScrollDriver {
    #[default]
    None,
    Editor,
    Preview,
}

pub struct ScrollSyncState {
    pub driver: ScrollDriver,
    pub target_preview_scroll: Option<f32>,
    pub target_preview_line: Option<usize>,
    pub preview_line_anchors: Vec<(usize, f32)>, // (source_line, preview_y)
}
```

#### Image & Clipboard Paste Handler
```rust
pub struct ClipboardHandler;

impl ClipboardHandler {
    /// Checks if clipboard holds an image. If so, saves PNG into assets/ and returns markdown link.
    pub fn try_paste_image(vault_root: &std::path::Path, note_rel_path: &std::path::Path) -> Option<String>;

    /// Checks if clipboard text is a URL.
    pub fn is_url(text: &str) -> bool;
}
```

---

## 5. Verification Method

To independently verify the architecture and implementation:

1. **Dependency Audit (Zero Network Enforcement)**:
   ```bash
   cargo tree --target all | findstr /i "reqwest hyper curl ureq"
   ```
   *Expected result: 0 matches.*
2. **Workspace Test Suite Execution**:
   ```bash
   cargo test --manifest-path airgap/Cargo.toml
   ```
   *Expected result: All unit, integration, and stress tests pass with 0 failures.*
3. **Unit Tests to Implement for Worker**:
   - `test_view_mode_cycle`: Verify `Split -> EditorOnly -> PreviewOnly -> Split` transitions.
   - `test_url_safety_sanitization`: Verify `http://example.com`, `https://example.com`, `mailto:test@example.com` pass; verify `file:///etc/passwd`, `powershell.exe`, and invalid protocols are rejected.
   - `test_undo_redo_stack`: Verify snapshots coalesce on typing bursts but create discrete states on Enter, Tab, and Find/Replace; verify `undo()` and `redo()` restore exact text and cursor positions.
   - `test_list_continuation`: Verify Enter on `- [ ] Task` creates `- [ ] `; verify Enter on empty `- [ ] ` clears prefix; verify numbered lists increment `1. ` -> `2. `.
   - `test_find_and_replace`: Verify plain text, whole-word, and regex match finding, `next_match` wrapping, and `replace_all` reverse-order integrity.
   - `test_smart_url_paste`: Verify pasting URL over `"Selected Text"` yields `"[Selected Text](https://...)"`.
4. **Invalidation Conditions**:
   - Introducing any HTTP client crate (`reqwest`, `hyper`, `curl`, `ureq`) invalidates this handoff.
   - Direct execution of non-whitelisted URL schemes (`file://`, shell scripts) invalidates security.
   - UI thread blockage exceeding 16 ms during preview rendering or link opening invalidates responsiveness standards.
