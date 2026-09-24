use crate::models::FocusedPanel;
use eframe::egui::{self, Ui};
use std::cell::RefCell;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

#[derive(Debug, Clone)]
pub enum TreeNode {
    File { name: String, path: PathBuf, index: usize },
    Directory { name: String, path: PathBuf, children: Vec<TreeNode> },
}

#[derive(Clone)]
pub struct OrganizedTree {
    pub(crate) roots: Rc<RefCell<Vec<TreeNode>>>,
    pub(crate) expanded: Rc<RefCell<HashSet<PathBuf>>>,
    pub(crate) selected_set: Rc<RefCell<HashSet<PathBuf>>>,
    pub(crate) file_list: Rc<RefCell<Vec<PathBuf>>>,
    pub(crate) last_clicked_idx: Rc<RefCell<Option<usize>>>,
    pub(crate) focused_panel: Rc<RefCell<FocusedPanel>>,
    pub(crate) queue: Rc<RefCell<Vec<PathBuf>>>,
    pub(crate) linked_paths: Rc<RefCell<HashSet<PathBuf>>>,
}

// recursively builds a display tree from root path
pub(crate) fn build_tree_from_path(root: &Path) -> Option<TreeNode> {
    let name = root.file_name().unwrap_or_default().to_string_lossy().to_string();
    if root.is_dir() {
        let mut children = Vec::new();
        if let Ok(entries) = fs::read_dir(root) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(child) = build_tree_from_path(&path) {
                    children.push(child);
                }
            }
        }
        children.sort_by(|a, b| {
            let a_is_dir = matches!(a, TreeNode::Directory { .. });
            let b_is_dir = matches!(b, TreeNode::Directory { .. });
            if a_is_dir && !b_is_dir {
                std::cmp::Ordering::Less
            } else if !a_is_dir && b_is_dir {
                std::cmp::Ordering::Greater
            } else {
                let a_name = match a {
                    TreeNode::File { name, .. } => name,
                    TreeNode::Directory { name, .. } => name,
                };
                let b_name = match b {
                    TreeNode::File { name, .. } => name,
                    TreeNode::Directory { name, .. } => name,
                };
                a_name.cmp(b_name)
            }
        });
        Some(TreeNode::Directory { name, path: root.to_path_buf(), children})
    } else {
        Some(TreeNode::File { name, path: root.to_path_buf(), index: 0 })
    }
}

// assigns flat depth first search indices to files in visual order
pub(crate) fn reindex_tree(node: &mut TreeNode, file_list: &mut Vec<PathBuf>, next_idx: &mut usize) {
    match node {
        TreeNode::File { path, index, .. } => {
            *index = *next_idx;
            file_list.push(path.clone());
            *next_idx += 1;
        }
        TreeNode::Directory { children, .. } => {
            for child in children.iter_mut() {
                reindex_tree(child, file_list, next_idx);
            }
        }
    }
}

// returns tree roots plus flat file list
pub(crate) fn build_roots_from_path(root: &Path) -> (Vec<TreeNode>, Vec<PathBuf>) {
    let mut file_list = Vec::new();
    let mut next_idx = 0;
    let mut roots = Vec::new();
    if let Some(tree) = build_tree_from_path(root) {
        if let TreeNode::Directory { children, .. } = tree {
            roots = children;
        }
    }
    for node in roots.iter_mut() {
        reindex_tree(node, &mut file_list, &mut next_idx);
    }
    (roots, file_list)
}

// draws every root of organized tree
pub(crate) fn draw_tree(ui: &mut Ui, tree: &OrganizedTree, copy_id_tx: &std::sync::mpsc::Sender<PathBuf>) {
    let roots = tree.roots.borrow();
    if roots.is_empty() {
        ui.label("No Organized Audio folder found");
    } else {
        for node in roots.iter() {
            draw_tree_node(ui, node, tree, copy_id_tx);
        }
    }
}

