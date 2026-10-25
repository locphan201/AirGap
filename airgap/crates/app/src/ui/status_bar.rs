use crate::state::{AppState, SaveStatus};
use crate::theme::ThemeMode;
use egui::{Color32, RichText, Ui};

pub fn render_status_bar(ui: &mut Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        // Left: Active note info
        if let Some(note) = &state.active_note_path {
            ui.label(
                RichText::new(format!("📝 {}", note.display()))
                    .size(12.0)
                    .color(Color32::from_rgb(161, 161, 170)),
            );
        } else {
            ui.label(
                RichText::new("AirGap Ready")
                    .size(12.0)
                    .color(Color32::from_rgb(113, 113, 122)),
            );
        }

        ui.separator();

        // Save Status Indicator
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
                    RichText::new("⏳ Autosaving...")
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

        // Right-aligned metrics and theme selector
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Theme toggle button
            let theme_label = format!("🌓 Theme: {}", state.theme_mode.name());
            if ui.button(RichText::new(theme_label).size(12.0)).clicked() {
                state.theme_mode = match state.theme_mode {
                    ThemeMode::Dark => ThemeMode::Light,
                    ThemeMode::Light => ThemeMode::System,
                    ThemeMode::System => ThemeMode::Dark,
                };
            }

            ui.separator();

            // Cursor line & column
            ui.label(
                RichText::new(format!(
                    "Ln {}, Col {}",
                    state.cursor_line, state.cursor_col
                ))
                .size(12.0)
                .color(Color32::from_rgb(161, 161, 170)),
            );

            ui.separator();

            // Word and character counts
            ui.label(
                RichText::new(format!(
                    "{} words, {} chars",
                    state.word_count, state.char_count
                ))
                .size(12.0)
                .color(Color32::from_rgb(161, 161, 170)),
            );
        });
    });
}
