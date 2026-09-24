use serde::{Serialize, Deserialize};
use std::path::{Path, PathBuf};
use std::collections::HashMap;

#[derive(Serialize, Deserialize)]
pub struct AppConfig {
    pub(crate) default_source_path: String,
    pub(crate) preserve_export_structure: bool,
    pub(crate) assets_path: String,
}

#[derive(Serialize, Deserialize, Default)]
pub struct AssetManifest {
    pub(crate) streamed_media_total_size: u64,
    pub(crate) bnk_files: HashMap<PathBuf, u64>,
}

// reads config.json from logs with defaults fallback
pub(crate) fn load_config(logs_dir: &Path) -> AppConfig {
    let path = logs_dir.join("config.json");
    if path.exists() {
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        AppConfig::default()
    }
}

// writes config.json to logs
pub(crate) fn save_config(logs_dir: &Path, config: &AppConfig) {
    let path = logs_dir.join("config.json");
    if let Ok(json) = serde_json::to_string_pretty(config) {
        let _ = std::fs::write(path, json);
    }
}

// reads asset_sizes.json which determines if process_audio_assets triggers for organized_audio
pub(crate) fn load_manifest(logs_root: &Path) -> AssetManifest {
    let path = logs_root.join("asset_sizes.json");
    if path.exists() {
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        AssetManifest::default()
    }
}

// writes asset_sizes.json
pub(crate) fn save_manifest(logs_root: &Path, manifest: &AssetManifest) {
    let path = logs_root.join("asset_sizes.json");
    if let Ok(json) = serde_json::to_string_pretty(manifest) {
        let _ = std::fs::write(path, json);
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            default_source_path: String::new(),
            preserve_export_structure: false,
            assets_path: String::new(),
        }
    }
}