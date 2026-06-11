use std::path::PathBuf;

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct Settings {
    pub restore_tabs_on_startup: bool,
    #[serde(default)]
    pub galaxy_mode: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            restore_tabs_on_startup: true,
            galaxy_mode: true,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct AppState {
    pub settings: Settings,
    pub tabs: Vec<PathBuf>,
    pub active_tab: usize,
    pub closed_tabs: Vec<PathBuf>,
    #[serde(default)]
    pub window_pos: Option<(f32, f32)>,
    #[serde(default)]
    pub window_size: Option<(f32, f32)>,
}

impl AppState {
    fn config_path() -> Option<PathBuf> {
        ProjectDirs::from("dev", "Pleiades", "PleiadesExplorer")
            .map(|dirs| dirs.config_dir().join("state.json"))
    }

    pub fn load() -> Self {
        if let Some(path) = Self::config_path() {
            if let Ok(data) = std::fs::read_to_string(&path) {
                if let Ok(state) = serde_json::from_str(&data) {
                    return state;
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) {
        let Some(path) = Self::config_path() else {
            return;
        };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(data) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, data);
        }
    }
}
