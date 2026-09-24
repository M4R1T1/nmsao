use super::MyApp;
use crate::assets::{
    get_bnk_sizes, get_folder_total_size, get_non_empty_bnks,
    process_audio_assets, process_in_memory_assets,
};
use crate::config::{load_manifest, save_manifest};
use crate::logging::log_message;
use crate::tree::build_roots_from_path;
use eframe::egui;
use std::collections::HashMap;
use std::path::PathBuf;

impl MyApp {

    // starts asset processing thread
    pub(super) fn start_processing(&mut self, source: PathBuf, dest: PathBuf, logs: PathBuf, ctx: &egui::Context) {
        let (tx, rx) = std::sync::mpsc::channel();
        self.progress_rx = Some(rx);
        self.processing_handle = Some(std::thread::spawn(move || {
            let mut manifest = load_manifest(&logs);
            let media_dir = source.join("media");
            let current_streamed_size = if media_dir.exists() {
                get_folder_total_size(&media_dir)
            } else {
                0
            };
            let current_bnk_sizes = get_bnk_sizes(&source);
            let streamed_changed = current_streamed_size != manifest.streamed_media_total_size;
            let non_empty_bnks = get_non_empty_bnks(&source);
            let filtered_bnk_sizes: HashMap<PathBuf, u64> = current_bnk_sizes
                .into_iter()
                .filter(|(path, _)| non_empty_bnks.contains(path))
                .collect();
            let in_memory_changed = filtered_bnk_sizes != manifest.bnk_files;
            if streamed_changed {
                let _ = tx.send("[Streamed] Media folder size changed, processing...".to_string());
                log_message("[Streamed] Media folder size changed, processing...");
                match process_audio_assets(&source, &dest, &logs, |msg| {
                    let full_msg = format!("[Streamed] {}", msg);
                    let _ = tx.send(full_msg.clone());
                    log_message(&full_msg);
                }) {
                    Ok(_) => {
                        manifest.streamed_media_total_size = current_streamed_size;
                        save_manifest(&logs, &manifest);
                        let _ = tx.send("[Streamed] Processing done, manifest updated".to_string());
                        log_message("[Streamed] Processing done, manifest updated");
                    }
                    Err(e) => {
                        let err_msg = format!("ERROR (Streamed): {}", e);
                        let _ = tx.send(err_msg.clone());
                        log_message(&err_msg);
                    }
                }
            } else {
                let _ = tx.send("[Streamed] No change, skipping".to_string());
                log_message("[Streamed] No change, skipping");
            }
            if in_memory_changed {
                let _ = tx.send("[InMemory] BNK sizes changed, processing...".to_string());
                log_message("[InMemory] BNK sizes changed, processing...");
                match process_in_memory_assets(&source, &dest, &logs, |msg| {
                    let full_msg = format!("[InMemory] {}", msg);
                    let _ = tx.send(full_msg.clone());
                    log_message(&full_msg);
                }) {
                    Ok(_) => {
                        manifest.bnk_files = filtered_bnk_sizes;
                        save_manifest(&logs, &manifest);
                        let _ = tx.send("[InMemory] Processing done, manifest updated".to_string());
                        log_message("[InMemory] Processing done, manifest updated");
                    }
                    Err(e) => {
                        let err_msg = format!("ERROR (InMemory): {}", e);
                        let _ = tx.send(err_msg.clone());
                        log_message(&err_msg);
                    }
                }
            } else {
                let _ = tx.send("[InMemory] No change, skipping".to_string());
                log_message("[InMemory] No change, skipping");
            }
            let _ = tx.send("PROCESSING_COMPLETED".to_string());
            log_message("PROCESSING_COMPLETED");
        }));
        ctx.request_repaint();
    }

    // rebuilds organized tree from current output_dir
    pub(super) fn refresh_organized_tree(&mut self) {
        let (new_roots, file_list) = build_roots_from_path(&self.output_dir);
        *self.organized_tree.roots.borrow_mut() = new_roots;
        *self.organized_tree.file_list.borrow_mut() = file_list;
        self.organized_tree.selected_set.borrow_mut().clear();
        *self.organized_tree.last_clicked_idx.borrow_mut() = None;
    }

    // opens clean audio confirmation dialog
    pub(super) fn clean_organized_audio(&mut self) {
        self.show_clean_audio_dialog = true;
        self.clean_audio_confirmation.clear();
        self.clean_audio_error = None;
        *self.new_project_dialog_open.borrow_mut() = true;
    }

    // deletes organized tree and its index files
    pub(super) fn try_clean_organized_audio(&mut self) -> bool {
        if self.clean_audio_confirmation.trim() != "DELETE" {
            self.clean_audio_error = Some("Type DELETE to confirm".to_string());
            return false;
        }
        let mut success = true;
        if let Ok(entries) = std::fs::read_dir(&self.output_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Err(e) = std::fs::remove_dir_all(&path) {
                        self.log_and_status(&format!("Failed to remove folder {:?}: {}", path, e));
                        success = false;
                    }
                } else {
                    if let Err(e) = std::fs::remove_file(&path) {
                        self.log_and_status(&format!("Failed to remove file {:?}: {}", path, e));
                        success = false;
                    }
                }
            }
        } else {
            self.log_and_status("Could not read organized_audio folder");
            success = false;
        }
        let files_to_delete = [
            self.logs_dir.join("asset_sizes.json"),
            self.logs_dir.join("in_memory_index.json"),
            self.logs_dir.join("streamed_index.json"),
        ];
        for file_path in files_to_delete {
            if file_path.exists() {
                if let Err(e) = std::fs::remove_file(&file_path) {
                    self.log_and_status(&format!("Failed to remove {:?}: {}", file_path, e));
                    success = false;
                }
            }
        }
        if success {
            self.log_and_status("Organized audio cleaned successfully");
            self.refresh_organized_tree();
            self.show_clean_audio_dialog = false;
            self.clean_audio_confirmation.clear();
            self.clean_audio_error = None;
            *self.new_project_dialog_open.borrow_mut() = false;
            true
        } else {
            self.log_and_status("Cleanup completed with errors");
            false
        }
    }
}