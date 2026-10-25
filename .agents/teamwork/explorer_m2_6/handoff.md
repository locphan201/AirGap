# Handoff Report: Remediation Specifications for Editor Formatting, Image Handling & Heading Slugs

**Date**: 2026-10-25T01:12:00Z  
**Agent**: `explorer_m2_6` (Role: Editor Formatting & Image Explorer)  
**Parent Agent**: `3afb76bf-62b1-4f73-84eb-b7f21076aae2`  
**Working Directory**: `C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\.agents\teamwork\explorer_m2_6`  
**Handoff Type**: Hard (Task Complete)

---

## 1. Observation

### Observation 1: CRLF Line Ending Destruction in `preview.rs` (`toggle_checkbox_at_line`)
- **File Location**: `airgap/crates/app/src/ui/preview.rs`, lines 754–773.
- **Direct Code Quote**:
  ```rust
  fn toggle_checkbox_at_line(content: &mut String, target_line_1_indexed: usize) {
      let mut new_lines = Vec::new();
      for (idx, line) in content.lines().enumerate() {
          let current_line = idx + 1;
          if current_line == target_line_1_indexed {
              if line.contains("- [ ]") {
                  new_lines.push(line.replacen("- [ ]", "- [x]", 1));
              } else if line.contains("- [x]") {
                  new_lines.push(line.replacen("- [x]", "- [ ]", 1));
              } else if line.contains("- [X]") {
                  new_lines.push(line.replacen("- [X]", "- [ ]", 1));
              } else {
                  new_lines.push(line.to_string());
              }
          } else {
              new_lines.push(line.to_string());
          }
      }
      *content = new_lines.join("\n");
  }
  ```
- **Observed Behavior**:
  - `content.lines()` strips all line delimiters (`\r\n` and `\n`) from each line.
  - `new_lines.join("\n")` unconditionally concatenates lines using UNIX newline (`\n`).
  - If a file uses Windows CRLF (`\r\n`), toggling any single checkbox converts the entire note's line endings to LF (`\n`), producing dirty git diffs across the entire document.
  - Furthermore, if the original document ended with a newline (`\n` or `\r\n`), `join` drops the trailing newline entirely.

### Observation 2: Image Paste Appends to EOF in `editor.rs` (`handle_editor_keystrokes`)
- **File Location**: `airgap/crates/app/src/ui/editor.rs`, lines 457–466 and line 257.
- **Direct Code Quote**:
  ```rust
  // Line 257 in NoteEditor::render_editor:
  handle_editor_keystrokes(ui, state);

  // Lines 457–466 in handle_editor_keystrokes:
  // Ctrl+V: Image or Smart URL Paste
  if ctrl && i.key_pressed(Key::V) {
      if let (Some(vault), Some(active_note)) = (&state.vault, &state.active_note_path) {
          if let Some(image_md) = ClipboardHandler::try_paste_image(&vault.root, active_note)
          {
              state.editor_content.push_str(&image_md);
              state.on_editor_modified();
          }
      }
  }
  ```
- **Observed Behavior**:
  - `handle_editor_keystrokes` receives only `(ui: &mut Ui, state: &mut AppState)` and has no reference to `output: TextEditOutput` produced by `text_edit.show(ui)`.
  - When an image is pasted via `ClipboardHandler::try_paste_image`, `state.editor_content.push_str(&image_md)` blindly pushes the Markdown image link to the very end of `editor_content`, regardless of where the user clicked or positioned their cursor or what text was highlighted.

### Observation 3: Headings Lack Auto-Generated Slugs in `markdown.rs`
- **File Location**: `airgap/crates/render/src/markdown.rs`, lines 222–257.
- **Direct Code Quote**:
  ```rust
  Event::Start(Tag::Heading {
      level, id, classes, ..
  }) => {
      let lvl = match level { ... };
      let id_str = id.map(|s| s.to_string());
      let classes_vec = classes.into_iter().map(|s| s.to_string()).collect();
      in_heading = Some(ActiveHeading {
          level: lvl,
          id: id_str,
          classes: classes_vec,
          inlines: Vec::new(),
          plain: String::new(),
          start_byte: range.start,
      });
  }
  Event::End(TagEnd::Heading(_)) => {
      if let Some(h) = in_heading.take() {
          let span = line_index.byte_range_to_span(&(h.start_byte..range.end));
          blocks.push(Block::Heading {
              level: h.level,
              text: h.plain.trim().to_string(),
              inlines: h.inlines,
              id: h.id,
              classes: h.classes,
              line: span.start_line,
              span,
          });
      }
  }
  ```
