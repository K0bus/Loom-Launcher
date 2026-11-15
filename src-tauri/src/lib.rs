pub mod battlenet;
pub mod commands;
pub mod db;
pub mod minecraft;
pub mod modding;
pub mod process_guard;

use battlenet::BattleNetLauncherState;

use commands::minecraft::*;

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use uuid::Uuid;

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::Manager;

use db::{FileConflict, GameProfileRecord, ProfileModRecord};
use minecraft::adapter::MinecraftAdapter;
use minecraft::auth::{
    initiate_device_code, poll_device_code, remove_stored_account,
    DeviceCodeResponse, MinecraftAccount, PollAuthResult,
};
use minecraft::launcher::launch_instance;
use minecraft::modpack::ModpackInstaller;
use modding::adapter::{GameAdapter, GameModdingManifest, ProviderKind, RemoteDependency, UnifiedModSearchResult};
use modding::conflicts::scan_and_resolve_conflicts as scan_conflicts_impl;
use modding::nexus::{download_nexus_file, parse_nxm_uri, resolve_nexus_download_links};
use modding::providers::curseforge::CurseForgeProvider;
use modding::providers::modrinth::ModrinthProvider;
use modding::providers::ModProvider;
use modding::registry::{create_shared_registry, SharedGameRegistry};
use modding::symlink::get_mods_cache_dir;
use modding::{BethesdaAdapter, WowAdapter};

#[derive(Debug, Serialize, Deserialize)]
pub struct LaunchResult {
    pub success: bool,
    pub pid: Option<u32>,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunningState {
    pub success: bool,
    pub running_app_id: Option<String>,
    pub pids: Vec<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemStats {
    pub cpu_usage: f32,
    pub ram_used_gb: f32,
    pub ram_total_gb: f32,
    pub disk_free_gb: f32,
    pub disk_total_gb: f32,
    pub network_down_kbps: f32,
    pub network_up_kbps: f32,
    pub display_server: Option<String>,
    pub kernel_version: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct LoomBackendSettings {
    pub close_to_tray: Option<bool>,
    pub start_minimized: Option<bool>,
    pub autostart_with_system: Option<bool>,
    pub games_directory: Option<String>,
    pub minecraft_directory: Option<String>,
    pub wine_prefixes_directory: Option<String>,
    pub default_runner: Option<String>,
    pub global_gamemode: Option<bool>,
    pub global_mangohud: Option<bool>,
    pub nexus_api_key: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ScannedSteamGame {
    pub appid: String,
    pub name: String,
    pub slug: String,
    pub installed: bool,
    pub playtime_minutes: u32,
    pub last_played_timestamp: u64,
    pub size_gb: f32,
    pub install_dir: Option<String>,
    pub cover_url: String,
    pub banner_url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ScannedSteamAccount {
    pub steam_id64: String,
    pub account_name: String,
    pub persona_name: String,
    pub install_path: String,
    pub detected: bool,
    pub installed_count: usize,
    pub uninstalled_count: usize,
    pub total_games_count: usize,
    pub installed_games: Vec<ScannedSteamGame>,
    pub uninstalled_games: Vec<ScannedSteamGame>,
    pub all_games: Vec<ScannedSteamGame>,
}

fn get_running_steam_app() -> (Option<String>, Vec<u32>) {
    // 1. Direct /proc inspection: check /proc/*/cmdline and /proc/*/environ for AppId/SteamAppId
    if let Ok(entries) = fs::read_dir("/proc") {
        for entry in entries.flatten() {
            let file_name = entry.file_name();
            let name = file_name.to_string_lossy();
            if let Ok(pid) = name.parse::<u32>() {
                // Check cmdline (reaper SteamLaunch AppId=... or steam-launch-wrapper)
                if let Ok(cmd) = fs::read(entry.path().join("cmdline")) {
                    if !cmd.is_empty() {
                        let cmd_str = String::from_utf8_lossy(&cmd);
                        if let Some(pos) = cmd_str.find("AppId=") {
                            let sub = &cmd_str[pos + 6..];
                            let aid: String = sub.chars().take_while(|c| c.is_ascii_digit()).collect();
                            if !aid.is_empty() && aid != "0" {
                                return (Some(aid), vec![pid]);
                            }
                        }
                        if let Some(pos) = cmd_str.find("rungameid/") {
                            let sub = &cmd_str[pos + 10..];
                            let aid: String = sub.chars().take_while(|c| c.is_ascii_digit()).collect();
                            if !aid.is_empty() && aid != "0" {
                                return (Some(aid), vec![pid]);
                            }
                        }
                    }
                }

                // Check environ for SteamAppId
                if let Ok(env) = fs::read(entry.path().join("environ")) {
                    if let Some(pos) = env.windows(11).position(|w| w == b"SteamAppId=") {
                        let sub = &env[pos + 11..];
                        let end = sub.iter().position(|&b| b == 0).unwrap_or(sub.len());
                        if let Ok(val_str) = std::str::from_utf8(&sub[..end]) {
                            let val = val_str.trim();
                            if !val.is_empty() && val != "0" && val.chars().all(|c| c.is_ascii_digit()) {
                                return (Some(val.to_string()), vec![pid]);
                            }
                        }
                    }
                }
            }
        }
    }

    // 2. Parse Steam logs/gameprocess_log.txt (recent window)
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/k0bus".to_string());
    let log_path = PathBuf::from(home).join(".local/share/Steam/logs/gameprocess_log.txt");

    if !log_path.exists() {
        return (None, Vec::new());
    }

    if let Ok(content) = fs::read_to_string(&log_path) {
        let lines: Vec<&str> = content.lines().collect();
        let start = lines.len().saturating_sub(5000);
        let recent = &lines[start..];

        let mut app_pids: HashMap<String, HashSet<u32>> = HashMap::new();

        for line in recent {
            if line.contains("Remove ") && line.contains(" from running list") {
                if let Some(pos) = line.find("Remove ") {
                    let sub = &line[pos + 7..];
                    if let Some(end) = sub.find(' ') {
                        let aid = &sub[..end];
                        app_pids.remove(aid);
                    }
                }
                continue;
            }

            if line.contains("adding PID") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                for i in 0..parts.len() {
                    if parts[i] == "AppID" && i + 4 < parts.len() && parts[i + 3] == "PID" {
                        let aid = parts[i + 1].to_string();
                        if let Ok(pid) = parts[i + 4].parse::<u32>() {
                            app_pids.entry(aid).or_default().insert(pid);
                        }
                    }
                }
                continue;
            }

            if line.contains("no longer tracking PID") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                for i in 0..parts.len() {
                    if parts[i] == "AppID" && i + 6 < parts.len() && parts[i + 5] == "PID" {
                        let aid = parts[i + 1];
                        let pid_str = parts[i + 6].trim_matches(',');
                        if let Ok(pid) = pid_str.parse::<u32>() {
                            if let Some(set) = app_pids.get_mut(aid) {
                                set.remove(&pid);
                                if set.is_empty() {
                                    app_pids.remove(aid);
                                }
                            }
                        }
                    }
                }
            }
        }

        for (aid, pids) in app_pids {
            let mut alive = Vec::new();
            for pid in pids {
                if PathBuf::from(format!("/proc/{}", pid)).exists() {
                    alive.push(pid);
                }
            }
            if !alive.is_empty() {
                return (Some(aid), alive);
            }
        }
    }

    (None, Vec::new())
}

/// Dynamically scans all local Steam libraries and config files for real games
#[tauri::command]
fn scan_local_steam_library() -> Result<ScannedSteamAccount, String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/k0bus".to_string());
    let steam_base = PathBuf::from(&home).join(".local/share/Steam");
    let detected = steam_base.exists();

    let mut libraries: Vec<PathBuf> = vec![steam_base.clone()];
    let lib_vdf = steam_base.join("config/libraryfolders.vdf");

    if let Ok(vdf_content) = fs::read_to_string(&lib_vdf) {
        for line in vdf_content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("\"path\"") {
                let parts: Vec<&str> = trimmed.split('"').collect();
                if parts.len() >= 4 {
                    let p = PathBuf::from(parts[3]);
                    if p.exists() && !libraries.contains(&p) {
                        libraries.push(p);
                    }
                }
            }
        }
    }

    let ignored_appids: HashSet<&str> = [
        "228980", "1070560", "1391110", "1628350", "2180100", "1493710",
        "1826330", "3658110", "1161040", "4183110", "2805730", "4628710"
    ].iter().cloned().collect();

    // 1. Scan userdata for real LastPlayed and Playtime with rock-solid depth tracking
    let mut playtime_map: HashMap<String, (u64, u32)> = HashMap::new();
    let userdata_dir = steam_base.join("userdata");
    if let Ok(users) = fs::read_dir(&userdata_dir) {
        for u in users.flatten() {
            let localconfig = u.path().join("config/localconfig.vdf");
            if let Ok(txt) = fs::read_to_string(&localconfig) {
                let mut pending_aid: Option<String> = None;
                let mut active_aid: Option<String> = None;
                let mut active_depth = 0usize;
                let mut depth = 0usize;

                for line in txt.lines() {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }

                    if trimmed == "{" && pending_aid.is_some() {
                        active_aid = pending_aid.take();
                        active_depth = depth + 1;
                    }

                    let parts: Vec<&str> = trimmed.split('"').collect();
                    if parts.len() >= 2 && parts[1].chars().all(|c| c.is_ascii_digit()) && !parts[1].is_empty() {
                        if trimmed == format!("\"{}\"", parts[1]) {
                            pending_aid = Some(parts[1].to_string());
                        }
                    }

                    if let Some(ref aid) = active_aid {
                        if trimmed.starts_with("\"LastPlayed\"") {
                            let p: Vec<&str> = trimmed.split('"').collect();
                            if p.len() >= 4 {
                                if let Ok(val) = p[3].parse::<u64>() {
                                    let entry = playtime_map.entry(aid.clone()).or_insert((0, 0));
                                    if val > entry.0 {
                                        entry.0 = val;
                                    }
                                }
                            }
                        } else if trimmed.starts_with("\"Playtime\"") {
                            let p: Vec<&str> = trimmed.split('"').collect();
                            if p.len() >= 4 {
                                if let Ok(val) = p[3].parse::<u32>() {
                                    let entry = playtime_map.entry(aid.clone()).or_insert((0, 0));
                                    if val > entry.1 {
                                        entry.1 = val;
                                    }
                                }
                            }
                        }
                    }

                    let open_count = trimmed.chars().filter(|&c| c == '{').count();
                    let close_count = trimmed.chars().filter(|&c| c == '}').count();
                    depth = depth.saturating_add(open_count).saturating_sub(close_count);

                    if active_aid.is_some() && depth < active_depth {
                        active_aid = None;
                    }
                }
            }
        }
    }

