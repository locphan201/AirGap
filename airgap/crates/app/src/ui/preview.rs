use crate::state::{AppState, ScrollDriver};
use crate::ui::editor_highlighter::fast_hash_str;
use crate::url_safety::open_external_url_safely;
use airgap_render::{
    parse_markdown, resolve_local_image_path, Block, CalloutKind, ColumnAlignment, InlineSpan,
    ListItem, SyntaxHighlighter,
};
use egui::text::{LayoutJob, TextFormat};
use egui::{Align, Color32, CornerRadius, FontId, Frame, Margin, RichText, Stroke, Ui};
use std::path::Path;
use std::sync::OnceLock;
use std::time::Instant;

static SYNTAX_HIGHLIGHTER: OnceLock<SyntaxHighlighter> = OnceLock::new();

fn get_highlighter() -> &'static SyntaxHighlighter {
    SYNTAX_HIGHLIGHTER.get_or_init(SyntaxHighlighter::new)
}

pub fn render_preview(ui: &mut Ui, state: &mut AppState) {
    let is_dark = state.theme_mode == crate::theme::ThemeMode::Dark;
    let note_content = state.editor_content.clone();
    let doc = parse_markdown(&note_content);

    let mut copied_state: Option<(u64, Instant)> = None;

    ui.vertical(|ui| {
        // Preview Header Bar
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("👁 Live Preview")
                    .strong()
                    .size(14.0)
                    .color(if is_dark {
                        Color32::from_rgb(161, 161, 170)
                    } else {
                        Color32::from_rgb(113, 113, 122)
                    }),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(format!("{} lines", doc.total_lines))
                        .size(12.0)
                        .color(Color32::from_rgb(113, 113, 122)),
                );
            });
        });

        ui.add_space(4.0);
        ui.separator();
        ui.add_space(4.0);

        // Preview Scroll Body
        let is_split = state.view_mode == crate::state::ViewMode::Split;
        let mut scroll_area = egui::ScrollArea::vertical()
            .id_salt("preview_scroll_area")
            .auto_shrink([false, false]);

        if is_split && state.scroll_sync.driver == ScrollDriver::Editor {
            if let Some(target_offset) = state.scroll_sync.target_preview_scroll.take() {
                scroll_area = scroll_area.vertical_scroll_offset(target_offset);
            }
        }

        let scroll_output = scroll_area.show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 8.0;

            if doc.blocks.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(60.0);
                    ui.label(
                        RichText::new("Empty Note")
                            .size(16.0)
                            .color(Color32::from_rgb(113, 113, 122)),
                    );
                });
                return;
            }

            let mut checkbox_toggle_line: Option<usize> = None;

            for block in &doc.blocks {
                match block {
                    Block::Heading {
                        level,
                        inlines,
                        line,
                        ..
                    } => {
                        render_heading(ui, *level, inlines, is_dark);
                        let _ = line;
                    }
                    Block::Paragraph { inlines, .. } => {
                        render_paragraph(
                            ui,
                            inlines,
                            is_dark,
                            state.vault.as_ref().map(|v| v.root.as_path()),
                            state.active_note_path.as_deref(),
                        );
                    }
                    Block::BlockQuote {
                        inlines, callout, ..
                    } => {
                        render_blockquote(ui, inlines, *callout, is_dark);
                    }
                    Block::List {
                        items,
                        is_ordered,
                        start_number,
                        ..
                    } => {
                        if let Some(toggled_line) =
                            render_list(ui, items, *is_ordered, *start_number, is_dark)
                        {
                            checkbox_toggle_line = Some(toggled_line);
                        }
                    }
                    Block::Table {
                        headers,
                        rows,
                        alignments,
                        ..
                    } => {
                        render_table(ui, headers, rows, alignments, is_dark);
                    }
                    Block::CodeBlock { language, code, .. } => {
                        render_code_block(ui, language, code, is_dark, &mut copied_state);
                    }
                    Block::MermaidBlock { source, .. } => {
                        render_mermaid_block(ui, source, is_dark);
                    }
                    Block::ThematicBreak { .. } => {
                        ui.add_space(6.0);
                        ui.separator();
                        ui.add_space(6.0);
                    }
                    Block::FootnoteDefinition { label, inlines, .. } => {
                        render_footnote(ui, label, inlines, is_dark);
                    }
                }
            }

            // Handle interactive task list checkbox toggle
            if let Some(toggle_line) = checkbox_toggle_line {
                toggle_checkbox_at_line(&mut state.editor_content, toggle_line);
                state.on_editor_modified();
            }
        });

        // Track preview scroll metrics for scroll sync
        state.scroll_sync.preview_scroll_y = scroll_output.state.offset.y;
        state.scroll_sync.preview_content_height = scroll_output.content_size.y;
        state.scroll_sync.preview_viewport_height = scroll_output.inner_rect.height();

        // Pointer-based driver detection and follower sync
        if is_split && ui.rect_contains_pointer(scroll_output.inner_rect) {
            state.scroll_sync.driver = ScrollDriver::Preview;

            let max_preview = (state.scroll_sync.preview_content_height
                - state.scroll_sync.preview_viewport_height)
                .max(1.0);
            let ratio = (state.scroll_sync.preview_scroll_y / max_preview).clamp(0.0, 1.0);
            let max_editor = (state.scroll_sync.editor_content_height
                - state.scroll_sync.editor_viewport_height)
                .max(1.0);
            state.scroll_sync.target_editor_scroll = Some(ratio * max_editor);
        }
    });
}

