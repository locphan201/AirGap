use crate::font::setup_fonts;
use crate::state::{AppState, ViewMode};
use crate::theme::apply_theme;
use crate::ui::{
    render_delete_modal, render_editor, render_preview, render_sidebar, render_status_bar,
    render_vault_picker,
};
use egui::{vec2, Color32, Context, Key, Rect, Stroke};
use std::time::Duration;

#[derive(Default)]
pub struct AirGapApp {
    pub state: AppState,
    fonts_initialized: bool,
}

impl AirGapApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self::default()
    }

    fn handle_keyboard_shortcuts(&mut self, ctx: &Context) {
        ctx.input(|i| {
            let ctrl = i.modifiers.command || i.modifiers.ctrl;
            let shift = i.modifiers.shift;

            // Ctrl+S: Save note
            if ctrl && !shift && i.key_pressed(Key::S) {
                self.state.save_active_note();
            }

            // Ctrl+\: Cycle view modes (Split, EditorOnly, PreviewOnly)
            if ctrl && !shift && i.key_pressed(Key::Backslash) {
                self.state.view_mode = self.state.view_mode.cycle();
            }

            // Ctrl+F: Open Find
            if ctrl && !shift && i.key_pressed(Key::F) {
                self.state.find_replace.is_open = true;
                self.state.find_replace.show_replace = false;
                self.state
                    .find_replace
                    .recalculate_matches(&self.state.editor_content);
            }

            // Ctrl+H: Open Find & Replace
            if ctrl && !shift && i.key_pressed(Key::H) {
                self.state.find_replace.is_open = true;
                self.state.find_replace.show_replace = true;
                self.state
                    .find_replace
                    .recalculate_matches(&self.state.editor_content);
            }

            // Ctrl+Shift+N: New folder
            if ctrl && shift && i.key_pressed(Key::N) {
                self.state.create_new_folder(None, "New Folder");
            }
            // Ctrl+N: New note
            else if ctrl && !shift && i.key_pressed(Key::N) {
                self.state.create_new_note(None);
            }

            // F2: Rename active note
            if i.key_pressed(Key::F2) {
                if let Some(active) = self.state.active_note_path.clone() {
                    self.state.start_rename(active);
                }
            }

            // Delete: Delete active note
            if i.key_pressed(Key::Delete) && self.state.renaming_item.is_none() {
                if let Some(active) = self.state.active_note_path.clone() {
                    self.state.request_delete(active);
                }
            }
        });
    }
}

impl eframe::App for AirGapApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        // One-time font initialization
        if !self.fonts_initialized {
            setup_fonts(ctx);
            self.fonts_initialized = true;
        }

        // Apply visual theme
        apply_theme(ctx, self.state.theme_mode);

        // Global keyboard accelerators
        self.handle_keyboard_shortcuts(ctx);

        // Autosave evaluation (debounced ~1s and on window blur)
        let is_focused = ctx.input(|i| i.focused);
        self.state.check_autosave(is_focused);

        // Request repaint while dirty so ~1s timer triggers without user movement
        if self.state.is_dirty() {
            ctx.request_repaint_after(Duration::from_millis(200));
        }

        // Render main view or vault picker
        if self.state.show_vault_picker || self.state.vault.is_none() {
            render_vault_picker(ctx, &mut self.state);
        } else {
            // Status bar at bottom
            egui::TopBottomPanel::bottom("status_bar")
                .exact_height(26.0)
                .show(ctx, |ui| {
                    render_status_bar(ui, &mut self.state);
                });

            // Resizable sidebar on left
            egui::SidePanel::left("sidebar")
                .resizable(true)
                .default_width(260.0)
                .min_width(180.0)
                .max_width(450.0)
                .show(ctx, |ui| {
                    render_sidebar(ui, &mut self.state);
                });

            // Central pane with triple view modes (EditorOnly, PreviewOnly, Split)
            egui::CentralPanel::default().show(ctx, |ui| {
                match self.state.view_mode {
                    ViewMode::EditorOnly => {
                        render_editor(ui, &mut self.state);
                    }
                    ViewMode::PreviewOnly => {
                        render_preview(ui, &mut self.state);
                    }
                    ViewMode::Split => {
                        let total_width = ui.available_width();
                        let total_height = ui.available_height();
                        let sep_width = 6.0;
                        let usable_width = (total_width - sep_width).max(200.0);

                        let ratio = self.state.split_ratio.clamp(0.15, 0.85);
                        let left_width = (usable_width * ratio).clamp(120.0, usable_width - 120.0);
                        let right_width = (usable_width - left_width).max(100.0);

                        let start_pos = ui.cursor().min;
                        let left_rect =
                            Rect::from_min_size(start_pos, vec2(left_width, total_height));
                        let sep_rect = Rect::from_min_size(
                            egui::pos2(start_pos.x + left_width, start_pos.y),
                            vec2(sep_width, total_height),
                        );
                        let right_rect = Rect::from_min_size(
                            egui::pos2(start_pos.x + left_width + sep_width, start_pos.y),
                            vec2(right_width, total_height),
                        );

                        // Left: Editor
                        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(left_rect), |ui| {
                            render_editor(ui, &mut self.state);
                        });

                        // Middle: Separator with drag resize & double click reset
                        let sep_response =
                            ui.allocate_rect(sep_rect, egui::Sense::click_and_drag());
                        if sep_response.dragged() {
                            let delta = sep_response.drag_delta().x;
                            let new_ratio = ((left_width + delta) / usable_width).clamp(0.15, 0.85);
                            self.state.split_ratio = new_ratio;
                        }
                        if sep_response.double_clicked() {
                            self.state.split_ratio = 0.5;
                        }

                        let sep_hovered = sep_response.hovered() || sep_response.dragged();
                        if sep_hovered {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                        }

                        let stroke_color = if sep_hovered {
                            ui.visuals().widgets.active.bg_fill
                        } else if self.state.theme_mode == crate::theme::ThemeMode::Dark {
                            Color32::from_rgb(39, 39, 42)
                        } else {
                            Color32::from_rgb(228, 228, 231)
                        };
                        ui.painter().vline(
                            sep_rect.center().x,
                            sep_rect.y_range(),
                            Stroke::new(1.5_f32, stroke_color),
                        );

                        // Right: Preview
                        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(right_rect), |ui| {
                            render_preview(ui, &mut self.state);
                        });
                    }
                }
            });

            // Delete confirmation modal if requested
            render_delete_modal(ctx, &mut self.state);
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Guaranteed flush of dirty buffer on app exit
        if self.state.is_dirty() {
            self.state.save_active_note();
        }
    }
}
