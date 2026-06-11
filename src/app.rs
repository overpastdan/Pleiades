use std::path::{Component, Path, PathBuf};

use eframe::egui;
use egui_extras::{Column, TableBuilder};

use crate::fs_entry::{format_modified, format_size, icon_for};
use crate::settings::{AppState, Settings};
use crate::theme::{self, Starfield};
use crate::sidebar;
use crate::tab::Tab;

/// An action requested via right-click context menu on a file/folder entry.
enum EntryAction {
    Navigate(PathBuf),
    Open(PathBuf),
    OpenInNewTab(PathBuf),
    CopyPath(PathBuf),
    Reveal(PathBuf),
    Delete(PathBuf),
}

pub struct ExplorerApp {
    tabs: Vec<Tab>,
    active_tab: usize,
    closed_tabs: Vec<PathBuf>,
    settings: Settings,
    show_settings: bool,
    dirty: bool,
    starfield: Starfield,
    applied_galaxy: Option<bool>,
}

impl ExplorerApp {
    pub fn new() -> Self {
        let state = AppState::load();

        let mut tabs: Vec<Tab> = if state.settings.restore_tabs_on_startup && !state.tabs.is_empty()
        {
            state.tabs.into_iter().map(Tab::new).collect()
        } else {
            Vec::new()
        };
        if tabs.is_empty() {
            tabs.push(Tab::new(default_start_path()));
        }

        let active_tab = state.active_tab.min(tabs.len() - 1);

        Self {
            tabs,
            active_tab,
            closed_tabs: state.closed_tabs,
            settings: state.settings,
            show_settings: false,
            dirty: false,
            starfield: Starfield::new(),
            applied_galaxy: None,
        }
    }

    fn save_state(&self) {
        let state = AppState {
            settings: self.settings.clone(),
            tabs: self.tabs.iter().map(|t| t.current_path.clone()).collect(),
            active_tab: self.active_tab,
            closed_tabs: self.closed_tabs.clone(),
        };
        state.save();
    }

    fn open_new_tab(&mut self, path: PathBuf) {
        self.tabs.push(Tab::new(path));
        self.active_tab = self.tabs.len() - 1;
        self.dirty = true;
    }

    fn close_tab(&mut self, idx: usize) {
        if self.tabs.len() == 1 {
            let path = self.tabs[idx].current_path.clone();
            self.closed_tabs.push(path);
            self.tabs[idx] = Tab::new(default_start_path());
            self.dirty = true;
            return;
        }
        let tab = self.tabs.remove(idx);
        self.closed_tabs.push(tab.current_path.clone());
        if self.active_tab >= self.tabs.len() {
            self.active_tab = self.tabs.len() - 1;
        } else if self.active_tab > idx {
            self.active_tab -= 1;
        }
        self.dirty = true;
    }

    fn reopen_closed_tab(&mut self) {
        if let Some(path) = self.closed_tabs.pop() {
            self.open_new_tab(path);
        }
    }

    /// Moves the tab at `from` so it lands at the position of the tab `to`
    /// (used by drag-to-reorder), keeping the active tab pointed at the same
    /// tab it was before.
    fn move_tab(&mut self, from: usize, to: usize) {
        if from == to || from >= self.tabs.len() || to >= self.tabs.len() {
            return;
        }
        let active_is_from = self.active_tab == from;
        let tab = self.tabs.remove(from);
        let insert_at = if from < to { to - 1 } else { to };

        // Track where the previously-active tab ended up.
        let mut active = self.active_tab;
        if !active_is_from {
            if active > from {
                active -= 1;
            }
            if active >= insert_at {
                active += 1;
            }
        }

        self.tabs.insert(insert_at, tab);
        self.active_tab = if active_is_from { insert_at } else { active };
        self.dirty = true;
    }

    fn handle_entry_action(&mut self, ctx: &egui::Context, action: EntryAction) {
        match action {
            EntryAction::Navigate(p) => {
                self.tabs[self.active_tab].navigate_to(p);
                self.dirty = true;
            }
            EntryAction::Open(p) => {
                let _ = opener::open(p);
            }
            EntryAction::OpenInNewTab(p) => self.open_new_tab(p),
            EntryAction::CopyPath(p) => ctx.copy_text(p.to_string_lossy().to_string()),
            EntryAction::Reveal(p) => {
                let _ = std::process::Command::new("explorer")
                    .arg(format!("/select,{}", p.display()))
                    .spawn();
            }
            EntryAction::Delete(p) => {
                if trash::delete(&p).is_ok() {
                    self.tabs[self.active_tab].refresh();
                }
            }
        }
    }
}

fn default_start_path() -> PathBuf {
    directories::UserDirs::new()
        .map(|d| d.home_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("C:\\"))
}