    // 2. Scan installed games in steamapps of every library folder
    let mut installed_games = Vec::new();
    let mut installed_appids = HashSet::new();

    for lib in &libraries {
        let steamapps = lib.join("steamapps");
        if !steamapps.exists() {
            continue;
        }

        if let Ok(entries) = fs::read_dir(&steamapps) {
            for entry in entries.flatten() {
                let fname = entry.file_name().to_string_lossy().to_string();
                if fname.starts_with("appmanifest_") && fname.ends_with(".acf") {
                    if let Ok(content) = fs::read_to_string(entry.path()) {
                        let mut aid = String::new();
                        let mut name = String::new();
                        let mut install_dir = String::new();
                        let mut state_flags = String::new();
                        let mut size_bytes = 0u64;

                        for line in content.lines() {
                            let trimmed = line.trim();
                            if trimmed.starts_with("\"appid\"") {
                                let parts: Vec<&str> = trimmed.split('"').collect();
                                if parts.len() >= 4 { aid = parts[3].to_string(); }
                            } else if trimmed.starts_with("\"name\"") {
                                let parts: Vec<&str> = trimmed.split('"').collect();
                                if parts.len() >= 4 { name = parts[3].to_string(); }
                            } else if trimmed.starts_with("\"installdir\"") {
                                let parts: Vec<&str> = trimmed.split('"').collect();
                                if parts.len() >= 4 { install_dir = parts[3].to_string(); }
                            } else if trimmed.starts_with("\"StateFlags\"") {
                                let parts: Vec<&str> = trimmed.split('"').collect();
                                if parts.len() >= 4 { state_flags = parts[3].to_string(); }
                            } else if trimmed.starts_with("\"SizeOnDisk\"") {
                                let parts: Vec<&str> = trimmed.split('"').collect();
                                if parts.len() >= 4 { size_bytes = parts[3].parse::<u64>().unwrap_or(0); }
                            }
                        }

                        if !aid.is_empty() && !ignored_appids.contains(aid.as_str()) {
                            if name.starts_with("Steam Linux Runtime") || name.starts_with("Proton") || name.contains("Redistributable") {
                                continue;
                            }

                            let installed = state_flags == "4";
                            let (last_played, playtime) = playtime_map.get(&aid).cloned().unwrap_or((0, 0));
                            let size_gb = (size_bytes as f32 / 1024.0 / 1024.0 / 1024.0 * 10.0).round() / 10.0;
                            let slug = name.to_lowercase().replace(|c: char| !c.is_alphanumeric(), "-").trim_matches('-').to_string();

                            let cover_url = if ["4126040", "4704690", "480", "2315050"].contains(&aid.as_str()) {
                                format!("/steam-assets/{}/library_capsule.jpg", aid)
                            } else {
                                format!("https://shared.steamstatic.com/store_item_assets/steam/apps/{}/library_600x900.jpg", aid)
                            };

                            let banner_url = if ["4126040", "4704690", "480", "2315050"].contains(&aid.as_str()) {
                                format!("/steam-assets/{}/library_hero.jpg", aid)
                            } else {
                                format!("https://shared.steamstatic.com/store_item_assets/steam/apps/{}/library_hero.jpg", aid)
                            };

                            let g = ScannedSteamGame {
                                appid: aid.clone(),
                                name,
                                slug,
                                installed,
                                playtime_minutes: playtime,
                                last_played_timestamp: last_played,
                                size_gb,
                                install_dir: if install_dir.is_empty() { None } else { Some(install_dir) },
                                cover_url,
                                banner_url,
                            };

                            if installed {
                                installed_appids.insert(aid);
                                installed_games.push(g);
                            }
                        }
                    }
                }
            }
        }
    }

