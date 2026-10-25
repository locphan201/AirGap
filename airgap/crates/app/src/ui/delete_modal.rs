use crate::state::AppState;
use egui::{Color32, Context, Key, RichText, Vec2};

pub fn render_delete_modal(ctx: &Context, state: &mut AppState) {
    if state.delete_confirm.is_none() {
        return;
    }

    let mut open = true;
    let file_name = state
        .delete_confirm
        .as_ref()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "item".to_string());

    egui::Window::new("Confirm Deletion")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .pivot(egui::Align2::CENTER_CENTER)
        .fixed_size(Vec2::new(380.0, 160.0))
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(8.0);
                ui.label(
                    RichText::new("🗑 Move to Recycle Bin?")
                        .size(16.0)
                        .strong()
                        .color(Color32::from_rgb(239, 68, 68)),
                );
                ui.add_space(10.0);
                ui.label(format!(
                    "Are you sure you want to delete '{}'?\nThis will safely move the item to your recycle bin.",
                    file_name
                ));

                ui.add_space(16.0);

                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let delete_btn = ui.add(
                            egui::Button::new(
                                RichText::new("Delete").strong().color(Color32::WHITE),
                            )
                            .fill(Color32::from_rgb(220, 38, 38)),
                        );

                        if delete_btn.clicked() || ui.input(|i| i.key_pressed(Key::Enter)) {
                            state.confirm_delete();
                        }

                        if ui.button("Cancel").clicked() || ui.input(|i| i.key_pressed(Key::Escape)) {
                            state.cancel_delete();
                        }
                    });
                });
            });
        });

    if !open {
        state.cancel_delete();
    }
}
