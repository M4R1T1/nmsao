use crate::models::FocusedPanel;
use super::MyApp;
use crate::ui::MyTabViewer;
use eframe::egui;
use egui_dock::DockArea;
use webbrowser;
use std::path::PathBuf;
use std::time::Instant;

impl MyApp {

    pub(super) fn show_toolbar(&mut self, ctx: &egui::Context) {
        egui::Panel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.menu_button("Project", |ui| {
                    if ui.button("  Preferences  ").clicked() {
                        self.show_preferences_dialog = true;
                        *self.new_project_dialog_open.borrow_mut() = true;
                        ui.close();
                    }
                    if ui.button("  New   ").clicked() {
                        self.show_new_project_dialog = true;
                        *self.new_project_dialog_open.borrow_mut() = true;
                        self.new_project_name.clear();
                        self.project_creation_error = None;
                    }
                    if ui.button("  Rename  ").clicked() {
                        let project = self.selected_project.borrow().clone();
                        if let Some(path) = project {
                            self.show_rename_dialog = true;
                            self.rename_old_path = Some(path.clone());
                            self.rename_new_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                            self.rename_error = None;
                            *self.new_project_dialog_open.borrow_mut() = true;
                            self.reset_rename_cursor = true;
                        } else {
                            self.log_and_status("No project selected");
                        }
                    }
                    if ui.button("  Delete  ").clicked() {
                        if self.selected_project.borrow().is_some() {
                            self.show_delete_dialog = true;
                            self.delete_confirmation.clear();
                            self.delete_error = None;
                            *self.new_project_dialog_open.borrow_mut() = true;
                        } else {
                            self.log_and_status("No project selected");
                        }
                    }
                    if ui.button("  Combine  ").clicked() {
                        self.show_combine_dialog = true;
                        *self.new_project_dialog_open.borrow_mut() = true;
                        self.combine_selections = vec![None];
                        self.combine_new_name.clear();
                        self.combine_error = None;
                        ui.close();
                    }
                    if ui.button("  Import  ").clicked() {
                        self.show_import_dialog = true;
                        *self.new_project_dialog_open.borrow_mut() = true;
                        self.import_path.clear();
                        self.import_error = None;
                        ui.close();
                    }
                    if ui.button("  Build Mod  ").clicked() {
                        self.build_mod();
                    }
                });
                ui.menu_button("Help", |ui| {
                    if ui.button(" Usage Guide").clicked() {
                        if let Err(e) = webbrowser::open("https://github.com/M4R1T1/nmsao") {
                            self.processing_status = Some(format!("failed to open web browser: {}", e));
                        }
                    }
                    if ui.button(" NMS Modding Discord").clicked() {
                        if let Err(e) = webbrowser::open("https://discord.gg/no-man-s-sky-modding-215514623384748034") {
                            self.processing_status = Some(format!("Failed to open browser: {}", e));
                        }
                    }
                });
                ui.menu_button("Process Assets", |ui| {
                    if ui.button(" Process ").clicked() {
                        let assets_root = self.config.borrow().assets_path.clone();
                        if assets_root.is_empty() {
                            self.log_and_status("Extraction folder not set: configure in Project > Preferences");
                        } else {
                            let root = PathBuf::from(&assets_root);
                            if !root.is_dir() {
                                self.log_and_status(&format!("Extraction folder does not exist: {}", assets_root));
                            } else {
                                let source = root.join("AUDIO").join("WINDOWS");
                                if !source.is_dir() {
                                    self.log_and_status(&format!("Expected AUDIO\\WINDOWS inside: {}", assets_root));
                                } else {
                                    self.start_processing(
                                        source,
                                        self.output_dir.clone(),
                                        self.logs_dir.clone(),
                                        ctx,
                                    );
                                }
                            }
                        }
                        ui.close();
                    }
                    if ui.button(" Clean Organized Audio Folders ").clicked() {
                        self.clean_organized_audio();
                        ui.close();
                    }
                });
                if let Some(status) = &self.processing_status {
                    ui.label(status);
                }
            });
        });
    }

    pub(super) fn show_bottom_panel(&mut self, ctx: &egui::Context) {
        egui::Panel::bottom("bottom_panel")
            .min_height(35.0)
            .show(ctx, |ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                let play_button = egui::Button::new(egui::RichText::new(" ▶").size(30.0)).min_size(egui::Vec2::new(80.0, 35.0));
                if ui.add(play_button).clicked() {
                    let focused = *self.focused_panel.borrow();
                    let (selected_path, error_message) = match focused {
                        FocusedPanel::OrganizedAudio => {
                            let set = self.organized_tree.selected_set.borrow();
                            let count = set.len();
                            if count == 1 {
                                (set.iter().next().cloned(), None)
                            } else if count > 1 {
                                (None, Some("Cannot play multiple files at once".to_string()))
                            } else {
                                (None, None)
                            }
                        }
                        FocusedPanel::SourceFiles => {
                            (self.selected_file.borrow().clone(), None)
                        }
                        FocusedPanel::FileQueue => (self.selected_file.borrow().clone(), None),
                    };
                    if let Some(err) = error_message {
                        self.processing_status = Some(err);
                        self.processing_status_time = Some(Instant::now());
                    } else if let Some(path) = selected_path {
                        let ext = path
                            .extension()
                            .and_then(|e| e.to_str())
                            .unwrap_or("")
                            .to_lowercase();
                        if ext == "wem" || ext == "wav" {
                            self.start_player(path);
                        } else {
                            self.processing_status = Some("Unsupported format for audio player".to_string());
                            self.processing_status_time = Some(Instant::now());
                        }
                    } else {
                        self.processing_status = Some("No file selected".to_string());
                        self.processing_status_time = Some(Instant::now());
                    }
                }
            });
        });
    }

    pub(super) fn show_central_panel(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut tab_viewer = MyTabViewer {
                selected_project: self.selected_project.clone(),
                projects_dir: self.projects_dir.clone(),
                link_tx: self.link_tx.as_ref().unwrap().clone(),
                remove_tx: self.remove_tx.as_ref().unwrap().clone(),
                silence_tx: self.silence_tx.as_ref().unwrap().clone(),
                export_tx: self.export_tx.as_ref().unwrap().clone(),
                copy_id_tx: self.copy_id_tx.as_ref().unwrap().clone(),
                pending_source_clear: self.pending_source_clear.clone(),
            };
            DockArea::new(&mut self.dock_state)
                .show_close_buttons(false)
                .draggable_tabs(false)
                .show_leaf_collapse_buttons(false)
                .show_leaf_close_all_buttons(false)
                .show_inside(ui, &mut tab_viewer);
        });
    }
}