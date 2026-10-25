# Handoff Report: Scroll Synchronization & Editor Conveniences Wiring Remediation Specification

**Date**: 2026-10-25T01:12:00Z  
**Agent**: `explorer_m2_4` (Role: Scroll & Editor Wire Explorer)  
**Parent Agent**: `3afb76bf-62b1-4f73-84eb-b7f21076aae2`  
**Working Directory**: `C:\Users\phan vinh loc\VSC\AmenoAI\Central_Management\.agents\teamwork\explorer_m2_4`  
**Target Codebase**: `airgap/crates/app`  
**Handoff Type**: Hard (Task Complete)  

---

## 1. Observation

### 1.1 Codebase Observations

1. **Facade Scroll Sync in `state.rs`, `editor.rs`, and `preview.rs`**:
   - In `airgap/crates/app/src/state.rs` (lines 41–49):
     ```rust
     #[derive(Debug, Clone, Default)]
     pub struct ScrollSyncState {
         pub driver: ScrollDriver,
         pub target_preview_scroll: Option<f32>,
         pub target_editor_scroll: Option<f32>,
         pub editor_line: usize,
         pub preview_scroll_y: f32,
         pub preview_content_height: f32,
     }
     ```
   - In `airgap/crates/app/src/ui/editor.rs` (lines 160–166, 236):
     ```rust
     egui::ScrollArea::vertical()
         .auto_shrink([false, false])
         .show(ui, |ui| {
             if ui.rect_contains_pointer(ui.max_rect()) {
                 state.scroll_sync.driver = ScrollDriver::Editor;
             }
             ...
             state.scroll_sync.editor_line = state.cursor_line;
     ```
   - In `airgap/crates/app/src/ui/preview.rs` (lines 55–57, 143–150):
     ```rust
     let scroll_area = egui::ScrollArea::vertical().auto_shrink([false, false]);
     let scroll_output = scroll_area.show(ui, |ui| { ... });
     state.scroll_sync.preview_scroll_y = scroll_output.state.offset.y;
     state.scroll_sync.preview_content_height = scroll_output.content_size.y;
     if ui.rect_contains_pointer(scroll_output.inner_rect) {
         state.scroll_sync.driver = ScrollDriver::Preview;
     }
     ```
   - Across the entire codebase, neither `target_preview_scroll` nor `target_editor_scroll` is ever read, passed, or applied. Neither `ScrollArea::vertical_scroll_offset` nor `ui.scroll_to_rect` is ever called.

2. **Unwired Editor Conveniences in `editor.rs`**:
   - In `airgap/crates/app/src/ui/editor.rs` (lines 419–467):
     ```rust
     fn handle_editor_keystrokes(ui: &mut Ui, state: &mut AppState) {
         ui.input(|i| {
             let ctrl = i.modifiers.command || i.modifiers.ctrl;
             let shift = i.modifiers.shift;
             // Escape, Ctrl+Z, Ctrl+Y, and Ctrl+V (image paste)
             ...
         });
     }
     ```
   - The helper functions `apply_list_continuation` (lines 472–566), `apply_indentation` (lines 570–604), and `apply_auto_pairing` (lines 607–642) are defined in `editor.rs` but are NEVER called inside `render_editor` or `handle_editor_keystrokes`.
   - `ClipboardHandler::try_paste_smart_url` is defined in `airgap/crates/app/src/clipboard.rs` (lines 49–60), but in `editor.rs` (lines 458–466), `Ctrl+V` only attempts `try_paste_image` and appends `image_md` to the end of `state.editor_content` via `state.editor_content.push_str(&image_md)`.
   - In `airgap/crates/app/tests/test_m2_integration.rs` (lines 249–299), tests only invoke `apply_list_continuation`, `apply_indentation`, `apply_auto_pairing`, and `try_paste_smart_url` as isolated standalone functions without testing UI event dispatch.

3. **CRLF Silent Conversion in `preview.rs`**:
   - In `airgap/crates/app/src/ui/preview.rs` (lines 754–773):
     ```rust
     fn toggle_checkbox_at_line(content: &mut String, target_line_1_indexed: usize) {
         let mut new_lines = Vec::new();
         for (idx, line) in content.lines().enumerate() { ... }
         *content = new_lines.join("\n");
     }
     ```
   - `content.lines()` strips `\r\n` and `*content = new_lines.join("\n")` always rejoins with `\n`, silently mutating CRLF files into LF files.

