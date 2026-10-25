use crate::state::AppState;
use egui::{Align, Color32, Context, Layout, RichText, Vec2};
use std::path::PathBuf;

pub fn render_vault_picker(ctx: &Context, state: &mut AppState) {
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(40.0);

            ui.heading(
                RichText::new("AirGap")
                    .size(32.0)
                    .strong()
                    .color(Color32::from_rgb(96, 165, 250)),
            );

            ui.label(
                RichText::new("Fast, native, local-only Markdown notes on real disk folders")
                    .size(15.0)
                    .color(Color32::from_rgb(161, 161, 170)),
            );

            ui.add_space(30.0);

            // Centered card container
            egui::Frame::group(ui.style())
                .inner_margin(egui::Margin::same(24))
                .corner_radius(egui::CornerRadius::same(8))
                .show(ui, |ui| {
                    ui.set_max_width(520.0);

                    ui.label(RichText::new("Open Local Vault").strong().size(18.0));
                    ui.label("Select any folder on your computer to open as your note vault:");

                    ui.add_space(12.0);

                    ui.horizontal(|ui| {
                        let text_edit = egui::TextEdit::singleline(&mut state.vault_picker_path)
                            .hint_text("C:\\Users\\...\\Notes")
                            .desired_width(380.0);
                        ui.add(text_edit);

                        if ui.button("Browse...").clicked() {
                            if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                                state.vault_picker_path = folder.to_string_lossy().into_owned();
                            }
                        }
                    });

                    if let Some(err) = &state.vault_picker_error {
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new(format!("⚠️ {}", err))
                                .color(Color32::from_rgb(239, 68, 68))
                                .size(13.0),
                        );
                    }

                    ui.add_space(16.0);

                    ui.horizontal(|ui| {
                        let open_btn = ui.add_sized(
                            Vec2::new(140.0, 32.0),
                            egui::Button::new(RichText::new("Open Vault").strong()),
                        );

                        if open_btn.clicked() {
                            let path = PathBuf::from(state.vault_picker_path.trim());
                            if path.as_os_str().is_empty() {
                                state.vault_picker_error =
                                    Some("Please specify a directory path.".to_string());
                            } else if !path.is_dir() {
                                state.vault_picker_error = Some(
                                    "Specified path does not exist or is not a directory."
                                        .to_string(),
                                );
                            } else {
                                match state.open_vault(path) {
                                    Ok(()) => {}
                                    Err(e) => {
                                        state.vault_picker_error =
                                            Some(format!("Error opening vault: {}", e));
                                    }
                                }
                            }
                        }

                        if state.vault.is_some() && ui.button("Cancel").clicked() {
                            state.show_vault_picker = false;
                            state.vault_picker_error = None;
                        }
                    });

                    ui.add_space(20.0);
                    ui.separator();
                    ui.add_space(16.0);

                    ui.label(RichText::new("Need a place to start?").strong());
                    ui.label("Create an organized sample vault with welcome notes and guides:");

                    ui.add_space(10.0);

                    if ui
                        .add_sized(
                            Vec2::new(220.0, 32.0),
                            egui::Button::new("✨ Create & Open Sample Vault"),
                        )
                        .clicked()
                    {
                        match AppState::create_sample_vault() {
                            Ok(sample_dir) => {
                                let _ = state.open_vault(sample_dir);
                            }
                            Err(e) => {
                                state.vault_picker_error =
                                    Some(format!("Failed to create sample vault: {}", e));
                            }
                        }
                    }
                });

            ui.add_space(40.0);

            // Zero-network privacy badge
            ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
                ui.label(
                    RichText::new(
                        "🛡️ AirGap 100% Local-Only: No Network Sockets, No Telemetry, No Analytics",
                    )
                    .size(12.0)
                    .color(Color32::from_rgb(113, 113, 122)),
                );
            });
        });
    });
}