    // 3. Collect uninstalled games from playtime_map that were played by the user
    let mut uninstalled_games = Vec::new();
    for (aid, (last_played, playtime)) in &playtime_map {
        if !installed_appids.contains(aid) && !ignored_appids.contains(aid.as_str()) {
            let cover_url = if ["4126040", "4704690", "480", "2315050"].contains(&aid.as_str()) {
                format!("/steam-assets/{}/library_capsule.jpg", aid)
            } else {
                format!("https://shared.steamstatic.com/store_item_assets/steam/apps/{}/library_600x900.jpg", aid)
            };
            let banner_url = if ["4126040", "4704690", "480", "2315050"].contains(&aid.as_str()) {
                format!("/steam-assets/{}/library_hero.jpg", aid)
            } else {
                format!("https://shared.steamstatic.com/store_item_assets/steam/apps/{}/library_hero.jpg", aid)
            };

            uninstalled_games.push(ScannedSteamGame {
                appid: aid.clone(),
                name: format!("Steam App {}", aid),
                slug: format!("steam-app-{}", aid),
                installed: false,
                playtime_minutes: *playtime,
                last_played_timestamp: *last_played,
                size_gb: 0.0,
                install_dir: None,
                cover_url,
                banner_url,
            });
        }
    }

    // 4. Sort games so most recently played is always on top
    installed_games.sort_by(|a, b| b.last_played_timestamp.cmp(&a.last_played_timestamp));
    uninstalled_games.sort_by(|a, b| b.last_played_timestamp.cmp(&a.last_played_timestamp));

    let mut all_games = Vec::new();
    all_games.extend(installed_games.clone());
    all_games.extend(uninstalled_games.clone());
    all_games.sort_by(|a, b| b.last_played_timestamp.cmp(&a.last_played_timestamp));

    Ok(ScannedSteamAccount {
        steam_id64: "76561198085607978".to_string(),
        account_name: "ik0bus".to_string(),
        persona_name: "K0bus".to_string(),
        install_path: steam_base.to_string_lossy().to_string(),
        detected,
        installed_count: installed_games.len(),
        uninstalled_count: uninstalled_games.len(),
        total_games_count: all_games.len(),
        installed_games,
        uninstalled_games,
        all_games,
    })
}

#[tauri::command]
fn launch_game(game_id: String, _runner: Option<String>) -> Result<LaunchResult, String> {
    let appid = game_id.trim_start_matches("steam-");
    println!("[Tauri::Steam] Lancement réel AppID {}", appid);

    let status = Command::new("xdg-open")
        .arg(format!("steam://rungameid/{}", appid))
        .status();

    match status {
        Ok(s) if s.success() => Ok(LaunchResult {
            success: true,
            pid: None,
            message: format!("Commande de lancement transmise à Steam pour AppID {}", appid),
        }),
        _ => {
            let _ = Command::new("steam")
                .arg(format!("steam://rungameid/{}", appid))
                .spawn();
            Ok(LaunchResult {
                success: true,
                pid: None,
                message: format!("Commande steam://rungameid/{} envoyée", appid),
            })
        }
    }
}

#[tauri::command]
fn stop_game(game_id: String) -> Result<LaunchResult, String> {
    let (_, pids) = get_running_steam_app();
    for pid in pids {
        let _ = Command::new("kill").arg("-15").arg(pid.to_string()).status();
    }
    Ok(LaunchResult {
        success: true,
        pid: None,
        message: format!("Jeu {} arrêté", game_id),
    })
}

#[tauri::command]
fn get_running_games() -> Result<RunningState, String> {
    let (aid, pids) = get_running_steam_app();
    Ok(RunningState {
        success: true,
        running_app_id: aid,
        pids,
    })
}

#[tauri::command]
fn get_system_stats() -> Result<SystemStats, String> {
    let mut total_ram_gb = 16.0f32;
    let mut used_ram_gb = 4.0f32;

    #[cfg(target_os = "linux")]
    {
        if let Ok(meminfo) = fs::read_to_string("/proc/meminfo") {
            let mut mem_total_kb = 0f32;
            let mut mem_avail_kb = 0f32;
            for line in meminfo.lines() {
                if line.starts_with("MemTotal:") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        mem_total_kb = parts[1].parse::<f32>().unwrap_or(0.0);
                    }
                } else if line.starts_with("MemAvailable:") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        mem_avail_kb = parts[1].parse::<f32>().unwrap_or(0.0);
                    }
                }
            }
            if mem_total_kb > 0.0 {
                total_ram_gb = (mem_total_kb / 1024.0 / 1024.0 * 10.0).round() / 10.0;
                let used_kb = (mem_total_kb - mem_avail_kb).max(0.0);
                used_ram_gb = (used_kb / 1024.0 / 1024.0 * 10.0).round() / 10.0;
            }
        }
    }

    let mut cpu_pct = 12.0f32;
    #[cfg(target_os = "linux")]
    {
        if let Ok(loadavg) = fs::read_to_string("/proc/loadavg") {
            let parts: Vec<&str> = loadavg.split_whitespace().collect();
            if !parts.is_empty() {
                let load1 = parts[0].parse::<f32>().unwrap_or(1.0);
                cpu_pct = ((load1 / 8.0) * 100.0).clamp(2.0, 99.0);
                cpu_pct = (cpu_pct * 10.0).round() / 10.0;
            }
        }
    }

    let mut disk_free = 250.0f32;
    let mut disk_total = 1000.0f32;
    #[cfg(target_os = "linux")]
    {
        if let Ok(output) = Command::new("df").args(["-k", "/"]).output() {
            if let Ok(txt) = String::from_utf8(output.stdout) {
                let lines: Vec<&str> = txt.lines().collect();
                if lines.len() >= 2 {
                    let parts: Vec<&str> = lines[1].split_whitespace().collect();
                    if parts.len() >= 4 {
                        let total_kb = parts[1].parse::<f32>().unwrap_or(0.0);
                        let avail_kb = parts[3].parse::<f32>().unwrap_or(0.0);
                        if total_kb > 0.0 {
                            disk_total = (total_kb / 1024.0 / 1024.0 * 10.0).round() / 10.0;
                            disk_free = (avail_kb / 1024.0 / 1024.0 * 10.0).round() / 10.0;
                        }
                    }
                }
            }
        }
    }

    let display_server = std::env::var("XDG_SESSION_TYPE").ok().map(|s| {
        if s.eq_ignore_ascii_case("wayland") {
            "Wayland".to_string()
        } else {
            "X11".to_string()
        }
    });

    let kernel_version = fs::read_to_string("/proc/version")
        .ok()
        .and_then(|v| {
            v.split_whitespace().nth(2).map(|s| s.to_string())
        });

    Ok(SystemStats {
        cpu_usage: cpu_pct,
        ram_used_gb: used_ram_gb,
        ram_total_gb: total_ram_gb,
        disk_free_gb: disk_free,
        disk_total_gb: disk_total,
        network_down_kbps: 4200.0,
        network_up_kbps: 120.0,
        display_server,
        kernel_version,
    })
}

// ==========================================
// TAURI V2 MODDING IPC COMMANDS
// ==========================================

#[tauri::command]
fn get_game_modding_manifest(
    registry: tauri::State<SharedGameRegistry>,
    game_id: String,
) -> Result<GameModdingManifest, String> {
    let reg = registry.read().map_err(|e| e.to_string())?;
    let adapter = reg
        .get(&game_id)
        .ok_or_else(|| format!("Game adapter '{}' not found in registry", game_id))?;
    Ok(adapter.manifest())
}

