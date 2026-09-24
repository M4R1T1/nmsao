mod dialogs;
mod export;
mod panels;
mod player;
mod processing;
mod projects;

use crate::config::{load_config, save_config, AppConfig};
use crate::logging::{init_logging, log_message};
use crate::models::{FocusedPanel, PendingClear};
use crate::tree::{build_roots_from_path, OrganizedTree};
use crate::ui::{
    FileBrowser, FileQueueState, ProjectState, TabContent,
};
use crate::wwise::create_silence_wem;
use eframe::egui;
use egui_dock::{DockState, NodeIndex};
use std::cell::RefCell;
use std::collections::HashSet;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Instant;

pub struct MyApp {
    output_dir: PathBuf,
    logs_dir: PathBuf,
    dock_state: DockState<TabContent>,
    processing_status: Option<String>,
    processing_status_time: Option<Instant>,
    progress_rx: Option<std::sync::mpsc::Receiver<String>>,
    processing_handle: Option<std::thread::JoinHandle<()>>,
    deps_dir: PathBuf,
    organized_tree: OrganizedTree,
    selected_file: Rc<RefCell<Option<PathBuf>>>,
    focused_panel: Rc<RefCell<FocusedPanel>>,
    projects_dir: PathBuf,
    show_new_project_dialog: bool,
    new_project_name: String,
    project_creation_error: Option<String>,
    project_name_edit_id: Option<egui::Id>,
    new_project_dialog_open: Rc<RefCell<bool>>,
    #[allow(dead_code)]
    file_queue: Rc<RefCell<Vec<PathBuf>>>,
    #[allow(dead_code)]
    queue_selected_index: Rc<RefCell<Option<usize>>>,
    selected_project: Rc<RefCell<Option<PathBuf>>>,
    link_tx: Option<std::sync::mpsc::Sender<Vec<PathBuf>>>,
    link_rx: Option<std::sync::mpsc::Receiver<Vec<PathBuf>>>,
    remove_tx: Option<std::sync::mpsc::Sender<(String, u32)>>,
    remove_rx: Option<std::sync::mpsc::Receiver<(String, u32)>>,
    show_rename_dialog: bool,
    rename_old_path: Option<PathBuf>,
    rename_new_name: String,
    rename_error: Option<String>,
    show_delete_dialog: bool,
    delete_confirmation: String,
    delete_error: Option<String>,
    rename_edit_id: Option<egui::Id>,
    reset_rename_cursor: bool,
    exported_dir: PathBuf,
    show_clean_audio_dialog: bool,
    clean_audio_confirmation: String,
    clean_audio_error: Option<String>,
    silence_tx: Option<std::sync::mpsc::Sender<Vec<PathBuf>>>,
    silence_rx: Option<std::sync::mpsc::Receiver<Vec<PathBuf>>>,
    pending_source_clear: Rc<RefCell<Option<PendingClear>>>,
    linked_paths: Rc<RefCell<HashSet<PathBuf>>>,
    linked_paths_source: Rc<RefCell<Option<PathBuf>>>,
    config: Rc<RefCell<AppConfig>>,
    show_preferences_dialog: bool,
    current_path: Rc<RefCell<PathBuf>>,
    export_tx: Option<std::sync::mpsc::Sender<Vec<PathBuf>>>,
    export_rx: Option<std::sync::mpsc::Receiver<Vec<PathBuf>>>,
    show_combine_dialog: bool,
    combine_selections: Vec<Option<PathBuf>>,
    combine_new_name: String,
    combine_error: Option<String>,
    imported_dir: PathBuf,
    show_import_dialog: bool,
    import_path: String,
    import_error: Option<String>,
    copy_id_tx: Option<std::sync::mpsc::Sender<PathBuf>>,
    copy_id_rx: Option<std::sync::mpsc::Receiver<PathBuf>>,
    audio_sink: Option<rodio::MixerDeviceSink>,
    audio_player: Option<rodio::Player>,
    player_modal_open: bool,
    player_source: Option<PathBuf>,
    player_bytes: Option<Vec<u8>>,
    player_duration: std::time::Duration,
    player_play_started: Option<Instant>,
    player_paused_at: std::time::Duration,
}

impl Default for MyApp {
    fn default() -> Self {
        let exe_dir = match std::env::current_exe() {
            Ok(path) => path.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| PathBuf::from(".")),
            Err(_) => PathBuf::from("."),
        };
        