// recursively draws one node and handles folder context menus and file selection
pub(crate) fn draw_tree_node(ui: &mut Ui, node: &TreeNode, tree: &OrganizedTree, copy_id_tx: &std::sync::mpsc::Sender<PathBuf>) {
    match node {
        TreeNode::Directory { name, path, children } => {
            let is_expanded = tree.expanded.borrow().contains(path);
            let header = egui::CollapsingHeader::new(format!("📁 {}", name))
                .default_open(is_expanded)
                .show(ui, |ui| {
                    for child in children {
                        draw_tree_node(ui, child, tree, copy_id_tx);
                    }
                });
            let header_response = header.header_response;
            header_response.context_menu(|ui| {
                if ui.button(" Select All ").clicked() {
                    let files = collect_files_in_directory(node);
                    let mut sel = tree.selected_set.borrow_mut();
                    sel.clear();
                    for file in files {
                        sel.insert(file);
                    }
                    *tree.last_clicked_idx.borrow_mut() = None;
                    ui.close();
                }
                if ui.button(" Queue All ").clicked() {
                    let files = collect_files_in_directory(node);
                    let mut queue = tree.queue.borrow_mut();
                    for file in files {
                        if !queue.contains(&file) {
                            queue.push(file);
                        }
                    }
                    ui.close();
                }
            });
            if header_response.clicked() {
                let mut exp = tree.expanded.borrow_mut();
                if is_expanded {
                    exp.remove(path);
                } else {
                    exp.insert(path.clone());
                }
            }
        }
        TreeNode::File { name: _, path, index } => {
            let selected = tree.selected_set.borrow().contains(path);
            let is_linked = tree.linked_paths.borrow().contains(path);
            let mut file_clicked = false;
            let inner = ui.horizontal(|ui| {
                let mut button = egui::Button::new(format!("📄 {}", display_name(path)));
                if selected {
                    button = button.fill(ui.style().visuals.selection.bg_fill);
                }
                let resp = ui.add(button);
                if resp.clicked() {
                    file_clicked = true;
                }
                if is_linked {
                    ui.label("<>");
                }
                resp
            });
            if inner.inner.secondary_clicked() {
                let _ = copy_id_tx.send(path.clone());
            }
            if file_clicked {
                *tree.focused_panel.borrow_mut() = FocusedPanel::OrganizedAudio;
                let modifiers = ui.input(|i| i.modifiers);
                let mut sel = tree.selected_set.borrow_mut();
                let mut last_idx = tree.last_clicked_idx.borrow_mut();
                if modifiers.ctrl {
                    if sel.contains(path) {
                        sel.remove(path);
                    } else {
                        sel.insert(path.clone());
                    }
                    *last_idx = Some(*index);
                } else if modifiers.shift {
                    if let Some(last) = *last_idx {
                        let file_list = tree.file_list.borrow();
                        if let (Some(last_path), Some(curr_path)) = (file_list.get(last), file_list.get(*index)) {
                            if last_path.parent() == curr_path.parent() {
                                let parent = last_path.parent();
                                let start = last.min(*index);
                                let end = last.max(*index);
                                for i in start..=end {
                                    if let Some(p) = file_list.get(i) {
                                        if p.parent() == parent {
                                            sel.insert(p.clone());
                                        }
                                    }
                                }
                                *last_idx = Some(*index);
                            }
                        }
                    } else {
                        sel.clear();
                        sel.insert(path.clone());
                        *last_idx = Some(*index);
                    }
                } else {
                    sel.clear();
                    sel.insert(path.clone());
                    *last_idx = Some(*index);
                }
            }
        }
    }
}

// recursively collects every file under a directory node
pub(crate) fn collect_files_in_directory(node: &TreeNode) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let TreeNode::Directory { children, .. } = node {
        for child in children {
            match child {
                TreeNode::File { path, .. } => files.push(path.clone()),
                TreeNode::Directory { .. } => files.extend(collect_files_in_directory(child)),
            }
        }
    }
    files
}

// shortname for wem files otherwise filename
pub(crate) fn display_name(path: &Path) -> String {
    if let Some(ext) = path.extension() {
        if ext == "wem" {
            if let Some(stem) = path.file_stem() {
                return stem.to_string_lossy().to_string();
            }
        }
    }
    path.file_name().unwrap_or_default().to_string_lossy().to_string()
}

// enables source linking for wav and wem files only
pub(crate) fn is_linkable_file(path: &Path) -> bool {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    matches!(ext.as_str(), "wav" | "wem")
}