use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use reqwest::Client;
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

use crate::db;

pub const CURSEFORGE_API_KEY: &str = "$2a$10$bL4bIL5pUWqfcO7KQtnMReakwtfHbNKh6v1uTpKlzhwoueEJQnPnm";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WowInstanceRecord {
    pub id: String,
    pub name: String,
    pub flavor: String,
    pub path: String,
    pub addons_path: String,
    pub version: String,
    pub game_version_type_id: i64,
    pub installed: bool,
    pub addons_count: usize,
    pub icon: Option<String>,
    pub last_played: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledWowAddonRecord {
    pub id: String,
    pub folder_name: String,
    pub title: String,
    pub version: String,
    pub author: Option<String>,
    pub notes: Option<String>,
    pub website: Option<String>,
    pub category: Option<String>,
    pub interface_version: Option<String>,
    pub size_bytes: u64,
    pub size_formatted: String,
    pub last_modified: i64,
    pub enabled: bool,
    pub update_available: Option<bool>,
    pub latest_version: Option<String>,
    pub download_url: Option<String>,
    pub curseforge_id: Option<i64>,
    pub file_id: Option<i64>,
    pub latest_file_id: Option<i64>,
    pub icon_url: Option<String>,
    pub secondary_folders: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallAddonResult {
    pub success: bool,
    pub folders: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WowCharacterRecord {
    pub name: String,
    pub full_name: String,
    pub realm: String,
    pub account: String,
    pub level: Option<u32>,
    pub class: Option<String>,
    pub race: Option<String>,
    pub faction: Option<String>,
    pub last_played: Option<i64>,
}

/// Resolves the root World of Warcraft installation folder on the system
pub fn resolve_wow_root_dir() -> Option<PathBuf> {
    let home = dirs::home_dir().unwrap_or_else(|| {
        PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string()))
    });

    let mut candidate_roots = Vec::new();

    // 1. Check SQLite battlenet_config prefix
    if let Ok(conn) = db::open_db() {
        if let Ok(cfg) = db::get_battlenet_config(&conn) {
            let pfx = PathBuf::from(&cfg.wine_prefix_path);
            candidate_roots.push(pfx.join("drive_c/Program Files (x86)/World of Warcraft"));
            candidate_roots.push(pfx.join("drive_c/Program Files/World of Warcraft"));
            candidate_roots.push(pfx.join("drive_c/Games/World of Warcraft"));
        }
    }

    // 2. Standard Linux Wine / Lutris / Bottles paths
    candidate_roots.push(home.join("Games/battlenet/drive_c/Program Files (x86)/World of Warcraft"));
    candidate_roots.push(home.join("Games/battlenet/drive_c/Program Files/World of Warcraft"));
    candidate_roots.push(home.join(".local/share/bottles/bottles/Battle-net/drive_c/Program Files (x86)/World of Warcraft"));
    candidate_roots.push(home.join(".local/share/loom/prefixes/battlenet/drive_c/Program Files (x86)/World of Warcraft"));

    for c in candidate_roots {
        if c.is_dir() {
            return Some(c);
        }
    }

    None
}

/// Scans World of Warcraft editions/instances (Forever, Retail, Classic, Classic Era)
pub fn scan_wow_instances_impl() -> Vec<WowInstanceRecord> {
    let mut instances = Vec::new();
    let wow_root = resolve_wow_root_dir();

    struct FlavorMeta {
        id: &'static str,
        name: &'static str,
        flavor: &'static str,
        sub_dir: &'static str,
        version_type: i64,
        default_version: &'static str,
        icon: &'static str,
    }

    let flavors = [
        FlavorMeta {
            id: "forever",
            name: "World of Warcraft: Forever",
            flavor: "forever",
            sub_dir: "_forever_",
            version_type: 88568,
            default_version: "1.60.1.70790",
            icon: "https://images.igdb.com/igdb/image/upload/t_cover_big/co7bkq.jpg",
        },
        FlavorMeta {
            id: "retail",
            name: "World of Warcraft (Retail)",
            flavor: "retail",
            sub_dir: "_retail_",
            version_type: 517,
            default_version: "11.0.2",
            icon: "https://images.igdb.com/igdb/image/upload/t_cover_big/co8b7k.jpg",
        },
        FlavorMeta {
            id: "classic",
            name: "World of Warcraft Classic",
            flavor: "classic",
            sub_dir: "_classic_",
            version_type: 67408,
            default_version: "1.15.5",
            icon: "https://images.igdb.com/igdb/image/upload/t_cover_big/co1trz.jpg",
        },
        FlavorMeta {
            id: "classic_era",
            name: "WoW Classic: Season of Discovery",
            flavor: "classic_era",
            sub_dir: "_classic_era_",
            version_type: 67408,
            default_version: "1.15.5",
            icon: "https://images.igdb.com/igdb/image/upload/t_cover_big/co7bkt.jpg",
        },
    ];

    let base_dir = wow_root.unwrap_or_else(|| PathBuf::from("/home/k0bus/Games/battlenet/drive_c/Program Files (x86)/World of Warcraft"));

    for fl in &flavors {
        let inst_path = base_dir.join(fl.sub_dir);
        let is_installed = inst_path.exists() && inst_path.is_dir();
        let addons_path = inst_path.join("Interface").join("AddOns");
        let mut addons_count = 0;

        if addons_path.exists() {
            if let Ok(entries) = fs::read_dir(&addons_path) {
                addons_count = entries.flatten().filter(|e| e.path().is_dir()).count();
            }
        }

        let mut last_played = None;
        let config_wtf = inst_path.join("WTF").join("Config.wtf");
        if config_wtf.exists() {
            if let Ok(meta) = fs::metadata(&config_wtf) {
                if let Ok(mtime) = meta.modified() {
                    if let Ok(dur) = mtime.duration_since(std::time::UNIX_EPOCH) {
                        last_played = Some(dur.as_secs() as i64);
                    }
                }
            }
        }

        // Try reading version from .flavor.info if present
        let mut detected_version = fl.default_version.to_string();
        let flavor_info = inst_path.join(".flavor.info");
        if flavor_info.exists() {
            if let Ok(content) = fs::read_to_string(&flavor_info) {
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("version=") {
                        let v = trimmed.trim_start_matches("version=").trim();
                        if !v.is_empty() {
                            detected_version = v.to_string();
                        }
                    }
                }
            }
        }

        instances.push(WowInstanceRecord {
            id: fl.id.to_string(),
            name: fl.name.to_string(),
            flavor: fl.flavor.to_string(),
            path: inst_path.to_string_lossy().to_string(),
            addons_path: addons_path.to_string_lossy().to_string(),
            version: detected_version,
            game_version_type_id: fl.version_type,
            installed: is_installed,
            addons_count,
            icon: Some(fl.icon.to_string()),
            last_played,
        });
    }

    instances
}

fn clean_toc_text(s: &str) -> String {
    let mut res = s.to_string();
    // Remove color codes like |cFFFFFFFF and |r
    let re_colors = ["|r", "|R"];
    for c in &re_colors {
        res = res.replace(c, "");
    }
    // Remove |c followed by 6 or 8 hex digits
    let mut cleaned = String::new();
    let mut chars = res.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '|' && (chars.peek() == Some(&'c') || chars.peek() == Some(&'C')) {
            chars.next(); // skip 'c'
            // skip up to 8 hex characters
            let mut skipped = 0;
            while skipped < 8 {
                if let Some(&next_c) = chars.peek() {
                    if next_c.is_ascii_hexdigit() {
                        chars.next();
                        skipped += 1;
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
        } else {
            cleaned.push(ch);
        }
    }
    cleaned.trim().to_string()
}

fn get_dir_size(p: &Path) -> u64 {
    let mut total = 0;
    if let Ok(entries) = fs::read_dir(p) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Ok(meta) = fs::metadata(&path) {
                    total += meta.len();
                }
            } else if path.is_dir() {
                total += get_dir_size(&path);
            }
        }
    }
    total
}

#[derive(Debug, Clone, Default)]
struct WowUpAddonEntry {
    external_id: Option<i64>,
    folder_list: Vec<String>,
    _name: Option<String>,
    installed_file_id: Option<i64>,
    installed_version: Option<String>,
}

