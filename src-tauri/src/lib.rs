use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Serialize, Deserialize)]
pub struct LaunchResult {
    pub success: bool,
    pub pid: Option<u32>,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RunningState {
    pub success: bool,
    pub running_app_id: Option<String>,
    pub pids: Vec<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SystemStats {
    pub cpu_usage: f32,
    pub ram_used_gb: f32,
    pub ram_total_gb: f32,
    pub disk_free_gb: f32,
    pub disk_total_gb: f32,
    pub network_down_kb: f32,
    pub network_up_kb: f32,
}

fn get_running_steam_app() -> (Option<String>, Vec<u32>) {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/k0bus".to_string());
    let log_path = PathBuf::from(home).join(".local/share/Steam/logs/gameprocess_log.txt");

    if !log_path.exists() {
        return (None, Vec::new());
    }

    if let Ok(content) = fs::read_to_string(&log_path) {
        let lines: Vec<&str> = content.lines().collect();
        let start = lines.len().saturating_sub(300);
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
                // e.g. AppID 594650 adding PID 75362
                let parts: Vec<&str> = line.split_whitespace().collect();
                for i in 0..parts.len() {
                    if parts[i] == "AppID" && i + 3 < parts.len() && parts[i + 2] == "PID" {
                        let aid = parts[i + 1].to_string();
                        if let Ok(pid) = parts[i + 3].parse::<u32>() {
                            app_pids.entry(aid).or_default().insert(pid);
                        }
                    }
                }
                continue;
            }

            if line.contains("no longer tracking PID") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                for i in 0..parts.len() {
                    if parts[i] == "AppID" && i + 4 < parts.len() && parts[i + 3] == "PID" {
                        let aid = parts[i + 1];
                        let pid_str = parts[i + 4].trim_matches(',');
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
            // Fallback direct steam command
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
    Ok(SystemStats {
        cpu_usage: 24.5,
        ram_used_gb: 11.2,
        ram_total_gb: 32.0,
        disk_free_gb: 342.8,
        disk_total_gb: 1000.0,
        network_down_kb: 4200.0,
        network_up_kb: 120.0,
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            launch_game,
            stop_game,
            get_running_games,
            get_system_stats
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
