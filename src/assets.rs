use crate::models::{InMemoryEntry, StreamedEntry};
use crate::soundbank::SoundBank;
use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

// copies streamed wem files into the organized tree renamed by wwise object path name
pub(crate) fn process_audio_assets(
        source_dir: &Path,
        dest_root: &Path,
        logs_root: &Path,
        progress_callback: impl Fn(&str),
    ) -> Result<Vec<StreamedEntry>> {
        let mut txt_files = Vec::new();
        if let Ok(entries) = fs::read_dir(source_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "txt") {
                    txt_files.push(path);
                }
            }
        }
        progress_callback(&format!("Found {} .txt files in '{}'", txt_files.len(), source_dir.display()));
        if txt_files.is_empty() {
            progress_callback("No .txt files found. Exiting.");
            return Ok(Vec::new());
        }
        progress_callback("Indexing .wem files...");
        let media_dir = source_dir.join("media");
        let mut wem_index = HashMap::new();
        if let Ok(entries) = fs::read_dir(&media_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "wem") {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        wem_index.insert(name.to_string(), path);
                    }
                }
            }
            progress_callback(&format!("Indexed {} .wem files", wem_index.len()));
        } else {
            progress_callback(&format!("Error: media folder not found at '{}'", media_dir.display()));
        }
        let mut all_entries = Vec::new();
        for txt_path in txt_files {
            progress_callback(&format!("Parsing {:?}", txt_path.file_name().unwrap_or_default()));
            let entries = match parse_txt_for_streamed(&txt_path) {
                Ok(e) => {
                    progress_callback(&format!("Extracted {} streamed entries from this file", e.len()));
                    e
                }
                Err(e) => {
                    progress_callback(&format!("Error parsing {:?}: {}", txt_path, e));
                    continue;
                }
            };
            for (id, name, generated_file, relative) in entries {
                let source_path = match wem_index.get(&generated_file) {
                    Some(p) => p.clone(),
                    None => {
                        progress_callback(&format!("Missing .wem {} for {}", generated_file, name));
                        continue;
                    }
                };
                let mut dest_path = dest_root.to_path_buf();
                for component in relative.split('\\') {
                    dest_path.push(component);
                }
                dest_path.pop();
                dest_path.push(format!("{}.wem", name));
                if let Some(parent) = dest_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                progress_callback(&format!("Copying {} -> {}", generated_file, dest_path.display()));
                std::fs::copy(&source_path, &dest_path)
                    .with_context(|| format!("Failed to copy {} to {}", source_path.display(), dest_path.display()))?;
                all_entries.push(StreamedEntry {
                    id,
                    name,
                    generated_file,
                    relative_path: relative,
                    source_path,
                    destination_path: dest_path,
                });
            }
        }
        let index_path = logs_root.join("streamed_index.json");
        let json = serde_json::to_string_pretty(&all_entries)?;
        std::fs::write(&index_path, json)?;
        progress_callback(&format!("Index written to {:?}", index_path));
        Ok(all_entries)
    }

// extracts in memory wem files from each bnk into the organized tree
pub(crate) fn process_in_memory_assets(
        source_dir: &Path,
        dest_root: &Path,
        logs_root: &Path,
        progress_callback: impl Fn(&str),
    ) -> Result<Vec<InMemoryEntry>> {
        let mut txt_files = Vec::new();
        if let Ok(entries) = fs::read_dir(source_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "txt") {
                    txt_files.push(path);
                }
            }
        }
        progress_callback(&format!("Found {} in .txt files", txt_files.len()));
        let mut all_entries = Vec::new();
        for txt_path in txt_files {
            let (in_memory_entries, _) = parse_txt_for_in_memory_and_streamed(&txt_path)?;
            if in_memory_entries.is_empty() {
                progress_callback("No In Memory entries found, skipping");
                continue;
            }
            let bnk_path = txt_path.with_extension("bnk");
            if !bnk_path.exists() {
                progress_callback(&format!("Warning: .bnk not found at {:?}", bnk_path));
                continue;
            }
            let bnk_relative_path = bnk_path
                .strip_prefix(source_dir)
                .unwrap_or(&bnk_path)
                .to_path_buf();
            let temp_dir = dest_root.join("temp").join(txt_path.file_stem().unwrap_or_default());
            match SoundBank::from_file(&bnk_path) {
                Ok(bank) => {
                    if let Err(e) = bank.extract_wems(&temp_dir) {
                        progress_callback(&format!("Failed to extract .wem from {}: {}", bnk_path.display(), e));
                        continue;
                    }
                }
                Err(e) => {
                    progress_callback(&format!("Failed to parse bnk {}: {}", bnk_path.display(), e));
                    continue;
                }
            }
            let mut wem_map: HashMap<u32, PathBuf> = HashMap::new();
            let extracted_wems: Vec<PathBuf> = fs::read_dir(&temp_dir)?
                .filter_map(|entry| entry.ok())
                .filter(|entry| entry.path().extension().map_or(false, |ext| ext == "wem"))
                .map(|entry| entry.path())
                .collect();
            for wem_path in &extracted_wems {
                if let Some(stem) = wem_path.file_stem().and_then(|s| s.to_str()) {
                    if let Ok(id) = stem.parse::<u32>() {
                        wem_map.insert(id, wem_path.clone());
                    } else {
                        progress_callback(&format!("Warning: Could not parse ID from filename: {:?}", wem_path));
                    }
                } else {
                    progress_callback(&format!("Warning: Missing file stem for {:?}", wem_path));
                }
            }
            if wem_map.len() != in_memory_entries.len() {
                progress_callback(&format!(
                    "Warning: Mismatch in {:?}: {} entries in text, {} wems extracted (map size: {})",
                    bnk_path, in_memory_entries.len(), extracted_wems.len(), wem_map.len()
                ));
            }
            for (id, name, relative) in in_memory_entries {
                if let Some(source_wem) = wem_map.get(&id) {
                    let mut dest_path = dest_root.to_path_buf();
                    for component in relative.split('\\') {
                        dest_path.push(component);
                    }
                    dest_path.pop();
                    dest_path.push(format!("{}.wem", name));
                    if let Some(parent) = dest_path.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    std::fs::copy(source_wem, &dest_path)?;
                    all_entries.push(InMemoryEntry {
                        id,
                        name: name.clone(),
                        relative_path: relative.clone(),
                        source_bnk_path: bnk_path.clone(),
                        original_wem_name: source_wem.file_name().unwrap_or_default().to_string_lossy().to_string(),
                        destination_path: dest_path.clone(),
                        bnk_relative_path: bnk_relative_path.clone(),
                    });
                } else {
                    progress_callback(&format!("Warning: No extracted WEM found for ID {}", id));
                }
            }
            if let Err(e) = std::fs::remove_dir_all(&temp_dir) {
                progress_callback(&format!("Warning: Failed to remove temp dir {:?}: {}", temp_dir, e));
            }
        }
        let index_path = logs_root.join("in_memory_index.json");
        let json = serde_json::to_string_pretty(&all_entries)?;
        std::fs::write(&index_path, json)?;
        progress_callback(&format!("In Memory index written to {:?}", index_path));
        Ok(all_entries)
    }