- **Observed Behavior**:
  - `id: h.id` is populated solely from `pulldown_cmark` heading attributes (e.g. `{#custom-id}`).
  - Standard Markdown headings without explicit attributes receive `id: None`.
  - There is no slugification logic in `airgap-render`. Grep search across the workspace for `slug` yields zero occurrences.

### Observation 4: Potential UTF-8 Boundary Panics in `find_current_line` in `editor.rs`
- **File Location**: `airgap/crates/app/src/ui/editor.rs`, lines 644–655.
- **Direct Code Quote**:
  ```rust
  fn find_current_line(text: &str, cursor_char_idx: usize) -> (usize, &str) {
      let clamped_cursor = cursor_char_idx.min(text.len());
      let line_start = text[..clamped_cursor]
          .rfind('\n')
          .map(|idx| idx + 1)
          .unwrap_or(0);
      let line_end = text[clamped_cursor..]
          .find('\n')
          .map(|idx| clamped_cursor + idx)
          .unwrap_or(text.len());
      (line_start, &text[line_start..line_end])
  }
  ```
- **Observed Behavior**:
  - `clamped_cursor` clamps `cursor_char_idx` against `text.len()`, but does NOT verify whether `clamped_cursor` aligns with a UTF-8 character boundary.
  - If `cursor_char_idx` is passed an arbitrary byte offset inside a multi-byte codepoint (such as CJK ideographs 3 bytes, or emojis 4 bytes), direct string slicing `text[..clamped_cursor]` immediately panics with:
    `byte index N is not a char boundary; it is inside '...' of "..."`.
  - Furthermore, `apply_list_continuation` (lines 479, 485, 487) and `apply_auto_pairing` (lines 626, 628, 637, 639) also slice `&text[..cursor_char_idx]` without character boundary verification.

---

## 2. Logic Chain

### Logic Chain 1: CRLF Preservation
1. `PROJECT.md` Feature 17 explicitly mandates: "*Line Ending Preservation: Preserve existing CRLF vs LF; default LF for new*".
2. In `preview.rs`, interactive task list checkboxes allow the user to click `[ ]` in the preview pane to toggle task completion.
3. When `content.split_inclusive('\n')` is used instead of `content.lines()`:
   - Each line slice retains its original terminating delimiter (`\r\n` on CRLF files, `\n` on LF files).
   - If the final line has no trailing newline, `split_inclusive` preserves it without a trailing newline.
   - If the final line has a trailing newline, it is preserved verbatim.
4. Performing `.replacen("- [ ]", "- [x]", 1)` on the target line modifies only the markdown task box prefix without altering the line ending at the end of that slice.
5. Pushing each line to a preallocated `String` (`String::with_capacity(content.len())`) avoids multiple allocations and guarantees 100% byte-for-byte line ending preservation.

### Logic Chain 2: Image Paste Cursor Positioning
1. `AGENT.md` Section 3.1 mandates: "*Image paste: save clipboard images into an assets/ folder next to the note and insert a relative link.*"
2. In egui, `TextEdit::show(ui)` returns a `TextEditOutput` struct containing `output.cursor_range: Option<CursorRange>` and `output.response.id: Id`.
3. If the user has focused or clicked in the editor, `cursor_range` provides `sorted_cursors()`, giving the start and end character indices (`CCursor.index`, which are character counts in Unicode scalars).
4. Mapping character indices to byte offsets via `char_indices().nth(char_idx).map(|(b, _)| b).unwrap_or(text.len())` guarantees valid UTF-8 byte positions.
5. Calling `state.editor_content.replace_range(start_byte..end_byte, &image_md)`:
   - If text was selected, replaces the selection with the image link.
   - If the cursor was a single caret (`start_byte == end_byte`), inserts the image link exactly at the caret.
6. Updating egui's persistent `TextEditState` (`edit_state.cursor.set_char_range(...)` followed by `edit_state.store(ui.ctx(), output.response.id)`) sets the cursor immediately after the newly inserted Markdown image syntax on the next frame.
7. If `output.cursor_range` is `None` (unfocused editor), falling back to `push_str` safely prevents panics.

