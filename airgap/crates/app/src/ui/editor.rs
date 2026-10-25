use crate::clipboard::ClipboardHandler;
use crate::state::{AppState, SaveStatus, ScrollDriver, ViewMode};
use crate::ui::editor_highlighter::EditorHighlighter;
use egui::text::LayoutJob;
use egui::{Color32, Key, Rect, RichText, Stroke, TextStyle, Ui};
use std::sync::Mutex;

static EDITOR_HIGHLIGHTER: Mutex<Option<EditorHighlighter>> = Mutex::new(None);

pub fn render_editor(ui: &mut Ui, state: &mut AppState) {
    let is_dark = state.theme_mode == crate::theme::ThemeMode::Dark;

    ui.vertical(|ui| {
        // Editor Toolbar Header
        ui.horizontal(|ui| {
            if let Some(note_path) = &state.active_note_path {
                ui.label(
                    RichText::new(note_path.to_string_lossy())
                        .strong()
                        .size(15.0),
                );

                ui.add_space(8.0);

                // Save status badge
                match &state.save_status {
                    SaveStatus::Saved => {
                        ui.label(
                            RichText::new("● Saved")
                                .size(12.0)
                                .color(Color32::from_rgb(34, 197, 94)),
                        );
                    }
                    SaveStatus::Unsaved => {
                        ui.label(
                            RichText::new("○ Unsaved")
                                .size(12.0)
                                .color(Color32::from_rgb(245, 158, 11)),
                        );
                    }
                    SaveStatus::Saving => {
                        ui.label(
                            RichText::new("⏳ Saving...")
                                .size(12.0)
                                .color(Color32::from_rgb(96, 165, 250)),
                        );
                    }
                    SaveStatus::Error(err) => {
                        ui.label(
                            RichText::new(format!("⚠️ {}", err))
                                .size(12.0)
                                .color(Color32::from_rgb(239, 68, 68)),
                        );
                    }
                }
            } else {
                ui.label(
                    RichText::new("No Note Selected")
                        .size(14.0)
                        .color(Color32::from_rgb(161, 161, 170)),
                );
            }

            // Right-aligned toolbar toggles
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // View Mode cycle button
                let mode_icon = match state.view_mode {
                    ViewMode::Split => "🗖 Split (Ctrl+\\)",
                    ViewMode::EditorOnly => "📝 Edit (Ctrl+\\)",
                    ViewMode::PreviewOnly => "👁 View (Ctrl+\\)",
                };
                if ui
                    .button(mode_icon)
                    .on_hover_text("Cycle view mode: Split -> Editor Only -> Preview Only")
                    .clicked()
                {
                    state.view_mode = state.view_mode.cycle();
                }

                // Find & Replace toggle button
                let find_btn = if state.find_replace.is_open {
                    "🔍 Find [On]"
                } else {
                    "🔍 Find"
                };
                if ui
                    .button(find_btn)
                    .on_hover_text("Open Find & Replace (Ctrl+F / Ctrl+H)")
                    .clicked()
                {
                    state.find_replace.is_open = !state.find_replace.is_open;
                    if state.find_replace.is_open {
                        state
                            .find_replace
                            .recalculate_matches(&state.editor_content);
                    }
                }

                let wrap_text = if state.word_wrap_enabled {
                    "↵ Wrap [On]"
                } else {
                    "↵ Wrap [Off]"
                };
                if ui
                    .button(wrap_text)
                    .on_hover_text("Toggle soft word wrap")
                    .clicked()
                {
                    state.word_wrap_enabled = !state.word_wrap_enabled;
                }

                let num_text = if state.line_numbers_enabled {
                    "# Lines [On]"
                } else {
                    "# Lines [Off]"
                };
                if ui
                    .button(num_text)
                    .on_hover_text("Toggle line numbers gutter")
                    .clicked()
                {
                    state.line_numbers_enabled = !state.line_numbers_enabled;
                }
            });
        });

        // Find & Replace Bar (when open)
        if state.find_replace.is_open {
            ui.add_space(2.0);
            render_find_replace_bar(ui, state, is_dark);
        }

        ui.add_space(4.0);
        ui.separator();
        ui.add_space(4.0);

        // Editor Body
        if state.active_note_path.is_none() {
            ui.vertical_centered(|ui| {
                ui.add_space(80.0);
                ui.label(
                    RichText::new("No Note Selected")
                        .size(18.0)
                        .strong()
                        .color(Color32::from_rgb(161, 161, 170)),
                );
                ui.add_space(8.0);
                ui.label("Select a note from the sidebar or click '+ Note' to begin.");
                ui.add_space(16.0);
                if ui.button("📄 Create New Note (Ctrl+N)").clicked() {
                    state.create_new_note(None);
                }
            });
            return;
        }

        // Active note editing area
        let line_count = state.editor_content.lines().count().max(1);

        let editor_scroll_id = "editor_scroll_area";
        let mut scroll_area = egui::ScrollArea::vertical()
            .id_salt(editor_scroll_id)
            .auto_shrink([false, false]);

        let is_split = state.view_mode == ViewMode::Split;
        if is_split && state.scroll_sync.driver == ScrollDriver::Preview {
            if let Some(target) = state.scroll_sync.target_editor_scroll.take() {
                scroll_area = scroll_area.vertical_scroll_offset(target);
            }
        }

        let editor_widget_salt = "airgap_editor_text_edit";
        let editor_id = ui.make_persistent_id(editor_widget_salt);

        if state.active_note_path.is_some()
            && !state.find_replace.is_open
            && ui.memory(|m| m.focused().is_none())
        {
            ui.memory_mut(|m| m.request_focus(editor_id));
        }

        // Pre-process editor keystroke conveniences (List continuation, Indent, Auto-pairing, Smart URL/Image Paste)
        handle_editor_keystroke_conveniences(ui, state, editor_id);

        let scroll_output = scroll_area.show(ui, |ui| {
            ui.horizontal_top(|ui| {
                // Line numbers gutter
                if state.line_numbers_enabled {
                    let mut gutter_job = LayoutJob::default();
                    let mono_font = egui::FontId::new(14.0, egui::FontFamily::Monospace);
                    let muted_color = Color32::from_rgb(113, 113, 122);
                    let active_color = if is_dark {
                        Color32::from_rgb(250, 250, 250)
                    } else {
                        Color32::from_rgb(24, 24, 27)
                    };

                    for i in 1..=line_count {
                        let is_cursor_line = i == state.cursor_line;
                        let line_str = format!("{:>4}\n", i);
                        gutter_job.append(
                            &line_str,
                            0.0,
                            egui::text::TextFormat {
                                font_id: mono_font.clone(),
                                color: if is_cursor_line {
                                    active_color
                                } else {
                                    muted_color
                                },
                                ..Default::default()
                            },
                        );
                    }

                    ui.add(egui::Label::new(gutter_job).selectable(false));
                    ui.separator();
                }

                // Mutex lock for editor layouter
                let mut highlighter_guard = EDITOR_HIGHLIGHTER.lock().unwrap();
                if highlighter_guard.is_none() {
                    *highlighter_guard = Some(EditorHighlighter::default());
                }
                let highlighter = highlighter_guard.as_mut().unwrap();

                let find_matches = state.find_replace.matches.clone();
                let active_match = state.find_replace.active_match;

                let mut layouter = |ui: &Ui, text: &str, wrap_width: f32| {
                    let opts = crate::ui::editor_highlighter::LayoutOptions {
                        wrap_width,
                        is_dark,
                        font_size: 14.0,
                        find_matches: &find_matches,
                        active_match,
                    };
                    highlighter.layout(ui, text, &opts)
                };

                let text_edit = egui::TextEdit::multiline(&mut state.editor_content)
                    .id_salt(editor_widget_salt)
                    .font(TextStyle::Monospace)
                    .lock_focus(true)
                    .layouter(&mut layouter)
                    .desired_width(f32::INFINITY);

                let output = text_edit.show(ui);

                // Active cursor line highlight background rect
                if let Some(cursor_range) = output.cursor_range {
                    let primary = cursor_range.primary;
                    state.cursor_line = primary.rcursor.row + 1;
                    state.cursor_col = primary.rcursor.column + 1;
                    state.scroll_sync.editor_line = state.cursor_line;
                    let min_idx = cursor_range
                        .primary
                        .ccursor
                        .index
                        .min(cursor_range.secondary.ccursor.index);
                    let max_idx = cursor_range
                        .primary
                        .ccursor
                        .index
                        .max(cursor_range.secondary.ccursor.index);
                    state.editor_char_range = Some(min_idx..max_idx);

                    let line_y =
                        output.response.rect.min.y + (primary.rcursor.row as f32 * 18.0) + 2.0;
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

                // Keyboard shortcuts & typing conveniences
                handle_editor_keystrokes(ui, state);
            });
        });

        // Update editor scroll metrics for scroll sync
        state.scroll_sync.editor_content_height = scroll_output.content_size.y;
        state.scroll_sync.editor_viewport_height = scroll_output.inner_rect.height();

        if is_split {
            if ui.rect_contains_pointer(scroll_output.inner_rect)
                || ui.memory(|m| m.has_focus(editor_id))
            {
                state.scroll_sync.driver = ScrollDriver::Editor;
            }

            if state.scroll_sync.driver == ScrollDriver::Editor {
                let max_editor_scroll =
                    (scroll_output.content_size.y - scroll_output.inner_rect.height()).max(1.0);
                let ratio = (scroll_output.state.offset.y / max_editor_scroll).clamp(0.0, 1.0);
                let max_preview_scroll = (state.scroll_sync.preview_content_height
                    - state.scroll_sync.preview_viewport_height)
                    .max(0.0);
                state.scroll_sync.target_preview_scroll = Some(ratio * max_preview_scroll);
            }
        }
    });
}

