use crate::state::AppState;
use airgap_core::{TreeNode, VaultItemType};
use egui::{Color32, Key, RichText, Sense, Ui};

pub fn render_sidebar(ui: &mut Ui, state: &mut AppState) {
    ui.vertical(|ui| {
        // Vault Header
        ui.horizontal(|ui| {
            let vault_name = state
                .vault
                .as_ref()
                .and_then(|v| v.root().file_name())
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "Vault".to_string());

            ui.label(
                RichText::new(format!("📁 {}", vault_name))
                    .strong()
                    .size(15.0),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button(RichText::new("⚙").size(14.0))
                    .on_hover_text("Switch / Open Vault")
                    .clicked()
                {
                    state.show_vault_picker = true;
                }

                if ui
                    .button(RichText::new("+ Folder").size(12.0))
                    .on_hover_text("New Folder (Ctrl+Shift+N)")
                    .clicked()
                {
                    state.create_new_folder(None, "New Folder");
                }

                if ui
                    .button(RichText::new("+ Note").size(12.0))
                    .on_hover_text("New Note (Ctrl+N)")
                    .clicked()
                {
                    state.create_new_note(None);
                }
            });
        });

        ui.add_space(4.0);
        ui.separator();
        ui.add_space(4.0);

        // Scrollable directory tree
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if let Some(tree) = state.tree.clone() {
                    if tree.children.is_empty() {
                        ui.add_space(20.0);
                        ui.vertical_centered(|ui| {
                            ui.label(
                                RichText::new("Vault is empty")
                                    .color(Color32::from_rgb(161, 161, 170))
                                    .size(13.0),
                            );
                            if ui.button("+ Create First Note").clicked() {
                                state.create_new_note(None);
                            }
                        });
                    } else {
                        for child in &tree.children {
                            render_tree_node(ui, state, child, 0);
                        }
                    }
                }
            });
    });
}

fn render_tree_node(ui: &mut Ui, state: &mut AppState, node: &TreeNode, depth: usize) {
    let indent = depth as f32 * 14.0;

    match node.item_type {
        VaultItemType::Folder => {
            let is_expanded = state.expanded_folders.contains(&node.relative_path);
            let arrow_icon = if is_expanded { "▼" } else { "▶" };

            ui.horizontal(|ui| {
                ui.add_space(indent);

                // Check if currently inline renaming this folder
                let is_renaming = state
                    .renaming_item
                    .as_ref()
                    .map(|(p, _)| p == &node.relative_path)
                    .unwrap_or(false);

                if is_renaming {
                    if let Some((_, name_buf)) = &mut state.renaming_item {
                        let text_edit = egui::TextEdit::singleline(name_buf).desired_width(140.0);
                        let response = ui.add(text_edit);
                        response.request_focus();

                        if response.lost_focus() || ui.input(|i| i.key_pressed(Key::Enter)) {
                            state.commit_rename();
                        } else if ui.input(|i| i.key_pressed(Key::Escape)) {
                            state.cancel_rename();
                        }
                    }
                } else {
                    let folder_text = format!("{} 📁 {}", arrow_icon, node.name);
                    let label = egui::Label::new(RichText::new(folder_text).strong().size(13.5))
                        .sense(Sense::click());

                    let response = ui.add(label);

                    if response.clicked() {
                        state.toggle_folder_expanded(&node.relative_path);
                    }

                    // Context Menu
                    response.context_menu(|ui| {
                        if ui.button("📄 New Note Here").clicked() {
                            state.create_new_note(Some(&node.relative_path));
                            state.expanded_folders.insert(node.relative_path.clone());
                            ui.close_menu();
                        }
                        if ui.button("📁 New Subfolder").clicked() {
                            state.create_new_folder(Some(&node.relative_path), "Subfolder");
                            state.expanded_folders.insert(node.relative_path.clone());
                            ui.close_menu();
                        }
                        if ui.button("✏️ Rename (F2)").clicked() {
                            state.start_rename(node.relative_path.clone());
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui
                            .button(RichText::new("🗑 Delete").color(Color32::from_rgb(239, 68, 68)))
                            .clicked()
                        {
                            state.request_delete(node.relative_path.clone());
                            ui.close_menu();
                        }
                    });
                }
            });

            if is_expanded {
                for child in &node.children {
                    render_tree_node(ui, state, child, depth + 1);
                }
            }
        }
        VaultItemType::Note => {
            let is_selected = state.active_note_path.as_ref() == Some(&node.relative_path);

            ui.horizontal(|ui| {
                ui.add_space(indent + 12.0);

                let is_renaming = state
                    .renaming_item
                    .as_ref()
                    .map(|(p, _)| p == &node.relative_path)
                    .unwrap_or(false);

                if is_renaming {
                    if let Some((_, name_buf)) = &mut state.renaming_item {
                        let text_edit = egui::TextEdit::singleline(name_buf).desired_width(140.0);
                        let response = ui.add(text_edit);
                        response.request_focus();

                        if response.lost_focus() || ui.input(|i| i.key_pressed(Key::Enter)) {
                            state.commit_rename();
                        } else if ui.input(|i| i.key_pressed(Key::Escape)) {
                            state.cancel_rename();
                        }
                    }
                } else {
                    let mut text = RichText::new(format!("📝 {}", node.name)).size(13.5);
                    if is_selected {
                        text = text.strong().color(Color32::from_rgb(96, 165, 250));
                    }

                    let label = egui::Label::new(text).sense(Sense::click());
                    let response = ui.add(label);

                    if response.clicked() {
                        state.select_note(node.relative_path.clone());
                    }

                    // Context Menu
                    response.context_menu(|ui| {
                        if ui.button("✏️ Rename (F2)").clicked() {
                            state.start_rename(node.relative_path.clone());
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui
                            .button(RichText::new("🗑 Delete").color(Color32::from_rgb(239, 68, 68)))
                            .clicked()
                        {
                            state.request_delete(node.relative_path.clone());
                            ui.close_menu();
                        }
                    });
                }
            });
        }
    }
}
