use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::db::{self, BattleNetGameRecord};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BattleNetGameMetadata {
    pub product_uid: String,
    pub name: String,
    pub install_path: String,
    pub executable_relative_path: String,
    pub version_build: Option<String>,
    pub build_key: Option<String>,
    pub branch: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ProductDbEntry {
    pub uid: String,
    pub product_code: Option<String>,
    pub install_path: String,
}

#[derive(Debug, Clone)]
pub struct BuildInfoRow {
    pub product: String,
    pub version: String,
    pub build_key: Option<String>,
    pub branch: Option<String>,
    pub active: bool,
}

/// Returns a human-friendly title for a Battle.net product UID
pub fn product_uid_to_name(uid: &str) -> &'static str {
    match uid.to_lowercase().as_str() {
        "wow" => "World of Warcraft",
        "wowf" | "wow_forever" => "World of Warcraft: Forever",
        "wow_classic" => "World of Warcraft Classic",
        "wow_classic_era" => "World of Warcraft Classic: Season of Discovery",
        "fenris" => "Diablo IV",
        "pro" => "Overwatch 2",
        "w3" => "Warcraft III: Reforged",
        "odin" => "Call of Duty: Modern Warfare",
        "diablo3" | "d3" => "Diablo III",
        "osi" => "Diablo II: Resurrected",
        "hs" | "hsb" => "Hearthstone",
        "sc2" | "s2" => "StarCraft II",
        "s1" => "StarCraft: Remastered",
        "hero" => "Heroes of the Storm",
        "rtro" => "Blizzard Arcade Collection",
        "anbs" => "Warcraft Arclight Rumble",
        "wlby" => "Crash Bandicoot 4: It's About Time",
        "zeus" => "Call of Duty: Black Ops Cold War",
        "viper" => "Call of Duty: Black Ops 4",
        "fore" => "Call of Duty: Vanguard",
        "lazarus" => "Call of Duty: Modern Warfare II",
        "aurora" => "Call of Duty: Warzone",
        _ => "Jeu Battle.net",
    }
}

/// Known executable paths for Blizzard games (strict game binaries only, no launcher stubs)
pub fn get_known_executable_relative_paths(product_uid: &str) -> Vec<&'static str> {
    match product_uid.to_lowercase().as_str() {
        "wow" | "wowf" | "wow_forever" => vec![
            "_forever_/Wow.exe",
            "_retail_/Wow.exe",
            "Wow.exe",
            "_classic_/WowClassic.exe",
            "_classic_era_/WowClassic.exe",
        ],
        "wow_classic" => vec!["_classic_/WowClassic.exe", "WowClassic.exe", "_classic_era_/WowClassic.exe"],
        "wow_classic_era" => vec!["_classic_era_/WowClassic.exe", "WowClassic.exe"],
        "fenris" => vec!["Diablo IV.exe"],
        "pro" => vec!["_retail_/Overwatch.exe", "Overwatch.exe"],
        "w3" => vec!["_retail_/x86_64/Warcraft III.exe", "Warcraft III.exe", "x86_64/Warcraft III.exe"],
        "odin" => vec!["ModernWarfare.exe", "CoD.exe"],
        "diablo3" | "d3" => vec!["x64/Diablo III64.exe", "Diablo III.exe", "Diablo III64.exe"],
        "osi" => vec!["D2R.exe"],
        "hs" | "hsb" => vec!["Hearthstone.exe"],
        "sc2" | "s2" => vec!["Support64/SC2Switcher_x64.exe", "Support/SC2Switcher.exe", "StarCraft II.exe"],
        "s1" => vec!["x86_64/StarCraft.exe", "StarCraft.exe"],
        "hero" => vec!["Support64/HeroesSwitcher_x64.exe", "Support/HeroesSwitcher.exe", "Heroes of the Storm.exe"],
        "rtro" => vec!["Blizzard Arcade Collection.exe"],
        _ => vec![],
    }
}

/// Converts a Windows/Wine path (e.g. C:\Games\World of Warcraft) to a Linux path inside the Wine prefix
pub fn resolve_wine_path(win_path: &str, prefix_path: &Path) -> PathBuf {
    let trimmed = win_path.trim().replace('\\', "/");

    // If it's already an absolute Linux path
    if trimmed.starts_with('/') {
        let p = PathBuf::from(&trimmed);
        if p.exists() {
            return p;
        }
    }

    // Windows drive format: C:/ or c:/
    if trimmed.len() >= 2 && trimmed.chars().nth(1) == Some(':') {
        let drive_letter = trimmed.chars().next().unwrap().to_ascii_lowercase();
        let rest = &trimmed[2..].trim_start_matches('/');

        if drive_letter == 'c' {
            let direct = prefix_path.join("drive_c").join(rest);
            if direct.exists() {
                return direct;
            }
            // Return direct path even if not yet created so caller knows the target
            return direct;
        } else {
            // Check dosdevices
            let dos_dev = prefix_path.join("dosdevices").join(format!("{}:", drive_letter)).join(rest);
            if dos_dev.exists() {
                return dos_dev;
            }
        }
    }

    // Fallback: check if relative to prefix drive_c
    prefix_path.join("drive_c").join(trimmed.trim_start_matches('/'))
}