fn render_find_replace_bar(ui: &mut Ui, state: &mut AppState, is_dark: bool) {
    let bg_color = if is_dark {
        Color32::from_rgb(24, 24, 27)
    } else {
        Color32::from_rgb(244, 244, 245)
    };
    let border_color = if is_dark {
        Color32::from_rgb(39, 39, 42)
    } else {
        Color32::from_rgb(228, 228, 231)
    };

    egui::Frame::NONE
        .fill(bg_color)
        .stroke(Stroke::new(1.0_f32, border_color))
        .corner_radius(egui::CornerRadius::same(6))
        .inner_margin(egui::Margin::symmetric(10, 6))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Find:").size(12.0).strong());

                let find_edit = ui.add(
                    egui::TextEdit::singleline(&mut state.find_replace.find_query)
                        .hint_text("Search note...")
                        .desired_width(180.0),
                );

                if find_edit.changed() {
                    state
                        .find_replace
                        .recalculate_matches(&state.editor_content);
                }

                // Match counter badge
                let match_count = state.find_replace.matches.len();
                let match_label = if state.find_replace.find_query.is_empty() {
                    String::new()
                } else if match_count == 0 {
                    "No matches".to_string()
                } else {
                    format!("{}/{}", state.find_replace.active_match + 1, match_count)
                };
                ui.label(
                    RichText::new(match_label)
                        .size(11.0)
                        .color(Color32::from_rgb(113, 113, 122)),
                );

                // Prev / Next match buttons
                if ui
                    .button("↑")
                    .on_hover_text("Previous match (Shift+Enter)")
                    .clicked()
                {
                    state.find_replace.prev_match();
                }
                if ui.button("↓").on_hover_text("Next match (Enter)").clicked() {
                    state.find_replace.next_match();
                }

                // Toggles
                if ui
                    .selectable_label(state.find_replace.case_sensitive, "Aa")
                    .on_hover_text("Match Case")
                    .clicked()
                {
                    state.find_replace.case_sensitive = !state.find_replace.case_sensitive;
                    state
                        .find_replace
                        .recalculate_matches(&state.editor_content);
                }
                if ui
                    .selectable_label(state.find_replace.whole_word, "\\b")
                    .on_hover_text("Match Whole Word")
                    .clicked()
                {
                    state.find_replace.whole_word = !state.find_replace.whole_word;
                    state
                        .find_replace
                        .recalculate_matches(&state.editor_content);
                }
                if ui
                    .selectable_label(state.find_replace.use_regex, ".*")
                    .on_hover_text("Use Regular Expression")
                    .clicked()
                {
                    state.find_replace.use_regex = !state.find_replace.use_regex;
                    state
                        .find_replace
                        .recalculate_matches(&state.editor_content);
                }

                // Toggle Replace Row
                let replace_text = if state.find_replace.show_replace {
                    "▾ Replace"
                } else {
                    "▸ Replace"
                };
                if ui.button(replace_text).clicked() {
                    state.find_replace.show_replace = !state.find_replace.show_replace;
                }

                // Close Button
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("✕").on_hover_text("Close (Escape)").clicked() {
                        state.find_replace.is_open = false;
                    }
                });
            });

            // Replace Row
            if state.find_replace.show_replace {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Replace:").size(12.0).strong());

                    ui.add(
                        egui::TextEdit::singleline(&mut state.find_replace.replace_query)
                            .hint_text("Replacement text...")
                            .desired_width(180.0),
                    );

                    let (line, col) = (state.cursor_line, state.cursor_col);
                    if ui.button("Replace").clicked() {
                        state.find_replace.replace_current(
                            &mut state.editor_content,
                            &mut state.undo_manager,
                            line,
                            col,
                        );
                        state.on_editor_modified();
                    }

                    if ui.button("Replace All").clicked() {
                        state.find_replace.replace_all(
                            &mut state.editor_content,
                            &mut state.undo_manager,
                            line,
                            col,
                        );
                        state.on_editor_modified();
                    }
                });
            }

            if let Some(err) = &state.find_replace.regex_error {
                ui.add_space(2.0);
                ui.label(
                    RichText::new(format!("Regex error: {}", err))
                        .size(11.0)
                        .color(Color32::from_rgb(239, 68, 68)),
                );
            }
        });
}