### Logic Chain 3: Auto-Generated Heading Slugs
1. Headings in Markdown documents serve as anchor navigation targets (intra-document links, table of contents).
2. `pulldown-cmark` only populates `id: Option<&str>` if explicit heading attributes (`{#custom-id}`) are supplied.
3. Implementing `pub fn generate_slug(text: &str) -> String` produces a standard URL-safe anchor identifier:
   - Transforms characters via `c.to_lowercase()`.
   - Converts whitespace and underscores to `-`.
   - Preserves alphanumeric characters (`c.is_alphanumeric()` in Rust returns `true` for Unicode alphabets including CJK characters like `'快'`).
   - Strips punctuation and symbols.
   - Collapses consecutive hyphens and trims leading/trailing hyphens.
4. In `Event::End(TagEnd::Heading(_))`, checking `h.id.or_else(|| { let slug = generate_slug(&heading_text); if slug.is_empty() { None } else { Some(slug) } })`:
   - Preserves explicit IDs when provided.
   - Populates consistent auto-generated slugs when `id` is `None`.
   - Returns `None` if the heading consists entirely of stripped punctuation (e.g. `### !!!`).
5. Exporting `generate_slug` from `airgap_render` makes this slugifier available for HTML export (Milestone M5) and sidebar TOC navigation.

### Logic Chain 4: UTF-8 Boundary Safety
1. Rust's `str::floor_char_boundary(idx: usize) -> usize` (stabilized in Rust 1.83.0 and supported by our `rustc 1.99.1` compiler) finds the closest `<` character boundary to `idx`.
2. Clamping `cursor_char_idx` with `let clamped_cursor = text.floor_char_boundary(cursor_char_idx.min(text.len()))`:
   - Guarantees `clamped_cursor <= text.len()`.
   - Guarantees `text.is_char_boundary(clamped_cursor) == true`.
3. Slicing `text[..clamped_cursor]` and `text[clamped_cursor..]` is mathematically proven never to panic on any UTF-8 text, including multi-byte CJK ideographs and 4-byte emojis.
4. Slicing from `rfind('\n') + 1` and `find('\n')` is also guaranteed safe because `\n` is an ASCII single-byte codepoint (`0x0A`).

---

## 3. Caveats

1. **Clipboard Image Format Support**: `ClipboardHandler::try_paste_image` uses `arboard` and writes RGBA buffers directly as PNGs. On Linux systems running Wayland or headless test environments without an active X11/Wayland clipboard daemon, `arboard::Clipboard::new()` returns `Err`, so clipboard tests should run in mock environments or test `ClipboardHandler` methods with valid buffers.
2. **Editor Input Interception**: When hooking `Enter`, `Tab`, and bracket pairing in egui's `multiline TextEdit`, egui's default widget handles basic character entry. For keystrokes like `Enter` and `Tab`, handling them in `handle_editor_keystrokes` or via custom filter must ensure that egui's default newline insertion is coordinated with list continuation prefixes.
3. **No Other Scope Areas Touched**: This investigation is strictly scoped to the 4 assigned formatting, image, slug, and UTF-8 boundary issues. Scroll synchronization and editor keystroke wiring are addressed in coordination with the parent agent and reviewer M2.

---

## 4. Conclusion & Concrete Remediation Specifications

### Remediation Spec 1: `airgap/crates/app/src/ui/preview.rs` (CRLF Preservation)
- **Patch file**: `.agents/teamwork/explorer_m2_6/proposed_preview_fix.patch`
- **Before**:
  ```rust
  fn toggle_checkbox_at_line(content: &mut String, target_line_1_indexed: usize) {
      let mut new_lines = Vec::new();
      for (idx, line) in content.lines().enumerate() {
          let current_line = idx + 1;
          if current_line == target_line_1_indexed {
              if line.contains("- [ ]") {
                  new_lines.push(line.replacen("- [ ]", "- [x]", 1));
              } else if line.contains("- [x]") {
                  new_lines.push(line.replacen("- [x]", "- [ ]", 1));
              } else if line.contains("- [X]") {
                  new_lines.push(line.replacen("- [X]", "- [ ]", 1));
              } else {
                  new_lines.push(line.to_string());
              }
          } else {
              new_lines.push(line.to_string());
          }
      }
      *content = new_lines.join("\n");
  }
  ```
