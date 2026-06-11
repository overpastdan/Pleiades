use std::path::PathBuf;

use crate::fs_entry::{read_dir_sorted, FsEntry};

pub struct Tab {
    pub current_path: PathBuf,
    pub address_text: String,
    pub filter: String,
    pub back_stack: Vec<PathBuf>,
    pub forward_stack: Vec<PathBuf>,
    pub entries: Vec<FsEntry>,
    pub selected: Option<PathBuf>,
    pub error: Option<String>,
}

impl Tab {
    pub fn new(path: PathBuf) -> Self {
        let mut tab = Self {
            current_path: path,
            address_text: String::new(),
            filter: String::new(),
            back_stack: Vec::new(),
            forward_stack: Vec::new(),
            entries: Vec::new(),
            selected: None,
            error: None,
        };
        tab.refresh();
        tab
    }

    /// Indices of entries matching the current case-insensitive filter.
    pub fn filtered_indices(&self) -> Vec<usize> {
        if self.filter.trim().is_empty() {
            return (0..self.entries.len()).collect();
        }
        let needle = self.filter.to_lowercase();
        self.entries
            .iter()
            .enumerate()
            .filter(|(_, e)| e.name.to_lowercase().contains(&needle))
            .map(|(i, _)| i)
            .collect()
    }

    pub fn title(&self) -> String {
        self.current_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| self.current_path.to_string_lossy().to_string())
    }

    pub fn refresh(&mut self) {
        if self.current_path.exists() {
            self.entries = read_dir_sorted(&self.current_path);
            self.error = None;
        } else {
            self.entries.clear();
            self.error = Some(format!("Path not found: {}", self.current_path.display()));
        }
        self.address_text = self.current_path.to_string_lossy().to_string();
    }

    pub fn navigate_to(&mut self, path: PathBuf) {
        if path == self.current_path {
            return;
        }
        self.back_stack.push(self.current_path.clone());
        self.forward_stack.clear();
        self.current_path = path;
        self.selected = None;
        self.filter.clear();
        self.refresh();
    }

    pub fn go_back(&mut self) {
        if let Some(prev) = self.back_stack.pop() {
            self.forward_stack.push(self.current_path.clone());
            self.current_path = prev;
            self.selected = None;
            self.filter.clear();
            self.refresh();
        }
    }

    pub fn go_forward(&mut self) {
        if let Some(next) = self.forward_stack.pop() {
            self.back_stack.push(self.current_path.clone());
            self.current_path = next;
            self.selected = None;
            self.filter.clear();
            self.refresh();
        }
    }

    pub fn go_up(&mut self) {
        if let Some(parent) = self.current_path.parent() {
            self.navigate_to(parent.to_path_buf());
        }
    }
}
