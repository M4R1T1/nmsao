use super::MyApp;
use crate::models::{InMemoryEntry, StreamedEntry};
use crate::wwise::convert_wem_to_ogg;
use eframe::egui;
use std::path::{Path, PathBuf};

impl MyApp {

    // converts sound files in file queue from wem to ogg
    // optionally preserves organized audio folder structure depending on config.json
    pub(super) fn export_audio(&mut self, paths: Vec<PathBuf>) {
        let preserve = self.config.borrow().preserve_export_structure;
        let export_root = self.exported_dir.join("exported_ogg");
        if let Err(e) = std::fs::create_dir_all(&export_root) {
            self.log_and_status(&format!("Failed to create export folder: {}", e));
            return;
        }
        let mut ok = 0;
        let mut failed = 0;
        for path in &paths {
            let out_path = if preserve {
                match self.relative_path_for(path) {
                    Some(rel) => {
                        let mut dest = export_root.clone();
                        for component in rel.split('\\') {
                            dest.push(component);
                        }
                        dest.pop();
                        let stem = path.file_stem().unwrap_or_default().to_string_lossy();
                        dest.push(format!("{}.ogg", stem));
                        dest
                    }
                    None => export_root
                        .join(path.file_name().unwrap_or_default())
                        .with_extension("ogg"),
                }
            } else {
                export_root
                    .join(path.file_name().unwrap_or_default())
                    .with_extension("ogg")
            };
            match convert_wem_to_ogg(path, &out_path) {
                Ok(_) => ok += 1,
                Err(e) => {
                    self.log_and_status(&format!("Failed to convert {:?}: {}", path.file_name().unwrap_or_default(), e));
                    failed += 1;
                }
            }
        }
        self.log_and_status(&format!(
            "Exported {} OGG files{}",
            ok,
            if failed > 0 {
                format!(", {} failed", failed)
            } else {
                String::new()
            }
        ));
    }

    // wwise relative path lookup for an organized audio path
    pub(super) fn relative_path_for(&self, destination: &Path) -> Option<String> {
        let streamed: Vec<StreamedEntry> =
            self.load_json(&self.logs_dir.join("streamed_index.json"));
        if let Some(e) = streamed.iter().find(|e| e.destination_path == destination) {
            return Some(e.relative_path.clone());
        }
        let in_mem: Vec<InMemoryEntry> =
            self.load_json(&self.logs_dir.join("in_memory_index.json"));
            in_mem.iter().find(|e| e.destination_path == destination).map(|e| e.relative_path.clone())
    }

    // copies organized audio sound file id to clipboard and reports its basic attributes in status bar
    pub(super) fn copy_id_for_path(&mut self, ctx: &egui::Context, path: PathBuf) {
        let streamed: Vec<StreamedEntry> =
            self.load_json(&self.logs_dir.join("streamed_index.json"));
        if let Some(e) = streamed.iter().find(|e| e.destination_path == path) {
            ctx.copy_text(e.id.to_string());
            self.log_and_status(&format!("Copied '{}' to clipboard (Streamed)", e.id));
            return;
        }
        let in_memory: Vec<InMemoryEntry> =
            self.load_json(&self.logs_dir.join("in_memory_index.json"));
        if let Some(e) = in_memory.iter().find(|e| e.destination_path == path) {
            ctx.copy_text(e.id.to_string());
            let bnk_name = e
                .source_bnk_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
        self.log_and_status(&format!("Copied '{}' to clipboard (In Memory) ({})", e.id, bnk_name));
        return;
        }
        self.log_and_status("Could not find Id for this file");
    }
}