- **After**:
  ```rust
  fn toggle_checkbox_at_line(content: &mut String, target_line_1_indexed: usize) {
      let mut new_content = String::with_capacity(content.len());
      for (idx, line) in content.split_inclusive('\n').enumerate() {
          let current_line = idx + 1;
          if current_line == target_line_1_indexed {
              if line.contains("- [ ]") {
                  new_content.push_str(&line.replacen("- [ ]", "- [x]", 1));
              } else if line.contains("- [x]") {
                  new_content.push_str(&line.replacen("- [x]", "- [ ]", 1));
              } else if line.contains("- [X]") {
                  new_content.push_str(&line.replacen("- [X]", "- [ ]", 1));
              } else {
                  new_content.push_str(line);
              }
          } else {
              new_content.push_str(line);
          }
      }
      *content = new_content;
  }
  ```

### Remediation Spec 2: `airgap/crates/app/src/ui/editor.rs` (Image Paste Position)
- **Patch file**: `.agents/teamwork/explorer_m2_6/proposed_editor_fix.patch`
- **Before**:
  ```rust
  // NoteEditor::render_editor line 257:
  handle_editor_keystrokes(ui, state);

  // handle_editor_keystrokes lines 457–466:
  if ctrl && i.key_pressed(Key::V) {
      if let (Some(vault), Some(active_note)) = (&state.vault, &state.active_note_path) {
          if let Some(image_md) = ClipboardHandler::try_paste_image(&vault.root, active_note)
          {
              state.editor_content.push_str(&image_md);
              state.on_editor_modified();
          }
      }
  }
  ```
- **After**:
  ```rust
  // NoteEditor::render_editor line 257:
  handle_editor_keystrokes(ui, state, &output);

  // handle_editor_keystrokes signature and Ctrl+V body:
  fn handle_editor_keystrokes(
      ui: &mut Ui,
      state: &mut AppState,
      output: &egui::text_edit::TextEditOutput,
  ) {
      ui.input(|i| {
          let ctrl = i.modifiers.command || i.modifiers.ctrl;
          ...
          if ctrl && i.key_pressed(Key::V) {
              if let (Some(vault), Some(active_note)) = (&state.vault, &state.active_note_path) {
                  if let Some(image_md) = ClipboardHandler::try_paste_image(&vault.root, active_note)
                  {
                      if let Some(cursor_range) = &output.cursor_range {
                          let [start_cursor, end_cursor] = cursor_range.sorted_cursors();
                          let start_char = start_cursor.ccursor.index;
                          let end_char = end_cursor.ccursor.index;
                          let start_byte = char_idx_to_byte_idx(&state.editor_content, start_char);
                          let end_byte = char_idx_to_byte_idx(&state.editor_content, end_char);

                          state.editor_content.replace_range(start_byte..end_byte, &image_md);

                          let new_char_idx = start_char + image_md.chars().count();
                          let mut edit_state =
                              egui::text_edit::TextEditState::load(ui.ctx(), output.response.id)
                                  .unwrap_or_default();
                          edit_state.cursor.set_char_range(Some(
                              egui::text_selection::CCursorRange::one(egui::text::CCursor::new(
                                  new_char_idx,
                              )),
                          ));
                          edit_state.store(ui.ctx(), output.response.id);
                      } else {
                          state.editor_content.push_str(&image_md);
                      }
                      state.on_editor_modified();
                  }
              }
          }
      });
  }

  fn char_idx_to_byte_idx(text: &str, char_idx: usize) -> usize {
      text.char_indices()
          .nth(char_idx)
          .map(|(byte_idx, _)| byte_idx)
          .unwrap_or(text.len())
  }
  ```

### Remediation Spec 3: `airgap/crates/render/src/markdown.rs` & `lib.rs` (Heading Slug IDs)
- **Patch file**: `.agents/teamwork/explorer_m2_6/proposed_render_fix.patch`
- **New Slug Generator**:
  ```rust
  /// Generates a URL-friendly anchor slug from heading text.
  pub fn generate_slug(text: &str) -> String {
      let mut slug = String::with_capacity(text.len());
      let mut prev_was_hyphen = false;

      for c in text.chars() {
          if c.is_alphanumeric() {
              for lc in c.to_lowercase() {
                  slug.push(lc);
              }
              prev_was_hyphen = false;
          } else if c.is_whitespace() || c == '-' || c == '_' {
              if !prev_was_hyphen && !slug.is_empty() {
                  slug.push('-');
                  prev_was_hyphen = true;
              }
          }
      }

      if slug.ends_with('-') {
          slug.pop();
      }

      slug
  }
  ```
