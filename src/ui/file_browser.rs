use crate::config::AppConfig;
use crate::models::FocusedPanel;
use crate::tree::is_linkable_file;
use crate::config::save_config;
use eframe::egui;
use egui::text::{CCursor, CCursorRange};
use egui::text_edit::TextEditState;
use egui::text_selection::TextCursorState;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::fs;
use egui::Ui;

pub struct FileBrowser {
    pub(crate) current_path: Rc<RefCell<PathBuf>>,
    pub(crate) entries: Vec<(PathBuf, bool)>,
    pub(crate) path_edit_buffer: String,
    pub(crate) status_message: Option<String>,
    pub(crate) selected_file: Rc<RefCell<Option<PathBuf>>>,
    pub(crate) focused_panel: Rc<RefCell<FocusedPanel>>,
    pub(crate) path_edit_id: Option<egui::Id>,
    pub(crate) new_project_dialog_open: Rc<RefCell<bool>>,
    pub(crate) error_highlight: Rc<RefCell<Option<PathBuf>>>,
}

impl FileBrowser {
    pub(crate) fn new(
        selected_file: Rc<RefCell<Option<PathBuf>>>,
        focused_panel: Rc<RefCell<FocusedPanel>>,
        new_project_dialog_open: Rc<RefCell<bool>>,
        error_highlight: Rc<RefCell<Option<PathBuf>>>,
        config: Rc<RefCell<AppConfig>>,
        logs_dir: PathBuf,
        current_path: Rc<RefCell<PathBuf>>,
    ) -> Self {
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("/"));
        let configured = config.borrow().default_source_path.clone();
        let resolved = if !configured.is_empty() {
            let candidate = PathBuf::from(&configured);
            if candidate.exists() && candidate.is_dir() {
                candidate
            } else {
                exe_dir
            }
        } else {
            exe_dir
        };
        *current_path.borrow_mut() = resolved.clone();
        {
            let mut cfg = config.borrow_mut();
            cfg.default_source_path = resolved.display().to_string();
            save_config(&logs_dir, &cfg);
        }
        let path_edit_buffer = resolved.display().to_string();
        let mut browser = Self {
            current_path,
            entries: Vec::new(),
            path_edit_buffer,
            status_message: None,
            selected_file,
            focused_panel,
            path_edit_id: None,
            new_project_dialog_open,
            error_highlight,
        };
        browser.refresh_entries();
        browser
    }

    // re-reads current directory, directories first then files alphabetically
    fn refresh_entries(&mut self) {
        self.entries.clear();
        if let Ok(read_dir) = fs::read_dir(&*self.current_path.borrow()) {
            for entry in read_dir.flatten() {
                let path = entry.path();
                let is_dir = path.is_dir();
                self.entries.push((path, is_dir));
            }
        }
        self.entries.sort_by(|a, b| {
            if a.1 && !b.1 {
                std::cmp::Ordering::Less
            } else if !a.1 && b.1 {
                std::cmp::Ordering::Greater
            } else {
                a.0.file_name().cmp(&b.0.file_name())
            }
        });
    }

    // changes directory based on navigable browser
    fn navigate_to(&mut self, path: &Path) {
        if path.is_dir() {
            *self.current_path.borrow_mut() = path.to_path_buf();
            self.path_edit_buffer = path.display().to_string();
            self.refresh_entries();
            self.status_message = None;
            *self.error_highlight.borrow_mut() = None;
            *self.selected_file.borrow_mut() = None;
        }
    }

    // changes directory based on pasted text path
    fn navigate_to_path(&mut self) {
        let new_path = PathBuf::from(&self.path_edit_buffer);
        if new_path.exists() && new_path.is_dir() {
            self.navigate_to(&new_path);
        } else {
            let relative = self.current_path.borrow().join(&self.path_edit_buffer);
            if relative.exists() && relative.is_dir() {
                self.navigate_to(&relative);
            } else {
                self.status_message = Some(format!(
                    "Error: '{}' does not exist",
                    self.path_edit_buffer,
                ));
            }
        }
    }

    // navigates to parent directory
    fn go_up(&mut self) {
        let parent = self.current_path.borrow().parent().map(|p| p.to_path_buf());
        if let Some(parent) = parent {
            self.navigate_to(&parent);
        }
    }

    // renders path field and entry list
    pub(crate) fn show(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            let text_edit_id = ui.make_persistent_id("path edit");
            self.path_edit_id = Some(text_edit_id);
            let mut state = TextEditState::load(ui.ctx(), text_edit_id).unwrap_or_default();
            let response = ui.add(egui::TextEdit::singleline(&mut self.path_edit_buffer).id(text_edit_id).desired_width(f32::INFINITY));
            if response.gained_focus() {
                let num_chars = self.path_edit_buffer.chars().count();
                let range = CCursorRange::two(CCursor::new(0), CCursor::new(num_chars));
                state.cursor = TextCursorState::from(range);
                state.store(ui.ctx(), text_edit_id);
            }
        });
        let ctx = ui.ctx();
            if !*self.new_project_dialog_open.borrow() && ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
                ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
                self.navigate_to_path();
            }
        if let Some(msg) = &self.status_message {
            ui.colored_label(egui::Color32::RED, msg);
        }
        ui.separator();
        let mut navigate_to_path = None;
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                if ui.button(" . . ").clicked() {
                    self.go_up();
                }
                for (path, is_dir) in &self.entries {
                    let filename = path.file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    let icon = if *is_dir { "📁 " } else { "📄 " };
                    let label = format!("{}{}", icon, filename);
                    if *is_dir {
                        if ui.button(label).clicked() {
                            navigate_to_path = Some(path.clone());
                        }
                    } else {
                        let linkable = is_linkable_file(path);
                        let selected = *self.selected_file.borrow() == Some(path.clone());
                        let is_error = *self.error_highlight.borrow() == Some(path.clone());
                        let mut button = egui::Button::new(label);
                        if linkable && selected {
                            button = button.fill(egui::Color32::DARK_GREEN);
                        } else if is_error {
                            button = button.fill(egui::Color32::RED);
                        }
                        if ui.add(button).clicked() {
                            if linkable {
                                *self.selected_file.borrow_mut() = Some(path.clone());
                                *self.focused_panel.borrow_mut() = FocusedPanel::SourceFiles;
                                *self.error_highlight.borrow_mut() = None;
                                self.status_message = None;
                            } else {
                                *self.error_highlight.borrow_mut() = Some(path.clone());
                            }
                        }
                    }
                }
            });
            if let Some(path) = navigate_to_path {
                self.navigate_to(&path);
            }
        }
}