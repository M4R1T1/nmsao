use crate::config::save_config;
use crate::utils::format_duration;

use super::MyApp;
use eframe::egui;
use egui::text::{CCursor, CCursorRange};
use egui::text_edit::TextEditState;
use egui::text_selection::TextCursorState;

impl MyApp {

    pub(super) fn show_modals(&mut self, ctx: &egui::Context) {
        self.show_preferences_dialog(ctx);
        self.show_new_project_dialog(ctx);
        self.show_rename_dialog(ctx);
        self.show_delete_dialog(ctx);
        self.show_combine_dialog(ctx);
        self.show_import_dialog(ctx);
        self.show_player_dialog(ctx);
        self.show_clean_audio_dialog(ctx);
    }

    fn show_preferences_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_preferences_dialog { return; }
        egui::Modal::new(egui::Id::new("Preferences"))
                .show(ctx, |ui| {
                    ui.set_width(700.0);
                    ui.heading("Preferences");
                    ui.add_space(8.0);
                    let mut changed = false;
                    ui.horizontal(|ui| {
                        ui.label("Default Source Path:");
                        {
                            let mut cfg = self.config.borrow_mut();
                            let resp = ui.add(
                                egui::TextEdit::singleline(&mut cfg.default_source_path)
                                    .desired_width(525.0)
                                    .id(egui::Id::new("prefs_default_source_path"))
                            );
                            if resp.changed() {
                                changed = true;
                            }
                        }
                        if ui.button("+=").clicked() {
                            let browser_path = self.current_path.borrow().display().to_string();
                            self.config.borrow_mut().default_source_path = browser_path;
                            changed = true;
                        }
                    });
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.label("EXTRACTED Folder Path:");
                        let mut cfg = self.config.borrow_mut();
                        let resp = ui.add(
                            egui::TextEdit::singleline(&mut cfg.assets_path)
                                .desired_width(533.0)
                                .id(egui::Id::new("prefs_assets_path"))
                        );
                        if resp.changed() {
                            changed = true;
                        }
                    });
                    if changed {
                        save_config(&self.logs_dir, &self.config.borrow());
                    }
                    ui.add_space(6.0);
                    {
                        let mut cfg = self.config.borrow_mut();
                        ui.horizontal(|ui| {
                            ui.label("Preserve organized folder structure when exporting game files to OGG");
                            let resp = ui.checkbox(&mut cfg.preserve_export_structure, "");
                            if resp.changed() {
                                changed = true;
                            }
                        });
                    }
                    ui.add_space(8.0);
                    ui.separator();
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                        if ui.button("Close").clicked() {
                            self.show_preferences_dialog = false;
                            *self.new_project_dialog_open.borrow_mut() = false;
                        }
                    });
                    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        self.show_preferences_dialog = false;
                        *self.new_project_dialog_open.borrow_mut() = false;
                        ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
                    }
                });
    }
    
    fn show_new_project_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_new_project_dialog { return; }
        egui::Modal::new(egui::Id::new("Create New Project"))
                .show(ctx, |ui| {
                    ui.set_width(350.0);
                    ui.heading("Create New Project");
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.label("Project Name:");
                        let response = ui.add(egui::TextEdit::singleline(&mut self.new_project_name));
                        if self.project_name_edit_id.is_none() {
                            self.project_name_edit_id = Some(response.id);
                        }
                        if self.project_name_edit_id.is_some() && ui.memory(|mem| mem.focused().is_none()) {
                            ui.memory_mut(|mem| mem.request_focus(response.id));
                        }
                    });
                    if let Some(err) = &self.project_creation_error {
                        ui.colored_label(egui::Color32::RED, err);
                    }
                    ui.separator();
                    let focused = ui.memory(|mem| mem.focused());
                    if focused == self.project_name_edit_id && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        self.try_create_project();
                        ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
                    }
                    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        self.show_new_project_dialog = false;
                        *self.new_project_dialog_open.borrow_mut() = false;
                        self.new_project_name.clear();
                        self.project_creation_error = None;
                        ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                        if ui.button("OK").clicked() {
                            self.try_create_project();
                        }
                        if ui.button("Cancel").clicked() {
                            self.show_new_project_dialog = false;
                            *self.new_project_dialog_open.borrow_mut() = false;
                            self.new_project_name.clear();
                            self.project_creation_error = None;
                        }
                    });
                });
    }

    fn show_rename_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_rename_dialog { return; }
        egui::Modal::new(egui::Id::new("Rename Project"))
                .show(ctx, |ui| {
                    ui.set_width(350.0);
                    ui.heading("Rename Project");
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.label("New Name:");
                        let rename_edit_id = ui.make_persistent_id("rename_edit");
                        self.rename_edit_id = Some(rename_edit_id);
                        let response = ui.add(egui::TextEdit::singleline(&mut self.rename_new_name).id(rename_edit_id));
                        if self.reset_rename_cursor {
                            let mut state = TextEditState::load(ui.ctx(), rename_edit_id).unwrap_or_default();
                            let len = self.rename_new_name.chars().count();
                            state.cursor = TextCursorState::from(CCursorRange::two(CCursor::new(len), CCursor::new(len)));
                            state.store(ui.ctx(), rename_edit_id);
                            self.reset_rename_cursor = false;
                        }
                        if !response.has_focus() {
                            response.request_focus();
                        }
                    });
                    if let Some(err) = &self.rename_error {
                        ui.colored_label(egui::Color32::RED, err);
                    }
                    ui.separator();
                    if ui.memory(|mem| mem.focused()) == self.rename_edit_id && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        self.try_rename_project();
                        ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
                    }
                    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        self.show_rename_dialog = false;
                        self.rename_old_path = None;
                        self.rename_new_name.clear();
                        self.rename_error = None;
                        *self.new_project_dialog_open.borrow_mut() = false;
                        ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                        if ui.button("OK").clicked() {
                            if self.try_rename_project() {
                                *self.new_project_dialog_open.borrow_mut() = false;
                            }
                        }
                        if ui.button("Cancel").clicked() {
                            self.show_rename_dialog = false;
                            self.rename_old_path = None;
                            self.rename_new_name.clear();
                            self.rename_error = None;
                            *self.new_project_dialog_open.borrow_mut() = false;
                        }
                    });
                });
    }

    fn show_delete_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_delete_dialog { return; }
        egui::Modal::new(egui::Id::new("Delete Project"))
                .show(ctx, |ui| {
                    ui.set_width(350.0);
                    ui.heading("Delete Project");
                    ui.add_space(8.0);
                    ui.label("Type DELETE to confirm deletion:");
                    let delete_edit_id = ui.make_persistent_id("delete_edit");
                    let response = ui.add(egui::TextEdit::singleline(&mut self.delete_confirmation).id(delete_edit_id));
                    if !response.has_focus() {
                        ui.memory_mut(|mem| mem.request_focus(response.id));
                    }
                    if let Some(err) = &self.delete_error {
                        ui.colored_label(egui::Color32::RED, err);
                    }
                    ui.separator();
                    if ui.memory(|mem| mem.focused()) == Some(delete_edit_id) && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        self.try_delete_project();
                        ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
                    }
                    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        self.show_delete_dialog = false;
                        self.delete_confirmation.clear();
                        self.delete_error = None;
                        *self.new_project_dialog_open.borrow_mut() = false;
                        ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                        if ui.button("OK").clicked() {
                            if self.try_delete_project() {
                                *self.new_project_dialog_open.borrow_mut() = false;
                            }
                        }
                        if ui.button("Cancel").clicked() {
                            self.show_delete_dialog = false;
                            self.delete_confirmation.clear();
                            self.delete_error = None;
                            *self.new_project_dialog_open.borrow_mut() = false;
                        }
                    });
                });
    }

    fn show_combine_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_combine_dialog { return; }
        egui::Modal::new(egui::Id::new("Combine Projects"))
                .show(ctx, |ui| {
                    ui.set_width(420.0);
                    ui.heading("Combine");
                    ui.add_space(8.0);
                    let mut projects = Vec::new();
                    if let Ok(entries) = std::fs::read_dir(&self.projects_dir) {
                        for entry in entries.flatten() {
                            let path = entry.path();
                            if path.is_dir() {
                                projects.push(path);
                            }
                        }
                    }
                    ui.label("Priority order (top wins)");
                    ui.add_space(4.0);
                    egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                        let mut remove_at: Option<usize> = None;
                        let total = self.combine_selections.len();
                        for i in 0..total {
                            ui.push_id(i, |ui| {
                                let slot = &mut self.combine_selections[i];
                                let is_bottom = i + 1 == total;
                                if slot.is_none() && !is_bottom {
                                    remove_at = Some(i);
                                }
                                let label = slot
                                    .as_ref()
                                    .and_then(|p| p.file_name())
                                    .map(|n| n.to_string_lossy().to_string())
                                    .unwrap_or_else(|| "Select...".to_string());
                                ui.horizontal(|ui| {
                                    ui.label(format!("{}:", i + 1));
                                    egui::ComboBox::from_label("")
                                        .selected_text(label)
                                        .show_ui(ui, |ui| {
                                            ui.selectable_value(slot, None, "None");
                                            for proj in &projects {
                                                let name = proj
                                                    .file_name()
                                                    .unwrap_or_default()
                                                    .to_string_lossy()
                                                    .to_string();
                                                ui.selectable_value(slot, Some(proj.clone()), name);
                                            }
                                        });
                                });
                            });
                        }
                        if let Some(idx) = remove_at {
                            self.combine_selections.remove(idx);
                        }
                    });
                    self.combine_selections.retain(|s| s.is_some());
                    self.combine_selections.push(None);
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        ui.label("Combined Project Name:");
                        ui.add(egui::TextEdit::singleline(&mut self.combine_new_name));
                    });
                    if let Some(err) = &self.combine_error {
                        ui.colored_label(egui::Color32::RED, err);
                    }
                    ui.add_space(8.0);
                    ui.separator();
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                        if ui.button("Combine").clicked() {
                            self.try_combine_projects();
                        }
                        if ui.button("Cancel").clicked() {
                            self.show_combine_dialog = false;
                            self.combine_selections = vec![None];
                            self.combine_new_name.clear();
                            self.combine_error = None;
                            *self.new_project_dialog_open.borrow_mut() = false;
                        }
                    });
                    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        self.show_combine_dialog = false;
                        self.combine_selections = vec![None];
                        self.combine_new_name.clear();
                        self.combine_error = None;
                        *self.new_project_dialog_open.borrow_mut() = false;
                        ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
                    }
                });
    }

    fn show_import_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_import_dialog { return; }
        egui::Modal::new(egui::Id::new("Import Project"))
                .show(ctx, |ui| {
                    ui.set_width(500.0);
                    ui.heading("Import Project");
                    ui.add_space(8.0);
                    ui.label("Path to external mod:");
                    let import_edit_id = ui.make_persistent_id("import_path_edit");
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.import_path)
                            .desired_width(f32::INFINITY)
                            .id(import_edit_id)
                    );
                    if !response.has_focus() {
                        ui.memory_mut(|mem| mem.request_focus(response.id));
                    }
                    if let Some(err) = &self.import_error {
                        ui.colored_label(egui::Color32::RED, err);
                    }
                    ui.add_space(8.0);
                    ui.separator();
                    if ui.memory(|mem| mem.focused()) == Some(import_edit_id)
                        && ui.input(|i| i.key_pressed(egui::Key::Enter))
                    {
                        self.try_import_project();
                        ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                        if ui.button("OK").clicked() {
                            self.try_import_project();
                        }
                        if ui.button("Cancel").clicked() {
                            self.show_import_dialog = false;
                            self.import_path.clear();
                            self.import_error = None;
                            *self.new_project_dialog_open.borrow_mut() = false;
                        }
                    });
                    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        self.show_import_dialog = false;
                        self.import_path.clear();
                        self.import_error = None;
                        *self.new_project_dialog_open.borrow_mut() = false;
                        ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
                    }
                });
    }

    fn show_player_dialog(&mut self, ctx: &egui::Context) {
        if !self.player_modal_open { return; }
            egui::Modal::new(egui::Id::new("Audio Player"))
                .show(ctx, |ui| {
                    ui.set_width(420.0);
                    let name = self.player_source
                        .as_ref()
                        .and_then(|p| p.file_name())
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| "Unknown".to_string());
                    ui.heading(name);
                    ui.add_space(8.0);
                    let is_playing = self
                        .audio_player
                        .as_ref()
                        .map(|s| !s.is_paused() && !s.empty())
                        .unwrap_or(false);
                    ui.horizontal(|ui| {
                        if ui.button("⏪").clicked() {
                            self.seek_relative(-10.0);
                        }
                        if ui.button(if is_playing { "⏸" } else { "▶" }).clicked() {
                            self.toggle_playback();
                        }
                        if ui.button("⏩").clicked() {
                            self.seek_relative(10.0);
                        }
                        ui.add_space(12.0);
                        let pos = self.current_position();
                        let total = self.player_duration;
                        ui.label(format!("{} / {}", format_duration(pos), format_duration(total)));
                    });
                    ui.add_space(8.0);
                    ui.separator();
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                        if ui.button("Close").clicked() {
                            if let Some(player) = &self.audio_player {
                                player.stop();
                            }
                            self.player_modal_open = false;
                            self.player_source = None;
                            self.player_bytes = None;
                            self.player_play_started = None;
                            self.player_paused_at = std::time::Duration::ZERO;
                        }
                    });
                    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        if let Some(player) = &self.audio_player {
                            player.stop();
                        }
                        self.player_modal_open = false;
                        self.player_source = None;
                        self.player_bytes = None;
                        ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
                    }
                    let finished = self.audio_player.as_ref().map(|p| p.empty()).unwrap_or(false);
                    if finished && self.player_play_started.is_some() {
                        self.player_play_started = None;
                        self.player_paused_at = std::time::Duration::ZERO;
                    }
                    let space_pressed = ctx.input(|i| i.key_pressed(egui::Key::Space));
                    if space_pressed {
                        self.toggle_playback();
                        ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Space));
                    }
                    let left_pressed = ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft));
                    let right_pressed = ctx.input(|i| i.key_pressed(egui::Key::ArrowRight));
                    if left_pressed {
                        self.seek_relative(-10.0);
                    }
                    if right_pressed {
                        self.seek_relative(10.0);
                    }
                });
                ctx.request_repaint();
    }

    fn show_clean_audio_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_clean_audio_dialog { return; }
        egui::Modal::new(egui::Id::new("Clean Organized Audio"))
                .show(ctx, |ui| {
                    ui.set_width(350.0);
                    ui.heading("Delete Organized Audio");
                    ui.add_space(8.0);
                    ui.label("Type DELETE to confirm deletion (program may lag)");
                    let clean_edit_id = ui.make_persistent_id("clean_edit");
                    let response = ui.add(egui::TextEdit::singleline(&mut self.clean_audio_confirmation).id(clean_edit_id));
                    if !response.has_focus() {
                        ui.memory_mut(|mem| mem.request_focus(response.id));
                    }
                    if let Some(err) = &self.clean_audio_error {
                        ui.colored_label(egui::Color32::RED, err);
                    }
                    ui.separator();
                    if ui.memory(|mem| mem.focused()) == Some(clean_edit_id) && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        self.try_clean_organized_audio();
                        ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
                    }
                    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        self.show_clean_audio_dialog = false;
                        self.clean_audio_confirmation.clear();
                        self.clean_audio_error = None;
                        *self.new_project_dialog_open.borrow_mut() = false;
                        ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                        if ui.button("OK").clicked() {
                            if self.try_clean_organized_audio() {
                                *self.new_project_dialog_open.borrow_mut() = false;
                            }
                        }
                        if ui.button("Cancel").clicked() {
                            self.show_clean_audio_dialog = false;
                            self.clean_audio_confirmation.clear();
                            self.clean_audio_error = None;
                            *self.new_project_dialog_open.borrow_mut() = false;
                        }
                    });
                });
    }
}