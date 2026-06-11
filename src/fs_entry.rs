use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub struct FsEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub size: u64,
    pub modified: Option<SystemTime>,
}

pub fn read_dir_sorted(path: &Path) -> Vec<FsEntry> {
    let mut entries: Vec<FsEntry> = std::fs::read_dir(path)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .filter_map(|e| {
                    let meta = e.metadata().ok()?;
                    Some(FsEntry {
                        name: e.file_name().to_string_lossy().to_string(),
                        path: e.path(),
                        is_dir: meta.is_dir(),
                        size: meta.len(),
                        modified: meta.modified().ok(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });

    entries
}

pub fn icon_for(entry: &FsEntry) -> &'static str {
    if entry.is_dir {
        return "📁";
    }
    let ext = Path::new(&entry.name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "svg" | "ico" => "🖼",
        "bat" | "cmd" | "ps1" | "sh" => "⚙",
        "exe" | "msi" => "💾",
        "zip" | "rar" | "7z" | "tar" | "gz" => "🗜",
        "txt" | "md" | "log" => "📄",
        "pdf" => "📕",
        "mp3" | "wav" | "flac" | "ogg" => "🎵",
        "mp4" | "mkv" | "avi" | "mov" => "🎬",
        "doc" | "docx" => "📘",
        "xls" | "xlsx" => "📗",
        "ppt" | "pptx" => "📙",
        "rs" | "py" | "js" | "ts" | "c" | "cpp" | "h" | "java" | "go" | "json" | "toml"
        | "yaml" | "yml" | "html" | "css" => "📝",
        _ => "📄",
    }
}

pub fn format_size(size: u64, is_dir: bool) -> String {
    if is_dir {
        return String::new();
    }
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut size_f = size as f64;
    let mut unit = 0;
    while size_f >= 1024.0 && unit < UNITS.len() - 1 {
        size_f /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{size} {}", UNITS[unit])
    } else {
        format!("{size_f:.1} {}", UNITS[unit])
    }
}

pub fn format_modified(modified: Option<SystemTime>) -> String {
    match modified {
        Some(t) => {
            let datetime: chrono::DateTime<chrono::Local> = t.into();
            datetime.format("%Y-%m-%d %H:%M").to_string()
        }
        None => String::new(),
    }
}