// =========================================================================
// PROTOBUF DECODER FOR product.db
// =========================================================================

struct ProtoReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> ProtoReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn has_more(&self) -> bool {
        self.pos < self.data.len()
    }

    fn read_varint(&mut self) -> Option<u64> {
        let mut result: u64 = 0;
        let mut shift = 0;
        while self.pos < self.data.len() {
            let byte = self.data[self.pos];
            self.pos += 1;
            result |= ((byte & 0x7F) as u64) << shift;
            if (byte & 0x80) == 0 {
                return Some(result);
            }
            shift += 7;
            if shift >= 64 {
                return None;
            }
        }
        None
    }

    fn read_tag(&mut self) -> Option<(u32, u8)> {
        let key = self.read_varint()?;
        let field_num = (key >> 3) as u32;
        let wire_type = (key & 0x07) as u8;
        Some((field_num, wire_type))
    }

    fn read_bytes(&mut self) -> Option<&'a [u8]> {
        let len = self.read_varint()? as usize;
        if self.pos + len <= self.data.len() {
            let bytes = &self.data[self.pos..self.pos + len];
            self.pos += len;
            Some(bytes)
        } else {
            None
        }
    }

    fn read_string(&mut self) -> Option<String> {
        let bytes = self.read_bytes()?;
        String::from_utf8(bytes.to_vec()).ok()
    }

    fn skip_wire(&mut self, wire_type: u8) -> bool {
        match wire_type {
            0 => self.read_varint().is_some(),
            1 => {
                if self.pos + 8 <= self.data.len() {
                    self.pos += 8;
                    true
                } else {
                    false
                }
            }
            2 => {
                if let Some(len) = self.read_varint() {
                    let len = len as usize;
                    if self.pos + len <= self.data.len() {
                        self.pos += len;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            5 => {
                if self.pos + 4 <= self.data.len() {
                    self.pos += 4;
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}

/// Parses the Agent `product.db` protobuf database.
/// `Database` contains repeated `ProductInstall` (tag 1, wire 2).
/// `ProductInstall`:
///   tag 1 (wire 2) -> uid (string)
///   tag 2 (wire 2) -> productCode (string)
///   tag 3 (wire 2) -> settings (sub-message)
///       tag 1 (wire 2) -> installPath (string)
pub fn parse_product_db(product_db_path: &Path) -> Result<Vec<ProductDbEntry>, String> {
    if !product_db_path.exists() {
        return Err(format!("Fichier product.db introuvable à l'emplacement: {:?}", product_db_path));
    }

    let bytes = fs::read(product_db_path).map_err(|e| format!("Impossible de lire product.db: {}", e))?;
    let mut entries = Vec::new();

    // 1. Structured Protobuf decoding
    let mut reader = ProtoReader::new(&bytes);
    while reader.has_more() {
        if let Some((tag, wire_type)) = reader.read_tag() {
            if tag == 1 && wire_type == 2 {
                // Repeated ProductInstall
                if let Some(sub_bytes) = reader.read_bytes() {
                    if let Some(entry) = parse_product_install_message(sub_bytes) {
                        if !entry.uid.is_empty() && !entry.install_path.is_empty() {
                            // Filter internal Agent, BDK, and Battle.net client components
                            let low = entry.uid.to_lowercase();
                            const IGNORED_INTERNAL_UIDS: &[&str] = &[
                                "agent", "bna", "battle.net", "battlenet", "bdk", "bdko", "client"
                            ];
                            if !IGNORED_INTERNAL_UIDS.contains(&low.as_str()) {
                                entries.push(entry);
                            }
                        }
                    }
                }
            } else {
                if !reader.skip_wire(wire_type) {
                    break;
                }
            }
        } else {
            break;
        }
    }

    // 2. Fallback heuristic string scanner if structured proto yielded nothing
    if entries.is_empty() {
        let content_lossy = String::from_utf8_lossy(&bytes);
        let known_uids = [
            "wow", "wow_classic", "fenris", "pro", "w3", "odin", "diablo3", "d3",
            "osi", "hs", "hsb", "sc2", "s2", "s1", "hero", "rtro"
        ];

        for &uid in &known_uids {
            if let Some(idx) = content_lossy.find(uid) {
                // Look for path indicators near this token
                let slice = &content_lossy[idx..std::cmp::min(idx + 500, content_lossy.len())];
                for drive in ["C:\\", "c:\\", "C:/", "c:/", "D:\\", "d:\\"] {
                    if let Some(d_idx) = slice.find(drive) {
                        let path_sub = &slice[d_idx..];
                        let end = path_sub
                            .find(|c: char| c == '\0' || c == '\n' || c == '\r' || (c as u32) < 32)
                            .unwrap_or(path_sub.len());
                        let candidate_path = path_sub[..end].trim();
                        if candidate_path.len() > 3 {
                            entries.push(ProductDbEntry {
                                uid: uid.to_string(),
                                product_code: Some(uid.to_string()),
                                install_path: candidate_path.to_string(),
                            });
                            break;
                        }
                    }
                }
            }
        }
    }

    Ok(entries)
}

fn parse_product_install_message(bytes: &[u8]) -> Option<ProductDbEntry> {
    let mut reader = ProtoReader::new(bytes);
    let mut uid = String::new();
    let mut product_code = None;
    let mut install_path = String::new();

    while reader.has_more() {
        if let Some((tag, wire_type)) = reader.read_tag() {
            match tag {
                1 if wire_type == 2 => {
                    if let Some(s) = reader.read_string() {
                        uid = s;
                    }
                }
                2 if wire_type == 2 => {
                    if let Some(s) = reader.read_string() {
                        product_code = Some(s);
                    }
                }
                3 if wire_type == 2 => {
                    // Settings message
                    if let Some(settings_bytes) = reader.read_bytes() {
                        let mut s_reader = ProtoReader::new(settings_bytes);
                        while s_reader.has_more() {
                            if let Some((s_tag, s_wire)) = s_reader.read_tag() {
                                if s_tag == 1 && s_wire == 2 {
                                    if let Some(p) = s_reader.read_string() {
                                        install_path = p;
                                    }
                                } else {
                                    if !s_reader.skip_wire(s_wire) {
                                        break;
                                    }
                                }
                            } else {
                                break;
                            }
                        }
                    }
                }
                _ => {
                    if !reader.skip_wire(wire_type) {
                        break;
                    }
                }
            }
        } else {
            break;
        }
    }

    if !uid.is_empty() {
        Some(ProductDbEntry {
            uid,
            product_code,
            install_path,
        })
    } else {
        None
    }
}

// =========================================================================
// .build.info PARSER
// =========================================================================

/// Parses Blizzard's pipe-delimited `.build.info` local manifest file.
pub fn parse_build_info(path: &Path) -> Result<Vec<BuildInfoRow>, String> {
    if !path.exists() {
        return Err(format!("Manifeste .build.info introuvable à l'emplacement: {:?}", path));
    }

    let content = fs::read_to_string(path)
        .map_err(|e| format!("Impossible de lire le manifeste .build.info: {}", e))?;

    let lines: Vec<&str> = content.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
    if lines.is_empty() {
        return Err("Fichier .build.info vide".to_string());
    }

    // First line: Header columns with types: e.g. Branch!STRING:0|Active!DEC:1|...
    let raw_headers: Vec<&str> = lines[0].split('|').collect();
    let headers: Vec<&str> = raw_headers
        .iter()
        .map(|h| h.split('!').next().unwrap_or(h).trim())
        .collect();

    let mut rows = Vec::new();

    for line in &lines[1..] {
        let cols: Vec<&str> = line.split('|').map(|c| c.trim()).collect();
        if cols.len() < headers.len() {
            continue;
        }

        let mut row_map: HashMap<&str, &str> = HashMap::new();
        for (idx, &h) in headers.iter().enumerate() {
            if idx < cols.len() {
                row_map.insert(h, cols[idx]);
            }
        }

        let product = row_map.get("Product").unwrap_or(&"").to_string();
        let version = row_map.get("Version").unwrap_or(&"").to_string();
        let build_key = row_map.get("Build Key").map(|s| s.to_string());
        let branch = row_map.get("Branch").map(|s| s.to_string());
        let active_str = row_map.get("Active").unwrap_or(&"0");
        let active = *active_str == "1";

        rows.push(BuildInfoRow {
            product,
            version,
            build_key,
            branch,
            active,
        });
    }

    Ok(rows)
}

/// Finds the `.build.info` file located directly or in subdirectories (_retail_, etc.)
pub fn locate_build_info(base_dir: &Path) -> Option<PathBuf> {
    // 1. Root
    let root = base_dir.join(".build.info");
    if root.exists() {
        return Some(root);
    }

    // 2. Common subdirectories
    for sub in &["_retail_", "_forever_", "_classic_", "_classic_era_", "_ptr_"] {
        let sub_path = base_dir.join(sub).join(".build.info");
        if sub_path.exists() {
            return Some(sub_path);
        }
    }

    // 3. One level search
    if let Ok(entries) = fs::read_dir(base_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                let candidate = p.join(".build.info");
                if candidate.exists() {
                    return Some(candidate);
                }
            }
        }
    }

    None
}

/// Locates the game executable relative to the game's root directory (ignoring launcher/helper binaries)
pub fn find_game_executable(game_dir: &Path, product_uid: &str) -> Option<String> {
    // 1. Check known relative paths for this product UID
    for rel in get_known_executable_relative_paths(product_uid) {
        if game_dir.join(rel).exists() {
            return Some(rel.to_string());
        }
    }

    // 2. Scan root directory for any .exe, strictly ignoring launcher stubs
    if let Ok(entries) = fs::read_dir(game_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() {
                if let Some(ext) = p.extension() {
                    if ext.to_string_lossy().eq_ignore_ascii_case("exe") {
                        let filename = p.file_name().unwrap().to_string_lossy().to_string();
                        let low = filename.to_lowercase();
                        if !low.contains("crash")
                            && !low.contains("error")
                            && !low.contains("uninstall")
                            && !low.contains("launcher")
                            && !low.contains("helper")
                        {
                            return Some(filename);
                        }
                    }
                }
            }
        }
    }

    // 3. Scan subdirectories (_retail_, etc.)
    for sub in &["_retail_", "_forever_", "_classic_", "_classic_era_", "x86_64", "Support64", "Support"] {
        let sub_dir = game_dir.join(sub);
        if sub_dir.is_dir() {
            if let Ok(entries) = fs::read_dir(&sub_dir) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.is_file() {
                        if let Some(ext) = p.extension() {
                            if ext.to_string_lossy().eq_ignore_ascii_case("exe") {
                                let filename = p.file_name().unwrap().to_string_lossy().to_string();
                                let low = filename.to_lowercase();
                                if !low.contains("crash")
                                    && !low.contains("error")
                                    && !low.contains("uninstall")
                                    && !low.contains("launcher")
                                    && !low.contains("helper")
                                {
                                    return Some(format!("{}/{}", sub, filename));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

/// Inspects an arbitrary game directory (external or inside prefix)
/// Validates the presence of `.build.info` or standard Blizzard binaries.
pub fn inspect_game_directory(path: &Path) -> Result<BattleNetGameMetadata, String> {
    if !path.exists() {
        return Err(format!("Le répertoire spécifié n'existe pas: {:?}", path));
    }
    if !path.is_dir() {
        return Err(format!("Le chemin spécifié n'est pas un dossier: {:?}", path));
    }

    // Attempt to locate and parse .build.info
    let mut detected_uid = String::new();
    let mut version_build = None;
    let mut build_key = None;
    let mut branch = None;

    if let Some(build_info_path) = locate_build_info(path) {
        if let Ok(rows) = parse_build_info(&build_info_path) {
            // Find active row, or fallback to first row
            let active_row = rows.iter().find(|r| r.active).or_else(|| rows.first());
            if let Some(row) = active_row {
                detected_uid = row.product.clone();
                // Check if specific WoW sub-version is present
                if detected_uid == "wow" {
                    if path.join("_forever_/Wow.exe").exists() && !path.join("_retail_/Wow.exe").exists() {
                        detected_uid = "wowf".to_string();
                    } else if path.join("_classic_era_/WowClassic.exe").exists() && !path.join("_retail_/Wow.exe").exists() {
                        detected_uid = "wow_classic_era".to_string();
                    } else if path.join("_classic_/WowClassic.exe").exists() && !path.join("_retail_/Wow.exe").exists() {
                        detected_uid = "wow_classic".to_string();
                    }
                }
                version_build = Some(row.version.clone());
                build_key = row.build_key.clone();
                branch = row.branch.clone();
            }
        }
    }

    // If .build.info was missing, attempt heuristic detection from directory name or executables
    if detected_uid.is_empty() {
        let dir_name = path
            .file_name()
            .map(|s| s.to_string_lossy().to_lowercase())
            .unwrap_or_default();

        if path.join("_forever_/Wow.exe").exists() {
            detected_uid = "wowf".to_string();
        } else if path.join("_classic_era_/WowClassic.exe").exists() {
            detected_uid = "wow_classic_era".to_string();
        } else if path.join("_classic_/WowClassic.exe").exists() {
            detected_uid = "wow_classic".to_string();
        } else if dir_name.contains("warcraft") || path.join("Wow.exe").exists() || path.join("_retail_/Wow.exe").exists() {
            detected_uid = "wow".to_string();
        } else if dir_name.contains("diablo iv") || path.join("Diablo IV.exe").exists() {
            detected_uid = "fenris".to_string();
        } else if dir_name.contains("overwatch") || path.join("_retail_/Overwatch.exe").exists() {
            detected_uid = "pro".to_string();
        } else if dir_name.contains("warcraft iii") || path.join("Warcraft III.exe").exists() {
            detected_uid = "w3".to_string();
        } else if dir_name.contains("hearthstone") || path.join("Hearthstone.exe").exists() {
            detected_uid = "hs".to_string();
        } else if dir_name.contains("starcraft ii") || path.join("StarCraft II.exe").exists() {
            detected_uid = "sc2".to_string();
        } else if dir_name.contains("diablo ii") || path.join("D2R.exe").exists() {
            detected_uid = "osi".to_string();
        } else if dir_name.contains("diablo iii") || path.join("Diablo III.exe").exists() {
            detected_uid = "diablo3".to_string();
        } else {
            return Err(format!(
                "Répertoire invalide: aucun manifest .build.info ou exécutable Blizzard reconnu dans {:?}",
                path
            ));
        }
    }

    let executable_rel = find_game_executable(path, &detected_uid).ok_or_else(|| {
        format!(
            "Aucun binaire exécutable (.exe) valide trouvé pour '{}' dans {:?}",
            detected_uid, path
        )
    })?;

    let name = product_uid_to_name(&detected_uid).to_string();
    let install_path_str = path.to_string_lossy().to_string();

    Ok(BattleNetGameMetadata {
        product_uid: detected_uid,
        name,
        install_path: install_path_str,
        executable_relative_path: executable_rel,
        version_build,
        build_key,
        branch,
    })
}

/// Detects the most recent modification time among config/logs/saves of a game instance
pub fn detect_directory_last_modified(dir: &Path) -> Option<i64> {
    if !dir.exists() {
        return None;
    }

    let mut newest: i64 = 0;
    let candidates = [
        dir.join("WTF").join("Config.wtf"),
        dir.join("Logs"),
        dir.join("Errors"),
        dir.join("Screenshots"),
        dir.join("Settings.txt"),
        dir.join("Variables.txt"),
        dir.join("FenrisDebug.txt"),
    ];

    for c in &candidates {
        if c.exists() {
            if let Ok(meta) = fs::metadata(c) {
                if let Ok(mtime) = meta.modified() {
                    if let Ok(dur) = mtime.duration_since(std::time::UNIX_EPOCH) {
                        let sec = dur.as_secs() as i64;
                        if sec > newest {
                            newest = sec;
                        }
                    }
                }
            }
        }
    }

    // Also inspect WTF/Account character folders for WoW instances
    let wtf_acc = dir.join("WTF").join("Account");
    if wtf_acc.is_dir() {
        if let Ok(accs) = fs::read_dir(&wtf_acc) {
            for acc in accs.flatten() {
                if acc.path().is_dir() {
                    if let Ok(realms) = fs::read_dir(acc.path()) {
                        for realm in realms.flatten() {
                            if realm.path().is_dir() {
                                if let Ok(chars) = fs::read_dir(realm.path()) {
                                    for ch in chars.flatten() {
                                        if let Ok(files) = fs::read_dir(ch.path()) {
                                            for f in files.flatten() {
                                                if let Ok(m) = f.metadata() {
                                                    if let Ok(mt) = m.modified() {
                                                        if let Ok(dur) = mt.duration_since(std::time::UNIX_EPOCH) {
                                                            let sec = dur.as_secs() as i64;
                                                            if sec > newest {
                                                                newest = sec;
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if newest > 0 {
        Some(newest)
    } else {
        None
    }
}

/// Detects all installed editions/instances of World of Warcraft inside a root WoW directory
/// Note: Editions are ONLY marked as installed if their real game client binary (Wow.exe / WowClassic.exe) exists.
pub fn detect_wow_editions(base_wow_path: &Path) -> Vec<BattleNetGameRecord> {
    let mut editions = Vec::new();
    let name_str = base_wow_path.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
    let is_wow_dir = name_str.contains("world of warcraft")
        || base_wow_path.join("_retail_").exists()
        || base_wow_path.join("_forever_").exists()
        || base_wow_path.join("_classic_").exists()
        || base_wow_path.join("_classic_era_").exists();

    if !is_wow_dir {
        return editions;
    }

    let p_str = base_wow_path.to_string_lossy().to_string();

    // Map versions from .build.info if available
    let build_info_map: HashMap<String, String> = locate_build_info(base_wow_path)
        .and_then(|p| parse_build_info(&p).ok())
        .map(|rows| {
            let mut map = HashMap::new();
            for r in rows {
                map.insert(r.product.to_lowercase(), r.version);
            }
            map
        })
        .unwrap_or_default();

    // 1. Forever Edition
    let forever_dir = base_wow_path.join("_forever_");
    if forever_dir.is_dir() {
        let exe = if forever_dir.join("Wow.exe").exists() {
            Some("_forever_/Wow.exe")
        } else if forever_dir.join("WowClassic.exe").exists() {
            Some("_forever_/WowClassic.exe")
        } else {
            None
        };

        if let Some(exe_rel) = exe {
            let last_played = detect_directory_last_modified(&forever_dir);
            let version = build_info_map
                .get("wowf")
                .cloned()
                .or_else(|| build_info_map.get("wow").cloned());
            editions.push(BattleNetGameRecord {
                product_uid: "wowf".to_string(),
                name: "World of Warcraft: Forever".to_string(),
                install_path: p_str.clone(),
                executable_relative_path: exe_rel.to_string(),
                version_build: version,
                is_external: false,
                is_installed: true,
                last_played,
                playtime_seconds: 0,
            });
        }
    }

    // 2. Retail
    let retail_dir = base_wow_path.join("_retail_");
    if retail_dir.is_dir() && retail_dir.join("Wow.exe").exists() {
        let last_played = detect_directory_last_modified(&retail_dir);
        let version = build_info_map.get("wow").cloned();
        editions.push(BattleNetGameRecord {
            product_uid: "wow".to_string(),
            name: "World of Warcraft".to_string(),
            install_path: p_str.clone(),
            executable_relative_path: "_retail_/Wow.exe".to_string(),
            version_build: version,
            is_external: false,
            is_installed: true,
            last_played,
            playtime_seconds: 0,
        });
    }

    // 3. Classic
    let classic_dir = base_wow_path.join("_classic_");
    if classic_dir.is_dir() && classic_dir.join("WowClassic.exe").exists() {
        let last_played = detect_directory_last_modified(&classic_dir);
        let version = build_info_map
            .get("wow_classic")
            .cloned()
            .or_else(|| build_info_map.get("wow").cloned());
        editions.push(BattleNetGameRecord {
            product_uid: "wow_classic".to_string(),
            name: "World of Warcraft Classic".to_string(),
            install_path: p_str.clone(),
            executable_relative_path: "_classic_/WowClassic.exe".to_string(),
            version_build: version,
            is_external: false,
            is_installed: true,
            last_played,
            playtime_seconds: 0,
        });
    }

    // 4. Classic Era
    let classic_era_dir = base_wow_path.join("_classic_era_");
    if classic_era_dir.is_dir() && classic_era_dir.join("WowClassic.exe").exists() {
        let last_played = detect_directory_last_modified(&classic_era_dir);
        let version = build_info_map
            .get("wow_classic_era")
            .cloned()
            .or_else(|| build_info_map.get("wow_classic").cloned());
        editions.push(BattleNetGameRecord {
            product_uid: "wow_classic_era".to_string(),
            name: "World of Warcraft Classic: Season of Discovery".to_string(),
            install_path: p_str.clone(),
            executable_relative_path: "_classic_era_/WowClassic.exe".to_string(),
            version_build: version,
            is_external: false,
            is_installed: true,
            last_played,
            playtime_seconds: 0,
        });
    }

    // 5. Fallback if Wow.exe is at root directly
    if base_wow_path.join("Wow.exe").exists() && editions.is_empty() {
        let last_played = detect_directory_last_modified(base_wow_path);
        let version = build_info_map.get("wow").cloned();
        editions.push(BattleNetGameRecord {
            product_uid: "wow".to_string(),
            name: "World of Warcraft".to_string(),
            install_path: p_str.clone(),
            executable_relative_path: "Wow.exe".to_string(),
            version_build: version,
            is_external: false,
            is_installed: true,
            last_played,
            playtime_seconds: 0,
        });
    }

    editions
}

/// Scans games in the Battle.net Wine prefix and combines them with any declared external games.
pub fn scan_prefix_and_external_games(
    prefix_path: &Path,
    conn: &rusqlite::Connection,
) -> Result<Vec<BattleNetGameRecord>, String> {
    // 0. Clean up internal/launcher components from SQLite if any previously slipped in
    let _ = conn.execute(
        "DELETE FROM battlenet_games WHERE product_uid IN ('battle.net', 'battlenet', 'agent', 'bna', 'bdk', 'bdko', 'client')",
        [],
    );

    // 1. Scan product.db if prefix exists
    let product_db_path = prefix_path
        .join("drive_c")
        .join("ProgramData")
        .join("Battle.net")
        .join("Agent")
        .join("product.db");

    let mut detected_internal = Vec::new();

    if product_db_path.exists() {
        if let Ok(entries) = parse_product_db(&product_db_path) {
            for entry in entries {
                let resolved_dir = resolve_wine_path(&entry.install_path, prefix_path);
                if resolved_dir.exists() {
                    let wow_editions = detect_wow_editions(&resolved_dir);
                    if !wow_editions.is_empty() {
                        for ed in wow_editions {
                            if !detected_internal.iter().any(|g: &BattleNetGameRecord| g.product_uid == ed.product_uid) {
                                detected_internal.push(ed);
                            }
                        }
                    } else if let Ok(meta) = inspect_game_directory(&resolved_dir) {
                        let last_played = detect_directory_last_modified(&resolved_dir);
                        let record = BattleNetGameRecord {
                            product_uid: meta.product_uid,
                            name: meta.name,
                            install_path: meta.install_path,
                            executable_relative_path: meta.executable_relative_path,
                            version_build: meta.version_build,
                            is_external: false,
                            is_installed: true,
                            last_played,
                            playtime_seconds: 0,
                        };
                        detected_internal.push(record);
                    }
                }
            }
        }
    }

    // 2. Also inspect standard prefix Program Files directories in case product.db is empty
    let program_files_paths = [
        prefix_path.join("drive_c").join("Program Files (x86)"),
        prefix_path.join("drive_c").join("Program Files"),
        prefix_path.join("drive_c").join("Games"),
    ];

    for pf in &program_files_paths {
        if pf.exists() {
            if let Ok(entries) = fs::read_dir(pf) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let wow_editions = detect_wow_editions(&path);
                        if !wow_editions.is_empty() {
                            for ed in wow_editions {
                                if !detected_internal.iter().any(|g| g.product_uid == ed.product_uid) {
                                    detected_internal.push(ed);
                                }
                            }
                        } else if locate_build_info(&path).is_some() {
                            if let Ok(meta) = inspect_game_directory(&path) {
                                if !detected_internal.iter().any(|g| g.product_uid == meta.product_uid) {
                                    let last_played = detect_directory_last_modified(&path);
                                    detected_internal.push(BattleNetGameRecord {
                                        product_uid: meta.product_uid,
                                        name: meta.name,
                                        install_path: meta.install_path,
                                        executable_relative_path: meta.executable_relative_path,
                                        version_build: meta.version_build,
                                        is_external: false,
                                        is_installed: true,
                                        last_played,
                                        playtime_seconds: 0,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 3. Synchronize with database: persist newly detected internal games
    for g in &detected_internal {
        let existing = db::get_battlenet_game(conn, &g.product_uid).unwrap_or(None);
        let mut to_save = g.clone();
        if let Some(e) = existing {
            to_save.last_played = match (e.last_played, g.last_played) {
                (Some(lp1), Some(lp2)) => Some(std::cmp::max(lp1, lp2)),
                (Some(lp1), None) => Some(lp1),
                (None, Some(lp2)) => Some(lp2),
                (None, None) => None,
            };
            to_save.playtime_seconds = e.playtime_seconds;
        }
        let _ = db::insert_or_replace_battlenet_game(conn, &to_save);
    }

    // 4. Verify all existing games in DB: reconcile is_installed flag and remove invalid launcher entries
    let stored_games = db::get_battlenet_games(conn).map_err(|e| e.to_string())?;
    for mut stored in stored_games {
        let exe_lower = stored.executable_relative_path.to_lowercase();
        let is_launcher_stub = exe_lower.contains("launcher") || exe_lower.contains("agent.exe");

        if stored.is_external {
            let p = PathBuf::from(&stored.install_path);
            let exe_exists = p.join(&stored.executable_relative_path).exists();
            let still_installed = p.exists() && exe_exists && !is_launcher_stub;
            if stored.is_installed != still_installed {
                stored.is_installed = still_installed;
                let _ = db::insert_or_replace_battlenet_game(conn, &stored);
            }
        } else {
            // Internal game: verify if actively detected and real executable exists
            let re_detected = detected_internal.iter().any(|g| g.product_uid == stored.product_uid);
            let full_exe_path = PathBuf::from(&stored.install_path).join(&stored.executable_relative_path);
            let exe_valid = full_exe_path.exists() && !is_launcher_stub;
            let should_be_installed = re_detected && exe_valid;

            if stored.is_installed != should_be_installed {
                stored.is_installed = should_be_installed;
                let _ = db::insert_or_replace_battlenet_game(conn, &stored);
            }
        }
    }

    // Return all games currently in DB
    db::get_battlenet_games(conn).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_product_uid_mapping() {
        assert_eq!(product_uid_to_name("wow"), "World of Warcraft");
        assert_eq!(product_uid_to_name("fenris"), "Diablo IV");
        assert_eq!(product_uid_to_name("pro"), "Overwatch 2");
        assert_eq!(product_uid_to_name("w3"), "Warcraft III: Reforged");
        assert_eq!(product_uid_to_name("odin"), "Call of Duty: Modern Warfare");
        assert_eq!(product_uid_to_name("sc2"), "StarCraft II");
    }

    #[test]
    fn test_resolve_wine_path() {
        let prefix = PathBuf::from("/home/user/.local/share/loom/prefixes/battlenet");
        let win_path = r"C:\Program Files (x86)\World of Warcraft";
        let resolved = resolve_wine_path(win_path, &prefix);
        assert_eq!(
            resolved,
            prefix.join("drive_c").join("Program Files (x86)/World of Warcraft")
        );
    }

    #[test]
    fn test_parse_build_info() {
        let tmp_dir = std::env::temp_dir().join("loom_test_build_info");
        let _ = fs::create_dir_all(&tmp_dir);
        let file_path = tmp_dir.join(".build.info");

        let content = "Branch!STRING:0|Active!DEC:1|Build Key!HEX:16|CDN Key!HEX:16|Install Key!HEX:16|IM Size!DEC:4|CDN Path!STRING:0|CDN Hosts!STRING:0|CDN Servers!STRING:0|Tags!STRING:0|Armadillo!STRING:0|Last Activated!STRING:0|Version!STRING:0|Keyring!HEX:16|Product!STRING:0\n\
us|1|5b404d0f6ad32ef3c8dbe4889c372551|d5ba948ba4ff558e82ef3bc104c86ec6|c6046e9bbdfae7f5ef8996b797177579|4582928|tpr/wow|level3.blizzard.com|http://level3.blizzard.com/|Windows?x86_64?EU?frFR?text?speech|||11.0.2.56453||wow\n";

        let mut f = fs::File::create(&file_path).unwrap();
        f.write_all(content.as_bytes()).unwrap();

        let parsed = parse_build_info(&file_path).expect("Failed to parse build info");
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].product, "wow");
        assert_eq!(parsed[0].version, "11.0.2.56453");
        assert_eq!(parsed[0].build_key.as_deref(), Some("5b404d0f6ad32ef3c8dbe4889c372551"));
        assert_eq!(parsed[0].active, true);

        let _ = fs::remove_dir_all(&tmp_dir);
    }

    #[test]
    fn test_inspect_game_directory_mock() {
        let tmp_dir = std::env::temp_dir().join("loom_test_game_diablo");
        let _ = fs::create_dir_all(&tmp_dir);
        let build_info_file = tmp_dir.join(".build.info");

        let content = "Branch!STRING:0|Active!DEC:1|Build Key!HEX:16|Version!STRING:0|Product!STRING:0\n\
us|1|deadbeef12345678|2.0.4.59888|fenris\n";
        fs::write(&build_info_file, content).unwrap();

        // Create mock executable
        let exe_file = tmp_dir.join("Diablo IV.exe");
        fs::write(&exe_file, b"MZ").unwrap();

        let meta = inspect_game_directory(&tmp_dir).expect("Failed to inspect mock directory");
        assert_eq!(meta.product_uid, "fenris");
        assert_eq!(meta.name, "Diablo IV");
        assert_eq!(meta.executable_relative_path, "Diablo IV.exe");
        assert_eq!(meta.version_build.as_deref(), Some("2.0.4.59888"));

        let _ = fs::remove_dir_all(&tmp_dir);
    }

    #[test]
    fn test_inspect_game_directory_missing_fails_cleanly() {
        let fake = PathBuf::from("/non/existent/path/blizzard/game");
        let result = inspect_game_directory(&fake);
        assert!(result.is_err());
        let err_msg = result.unwrap_err();
        assert!(err_msg.contains("n'existe pas"));
    }

    #[test]
    fn test_detect_wow_editions_ignores_launcher_stubs() {
        let tmp_dir = std::env::temp_dir().join("loom_test_wow_launcher_stub");
        let _ = fs::create_dir_all(&tmp_dir);

        // Create _retail_ with only a launcher stub
        let retail_dir = tmp_dir.join("_retail_");
        let _ = fs::create_dir_all(&retail_dir);
        let launcher_stub = tmp_dir.join("World of Warcraft Launcher.exe");
        fs::write(&launcher_stub, b"MZ").unwrap();

        // Should NOT detect retail as installed since Wow.exe is missing
        let detected = detect_wow_editions(&tmp_dir);
        assert!(detected.is_empty(), "Launcher stub without Wow.exe should not detect an installed edition");

        // Now add real Wow.exe inside _retail_
        let real_wow_exe = retail_dir.join("Wow.exe");
        fs::write(&real_wow_exe, b"MZ").unwrap();

        let detected_with_real = detect_wow_editions(&tmp_dir);
        assert_eq!(detected_with_real.len(), 1);
        assert_eq!(detected_with_real[0].product_uid, "wow");
        assert_eq!(detected_with_real[0].executable_relative_path, "_retail_/Wow.exe");

        let _ = fs::remove_dir_all(&tmp_dir);
    }

    #[test]
    fn test_find_game_executable_strictly_ignores_launcher() {
        let tmp_dir = std::env::temp_dir().join("loom_test_find_exe_no_launcher");
        let _ = fs::create_dir_all(&tmp_dir);

        // Put launcher stub
        let stub = tmp_dir.join("Diablo IV Launcher.exe");
        fs::write(&stub, b"MZ").unwrap();

        let found = find_game_executable(&tmp_dir, "fenris");
        assert_eq!(found, None, "Should not return Launcher stub as game executable");

        // Put real executable
        let real = tmp_dir.join("Diablo IV.exe");
        fs::write(&real, b"MZ").unwrap();

        let found_real = find_game_executable(&tmp_dir, "fenris");
        assert_eq!(found_real, Some("Diablo IV.exe".to_string()));

        let _ = fs::remove_dir_all(&tmp_dir);
    }

    #[test]
    fn test_scan_cleans_up_launcher_and_ghost_entries() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::init_schema(&conn).unwrap();

        // Insert a ghost game pointing to a launcher stub
        let fake_record = BattleNetGameRecord {
            product_uid: "wow".to_string(),
            name: "World of Warcraft".to_string(),
            install_path: "/tmp/fake_wow".to_string(),
            executable_relative_path: "World of Warcraft Launcher.exe".to_string(),
            version_build: Some("11.0.2".to_string()),
            is_external: false,
            is_installed: true,
            last_played: None,
            playtime_seconds: 0,
        };
        db::insert_or_replace_battlenet_game(&conn, &fake_record).unwrap();

        // Also insert an internal tool UID that should be removed
        let internal_tool = BattleNetGameRecord {
            product_uid: "battle.net".to_string(),
            name: "Battle.net Client".to_string(),
            install_path: "/tmp/fake_bnet".to_string(),
            executable_relative_path: "Battle.net.exe".to_string(),
            version_build: None,
            is_external: false,
            is_installed: true,
            last_played: None,
            playtime_seconds: 0,
        };
        db::insert_or_replace_battlenet_game(&conn, &internal_tool).unwrap();

        // Run scan with non-existent prefix
        let prefix = PathBuf::from("/tmp/non_existent_prefix_for_test");
        let results = scan_prefix_and_external_games(&prefix, &conn).unwrap();

        // 'battle.net' UID must be deleted
        assert!(!results.iter().any(|g| g.product_uid == "battle.net"));

        // 'wow' pointing to launcher must have is_installed = false
        if let Some(wow_rec) = results.iter().find(|g| g.product_uid == "wow") {
            assert!(!wow_rec.is_installed, "Ghost entry pointing to launcher must not be marked installed");
        }
    }
}