/// Builds clickable breadcrumb segments `(label, path)` from a path.
fn breadcrumb_segments(path: &Path) -> Vec<(String, PathBuf)> {
    let mut segments = Vec::new();
    let mut acc = PathBuf::new();
    for comp in path.components() {
        match comp {
            Component::Prefix(p) => {
                // e.g. "C:" — navigate target needs the trailing separator
                // so it resolves to the drive root.
                let label = p.as_os_str().to_string_lossy().to_string();
                let mut root = p.as_os_str().to_os_string();
                root.push("\\");
                acc = PathBuf::from(root);
                segments.push((label, acc.clone()));
            }
            Component::RootDir => {
                if segments.is_empty() {
                    acc.push(std::path::MAIN_SEPARATOR.to_string());
                    segments.push((std::path::MAIN_SEPARATOR.to_string(), acc.clone()));
                }
            }
            Component::Normal(s) => {
                acc.push(s);
                segments.push((s.to_string_lossy().to_string(), acc.clone()));
            }
            _ => {}
        }
    }
    segments
}

impl eframe::App for ExplorerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Apply theme when the galaxy setting changes (and on first frame).
        if self.applied_galaxy != Some(self.settings.galaxy_mode) {
            theme::apply_style(ctx, self.settings.galaxy_mode);
            self.applied_galaxy = Some(self.settings.galaxy_mode);
        }
        if self.settings.galaxy_mode {
            self.starfield.paint(ctx);
        }

        let reopen_pressed = ctx.input_mut(|i| {
            i.consume_key(egui::Modifiers::CTRL | egui::Modifiers::SHIFT, egui::Key::T)
        });
        if reopen_pressed {
            self.reopen_closed_tab();
        }

        let mut breadcrumb_target: Option<PathBuf> = None;

        // Tab bar + toolbar
        egui::TopBottomPanel::top("tabs_and_toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let mut to_select = None;
                let mut to_close = None;
                let mut reorder: Option<(usize, usize)> = None;

                for i in 0..self.tabs.len() {
                    let selected = i == self.active_tab;
                    let title = self.tabs[i].title();

                    ui.horizontal(|ui| {
                        // Sense both click and drag: a press without movement is a
                        // click (select), a press with movement starts a drag (reorder).
                        let resp = ui
                            .selectable_label(selected, title)
                            .interact(egui::Sense::click_and_drag());

                        if resp.clicked() {
                            to_select = Some(i);
                        }
                        if resp.dragged() {
                            egui::DragAndDrop::set_payload(ui.ctx(), i);
                            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                        }

                        // Show an insertion line when another tab is dragged over this one.
                        if let Some(src) = resp.dnd_hover_payload::<usize>() {
                            if *src != i {
                                let rect = resp.rect;
                                ui.painter().vline(
                                    rect.left(),
                                    rect.y_range(),
                                    egui::Stroke::new(2.0, egui::Color32::from_rgb(180, 160, 255)),
                                );
                            }
                        }
                        if let Some(src) = resp.dnd_release_payload::<usize>() {
                            reorder = Some((*src, i));
                        }

                        if ui.small_button("x").clicked() {
                            to_close = Some(i);
                        }
                    });
                }

                if ui.button("+").clicked() {
                    let path = self.tabs[self.active_tab].current_path.clone();
                    self.open_new_tab(path);
                }

                if let Some((from, to)) = reorder {
                    self.move_tab(from, to);
                }
                if let Some(i) = to_select {
                    self.active_tab = i;
                }
                if let Some(i) = to_close {
                    self.close_tab(i);
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("⚙").clicked() {
                        self.show_settings = !self.show_settings;
                    }
                });
            });

            ui.separator();

            // Nav buttons + breadcrumbs
            ui.horizontal(|ui| {
                let tab = &mut self.tabs[self.active_tab];

                if ui
                    .add_enabled(!tab.back_stack.is_empty(), egui::Button::new("⬅"))
                    .clicked()
                {
                    tab.go_back();
                    self.dirty = true;
                }
                if ui
                    .add_enabled(!tab.forward_stack.is_empty(), egui::Button::new("➡"))
                    .clicked()
                {
                    tab.go_forward();
                    self.dirty = true;
                }
                if ui.button("⬆").clicked() {
                    tab.go_up();
                    self.dirty = true;
                }

                ui.separator();

                for (i, (label, path)) in breadcrumb_segments(&tab.current_path).into_iter().enumerate() {
                    if i > 0 {
                        ui.label("›");
                    }
                    if ui.selectable_label(false, label).clicked() {
                        breadcrumb_target = Some(path);
                    }
                }
            });

            // Editable address bar + search box
            ui.horizontal(|ui| {
                let tab = &mut self.tabs[self.active_tab];
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut tab.filter)
                            .hint_text("🔍 Search")
                            .desired_width(180.0),
                    );
                    ui.separator();
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut tab.address_text)
                            .desired_width(f32::INFINITY),
                    );
                    if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        let new_path = PathBuf::from(tab.address_text.clone());
                        tab.navigate_to(new_path);
                        self.dirty = true;
                    }
                });
            });
        });

        if let Some(path) = breadcrumb_target {
            self.tabs[self.active_tab].navigate_to(path);
            self.dirty = true;
        }

        // Settings window
        if self.show_settings {
            let mut open = self.show_settings;
            egui::Window::new("Settings")
                .open(&mut open)
                .resizable(false)
                .show(ctx, |ui| {
                    if ui
                        .checkbox(&mut self.settings.restore_tabs_on_startup, "Restore tabs on startup")
                        .changed()
                    {
                        self.dirty = true;
                    }
                    if ui
                        .checkbox(&mut self.settings.galaxy_mode, "🌌 Galaxy mode")
                        .changed()
                    {
                        self.dirty = true;
                    }
                    ui.separator();
                    ui.label("Reopen closed tab: Ctrl+Shift+T");
                    ui.label("(Press repeatedly to step further back through closed tabs)");
                    ui.label("Middle-click a folder to open it in a new tab.");
                    ui.label("Right-click a file or folder for more actions.");
                });
            self.show_settings = open;
        }

        // Sidebar
        egui::SidePanel::left("sidebar")
            .default_width(160.0)
            .show(ctx, |ui| match sidebar::show(ui) {
                Some(sidebar::Action::Navigate(path)) => {
                    self.tabs[self.active_tab].navigate_to(path);
                    self.dirty = true;
                }
                Some(sidebar::Action::OpenInNewTab(path)) => {
                    self.open_new_tab(path);
                }
                None => {}
            });

        // File list
        let mut entry_action: Option<EntryAction> = None;
        egui::CentralPanel::default().show(ctx, |ui| {
            let tab = &mut self.tabs[self.active_tab];

            if let Some(err) = &tab.error {
                ui.colored_label(egui::Color32::RED, err);
                return;
            }

            let indices = tab.filtered_indices();
            if indices.is_empty() {
                ui.weak(if tab.entries.is_empty() {
                    "This folder is empty."
                } else {
                    "No items match your search."
                });
                return;
            }

            let mut new_selected = None;
            let mut nav_target = None;
            let mut open_target = None;

            TableBuilder::new(ui)
                .striped(true)
                .resizable(true)
                .column(Column::remainder().at_least(200.0))
                .column(Column::auto().at_least(80.0))
                .column(Column::auto().at_least(140.0))
                .header(22.0, |mut header| {
                    header.col(|ui| {
                        ui.strong("Name");
                    });
                    header.col(|ui| {
                        ui.strong("Size");
                    });
                    header.col(|ui| {
                        ui.strong("Modified");
                    });
                })
                .body(|mut body| {
                    for &idx in &indices {
                        let entry = &tab.entries[idx];
                        body.row(22.0, |mut row| {
                            row.col(|ui| {
                                let label = format!("{} {}", icon_for(entry), entry.name);
                                let selected = tab.selected.as_ref() == Some(&entry.path);
                                let resp = ui.selectable_label(selected, label);
                                if resp.clicked() {
                                    new_selected = Some(entry.path.clone());
                                }
                                if resp.double_clicked() {
                                    if entry.is_dir {
                                        nav_target = Some(entry.path.clone());
                                    } else {
                                        open_target = Some(entry.path.clone());
                                    }
                                }
                                if entry.is_dir && resp.clicked_by(egui::PointerButton::Middle) {
                                    entry_action = Some(EntryAction::OpenInNewTab(entry.path.clone()));
                                }
                                resp.context_menu(|ui| {
                                    let p = entry.path.clone();
                                    if entry.is_dir {
                                        if ui.button("Open").clicked() {
                                            entry_action = Some(EntryAction::Navigate(p.clone()));
                                            ui.close();
                                        }
                                        if ui.button("Open in new tab").clicked() {
                                            entry_action = Some(EntryAction::OpenInNewTab(p.clone()));
                                            ui.close();
                                        }
                                    } else if ui.button("Open").clicked() {
                                        entry_action = Some(EntryAction::Open(p.clone()));
                                        ui.close();
                                    }
                                    ui.separator();
                                    if ui.button("Copy path").clicked() {
                                        entry_action = Some(EntryAction::CopyPath(p.clone()));
                                        ui.close();
                                    }
                                    if ui.button("Reveal in Windows Explorer").clicked() {
                                        entry_action = Some(EntryAction::Reveal(p.clone()));
                                        ui.close();
                                    }
                                    ui.separator();
                                    if ui.button("Delete to Recycle Bin").clicked() {
                                        entry_action = Some(EntryAction::Delete(p));
                                        ui.close();
                                    }
                                });
                            });
                            row.col(|ui| {
                                ui.label(format_size(entry.size, entry.is_dir));
                            });
                            row.col(|ui| {
                                ui.label(format_modified(entry.modified));
                            });
                        });
                    }
                });

            if let Some(p) = new_selected {
                tab.selected = Some(p);
            }
            if let Some(p) = nav_target {
                tab.navigate_to(p);
                self.dirty = true;
            }
            if let Some(p) = open_target {
                let _ = opener::open(p);
            }
        });

        if let Some(action) = entry_action {
            self.handle_entry_action(ctx, action);
        }

        if self.dirty {
            self.save_state();
            self.dirty = false;
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.save_state();
    }
}