fn render_heading(ui: &mut Ui, level: u8, inlines: &[InlineSpan], is_dark: bool) {
    let (font_size, underline) = match level {
        1 => (26.0, true),
        2 => (21.0, true),
        3 => (18.0, false),
        4 => (16.0, false),
        5 => (14.0, false),
        _ => (13.0, false),
    };

    ui.add_space(match level {
        1 => 12.0,
        2 => 10.0,
        3 => 8.0,
        _ => 6.0,
    });

    let mut job = LayoutJob::default();
    let text_color = if is_dark {
        match level {
            1 => Color32::from_rgb(96, 165, 250),
            2 => Color32::from_rgb(56, 189, 248),
            3 => Color32::from_rgb(52, 211, 153),
            _ => Color32::from_rgb(251, 191, 36),
        }
    } else {
        match level {
            1 => Color32::from_rgb(29, 78, 216),
            2 => Color32::from_rgb(2, 132, 199),
            3 => Color32::from_rgb(5, 150, 105),
            _ => Color32::from_rgb(217, 119, 6),
        }
    };

    let font_id = FontId::new(font_size, egui::FontFamily::Proportional);

    for inline in inlines {
        job.append(
            inline.plain_text(),
            0.0,
            TextFormat {
                font_id: font_id.clone(),
                color: text_color,
                ..Default::default()
            },
        );
    }

    if job.is_empty() {
        job.append(
            "(empty heading)",
            0.0,
            TextFormat {
                font_id,
                color: Color32::from_rgb(113, 113, 122),
                italics: true,
                ..Default::default()
            },
        );
    }

    ui.label(job);

    if underline {
        ui.add_space(2.0);
        let stroke_color = if is_dark {
            Color32::from_rgb(39, 39, 42)
        } else {
            Color32::from_rgb(228, 228, 231)
        };
        ui.painter().hline(
            ui.available_rect_before_wrap().x_range(),
            ui.cursor().top(),
            Stroke::new(1.0_f32, stroke_color),
        );
        ui.add_space(4.0);
    }
}

