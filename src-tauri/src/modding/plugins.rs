use std::fs;
use std::path::Path;

use crate::db::{open_db, get_bethesda_plugins, insert_or_replace_bethesda_plugin, BethesdaPluginRecord};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginType {
    Master,      // .esm
    LightMaster, // .esl
    Standard,    // .esp
}

impl PluginType {
    pub fn from_filename(filename: &str) -> Option<Self> {
        let lower = filename.to_lowercase();
        if lower.ends_with(".esm") {
            Some(PluginType::Master)
        } else if lower.ends_with(".esl") {
            Some(PluginType::LightMaster)
        } else if lower.ends_with(".esp") {
            Some(PluginType::Standard)
        } else {
            None
        }
    }

    pub fn priority_tier(&self) -> i32 {
        match self {
            PluginType::Master => 0,
            PluginType::LightMaster => 1,
            PluginType::Standard => 2,
        }
    }
}

/// Scans a directory for plugin files (.esm, .esl, .esp)
pub fn scan_plugins_in_directory(dir: &Path) -> Vec<String> {
    let mut plugins = Vec::new();
    if !dir.is_dir() {
        return plugins;
    }

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if PluginType::from_filename(name).is_some() {
                        plugins.push(name.to_string());
                    }
                }
            }
        }
    }

    // Sort: .esm first, then .esl, then .esp, then alphabetical
    plugins.sort_by(|a, b| {
        let type_a = PluginType::from_filename(a).map(|t| t.priority_tier()).unwrap_or(3);
        let type_b = PluginType::from_filename(b).map(|t| t.priority_tier()).unwrap_or(3);
        match type_a.cmp(&type_b) {
            std::cmp::Ordering::Equal => a.to_lowercase().cmp(&b.to_lowercase()),
            other => other,
        }
    });

    plugins
}

/// Syncs found plugins with the SQLite `bethesda_plugins` table
pub fn sync_bethesda_plugins(
    profile_id: &str,
    discovered_plugins: &[String],
    source_mod_id: Option<&str>,
) -> Result<Vec<BethesdaPluginRecord>, String> {
    let conn = open_db().map_err(|e| e.to_string())?;
    let existing = get_bethesda_plugins(&conn, profile_id).map_err(|e| e.to_string())?;

    let mut current_map: std::collections::HashMap<String, BethesdaPluginRecord> = existing
        .into_iter()
        .map(|p| (p.filename.to_lowercase(), p))
        .collect();

    let mut next_load_order = current_map.values().map(|p| p.load_order_index).max().unwrap_or(0) + 1;

    for filename in discovered_plugins {
        let key = filename.to_lowercase();
        if !current_map.contains_key(&key) {
            let record = BethesdaPluginRecord {
                id: format!("plugin-{}", uuid::Uuid::new_v4().simple()),
                profile_id: profile_id.to_string(),
                mod_id: source_mod_id.map(|s| s.to_string()),
                filename: filename.clone(),
                load_order_index: next_load_order,
                is_active: true,
            };
            let _ = insert_or_replace_bethesda_plugin(&conn, &record);
            current_map.insert(key, record);
            next_load_order += 1;
        }
    }

    get_bethesda_plugins(&conn, profile_id).map_err(|e| e.to_string())
}

/// Formats the `plugins.txt` content according to Bethesda standards:
/// Active plugins are prefixed with `*` (e.g. `*Unofficial Skyrim Special Edition Patch.esp`)
pub fn format_plugins_txt(plugins: &[BethesdaPluginRecord]) -> String {
    let mut sorted = plugins.to_vec();
    sorted.sort_by(|a, b| {
        let type_a = PluginType::from_filename(&a.filename).map(|t| t.priority_tier()).unwrap_or(3);
        let type_b = PluginType::from_filename(&b.filename).map(|t| t.priority_tier()).unwrap_or(3);
        match type_a.cmp(&type_b) {
            std::cmp::Ordering::Equal => a.load_order_index.cmp(&b.load_order_index),
            other => other,
        }
    });

    let mut lines = Vec::new();
    for p in sorted {
        if p.is_active {
            lines.push(format!("*{}", p.filename));
        } else {
            lines.push(p.filename);
        }
    }
    lines.join("\n")
}

/// Writes `plugins.txt` both into the target directory and to Wine/Proton AppData if found
pub fn write_plugins_txt(
    target_dir: &Path,
    wine_appdata_dir: Option<&Path>,
    content: &str,
) -> Result<(), String> {
    let target_file = target_dir.join("plugins.txt");
    fs::write(&target_file, content)
        .map_err(|e| format!("Failed to write plugins.txt to {}: {}", target_file.display(), e))?;

    if let Some(wine_dir) = wine_appdata_dir {
        if let Err(e) = fs::create_dir_all(wine_dir) {
            eprintln!("[Plugins] Could not create Wine AppData dir: {}", e);
        } else {
            let wine_file = wine_dir.join("plugins.txt");
            if let Err(e) = fs::write(&wine_file, content) {
                eprintln!("[Plugins] Failed to write Wine plugins.txt: {}", e);
            }
        }
    }

    Ok(())
}