fn set_editor_cursor(ctx: &egui::Context, editor_id: egui::Id, char_idx: usize) {
    let ccursor = egui::text::CCursor::new(char_idx);
    let range = egui::text_selection::CCursorRange::one(ccursor);
    let mut tes = egui::text_edit::TextEditState::load(ctx, editor_id).unwrap_or_default();
    tes.cursor.set_char_range(Some(range));
    tes.store(ctx, editor_id);
}

fn handle_editor_keystroke_conveniences(ui: &mut Ui, state: &mut AppState, editor_id: egui::Id) {
    let is_focused = ui.memory(|m| m.has_focus(editor_id))
        || (state.active_note_path.is_some()
            && !state.find_replace.is_open
            && ui.memory(|m| m.focused().is_none()));
    if !is_focused {
        return;
    }

    let (char_start, char_end) = if let Some(range) = &state.editor_char_range {
        (range.start, range.end)
    } else if let Some(tes) = egui::text_edit::TextEditState::load(ui.ctx(), editor_id) {
        if let Some(cr) = tes.cursor.char_range() {
            let min_idx = cr.primary.index.min(cr.secondary.index);
            let max_idx = cr.primary.index.max(cr.secondary.index);
            (min_idx, max_idx)
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

    enum EditAction {
        ReplaceRange {
            byte_start: usize,
            byte_end: usize,
            text: String,
        },
        SetText {
            text: String,
            new_char: usize,
        },
    }

    let mut action: Option<EditAction> = None;

    ui.input_mut(|i| {
        let ctrl = i.modifiers.command || i.modifiers.ctrl;

        // 1. Ctrl+V: Image Paste at cursor or Smart URL Paste on selection
        let paste_pressed = (ctrl
            && i.events.iter().any(|e| {
                matches!(
                    e,
                    egui::Event::Key {
                        key: Key::V,
                        pressed: true,
                        ..
                    }
                )
            }))
            || i.events.iter().any(|e| matches!(e, egui::Event::Paste(_)));

        if paste_pressed {
            // Check image paste first
            if let (Some(vault), Some(active_note)) = (&state.vault, &state.active_note_path) {
                if let Some(image_md) = ClipboardHandler::try_paste_image(&vault.root, active_note)
                {
                    action = Some(EditAction::ReplaceRange {
                        byte_start,
                        byte_end,
                        text: image_md,
                    });
                    i.events.retain(|e| {
                        !matches!(
                            e,
                            egui::Event::Paste(_)
                                | egui::Event::Key {
                                    key: Key::V,
                                    pressed: true,
                                    ..
                                }
                        )
                    });
                    return;
                }
            }

            // Check smart URL paste on selection
            if !selected_text.is_empty() {
                let clip_opt = i
                    .events
                    .iter()
                    .find_map(|e| {
                        if let egui::Event::Paste(s) = e {
                            Some(s.clone())
                        } else {
                            None
                        }
                    })
                    .or_else(|| {
                        arboard::Clipboard::new()
                            .ok()
                            .and_then(|mut c| c.get_text().ok())
                    });

                if let Some(clip_str) = clip_opt {
                    if let Some(smart_link) =
                        ClipboardHandler::try_paste_smart_url(&clip_str, &selected_text)
                    {
                        action = Some(EditAction::ReplaceRange {
                            byte_start,
                            byte_end,
                            text: smart_link,
                        });
                        i.events.retain(|e| {
                            !matches!(
                                e,
                                egui::Event::Paste(_)
                                    | egui::Event::Key {
                                        key: Key::V,
                                        pressed: true,
                                        ..
                                    }
                            )
                        });
                        return;
                    }
                }
            }
        }

        // 2. Tab & Shift+Tab indentation
        let mut tab_action = None;
        for e in &i.events {
            if let egui::Event::Key {
                key: Key::Tab,
                pressed: true,
                modifiers,
                ..
            } = e
            {
                tab_action = Some(modifiers.shift);
                break;
            }
        }
        if let Some(is_outdent) = tab_action {
            let (new_text, new_cursor_byte) =
                apply_indentation(&state.editor_content, byte_start, is_outdent);
            let new_char = byte_to_char_idx(&new_text, new_cursor_byte);
            action = Some(EditAction::SetText {
                text: new_text,
                new_char,
            });
            i.events.retain(|e| {
                !matches!(
                    e,
                    egui::Event::Key {
                        key: Key::Tab,
                        pressed: true,
                        ..
                    }
                )
            });
            return;
        }

        // 3. Enter key: List/Task/Quote continuation
        let enter_pressed = i.events.iter().any(|e| {
            matches!(e, egui::Event::Key { key: Key::Enter, pressed: true, modifiers, .. }
                if !modifiers.shift && !modifiers.ctrl && !modifiers.command && !modifiers.alt)
        });
        if enter_pressed {
            if let Some((new_text, new_cursor_byte)) =
                apply_list_continuation(&state.editor_content, byte_start)
            {
                let new_char = byte_to_char_idx(&new_text, new_cursor_byte);
                action = Some(EditAction::SetText {
                    text: new_text,
                    new_char,
                });
                i.events.retain(|e| {
                    !matches!(
                        e,
                        egui::Event::Key {
                            key: Key::Enter,
                            pressed: true,
                            ..
                        }
                    )
                });
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
            if let Some((new_text, new_cursor_byte)) =
                apply_auto_pairing(&state.editor_content, byte_start, ch, &selected_text)
            {
                let new_char = byte_to_char_idx(&new_text, new_cursor_byte);
                action = Some(EditAction::SetText {
                    text: new_text,
                    new_char,
                });
                let ch_str = ch.to_string();
                i.events
                    .retain(|e| !matches!(e, egui::Event::Text(s) if s == &ch_str));
            }
        }
    });

    if let Some(act) = action {
        match act {
            EditAction::ReplaceRange {
                byte_start,
                byte_end,
                text,
                ..
            } => {
                state
                    .editor_content
                    .replace_range(byte_start..byte_end, &text);
                let new_byte = byte_start + text.len();
                let new_char = byte_to_char_idx(&state.editor_content, new_byte);
                state.editor_char_range = Some(new_char..new_char);
                set_editor_cursor(ui.ctx(), editor_id, new_char);
            }
            EditAction::SetText { text, new_char } => {
                state.editor_content = text;
                state.editor_char_range = Some(new_char..new_char);
                set_editor_cursor(ui.ctx(), editor_id, new_char);
            }
        }
        state.on_editor_modified();
    }
}

fn handle_editor_keystrokes(ui: &mut Ui, state: &mut AppState) {
    ui.input(|i| {
        let ctrl = i.modifiers.command || i.modifiers.ctrl;
        let shift = i.modifiers.shift;

        // Escape: close find & replace bar
        if i.key_pressed(Key::Escape) && state.find_replace.is_open {
            state.find_replace.is_open = false;
        }

        // Ctrl+Z: Undo
        if ctrl && !shift && i.key_pressed(Key::Z) {
            let (cur_line, cur_col) = (state.cursor_line, state.cursor_col);
            if let Some(snap) = state
                .undo_manager
                .undo(&state.editor_content, cur_line, cur_col)
            {
                state.editor_content = snap.text;
                state.cursor_line = snap.cursor_line;
                state.cursor_col = snap.cursor_col;
                state.on_editor_modified();
            }
        }

        // Ctrl+Y or Ctrl+Shift+Z: Redo
        if (ctrl && i.key_pressed(Key::Y)) || (ctrl && shift && i.key_pressed(Key::Z)) {
            let (cur_line, cur_col) = (state.cursor_line, state.cursor_col);
            if let Some(snap) = state
                .undo_manager
                .redo(&state.editor_content, cur_line, cur_col)
            {
                state.editor_content = snap.text;
                state.cursor_line = snap.cursor_line;
                state.cursor_col = snap.cursor_col;
                state.on_editor_modified();
            }
        }
    });
}

/// Applies CommonMark list continuation on Enter.
/// Returns Some((new_content, new_cursor_pos)) if matched, None otherwise.
pub fn apply_list_continuation(text: &str, cursor_char_idx: usize) -> Option<(String, usize)> {
    let safe_cursor = floor_char_boundary(text, cursor_char_idx.min(text.len()));
    let (line_start, line_text) = find_current_line(text, safe_cursor);

    // 1. Task list item: "- [ ] " or "- [x] "
    if let Some(rest) = line_text.strip_prefix("- [ ] ") {
        if rest.trim().is_empty() {
            // Empty item: erase prefix
            let new_text = format!("{}{}", &text[..line_start], &text[safe_cursor..]);
            return Some((new_text, line_start));
        } else {
            let prefix = "\n- [ ] ";
            let new_text = format!("{}{}{}", &text[..safe_cursor], prefix, &text[safe_cursor..]);
            return Some((new_text, safe_cursor + prefix.len()));
        }
    }

    if let Some(rest) = line_text.strip_prefix("- [x] ") {
        if rest.trim().is_empty() {
            let new_text = format!("{}{}", &text[..line_start], &text[safe_cursor..]);
            return Some((new_text, line_start));
        } else {
            let prefix = "\n- [ ] ";
            let new_text = format!("{}{}{}", &text[..safe_cursor], prefix, &text[safe_cursor..]);
            return Some((new_text, safe_cursor + prefix.len()));
        }
    }

    // 2. Unordered bullet list: "- ", "* ", "+ "
    for bullet in &["- ", "* ", "+ "] {
        if let Some(rest) = line_text.strip_prefix(bullet) {
            if rest.trim().is_empty() {
                let new_text = format!("{}{}", &text[..line_start], &text[safe_cursor..]);
                return Some((new_text, line_start));
            } else {
                let prefix = format!("\n{}", bullet);
                let new_text =
                    format!("{}{}{}", &text[..safe_cursor], prefix, &text[safe_cursor..]);
                return Some((new_text, safe_cursor + prefix.len()));
            }
        }
    }

    // 3. Numbered list: "1. ", "2. ", etc.
    if let Some(dot_idx) = line_text.find(". ") {
        let num_str = &line_text[..dot_idx];
        if let Ok(num) = num_str.parse::<u64>() {
            let rest = &line_text[dot_idx + 2..];
            if rest.trim().is_empty() {
                let new_text = format!("{}{}", &text[..line_start], &text[safe_cursor..]);
                return Some((new_text, line_start));
            } else {
                let prefix = format!("\n{}. ", num + 1);
                let new_text =
                    format!("{}{}{}", &text[..safe_cursor], prefix, &text[safe_cursor..]);
                return Some((new_text, safe_cursor + prefix.len()));
            }
        }
    }

    // 4. Blockquote: "> "
    if let Some(rest) = line_text.strip_prefix("> ") {
        if rest.trim().is_empty() {
            let new_text = format!("{}{}", &text[..line_start], &text[safe_cursor..]);
            return Some((new_text, line_start));
        } else {
            let prefix = "\n> ";
            let new_text = format!("{}{}{}", &text[..safe_cursor], prefix, &text[safe_cursor..]);
            return Some((new_text, safe_cursor + prefix.len()));
        }
    }

    None
}

/// Applies 2-space indentation or outdentation.
pub fn apply_indentation(text: &str, cursor_char_idx: usize, is_outdent: bool) -> (String, usize) {
    let safe_cursor = floor_char_boundary(text, cursor_char_idx.min(text.len()));
    let (line_start, line_text) = find_current_line(text, safe_cursor);

    if is_outdent {
        if let Some(stripped) = line_text.strip_prefix("  ") {
            let new_text = format!(
                "{}{}{}",
                &text[..line_start],
                stripped,
                &text[line_start + line_text.len()..]
            );
            let new_cursor = safe_cursor.saturating_sub(2).max(line_start);
            (new_text, new_cursor)
        } else if let Some(stripped) = line_text.strip_prefix(' ') {
            let new_text = format!(
                "{}{}{}",
                &text[..line_start],
                stripped,
                &text[line_start + line_text.len()..]
            );
            let new_cursor = safe_cursor.saturating_sub(1).max(line_start);
            (new_text, new_cursor)
        } else {
            (text.to_string(), safe_cursor)
        }
    } else {
        let new_text = format!(
            "{}  {}{}",
            &text[..line_start],
            line_text,
            &text[line_start + line_text.len()..]
        );
        (new_text, safe_cursor + 2)
    }
}

/// Applies auto-pairing for enclosures: (), [], {}, "", ``.
pub fn apply_auto_pairing(
    text: &str,
    cursor_char_idx: usize,
    open_char: char,
    selected: &str,
) -> Option<(String, usize)> {
    let safe_cursor = floor_char_boundary(text, cursor_char_idx.min(text.len()));
    let close_char = match open_char {
        '(' => ')',
        '[' => ']',
        '{' => '}',
        '"' => '"',
        '`' => '`',
        _ => return None,
    };

    if !selected.is_empty() {
        let wrapped = format!("{}{}{}", open_char, selected, close_char);
        let sel_end = floor_char_boundary(text, (safe_cursor + selected.len()).min(text.len()));
        let new_text = format!("{}{}{}", &text[..safe_cursor], wrapped, &text[sel_end..]);
        return Some((new_text, safe_cursor + wrapped.len()));
    }

    // Insert pair and center cursor
    let pair = format!("{}{}", open_char, close_char);
    let new_text = format!("{}{}{}", &text[..safe_cursor], pair, &text[safe_cursor..]);
    Some((new_text, safe_cursor + 1))
}

pub fn floor_char_boundary(s: &str, mut index: usize) -> usize {
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
    let safe_byte = floor_char_boundary(text, byte_idx.min(text.len()));
    text[..safe_byte].chars().count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_continuation_task() {
        let text = "- [ ] Buy milk";
        let (new_text, cursor) = apply_list_continuation(text, text.len()).unwrap();
        assert_eq!(new_text, "- [ ] Buy milk\n- [ ] ");
        assert_eq!(cursor, 21);
    }

    #[test]
    fn test_list_continuation_empty_task_exits() {
        let text = "- [ ] ";
        let (new_text, cursor) = apply_list_continuation(text, text.len()).unwrap();
        assert_eq!(new_text, "");
        assert_eq!(cursor, 0);
    }

    #[test]
    fn test_list_continuation_numbered() {
        let text = "1. First item";
        let (new_text, cursor) = apply_list_continuation(text, text.len()).unwrap();
        assert_eq!(new_text, "1. First item\n2. ");
        assert_eq!(cursor, 17);
    }

    #[test]
    fn test_list_continuation_bullet() {
        let text = "- Item A";
        let (new_text, cursor) = apply_list_continuation(text, text.len()).unwrap();
        assert_eq!(new_text, "- Item A\n- ");
        assert_eq!(cursor, 11);
    }

    #[test]
    fn test_auto_pairing_wrap() {
        let text = "Hello world";
        let (new_text, cursor) = apply_auto_pairing(text, 6, '[', "world").unwrap();
        assert_eq!(new_text, "Hello [world]");
        assert_eq!(cursor, 13);
    }

    #[test]
    fn test_auto_pairing_center_cursor() {
        let text = "foo";
        let (new_text, cursor) = apply_auto_pairing(text, 3, '(', "").unwrap();
        assert_eq!(new_text, "foo()");
        assert_eq!(cursor, 4);
    }

    #[test]
    fn test_indent_and_outdent() {
        let text = "Hello";
        let (indented, c1) = apply_indentation(text, 0, false);
        assert_eq!(indented, "  Hello");
        assert_eq!(c1, 2);

        let (outdented, c2) = apply_indentation(&indented, 2, true);
        assert_eq!(outdented, "Hello");
        assert_eq!(c2, 0);
    }
}