fn render_paragraph(
    ui: &mut Ui,
    inlines: &[InlineSpan],
    is_dark: bool,
    vault_root: Option<&Path>,
    note_path: Option<&Path>,
) {
    ui.horizontal_wrapped(|ui| {
        for inline in inlines {
            match inline {
                InlineSpan::Text(t) => {
                    ui.label(t);
                }
                InlineSpan::Bold(t) => {
                    ui.label(RichText::new(t).strong());
                }
                InlineSpan::Italic(t) => {
                    ui.label(RichText::new(t).italics());
                }
                InlineSpan::Strikethrough(t) => {
                    ui.label(RichText::new(t).strikethrough());
                }
                InlineSpan::Code(code) => {
                    let bg = if is_dark {
                        Color32::from_rgba_premultiplied(244, 114, 182, 35)
                    } else {
                        Color32::from_rgba_premultiplied(190, 24, 93, 25)
                    };
                    let fg = if is_dark {
                        Color32::from_rgb(244, 114, 182)
                    } else {
                        Color32::from_rgb(190, 24, 93)
                    };
                    ui.label(
                        RichText::new(code)
                            .font(FontId::monospace(13.0))
                            .color(fg)
                            .background_color(bg),
                    );
                }
                InlineSpan::StyledText { text, style } => {
                    let mut rt = RichText::new(text);
                    if style.bold {
                        rt = rt.strong();
                    }
                    if style.italic {
                        rt = rt.italics();
                    }
                    if style.strikethrough {
                        rt = rt.strikethrough();
                    }
                    ui.label(rt);
                }
                InlineSpan::Link { text, url, title } => {
                    let link_label = if text.is_empty() { url } else { text };
                    let response = ui.add(egui::Link::new(
                        RichText::new(link_label)
                            .color(if is_dark {
                                Color32::from_rgb(96, 165, 250)
                            } else {
                                Color32::from_rgb(29, 78, 216)
                            })
                            .underline(),
                    ));

                    let tooltip = match title {
                        Some(t) => format!("↗ Open in browser: {} ({})", url, t),
                        None => format!("↗ Open in browser: {}", url),
                    };
                    response.clone().on_hover_text(tooltip);

                    if response.clicked() {
                        let _ = open_external_url_safely(url);
                    }
                }
                InlineSpan::Image { alt, url, .. } => {
                    if let (Some(root), Some(note)) = (vault_root, note_path) {
                        match resolve_local_image_path(root, note, url) {
                            Ok(resolved) => {
                                let uri = format!("file://{}", resolved.display());
                                ui.add(
                                    egui::Image::new(uri)
                                        .max_width(ui.available_width().min(600.0))
                                        .corner_radius(CornerRadius::same(4)),
                                );
                            }
                            Err(err) => {
                                ui.label(
                                    RichText::new(format!("🖼 [Image: {} ({})]", alt, err))
                                        .italics()
                                        .color(Color32::from_rgb(239, 68, 68)),
                                );
                            }
                        }
                    } else {
                        ui.label(RichText::new(format!("🖼 [Image: {}]", alt)).italics());
                    }
                }
                InlineSpan::FootnoteRef(label) => {
                    ui.label(
                        RichText::new(format!("[^{}]", label))
                            .size(11.0)
                            .color(if is_dark {
                                Color32::from_rgb(96, 165, 250)
                            } else {
                                Color32::from_rgb(29, 78, 216)
                            }),
                    );
                }
                InlineSpan::SoftBreak => {
                    ui.label(" ");
                }
                InlineSpan::HardBreak => {
                    ui.end_row();
                }
            }
        }
    });
}

