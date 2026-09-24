use super::MyApp;
use crate::logging::log_message;
use crate::models::{
    BuildEntry, BuildManifest, InMemoryEntry, LinkedInMemoryEntry,
    LinkedStreamedEntry, StreamedEntry,
};
use crate::soundbank::SoundBank;
use crate::tree::is_linkable_file;
use crate::utils::{assign_source_keys, resolve_imported_source, source_file_display};
use crate::wwise::{convert_wav_to_wem, convert_wav_to_wem_bytes, create_silence_wem};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

impl MyApp {

    // creates a new project folder with empty map json files
    pub(super) fn try_create_project(&mut self) -> bool {
        let name = self.new_project_name.trim().to_string();
        if name.is_empty() {
            self.project_creation_error = Some("Name cannot be empty".to_string());
            return false;
        }
        let project_path = self.projects_dir.join(&name);
        if project_path.exists() {
            self.project_creation_error = Some("Project already exists".to_string());
            return false;
        }
        match std::fs::create_dir_all(&project_path) {
            Ok(_) => {
                let streamed_map_path = project_path.join("streamed_map.json");
                let in_memory_map_path = project_path.join("in_memory_map.json");
                let empty_array = "[]";
                let result1 = std::fs::write(&streamed_map_path, empty_array);
                let result2 = std::fs::write(&in_memory_map_path, empty_array);
                if result1.is_err() || result2.is_err() {
                    let err_msg = format!("Failed to create JSON map files: {:?}, {:?}", result1.err(), result2.err());
                    self.project_creation_error = Some(err_msg);
                    return false;
                }
                self.show_new_project_dialog = false;
                *self.new_project_dialog_open.borrow_mut() = false;
                self.new_project_name.clear();
                self.project_creation_error = None;
                self.log_and_status(&format!("Project '{}' created", name));
                true
            }
            Err(e) => {
                let err_msg = format!("Failed to create project: {}", e);
                self.project_creation_error = Some(err_msg.clone());
                log_message(&format!("Project creation error: {}", err_msg));
                false
            }
        }
    }

    // links the selected organized audio file with selected source file
    pub(super) fn link_to_source(&mut self, audio_paths: Vec<PathBuf>) {
        let source_path = self.selected_file.borrow().clone();
        if source_path.is_none() {
            self.log_and_status("No source file selected");
            return;
        }
        let source_path = source_path.unwrap();
        if !is_linkable_file(&source_path) {
            self.log_and_status("Invalid source file type");
            return;
        }
        let selected_project = self.selected_project.borrow().clone();
        if selected_project.is_none() {
            self.log_and_status("No project selected");
            return;
        }
        let project_path = selected_project.unwrap();
        let streamed_index: Vec<StreamedEntry> = self.load_json(&self.logs_dir.join("streamed_index.json"));
        let in_memory_index: Vec<InMemoryEntry> = self.load_json(&self.logs_dir.join("in_memory_index.json"));
        let streamed_map_path = project_path.join("streamed_map.json");
        let in_memory_map_path = project_path.join("in_memory_map.json");
        let mut streamed_map: Vec<LinkedStreamedEntry> = self.load_json(&streamed_map_path);
        let mut in_memory_map: Vec<LinkedInMemoryEntry> = self.load_json(&in_memory_map_path);
        for audio_path in audio_paths {
            if let Some(entry) = streamed_index.iter().find(|e| e.destination_path == audio_path) {
                streamed_map.retain(|e| e.id != entry.id);
                streamed_map.push(LinkedStreamedEntry {
                    source_file_path: source_path.clone(),
                    id: entry.id,
                    name: entry.name.clone(),
                    generated_file: entry.generated_file.clone(),
                    relative_path: entry.relative_path.clone(),
                });
            } else if let Some(entry) = in_memory_index.iter().find(|e| e.destination_path == audio_path) {
                in_memory_map.retain(|e| e.id != entry.id);
                in_memory_map.push(LinkedInMemoryEntry {
                    source_file_path: source_path.clone(),
                    id: entry.id,
                    name: entry.name.clone(),
                    relative_path: entry.relative_path.clone(),
                    source_bnk_path: entry.source_bnk_path.clone(),
                    original_wem_name: entry.original_wem_name.clone(),
                    bnk_relative_path: entry.bnk_relative_path.clone(),
                });
            } else {
                self.log_and_status(&format!("No index entry for {:?}", audio_path));
            }
        }
        if !streamed_map.is_empty() {
            std::fs::write(&streamed_map_path, serde_json::to_string_pretty(&streamed_map).unwrap()).ok();
        }
        if !in_memory_map.is_empty() {
            std::fs::write(&in_memory_map_path, serde_json::to_string_pretty(&in_memory_map).unwrap()).ok();
        }
        self.refresh_linked_paths();
        self.log_and_status("Linked to source");
    }