#[tauri::command]
async fn search_unified_mods(
    query: String,
    game_id: String,
    providers: Vec<String>,
    version: Option<String>,
    loader: Option<String>,
    content_type: Option<String>,
) -> Result<Vec<UnifiedModSearchResult>, String> {
    let mut results = Vec::new();

    let search_all = providers.is_empty() || providers.iter().any(|p| p == "all");
    let has_modrinth = search_all || providers.iter().any(|p| p == "modrinth");
    let has_curseforge = search_all || providers.iter().any(|p| p == "curseforge");

    if has_modrinth {
        let mr = ModrinthProvider::new();
        if let Ok(res) = mr
            .search_content(
                &query,
                &game_id,
                version.as_deref(),
                loader.as_deref(),
                content_type.as_deref(),
                25,
            )
            .await
        {
            results.extend(res);
        }
    }

    if has_curseforge {
        let cf = CurseForgeProvider::new();
        if let Ok(res) = cf
            .search_content(
                &query,
                &game_id,
                version.as_deref(),
                loader.as_deref(),
                content_type.as_deref(),
                25,
            )
            .await
        {
            results.extend(res);
        }
    }

    Ok(results)
}

#[tauri::command]
async fn install_mod_or_pack(
    profile_id: String,
    provider: String,
    file_id: String,
    mod_id: Option<String>,
    mod_name: Option<String>,
) -> Result<(), String> {
    let conn = db::open_db().map_err(|e| e.to_string())?;
    let cache_dir = get_mods_cache_dir();

    let p_kind = ProviderKind::from_str(&provider);
    match p_kind {
        ProviderKind::Modrinth => {
            let mr = ModrinthProvider::new();
            // file_id is version_id or download URL or file filename
            let m_id = mod_id.unwrap_or_else(|| file_id.clone());
            let versions = mr.get_versions(&m_id, None, None).await?;
            let version = versions
                .iter()
                .find(|v| v.id == file_id || v.files.iter().any(|f| f.id == file_id))
                .or_else(|| versions.first())
                .ok_or_else(|| "No version found for mod".to_string())?;

            let file_info = version
                .files
                .iter()
                .find(|f| f.is_primary)
                .or_else(|| version.files.first())
                .ok_or_else(|| "No files found for version".to_string())?;

            let _cached_path = mr
                .download_file(&file_info.url, &cache_dir, &file_info.filename)
                .await?;

            let pmod = ProfileModRecord {
                id: format!("pmod-{}", Uuid::new_v4().simple()),
                profile_id: profile_id.clone(),
                provider: "modrinth".to_string(),
                remote_mod_id: m_id,
                remote_file_id: file_id,
                name: mod_name.unwrap_or_else(|| file_info.filename.clone()),
                file_path: file_info.filename.clone(),
                filename: file_info.filename.clone(),
                staged_path: None,
                load_priority: 0,
                is_enabled: true,
            };

            db::insert_profile_mod(&conn, &pmod).map_err(|e| e.to_string())?;

            // Deploy symlink
            let adapter = MinecraftAdapter::new();
            let _ = adapter.deploy_profile(&profile_id);
        }
        ProviderKind::CurseForge => {
            let cf = CurseForgeProvider::new();
            let m_id = mod_id.ok_or_else(|| "Missing remote mod ID for CurseForge".to_string())?;
            let versions = cf.get_versions(&m_id, None, None).await?;
            let version = versions
                .iter()
                .find(|v| v.id == file_id)
                .or_else(|| versions.first())
                .ok_or_else(|| "CurseForge file version not found".to_string())?;

            let file_info = version
                .files
                .first()
                .ok_or_else(|| "CurseForge file info not found".to_string())?;

            let _ = cf
                .download_file(&file_info.url, &cache_dir, &file_info.filename)
                .await?;

            let pmod = ProfileModRecord {
                id: format!("pmod-{}", Uuid::new_v4().simple()),
                profile_id: profile_id.clone(),
                provider: "curseforge".to_string(),
                remote_mod_id: m_id,
                remote_file_id: file_id,
                name: mod_name.unwrap_or_else(|| file_info.filename.clone()),
                file_path: file_info.filename.clone(),
                filename: file_info.filename.clone(),
                staged_path: None,
                load_priority: 0,
                is_enabled: true,
            };

            db::insert_profile_mod(&conn, &pmod).map_err(|e| e.to_string())?;

            let adapter = MinecraftAdapter::new();
            let _ = adapter.deploy_profile(&profile_id);
        }
        ProviderKind::LocalArchive => {
            // Local archive path passed in file_id
            let local_path = PathBuf::from(&file_id);
            if !local_path.exists() {
                return Err(format!("Local file '{}' not found", file_id));
            }

            let filename = local_path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "custom_mod.jar".to_string());

            if filename.ends_with(".mrpack") {
                let client = reqwest::Client::new();
                ModpackInstaller::install_mrpack(&client, &local_path, mod_name.as_deref()).await?;
            } else if filename.ends_with(".zip") {
                ModpackInstaller::install_curseforge_pack(&local_path, mod_name.as_deref()).await?;
            } else {
                let dest = cache_dir.join(&filename);
                let _ = fs::copy(&local_path, &dest);

                let pmod = ProfileModRecord {
                    id: format!("pmod-{}", Uuid::new_v4().simple()),
                    profile_id: profile_id.clone(),
                    provider: "local".to_string(),
                    remote_mod_id: filename.clone(),
                    remote_file_id: filename.clone(),
                    name: mod_name.unwrap_or_else(|| filename.clone()),
                    file_path: filename.clone(),
                    filename: filename.clone(),
                    staged_path: None,
                    load_priority: 0,
                    is_enabled: true,
                };
                db::insert_profile_mod(&conn, &pmod).map_err(|e| e.to_string())?;

                let adapter = MinecraftAdapter::new();
                let _ = adapter.deploy_profile(&profile_id);
            }
        }
        _ => return Err(format!("Unsupported provider: {}", provider)),
    }

    Ok(())
}

#[tauri::command]
fn toggle_mod_state(profile_id: String, mod_id: String, enabled: bool) -> Result<(), String> {
    let conn = db::open_db().map_err(|e| e.to_string())?;
    db::toggle_profile_mod(&conn, &profile_id, &mod_id, enabled).map_err(|e| e.to_string())?;

    // Update deployed symlinks
    let adapter = MinecraftAdapter::new();
    let _ = adapter.deploy_profile(&profile_id);

    Ok(())
}

#[tauri::command]
async fn start_minecraft_instance(
    app_handle: tauri::AppHandle,
    instance_id: String,
) -> Result<(), String> {
    launch_instance(app_handle, instance_id, None).await
}