4. **UTF-8 Boundary Safety in `find_current_line`**:
   - In `airgap/crates/app/src/ui/editor.rs` (lines 644–655):
     ```rust
     fn find_current_line(text: &str, cursor_char_idx: usize) -> (usize, &str) {
         let clamped_cursor = cursor_char_idx.min(text.len());
         let line_start = text[..clamped_cursor].rfind('\n').map(|idx| idx + 1).unwrap_or(0);
         let line_end = text[clamped_cursor..].find('\n').map(|idx| clamped_cursor + idx).unwrap_or(text.len());
         (line_start, &text[line_start..line_end])
     }
     ```
   - Slicing `text[..clamped_cursor]` panics if `clamped_cursor` falls within a multi-byte UTF-8 character boundary.

5. **egui 0.31 Engine Internals**:
   - `egui::ScrollArea::vertical().vertical_scroll_offset(offset: f32) -> Self` directly sets `state.offset.y = offset` in `Prepared::begin` (`egui-0.31.1/src/containers/scroll_area.rs`, lines 341, 544).
   - In `egui::TextEdit::multiline` (`egui-0.31.1/src/widgets/text_edit/builder.rs`, lines 947–984), `TextEdit` handles `Event::Paste`, `Event::Text`, `Event::Key(Key::Tab)`, and `Event::Key(Key::Enter)` by immediately mutating its internal buffer and deleting active selections.
   - Any interception that occurs *after* `TextEdit::show` runs is too late because the active selection has already been deleted and a raw `\n` or `\t` has already been inserted.
   - Keystroke events can be intercepted *before* `TextEdit::show` by inspecting `ui.input_mut(|i| ...)`, modifying `state.editor_content`, updating `TextEditState` cursor position via `TextEditState::store(ui.ctx(), editor_id)`, and consuming the handled event via `i.events.retain(...)`.

---

## 2. Logic Chain

### 2.1 Scroll Synchronization Architecture (Infinite Loop Prevention & Actuation)

1. **Root Cause of Feedback Loops**:
   - In a dual-scroll setup (Editor on the left, Preview on the right), if pane A updates its scroll and sets pane B's scroll target, and pane B's movement is then interpreted as user interaction that sets pane A's scroll target, a bidirectional cyclic ping-pong loop occurs.
   - Discrepancies in rendered heights (monospace text vs rich HTML-like preview with diagrams and images) exacerbate this, causing jitter, oscillation, or scroll lock.

2. **Resolution — Strict Driver Authority with One-Way Consumption**:
   - At any moment, only **one** pane is designated as the active `ScrollDriver` (`Editor` or `Preview`), based on user pointer location and input activity.
   - **Rule 1 (Driver Immunity)**: The active `ScrollDriver` NEVER receives a programmatic scroll override (`.vertical_scroll_offset(...)` is NEVER called on the driver). The driver scrolls purely according to native user input (mouse wheel, scrollbar drag, kinetic scroll, cursor movement).
   - **Rule 2 (Follower Actuation via `.take()`)**: Only the follower pane calls `.vertical_scroll_offset(target)`. The target offset is stored in `ScrollSyncState` as `Option<f32>` and consumed using `.take()` on each frame.
   - **Rule 3 (One-Way Update)**: The follower pane NEVER computes or mutates the target scroll for the driver. When `driver == ScrollDriver::Editor`, Preview consumes `target_preview_scroll` and does not set `target_editor_scroll`. When `driver == ScrollDriver::Preview`, Editor consumes `target_editor_scroll` and does not set `target_preview_scroll`.
   - **Rule 4 (Proportional Mapping)**:
     - Editor scrollable range: `max_editor = (editor_content_height - editor_viewport_height).max(1.0)`
     - Editor ratio: `ratio = (editor_offset_y / max_editor).clamp(0.0, 1.0)`
     - Target preview offset: `target_preview = ratio * (preview_content_height - preview_viewport_height).max(0.0)`
     - Symmetric calculation applies when Preview is the driver.
   - **Proof of Loop Prevention**: Because the follower never emits scroll commands back to the driver, feedback is topologically impossible (directed acyclic data flow).