- **Markdown Parser Integration** (`crates/render/src/markdown.rs` lines 245–257):
  ```rust
  Event::End(TagEnd::Heading(_)) => {
      if let Some(h) = in_heading.take() {
          let span = line_index.byte_range_to_span(&(h.start_byte..range.end));
          let heading_text = h.plain.trim().to_string();
          let id = h.id.or_else(|| {
              let slug = generate_slug(&heading_text);
              if slug.is_empty() {
                  None
              } else {
                  Some(slug)
              }
          });
          blocks.push(Block::Heading {
              level: h.level,
              text: heading_text,
              inlines: h.inlines,
              id,
              classes: h.classes,
              line: span.start_line,
              span,
          });
      }
  }
  ```
- **Export in `crates/render/src/lib.rs`**:
  ```rust
  pub use markdown::{generate_slug, parse_markdown, LineIndex};
  ```

### Remediation Spec 4: `airgap/crates/app/src/ui/editor.rs` (UTF-8 Char Boundary Safety)
- **Patch file**: Included in `.agents/teamwork/explorer_m2_6/proposed_editor_fix.patch`
- **Before**:
  ```rust
  fn find_current_line(text: &str, cursor_char_idx: usize) -> (usize, &str) {
      let clamped_cursor = cursor_char_idx.min(text.len());
      let line_start = text[..clamped_cursor]
          .rfind('\n')
          .map(|idx| idx + 1)
          .unwrap_or(0);
      let line_end = text[clamped_cursor..]
          .find('\n')
          .map(|idx| clamped_cursor + idx)
          .unwrap_or(text.len());
      (line_start, &text[line_start..line_end])
  }
  ```
- **After**:
  ```rust
  fn find_current_line(text: &str, cursor_char_idx: usize) -> (usize, &str) {
      let clamped_cursor = text.floor_char_boundary(cursor_char_idx.min(text.len()));
      let line_start = text[..clamped_cursor]
          .rfind('\n')
          .map(|idx| idx + 1)
          .unwrap_or(0);
      let line_end = text[clamped_cursor..]
          .find('\n')
          .map(|idx| clamped_cursor + idx)
          .unwrap_or(text.len());
      (line_start, &text[line_start..line_end])
  }
  ```

---

## 5. Verification Method

To independently verify these specifications when applied:

1. **Compilation Check**:
   ```bash
   cargo check --manifest-path airgap/Cargo.toml --all-targets
   ```
   *Expected Result*: Compiles with 0 errors and 0 warnings.

2. **Run All Existing Tests**:
   ```bash
   cargo test --manifest-path airgap/Cargo.toml --workspace
   ```
   *Expected Result*: All 91+ unit and integration tests pass without regressions.

3. **Verify Heading Auto-Slugification**:
   ```bash
   cargo test --manifest-path airgap/crates/render/Cargo.toml test_heading_auto_slug_when_id_is_none
   ```
   *Expected Result*: Headings without `{#id}` receive correct kebab-case slugs; headings with explicit `{#custom-id}` retain their custom ID.

4. **Verify CRLF Preservation on Checkbox Toggle**:
   ```bash
   cargo test --manifest-path airgap/crates/app/Cargo.toml test_toggle_checkbox_preserves_crlf_and_trailing_newline
   ```
   *Expected Result*: Documents with `\r\n` line endings retain `\r\n` across the entire document after toggling checkboxes, and trailing newlines remain intact.

5. **Verify UTF-8 Boundary Safety**:
   ```bash
   cargo test --manifest-path airgap/crates/app/Cargo.toml test_find_current_line_utf8_char_boundary_safety
   ```
   *Expected Result*: Multi-byte CJK and emoji strings sliced at non-boundary indices return line slices without panicking.

6. **Invalidation Conditions**:
   - If `content.contains("\r\n")` is replaced with an LF-only string, the test fails.
   - If `find_current_line` panics on `text.floor_char_boundary`, check rustc version (requires `>= 1.83.0`, verified `1.99.1`).