/// Checks if WowUp configuration has cached addon mappings on the system
fn load_wowup_mappings(instance_path: &Path) -> HashMap<String, WowUpAddonEntry> {
    let mut mapping = HashMap::new();
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));

    // Find installation id if preferences.json exists
    let mut target_install_id: Option<String> = None;
    let pref_candidates = [
        home.join(".var/app/io.wowup.WowUp/config/WowUpCf/preferences.json"),
        home.join(".config/WowUp/preferences.json"),
        home.join(".config/WowUpCf/preferences.json"),
    ];
    let inst_path_str = instance_path.to_string_lossy().to_lowercase();
    for pref_path in &pref_candidates {
        if let Ok(pref_content) = fs::read_to_string(pref_path) {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&pref_content) {
                if let Some(insts) = val.get("wow_installations").and_then(|v| v.as_array()) {
                    for inst in insts {
                        if let (Some(id), Some(loc)) = (inst.get("id").and_then(|v| v.as_str()), inst.get("location").and_then(|v| v.as_str())) {
                            let loc_low = loc.to_lowercase();
                            if loc_low.contains(&inst_path_str) || inst_path_str.contains(&loc_low)
                                || (inst_path_str.contains("_forever_") && loc_low.contains("_forever_"))
                                || (inst_path_str.contains("_retail_") && loc_low.contains("_retail_"))
                                || (inst_path_str.contains("_classic_") && loc_low.contains("_classic_"))
                                || (inst_path_str.contains("_classic_era_") && loc_low.contains("_classic_era_"))
                            {
                                target_install_id = Some(id.to_string());
                                break;
                            }
                        }
                    }
                }
            }
        }
        if target_install_id.is_some() {
            break;
        }
    }

    let candidates = [
        home.join(".var/app/io.wowup.WowUp/config/WowUpCf/addons.json"),
        home.join(".config/WowUp/addons.json"),
        home.join(".config/WowUpCf/addons.json"),
    ];

    for p in &candidates {
        if !p.exists() {
            continue;
        }
        if let Ok(content) = fs::read_to_string(p) {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(obj) = val.as_object() {
                    for (_k, v) in obj {
                        if let Some(ref tid) = target_install_id {
                            if let Some(iid) = v.get("installationId").and_then(|i| i.as_str()) {
                                if iid != tid {
                                    continue;
                                }
                            }
                        }

                        let folders_raw = v.get("installedFolders").and_then(|f| f.as_str()).unwrap_or("");
                        let folder_list: Vec<String> = folders_raw
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();

                        if folder_list.is_empty() {
                            continue;
                        }

                        let ext_id = v.get("externalId")
                            .and_then(|id| {
                                if let Some(n) = id.as_i64() {
                                    Some(n)
                                } else if let Some(s) = id.as_str() {
                                    s.parse::<i64>().ok()
                                } else {
                                    None
                                }
                            });

                        let installed_file_id = v.get("installedExternalReleaseId")
                            .and_then(|id| {
                                if let Some(n) = id.as_i64() {
                                    Some(n)
                                } else if let Some(s) = id.as_str() {
                                    s.parse::<i64>().ok()
                                } else {
                                    None
                                }
                            });

                        let installed_version = v.get("installedVersion")
                            .and_then(|v| v.as_str())
                            .filter(|s| !s.is_empty())
                            .map(|s| s.to_string());

                        let name = v.get("name").and_then(|n| n.as_str()).map(|s| s.to_string());

                        let entry = WowUpAddonEntry {
                            external_id: ext_id,
                            folder_list: folder_list.clone(),
                            _name: name,
                            installed_file_id,
                            installed_version,
                        };

                        for f in &folder_list {
                            mapping.insert(f.to_lowercase(), entry.clone());
                        }
                    }
                }
            }
        }
        if !mapping.is_empty() {
            break;
        }
    }

    mapping
}

fn pick_best_curseforge_file<'a>(
    files: &'a [serde_json::Value],
    target_type_id: Option<i64>,
) -> Option<&'a serde_json::Value> {
    if files.is_empty() {
        return None;
    }

    let mut matches = Vec::new();
    if let Some(target_type) = target_type_id {
        for f in files {
            let types: Vec<i64> = f.get("sortableGameVersions")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.get("gameVersionTypeId").and_then(|t| t.as_i64()))
                        .collect()
                })
                .unwrap_or_default();

            if types.contains(&target_type) {
                matches.push(f);
            }
        }

        // Fallback for Forever (88568) to Classic (67408)
        if matches.is_empty() && target_type == 88568 {
            for f in files {
                let types: Vec<i64> = f.get("sortableGameVersions")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.get("gameVersionTypeId").and_then(|t| t.as_i64()))
                            .collect()
                    })
                    .unwrap_or_default();

                if types.contains(&67408) {
                    matches.push(f);
                }
            }
        }
    }

    let candidates = if !matches.is_empty() {
        matches
    } else {
        files.iter().collect()
    };

    candidates.into_iter().max_by_key(|f| {
        let fid = f.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
        let rel_type = f.get("releaseType").and_then(|v| v.as_u64()).unwrap_or(99);
        let tscore = match rel_type {
            1 => 3,
            2 => 2,
            3 => 1,
            _ => 0,
        };
        (tscore, fid)
    })
}

fn is_newer_version(installed: &str, latest: &str) -> bool {
    let clean = |s: &str| -> String {
        let s = s.trim();
        let s = s.strip_prefix('v').or_else(|| s.strip_prefix('V')).unwrap_or(s);
        let s = s.trim_end_matches(".zip");
        s.trim().to_string()
    };

    let inst_clean = clean(installed);
    let lat_clean = clean(latest);

    if inst_clean.is_empty() || lat_clean.is_empty() || inst_clean.eq_ignore_ascii_case(&lat_clean) {
        return false;
    }

    // Compare numeric components (e.g. "1.15.20" vs "1.15.21", "839" vs "843")
    let extract_nums = |s: &str| -> Vec<u64> {
        s.split(|c: char| !c.is_ascii_digit())
            .filter_map(|p| p.parse::<u64>().ok())
            .collect()
    };

    let nums_inst = extract_nums(&inst_clean);
    let nums_lat = extract_nums(&lat_clean);

    if !nums_inst.is_empty() && !nums_lat.is_empty() {
        return nums_lat > nums_inst;
    }

    false
}

struct RawAddonMeta {
    folder_name: String,
    title: String,
    version: String,
    author: Option<String>,
    notes: Option<String>,
    website: Option<String>,
    category: Option<String>,
    interface_version: Option<String>,
    curse_id: Option<i64>,
    file_id: Option<i64>,
    icon_url: Option<String>,
    latest_version: Option<String>,
    download_url: Option<String>,
    size_bytes: u64,
    _size_formatted: String,
    last_modified: i64,
    enabled: bool,
    part_of: Option<String>,
    dependencies: Vec<String>,
    _load_on_demand: bool,
}