fn render_blockquote(
    ui: &mut Ui,
    inlines: &[InlineSpan],
    callout: Option<CalloutKind>,
    is_dark: bool,
) {
    let (border_color, bg_color, badge_text, badge_icon) = match callout {
        Some(CalloutKind::Note) => (
            Color32::from_rgb(59, 130, 246),
            if is_dark {
                Color32::from_rgba_unmultiplied(59, 130, 246, 25)
            } else {
                Color32::from_rgba_unmultiplied(59, 130, 246, 15)
            },
            "NOTE",
            "ℹ️",
        ),
        Some(CalloutKind::Tip) => (
            Color32::from_rgb(34, 197, 94),
            if is_dark {
                Color32::from_rgba_unmultiplied(34, 197, 94, 25)
            } else {
                Color32::from_rgba_unmultiplied(34, 197, 94, 15)
            },
            "TIP",
            "💡",
        ),
        Some(CalloutKind::Important) => (
            Color32::from_rgb(168, 85, 247),
            if is_dark {
                Color32::from_rgba_unmultiplied(168, 85, 247, 25)
            } else {
                Color32::from_rgba_unmultiplied(168, 85, 247, 15)
            },
            "IMPORTANT",
            "🟣",
        ),
        Some(CalloutKind::Warning) => (
            Color32::from_rgb(245, 158, 11),
            if is_dark {
                Color32::from_rgba_unmultiplied(245, 158, 11, 25)
            } else {
                Color32::from_rgba_unmultiplied(245, 158, 11, 15)
            },
            "WARNING",
            "⚠️",
        ),
        Some(CalloutKind::Caution) => (
            Color32::from_rgb(239, 68, 68),
            if is_dark {
                Color32::from_rgba_unmultiplied(239, 68, 68, 25)
            } else {
                Color32::from_rgba_unmultiplied(239, 68, 68, 15)
            },
            "CAUTION",
            "🛑",
        ),
        None => (
            if is_dark {
                Color32::from_rgb(113, 113, 122)
            } else {
                Color32::from_rgb(161, 161, 170)
            },
            if is_dark {
                Color32::from_rgb(24, 24, 27)
            } else {
                Color32::from_rgb(244, 244, 245)
            },
            "",
            "",
        ),
    };

    Frame::NONE
        .fill(bg_color)
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin::symmetric(12, 10))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                // Left border accent line
                ui.painter().vline(
                    ui.cursor().left(),
                    ui.cursor().y_range(),
                    Stroke::new(3.5_f32, border_color),
                );
                ui.add_space(8.0);

                ui.vertical(|ui| {
                    if callout.is_some() {
                        ui.horizontal(|ui| {
                            ui.label(badge_icon);
                            ui.label(
                                RichText::new(badge_text)
                                    .strong()
                                    .size(12.0)
                                    .color(border_color),
                            );
                        });
                        ui.add_space(2.0);
                    }

                    render_paragraph(ui, inlines, is_dark, None, None);
                });
            });
        });
}

fn render_list(
    ui: &mut Ui,
    items: &[ListItem],
    is_ordered: bool,
    start_number: Option<u64>,
    is_dark: bool,
) -> Option<usize> {
    let mut toggled_line: Option<usize> = None;

    let start = start_number.unwrap_or(1);
    for (idx, item) in items.iter().enumerate() {
        ui.horizontal(|ui| {
            ui.add_space(10.0);

            if let Some(mut checked) = item.task_status {
                let checkbox = ui.checkbox(&mut checked, "");
                if checkbox.changed() {
                    toggled_line = Some(item.span.start_line);
                }
            } else if is_ordered {
                ui.label(
                    RichText::new(format!("{}.", start + idx as u64))
                        .color(Color32::from_rgb(113, 113, 122))
                        .monospace(),
                );
            } else {
                ui.label(
                    RichText::new("•")
                        .color(if is_dark {
                            Color32::from_rgb(96, 165, 250)
                        } else {
                            Color32::from_rgb(29, 78, 216)
                        })
                        .size(16.0),
                );
            }

            render_paragraph(ui, &item.inlines, is_dark, None, None);
        });
    }

    toggled_line
}

