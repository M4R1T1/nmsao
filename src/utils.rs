use crate::logging::log_message;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

// display name for linked file source and mapping for silenced links
pub(crate) fn source_file_display(path: &Path) -> String {
    if path == Path::new("SILENCE") {
        return "SILENCE".to_string();
    }
    path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string_lossy().to_string())
}

// handles source file name duplication
pub(crate) fn assign_source_keys(paths: &[PathBuf]) -> HashMap<PathBuf, String> {
    let mut unique: Vec<&PathBuf> = paths.iter().collect();
    unique.sort();
    unique.dedup();
    let mut used: HashSet<String> = HashSet::new();
    let mut result: HashMap<PathBuf, String> = HashMap::new();
    for path in unique {
        let base = source_file_display(path);
        if used.insert(base.clone()) {
            result.insert(path.clone(), base);
            continue;
        }
        let stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| base.clone());
        let ext = path.extension().map(|e| e.to_string_lossy().to_string());
        let mut n = 2u32;
        loop {
            let candidate = match &ext {
                Some(e) => format!("{}[{}].{}", stem, n, e),
                None => format!("{}[{}]", stem, n),
            };
            if used.insert(candidate.clone()) {
                result.insert(path.clone(), candidate);
                break;
            }
            n += 1;
        }
    }
    result
}

// handles import source file name duplication
pub(crate) fn unique_import_name(stem: &str, used: &mut HashSet<String>) -> String {
    let base = format!("{}.wem", stem);
    if used.insert(base.clone()) {
        return base;
    }
    let mut n = 2u32;
    loop {
        let candidate = format!("{}[{}].wem", stem, n);
        if used.insert(candidate.clone()) {
            return candidate;
        }
        n += 1;
    }
}

// matches import source files by bytes to prevent unnnecessary duplication
pub(crate) fn resolve_imported_source<F>(
    source_key: &str,
    name_gen: F,
    bytes: Vec<u8>,
    dest_dir: &Path,
    used_names: &mut HashSet<String>,
    variants: &mut HashMap<String, Vec<(PathBuf, Vec<u8>)>>,
    copied: &mut usize,
    failed: &mut usize,
) -> Option<PathBuf>
where
    F: FnOnce() -> String,
{
    if let Some(existing) = variants.get(source_key) {
        for (path, existing_bytes) in existing {
            if existing_bytes.as_slice() == bytes.as_slice() {
                return Some(path.clone());
            }
        }
    }
    let stem = name_gen();
    let dest_name = unique_import_name(&stem, used_names);
    let dest_path = dest_dir.join(&dest_name);
    match std::fs::write(&dest_path, &bytes) {
        Ok(_) => {
            *copied += 1;
            variants
                .entry(source_key.to_string())
                .or_default()
                .push((dest_path.clone(), bytes));
            Some(dest_path)
        }
        Err(e) => {
            log_message(&format!("Failed to write {}: {}", dest_name, e));
            *failed += 1;
            None
        }
    }
}

// formats audio player duration as mm:ss
pub(crate) fn format_duration(d: std::time::Duration) -> String {
    let total = d.as_secs();
    format!("{:02}:{:02}", total / 60, total % 60)
}