    // replaces the source file path with the SILENCE entry
    pub(super) fn add_silence_entries(&mut self, audio_paths: Vec<PathBuf>) {
        let selected_project = self.selected_project.borrow().clone();
        if selected_project.is_none() {
            self.log_and_status("No project selected");
            return;
        }
        let project_path = selected_project.unwrap();
        let streamed_index: Vec<StreamedEntry> = self.load_json(&self.logs_dir.join("streamed_index.json"));
        let in_memory_index: Vec<InMemoryEntry> = self.load_json(&self.logs_dir.join("in_memory_index.json"));
        let streamed_map_path = project_path.join("streamed_map.json");
        let in_memory_map_path = project_path.join("in_memory_map.json");
        let mut streamed_map: Vec<LinkedStreamedEntry> = self.load_json(&streamed_map_path);
        let mut in_memory_map: Vec<LinkedInMemoryEntry> = self.load_json(&in_memory_map_path);
        for audio_path in audio_paths {
            if let Some(entry) = streamed_index.iter().find(|e| e.destination_path == audio_path) {
                streamed_map.retain(|e| e.id != entry.id);
                streamed_map.push(LinkedStreamedEntry {
                    source_file_path: PathBuf::from("SILENCE"),
                    id: entry.id,
                    name: entry.name.clone(),
                    generated_file: entry.generated_file.clone(),
                    relative_path: entry.relative_path.clone(),
                });
            } else if let Some(entry) = in_memory_index.iter().find(|e| e.destination_path == audio_path) {
                in_memory_map.retain(|e| e.id != entry.id);
                in_memory_map.push(LinkedInMemoryEntry {
                    source_file_path: PathBuf::from("SILENCE"),
                    id: entry.id,
                    name: entry.name.clone(),
                    relative_path: entry.relative_path.clone(),
                    source_bnk_path: entry.source_bnk_path.clone(),
                    original_wem_name: entry.original_wem_name.clone(),
                    bnk_relative_path: entry.bnk_relative_path.clone(),
                });
            } else {
                self.log_and_status(&format!("No index entry for {:?}", audio_path));
            }
        }
        if !streamed_map.is_empty() {
            std::fs::write(&streamed_map_path, serde_json::to_string_pretty(&streamed_map).unwrap()).ok();
        }
        if !in_memory_map.is_empty() {
            std::fs::write(&in_memory_map_path, serde_json::to_string_pretty(&in_memory_map).unwrap()).ok();
        }
        self.refresh_linked_paths();
        self.log_and_status("Silence linked");
    }

    // removes one singular linked sound from either streamed or in memory project map
    pub(super) fn remove_from_project_map(&mut self, map_type: String, id: u32) {
        let selected_project = self.selected_project.borrow().clone();
        if selected_project.is_none() {
            return;
        }
        let project_path = selected_project.unwrap();
        let map_path = if map_type == "streamed" {
            project_path.join("streamed_map.json")
        } else {
            project_path.join("in_memory_map.json")
        };
        if !map_path.exists() { return; }
        if map_type == "streamed" {
            let mut map: Vec<LinkedStreamedEntry> = self.load_json(&map_path);
            map.retain(|e| e.id != id);
            std::fs::write(&map_path, serde_json::to_string_pretty(&map).unwrap()).ok();
        } else {
            let mut map: Vec<LinkedInMemoryEntry> = self.load_json(&map_path);
            map.retain(|e| e.id != id);
            std::fs::write(&map_path, serde_json::to_string_pretty(&map).unwrap()).ok();
        }
    }

    // renames selected project
    pub(super) fn try_rename_project(&mut self) -> bool {
        let old_path = match self.rename_old_path.clone() {
            Some(p) => p,
            None => {
                self.rename_error = Some("No project selected".to_string());
                return false;
            }
        };
        let new_name = self.rename_new_name.trim().to_string();
        if new_name.is_empty() {
            self.rename_error = Some("Name cannot be empty".to_string());
            return false;
        }
        let parent = old_path.parent().unwrap_or(&self.projects_dir);
        let new_path = parent.join(&new_name);
        if new_path.exists() {
            self.rename_error = Some("Project already exists".to_string());
            return false;
        }
        match std::fs::rename(&old_path, &new_path) {
            Ok(_) => {
                *self.selected_project.borrow_mut() = Some(new_path.clone());
                self.show_rename_dialog = false;
                self.rename_old_path = None;
                self.rename_new_name.clear();
                self.rename_error = None;
                self.log_and_status(&format!("Project renamed to '{}'", new_name));
                true
            }
            Err(e) => {
                self.rename_error = Some(format!("Failed to rename: {}", e));
                false
            }
        }
    }