fn render_table(
    ui: &mut Ui,
    headers: &[String],
    rows: &[Vec<String>],
    alignments: &[ColumnAlignment],
    is_dark: bool,
) {
    if headers.is_empty() && rows.is_empty() {
        return;
    }

    let border_stroke = Stroke::new(
        1.0_f32,
        if is_dark {
            Color32::from_rgb(39, 39, 42)
        } else {
            Color32::from_rgb(228, 228, 231)
        },
    );

    Frame::NONE
        .stroke(border_stroke)
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin::same(8))
        .show(ui, |ui| {
            egui::Grid::new(ui.id().with("preview_table"))
                .striped(true)
                .spacing([14.0, 8.0])
                .show(ui, |ui| {
                    // Header row
                    for (col_idx, header) in headers.iter().enumerate() {
                        let align = alignments
                            .get(col_idx)
                            .copied()
                            .unwrap_or(ColumnAlignment::None);
                        let header_text = RichText::new(header).strong();
                        match align {
                            ColumnAlignment::Right => {
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.label(header_text);
                                    },
                                );
                            }
                            ColumnAlignment::Center => {
                                ui.vertical_centered(|ui| {
                                    ui.label(header_text);
                                });
                            }
                            _ => {
                                ui.label(header_text);
                            }
                        }
                    }
                    ui.end_row();

                    // Data rows
                    for row in rows {
                        for (col_idx, cell) in row.iter().enumerate() {
                            let align = alignments
                                .get(col_idx)
                                .copied()
                                .unwrap_or(ColumnAlignment::None);
                            match align {
                                ColumnAlignment::Right => {
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            ui.label(cell);
                                        },
                                    );
                                }
                                ColumnAlignment::Center => {
                                    ui.vertical_centered(|ui| {
                                        ui.label(cell);
                                    });
                                }
                                _ => {
                                    ui.label(cell);
                                }
                            }
                        }
                        ui.end_row();
                    }
                });
        });
}

fn render_code_block(
    ui: &mut Ui,
    language: &str,
    code: &str,
    is_dark: bool,
    copied_state: &mut Option<(u64, Instant)>,
) {
    let highlighter = get_highlighter();
    let theme_colors = highlighter.theme_palette(is_dark);
    let bg_color = Color32::from_rgb(theme_colors.bg.0, theme_colors.bg.1, theme_colors.bg.2);
    let border_color = if is_dark {
        Color32::from_rgb(39, 39, 42)
    } else {
        Color32::from_rgb(228, 228, 231)
    };

    let block_hash = fast_hash_str(code);

    Frame::NONE
        .fill(bg_color)
        .stroke(Stroke::new(1.0_f32, border_color))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin::symmetric(12, 10))
        .show(ui, |ui| {
            // Header Bar
            ui.horizontal(|ui| {
                let display_lang = if language.is_empty() {
                    "CODE"
                } else {
                    language
                };
                ui.label(
                    RichText::new(display_lang.to_uppercase())
                        .font(FontId::monospace(11.0))
                        .strong()
                        .color(if is_dark {
                            Color32::from_rgb(161, 161, 170)
                        } else {
                            Color32::from_rgb(113, 113, 122)
                        }),
                );

                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    let is_recently_copied = copied_state
                        .as_ref()
                        .map(|(h, t)| *h == block_hash && t.elapsed().as_secs() < 2)
                        .unwrap_or(false);

                    let (copy_label, copy_color) = if is_recently_copied {
                        ("✓ Copied!", Color32::from_rgb(34, 197, 94))
                    } else {
                        (
                            "📋 Copy",
                            if is_dark {
                                Color32::from_rgb(212, 212, 216)
                            } else {
                                Color32::from_rgb(82, 82, 91)
                            },
                        )
                    };

                    if ui
                        .button(RichText::new(copy_label).size(12.0).color(copy_color))
                        .clicked()
                    {
                        ui.ctx().copy_text(code.to_string());
                        *copied_state = Some((block_hash, Instant::now()));
                    }
                });
            });

            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);

            // Highlighted Code Content
            let highlighted_lines = highlighter.highlight_code(code, language, is_dark);
            let mut job = LayoutJob::default();
            let mono_font = FontId::monospace(13.0);

            for line in highlighted_lines {
                for span in line {
                    job.append(
                        &span.text,
                        0.0,
                        TextFormat {
                            font_id: mono_font.clone(),
                            color: Color32::from_rgb(span.r, span.g, span.b),
                            italics: span.italic,
                            ..Default::default()
                        },
                    );
                }
            }

            egui::ScrollArea::horizontal()
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    ui.label(job);
                });
        });
}