        let output_dir = exe_dir.join("organized_audio");
        std::fs::create_dir_all(&output_dir).ok();
        let deps_dir = exe_dir.join("dependencies");
        std::fs::create_dir_all(&deps_dir).ok();
        if let Err(e) = create_silence_wem(&deps_dir) {
            log_message(&format!("Warning: Failed to create Silence1ms.wen: {}", e));
        }
        let logs_dir = exe_dir.join("logs");
        std::fs::create_dir_all(&logs_dir).ok();
        init_logging(logs_dir.join("master.log"));
        let config = Rc::new(RefCell::new(load_config(&logs_dir)));
        let config_path = logs_dir.join("config.json");
        if !config_path.exists() {
            save_config(&logs_dir, &config.borrow());
        }
        let projects_dir = exe_dir.join("projects");
        std::fs::create_dir_all(&projects_dir).ok();
        let exported_dir = exe_dir.join("exported");
        std::fs::create_dir_all(&exported_dir).ok();
        let imported_dir = exe_dir.join("imported");
        std::fs::create_dir_all(&imported_dir).ok();
        let selected_file = Rc::new(RefCell::new(None));
        let new_project_dialog_open = Rc::new(RefCell::new(false));
        let file_queue = Rc::new(RefCell::new(Vec::new()));
        let queue_selected_index = Rc::new(RefCell::new(None));
        let focused_panel = Rc::new(RefCell::new(FocusedPanel::OrganizedAudio));
        let queue_selected_set = Rc::new(RefCell::new(HashSet::new()));
        let queue_last_clicked = Rc::new(RefCell::new(None));
        let error_highlight = Rc::new(RefCell::new(None));
        let pending_source_clear: Rc<RefCell<Option<PendingClear>>> = Rc::new(RefCell::new(None));
        let linked_paths = Rc::new(RefCell::new(HashSet::new()));
        let linked_paths_source = Rc::new(RefCell::new(None));
        let current_path = Rc::new(RefCell::new(PathBuf::new()));

        let selected_project = Rc::new(RefCell::new(None));
        let (link_tx, link_rx) = std::sync::mpsc::channel();
        let (remove_tx, remove_rx) = std::sync::mpsc::channel();
        let (silence_tx, silence_rx): (
            std::sync::mpsc::Sender<Vec<PathBuf>>,
            std::sync::mpsc::Receiver<Vec<PathBuf>>
        ) = std::sync::mpsc::channel();
        let (export_tx, export_rx): (
            std::sync::mpsc::Sender<Vec<PathBuf>>,
            std::sync::mpsc::Receiver<Vec<PathBuf>>,
        ) = std::sync::mpsc::channel();
        let (copy_id_tx, copy_id_rx) = std::sync::mpsc::channel();

        let file_queue_state = FileQueueState {
            queue: file_queue.clone(),
            selected_set: queue_selected_set.clone(),
            last_clicked_idx: queue_last_clicked.clone(),
            selected_index: queue_selected_index.clone(),
            focused_panel: focused_panel.clone(),
            selected_file: selected_file.clone(),
        };

        let (roots, file_list) = build_roots_from_path(&output_dir);
        let organized_tree = OrganizedTree {
            roots: Rc::new(RefCell::new(roots)),
            expanded: Rc::new(RefCell::new(HashSet::new())),
            selected_set: Rc::new(RefCell::new(HashSet::new())),
            file_list: Rc::new(RefCell::new(file_list)),
            last_clicked_idx: Rc::new(RefCell::new(None)),
            focused_panel: focused_panel.clone(),
            queue: file_queue.clone(),
            linked_paths: linked_paths.clone(),
        };

        let mut dock_state = DockState::new(vec![
            TabContent::OrganizedTree(organized_tree.clone()),
            TabContent::FileQueue(file_queue_state),
        ]);

        let main_surface = dock_state.main_surface_mut();

        let [_, left_pane_index] = main_surface.split_left(
            NodeIndex::root(),
            0.5,
            vec![TabContent::FileBrowser(FileBrowser::new(
                selected_file.clone(),
                focused_panel.clone(),
                new_project_dialog_open.clone(),
                error_highlight.clone(),
                config.clone(),
                logs_dir.clone(),
                current_path.clone(),
            ))],
        );

        let [_, _bottom_pane_index] = main_surface.split_below(
            left_pane_index,
            0.5,
            vec![
                TabContent::ProjectSelector(ProjectState),
            ]
        );