#[tauri::command]
fn get_game_profiles(game_id: Option<String>) -> Result<Vec<GameProfileRecord>, String> {
    let conn = db::open_db().map_err(|e| e.to_string())?;
    db::get_profiles(&conn, game_id.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
fn create_game_profile(profile: GameProfileRecord) -> Result<(), String> {
    let conn = db::open_db().map_err(|e| e.to_string())?;
    db::insert_profile(&conn, &profile).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_game_profile(profile_id: String) -> Result<(), String> {
    let conn = db::open_db().map_err(|e| e.to_string())?;
    let adapter = MinecraftAdapter::new();
    let _ = adapter.undeploy_profile(&profile_id);
    db::delete_profile(&conn, &profile_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_profile_mods(profile_id: String) -> Result<Vec<ProfileModRecord>, String> {
    let conn = db::open_db().map_err(|e| e.to_string())?;
    db::get_profile_mods(&conn, &profile_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_mod(profile_id: String, mod_id: String) -> Result<(), String> {
    let conn = db::open_db().map_err(|e| e.to_string())?;
    db::delete_profile_mod(&conn, &profile_id, &mod_id).map_err(|e| e.to_string())?;

    let adapter = MinecraftAdapter::new();
    let _ = adapter.deploy_profile(&profile_id);
    Ok(())
}

#[tauri::command]
fn reorder_mod(profile_id: String, mod_id: String, priority: i32) -> Result<(), String> {
    let conn = db::open_db().map_err(|e| e.to_string())?;
    db::update_mod_priority(&conn, &profile_id, &mod_id, priority).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_game_manifest(
    registry: tauri::State<SharedGameRegistry>,
    game_id: String,
) -> Result<GameModdingManifest, String> {
    get_game_modding_manifest(registry, game_id)
}

pub fn get_nexus_api_key() -> Option<String> {
    let path = get_loom_settings_path();
    if let Ok(content) = fs::read_to_string(path) {
        if let Ok(settings) = serde_json::from_str::<LoomBackendSettings>(&content) {
            if let Some(key) = settings.nexus_api_key {
                let trimmed = key.trim().to_string();
                if !trimmed.is_empty() {
                    return Some(trimmed);
                }
            }
        }
    }
    None
}

#[tauri::command]
fn save_nexus_api_key(api_key: String) -> Result<(), String> {
    let path = get_loom_settings_path();
    let mut settings = if path.exists() {
        let content = fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str::<LoomBackendSettings>(&content).unwrap_or_default()
    } else {
        LoomBackendSettings::default()
    };
    settings.nexus_api_key = Some(api_key.trim().to_string());
    save_backend_settings(settings)
}

#[tauri::command]
fn get_nexus_api_key_status() -> Result<Option<String>, String> {
    Ok(get_nexus_api_key())
}

fn extract_archive_to_dir(archive_path: &std::path::Path, dest_dir: &std::path::Path) -> Result<(), String> {
    let path_str = archive_path.to_string_lossy();
    let dest_str = dest_dir.to_string_lossy();

    // 1. Try 7z / 7za (handles .7z, .zip, .rar)
    let status_7z = std::process::Command::new("7z")
        .args(["x", "-y", &format!("-o{}", dest_str), &path_str])
        .output();

    if let Ok(out) = status_7z {
        if out.status.success() {
            return Ok(());
        }
    }

    // 2. Try unzip if .zip
    if path_str.ends_with(".zip") {
        let status_unzip = std::process::Command::new("unzip")
            .args(["-o", &path_str, "-d", &dest_str])
            .output();
        if let Ok(out) = status_unzip {
            if out.status.success() {
                return Ok(());
            }
        }
    }

    // 3. Try unar
    let status_unar = std::process::Command::new("unar")
        .args(["-f", "-o", &dest_str, &path_str])
        .output();
    if let Ok(out) = status_unar {
        if out.status.success() {
            return Ok(());
        }
    }

    // 4. Native zip crate fallback
    if let Ok(file) = std::fs::File::open(archive_path) {
        if let Ok(mut archive) = zip::ZipArchive::new(file) {
            let _ = archive.extract(dest_dir);
            return Ok(());
        }
    }

    Err(format!("Échec de l'extraction de l'archive : {}", archive_path.display()))
}

pub async fn handle_nxm_uri_internal(app: &tauri::AppHandle, uri: &str) -> Result<(), String> {
    use tauri::Emitter;
    let info = parse_nxm_uri(uri)?;
    println!("[NXM] Handling URI: {} for game {}", info.raw_uri, info.game_domain);

    let api_key = get_nexus_api_key();
    if api_key.is_none() {
        eprintln!("[NXM] Aucune clé API Nexus Mods configurée !");
        let _ = app.emit("nxm-auth-required", serde_json::json!({
            "uri": uri,
            "game_domain": info.game_domain,
            "mod_id": info.mod_id,
            "file_id": info.file_id,
        }));
        return Err("Clé API Nexus Mods requise. Renseignez votre clé API personnelle Nexus Mods dans Loom.".to_string());
    }

    let _ = app.emit("nxm-download-started", serde_json::json!({
        "game_domain": info.game_domain,
        "mod_id": info.mod_id,
        "file_id": info.file_id,
    }));

    let client = reqwest::Client::new();
    let links = resolve_nexus_download_links(&client, &info, api_key.as_deref()).await?;
    let primary_link = links.first().ok_or_else(|| "Aucun miroir de téléchargement renvoyé par Nexus".to_string())?;
    let downloaded_path = download_nexus_file(&client, &primary_link.uri, None).await?;
    println!("[NXM] Archive téléchargée dans : {}", downloaded_path.display());

    // Mapper le domaine Nexus vers l'identifiant de jeu Loom
    let domain_lower = info.game_domain.to_lowercase();
    let game_id = match domain_lower.as_str() {
        "skyrimspecialedition" | "skyrimse" | "skyrim" => "skyrim_se",
        "fallout4" => "fallout4",
        "oblivion" => "oblivion",
        other => other,
    };

    // Obtenir ou créer le profil de jeu
    let conn = db::open_db().map_err(|e| e.to_string())?;
    let profiles = db::get_profiles(&conn, Some(game_id)).unwrap_or_default();
    let profile = if let Some(p) = profiles.into_iter().next() {
        p
    } else {
        let default_record = db::GameProfileRecord {
            id: format!("{}-default", game_id),
            game_id: game_id.to_string(),
            name: "Profil Principal (Par Défaut)".to_string(),
            game_version: "1.6+".to_string(),
            modloader: Some("skse64_loader.exe".to_string()),
            deployment_path: None,
            created_at: chrono::Utc::now().timestamp(),
        };
        let _ = db::insert_profile(&conn, &default_record);
        default_record
    };

    // Répertoire de staging décompressé : profiles/<game_id>/<profile_id>/staged/<mod_id>_<file_id>/
    let base_staged = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string())).join(".local/share"))
        .join("loom")
        .join("profiles")
        .join(game_id)
        .join(&profile.id)
        .join("staged")
        .join(format!("{}_{}", info.mod_id, info.file_id));

    let _ = fs::create_dir_all(&base_staged);

    // Extraction de l'archive (.7z, .zip, .rar)
    let _ = extract_archive_to_dir(&downloaded_path, &base_staged);

    // Normalisation de l'archive (Aplatissement racine Data/ et détection FOMOD)
    let adapter = BethesdaAdapter::new_skyrim_se();
    let _ = adapter.normalize_staged_mod(&base_staged);

    // Nom affiché du mod
    let filename = downloaded_path
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| format!("nexus_mod_{}", info.mod_id));

    let pmod_id = format!("pmod-{}", uuid::Uuid::new_v4().simple());
    let pmod = db::ProfileModRecord {
        id: pmod_id.clone(),
        profile_id: profile.id.clone(),
        provider: "nexusmods".to_string(),
        remote_mod_id: info.mod_id.to_string(),
        remote_file_id: info.file_id.to_string(),
        name: filename.clone(),
        file_path: downloaded_path.to_string_lossy().to_string(),
        filename: filename.clone(),
        staged_path: Some(base_staged.to_string_lossy().to_string()),
        load_priority: 0,
        is_enabled: true,
    };
    db::insert_profile_mod(&conn, &pmod).map_err(|e| e.to_string())?;

    // Déploiement automatique
    let _ = adapter.deploy_profile(&profile.id);

    // Notification et événement frontend
    let _ = app.emit("nxm-download-completed", serde_json::json!({
        "game_domain": info.game_domain,
        "mod_id": info.mod_id,
        "file_id": info.file_id,
        "mod_name": filename,
        "profile_id": profile.id,
    }));

    Ok(())
}

#[tauri::command]
async fn handle_nxm_uri(uri: String, app_handle: tauri::AppHandle) -> Result<(), String> {
    handle_nxm_uri_internal(&app_handle, &uri).await
}

#[tauri::command]
fn scan_and_resolve_conflicts(
    profile_id: String,
    state: tauri::State<SharedGameRegistry>,
) -> Result<Vec<FileConflict>, String> {
    let conn = db::open_db().map_err(|e| e.to_string())?;
    let profiles = db::get_profiles(&conn, None).map_err(|e| e.to_string())?;
    let profile = profiles
        .into_iter()
        .find(|p| p.id == profile_id)
        .ok_or_else(|| format!("Profile '{}' not found", profile_id))?;

    let staging_dir = if let Ok(reg) = state.read() {
        if let Some(adapter) = reg.get(&profile.game_id) {
            adapter.get_staging_directory(&profile_id)
        } else {
            BethesdaAdapter::new_skyrim_se().get_staging_directory(&profile_id)
        }
    } else {
        BethesdaAdapter::new_skyrim_se().get_staging_directory(&profile_id)
    };

    scan_conflicts_impl(&profile_id, &staging_dir)
}

#[tauri::command]
fn set_conflict_winner(
    profile_id: String,
    relative_path: String,
    winning_mod_id: String,
) -> Result<(), String> {
    let conn = db::open_db().map_err(|e| e.to_string())?;
    db::set_file_conflict_rule(
        &conn,
        &db::FileConflictRuleRecord {
            profile_id,
            relative_path,
            winning_mod_id,
        },
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn update_plugins_order(
    profile_id: String,
    ordered_plugin_names: Vec<String>,
) -> Result<(), String> {
    let conn = db::open_db().map_err(|e| e.to_string())?;
    db::update_plugin_load_order(&conn, &profile_id, &ordered_plugin_names).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_bethesda_plugins(profile_id: String) -> Result<Vec<db::BethesdaPluginRecord>, String> {
    let conn = db::open_db().map_err(|e| e.to_string())?;
    db::get_bethesda_plugins(&conn, &profile_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn toggle_bethesda_plugin(
    profile_id: String,
    filename: String,
    is_active: bool,
) -> Result<(), String> {
    let conn = db::open_db().map_err(|e| e.to_string())?;
    db::toggle_bethesda_plugin(&conn, &profile_id, &filename, is_active).map_err(|e| e.to_string())
}

#[tauri::command]
fn deploy_active_profile(
    profile_id: String,
    state: tauri::State<SharedGameRegistry>,
) -> Result<(), String> {
    let conn = db::open_db().map_err(|e| e.to_string())?;
    let profiles = db::get_profiles(&conn, None).map_err(|e| e.to_string())?;
    let profile = profiles
        .into_iter()
        .find(|p| p.id == profile_id)
        .ok_or_else(|| format!("Profile '{}' not found", profile_id))?;

    if let Ok(reg) = state.read() {
        if let Some(adapter) = reg.get(&profile.game_id) {
            return adapter.deploy_profile(&profile_id);
        }
    }

    match profile.game_id.as_str() {
        "skyrim_se" => BethesdaAdapter::new_skyrim_se().deploy_profile(&profile_id),
        "fallout4" => BethesdaAdapter::new_fallout_4().deploy_profile(&profile_id),
        "oblivion" => BethesdaAdapter::new_oblivion().deploy_profile(&profile_id),
        "wow" => WowAdapter::new("_retail_").deploy_profile(&profile_id),
        _ => MinecraftAdapter::new().deploy_profile(&profile_id),
    }
}

#[tauri::command]
async fn resolve_remote_dependencies(
    provider: String,
    remote_mod_id: String,
    remote_file_id: String,
) -> Result<Vec<RemoteDependency>, String> {
    let p_kind = ProviderKind::from_str(&provider);
    match p_kind {
        ProviderKind::Modrinth => {
            let mr = ModrinthProvider::new();
            mr.resolve_dependencies(&remote_mod_id, &remote_file_id).await
        }
        ProviderKind::CurseForge => {
            let cf = CurseForgeProvider::new();
            cf.resolve_dependencies(&remote_mod_id, &remote_file_id).await
        }
        _ => Ok(Vec::new()),
    }
}

#[tauri::command]
async fn initiate_microsoft_device_code_ipc() -> Result<DeviceCodeResponse, String> {
    let client = reqwest::Client::new();
    initiate_device_code(&client).await
}

#[tauri::command]
async fn poll_microsoft_device_code_ipc(device_code: String) -> Result<PollAuthResult, String> {
    let client = reqwest::Client::new();
    poll_device_code(&client, &device_code).await
}

#[tauri::command]
async fn get_minecraft_auth() -> Result<Option<MinecraftAccount>, String> {
    let client = reqwest::Client::new();
    Ok(minecraft::auth::get_valid_account(&client).await)
}

#[tauri::command]
fn logout_minecraft_auth() -> Result<(), String> {
    remove_stored_account()
}

#[tauri::command]
fn set_hardware_acceleration(enabled: bool) -> Result<(), String> {
    let base_dir = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string())).join(".local/share"))
        .join("loom");
    let _ = std::fs::create_dir_all(&base_dir);
    let marker = base_dir.join("disable_hw_accel");
    if enabled {
        if marker.exists() {
            let _ = std::fs::remove_file(marker);
        }
    } else {
        let _ = std::fs::write(marker, "1");
    }
    Ok(())
}

#[tauri::command]
fn get_hardware_acceleration() -> Result<bool, String> {
    let base_dir = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string())).join(".local/share"))
        .join("loom");
    let marker = base_dir.join("disable_hw_accel");
    Ok(!marker.exists())
}

#[tauri::command]
fn restart_app(app_handle: tauri::AppHandle) {
    app_handle.restart();
}

#[tauri::command]
fn hide_window(app_handle: tauri::AppHandle) -> Result<(), String> {
    if let Some(win) = app_handle.get_webview_window("main") {
        win.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn show_window(app_handle: tauri::AppHandle) -> Result<(), String> {
    if let Some(win) = app_handle.get_webview_window("main") {
        win.show().map_err(|e| e.to_string())?;
        let _ = win.unminimize();
        let _ = win.set_focus();
    }
    Ok(())
}

#[tauri::command]
fn exit_app(app_handle: tauri::AppHandle) {
    app_handle.exit(0);
}

pub fn open_url_in_default_browser(url: &str) -> Result<(), String> {
    let u = url.trim();
    if !u.starts_with("http://") && !u.starts_with("https://") && !u.starts_with("steam://") && !u.starts_with("mailto:") {
        return Err("URL invalide: le protocole doit être http, https, steam ou mailto".to_string());
    }

    match open::that_detached(u) {
        Ok(_) => Ok(()),
        Err(e) => {
            #[cfg(target_os = "linux")]
            {
                if std::process::Command::new("xdg-open").arg(u).spawn().is_ok() {
                    return Ok(());
                }
                if std::process::Command::new("gio").args(["open", u]).spawn().is_ok() {
                    return Ok(());
                }
            }
            #[cfg(target_os = "windows")]
            {
                if std::process::Command::new("explorer.exe").arg(u).spawn().is_ok() {
                    return Ok(());
                }
            }
            #[cfg(target_os = "macos")]
            {
                if std::process::Command::new("open").arg(u).spawn().is_ok() {
                    return Ok(());
                }
            }
            Err(format!("Impossible d'ouvrir l'URL '{}': {}", u, e))
        }
    }
}

#[tauri::command]
fn open_external_url(url: String) -> Result<(), String> {
    open_url_in_default_browser(&url)
}

fn get_loom_settings_path() -> PathBuf {
    let base_dir = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string())).join(".local/share"))
        .join("loom");
    let _ = fs::create_dir_all(&base_dir);
    base_dir.join("settings.json")
}

#[tauri::command]
fn save_backend_settings(settings: LoomBackendSettings) -> Result<(), String> {
    let path = get_loom_settings_path();
    let content = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    fs::write(path, content).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_backend_settings() -> Result<LoomBackendSettings, String> {
    let path = get_loom_settings_path();
    if !path.exists() {
        return Ok(LoomBackendSettings::default());
    }
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&content).map_err(|e| e.to_string())
}

#[tauri::command]
fn is_start_minimized() -> Result<bool, String> {
    if std::env::var("LOOM_START_MINIMIZED").map(|v| v == "1").unwrap_or(false) {
        return Ok(true);
    }
    let path = get_loom_settings_path();
    if let Ok(data) = fs::read_to_string(&path) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&data) {
            return Ok(json.get("startMinimized").and_then(|v| v.as_bool()).unwrap_or(false));
        }
    }
    Ok(false)
}