    // deletes selected project with confirmation dialog
    pub(super) fn try_delete_project(&mut self) -> bool {
        let project = self.selected_project.borrow().clone();
        if project.is_none() {
            self.delete_error = Some("No project selected".to_string());
            return false;
        }
        if self.delete_confirmation.trim() != "DELETE" {
            self.delete_error = Some("Type DELETE to confirm".to_string());
            return false;
        }
        let path = project.unwrap();
        match std::fs::remove_dir_all(&path) {
            Ok(_) => {
                *self.selected_project.borrow_mut() = None;
                self.show_delete_dialog = false;
                self.delete_confirmation.clear();
                self.delete_error = None;
                self.log_and_status("Project deleted");
                true
            }
            Err(e) => {
                self.delete_error = Some(format!("Failed to delete: {}", e));
                false
            }
        }
    }

    // builds exported mod with streamed wem files, bnk, and mapping.json (streamed and in memory where applicable)
    pub(super) fn build_mod(&mut self) {
        let project = self.selected_project.borrow().clone();
        if project.is_none() {
            self.log_and_status("No project selected");
            return;
        }
        let project_path = project.unwrap();
        let project_name = project_path.file_name().unwrap_or_default().to_string_lossy().to_string();
        let streamed_map_path = project_path.join("streamed_map.json");
        let in_memory_map_path = project_path.join("in_memory_map.json");
        let streamed_entries: Vec<LinkedStreamedEntry> = self.load_json(&streamed_map_path);
        let in_memory_entries: Vec<LinkedInMemoryEntry> = self.load_json(&in_memory_map_path);
        let base_dir = self.exported_dir.join(&project_name).join("AUDIO").join("WINDOWS");
        let mut created_any = false;
        if !streamed_entries.is_empty() {
            let media_dir = base_dir.join("MEDIA");
            if let Err(e) = std::fs::create_dir_all(&media_dir) {
                self.log_and_status(&format!("Failed to create MEDIA folder: {}", e));
                return;
            }
            let mut converted_count = 0;
            for entry in &streamed_entries {
                let output_path = media_dir.join(&entry.generated_file);
                if entry.source_file_path == PathBuf::from("SILENCE") {
                    let silence_source = self.deps_dir.join("Silence1ms.wem");
                    if !silence_source.exists() {
                        if let Err(e) = create_silence_wem(&self.deps_dir) {
                            self.log_and_status(&format!("Failed to create Silence1ms.wem: {}", e));
                            continue;
                        }
                    }
                    if let Err(e) = std::fs::copy(&silence_source, &output_path) {
                        self.log_and_status(&format!("Failed to copy silence wem: {}", e));
                        continue;
                    }
                    converted_count += 1;
                    continue;
                }
                let source_path = &entry.source_file_path;
                if !source_path.exists() {
                    self.log_and_status(&format!("Source file missing: {:?}", source_path));
                    continue;
                }
                if source_path.extension().and_then(|e| e.to_str()) == Some("wem") {
                    if let Err(e) = std::fs::copy(source_path, &output_path) {
                        self.log_and_status(&format!("Failed to copy existing WEM {}: {}", entry.name, e));
                        continue;
                    }
                    converted_count += 1;
                } else {
                    if let Err(e) = convert_wav_to_wem(
                        &source_path.to_string_lossy(),
                        &output_path.to_string_lossy(),
                    ) {
                        self.log_and_status(&format!("Conversion failed for {}: {}", entry.name, e));
                        continue;
                    }
                    converted_count += 1;
                }
            }
            if converted_count > 0 {
                self.log_and_status(&format!("Converted {} streamed WAV files to WEM", converted_count));
            } else {
                self.log_and_status("No streamed files converted");
            }
            created_any = true;
        }
        if !in_memory_entries.is_empty() {
            if let Err(e) = std::fs::create_dir_all(&base_dir) {
                self.log_and_status(&format!("Failed to create WINDOWS folder: {}", e));
                return;
            }
            created_any = true;
            let mut groups: HashMap<PathBuf, Vec<&LinkedInMemoryEntry>> = HashMap::new();
            for entry in &in_memory_entries {
                groups.entry(entry.source_bnk_path.clone()).or_default().push(entry);
            }
            for (bnk_path, entries) in groups {
                let mut bank = match SoundBank::from_file(&bnk_path) {
                    Ok(b) => b,
                    Err(e) => {
                        self.log_and_status(&format!("Failed to load BNK {:?}: {}", bnk_path, e));
                        continue;
                    }
                };
                let rel_path = entries.first().map(|e| e.bnk_relative_path.clone()).unwrap_or_default();
                for entry in entries {
                    let wem_data = if entry.source_file_path == PathBuf::from("SILENCE") {
                        let silence_path = self.deps_dir.join("Silence1ms.wem");
                        if !silence_path.exists() {
                            if let Err(e) = create_silence_wem(&self.deps_dir) {
                                self.log_and_status(&format!("Failed to create silence WEM: {}", e));
                                continue;
                            }
                        }
                        match std::fs::read(&silence_path) {
                            Ok(b) => b,
                            Err(e) => {
                                self.log_and_status(&format!("Failed to read silence WEM: {}", e));
                                continue;
                            }
                        }
                    } else {
                        let src = &entry.source_file_path;
                        if !src.exists() {
                            self.log_and_status(&format!("Source file missing: {:?}", src));
                            continue;
                        }
                        if src.extension().and_then(|e| e.to_str()) == Some("wem") {
                            match std::fs::read(src) {
                                Ok(b) => b,
                                Err(e) => {
                                    self.log_and_status(&format!("Failed to read source WEM: {}", e));
                                    continue;
                                }
                            }
                        } else {
                            let wav_bytes = match std::fs::read(src) {
                                Ok(b) => b,
                                Err(e) => {
                                    self.log_and_status(&format!("Failed to read source WAV: {}", e));
                                    continue;
                                }
                            };
                            match convert_wav_to_wem_bytes(&wav_bytes) {
                                Ok(data) => data,
                                Err(e) => {
                                    self.log_and_status(&format!("Conversion failed for {}: {}", entry.name, e));
                                    continue;
                                }
                            }
                        }
                    };
                    if let Err(e) = bank.replace_wem(entry.id, &wem_data) {
                        self.log_and_status(&format!("Replace failed for ID {}: {}", entry.id, e));
                    }
                }
                let output_path = base_dir.join(rel_path);
                if let Some(parent) = output_path.parent() {
                    if let Err(e) = std::fs::create_dir_all(parent) {
                        self.log_and_status(&format!("Failed to create folders: {}", e));
                        continue;
                    }
                }
                if let Err(e) = bank.write_to_path(&output_path) {
                    self.log_and_status(&format!("Failed to write modified BNK: {}", e));
                } else {
                    self.log_and_status(&format!("Written modified BNK: {:?}", output_path));
                }
            }
        }
        if created_any {
            self.log_and_status(&format!("Build folders created for '{}'", project_name));
        } else {
            self.log_and_status("No entries to build (both maps empty)");
        }
        if !streamed_entries.is_empty() || !in_memory_entries.is_empty() {
            self.write_build_manifest(&project_name, &streamed_entries, &in_memory_entries);
        }
    }

