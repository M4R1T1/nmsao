use serde::{Serialize, Deserialize};
use std::path::PathBuf;
use std::collections::BTreeMap;
use std::path::Path;
use std::time::Instant;

pub(crate) const CLEAR_AUTO_REVERT_SECS: f32 = 8.0;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StreamedEntry {
    pub(crate) id: u32,
    pub(crate) name: String,
    pub(crate) generated_file: String,
    pub(crate) relative_path: String,
    pub(crate) source_path: PathBuf,
    pub(crate) destination_path: PathBuf,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InMemoryEntry {
    pub(crate) id: u32,
    pub(crate) name: String,
    pub(crate) relative_path: String,
    pub(crate) source_bnk_path: PathBuf,
    pub(crate) original_wem_name: String,
    pub(crate) destination_path: PathBuf,
    #[serde(default)]
    pub(crate) bnk_relative_path: PathBuf,
}

#[derive(Serialize, Deserialize, Default)]
pub struct LinkedStreamedEntry {
    pub(crate) source_file_path: PathBuf,
    pub(crate) id: u32,
    pub(crate) name: String,
    pub(crate) generated_file: String,
    pub(crate) relative_path: String,
}

#[derive(Serialize, Deserialize, Default)]
pub struct LinkedInMemoryEntry {
    pub(crate) source_file_path: PathBuf,
    pub(crate) id: u32,
    pub(crate) name: String,
    pub(crate) relative_path: String,
    pub(crate) source_bnk_path: PathBuf,
    pub(crate) original_wem_name: String,
    #[serde(default)]
    pub(crate) bnk_relative_path: PathBuf,
}

#[derive(Serialize, Deserialize)]
pub struct BuildManifest {
    pub(crate) project: String,
    #[serde(default, skip_serializing_if = "map_is_empty")]
    pub(crate) streamed: BTreeMap<String, Vec<BuildEntry>>,
    #[serde(default, skip_serializing_if = "map_is_empty")]
    pub(crate) in_memory: BTreeMap<String, Vec<BuildEntry>>,
}

#[derive(Serialize, Deserialize)]
pub struct BuildEntry {
    pub(crate) id: u32,
    pub(crate) name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) bnk: Option<String>,
}

#[derive(PartialEq, Copy, Clone)]
pub enum FocusedPanel {
    SourceFiles,
    OrganizedAudio,
    FileQueue,
}

pub struct PendingClear {
    pub(crate) is_streamed: bool,
    pub(crate) source_path: PathBuf,
    pub(crate) started: Instant,
}

// countdown label for source file blanket linked clear
pub(crate) fn clear_button_label(pending: Option<&PendingClear>, is_streamed: bool, src_path: &Path) -> String {
    match pending {
        Some(pc) if pc.is_streamed == is_streamed && pc.source_path == src_path => {
            let e = pc.started.elapsed().as_secs_f32();
            if e < 1.0 {
                "  2  ".to_string()
            } else if e < 2.0 {
                "  1  ".to_string()
            } else {
                "❌".to_string()
            }
        }
        _ => "⚠".to_string(),
    }
}

pub(crate) fn map_is_empty<K, V>(m: &BTreeMap<K, V>) -> bool {
    m.is_empty()
}