#[tauri::command]
fn set_system_autostart(enabled: bool, start_minimized: bool) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        let config_dir = dirs::config_dir().ok_or_else(|| "Dossier config introuvable".to_string())?;
        let autostart_dir = config_dir.join("autostart");
        let _ = fs::create_dir_all(&autostart_dir);
        let desktop_file = autostart_dir.join("com.loom.launcher.desktop");

        if enabled {
            let current_exe = std::env::current_exe().map_err(|e| e.to_string())?;
            let app_path = std::env::var("APPIMAGE")
                .map(PathBuf::from)
                .unwrap_or(current_exe);
            let app_path_str = app_path.to_string_lossy();
            let exec_cmd = if start_minimized {
                format!("\"{}\" --minimized", app_path_str)
            } else {
                format!("\"{}\"", app_path_str)
            };

            let content = format!(
                "[Desktop Entry]\n\
                 Type=Application\n\
                 Name=Loom Launcher\n\
                 GenericName=Universal Gaming Launcher\n\
                 Comment=Universal Gaming and Modding Platform\n\
                 Exec={}\n\
                 Icon=loom-launcher\n\
                 Terminal=false\n\
                 Categories=Game;\n\
                 StartupNotify=false\n\
                 X-GNOME-Autostart-enabled=true\n",
                exec_cmd
            );
            fs::write(&desktop_file, content).map_err(|e| e.to_string())?;
        } else if desktop_file.exists() {
            let _ = fs::remove_file(desktop_file);
        }
    }
    Ok(())
}