/// Scans installed AddOns for a given WoW instance using the full CurseForge / WowUp methodology
pub async fn get_wow_addons_impl(
    instance_path: String,
    check_updates: bool,
    game_version_type_id: Option<i64>,
) -> Result<Vec<InstalledWowAddonRecord>, String> {
    let inst_path = PathBuf::from(&instance_path);
    let addons_dir = if inst_path.ends_with("AddOns") {
        inst_path.clone()
    } else {
        inst_path.join("Interface").join("AddOns")
    };

    if !addons_dir.exists() {
        let _ = fs::create_dir_all(&addons_dir);
        return Ok(Vec::new());
    }

    // ── Phase 1 : scan brut de tous les dossiers et parsing complet des TOCs ──
    let mut raw_addons: Vec<RawAddonMeta> = Vec::new();
    if let Ok(entries) = fs::read_dir(&addons_dir) {
        for entry in entries.flatten() {
            let folder_path = entry.path();
            if !folder_path.is_dir() {
                continue;
            }

            let folder_name = folder_path.file_name().unwrap_or_default().to_string_lossy().to_string();
            let mut files = Vec::new();
            if let Ok(dir_entries) = fs::read_dir(&folder_path) {
                for de in dir_entries.flatten() {
                    files.push(de.file_name().to_string_lossy().to_string());
                }
            }

            let has_active_toc = files.iter().any(|f| f.ends_with(".toc") && !f.ends_with(".toc.disabled"));
            let is_enabled = has_active_toc || !files.iter().any(|f| f.ends_with(".toc.disabled"));

            // Locate best TOC file
            let toc_candidate = files.iter().find(|f| {
                let low = f.to_lowercase();
                low == format!("{}.toc", folder_name.to_lowercase())
                    || low == format!("{}.toc.disabled", folder_name.to_lowercase())
            }).or_else(|| {
                files.iter().find(|f| f.ends_with(".toc") || f.ends_with(".toc.disabled"))
            });

            let mut title = folder_name.clone();
            let mut version = "1.0.0".to_string();
            let mut author = None;
            let mut notes = None;
            let mut website = None;
            let mut category = None;
            let mut interface_version = None;
            let mut curse_id = None;
            let mut part_of = None;
            let mut dependencies = Vec::new();
            let mut load_on_demand = false;

            if let Some(toc_name) = toc_candidate {
                let toc_file_path = folder_path.join(toc_name);
                if let Ok(content) = fs::read_to_string(&toc_file_path) {
                    for line in content.lines() {
                        let trimmed = line.trim();
                        if trimmed.starts_with("## Title:") {
                            let raw_t = trimmed.trim_start_matches("## Title:").trim();
                            title = clean_toc_text(raw_t);
                        } else if trimmed.starts_with("## Version:") {
                            version = trimmed.trim_start_matches("## Version:").trim().to_string();
                        } else if trimmed.starts_with("## Author:") {
                            author = Some(trimmed.trim_start_matches("## Author:").trim().to_string());
                        } else if trimmed.starts_with("## Notes-frFR:") {
                            notes = Some(clean_toc_text(trimmed.trim_start_matches("## Notes-frFR:").trim()));
                        } else if trimmed.starts_with("## Notes:") && notes.is_none() {
                            notes = Some(clean_toc_text(trimmed.trim_start_matches("## Notes:").trim()));
                        } else if trimmed.starts_with("## X-Website:") || trimmed.starts_with("## Website:") {
                            website = Some(trimmed.split_once(':').map(|x| x.1.trim().to_string()).unwrap_or_default());
                        } else if trimmed.starts_with("## Category-frFR:") || trimmed.starts_with("## Category:") {
                            category = Some(trimmed.split_once(':').map(|x| x.1.trim().to_string()).unwrap_or_default());
                        } else if trimmed.starts_with("## Interface:") {
                            interface_version = Some(trimmed.trim_start_matches("## Interface:").trim().to_string());
                        } else if trimmed.starts_with("## X-Curse-Project-ID:") {
                            let id_str = trimmed.trim_start_matches("## X-Curse-Project-ID:").trim();
                            if let Ok(id_num) = id_str.parse::<i64>() {
                                curse_id = Some(id_num);
                            }
                        } else if trimmed.starts_with("## X-Part-Of:") || trimmed.starts_with("## X-Polestar-Part:") {
                            let p = trimmed.split_once(':').map(|x| x.1.trim().to_string()).unwrap_or_default();
                            if !p.is_empty() {
                                part_of = Some(p);
                            }
                        } else if trimmed.starts_with("## Dependencies:") || trimmed.starts_with("## RequiredDeps:") {
                            let deps_str = trimmed.split_once(':').map(|x| x.1.trim()).unwrap_or("");
                            for d in deps_str.replace(';', ",").split(',') {
                                let dt = d.trim();
                                if !dt.is_empty() {
                                    dependencies.push(dt.to_string());
                                }
                            }
                        } else if trimmed.starts_with("## LoadOnDemand:") {
                            let v = trimmed.trim_start_matches("## LoadOnDemand:").trim();
                            load_on_demand = v == "1" || v.eq_ignore_ascii_case("true");
                        }
                    }
                }
            }

            // Check .loom_addon.json metadata if present
            let meta_file = folder_path.join(".loom_addon.json");
            let mut file_id = None;
            let mut icon_url = None;
            let mut latest_version = None;
            let mut download_url = None;

            if meta_file.exists() {
                if let Ok(meta_content) = fs::read_to_string(&meta_file) {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&meta_content) {
                        if curse_id.is_none() {
                            curse_id = val.get("modId").and_then(|v| v.as_i64());
                        }
                        file_id = val.get("fileId").and_then(|v| v.as_i64());
                        icon_url = val.get("iconUrl").and_then(|v| v.as_str()).map(|s| s.to_string());
                        latest_version = val.get("latestVersion").and_then(|v| v.as_str()).map(|s| s.to_string());
                        download_url = val.get("downloadUrl").and_then(|v| v.as_str()).map(|s| s.to_string());
                    }
                }
            }

            let size_bytes = get_dir_size(&folder_path);
            let mb = (size_bytes as f64) / (1024.0 * 1024.0);
            let size_formatted = if mb < 0.1 {
                format!("{} Ko", size_bytes / 1024)
            } else {
                format!("{:.1} Mo", mb)
            };

            let mut last_modified = 0;
            if let Ok(meta) = fs::metadata(&folder_path) {
                if let Ok(mtime) = meta.modified() {
                    if let Ok(dur) = mtime.duration_since(std::time::UNIX_EPOCH) {
                        last_modified = dur.as_secs() as i64;
                    }
                }
            }

            raw_addons.push(RawAddonMeta {
                folder_name,
                title,
                version,
                author,
                notes,
                website,
                category,
                interface_version,
                curse_id,
                file_id,
                icon_url,
                latest_version,
                download_url,
                size_bytes,
                _size_formatted: size_formatted,
                last_modified,
                enabled: is_enabled,
                part_of,
                dependencies,
                _load_on_demand: load_on_demand,
            });
        }
    }

    // ── Phase 2 : Enrichissement via configurations existantes (WowUp) ────────
    let wowup_map = load_wowup_mappings(&inst_path);
    for a in &mut raw_addons {
        let f_low = a.folder_name.to_lowercase();
        if let Some(entry) = wowup_map.get(&f_low) {
            // WowUp mapping takes precedence over legacy TOC X-Curse-Project-ID
            if let Some(mid) = entry.external_id {
                a.curse_id = Some(mid);
            }
            if a.file_id.is_none() {
                a.file_id = entry.installed_file_id;
            }
            if a.version.is_empty() || a.version == "@project-version@" || a.version == "0.0.0" {
                if let Some(ref wv) = entry.installed_version {
                    if !wv.is_empty() {
                        a.version = wv.clone();
                    }
                }
            }
        }
    }

    // ── Phase 3 : Résolution CurseForge pour les dossiers racines sans curse_id ─
    let client = Client::new();
    let cf_key = CURSEFORGE_API_KEY;

    // Collect candidate folder names that still need a CurseForge lookup
    // Only search root folders (skip obvious secondary folders like *_Options, *_Data, etc.)
    for a in &mut raw_addons {
        if a.curse_id.is_some() {
            continue;
        }

        // Test slugs: exact folder name lowercase, and replacing '_' with '-'
        let folder_lower = a.folder_name.to_lowercase();
        // Remove leading '!' if any (e.g. !BugGrabber -> buggrabber)
        let clean_name = folder_lower.trim_start_matches('!');
        let slug_candidates = [
            clean_name.to_string(),
            clean_name.replace('_', "-"),
        ];

        for slug in &slug_candidates {
            let url = format!("https://api.curseforge.com/v1/mods/search?gameId=1&slug={}", slug);
            if let Ok(resp) = client
                .get(&url)
                .header("x-api-key", cf_key)
                .header("Accept", "application/json")
                .send()
                .await
            {
                if resp.status().is_success() {
                    if let Ok(data) = resp.json::<serde_json::Value>().await {
                        if let Some(arr) = data.get("data").and_then(|v| v.as_array()) {
                            if let Some(first_match) = arr.first() {
                                if let Some(mid) = first_match.get("id").and_then(|v| v.as_i64()) {
                                    a.curse_id = Some(mid);
                                    if a.icon_url.is_none() {
                                        a.icon_url = first_match.get("logo")
                                            .and_then(|l| l.get("thumbnailUrl").or_else(|| l.get("url")))
                                            .and_then(|u| u.as_str())
                                            .map(|s| s.to_string());
                                    }
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // ── Phase 4 : CurseForge Metadata & Module Matching (La méthode officielle) ──
    // On rassemble tous les modIds identifiés et on interroge l'API CurseForge /v1/mods
    let mut all_mod_ids: HashSet<i64> = raw_addons.iter().filter_map(|a| a.curse_id).collect();
    // Also include mod IDs from WowUp map
    for entry in wowup_map.values() {
        if let Some(mid) = entry.external_id {
            all_mod_ids.insert(mid);
        }
    }

    let mod_ids_vec: Vec<i64> = all_mod_ids.into_iter().collect();

    // Map: folder_name (lowercase) -> ModId
    let mut folder_to_mod_id: HashMap<String, i64> = HashMap::new();
    // Map: ModId -> CurseForge Mod Info (name, icon, latest_version, file_id, download_url)
    struct CfModInfo {
        name: String,
        icon_url: Option<String>,
        file_id: Option<i64>,
        latest_version: Option<String>,
        download_url: Option<String>,
        modules: Vec<String>,
    }
    let mut cf_mods_info: HashMap<i64, CfModInfo> = HashMap::new();

    if !mod_ids_vec.is_empty() {
        // Query in chunks of 50 to respect CurseForge API limits
        for chunk in mod_ids_vec.chunks(50) {
            let cf_res = client
                .post("https://api.curseforge.com/v1/mods")
                .header("Content-Type", "application/json")
                .header("x-api-key", cf_key)
                .json(&serde_json::json!({ "modIds": chunk }))
                .send()
                .await;

            if let Ok(res) = cf_res {
                if res.status().is_success() {
                    if let Ok(data) = res.json::<serde_json::Value>().await {
                        if let Some(mods_arr) = data.get("data").and_then(|v| v.as_array()) {
                            for m in mods_arr {
                                if let Some(mid) = m.get("id").and_then(|v| v.as_i64()) {
                                    let name = m.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                    let icon = m.get("logo")
                                        .and_then(|l| l.get("thumbnailUrl").or_else(|| l.get("url")))
                                        .and_then(|u| u.as_str())
                                        .map(|s| s.to_string());

                                    let mut file_id = None;
                                    let mut latest_version = None;
                                    let mut download_url = None;
                                    let mut module_names = Vec::new();

                                    if let Some(files) = m.get("latestFiles").and_then(|v| v.as_array()) {
                                        if let Some(f) = pick_best_curseforge_file(files, game_version_type_id) {
                                            file_id = f.get("id").and_then(|v| v.as_i64());
                                            latest_version = f.get("displayName")
                                                .or_else(|| f.get("fileName"))
                                                .and_then(|v| v.as_str())
                                                .map(|s| s.to_string());
                                            download_url = f.get("downloadUrl").and_then(|v| v.as_str()).map(|s| s.to_string());

                                            // Fallback CDN if downloadUrl is missing
                                            if download_url.is_none() {
                                                if let (Some(fid), Some(fname)) = (file_id, f.get("fileName").and_then(|v| v.as_str())) {
                                                    let p1 = fid / 1000;
                                                    let p2 = fid % 1000;
                                                    download_url = Some(format!("https://edge.forgecdn.net/files/{}/{}/{}", p1, p2, fname));
                                                }
                                            }

                                            if let Some(mods) = f.get("modules").and_then(|v| v.as_array()) {
                                                for mod_item in mods {
                                                    if let Some(mod_name) = mod_item.get("name").and_then(|v| v.as_str()) {
                                                        module_names.push(mod_name.to_string());
                                                        // Associate this module folder with this ModId!
                                                        folder_to_mod_id.insert(mod_name.to_lowercase(), mid);
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    cf_mods_info.insert(mid, CfModInfo {
                                        name,
                                        icon_url: icon,
                                        file_id,
                                        latest_version,
                                        download_url,
                                        modules: module_names,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Also populate folder_to_mod_id from WowUp installedFolders mappings
    for (folder_lower, entry) in &wowup_map {
        if let Some(mid) = entry.external_id {
            for f in &entry.folder_list {
                folder_to_mod_id.entry(f.to_lowercase()).or_insert(mid);
            }
            folder_to_mod_id.entry(folder_lower.clone()).or_insert(mid);
        }
    }

    // Also link any addon that already had curse_id directly
    for a in &raw_addons {
        if let Some(cid) = a.curse_id {
            folder_to_mod_id.entry(a.folder_name.to_lowercase()).or_insert(cid);
        }
    }

    // ── Phase 5 : Dépendances TOC & X-Part-Of pour les dossiers secondaires ───
    for a in &raw_addons {
        let a_lower = a.folder_name.to_lowercase();
        if folder_to_mod_id.contains_key(&a_lower) {
            continue;
        }

        // 1. Tag X-Part-Of (ex: Spoken_Bosses -> PartOf: Spoken)
        if let Some(ref parent) = a.part_of {
            if let Some(&mid) = folder_to_mod_id.get(&parent.to_lowercase()) {
                folder_to_mod_id.insert(a_lower.clone(), mid);
                continue;
            }
        }

        // 2. Dependencies / RequiredDeps (ex: DBM-GUI -> Dependencies: DBM-Core)
        for dep in &a.dependencies {
            if let Some(&mid) = folder_to_mod_id.get(&dep.to_lowercase()) {
                folder_to_mod_id.insert(a_lower.clone(), mid);
                break;
            }
        }
    }

    // ── Phase 6 : Construction de la liste finale d'addons ────────────────────
    // Regroupement par Mod ID pour tous les addons rattachés à CurseForge
    let mut curse_groups: HashMap<i64, Vec<usize>> = HashMap::new();
    let mut orphan_indices: Vec<usize> = Vec::new();

    for (i, a) in raw_addons.iter().enumerate() {
        let lower = a.folder_name.to_lowercase();
        if let Some(&mid) = folder_to_mod_id.get(&lower) {
            curse_groups.entry(mid).or_default().push(i);
        } else {
            orphan_indices.push(i);
        }
    }

    let mut final_addons: Vec<InstalledWowAddonRecord> = Vec::new();

    // A) Création des addons CurseForge (un seul record par mod CurseForge)
    for (mid, indices) in &curse_groups {
        let cf_info = cf_mods_info.get(mid);

        // Déterminer le dossier principal du mod :
        // 1. Si un dossier correspond au premier module officiel de CF
        // 2. Sinon, le dossier dont le nom est le plus court
        let mut sorted_indices = indices.clone();
        sorted_indices.sort_by_key(|&i| {
            let fname = &raw_addons[i].folder_name;
            let is_exact_module = cf_info.map(|c| c.modules.first().map(|m| m.eq_ignore_ascii_case(fname)).unwrap_or(false)).unwrap_or(false);
            let priority = if is_exact_module { 0 } else { 1 };
            (priority, fname.len(), fname.to_lowercase())
        });

        let primary_idx = sorted_indices[0];
        let p = &raw_addons[primary_idx];

        // 1. Déterminer le file_id installé à travers tous les sous-dossiers :
        let mut installed_file_id = None;
        for &idx in &sorted_indices {
            if let Some(fid) = raw_addons[idx].file_id {
                installed_file_id = Some(fid);
                break;
            }
        }
        if installed_file_id.is_none() {
            for &idx in &sorted_indices {
                let f_low = raw_addons[idx].folder_name.to_lowercase();
                if let Some(entry) = wowup_map.get(&f_low) {
                    if let Some(wfid) = entry.installed_file_id {
                        installed_file_id = Some(wfid);
                        break;
                    }
                }
            }
        }

        // 2. Déterminer la version installée :
        let mut installed_version = p.version.clone();
        if installed_version.is_empty() || installed_version == "@project-version@" || installed_version == "0.0.0" {
            for &idx in &sorted_indices {
                let v = &raw_addons[idx].version;
                if !v.is_empty() && v != "@project-version@" && v != "0.0.0" {
                    installed_version = v.clone();
                    break;
                }
            }
        }
        if installed_version.is_empty() || installed_version == "@project-version@" || installed_version == "0.0.0" {
            for &idx in &sorted_indices {
                let f_low = raw_addons[idx].folder_name.to_lowercase();
                if let Some(entry) = wowup_map.get(&f_low) {
                    if let Some(ref wv) = entry.installed_version {
                        if !wv.is_empty() {
                            installed_version = wv.clone();
                            break;
                        }
                    }
                }
            }
        }

        let title = cf_info.map(|c| c.name.clone())
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| p.title.clone());

        let icon_url = cf_info.and_then(|c| c.icon_url.clone())
            .or_else(|| p.icon_url.clone());

        let latest_version = cf_info.and_then(|c| c.latest_version.clone())
            .or_else(|| p.latest_version.clone());

        let download_url = cf_info.and_then(|c| c.download_url.clone())
            .or_else(|| p.download_url.clone());

        let latest_file_id = cf_info.and_then(|c| c.file_id);

        let mut secondary_folders = Vec::new();
        let mut total_size = p.size_bytes;

        for &sec_idx in &sorted_indices[1..] {
            let sec = &raw_addons[sec_idx];
            secondary_folders.push(sec.folder_name.clone());
            total_size += sec.size_bytes;
        }

        // Determine if update is available
        let mut update_available = false;
        if check_updates {
            if let (Some(cur_fid), Some(lfid)) = (installed_file_id, latest_file_id) {
                if lfid > cur_fid {
                    update_available = true;
                }
            } else if let Some(ref lat_ver) = latest_version {
                if is_newer_version(&installed_version, lat_ver) {
                    update_available = true;
                }
            }
        }

        let mb = (total_size as f64) / (1024.0 * 1024.0);
        let size_formatted = if mb < 0.1 {
            format!("{} Ko", total_size / 1024)
        } else {
            format!("{:.1} Mo", mb)
        };

        final_addons.push(InstalledWowAddonRecord {
            id: format!("cf-{}", mid),
            folder_name: p.folder_name.clone(),
            title,
            version: installed_version,
            author: p.author.clone(),
            notes: p.notes.clone(),
            website: p.website.clone(),
            category: p.category.clone(),
            interface_version: p.interface_version.clone(),
            size_bytes: total_size,
            size_formatted,
            last_modified: p.last_modified,
            enabled: p.enabled,
            update_available: Some(update_available),
            latest_version,
            download_url,
            curseforge_id: Some(*mid),
            file_id: installed_file_id,
            latest_file_id,
            icon_url,
            secondary_folders,
        });
    }

    // B) Regroupement des orphelins restants (non trouvés sur CurseForge)
    // On regroupe les sous-modules orphelins (ex: Narcissus_Achievements -> Narcissus, TradeSkillMaster_AppHelper -> TradeSkillMaster)
    let mut orphan_secondary_set: HashSet<usize> = HashSet::new();
    let mut orphan_parent_map: HashMap<usize, Vec<String>> = HashMap::new();

    // Sort orphan_indices by length so shorter (parents) are processed first
    let mut sorted_orphans = orphan_indices.clone();
    sorted_orphans.sort_by_key(|&i| raw_addons[i].folder_name.len());

    for &i in &sorted_orphans {
        let child_name = &raw_addons[i].folder_name;
        let child_deps = &raw_addons[i].dependencies;
        let child_part_of = &raw_addons[i].part_of;

        // Find candidate parent among other orphans
        let parent: Option<usize> = sorted_orphans
            .iter()
            .filter(|&&j| j != i && !orphan_secondary_set.contains(&j))
            .find(|&&j| {
                let p_name = &raw_addons[j].folder_name;
                // Match via X-Part-Of
                if let Some(ref part) = child_part_of {
                    if part.eq_ignore_ascii_case(p_name) {
                        return true;
                    }
                }
                // Match via Dependencies
                if child_deps.iter().any(|d| d.eq_ignore_ascii_case(p_name)) {
                    return true;
                }
                // Match via Underscore prefix
                let p_lower = p_name.to_lowercase();
                let c_lower = child_name.to_lowercase();
                if c_lower.starts_with(&format!("{}_", p_lower)) {
                    return true;
                }
                false
            })
            .copied();

        if let Some(p_idx) = parent {
            orphan_parent_map.entry(p_idx).or_default().push(child_name.clone());
            orphan_secondary_set.insert(i);
        }
    }

    for &i in &orphan_indices {
        if orphan_secondary_set.contains(&i) {
            continue;
        }
        let o = &raw_addons[i];
        let mut total_size = o.size_bytes;
        let mut secondary_folders = Vec::new();

        if let Some(secs) = orphan_parent_map.remove(&i) {
            for sec_name in secs {
                if let Some(sec) = raw_addons.iter().find(|a| a.folder_name == sec_name) {
                    total_size += sec.size_bytes;
                }
                secondary_folders.push(sec_name);
            }
        }

        let mb = (total_size as f64) / (1024.0 * 1024.0);
        let size_formatted = if mb < 0.1 {
            format!("{} Ko", total_size / 1024)
        } else {
            format!("{:.1} Mo", mb)
        };

        final_addons.push(InstalledWowAddonRecord {
            id: o.folder_name.to_lowercase(),
            folder_name: o.folder_name.clone(),
            title: o.title.clone(),
            version: o.version.clone(),
            author: o.author.clone(),
            notes: o.notes.clone(),
            website: o.website.clone(),
            category: o.category.clone(),
            interface_version: o.interface_version.clone(),
            size_bytes: total_size,
            size_formatted,
            last_modified: o.last_modified,
            enabled: o.enabled,
            update_available: Some(false),
            latest_version: o.latest_version.clone(),
            download_url: o.download_url.clone(),
            curseforge_id: None,
            file_id: None,
            latest_file_id: None,
            icon_url: None,
            secondary_folders,
        });
    }

    // Tri alphabétique final par titre
    final_addons.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));

    Ok(final_addons)
}

/// Installs a WoW AddOn ZIP file into instance's Interface/AddOns directory
pub async fn install_wow_addon_impl(
    instance_path: String,
    download_url: String,
    mod_id: Option<i64>,
    file_id: Option<i64>,
    mod_name: Option<String>,
    version: Option<String>,
    icon_url: Option<String>,
) -> Result<InstallAddonResult, String> {
    let inst_path = PathBuf::from(&instance_path);
    let addons_dir = if inst_path.ends_with("AddOns") {
        inst_path.clone()
    } else {
        inst_path.join("Interface").join("AddOns")
    };

    fs::create_dir_all(&addons_dir).map_err(|e| format!("Impossible de créer le dossier AddOns: {}", e))?;

    // Download ZIP
    let client = Client::new();
    let resp = client
        .get(&download_url)
        .header("User-Agent", "LoomLauncher/1.0")
        .send()
        .await
        .map_err(|e| format!("Erreur lors du téléchargement de l'addon: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Échec du téléchargement (HTTP {})", resp.status()));
    }

    let bytes = resp.bytes().await.map_err(|e| format!("Erreur lecture des octets: {}", e))?;
    let reader = Cursor::new(bytes);
    let mut archive = ZipArchive::new(reader).map_err(|e| format!("Archive ZIP invalide: {}", e))?;

    let mut top_folders = HashSet::new();

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| format!("Erreur lecture fichier ZIP: {}", e))?;
        let outpath = match file.enclosed_name() {
            Some(path) => addons_dir.join(path),
            None => continue,
        };

        // Determine top folder
        if let Some(enc) = file.enclosed_name() {
            if let Some(comp) = enc.components().next() {
                top_folders.insert(comp.as_os_str().to_string_lossy().to_string());
            }
        }

        if file.name().ends_with('/') {
            fs::create_dir_all(&outpath).map_err(|e| format!("Erreur création dossier: {}", e))?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(p).map_err(|e| format!("Erreur création parent: {}", e))?;
                }
            }
            let mut outfile = File::create(&outpath).map_err(|e| format!("Erreur écriture fichier: {}", e))?;
            std::io::copy(&mut file, &mut outfile).map_err(|e| format!("Erreur extraction: {}", e))?;
        }
    }

    // Write .loom_addon.json metadata into each top folder
    let meta_json = serde_json::json!({
        "modId": mod_id,
        "fileId": file_id,
        "modName": mod_name,
        "version": version,
        "iconUrl": icon_url,
        "downloadUrl": download_url,
        "installedAt": chrono::Utc::now().to_rfc3339()
    });

    let meta_str = serde_json::to_string_pretty(&meta_json).unwrap_or_default();
    for f in &top_folders {
        let target_meta = addons_dir.join(f).join(".loom_addon.json");
        let _ = fs::write(target_meta, &meta_str);
    }

    let mut folders_vec: Vec<String> = top_folders.into_iter().collect();
    folders_vec.sort();

    Ok(InstallAddonResult {
        success: true,
        folders: folders_vec,
    })
}

/// Uninstalls a WoW AddOn (main folder + secondary sub-folders)
pub fn uninstall_wow_addon_impl(
    instance_path: String,
    folder_name: String,
    secondary_folders: Vec<String>,
) -> Result<(), String> {
    let inst_path = PathBuf::from(&instance_path);
    let addons_dir = if inst_path.ends_with("AddOns") {
        inst_path.clone()
    } else {
        inst_path.join("Interface").join("AddOns")
    };

    let main_dir = addons_dir.join(&folder_name);
    if main_dir.exists() {
        let _ = fs::remove_dir_all(main_dir);
    }

    for sec in secondary_folders {
        let sec_dir = addons_dir.join(&sec);
        if sec_dir.exists() {
            let _ = fs::remove_dir_all(sec_dir);
        }
    }

    Ok(())
}

/// Toggles addon state (renames .toc <-> .toc.disabled)
pub fn toggle_wow_addon_impl(
    instance_path: String,
    folder_name: String,
    secondary_folders: Vec<String>,
    enabled: bool,
) -> Result<bool, String> {
    let inst_path = PathBuf::from(&instance_path);
    let addons_dir = if inst_path.ends_with("AddOns") {
        inst_path.clone()
    } else {
        inst_path.join("Interface").join("AddOns")
    };

    let mut all_folders = vec![folder_name];
    all_folders.extend(secondary_folders);

    for f_name in all_folders {
        let folder = addons_dir.join(&f_name);
        if !folder.is_dir() {
            continue;
        }

        if let Ok(entries) = fs::read_dir(&folder) {
            for entry in entries.flatten() {
                let p = entry.path();
                let file_name = p.file_name().unwrap_or_default().to_string_lossy().to_string();

                if enabled {
                    // Re-enable: .toc.disabled -> .toc
                    if file_name.ends_with(".toc.disabled") {
                        let new_name = file_name.trim_end_matches(".disabled");
                        let new_path = folder.join(new_name);
                        let _ = fs::rename(&p, &new_path);
                    }
                } else {
                    // Disable: .toc -> .toc.disabled
                    if file_name.ends_with(".toc") && !file_name.ends_with(".toc.disabled") {
                        let new_name = format!("{}.disabled", file_name);
                        let new_path = folder.join(new_name);
                        let _ = fs::rename(&p, &new_path);
                    }
                }
            }
        }
    }

    Ok(enabled)
}

/// Opens the instance Interface/AddOns directory in system file explorer
pub fn open_wow_addons_folder_impl(instance_path: String) -> Result<(), String> {
    let inst_path = PathBuf::from(&instance_path);
    let addons_dir = if inst_path.ends_with("AddOns") {
        inst_path.clone()
    } else {
        inst_path.join("Interface").join("AddOns")
    };

    let _ = fs::create_dir_all(&addons_dir);

    #[cfg(target_os = "linux")]
    {
        if std::process::Command::new("xdg-open").arg(&addons_dir).spawn().is_err() {
            let _ = std::process::Command::new("gio").args(["open", &addons_dir.to_string_lossy()]).spawn();
        }
    }
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("explorer.exe").arg(&addons_dir).spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(&addons_dir).spawn();
    }

    Ok(())
}

/// Creates a backup ZIP file of AddOns and WTF configuration
pub fn export_wow_addons_impl(instance_path: String, flavor: Option<String>) -> Result<String, String> {
    let inst_path = PathBuf::from(&instance_path);
    if !inst_path.exists() {
        return Err("Chemin d'instance WoW introuvable".to_string());
    }

    let download_dir = dirs::download_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")).join("Downloads"));
    let _ = fs::create_dir_all(&download_dir);

    let date_str = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let flavor_name = flavor.unwrap_or_else(|| "addons".to_string());
    let filename = format!("loom-wow-backup-{}-{}.zip", flavor_name, date_str);
    let out_file_path = download_dir.join(&filename);

    let file = File::create(&out_file_path).map_err(|e| format!("Impossible de créer le fichier ZIP: {}", e))?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    // Helper closure to add directory recursively to zip
    fn add_dir_to_zip<W: Write + std::io::Seek>(
        zip: &mut ZipWriter<W>,
        src_dir: &Path,
        prefix: &str,
        options: SimpleFileOptions,
    ) -> Result<(), String> {
        if !src_dir.exists() {
            return Ok(());
        }
        if let Ok(entries) = fs::read_dir(src_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                let zip_path = if prefix.is_empty() {
                    name.clone()
                } else {
                    format!("{}/{}", prefix, name)
                };

                if path.is_dir() {
                    let _ = zip.add_directory(&zip_path, options);
                    let _ = add_dir_to_zip(zip, &path, &zip_path, options);
                } else if path.is_file() {
                    if let Ok(mut f) = File::open(&path) {
                        let mut buf = Vec::new();
                        if f.read_to_end(&mut buf).is_ok() {
                            let _ = zip.start_file(&zip_path, options);
                            let _ = zip.write_all(&buf);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    let addons_dir = inst_path.join("Interface").join("AddOns");
    if addons_dir.exists() {
        let _ = add_dir_to_zip(&mut zip, &addons_dir, "Interface/AddOns", options);
    }

    let wtf_dir = inst_path.join("WTF");
    if wtf_dir.exists() {
        let _ = add_dir_to_zip(&mut zip, &wtf_dir, "WTF", options);
    }

    zip.finish().map_err(|e| format!("Erreur finalisation ZIP: {}", e))?;

    Ok(out_file_path.to_string_lossy().to_string())
}

/// Helper mapping class IDs to official WoW class names
fn wow_class_id_to_name(id: u32) -> &'static str {
    match id {
        1 => "WARRIOR",
        2 => "PALADIN",
        3 => "HUNTER",
        4 => "ROGUE",
        5 => "PRIEST",
        6 => "DEATHKNIGHT",
        7 => "SHAMAN",
        8 => "MAGE",
        9 => "WARLOCK",
        10 => "MONK",
        11 => "DRUID",
        12 => "DEMONHUNTER",
        13 => "EVOKER",
        _ => "UNKNOWN",
    }
}

/// Scans and retrieves all characters belonging to a specific WoW instance
pub fn get_wow_characters_impl(instance_path: &str) -> Vec<WowCharacterRecord> {
    let inst_path = PathBuf::from(instance_path);
    let account_root = inst_path.join("WTF").join("Account");

    if !account_root.exists() || !account_root.is_dir() {
        return Vec::new();
    }

    let mut characters = Vec::new();

    let Ok(account_entries) = fs::read_dir(&account_root) else {
        return Vec::new();
    };

    for acc_entry in account_entries.flatten() {
        let acc_path = acc_entry.path();
        if !acc_path.is_dir() {
            continue;
        }

        let acc_name = acc_entry.file_name().to_string_lossy().to_string();
        if acc_name == "SavedVariables" {
            continue;
        }

        // Cache metadata extracted from SavedVariables
        let mut char_meta_map: HashMap<String, (Option<String>, Option<u32>, Option<String>, Option<String>, Option<String>)> = HashMap::new();

        // 1. Inspect BtWQuests.lua
        let btw_file = acc_path.join("SavedVariables").join("BtWQuests.lua");
        if btw_file.exists() {
            if let Ok(content) = fs::read_to_string(&btw_file) {
                // Parse blocks in BtWQuests_Characters = { ... }
                let lines: Vec<&str> = content.lines().collect();
                let mut in_char_block = false;
                let mut depth: i32 = 0;
                let mut cur_name: Option<String> = None;
                let mut cur_class: Option<String> = None;
                let mut cur_race: Option<String> = None;
                let mut cur_level: Option<u32> = None;
                let mut cur_realm: Option<String> = None;
                let mut cur_faction: Option<String> = None;

                for line in lines {
                    let trimmed = line.trim();
                    if trimmed.starts_with("BtWQuests_Characters = {") {
                        in_char_block = true;
                        depth = 1;
                        continue;
                    }

                    if in_char_block {
                        // Track Lua table nesting depth
                        if trimmed == "{" || trimmed.ends_with("= {") {
                            depth += 1;
                        }

                        // Only parse character attributes at depth == 2 (direct fields of character table)
                        if depth == 2 {
                            if trimmed.starts_with("[\"name\"] = \"") {
                                let val = trimmed.trim_start_matches("[\"name\"] = \"").trim_end_matches("\",").trim_end_matches('"');
                                cur_name = Some(val.to_string());
                            } else if trimmed.starts_with("[\"class\"] = ") {
                                let val = trimmed.trim_start_matches("[\"class\"] = ").trim_end_matches(',').trim();
                                if let Ok(num) = val.parse::<u32>() {
                                    cur_class = Some(wow_class_id_to_name(num).to_string());
                                }
                            } else if trimmed.starts_with("[\"race\"] = \"") {
                                let val = trimmed.trim_start_matches("[\"race\"] = \"").trim_end_matches("\",").trim_end_matches('"');
                                cur_race = Some(val.to_string());
                            } else if trimmed.starts_with("[\"level\"] = ") {
                                let val = trimmed.trim_start_matches("[\"level\"] = ").trim_end_matches(',').trim();
                                if let Ok(num) = val.parse::<u32>() {
                                    cur_level = Some(num);
                                }
                            } else if trimmed.starts_with("[\"realm\"] = \"") {
                                let val = trimmed.trim_start_matches("[\"realm\"] = \"").trim_end_matches("\",").trim_end_matches('"');
                                cur_realm = Some(val.to_string());
                            } else if trimmed.starts_with("[\"faction\"] = \"") {
                                let val = trimmed.trim_start_matches("[\"faction\"] = \"").trim_end_matches("\",").trim_end_matches('"');
                                cur_faction = Some(val.to_string());
                            }
                        }

                        if trimmed == "}," || trimmed == "}" {
                            depth -= 1;
                            if depth == 1 {
                                // End of a character block
                                if let Some(ref n) = cur_name {
                                    let meta = (
                                        cur_class.clone(),
                                        cur_level,
                                        cur_race.clone(),
                                        cur_realm.clone(),
                                        cur_faction.clone(),
                                    );
                                    char_meta_map.insert(n.to_lowercase(), meta.clone());
                                    char_meta_map.insert(n.replace(' ', "-").to_lowercase(), meta.clone());
                                    if let Some(first_word) = n.split_whitespace().next() {
                                        char_meta_map.insert(first_word.to_lowercase(), meta);
                                    }
                                }
                                cur_name = None;
                                cur_class = None;
                                cur_race = None;
                                cur_level = None;
                                cur_realm = None;
                                cur_faction = None;
                            } else if depth <= 0 {
                                in_char_block = false;
                            }
                        }
                    }
                }
            }
        }

        // 2. Inspect Attune.lua for class/level fallback
        let attune_file = acc_path.join("SavedVariables").join("Attune.lua");
        if attune_file.exists() {
            if let Ok(content) = fs::read_to_string(&attune_file) {
                let lines: Vec<&str> = content.lines().collect();
                let mut cur_toon = None;
                for line in lines {
                    let trimmed = line.trim();
                    if trimmed.starts_with("[\"") && trimmed.ends_with("\"] = {") && !trimmed.contains("stats") && !trimmed.contains("skip") {
                        let toon_name = trimmed.trim_start_matches("[\"").trim_end_matches("\"] = {");
                        cur_toon = Some(toon_name.to_lowercase());
                    } else if let Some(ref tname) = cur_toon {
                        if trimmed.starts_with("[\"class\"] = \"") {
                            let cls = trimmed.trim_start_matches("[\"class\"] = \"").trim_end_matches("\",").trim_end_matches('"');
                            char_meta_map.entry(tname.clone()).or_insert_with(|| (None, None, None, None, None)).0 = Some(cls.to_string());
                        } else if trimmed.starts_with("[\"level\"] = \"") || trimmed.starts_with("[\"level\"] = ") {
                            let lvl_str = trimmed.trim_start_matches("[\"level\"] = ").trim_matches('"').trim_end_matches(',').trim();
                            if let Ok(lvl) = lvl_str.parse::<u32>() {
                                char_meta_map.entry(tname.clone()).or_insert_with(|| (None, None, None, None, None)).1 = Some(lvl);
                            }
                        }
                    }
                }
            }
        }

        // 3. Inspect Realm subdirectories for characters
        let Ok(realm_entries) = fs::read_dir(&acc_path) else {
            continue;
        };

        for r_entry in realm_entries.flatten() {
            let r_path = r_entry.path();
            if !r_path.is_dir() {
                continue;
            }

            let r_name = r_entry.file_name().to_string_lossy().to_string();
            if r_name == "SavedVariables" {
                continue;
            }

            let Ok(char_entries) = fs::read_dir(&r_path) else {
                continue;
            };

            for c_entry in char_entries.flatten() {
                let c_path = c_entry.path();
                if !c_path.is_dir() {
                    continue;
                }

                let full_name = c_entry.file_name().to_string_lossy().to_string();
                if full_name == "SavedVariables" {
                    continue;
                }

                // Determine latest modified timestamp in this character folder
                let mut max_mtime = 0;
                if let Ok(files) = fs::read_dir(&c_path) {
                    for f in files.flatten() {
                        if let Ok(meta) = f.metadata() {
                            if let Ok(mt) = meta.modified() {
                                if let Ok(dur) = mt.duration_since(std::time::UNIX_EPOCH) {
                                    let s = dur.as_secs() as i64;
                                    if s > max_mtime {
                                        max_mtime = s;
                                    }
                                }
                            }
                        }
                    }
                }

                let display_name = full_name.split('-').next().unwrap_or(&full_name).to_string();
                let lookup_keys = [
                    full_name.to_lowercase(),
                    display_name.to_lowercase(),
                    full_name.replace('-', " ").to_lowercase(),
                ];

                let mut char_class = None;
                let mut char_level = None;
                let mut char_race = None;
                let mut char_realm = r_name.clone();
                let mut char_faction = None;

                for k in &lookup_keys {
                    if let Some((cls, lvl, race, realm, faction)) = char_meta_map.get(k) {
                        if cls.is_some() && char_class.is_none() {
                            char_class = cls.clone();
                        }
                        if lvl.is_some() && char_level.is_none() {
                            char_level = *lvl;
                        }
                        if race.is_some() && char_race.is_none() {
                            char_race = race.clone();
                        }
                        if let Some(ref r) = realm {
                            char_realm = r.clone();
                        }
                        if faction.is_some() && char_faction.is_none() {
                            char_faction = faction.clone();
                        }
                    }
                }

                if char_class.is_none() {
                    let cfg_file = c_path.join("config-cache.wtf");
                    if cfg_file.exists() {
                        if let Ok(content) = fs::read_to_string(&cfg_file) {
                            for line in content.lines() {
                                let trimmed = line.trim();
                                if trimmed.starts_with("SET EJLootClass \"") {
                                    let cls_id_str = trimmed.trim_start_matches("SET EJLootClass \"").trim_end_matches('"');
                                    if let Ok(id) = cls_id_str.parse::<u32>() {
                                        char_class = Some(wow_class_id_to_name(id).to_string());
                                    }
                                }
                            }
                        }
                    }
                }

                // Fallback faction estimation from race if missing
                if char_faction.is_none() {
                    if let Some(ref r) = char_race {
                        let r_low = r.to_lowercase();
                        if matches!(r_low.as_str(), "dwarf" | "human" | "nightelf" | "gnome" | "draenei" | "worgen" | "nain" | "humain") {
                            char_faction = Some("Alliance".to_string());
                        } else {
                            char_faction = Some("Horde".to_string());
                        }
                    }
                }

                characters.push(WowCharacterRecord {
                    name: display_name,
                    full_name,
                    realm: char_realm,
                    account: acc_name.clone(),
                    level: char_level,
                    class: char_class,
                    race: char_race,
                    faction: char_faction,
                    last_played: if max_mtime > 0 { Some(max_mtime) } else { None },
                });
            }
        }
    }

    // Sort by last_played descending (most recently played first)
    characters.sort_by_key(|c| std::cmp::Reverse(c.last_played.unwrap_or(0)));
    characters
}

pub async fn search_wow_addons_impl(
    query: Option<String>,
    category_id: Option<u32>,
    game_version_type_id: Option<i64>,
    sort_field: Option<u32>,
    page_size: Option<u32>,
) -> Result<serde_json::Value, String> {
    let client = Client::new();
    let mut url = format!(
        "https://api.curseforge.com/v1/mods/search?gameId=1&pageSize={}",
        page_size.unwrap_or(36)
    );

    if let Some(sort) = sort_field {
        url.push_str(&format!("&sortField={}&sortOrder=desc", sort));
    }
    if let Some(cat) = category_id {
        if cat > 0 {
            url.push_str(&format!("&categoryId={}", cat));
        }
    }
    if let Some(gv) = game_version_type_id {
        if gv > 0 {
            url.push_str(&format!("&gameVersionTypeId={}", gv));
        }
    }
    if let Some(q) = query {
        let trimmed = q.trim();
        if !trimmed.is_empty() {
            url.push_str(&format!("&searchFilter={}", urlencoding::encode(trimmed)));
        }
    }

    let resp = client
        .get(&url)
        .header("x-api-key", CURSEFORGE_API_KEY)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("Erreur réseau lors de la recherche CurseForge: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("CurseForge a retourné une erreur HTTP {}", resp.status()));
    }

    let val = resp
        .json::<serde_json::Value>()
        .await
        .map_err(|e| format!("Erreur décodage JSON CurseForge: {}", e))?;

    Ok(val)
}

pub async fn get_wow_addon_details_impl(mod_id: i64) -> Result<serde_json::Value, String> {
    let client = Client::new();
    let url = format!("https://api.curseforge.com/v1/mods/{}", mod_id);

    let resp = client
        .get(&url)
        .header("x-api-key", CURSEFORGE_API_KEY)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("Erreur réseau lors de la récupération des détails CurseForge: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("CurseForge a retourné une erreur HTTP {}", resp.status()));
    }

    let val = resp
        .json::<serde_json::Value>()
        .await
        .map_err(|e| format!("Erreur décodage JSON CurseForge: {}", e))?;

    Ok(val)
}

pub async fn get_wow_addon_description_impl(mod_id: i64) -> Result<serde_json::Value, String> {
    let client = Client::new();
    let url = format!("https://api.curseforge.com/v1/mods/{}/description", mod_id);

    let resp = client
        .get(&url)
        .header("x-api-key", CURSEFORGE_API_KEY)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("Erreur réseau lors de la récupération de la description: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("CurseForge a retourné une erreur HTTP {}", resp.status()));
    }

    let val = resp
        .json::<serde_json::Value>()
        .await
        .map_err(|e| format!("Erreur décodage JSON CurseForge: {}", e))?;

    Ok(val)
}

pub async fn get_wow_addon_files_impl(mod_id: i64, page_size: Option<u32>) -> Result<serde_json::Value, String> {
    let client = Client::new();
    let size = page_size.unwrap_or(20);
    let url = format!("https://api.curseforge.com/v1/mods/{}/files?pageSize={}", mod_id, size);

    let resp = client
        .get(&url)
        .header("x-api-key", CURSEFORGE_API_KEY)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("Erreur réseau lors de la récupération des fichiers de l'addon: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("CurseForge a retourné une erreur HTTP {}", resp.status()));
    }

    let val = resp
        .json::<serde_json::Value>()
        .await
        .map_err(|e| format!("Erreur décodage JSON CurseForge: {}", e))?;

    Ok(val)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_search_wow_addons_curseforge() {
        let res = search_wow_addons_impl(Some("Details".to_string()), None, None, Some(2), Some(5)).await;
        assert!(res.is_ok(), "CurseForge WoW addon search should succeed: {:?}", res.err());
        let data = res.unwrap();
        let items = data.get("data").and_then(|v| v.as_array());
        assert!(items.is_some() && !items.unwrap().is_empty(), "Should return at least 1 mod for Details");
    }

    #[tokio::test]
    async fn test_scan_wow_addons() {
        let instances = scan_wow_instances_impl();
        if let Some(inst) = instances.iter().find(|i| i.installed && i.id == "forever") {
            let addons = get_wow_addons_impl(inst.path.clone(), true, Some(inst.game_version_type_id)).await;
            assert!(addons.is_ok());
            let list = addons.unwrap();
            println!("TOTAL ADDONS DETECTED: {}", list.len());
            let mut update_count = 0;
            for (idx, a) in list.iter().enumerate() {
                let upd = a.update_available.unwrap_or(false);
                if upd { update_count += 1; }
                println!("{:2}. {:35} | Ver: {:15} | Latest: {:15} | FID: {:?} -> {:?} | Upd: {}",
                    idx + 1, a.title, a.version, a.latest_version.as_deref().unwrap_or("-"),
                    a.file_id, a.latest_file_id, upd
                );
            }
            println!("TOTAL UPDATES AVAILABLE: {}", update_count);
            assert!(list.len() <= 60, "Addon list should be grouped to around ~30 addons, got {}", list.len());
        }
    }

    #[tokio::test]
    async fn test_multi_folder_addon_grouping_and_metadata() {
        let tmp_dir = std::env::temp_dir().join(format!("loom_test_wow_{}", uuid::Uuid::new_v4()));
        let addons_dir = tmp_dir.join("Interface").join("AddOns");
        let _ = fs::create_dir_all(&addons_dir);

        // Create a multi-folder addon (e.g. MyAddon and MyAddon_Sub) with .loom_addon.json
        let main_folder = addons_dir.join("MyAddon");
        let sub_folder = addons_dir.join("MyAddon_Sub");
        let _ = fs::create_dir_all(&main_folder);
        let _ = fs::create_dir_all(&sub_folder);

        let _ = fs::write(main_folder.join("MyAddon.toc"), "## Title: My Addon\n## Version: 1.0.0\n");
        let _ = fs::write(sub_folder.join("MyAddon_Sub.toc"), "## Title: My Addon Sub\n## Dependencies: MyAddon\n");

        let meta = serde_json::json!({
            "modId": 9999999,
            "fileId": 100,
            "modName": "My Addon",
            "version": "1.0.0"
        });
        let meta_str = serde_json::to_string(&meta).unwrap();
        let _ = fs::write(main_folder.join(".loom_addon.json"), &meta_str);
        let _ = fs::write(sub_folder.join(".loom_addon.json"), &meta_str);

        // Scan this isolated instance
        let addons = get_wow_addons_impl(tmp_dir.to_string_lossy().to_string(), false, None).await.unwrap();
        assert_eq!(addons.len(), 1, "Should group into exactly 1 addon");
        let a = &addons[0];
        assert_eq!(a.folder_name, "MyAddon");
        assert_eq!(a.secondary_folders, vec!["MyAddon_Sub"]);
        assert_eq!(a.file_id, Some(100));

        // Cleanup
        let _ = fs::remove_dir_all(&tmp_dir);
    }

    #[test]
    fn test_get_wow_characters() {
        let instances = scan_wow_instances_impl();
        if let Some(inst) = instances.iter().find(|i| i.installed && i.id == "forever") {
            let chars = get_wow_characters_impl(&inst.path);
            println!("Characters found in {}: {}", inst.name, chars.len());
            for c in &chars {
                println!(" - Character: {} | Realm: {} | Lvl: {:?} | Class: {:?} | Race: {:?} | Faction: {:?} | LastPlayed: {:?}",
                    c.name, c.realm, c.level, c.class, c.race, c.faction, c.last_played
                );
            }
        }
    }
}