// extracts streamed and in memory object paths from EXTRACTED wwise txt files
pub(crate) fn parse_txt_for_in_memory_and_streamed(txt_path: &Path) -> Result<(Vec<(u32, String, String)>, Vec<(u32, String, String, String)>), anyhow::Error> {
    let content = std::fs::read_to_string(txt_path)?;
    let mut streamed_entries = Vec::new();
    let mut in_memory_entries = Vec::new();
    let mut in_streamed = false;
    let mut in_in_memory = false;
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with("In Memory Audio") {
            in_in_memory = true;
            in_streamed = false;
            continue;
        }
        if line.starts_with("Streamed Audio") {
            in_streamed = true;
            in_in_memory = false;
            continue;
        }
        if !in_streamed && !in_in_memory {
            continue;
        }
        let parts: Vec<&str> = line.split('\t').collect();
        if in_streamed {
            if parts.len() < 5 { continue; }
            let id = parts[0].parse::<u32>().unwrap_or(0);
            let name = parts[1].to_string();
            let generated_file = parts[3].to_string();
            let wwise_path = parts[4].trim();
            let relative = wwise_path
                .strip_prefix("\\Actor-Mixer Hierarchy\\")
                .unwrap_or(wwise_path)
                .to_string();
            streamed_entries.push((id, name, generated_file, relative));
        } else if in_in_memory {
            if parts.len() < 5 { continue; }
            let id = parts[0].parse::<u32>().unwrap_or(0);
            let name = parts[1].to_string();
            let wwise_path = parts[4].trim();
            let relative = wwise_path
                .strip_prefix("\\Actor-Mixer Hierarchy\\")
                .unwrap_or(wwise_path)
                .to_string();
            in_memory_entries.push((id, name, relative));
        }
    }
    Ok((in_memory_entries, streamed_entries))
}

// convenience wrapper for streamed entries
pub(crate) fn parse_txt_for_streamed(txt_path: &Path) -> Result<Vec<(u32, String, String, String)>, anyhow::Error> {
    let (_, streamed) = parse_txt_for_in_memory_and_streamed(txt_path)?;
    Ok(streamed)
}

// set of bnk paths that pair with a txt containing in memory entries
pub(crate) fn get_non_empty_bnks(source_dir: &Path) -> HashSet<PathBuf> {
    let mut result = HashSet::new();
    if let Ok(entries) = fs::read_dir(source_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "txt") {
                if let Ok((in_memory_entries, _)) = parse_txt_for_in_memory_and_streamed(&path) {
                    if !in_memory_entries.is_empty() {
                        let bnk_path = path.with_extension("bnk");
                        if bnk_path.exists() {
                            result.insert(bnk_path);
                        }
                    }
                }
            }
        }
    }
    result
}

// recursively total file sizes
pub(crate) fn get_folder_total_size(folder: &Path) -> u64 {
    let mut total = 0;
    if let Ok(entries) = fs::read_dir(folder) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                total += fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            } else if path.is_dir() {
                total += get_folder_total_size(&path);
            }
        }
    }
    total
}

// returns data for bnk file sizes
pub(crate) fn get_bnk_sizes(source_dir: &Path) -> HashMap<PathBuf, u64> {
    let mut map = HashMap::new();
    if let Ok(entries) = fs::read_dir(source_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "bnk") {
                if let Ok(metadata) = fs::metadata(&path) {
                    map.insert(path, metadata.len());
                }
            }
        }
    }
    map
}