3. **Field Clean-up in `ScrollSyncState`**:
   - Keep all fields actively functional:
     - `driver: ScrollDriver` — Current active driver (`None`, `Editor`, `Preview`).
     - `target_preview_scroll: Option<f32>` — Consumed by `preview.rs` via `.take()`.
     - `target_editor_scroll: Option<f32>` — Consumed by `editor.rs` via `.take()`.
     - `editor_line: usize` — Current active line in editor.
     - `editor_content_height: f32` — Content height reported by editor `ScrollAreaOutput`.
     - `editor_viewport_height: f32` — Viewport height reported by editor `ScrollAreaOutput`.
     - `preview_scroll_y: f32` — Current scroll offset of preview pane.
     - `preview_content_height: f32` — Content height reported by preview `ScrollAreaOutput`.
     - `preview_viewport_height: f32` — Viewport height reported by preview `ScrollAreaOutput`.
   - Every single field has active read/write sites; zero dead fields remain.

---

### 2.2 Editor Conveniences Wiring Architecture

1. **Pre-Show Interception Hook**:
   - `TextEdit::show(ui)` processes raw events internally. If the user presses `[` with text `"hello"` selected, `TextEdit` deletes `"hello"` and inserts `[`.
   - To implement auto-pairing wrapping (`[hello]`), smart URL paste (`[selected](url)`), list continuation (`\n- [ ] `), and 2-space tab indent (`  `), interception MUST happen in a hook executed immediately **before** `text_edit.show(ui)`.
   