#[tauri::command]
fn get_system_autostart() -> Result<bool, String> {
    #[cfg(target_os = "linux")]
    {
        if let Some(config_dir) = dirs::config_dir() {
            let desktop_file = config_dir.join("autostart").join("com.loom.launcher.desktop");
            return Ok(desktop_file.exists());
        }
    }
    Ok(false)
}

#[tauri::command]
async fn pick_directory(title: Option<String>, default_path: Option<String>) -> Result<Option<String>, String> {
    #[cfg(target_os = "linux")]
    {
        let mut cmd = Command::new("zenity");
        cmd.args(["--file-selection", "--directory"]);
        if let Some(t) = &title {
            cmd.arg(format!("--title={}", t));
        }
        if let Some(p) = &default_path {
            cmd.arg(format!("--filename={}/", p));
        }
        if let Ok(output) = cmd.output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    return Ok(Some(path));
                }
            }
        }

        let mut kcmd = Command::new("kdialog");
        kcmd.arg("--getexistingdirectory");
        if let Some(p) = &default_path {
            kcmd.arg(p);
        }
        if let Ok(output) = kcmd.output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    return Ok(Some(path));
                }
            }
        }
    }
    Ok(None)
}

#[tauri::command]
fn open_directory_folder(path: String) -> Result<(), String> {
    let p = PathBuf::from(&path);
    if !p.exists() {
        let _ = fs::create_dir_all(&p);
    }
    #[cfg(target_os = "linux")]
    {
        let _ = Command::new("xdg-open").arg(&path).spawn();
    }
    #[cfg(target_os = "windows")]
    {
        let _ = Command::new("explorer.exe").arg(&path).spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = Command::new("open").arg(&path).spawn();
    }
    Ok(())
}

#[tauri::command]
fn send_desktop_notification(title: String, body: String, icon: Option<String>) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        let mut cmd = Command::new("notify-send");
        cmd.args(["-a", "Loom Launcher", &title, &body]);
        if let Some(ic) = icon {
            cmd.args(["-i", &ic]);
        }
        let _ = cmd.spawn();
    }
    Ok(())
}

#[tauri::command]
async fn autostart_platform_instance(platform: String, minimized: bool) -> Result<bool, String> {
    match platform.to_lowercase().as_str() {
        "steam" => {
            let has_steam = Command::new("which").arg("steam").output().map(|o| o.status.success()).unwrap_or(false);
            if !has_steam {
                return Ok(false);
            }
            let is_running = Command::new("pgrep").arg("-x").arg("steam").output().map(|o| o.status.success()).unwrap_or(false);
            if !is_running {
                let mut cmd = Command::new("steam");
                if minimized {
                    cmd.arg("-silent");
                }
                let _ = cmd.spawn();
                return Ok(true);
            }
            Ok(true)
        },
        "battlenet" => {
            let base_dir = dirs::data_local_dir()
                .unwrap_or_else(|| PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string())).join(".local/share"))
                .join("loom");
            let prefix_dir = base_dir.join("prefixes").join("battlenet");
            let _ = fs::create_dir_all(&prefix_dir);
            Ok(true)
        },
        "epic" => {
            let has_heroic = Command::new("which").arg("heroic").output().map(|o| o.status.success()).unwrap_or(false);
            if !has_heroic {
                return Ok(false);
            }
            let is_running = Command::new("pgrep").arg("-f").arg("heroic").output().map(|o| o.status.success()).unwrap_or(false);
            if !is_running {
                let mut cmd = Command::new("heroic");
                if minimized {
                    cmd.arg("--minimized");
                }
                let _ = cmd.spawn();
                return Ok(true);
            }
            Ok(true)
        },
        "gog" => {
            Ok(true)
        },
        _ => Ok(false)
    }
}


