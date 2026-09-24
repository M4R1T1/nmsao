use super::MyTabViewer;
use crate::tree::{draw_tree, OrganizedTree};
use eframe::egui;
use std::path::PathBuf;

pub(crate) fn show_organized_tree_tab(ui: &mut egui::Ui, tree: &OrganizedTree, viewer: &mut MyTabViewer) {
    ui.horizontal(|ui| {
        if ui.button("Queue Sounds").clicked() {
            let selected = tree.selected_set.borrow();
            let mut queue = tree.queue.borrow_mut();
            for path in selected.iter() {
                if !queue.contains(path) {
                    queue.push(path.clone());
                }
            }
        }
        if ui.button(" Link to Source ").clicked() {
            let paths: Vec<PathBuf> = tree.selected_set.borrow().iter().cloned().collect();
            let _ = viewer.link_tx.send(paths);
        }
        if ui.button(" Silence ").clicked() {
            let paths: Vec<PathBuf> = tree.selected_set.borrow().iter().cloned().collect();
            if !paths.is_empty() {
                let _ = viewer.silence_tx.send(paths);
            }
        }
    });
    ui.separator();
    egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            draw_tree(ui, tree, &viewer.copy_id_tx);
        });
}