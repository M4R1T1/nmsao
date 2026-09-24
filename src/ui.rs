mod file_browser;
mod file_queue;
mod organized_tree;
mod project_selector;

use crate::models::PendingClear;
use crate::tree::OrganizedTree;
use organized_tree::*;
use egui::WidgetText;
use egui_dock::TabViewer;
use std::path::PathBuf;
use std::rc::Rc;
use std::cell::RefCell;

pub use file_browser::FileBrowser;
pub use file_queue::FileQueueState;
pub use project_selector::ProjectState;

pub enum TabContent {
    FileBrowser(FileBrowser),
    FileQueue(FileQueueState),
    OrganizedTree(OrganizedTree),
    ProjectSelector(ProjectState),
}

pub struct MyTabViewer {
    pub(crate) selected_project: Rc<RefCell<Option<PathBuf>>>,
    pub(crate) projects_dir: PathBuf,
    pub(crate) link_tx: std::sync::mpsc::Sender<Vec<PathBuf>>,
    pub(crate) remove_tx: std::sync::mpsc::Sender<(String, u32)>,
    pub(crate) silence_tx: std::sync::mpsc::Sender<Vec<PathBuf>>,
    pub(crate) export_tx: std::sync::mpsc::Sender<Vec<PathBuf>>,
    pub(crate) copy_id_tx: std::sync::mpsc::Sender<PathBuf>,
    pub(crate) pending_source_clear: Rc<RefCell<Option<PendingClear>>>,
}

impl TabViewer for MyTabViewer {
    type Tab = TabContent;

    fn title(&mut self, tab: &mut Self::Tab) -> WidgetText {
        match tab {
            TabContent::FileBrowser(_) => "Source Files".into(),
            TabContent::FileQueue(state) => {
                let count = state.queue.borrow().len();
                if count == 0 {
                    "File Queue".into()
                } else {
                    format!("File Queue ({})", count).into()
                }
            }
            TabContent::OrganizedTree(_) => "Organized Audio".into(),
            TabContent::ProjectSelector(_) => "Project".into(),
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        match tab {
            TabContent::FileBrowser(browser) => {
                browser.show(ui);
            }
            TabContent::FileQueue(state) => {
                state.show(ui, self);
            }
            TabContent::OrganizedTree(tree) => {
                show_organized_tree_tab(ui, tree, self);
            }
            TabContent::ProjectSelector(_) => {
                project_selector::show(ui, self);
            }
        }
    }
}