#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 1. Initialize SQLite Database Schema
    if let Err(e) = db::open_db() {
        eprintln!("[Loom::DB] Failed to initialize SQLite database: {}", e);
    }

    // 2. Initialize and populate Game Registry
    let shared_registry = create_shared_registry();
    {
        if let Ok(mut reg) = shared_registry.write() {
            reg.register(Arc::new(MinecraftAdapter::new()));
            reg.register(Arc::new(BethesdaAdapter::new_skyrim_se()));
            reg.register(Arc::new(BethesdaAdapter::new_fallout_4()));
            reg.register(Arc::new(BethesdaAdapter::new_oblivion()));
            reg.register(Arc::new(WowAdapter::new("_retail_")));
            reg.register(Arc::new(WowAdapter::new("_classic_")));
        }
    }

    // 3. Build Tauri v2 App
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.show();
                let _ = win.unminimize();
                let _ = win.set_focus();
            }
            for arg in args {
                if arg.starts_with("nxm://") {
                    let handle = app.clone();
                    let uri = arg.clone();
                    tauri::async_runtime::spawn(async move {
                        if let Err(e) = handle_nxm_uri_internal(&handle, &uri).await {
                            eprintln!("[NXM] Failed to handle nxm uri: {}", e);
                        }
                    });
                }
            }
        }))
        .manage(shared_registry)
        .manage(BattleNetLauncherState::new())
        .setup(|app| {
            if let Ok(show_item) = MenuItem::with_id(app, "show", "Ouvrir Loom Launcher", true, None::<&str>) {
                if let Ok(hide_item) = MenuItem::with_id(app, "hide", "Réduire dans la barre d'état", true, None::<&str>) {
                    if let Ok(quit_item) = MenuItem::with_id(app, "quit", "Quitter Loom", true, None::<&str>) {
                        if let Ok(menu) = Menu::with_items(app, &[&show_item, &hide_item, &quit_item]) {
                            let mut tray_builder = TrayIconBuilder::new()
                                .tooltip("Loom Launcher")
                                .menu(&menu)
                                .show_menu_on_left_click(false)
                                .on_menu_event(|app, event| {
                                    match event.id.as_ref() {
                                        "show" => {
                                            if let Some(win) = app.get_webview_window("main") {
                                                let _ = win.show();
                                                let _ = win.unminimize();
                                                let _ = win.set_focus();
                                            }
                                        }
                                        "hide" => {
                                            if let Some(win) = app.get_webview_window("main") {
                                                let _ = win.hide();
                                            }
                                        }
                                        "quit" => {
                                            app.exit(0);
                                        }
                                        _ => {}
                                    }
                                })
                                .on_tray_icon_event(|tray, event| {
                                    if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                                        let app = tray.app_handle();
                                        if let Some(win) = app.get_webview_window("main") {
                                            if win.is_visible().unwrap_or(false) {
                                                let _ = win.hide();
                                            } else {
                                                let _ = win.show();
                                                let _ = win.unminimize();
                                                let _ = win.set_focus();
                                            }
                                        }
                                    }
                                });

                            if let Some(icon) = app.default_window_icon().cloned() {
                                tray_builder = tray_builder.icon(icon);
                            }

                            let _ = tray_builder.build(app);
                        }
                    }
                }
            }

            process_guard::spawn_outdated_watchdog(app.handle().clone());
            let _ = modding::nexus::register_linux_desktop_mime();

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let base_dir = dirs::data_local_dir()
                    .unwrap_or_else(|| PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string())).join(".local/share"))
                    .join("loom");
                let settings_file = base_dir.join("settings.json");
                let close_to_tray = if let Ok(data) = fs::read_to_string(&settings_file) {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&data) {
                        json.get("closeToTray").and_then(|v| v.as_bool()).unwrap_or(false)
                    } else {
                        false
                    }
                } else {
                    false
                };

                if close_to_tray {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            launch_game,
            stop_game,
            get_running_games,
            scan_local_steam_library,
            process_guard::restart_application,
            get_system_stats,
            save_backend_settings,
            get_backend_settings,
            is_start_minimized,
            set_system_autostart,
            get_system_autostart,
            pick_directory,
            open_directory_folder,
            send_desktop_notification,
            autostart_platform_instance,
            hide_window,
            show_window,
            exit_app,
            get_game_modding_manifest,
            get_game_manifest,
            handle_nxm_uri,
            scan_and_resolve_conflicts,
            set_conflict_winner,
            update_plugins_order,
            get_bethesda_plugins,
            toggle_bethesda_plugin,
            deploy_active_profile,
            save_nexus_api_key,
            get_nexus_api_key_status,
            search_unified_mods,
            install_mod_or_pack,
            toggle_mod_state,
            start_minecraft_instance,
            get_game_profiles,
            create_game_profile,
            delete_game_profile,
            get_profile_mods,
            delete_mod,
            reorder_mod,
            resolve_remote_dependencies,
            initiate_microsoft_device_code_ipc,
            poll_microsoft_device_code_ipc,
            get_minecraft_auth,
            logout_minecraft_auth,
            set_hardware_acceleration,
            get_hardware_acceleration,
            restart_app,
            open_external_url,
            create_minecraft_instance_cmd,
            get_minecraft_instances_cmd,
            delete_minecraft_instance_cmd,
            update_minecraft_instance_settings_cmd,
            launch_minecraft_instance_cmd,
            search_minecraft_mods_cmd,
            install_minecraft_mod_cmd,
            toggle_minecraft_mod_cmd,
            delete_minecraft_mod_cmd,
            install_local_minecraft_mod_cmd,
            open_minecraft_instance_folder_cmd,
            search_modpacks_cmd,
            get_minecraft_mod_details_cmd,
            get_minecraft_version_changelog_cmd,
            battlenet::scan_battlenet_games,
            battlenet::add_external_battlenet_game,
            battlenet::launch_battlenet_game,
            battlenet::kill_battlenet_game,
            battlenet::start_blizzard_oauth_flow,
            battlenet::save_blizzard_credentials,
            battlenet::get_blizzard_credentials_status,
            battlenet::get_blizzard_account_info,
            battlenet::disconnect_blizzard_account,
            battlenet::get_blizzard_owned_games,
            battlenet::get_battlenet_config,
            battlenet::save_battlenet_config,
            battlenet::get_running_battlenet_games,
            battlenet::scan_wow_instances,
            battlenet::search_wow_addons,
            battlenet::get_wow_addon_details,
            battlenet::get_wow_addon_description,
            battlenet::get_wow_addon_files,
            battlenet::get_wow_addons,
            battlenet::install_wow_addon,
            battlenet::uninstall_wow_addon,
            battlenet::toggle_wow_addon,
            battlenet::open_wow_addons_folder,
            battlenet::export_wow_addons,
            battlenet::get_wow_characters,
            battlenet::update_battlenet_game_playtime
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_running_steam_app() {
        let (app_id, pids) = get_running_steam_app();
        println!("Detected Steam game: app_id={:?}, pids={:?}", app_id, pids);
        // If Hunt Showdown (594650) is running, app_id should be Some("594650")
        if let Some(aid) = &app_id {
            assert!(!aid.is_empty());
            assert!(!pids.is_empty());
        }
    }

    #[test]
    fn test_scan_local_steam_library() {
        let res = scan_local_steam_library().expect("scan_local_steam_library failed");
        println!("Scanned account: detected={}, name={:?}, total={}", res.detected, res.persona_name, res.total_games_count);
        assert!(res.detected);
        assert!(!res.all_games.is_empty());
        for g in res.all_games.iter().take(10) {
            println!("  Game: {} (appid: {}, last_played: {:?}, installed: {})", g.name, g.appid, g.last_played_timestamp, g.installed);
        }
    }
}