        Self {
            output_dir,
            logs_dir,
            dock_state,
            processing_status: None,
            processing_status_time: None,
            progress_rx: None,
            processing_handle: None,
            deps_dir,
            organized_tree,
            selected_file,
            focused_panel,
            projects_dir,
            show_new_project_dialog: false,
            new_project_name: String::new(),
            project_creation_error: None,
            project_name_edit_id: None,
            new_project_dialog_open,
            file_queue,
            queue_selected_index,
            selected_project: selected_project.clone(),
            link_tx: Some(link_tx),
            link_rx: Some(link_rx),
            remove_tx: Some(remove_tx),
            remove_rx: Some(remove_rx),
            show_rename_dialog: false,
            rename_old_path: None,
            rename_new_name: String::new(),
            rename_error: None,
            show_delete_dialog: false,
            delete_confirmation: String::new(),
            delete_error: None,
            rename_edit_id: None,
            reset_rename_cursor: false,
            exported_dir,
            show_clean_audio_dialog: false,
            clean_audio_confirmation: String::new(),
            clean_audio_error: None,
            silence_tx: Some(silence_tx),
            silence_rx: Some(silence_rx),
            pending_source_clear,
            linked_paths,
            linked_paths_source,
            config,
            show_preferences_dialog: false,
            current_path,
            export_tx: Some(export_tx),
            export_rx: Some(export_rx),
            show_combine_dialog: false,
            combine_selections: vec![None],
            combine_new_name: String::new(),
            combine_error: None,
            imported_dir,
            show_import_dialog: false,
            import_path: String::new(),
            import_error: None,
            copy_id_tx: Some(copy_id_tx),
            copy_id_rx: Some(copy_id_rx),
            audio_sink: None,
            audio_player: None,
            player_modal_open: false,
            player_source: None,
            player_bytes: None,
            player_duration: std::time::Duration::ZERO,
            player_play_started: None,
            player_paused_at: std::time::Duration::ZERO,
        }
    }
}

impl MyApp {

    pub(super) fn poll_channels(&mut self, ctx: &egui::Context) {

        let mut finished = false;
        if let Some(rx) = &self.progress_rx {
            while let Ok(msg) = rx.try_recv() {
                if msg == "PROCESSING_COMPLETED" {
                    self.processing_status = Some("Processing Finished".to_string());
                    self.processing_status_time = Some(Instant::now());
                    finished = true;
                } else if msg.starts_with("ERROR:") {
                    self.processing_status = Some(msg);
                    finished = true;
                } else {
                    self.processing_status = Some(msg);
                }
                ctx.request_repaint();
            }
        }
        if finished {
            self.refresh_organized_tree();
            self.progress_rx = None;
            self.processing_handle = None;
        }
        if self.progress_rx.is_some() {
            ctx.request_repaint();
        }
        let timeout = std::time::Duration::from_secs(3);
        if let Some(t) = self.processing_status_time {
            if t.elapsed() > timeout {
                self.processing_status = None;
                self.processing_status_time = None;
            }
        }

        let mut pending = Vec::new();
        if let Some(rx) = &self.link_rx {
            while let Ok(paths) = rx.try_recv() {
                pending.push(paths);
            }
        }
        for paths in pending {
            self.link_to_source(paths);
            ctx.request_repaint();
        }

        if let Some(rx) = &self.remove_rx {
            let mut pending_removals = Vec::new();
            while let Ok(removal) = rx.try_recv() {
                pending_removals.push(removal);
            }
            let had_removals = !pending_removals.is_empty();
            for (map_type, id) in pending_removals {
                self.remove_from_project_map(map_type, id);
            }
            if had_removals {
                self.refresh_linked_paths();
                ctx.request_repaint();
            }
        }

        let mut silence_pending = Vec::new();
        if let Some(rx) = &self.silence_rx {
            while let Ok(paths) = rx.try_recv() {
                silence_pending.push(paths);
            }
        }
        for paths in silence_pending {
            self.add_silence_entries(paths);
            ctx.request_repaint();
        }

        let mut export_pending = Vec::new();
        if let Some(rx) = &self.export_rx {
            while let Ok(paths) = rx.try_recv() {
                export_pending.push(paths);
            }
        }
        for paths in export_pending {
            self.export_audio(paths);
            ctx.request_repaint();
        }

        let mut copy_id_pending = Vec::new();
        if let Some(rx) = &self.copy_id_rx {
            while let Ok(path) = rx.try_recv() {
                copy_id_pending.push(path);
            }
        }
        for path in copy_id_pending {
            self.copy_id_for_path(ctx, path);
            ctx.request_repaint();
        }
    }
    
    // sends message to status bar and logs it in master.log
    pub(super) fn log_and_status(&mut self, msg: &str) {
        self.processing_status = Some(msg.to_string());
        self.processing_status_time = Some(Instant::now());
        log_message(msg);
    }

    pub(super) fn load_json<T: serde::de::DeserializeOwned + Default>(&self, path: &std::path::Path) -> T {
        if path.exists() {
            std::fs::read_to_string(path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            Default::default()
        }
    }
}

impl eframe::App for MyApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx();
        {
            let current = self.selected_project.borrow().clone();
            let loaded = self.linked_paths_source.borrow().clone();
            if current != loaded {
                self.refresh_linked_paths();
            }
        }
        self.show_toolbar(ctx);
        self.show_bottom_panel(ctx);
        self.show_central_panel(ctx);
        self.show_modals(ctx);
        self.poll_channels(ctx);
    }
}