    // writes mapping.json for exported project
    pub(super) fn write_build_manifest(
        &mut self,
        project_name: &str,
        streamed_entries: &[LinkedStreamedEntry],
        in_memory_entries: &[LinkedInMemoryEntry],
    ) {
        let streamed_paths: Vec<PathBuf> = streamed_entries.iter().map(|e| e.source_file_path.clone()).collect();
        let streamed_keys = assign_source_keys(&streamed_paths);
        let inmemory_paths: Vec<PathBuf> = in_memory_entries.iter().map(|e| e.source_file_path.clone()).collect();
        let inmemory_keys = assign_source_keys(&inmemory_paths);
        let mut streamed: BTreeMap<String, Vec<BuildEntry>> = BTreeMap::new();
        for e in streamed_entries {
            let key = streamed_keys.get(&e.source_file_path).cloned().unwrap_or_else(|| source_file_display(&e.source_file_path));
            streamed.entry(key).or_default().push(BuildEntry {
                id: e.id,
                name: e.name.clone(),
                bnk: None,
            });
        }
        let mut in_memory: BTreeMap<String, Vec<BuildEntry>> = BTreeMap::new();
        for e in in_memory_entries {
            let key = inmemory_keys.get(&e.source_file_path).cloned().unwrap_or_else(|| source_file_display(&e.source_file_path));
            let bnk_name = e.source_bnk_path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            in_memory.entry(key).or_default().push(BuildEntry {
                id: e.id,
                name: e.name.clone(),
                bnk: Some(bnk_name),
            });
        }
        for v in streamed.values_mut() {
            v.sort_by_key(|e| e.id);
        }
        for v in in_memory.values_mut() {
            v.sort_by_key(|e| e.id);
        }
        let manifest = BuildManifest {
            project: project_name.to_string(),
            streamed,
            in_memory,
        };
        let manifest_path = self.exported_dir.join(project_name).join("mapping.json");
        if let Some(parent) = manifest_path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                self.log_and_status(&format!("Failed to create manifest folder: {}", e));
                return;
            }
        }
        match serde_json::to_string_pretty(&manifest) {
            Ok(json) => {
                if let Err(e) = std::fs::write(&manifest_path, json) {
                    self.log_and_status(&format!("Failed to write build manifest: {}", e));
                } else {
                    self.log_and_status(&format!("Wrote build manifest: {:?}", manifest_path));
                }
            }
            Err(e) => {
                self.log_and_status(&format!("Failed to serialize build manifest: {}", e));
            }
        }
    }

    // refreshes the set of audio paths linked in the active project
    pub(super) fn refresh_linked_paths(&mut self) {
        let proj = self.selected_project.borrow().clone();
        let mut set = HashSet::new();
        if let Some(ref proj_path) = proj {
            let streamed_map: Vec<LinkedStreamedEntry> = self.load_json(&proj_path.join("streamed_map.json"));
            let in_memory_map: Vec<LinkedInMemoryEntry> = self.load_json(&proj_path.join("in_memory_map.json"));
            let streamed_index: Vec<StreamedEntry> = self.load_json(&self.logs_dir.join("streamed_index.json"));
            let in_memory_index: Vec<InMemoryEntry> = self.load_json(&self.logs_dir.join("in_memory_index.json"));
            let streamed_dest: HashMap<u32, PathBuf> = streamed_index
                .iter()
                .map(|e| (e.id, e.destination_path.clone()))
                .collect();
            let inmemory_dest: HashMap<u32, PathBuf> = in_memory_index
                .iter()
                .map(|e| (e.id, e.destination_path.clone()))
                .collect();
            for e in &streamed_map {
                if let Some(p) = streamed_dest.get(&e.id) {
                    set.insert(p.clone());
                }
            }
            for e in &in_memory_map {
                if let Some(p) = inmemory_dest.get(&e.id) {
                    set.insert(p.clone());
                }
            }
        }
        *self.linked_paths.borrow_mut() = set;
        *self.linked_paths_source.borrow_mut() = proj;
    }

    // combines two or more projects into a new one with first listed taking conflict priority
    pub(super) fn try_combine_projects(&mut self) -> bool {
        let name = self.combine_new_name.trim().to_string();
        let selected: Vec<PathBuf> = self
            .combine_selections
            .iter()
            .filter_map(|s| s.clone())
            .collect();
        if selected.len() < 2 {
            self.combine_error = Some("Select at least two projects".to_string());
            return false;
        }
        let mut seen: HashSet<PathBuf> = HashSet::new();
        for p in &selected {
            if !seen.insert(p.clone()) {
                self.combine_error = Some("Duplicate projects selected".to_string());
                return false;
            }
        }
        if name.is_empty() {
            self.combine_error = Some("Name cannot be empty".to_string());
            return false;
        }
        let project_path = self.projects_dir.join(&name);
        if project_path.exists() {
            self.combine_error = Some("Project already exists".to_string());
            return false;
        }
        let mut streamed: HashMap<u32, LinkedStreamedEntry> = HashMap::new();
        let mut in_memory: HashMap<u32, LinkedInMemoryEntry> = HashMap::new();
        for proj in &selected {
            let s: Vec<LinkedStreamedEntry> = self.load_json(&proj.join("streamed_map.json"));
            for e in s {
                streamed.entry(e.id).or_insert(e);
            }
            let m: Vec<LinkedInMemoryEntry> = self.load_json(&proj.join("in_memory_map.json"));
            for e in m {
                in_memory.entry(e.id).or_insert(e);
            }
        }
        if let Err(e) = std::fs::create_dir_all(&project_path) {
            self.combine_error = Some(format!("Failed to create project: {}", e));
            return false;
        }
        let mut streamed_vec: Vec<LinkedStreamedEntry> = streamed.into_values().collect();
        streamed_vec.sort_by_key(|e| e.id);
        let mut in_memory_vec: Vec<LinkedInMemoryEntry> = in_memory.into_values().collect();
        in_memory_vec.sort_by_key(|e| e.id);
        match serde_json::to_string_pretty(&streamed_vec) {
            Ok(json) => {
                if let Err(e) = std::fs::write(project_path.join("streamed_map.json"), json) {
                    self.combine_error = Some(format!("Failed to write streamed map: {}", e));
                    return false;
                }
            }
            Err(e) => {
                self.combine_error = Some(format!("Failed to serialize streamed map: {}", e));
                return false;
            }
        }
        match serde_json::to_string_pretty(&in_memory_vec) {
            Ok(json) => {
                if let Err(e) = std::fs::write(project_path.join("in_memory_map.json"), json) {
                    self.combine_error = Some(format!("Failed to write in memory map: {}", e));
                    return false;
                }
            }
            Err(e) => {
                self.combine_error = Some(format!("Failed to serialize in memory map: {}", e));
                return false;
            }
        }
        self.show_combine_dialog = false;
        self.combine_selections = vec![None];
        self.combine_new_name.clear();
        self.combine_error = None;
        *self.new_project_dialog_open.borrow_mut() = false;
        self.log_and_status(&format!("Combined project '{}' created", name));
        true
    }

    // imports an external mod as project with mapping.json presence check
    pub(super) fn try_import_project(&mut self) -> bool {
        let path_str = self.import_path.trim().to_string();
        if path_str.is_empty() {
            self.import_error = Some("Path cannot be empty".to_string());
            return false;
        }
        let source = PathBuf::from(&path_str);
        if !source.is_dir() {
            self.import_error = Some(format!("Path does not exist: {}", path_str));
            return false;
        }
        let mapping_path = source.join("mapping.json");
        let manifest: BuildManifest = if mapping_path.exists() {
            match std::fs::read_to_string(&mapping_path) {
                Ok(s) => match serde_json::from_str(&s) {
                    Ok(m) => m,
                    Err(e) => {
                        self.import_error = Some(format!("Failed to parse mapping.json: {}", e));
                        return false;
                    }
                },
                Err(e) => {
                    self.import_error = Some(format!("Failed to read mapping.json: {}", e));
                    return false;
                }
            }
        } else {
            BuildManifest {
                project: source
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default(),
                streamed: BTreeMap::new(),
                in_memory: BTreeMap::new(),
            }
        };
        let project_name = if !manifest.project.trim().is_empty() {
            manifest.project.trim().to_string()
        } else {
            source.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()
        };
        if project_name.is_empty() {
            self.import_error = Some("Could not determine project name".to_string());
            return false;
        }
        let project_path = self.projects_dir.join(&project_name);
        if project_path.exists() {
            self.import_error = Some(format!("Project '{}' already exists", project_name));
            return false;
        }
        let dest_dir = self.imported_dir.join(&project_name);
        if dest_dir.exists() {
            self.import_error = Some(format!("Imported folder '{}' already exists", project_name));
            return false;
        }
        if let Err(e) = std::fs::create_dir_all(&dest_dir) {
            self.import_error = Some(format!("Failed to create import folder: {}", e));
            return false;
        }
        let windows_dir = source.join("AUDIO").join("WINDOWS");
        let media_dir = windows_dir.join("MEDIA");
        let local_in_memory_index: Vec<InMemoryEntry> =
            self.load_json(&self.logs_dir.join("in_memory_index.json"));
        let local_in_memory_by_id: HashMap<u32, InMemoryEntry> = local_in_memory_index
            .into_iter()
            .map(|e| (e.id, e))
            .collect();
        let local_streamed_index: Vec<StreamedEntry> =
            self.load_json(&self.logs_dir.join("streamed_index.json"));
        let local_streamed_by_id: HashMap<u32, StreamedEntry> = local_streamed_index
            .into_iter()
            .map(|e| (e.id, e))
            .collect();
        let mut streamed_map_entries: Vec<LinkedStreamedEntry> = Vec::new();
        let mut in_memory_map_entries: Vec<LinkedInMemoryEntry> = Vec::new();
        let mut used_names: HashSet<String> = HashSet::new();
        let mut copied = 0usize;
        let mut failed = 0usize;
        let mut skipped: Vec<String> = Vec::new();
        let mut source_variants: HashMap<String, Vec<(PathBuf, Vec<u8>)>> = HashMap::new();
        let mut handled_streamed_ids: HashSet<u32> = HashSet::new();
        let mut handled_in_memory_ids: HashSet<u32> = HashSet::new();
        let mut fallback_counter: u32 = 1;
        for (source_key, entries) in &manifest.streamed {
            if source_key == "SILENCE" {
                for entry in entries {
                    streamed_map_entries.push(LinkedStreamedEntry {
                        source_file_path: PathBuf::from("SILENCE"),
                        id: entry.id,
                        name: entry.name.clone(),
                        generated_file: format!("{}.wem", entry.id),
                        relative_path: entry.name.clone(),
                    });
                    handled_streamed_ids.insert(entry.id);
                }
                continue;
            }
            let base_stem = Path::new(source_key)
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| source_key.clone());
            for entry in entries {
                let source_wem = media_dir.join(format!("{}.wem", entry.id));
                let bytes = match std::fs::read(&source_wem) {
                    Ok(b) => b,
                    Err(_) => {
                        skipped.push(format!("missing {}.wem for {}", entry.id, entry.name));
                        continue;
                    }
                };
                if let Some(dest_path) = resolve_imported_source(
                    source_key,
                    || base_stem.clone(),
                    bytes,
                    &dest_dir,
                    &mut used_names,
                    &mut source_variants,
                    &mut copied,
                    &mut failed,
                ) {
                    streamed_map_entries.push(LinkedStreamedEntry {
                        source_file_path: dest_path,
                        id: entry.id,
                        name: entry.name.clone(),
                        generated_file: format!("{}.wem", entry.id),
                        relative_path: entry.name.clone(),
                    });
                    handled_streamed_ids.insert(entry.id);
                }
            }
        }
        let mut external_bnk_cache: HashMap<PathBuf, SoundBank> = HashMap::new();
        let mut local_bnk_cache: HashMap<PathBuf, SoundBank> = HashMap::new();
        for (source_key, entries) in &manifest.in_memory {
            let base_stem = if source_key == "SILENCE" {
                "SILENCE".to_string()
            } else {
                Path::new(source_key)
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| source_key.clone())
            };
            for entry in entries {
                let (local_bnk_path, local_bnk_relative) = match local_in_memory_by_id.get(&entry.id) {
                    Some(e) => (e.source_bnk_path.clone(), e.bnk_relative_path.clone()),
                    None => {
                        skipped.push(format!(
                            "id {} ('{}') not in local in_memory_index",
                            entry.id, entry.name
                        ));
                        continue;
                    }
                };
                if source_key == "SILENCE" {
                    in_memory_map_entries.push(LinkedInMemoryEntry {
                        source_file_path: PathBuf::from("SILENCE"),
                        id: entry.id,
                        name: entry.name.clone(),
                        relative_path: entry.name.clone(),
                        source_bnk_path: local_bnk_path,
                        original_wem_name: format!("{}.wem", entry.id),
                        bnk_relative_path: local_bnk_relative,
                    });
                    handled_streamed_ids.insert(entry.id);
                    continue;
                }
                let external_bnk_name = match &entry.bnk {
                    Some(n) if !n.is_empty() => n.clone(),
                    _ => {
                        skipped.push(format!(
                            "no bnk recorded in mapping.json for '{}'",
                            entry.name
                        ));
                        continue;
                    }
                };
                let ext_path = windows_dir.join(&external_bnk_name);
                if !external_bnk_cache.contains_key(&ext_path) {
                    match SoundBank::from_file(&ext_path) {
                        Ok(b) => { external_bnk_cache.insert(ext_path.clone(), b); }
                        Err(e) => {
                            skipped.push(format!(
                                "failed to open external {}: {}",
                                external_bnk_name, e
                            ));
                            continue;
                        }
                    }
                }
                let ext_bank = external_bnk_cache.get(&ext_path).unwrap();
                let wem_bytes = match ext_bank.get_wem_bytes(entry.id) {
                    Some(b) => b.to_vec(),
                    None => {
                        skipped.push(format!(
                            "id {} not found in external {}",
                            entry.id, external_bnk_name
                        ));
                        continue;
                    }
                };
                if let Some(dest_path) = resolve_imported_source(
                    source_key,
                    || base_stem.clone(),
                    wem_bytes,
                    &dest_dir,
                    &mut used_names,
                    &mut source_variants,
                    &mut copied,
                    &mut failed,
                ) {
                    in_memory_map_entries.push(LinkedInMemoryEntry {
                        source_file_path: dest_path,
                        id: entry.id,
                        name: entry.name.clone(),
                        relative_path: entry.name.clone(),
                        source_bnk_path: local_bnk_path,
                        original_wem_name: format!("{}.wem", entry.id),
                        bnk_relative_path: local_bnk_relative,
                    });
                    handled_in_memory_ids.insert(entry.id);
                }
            }
        }
        if let Ok(entries) = std::fs::read_dir(&media_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("wem") {
                    continue;
                }
                let id: u32 = match path.file_stem().and_then(|s| s.to_str()).and_then(|s| s.parse().ok()) {
                    Some(i) => i,
                    None => continue,
                };
                if handled_streamed_ids.contains(&id) {
                    continue;
                }
                let local = match local_streamed_by_id.get(&id) {
                    Some(e) => e.clone(),
                    None => {
                        skipped.push(format!("{}.wem not in local streamed_index", id));
                        continue;
                    }
                };
                let bytes = match std::fs::read(&path) {
                    Ok(b) => b,
                    Err(e) => {
                        self.log_and_status(&format!("Failed to read {}.wem: {}", id, e));
                        failed += 1;
                        continue;
                    }
                };
                if let Ok(local_bytes) = std::fs::read(&local.source_path) {
                    if local_bytes == bytes {
                        skipped.push(format!("{}.wem unchanged, skipping", id));
                        continue;
                    }
                }
                if let Some(dest_path) = resolve_imported_source(
                    "__fallback__",
                    || {
                        let s = format!("{}[{}]", project_name, fallback_counter);
                        fallback_counter += 1;
                        s
                    },
                    bytes,
                    &dest_dir,
                    &mut used_names,
                    &mut source_variants,
                    &mut copied,
                    &mut failed,
                ) {
                    streamed_map_entries.push(LinkedStreamedEntry {
                        source_file_path: dest_path,
                        id,
                        name: local.name.clone(),
                        generated_file: local.generated_file.clone(),
                        relative_path: local.relative_path.clone(),
                    });
                    handled_streamed_ids.insert(id);
                }
            }
        }
        if let Ok(entries) = std::fs::read_dir(&windows_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("bnk") {
                    continue;
                }
                if external_bnk_cache.contains_key(&path) {
                    continue;
                }
                let external_bank = match SoundBank::from_file(&path) {
                    Ok(b) => b,
                    Err(e) => {
                        skipped.push(format!(
                            "failed to open external {}: {}",
                            path.file_name().unwrap_or_default().to_string_lossy(),
                            e
                        ));
                        continue;
                    }
                };
                for desc in &external_bank.didx_entries {
                    let id = desc.id;
                    if handled_in_memory_ids.contains(&id) {
                        continue;
                    }
                    let local = match local_in_memory_by_id.get(&id) {
                        Some(e) => e.clone(),
                        None => {
                            continue;
                        }
                    };
                    let wem_bytes = match external_bank.get_wem_bytes(id) {
                        Some(b) => b.to_vec(),
                        None => continue,
                    };
                    if !local_bnk_cache.contains_key(&local.source_bnk_path) {
                        match SoundBank::from_file(&local.source_bnk_path) {
                            Ok(b) => { local_bnk_cache.insert(local.source_bnk_path.clone(), b); }
                            Err(_) => continue,
                        }
                    }
                    let local_bank = local_bnk_cache.get(&local.source_bnk_path).unwrap();
                    let local_bytes = match local_bank.get_wem_bytes(id) {
                        Some(b) => b,
                        None => continue,
                    };
                    if local_bytes == wem_bytes.as_slice() {
                        continue;
                    }
                    if let Some(dest_path) = resolve_imported_source(
                        "__fallback__",
                        || {
                            let s = format!("{}[{}]", project_name, fallback_counter);
                            fallback_counter += 1;
                            s
                        },
                        wem_bytes,
                        &dest_dir,
                        &mut used_names,
                        &mut source_variants,
                        &mut copied,
                        &mut failed,
                    ) {
                        in_memory_map_entries.push(LinkedInMemoryEntry {
                            source_file_path: dest_path,
                            id,
                            name: local.name.clone(),
                            relative_path: local.relative_path.clone(),
                            source_bnk_path: local.source_bnk_path.clone(),
                            original_wem_name: local.original_wem_name.clone(),
                            bnk_relative_path: local.bnk_relative_path.clone(),
                        });
                        handled_in_memory_ids.insert(id);
                    }
                }
            }
        }
        if copied == 0 {
            let _ = std::fs::remove_dir(&dest_dir);
        }
        if let Err(e) = std::fs::create_dir_all(&project_path) {
            self.import_error = Some(format!("Failed to create project folder: {}", e));
            return false;
        }
        streamed_map_entries.sort_by_key(|e| e.id);
        in_memory_map_entries.sort_by_key(|e| e.id);
        if let Err(e) = std::fs::write(
            project_path.join("streamed_map.json"),
            serde_json::to_string_pretty(&streamed_map_entries).unwrap_or_default(),
        ) {
            self.import_error = Some(format!("Failed to write streamed map: {}", e));
            return false;
        }
        if let Err(e) = std::fs::write(
            project_path.join("in_memory_map.json"),
            serde_json::to_string_pretty(&in_memory_map_entries).unwrap_or_default(),
        ) {
            self.import_error = Some(format!("Failed to write in-memory map: {}", e));
            return false;
        }
        if !skipped.is_empty() {
            log_message(&format!("Import '{}': {} entries skipped", project_name, skipped.len()));
            for s in &skipped {
                log_message(&format!("  - {}", s));
            }
        }
        self.show_import_dialog = false;
        self.import_path.clear();
        self.import_error = None;
        *self.new_project_dialog_open.borrow_mut() = false;
        self.log_and_status(&format!(
            "Imported '{}': {} files ({} failed, {} skipped)",
            project_name, copied, failed, skipped.len()
        ));
        true
    }
}