2. **Hook Execution Pipeline (`handle_editor_keystroke_conveniences`)**:
   - **Pre-condition**: Check that the editor widget has keyboard focus (`ui.memory(|m| m.has_focus(editor_id))`). If not focused, pass through.
   - **Cursor & Selection Resolution**:
     - Retrieve active cursor / selection from `state.editor_char_range` (recorded from previous frame's `output.cursor_range.as_sorted_char_range()`), or fallback to `TextEditState::load(ui.ctx(), editor_id)`.
     - Convert character indices (`CCursor`) to byte offsets in `state.editor_content` using UTF-8 safe `char_to_byte_idx`.
     - Extract `selected_text = &state.editor_content[byte_start..byte_end]` if `byte_start < byte_end`.
   - **Key 1: Enter Key (`apply_list_continuation`)**:
     - Detect `Event::Key { key: Key::Enter, pressed: true, modifiers, .. }` where modifiers are empty (no Shift/Ctrl/Alt).
     - Call `apply_list_continuation(&state.editor_content, byte_start)`.
     - If `Some((new_text, new_cursor_byte))` is returned:
       - Update `state.editor_content = new_text`.
       - Update cursor position in `TextEditState` to `byte_to_char_idx(&state.editor_content, new_cursor_byte)`.
       - Remove `Event::Key { key: Key::Enter, .. }` from `ui.input_mut(|i| i.events)`.
       - Call `state.on_editor_modified()`.
     - If `None` is returned:
       - Leave `Event::Key { key: Key::Enter, .. }` untouched. `TextEdit` will perform its default standard newline insertion.
   - **Key 2: Tab & Shift+Tab (`apply_indentation`)**:
     - Detect `Event::Key { key: Key::Tab, pressed: true, modifiers, .. }`.
     - Determine `is_outdent = modifiers.shift`.
     - Call `apply_indentation(&state.editor_content, byte_start, is_outdent)`.
     - Update `state.editor_content = new_text`.
     - Update cursor position in `TextEditState` to `byte_to_char_idx(&state.editor_content, new_cursor_byte)`.
     - Remove `Event::Key { key: Key::Tab, .. }` from `ui.input_mut(|i| i.events)`.
     - Call `state.on_editor_modified()`.
   - **Key 3: Auto-Pairing Enclosures (`(`, `[`, `{`, `"`, ````)**:
     - Detect `Event::Text(s)` where `s` is `"("`, `"["`, `"{"`, `"\""`, or `"\`"`.
     - Call `apply_auto_pairing(&state.editor_content, byte_start, ch, selected_text)`.
     - If `Some((new_text, new_cursor_byte))` is returned:
       - Update `state.editor_content = new_text`.
       - Update cursor position in `TextEditState` to `byte_to_char_idx(&state.editor_content, new_cursor_byte)`.
       - Remove the matching `Event::Text` from `ui.input_mut(|i| i.events)`.
       - Call `state.on_editor_modified()`.
   - **Key 4: Smart URL Paste (`Ctrl+V` / `Cmd+V`)**:
     - Detect `Ctrl+V` or `Event::Paste`.
     - First check: If clipboard contains an image via `ClipboardHandler::try_paste_image(&vault.root, active_note)`:
       - Replace selection (or insert at cursor) with `image_md` (Fixing Finding 4: image pasted at cursor, not appended to file!).
       - Consume paste event.
     - Second check: If `!selected_text.is_empty()` and clipboard contains text:
       - Check `ClipboardHandler::try_paste_smart_url(&clipboard_text, selected_text)`.
       - If `Some(markdown_link)` is returned:
         - Replace `byte_start..byte_end` with `markdown_link`.
         - Place cursor after the inserted link.
         - Consume paste event.
     - Third check: If neither image nor smart URL applies, leave paste event untouched for standard text pasting.

---

## 3. Caveats

1. **Non-ASCII Unicode Character Indexing**:
   - egui's `CCursor` represents character count, whereas Rust string indexing uses byte offsets. Slicing Rust `&str` directly with `CCursor.index` causes immediate panics on CJK text, accented characters, or emojis. Conversion helper functions `char_to_byte_idx` and `byte_to_char_idx` are mandatory.
2. **Scroll Area ID Stability**:
   - Explicit `id_salt` strings (`"airgap_editor_scroll_area"` and `"airgap_preview_scroll_area"`) must be assigned to both ScrollAreas to ensure stable persisted offsets across view mode toggles (`Ctrl+\`).
3. **Single View Mode Optimization**:
   - Scroll synchronization is only meaningful in `ViewMode::Split`. When `view_mode != ViewMode::Split`, scroll calculations and overrides must be skipped.

---

## 4. Conclusion & Concrete Code Diff Specification

### 4.1 Changes to `airgap/crates/app/src/state.rs`

Update `ScrollSyncState` and add `editor_char_range` to `AppState`:

```rust
// In airgap/crates/app/src/state.rs

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

pub struct AppState {
    ...
    // M2 additions
    pub view_mode: ViewMode,
    pub split_ratio: f32,
    pub scroll_sync: ScrollSyncState,
    pub undo_manager: UndoManager,
    pub find_replace: FindReplaceState,
    pub editor_char_range: Option<std::ops::Range<usize>>,
}
```

In `AppState::default()`:
```rust
    editor_char_range: None,
```

In `AppState::select_note`:
```rust
    self.editor_char_range = None;
```

---

### 4.2 Changes to `airgap/crates/app/src/ui/editor.rs`

1. **UTF-8 Boundary Fix in `find_current_line` (Fixing Finding 6)**:
```rust
fn floor_char_boundary(s: &str, mut index: usize) -> usize {
    if index >= s.len() {
        return s.len();
    }
    while index > 0 && !s.is_char_boundary(index) {
        index -= 1;
    }
    index
}

fn find_current_line(text: &str, cursor_char_idx: usize) -> (usize, &str) {
    let clamped_cursor = floor_char_boundary(text, cursor_char_idx.min(text.len()));
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

pub fn char_to_byte_idx(text: &str, char_idx: usize) -> usize {
    text.char_indices()
        .nth(char_idx)
        .map(|(byte_idx, _)| byte_idx)
        .unwrap_or(text.len())
}

pub fn byte_to_char_idx(text: &str, byte_idx: usize) -> usize {
    text[..byte_idx.min(text.len())].chars().count()
}
```

2. **Pre-Execution Convenience Interceptor (`handle_editor_keystroke_conveniences`)**:
```rust
fn set_editor_cursor(ui: &mut Ui, editor_id: egui::Id, char_idx: usize) {
    let ccursor = egui::text::CCursor::new(char_idx);
    let range = egui::text_selection::CCursorRange::one(ccursor);
    let mut tes = egui::text_edit::TextEditState::load(ui.ctx(), editor_id).unwrap_or_default();
    tes.cursor.set_char_range(Some(range));
    tes.store(ui.ctx(), editor_id);
}

fn handle_editor_keystroke_conveniences(ui: &mut Ui, state: &mut AppState, editor_id: egui::Id) {
    let is_focused = ui.memory(|m| m.has_focus(editor_id));
    if !is_focused {
        return;
    }

    let (char_start, char_end) = if let Some(range) = &state.editor_char_range {
        (range.start, range.end)
    } else if let Some(tes) = egui::text_edit::TextEditState::load(ui.ctx(), editor_id) {
        if let Some(cr) = tes.cursor.char_range() {
            let [min, max] = cr.sorted();
            (min.index, max.index)
        } else {
            (0, 0)
        }
    } else {
        (0, 0)
    };

    let byte_start = char_to_byte_idx(&state.editor_content, char_start);
    let byte_end = char_to_byte_idx(&state.editor_content, char_end);
    let selected_text = if byte_start < byte_end && byte_end <= state.editor_content.len() {
        state.editor_content[byte_start..byte_end].to_string()
    } else {
        String::new()
    };

    ui.input_mut(|i| {
        let ctrl = i.modifiers.command || i.modifiers.ctrl;

        // 1. Ctrl+V: Image Paste at cursor or Smart URL Paste on selection
        let paste_pressed = ctrl && i.events.iter().any(|e| matches!(e, egui::Event::Key { key: Key::V, pressed: true, .. } | egui::Event::Paste(_)));
        if paste_pressed {
            // Check image paste first
            if let (Some(vault), Some(active_note)) = (&state.vault, &state.active_note_path) {
                if let Some(image_md) = ClipboardHandler::try_paste_image(&vault.root, active_note) {
                    state.editor_content.replace_range(byte_start..byte_end, &image_md);
                    let new_byte = byte_start + image_md.len();
                    let new_char = byte_to_char_idx(&state.editor_content, new_byte);
                    set_editor_cursor(ui, editor_id, new_char);
                    state.on_editor_modified();
                    i.events.retain(|e| !matches!(e, egui::Event::Paste(_) | egui::Event::Key { key: Key::V, pressed: true, .. }));
                    return;
                }
            }

            // Check smart URL paste on selection
            if !selected_text.is_empty() {
                let clipboard_text = arboard::Clipboard::new().ok().and_then(|mut c| c.get_text().ok());
                if let Some(clip_str) = clipboard_text {
                    if let Some(smart_link) = ClipboardHandler::try_paste_smart_url(&clip_str, &selected_text) {
                        state.editor_content.replace_range(byte_start..byte_end, &smart_link);
                        let new_byte = byte_start + smart_link.len();
                        let new_char = byte_to_char_idx(&state.editor_content, new_byte);
                        set_editor_cursor(ui, editor_id, new_char);
                        state.on_editor_modified();
                        i.events.retain(|e| !matches!(e, egui::Event::Paste(_) | egui::Event::Key { key: Key::V, pressed: true, .. }));
                        return;
                    }
                }
            }
        }

        // 2. Tab & Shift+Tab indentation
        let mut tab_action = None;
        for e in &i.events {
            if let egui::Event::Key { key: Key::Tab, pressed: true, modifiers, .. } = e {
                tab_action = Some(modifiers.shift);
                break;
            }
        }
        if let Some(is_outdent) = tab_action {
            let (new_text, new_cursor_byte) = apply_indentation(&state.editor_content, byte_start, is_outdent);
            state.editor_content = new_text;
            let new_char = byte_to_char_idx(&state.editor_content, new_cursor_byte);
            set_editor_cursor(ui, editor_id, new_char);
            state.on_editor_modified();
            i.events.retain(|e| !matches!(e, egui::Event::Key { key: Key::Tab, pressed: true, .. }));
            return;
        }

        // 3. Enter key: List/Task/Quote continuation
        let enter_pressed = i.events.iter().any(|e| matches!(e, egui::Event::Key { key: Key::Enter, pressed: true, modifiers, .. } if !modifiers.shift && !modifiers.ctrl && !modifiers.command && !modifiers.alt));
        if enter_pressed {
            if let Some((new_text, new_cursor_byte)) = apply_list_continuation(&state.editor_content, byte_start) {
                state.editor_content = new_text;
                let new_char = byte_to_char_idx(&state.editor_content, new_cursor_byte);
                set_editor_cursor(ui, editor_id, new_char);
                state.on_editor_modified();
                i.events.retain(|e| !matches!(e, egui::Event::Key { key: Key::Enter, pressed: true, .. }));
                return;
            }
        }

        // 4. Auto-pairing enclosures: '(', '[', '{', '"', '`'
        let mut auto_pair_char = None;
        for e in &i.events {
            if let egui::Event::Text(s) = e {
                if s.chars().count() == 1 {
                    let ch = s.chars().next().unwrap();
                    if matches!(ch, '(' | '[' | '{' | '"' | '`') {
                        auto_pair_char = Some(ch);
                        break;
                    }
                }
            }
        }
        if let Some(ch) = auto_pair_char {
            if let Some((new_text, new_cursor_byte)) = apply_auto_pairing(&state.editor_content, byte_start, ch, &selected_text) {
                state.editor_content = new_text;
                let new_char = byte_to_char_idx(&state.editor_content, new_cursor_byte);
                set_editor_cursor(ui, editor_id, new_char);
                state.on_editor_modified();
                i.events.retain(|e| match e {
                    egui::Event::Text(s) if s.chars().count() == 1 && s.chars().next() == Some(ch) => false,
                    _ => true,
                });
            }
        }
    });
}
```

3. **Wiring inside `render_editor`**:
```rust
        // Active note editing area
        let line_count = state.editor_content.lines().count().max(1);

        let editor_scroll_id = "editor_scroll_area";
        let mut scroll_area = egui::ScrollArea::vertical()
            .id_salt(editor_scroll_id)
            .auto_shrink([false, false]);

        // Actuate scroll sync if Preview is driver
        if state.view_mode == ViewMode::Split && state.scroll_sync.driver == ScrollDriver::Preview {
            if let Some(target) = state.scroll_sync.target_editor_scroll.take() {
                scroll_area = scroll_area.vertical_scroll_offset(target);
            }
        }

        let editor_widget_salt = "airgap_editor_text_edit";
        let editor_id = ui.make_persistent_id(editor_widget_salt);

        // Pre-process editor keystrokes
        handle_editor_keystroke_conveniences(ui, state, editor_id);

        let scroll_output = scroll_area.show(ui, |ui| {
            let pointer_in_editor = ui.rect_contains_pointer(ui.max_rect());

            ui.horizontal_top(|ui| {
                // Line numbers gutter...
                ...
                let text_edit = egui::TextEdit::multiline(&mut state.editor_content)
                    .id_salt(editor_widget_salt)
                    .font(TextStyle::Monospace)
                    .lock_focus(true)
                    .layouter(&mut layouter)
                    .desired_width(f32::INFINITY);

                let output = text_edit.show(ui);

                // Cursor range tracking & line highlight
                if let Some(cursor_range) = output.cursor_range {
                    let primary = cursor_range.primary;
                    state.cursor_line = primary.rcursor.row + 1;
                    state.cursor_col = primary.rcursor.column + 1;
                    state.scroll_sync.editor_line = state.cursor_line;
                    state.editor_char_range = Some(cursor_range.as_sorted_char_range());

                    let line_y = output.response.rect.min.y + (primary.rcursor.row as f32 * 18.0) + 2.0;
                    let line_rect = Rect::from_min_size(
                        egui::pos2(output.response.rect.min.x, line_y),
                        egui::vec2(output.response.rect.width(), 18.0),
                    );
                    let highlight_bg = if is_dark {
                        Color32::from_rgba_unmultiplied(255, 255, 255, 8)
                    } else {
                        Color32::from_rgba_unmultiplied(0, 0, 0, 8)
                    };
                    ui.painter().rect_filled(line_rect, 0.0, highlight_bg);
                }

                if output.response.changed() {
                    state.on_editor_modified();
                }

                handle_editor_keystrokes(ui, state);
            });
        });

        // Update editor scroll metrics
        state.scroll_sync.editor_content_height = scroll_output.content_size.y;
        state.scroll_sync.editor_viewport_height = scroll_output.inner_rect.height();

        if state.view_mode == ViewMode::Split {
            if ui.rect_contains_pointer(scroll_output.inner_rect) || ui.memory(|m| m.has_focus(editor_id)) {
                state.scroll_sync.driver = ScrollDriver::Editor;
            }

            if state.scroll_sync.driver == ScrollDriver::Editor {
                let max_editor_scroll = (scroll_output.content_size.y - scroll_output.inner_rect.height()).max(1.0);
                let ratio = (scroll_output.state.offset.y / max_editor_scroll).clamp(0.0, 1.0);
                let max_preview_scroll = (state.scroll_sync.preview_content_height - state.scroll_sync.preview_viewport_height).max(0.0);
                state.scroll_sync.target_preview_scroll = Some(ratio * max_preview_scroll);
            }
        }
```

---

### 4.3 Changes to `airgap/crates/app/src/ui/preview.rs`

1. **Actuate Scroll Sync in `render_preview`**:
```rust
        let preview_scroll_id = "preview_scroll_area";
        let mut scroll_area = egui::ScrollArea::vertical()
            .id_salt(preview_scroll_id)
            .auto_shrink([false, false]);

        // Actuate scroll sync if Editor is driver
        if state.view_mode == ViewMode::Split && state.scroll_sync.driver == ScrollDriver::Editor {
            if let Some(target) = state.scroll_sync.target_preview_scroll.take() {
                scroll_area = scroll_area.vertical_scroll_offset(target);
            }
        }

        let scroll_output = scroll_area.show(ui, |ui| {
            ...
        });

        // Track preview scroll metrics
        state.scroll_sync.preview_scroll_y = scroll_output.state.offset.y;
        state.scroll_sync.preview_content_height = scroll_output.content_size.y;
        state.scroll_sync.preview_viewport_height = scroll_output.inner_rect.height();

        if state.view_mode == ViewMode::Split {
            if ui.rect_contains_pointer(scroll_output.inner_rect) {
                state.scroll_sync.driver = ScrollDriver::Preview;
            }

            if state.scroll_sync.driver == ScrollDriver::Preview {
                let max_preview_scroll = (scroll_output.content_size.y - scroll_output.inner_rect.height()).max(1.0);
                let ratio = (scroll_output.state.offset.y / max_preview_scroll).clamp(0.0, 1.0);
                let max_editor_scroll = (state.scroll_sync.editor_content_height - state.scroll_sync.editor_viewport_height).max(0.0);
                state.scroll_sync.target_editor_scroll = Some(ratio * max_editor_scroll);
            }
        }
```

2. **Preserve Line Endings in `toggle_checkbox_at_line` (Fixing Finding 3)**:
```rust
fn toggle_checkbox_at_line(content: &mut String, target_line_1_indexed: usize) {
    let newline = if content.contains("\r\n") { "\r\n" } else { "\n" };
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
    *content = new_lines.join(newline);
}
```

---

## 5. Verification Method

### 5.1 Test Execution Instructions for Worker

The Worker can execute the following verification steps:

1. **Workspace Unit & Adversarial Tests**:
   ```bash
   cargo test --manifest-path airgap/Cargo.toml --workspace
   ```
2. **Clippy Static Analysis**:
   ```bash
   cargo clippy --manifest-path airgap/Cargo.toml --workspace --all-targets -- -D warnings
   ```
3. **Format Check**:
   ```bash
   cargo fmt --manifest-path airgap/Cargo.toml --all -- --check
   ```
4. **Interactive GUI Event Integration Tests (`test_m2_integration.rs`)**:
   Add genuine end-to-end GUI event tests using `egui::Context::default()`:
   - **Test 1**: Verify `Key::Enter` inside `render_editor` on `- [ ] Task 1` extends to `- [ ] Task 1\n- [ ] `.
   - **Test 2**: Verify `Key::Tab` inside `render_editor` indents with 2 spaces (`  `).
   - **Test 3**: Verify `Event::Text("[")` with selection `"world"` produces `"[world]"`.
   - **Test 4**: Verify `Event::Key(Key::Enter)` on empty bullet `- [ ] ` erases the bullet.
   - **Test 5**: Verify `render_preview` and `render_editor` scroll sync: setting `state.scroll_sync.driver = ScrollDriver::Editor` with non-zero offset generates `target_preview_scroll` and Preview applies it on next frame.
   - **Test 6**: Verify `toggle_checkbox_at_line` preserves CRLF on `Line 1\r\n- [ ] Task\r\nLine 3`.
