use super::MyTabViewer;
use crate::models::{
    LinkedInMemoryEntry, LinkedStreamedEntry, PendingClear, CLEAR_AUTO_REVERT_SECS,
};
use crate::utils::{assign_source_keys, source_file_display};
use crate::models::clear_button_label;
use eframe::egui;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

pub struct ProjectState;

pub(crate) fn show(ui: &mut egui::Ui, viewer: &mut MyTabViewer) {
    let expired = viewer
        .pending_source_clear
        .borrow()
        .as_ref()
        .map(|pc| pc.started.elapsed().as_secs_f32() >= CLEAR_AUTO_REVERT_SECS)
        .unwrap_or(false);
    if expired {
        *viewer.pending_source_clear.borrow_mut() = None;
    }
    let mut projects = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&viewer.projects_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                projects.push(path);
            }
        }
    }
    let selected = viewer.selected_project.borrow().clone();
    egui::ComboBox::from_label("")
        .selected_text(selected.as_ref().map(|p| p.file_name().unwrap_or_default().to_string_lossy().to_string()).unwrap_or_else(|| "None".to_string()))
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut *viewer.selected_project.borrow_mut(), None, "None");
            for proj in projects {
                let name = proj.file_name().unwrap_or_default().to_string_lossy().to_string();
                ui.selectable_value(&mut *viewer.selected_project.borrow_mut(), Some(proj), name);
            }
        });
    let selected = viewer.selected_project.borrow().clone();
    if selected.is_none() {
        ui.label("No project selected");
        return;
    }
    let project_path = selected.unwrap();
    let streamed_map_path = project_path.join("streamed_map.json");
    let in_memory_map_path = project_path.join("in_memory_map.json");
    let streamed_entries: Vec<LinkedStreamedEntry> = if streamed_map_path.exists() {
        std::fs::read_to_string(&streamed_map_path).ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    let in_memory_entries: Vec<LinkedInMemoryEntry> = if in_memory_map_path.exists() {
        std::fs::read_to_string(&in_memory_map_path).ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    fn sorted_groups<T, F, G>(entries: Vec<T>, key: F, sort_key: G) -> Vec<(PathBuf, Vec<T>)>
    where
        F: Fn(&T) -> PathBuf,
        G: Fn(&T) -> String,
    {
        let mut map: HashMap<PathBuf, Vec<T>> = HashMap::new();
        for entry in entries {
            let src = key(&entry);
            map.entry(src).or_default().push(entry);
        }
        let mut groups: Vec<(PathBuf, Vec<T>)> = map.into_iter().collect();
        groups.sort_by(|a, b| {
            a.0.file_name()
            .cmp(&b.0.file_name())
            .then_with(|| a.0.cmp(&b.0))
        });
        for (_, entries) in &mut groups {
            entries.sort_by(|a, b| sort_key(a).cmp(&sort_key(b)));
        }
        groups
    }
    let streamed_paths: Vec<PathBuf> = streamed_entries.iter().map(|e| e.source_file_path.clone()).collect();
    let streamed_source_keys = assign_source_keys(&streamed_paths);
    let inmemory_paths: Vec<PathBuf> = in_memory_entries.iter().map(|e| e.source_file_path.clone()).collect();
    let inmemory_source_keys = assign_source_keys(&inmemory_paths);
    let streamed_groups = sorted_groups(streamed_entries, |e| e.source_file_path.clone(), |e| e.name.clone());
    let in_memory_groups = sorted_groups(in_memory_entries, |e| e.source_file_path.clone(), |e| e.name.clone());
    let mut rendered: Vec<(bool, PathBuf)> = Vec::new();
    egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
        if !streamed_groups.is_empty() {
            ui.heading("Streamed");
            for (src_path, entries) in streamed_groups {
                let src_name = streamed_source_keys.get(&src_path).cloned().unwrap_or_else(|| source_file_display(&src_path));
                let ids: Vec<u32> = entries.iter().map(|e| e.id).collect();
                let label = {
                    let borrow = viewer.pending_source_clear.borrow();
                    clear_button_label(borrow.as_ref(), true, &src_path)
                };
                rendered.push((true, src_path.clone()));
                let mut clicked = false;
                ui.horizontal(|ui| {
                    ui.label(format!("📁 {}", src_name));
                    if ui.button(label).clicked() {
                        clicked = true;
                    }
                });
                if clicked {
                    let elapsed = {
                        let borrow = viewer.pending_source_clear.borrow();
                        borrow.as_ref()
                            .filter(|pc| pc.is_streamed && pc.source_path == src_path)
                            .map(|pc| pc.started.elapsed().as_secs_f32())
                    };
                    match elapsed {
                        None => {
                            *viewer.pending_source_clear.borrow_mut() = Some(PendingClear {
                                is_streamed: true,
                                source_path: src_path.clone(),
                                started: Instant::now(),
                            });
                        }
                        Some(e) if e < 2.0 => {
                            *viewer.pending_source_clear.borrow_mut() = None;
                        }
                        Some(_) => {
                            for id in ids {
                                let _ = viewer.remove_tx.send(("streamed".to_string(), id));
                            }
                            *viewer.pending_source_clear.borrow_mut() = None;
                        }
                    }
                }
                ui.indent("streamed_indent", |ui| {
                    for entry in entries {
                        ui.horizontal(|ui| {
                            if ui.button("❌").clicked() {
                                let _ = viewer.remove_tx.send(("streamed".to_string(), entry.id));
                            }
                            ui.label(format!("  --->  {}", entry.name));
                        });
                    }
                });
            }
            ui.separator();
        }
        if !in_memory_groups.is_empty() {
            ui.heading("In Memory");
            for (src_path, entries) in in_memory_groups {
                let src_name = inmemory_source_keys.get(&src_path).cloned().unwrap_or_else(|| source_file_display(&src_path));
                let ids: Vec<u32> = entries.iter().map(|e| e.id).collect();
                let label = {
                    let borrow = viewer.pending_source_clear.borrow();
                    clear_button_label(borrow.as_ref(), false, &src_path)
                };
                rendered.push((false, src_path.clone()));
                let mut clicked = false;
                ui.horizontal(|ui| {
                    ui.label(format!("📁 {}", src_name));
                    if ui.button(label).clicked() {
                        clicked = true;
                    }
                });
                if clicked {
                    let elapsed = {
                        let borrow = viewer.pending_source_clear.borrow();
                        borrow.as_ref()
                            .filter(|pc| !pc.is_streamed && pc.source_path == src_path)
                            .map(|pc| pc.started.elapsed().as_secs_f32())
                    };
                    match elapsed {
                        None => {
                            *viewer.pending_source_clear.borrow_mut() = Some(PendingClear {
                                is_streamed: false,
                                source_path: src_path.clone(),
                                started: Instant::now(),
                            });
                        }
                        Some(e) if e < 2.0 => {
                            *viewer.pending_source_clear.borrow_mut() = None;
                        }
                        Some(_) => {
                            for id in ids {
                                let _ = viewer.remove_tx.send(("in_memory".to_string(), id));
                            }
                            *viewer.pending_source_clear.borrow_mut() = None;
                        }
                    }
                }
                ui.indent("inmemory_indent", |ui| {
                    for entry in entries {
                        ui.horizontal(|ui| {
                            if ui.button("❌").clicked() {
                                let _ = viewer.remove_tx.send(("in_memory".to_string(), entry.id));
                            }
                            ui.label(format!("  --->  {}", entry.name));
                        });
                    }
                });
            }
        }
    });
    let should_prune = {
        let borrow = viewer.pending_source_clear.borrow();
        match borrow.as_ref() {
            Some(pc) => !rendered.contains(&(pc.is_streamed, pc.source_path.clone())),
            None => false,
        }
    };
    if should_prune {
        *viewer.pending_source_clear.borrow_mut() = None;
    }
    if viewer.pending_source_clear.borrow().is_some() {
        ui.ctx().request_repaint();
    }
}