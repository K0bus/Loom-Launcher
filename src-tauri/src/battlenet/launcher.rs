use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::RwLock;

use crate::db::{self, BattleNetConfigRecord};

#[derive(Clone, Default)]
pub struct BattleNetLauncherState {
    /// Maps product_uid to process PID
    running_pids: Arc<RwLock<HashMap<String, u32>>>,
}

impl BattleNetLauncherState {
    pub fn new() -> Self {
        Self {
            running_pids: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn is_running(&self, product_uid: &str) -> bool {
        let scanned = scan_running_battlenet_processes();
        scanned.contains_key(product_uid)
    }

    pub async fn get_running_games(&self) -> Vec<String> {
        let scanned = scan_running_battlenet_processes();
        let mut map = self.running_pids.write().await;
        *map = scanned.clone();
        scanned.into_keys().collect()
    }
}

/// Proactively scans Linux /proc for running Blizzard game client processes (strictly excluding launchers/agents)
pub fn scan_running_battlenet_processes() -> HashMap<String, u32> {
    let mut detected = HashMap::new();

    let proc_dir = match std::fs::read_dir("/proc") {
        Ok(dir) => dir,
        Err(_) => return detected,
    };

    let my_pid = std::process::id();

    for entry in proc_dir.flatten() {
        let file_name = entry.file_name();
        let name_str = file_name.to_string_lossy();
        if let Ok(pid) = name_str.parse::<u32>() {
            if pid == my_pid {
                continue;
            }
            let cmdline_path = entry.path().join("cmdline");
            if let Ok(cmd_bytes) = std::fs::read(cmdline_path) {
                if cmd_bytes.is_empty() {
                    continue;
                }
                let cmd_str = String::from_utf8_lossy(&cmd_bytes);
                let args: Vec<&str> = cmd_str.split('\0').filter(|s| !s.is_empty()).collect();
                if args.is_empty() {
                    continue;
                }

                let exe0 = args[0].to_lowercase();
                if exe0.contains("node")
                    || exe0.contains("python")
                    || exe0.contains("bash")
                    || exe0.contains("grep")
                    || exe0.contains("cargo")
                    || exe0.contains("wineserver")
                {
                    continue;
                }

                // Strictly ignore launcher, updater, and helper processes
                let is_launcher = args.iter().any(|arg| {
                    let a = arg.to_lowercase();
                    a.ends_with("battle.net.exe")
                        || a.ends_with("battle.net launcher.exe")
                        || a.ends_with("agent.exe")
                        || a.ends_with("agenthelper.exe")
                        || a.ends_with("blizzarderror.exe")
                        || a.ends_with("blizzard uninstaller.exe")
                        || a.ends_with("winedevice.exe")
                        || a.ends_with("conhost.exe")
                });
                if is_launcher {
                    continue;
                }

                // 1. Look for explicit -uid argument on the game binary (e.g., Wow.exe ... -uid wowf)
                let mut found_uid: Option<String> = None;
                for i in 0..args.len() {
                    if args[i] == "-uid" && i + 1 < args.len() {
                        let uid_val = args[i + 1].trim().to_lowercase();
                        const IGNORED_UIDS: &[&str] = &[
                            "agent", "bna", "battle.net", "battlenet", "bdk", "bdko", "client"
                        ];
                        if !uid_val.is_empty() && !IGNORED_UIDS.contains(&uid_val.as_str()) {
                            found_uid = Some(uid_val);
                            break;
                        }
                    }
                }

                // 2. If not found via -uid, deduce from path and binary name
                if found_uid.is_none() {
                    let cmd_lower = cmd_str.to_lowercase();
                    if cmd_lower.contains("_forever_\\wow.exe") || cmd_lower.contains("_forever_/wow.exe") {
                        found_uid = Some("wowf".to_string());
                    } else if cmd_lower.contains("_classic_era_\\wowclassic.exe") || cmd_lower.contains("_classic_era_/wowclassic.exe") {
                        found_uid = Some("wow_classic_era".to_string());
                    } else if cmd_lower.contains("_classic_\\wowclassic.exe") || cmd_lower.contains("_classic_/wowclassic.exe") || cmd_lower.contains("wowclassic.exe") {
                        found_uid = Some("wow_classic".to_string());
                    } else if cmd_lower.contains("_retail_\\wow.exe") || cmd_lower.contains("_retail_/wow.exe") || cmd_lower.contains("wow.exe") {
                        found_uid = Some("wow".to_string());
                    } else if cmd_lower.contains("diablo iv.exe") {
                        found_uid = Some("fenris".to_string());
                    } else if cmd_lower.contains("overwatch.exe") {
                        found_uid = Some("pro".to_string());
                    } else if cmd_lower.contains("warcraft iii.exe") {
                        found_uid = Some("w3".to_string());
                    } else if cmd_lower.contains("sc2_x64.exe") || cmd_lower.contains("starcraft ii.exe") || cmd_lower.contains("sc2switcher") {
                        found_uid = Some("sc2".to_string());
                    } else if cmd_lower.contains("hearthstone.exe") {
                        found_uid = Some("hs".to_string());
                    } else if cmd_lower.contains("heroesofthestorm") || cmd_lower.contains("heroesswitcher") {
                        found_uid = Some("hero".to_string());
                    } else if cmd_lower.contains("d2r.exe") {
                        found_uid = Some("osi".to_string());
                    } else if cmd_lower.contains("diablo iii64.exe") || cmd_lower.contains("diablo iii.exe") {
                        found_uid = Some("diablo3".to_string());
                    } else if cmd_lower.contains("starcraft.exe") {
                        found_uid = Some("s1".to_string());
                    } else if cmd_lower.contains("modernwarfare.exe") || cmd_lower.contains("cod.exe") {
                        found_uid = Some("odin".to_string());
                    } else if cmd_lower.contains("blizzard arcade collection.exe") {
                        found_uid = Some("rtro".to_string());
                    }
                }

                if let Some(uid) = found_uid {
                    detected.insert(uid, pid);
                }
            }
        }
    }

    detected
}

/// Helper to check if a binary exists in the system PATH
fn binary_exists_in_path(bin_name: &str) -> bool {
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in path_var.split(':') {
            if Path::new(dir).join(bin_name).exists() {
                return true;
            }
        }
    }
    false
}

/// Resolves the optimal Wine/Proton runner binary for the prefix
fn resolve_battlenet_runner(prefix: &Path, configured_runner: &str) -> String {
    if !configured_runner.is_empty() && configured_runner != "wine" {
        return configured_runner.to_string();
    }
    // 1. Check config_info in prefix for Proton path
    let config_info = prefix.join("config_info");
    if config_info.exists() {
        if let Ok(content) = std::fs::read_to_string(&config_info) {
            for line in content.lines() {
                if line.contains("compatibilitytools.d") || line.contains("runners/wine") {
                    if let Some(pos) = line.find("compatibilitytools.d/") {
                        let sub = &line[pos + "compatibilitytools.d/".len()..];
                        if let Some(slash) = sub.find('/') {
                            let tool_name = &sub[..slash];
                            if let Some(home) = dirs::home_dir() {
                                let wine_bin = home
                                    .join(".local/share/Steam/compatibilitytools.d")
                                    .join(tool_name)
                                    .join("files/bin/wine");
                                if wine_bin.exists() {
                                    return wine_bin.to_string_lossy().to_string();
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    // 2. Check installed GE-Proton in Steam compatibilitytools
    if let Some(home) = dirs::home_dir() {
        let compat = home.join(".local/share/Steam/compatibilitytools.d");
        if compat.exists() {
            if let Ok(entries) = std::fs::read_dir(compat) {
                let mut ge_dirs: Vec<PathBuf> = entries
                    .filter_map(|e| e.ok())
                    .map(|e| e.path())
                    .filter(|p| {
                        p.file_name()
                            .map(|n| n.to_string_lossy().starts_with("GE-Proton"))
                            .unwrap_or(false)
                    })
                    .collect();
                ge_dirs.sort();
                ge_dirs.reverse();
                for ge in ge_dirs {
                    let wine_bin = ge.join("files").join("bin").join("wine");
                    if wine_bin.exists() {
                        return wine_bin.to_string_lossy().to_string();
                    }
                }
            }
        }
    }
    "wine".to_string()
}

/// Resolves the actual Battle.net executable inside the Wine prefix or custom path
fn resolve_battlenet_executable(cfg: &BattleNetConfigRecord) -> Result<PathBuf, String> {
    let direct = PathBuf::from(&cfg.battlenet_exe_path);
    if direct.exists() {
        return Ok(direct);
    }

    let prefix = PathBuf::from(&cfg.wine_prefix_path);
    let candidates = [
        prefix.join("drive_c").join("Program Files (x86)").join("Battle.net").join("Battle.net.exe"),
        prefix.join("drive_c").join("Program Files (x86)").join("Battle.net").join("Battle.net Launcher.exe"),
        prefix.join("drive_c").join("Program Files").join("Battle.net").join("Battle.net.exe"),
        prefix.join("drive_c").join("Battle.net").join("Battle.net.exe"),
    ];

    for candidate in &candidates {
        if candidate.exists() {
            return Ok(candidate.clone());
        }
    }

    Err(format!(
        "Le client Battle.net (Battle.net.exe) est introuvable. Emplacements vérifiés : '{:?}'. Veuillez installer Battle.net dans le préfixe '{}' ou corriger la configuration.",
        candidates, cfg.wine_prefix_path
    ))
}

pub fn is_battlenet_running() -> bool {
    if let Ok(output) = std::process::Command::new("pgrep")
        .arg("-f")
        .arg("Battle.net.exe")
        .output()
    {
        return output.status.success() && !output.stdout.is_empty();
    }
    false
}

pub fn ensure_battlenet_minimized_config(prefix: &Path) {
    let users_dir = prefix.join("drive_c/users");
    if let Ok(entries) = std::fs::read_dir(users_dir) {
        for entry in entries.flatten() {
            let cfg_path = entry.path().join("AppData/Roaming/Battle.net/Battle.net.config");
            if cfg_path.exists() {
                if let Ok(content) = std::fs::read_to_string(&cfg_path) {
                    if let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(root) = json.as_object_mut() {
                            let client = root
                                .entry("Client")
                                .or_insert_with(|| serde_json::json!({}));
                            if let Some(obj) = client.as_object_mut() {
                                obj.insert("AutoStartMinimized".to_string(), serde_json::json!("true"));
                                obj.insert("MinimizedOnStartup".to_string(), serde_json::json!("true"));
                                obj.insert("GameLaunchWindowBehavior".to_string(), serde_json::json!("2"));
                                obj.insert("HideOnClose".to_string(), serde_json::json!("true"));
                                obj.insert("RestoreWindowOnGameEnd".to_string(), serde_json::json!("false"));
                            }
                            if let Ok(updated) = serde_json::to_string_pretty(&json) {
                                let _ = std::fs::write(&cfg_path, updated);
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Launches a Battle.net game delegated to the Battle.net launcher client inside the Wine prefix.
/// Formula: WINEPREFIX="<path>" [gamemoderun] [mangohud] <runner_bin> "<path_to_Battle.net.exe>" --exec="launch <game_uid>"
pub async fn launch_battlenet_game(
    _state: &BattleNetLauncherState,
    product_uid: &str,
) -> Result<(), String> {
    let conn = db::open_db().map_err(|e| e.to_string())?;
    let config = db::get_battlenet_config(&conn).map_err(|e| e.to_string())?;

    let prefix_path = PathBuf::from(&config.wine_prefix_path);
    if !prefix_path.exists() {
        return Err(format!(
            "Le préfixe Wine Battle.net n'est pas initialisé à l'emplacement '{}'. Veuillez créer le préfixe ou installer Battle.net.",
            config.wine_prefix_path
        ));
    }

    let bnet_exe = resolve_battlenet_executable(&config)?;
    let runner_bin = resolve_battlenet_runner(&prefix_path, &config.runner_executable);

    println!(
        "[Loom::BattleNet] Lancement du jeu '{}' via Battle.net dans le préfixe '{}'",
        product_uid, config.wine_prefix_path
    );

    // Build command with wrappers
    let has_gamemode = config.enable_gamemode && binary_exists_in_path("gamemoderun");
    let has_mangohud = config.enable_mangohud && binary_exists_in_path("mangohud");

    let mut cmd_args: Vec<String> = Vec::new();
    let root_binary: String;

    if has_gamemode {
        root_binary = "gamemoderun".to_string();
        if has_mangohud {
            cmd_args.push("mangohud".to_string());
        }
        cmd_args.push(runner_bin);
    } else if has_mangohud {
        root_binary = "mangohud".to_string();
        cmd_args.push(runner_bin);
    } else {
        root_binary = runner_bin;
    }

    cmd_args.push(bnet_exe.to_string_lossy().to_string());

    ensure_battlenet_minimized_config(&prefix_path);
    let is_running = is_battlenet_running();
    if !is_running {
        cmd_args.push("--autostarted".to_string());
    }

    let bnet_launch_uid = match product_uid {
        "wow_forever" => "wowf",
        other => other,
    };
    cmd_args.push(format!("--exec=launch {}", bnet_launch_uid));

    let mut command = Command::new(&root_binary);
    command.args(&cmd_args);

    // Set standard environment variables (silence noisy Wine/VKD3D fixmes and avoid forced experimental DXR)
    command.env("WINEPREFIX", &config.wine_prefix_path);
    command.env("WINEARCH", "win64");
    command.env("DXVK_ASYNC", "1");
    command.env("WINEDEBUG", "-all");
    command.env("VKD3D_DEBUG", "none");

    // Apply custom user-defined env vars
    if let Some(ref custom_json) = config.custom_env_vars {
        if let Ok(parsed) = serde_json::from_str::<HashMap<String, String>>(custom_json) {
            for (k, v) in parsed {
                command.env(k, v);
            }
        }
    }

    // Redirect stdout/stderr to a dedicated log file to avoid terminal spam and thread assertion leaks
    let log_dir = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string())).join(".local/share"))
        .join("loom")
        .join("logs");
    let _ = std::fs::create_dir_all(&log_dir);
    let log_file_path = log_dir.join("battlenet.log");

    if let Ok(file) = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&log_file_path)
    {
        if let Ok(file_err) = file.try_clone() {
            command.stdout(std::process::Stdio::from(file));
            command.stderr(std::process::Stdio::from(file_err));
        } else {
            command.stdout(std::process::Stdio::null());
            command.stderr(std::process::Stdio::null());
        }
    } else {
        command.stdout(std::process::Stdio::null());
        command.stderr(std::process::Stdio::null());
    }

    // Spawn process asynchronously
    let child = command
        .spawn()
        .map_err(|e| format!("Échec du lancement du processus Wine Battle.net: {}", e))?;

    if let Some(pid) = child.id() {
        println!("[Loom::BattleNet] Commande Battle.net transmise (PID runner: {})", pid);
        // Do NOT register runner PID in state.running_pids: the actual game process
        // is tracked by scan_running_battlenet_processes when the game executable starts.
    }

    // Update last_played timestamp in database
    let now_ts = chrono::Utc::now().timestamp();
    let _ = db::update_battlenet_game_last_played(&conn, product_uid, now_ts);

    Ok(())
}

/// Force-kills the Battle.net game cleanly without killing the whole Wine prefix unless no game is found.
pub async fn kill_battlenet_game(
    state: &BattleNetLauncherState,
    product_uid: &str,
) -> Result<(), String> {
    println!("[Loom::BattleNet] Arrêt ciblé du jeu '{}'", product_uid);

    // 1. Terminate tracked game PID directly
    let pid_opt = {
        let scanned = scan_running_battlenet_processes();
        scanned.get(product_uid).copied()
    };

    if let Some(pid) = pid_opt {
        // Send SIGTERM
        let _ = Command::new("kill").arg("-15").arg(pid.to_string()).status().await;
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        // Send SIGKILL if still present
        if Path::new(&format!("/proc/{}", pid)).exists() {
            let _ = Command::new("kill").arg("-9").arg(pid.to_string()).status().await;
        }
        let mut map = state.running_pids.write().await;
        map.remove(product_uid);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_running_battlenet_processes() {
        let scanned = scan_running_battlenet_processes();
        println!("Currently running Battle.net processes: {:?}", scanned);
        if let Some(&pid) = scanned.get("wowf") {
            assert!(pid > 0);
        }
    }
}