fn render_mermaid_block(ui: &mut Ui, source: &str, is_dark: bool) {
    let border_color = if is_dark {
        Color32::from_rgb(59, 130, 246)
    } else {
        Color32::from_rgb(37, 99, 235)
    };
    let bg_color = if is_dark {
        Color32::from_rgb(15, 23, 42)
    } else {
        Color32::from_rgb(241, 245, 249)
    };

    Frame::NONE
        .fill(bg_color)
        .stroke(Stroke::new(1.0_f32, border_color))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin::symmetric(12, 10))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("📊 MERMAID DIAGRAM")
                        .font(FontId::monospace(11.0))
                        .strong()
                        .color(border_color),
                );
            });

            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);

            ui.label(
                RichText::new(source)
                    .font(FontId::monospace(12.0))
                    .color(if is_dark {
                        Color32::from_rgb(148, 163, 184)
                    } else {
                        Color32::from_rgb(71, 85, 105)
                    }),
            );
        });
}

fn render_footnote(ui: &mut Ui, label: &str, inlines: &[InlineSpan], is_dark: bool) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!("[^{}]:", label))
                .strong()
                .color(if is_dark {
                    Color32::from_rgb(96, 165, 250)
                } else {
                    Color32::from_rgb(29, 78, 216)
                }),
        );
        render_paragraph(ui, inlines, is_dark, None, None);
    });
}

pub fn toggle_checkbox_at_line(content: &mut String, target_line_1_indexed: usize) {
    let mut new_content = String::with_capacity(content.len());
    for (idx, line_slice) in content.split_inclusive('\n').enumerate() {
        let current_line = idx + 1;
        if current_line == target_line_1_indexed {
            if line_slice.contains("- [ ]") {
                new_content.push_str(&line_slice.replacen("- [ ]", "- [x]", 1));
            } else if line_slice.contains("- [x]") {
                new_content.push_str(&line_slice.replacen("- [x]", "- [ ]", 1));
            } else if line_slice.contains("- [X]") {
                new_content.push_str(&line_slice.replacen("- [X]", "- [ ]", 1));
            } else {
                new_content.push_str(line_slice);
            }
        } else {
            new_content.push_str(line_slice);
        }
    }
    *content = new_content;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toggle_checkbox_crlf_preservation() {
        let mut doc = String::from("Header\r\n- [ ] Task 1\r\n- [x] Task 2\r\nFooter\r\n");
        toggle_checkbox_at_line(&mut doc, 2);
        assert_eq!(doc, "Header\r\n- [x] Task 1\r\n- [x] Task 2\r\nFooter\r\n");

        toggle_checkbox_at_line(&mut doc, 3);
        assert_eq!(doc, "Header\r\n- [x] Task 1\r\n- [ ] Task 2\r\nFooter\r\n");
    }

    #[test]
    fn test_toggle_checkbox_lf_preservation() {
        let mut doc = String::from("Header\n- [ ] Task 1\nFooter\n");
        toggle_checkbox_at_line(&mut doc, 2);
        assert_eq!(doc, "Header\n- [x] Task 1\nFooter\n");
    }

    #[test]
    fn test_toggle_checkbox_no_trailing_newline() {
        let mut doc = String::from("Header\n- [ ] Task 1");
        toggle_checkbox_at_line(&mut doc, 2);
        assert_eq!(doc, "Header\n- [x] Task 1");
    }
}
