use std::path::PathBuf;

use eframe::egui;

/// What the user wants to do with a sidebar entry.
pub enum Action {
    /// Navigate the current tab to this path.
    Navigate(PathBuf),
    /// Open this path in a new tab.
    OpenInNewTab(PathBuf),
}

fn entry(ui: &mut egui::Ui, label: &str, path: &std::path::Path) -> Option<Action> {
    let resp = ui.selectable_label(false, label);
    if resp.clicked() {
        return Some(Action::Navigate(path.to_path_buf()));
    }
    if resp.clicked_by(egui::PointerButton::Middle) {
        return Some(Action::OpenInNewTab(path.to_path_buf()));
    }
    None
}

/// Draws the quick-access sidebar. Returns an action if the user clicked
/// (navigate) or middle-clicked (open in new tab) a location.
pub fn show(ui: &mut egui::Ui) -> Option<Action> {
    let mut action = None;

    ui.label(egui::RichText::new("Quick access").strong());
    if let Some(user_dirs) = directories::UserDirs::new() {
        if let Some(a) = entry(ui, "🏠 Home", user_dirs.home_dir()) {
            action = Some(a);
        }
        if let Some(p) = user_dirs.desktop_dir() {
            if let Some(a) = entry(ui, "🖥 Desktop", p) {
                action = Some(a);
            }
        }
        if let Some(p) = user_dirs.document_dir() {
            if let Some(a) = entry(ui, "📄 Documents", p) {
                action = Some(a);
            }
        }
        if let Some(p) = user_dirs.download_dir() {
            if let Some(a) = entry(ui, "⬇ Downloads", p) {
                action = Some(a);
            }
        }
        if let Some(p) = user_dirs.picture_dir() {
            if let Some(a) = entry(ui, "🖼 Pictures", p) {
                action = Some(a);
            }
        }
    }

    ui.separator();
    ui.label(egui::RichText::new("This PC").strong());
    for letter in b'A'..=b'Z' {
        let drive = format!("{}:\\", letter as char);
        let drive_path = PathBuf::from(&drive);
        if drive_path.exists() {
            if let Some(a) = entry(ui, &format!("💾 {drive}"), &drive_path) {
                action = Some(a);
            }
        }
    }

    action
}
