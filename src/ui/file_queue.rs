use super::MyTabViewer;
use crate::models::FocusedPanel;
use crate::tree::display_name;
use eframe::egui;
use std::cell::RefCell;
use std::collections::HashSet;
use std::path::PathBuf;
use std::rc::Rc;

pub struct FileQueueState {
    pub(crate) queue: Rc<RefCell<Vec<PathBuf>>>,
    pub(crate) selected_set: Rc<RefCell<HashSet<usize>>>,
    pub(crate) last_clicked_idx: Rc<RefCell<Option<usize>>>,
    pub(crate) selected_index: Rc<RefCell<Option<usize>>>,
    pub(crate) focused_panel: Rc<RefCell<FocusedPanel>>,
    pub(crate) selected_file: Rc<RefCell<Option<PathBuf>>>,
}

impl FileQueueState {
    pub(crate) fn show(&self, ui: &mut egui::Ui, viewer: &mut MyTabViewer) {
        ui.horizontal(|ui| {
            if ui.button("Clear Queue").clicked() {
                self.queue.borrow_mut().clear();
                self.selected_set.borrow_mut().clear();
                *self.selected_index.borrow_mut() = None;
            }
            if ui.button("Remove Selected").clicked() {
                let mut indices: Vec<usize> = self.selected_set.borrow().iter().cloned().collect();
                if !indices.is_empty() {
                    indices.sort_unstable();
                    indices.reverse();
                    let mut queue = self.queue.borrow_mut();
                    for idx in indices {
                        if idx < queue.len() {
                            queue.remove(idx);
                        }
                    }
                    self.selected_set.borrow_mut().clear();
                    *self.selected_index.borrow_mut() = None;
                } else {
                    let idx_to_remove = self.selected_index.borrow().clone();
                    if let Some(idx) = idx_to_remove {
                        let mut queue = self.queue.borrow_mut();
                        if idx < queue.len() {
                            queue.remove(idx);
                            *self.selected_index.borrow_mut() = None;
                        }
                    }
                }
            }
            if ui.button(" Link to Source ").clicked() {
                let queue = self.queue.borrow();
                let paths: Vec<PathBuf> = queue.iter().cloned().collect();
                if !paths.is_empty() {
                    let _ = viewer.link_tx.send(paths);
                }
            }
            if ui.button(" Silence ").clicked() {
                let queue = self.queue.borrow();
                let paths: Vec<PathBuf> = queue.iter().cloned().collect();
                if !paths.is_empty() {
                    let _ = viewer.silence_tx.send(paths);
                }
            }
            if ui.button(" Export OGG (slow) ").clicked() {
                let queue = self.queue.borrow();
                let paths: Vec<PathBuf> = queue.iter().cloned().collect();
                if !paths.is_empty() {
                    let _ = viewer.export_tx.send(paths);
                }
            }
        });
        ui.separator();
        egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
            let queue = self.queue.borrow();
            for (i, path) in queue.iter().enumerate() {
                let display = display_name(path);
                let is_selected = self.selected_set.borrow().contains(&i);
                let mut button = egui::Button::new(display);
                if is_selected {
                    button = button.fill(ui.style().visuals.selection.bg_fill);
                }
                if ui.add(button).clicked() {
                    let modifiers = ui.input(|i| i.modifiers);
                    let mut sel = self.selected_set.borrow_mut();
                    let mut last = self.last_clicked_idx.borrow_mut();
                    if modifiers.ctrl {
                        if sel.contains(&i) {
                            sel.remove(&i);
                        } else {
                            sel.insert(i);
                        }
                        *last = Some(i);
                    } else if modifiers.shift {
                        if let Some(anchor) = *last {
                            let start = anchor.min(i);
                            let end = anchor.max(i);
                            for idx in start..=end {
                                sel.insert(idx);
                            }
                        } else {
                            sel.clear();
                            sel.insert(i);
                            *last = Some(i);
                        }
                    } else {
                        sel.clear();
                        sel.insert(i);
                        *last = Some(i);
                    }
                    if let Some(path) = queue.get(i) {
                        *self.selected_file.borrow_mut() = Some(path.clone());
                    }
                    *self.focused_panel.borrow_mut() = FocusedPanel::FileQueue;
                }
            }
        });
    }
}