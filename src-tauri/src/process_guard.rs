use std::path::PathBuf;
#[cfg(not(debug_assertions))]
use std::time::SystemTime;

/// Scans the Linux `/proc` filesystem at startup to detect and clean up:
/// 1. Ghost processes: zombies, stopped processes, processes with deleted binaries.
/// 2. Outdated processes: running instances that were started from an older AppImage
///    or binary that has since been modified/rebuilt on disk.
#[cfg(target_os = "linux")]
pub fn cleanup_outdated_or_ghost_instances() {
    let my_pid = std::process::id();
    let current_exe = std::fs::read_link("/proc/self/exe").ok();
    let current_appimage = std::env::var("APPIMAGE").ok().map(PathBuf::from);
    let current_appdir = std::env::var("APPDIR").ok().map(PathBuf::from);

    // Timestamp of our AppImage on disk
    let appimage_disk_mtime = current_appimage.as_ref().and_then(|p| {
        std::fs::metadata(p).ok().and_then(|m| m.modified().ok())
    });

    // Timestamp of our executable binary
    let current_exe_mtime = current_exe.as_ref().and_then(|p| {
        std::fs::metadata(p).ok().and_then(|m| m.modified().ok())
    });

    let proc_entries = match std::fs::read_dir("/proc") {
        Ok(entries) => entries,
        Err(_) => return,
    };

    let mut to_terminate: Vec<(u32, Option<u32>, String)> = Vec::new();

    for entry in proc_entries.flatten() {
        let name = entry.file_name();
        let pid_str = name.to_string_lossy();
        let pid: u32 = match pid_str.parse() {
            Ok(p) => p,
            Err(_) => continue,
        };

        if pid == my_pid {
            continue;
        }

        let proc_path = entry.path();

        // Retrieve PPID if available
        let mut ppid: Option<u32> = None;
        let mut is_zombie_or_stopped = false;
        if let Ok(status) = std::fs::read_to_string(proc_path.join("status")) {
            for line in status.lines() {
                if line.starts_with("PPid:") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        ppid = parts[1].parse::<u32>().ok();
                    }
                } else if line.starts_with("State:") {
                    if line.contains('Z') || line.contains('T') {
                        is_zombie_or_stopped = true;
                    }
                }
            }
        }

        // Read target executable path
        let exe_link = match std::fs::read_link(proc_path.join("exe")) {
            Ok(l) => l,
            Err(_) => {
                // If the exe link cannot be read, check comm
                if let Ok(comm) = std::fs::read_to_string(proc_path.join("comm")) {
                    let comm_clean = comm.trim();
                    if comm_clean == "loom-launcher" || comm_clean == "loom_launcher" {
                        to_terminate.push((pid, ppid, "Processus fantôme avec exécutable inaccessible".to_string()));
                    }
                }
                continue;
            }
        };

        let exe_str = exe_link.to_string_lossy();

        // Filter out non-loom processes and WebKit helper processes
        if exe_str.contains("WebKitWebProcess") || exe_str.contains("WebKitNetworkProcess") {
            continue;
        }

        let is_loom_launcher = exe_str.ends_with("/loom-launcher") || exe_str == "loom-launcher";
        let is_loom_appimage = (exe_str.contains("loom") || exe_str.contains("Loom")) && exe_str.ends_with(".appimage");

        if !is_loom_launcher && !is_loom_appimage {
            continue;
        }

        // Process start time (directory modification time)
        let proc_start_time = std::fs::metadata(&proc_path)
            .ok()
            .and_then(|m| m.modified().ok());

        let mut reason: Option<String> = None;

        // 1. Ghost / Deleted binary
        if exe_str.contains("(deleted)") {
            reason = Some("Binaire exécutable marqué comme supprimé (deleted)".to_string());
        } else if is_zombie_or_stopped {
            reason = Some("Processus en état zombie ou stoppé".to_string());
        }

        // 2. Check AppImage obsolescence (disk file updated after running process started)
        if reason.is_none() {
            if let (Some(disk_mtime), Some(start_time)) = (appimage_disk_mtime, proc_start_time) {
                // If disk file was modified after other process started, other process is outdated
                if disk_mtime > start_time {
                    reason = Some(format!(
                        "AppImage sur disque mise à jour ({:?}) après le démarrage du processus ({:?})",
                        disk_mtime, start_time
                    ));
                }
            }
        }

        // 3. Check AppImage mount mismatch
        if reason.is_none() && current_appdir.is_some() {
            if let Ok(env_bytes) = std::fs::read(proc_path.join("environ")) {
                let env_str = String::from_utf8_lossy(&env_bytes);
                for var in env_str.split('\0') {
                    if var.starts_with("APPDIR=") {
                        let other_appdir = PathBuf::from(&var[7..]);
                        if let Some(ref my_appdir) = current_appdir {
                            if &other_appdir != my_appdir {
                                reason = Some(format!(
                                    "Exécution sur un ancien point de montage AppImage ({:?} != {:?})",
                                    other_appdir, my_appdir
                                ));
                            }
                        }
                        break;
                    }
                }
            }
        }

        // 4. Binary mtime check (for standalone binaries outside AppImage)
        if reason.is_none() {
            if let (Some(my_mtime), Ok(other_meta)) = (current_exe_mtime, std::fs::metadata(&exe_link)) {
                if let Ok(other_mtime) = other_meta.modified() {
                    if my_mtime > other_mtime {
                        reason = Some("Binaire du processus existant plus ancien que le nouveau binaire".to_string());
                    }
                }
            }
        }

        // 5. Explicit CLI flag: --replace or --restart
        if reason.is_none() && std::env::args().any(|a| a == "--replace" || a == "--kill-old" || a == "--restart") {
            reason = Some("Remplacement forcé demandé par argument CLI".to_string());
        }

        if let Some(r) = reason {
            to_terminate.push((pid, ppid, r));
        }
    }

    if to_terminate.is_empty() {
        return;
    }

    eprintln!("[ProcessGuard] {} processus fantôme(s) ou obsolète(s) détecté(s) :", to_terminate.len());
    for (pid, ppid, ref r) in &to_terminate {
        eprintln!("[ProcessGuard] -> Arrêt du PID {} ({})", pid, r);
        terminate_process(*pid);
        if let Some(parent) = ppid {
            if *parent > 1 && *parent != my_pid {
                // If parent is an AppImage wrapper, also terminate it
                if let Ok(parent_exe) = std::fs::read_link(format!("/proc/{}/exe", parent)) {
                    let parent_str = parent_exe.to_string_lossy();
                    if parent_str.contains("loom") || parent_str.ends_with(".appimage") {
                        eprintln!("[ProcessGuard] -> Arrêt du processus parent wrapper PID {}", parent);
                        terminate_process(*parent);
                    }
                }
            }
        }
    }

    // Give D-Bus daemon 200ms to register disconnection and release com.loom.launcher.SingleInstance
    std::thread::sleep(std::time::Duration::from_millis(200));
}

#[cfg(not(target_os = "linux"))]
pub fn cleanup_outdated_or_ghost_instances() {}

/// Terminates a process gracefully with SIGTERM, falling back to SIGKILL
#[cfg(target_os = "linux")]
fn terminate_process(pid: u32) {
    let pid_i32 = pid as i32;

    // Send SIGTERM
    unsafe {
        libc::kill(pid_i32, libc::SIGTERM);
    }

    let proc_dir = PathBuf::from(format!("/proc/{}", pid));
    for _ in 0..12 {
        if !proc_dir.exists() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    // Force SIGKILL if still hanging
    if proc_dir.exists() {
        unsafe {
            libc::kill(pid_i32, libc::SIGKILL);
        }
        for _ in 0..6 {
            if !proc_dir.exists() {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }
}

/// Spawns a background task in the running app to monitor if its executable on disk
/// is replaced by a newer AppImage or binary.
pub fn spawn_outdated_watchdog(app: tauri::AppHandle) {
    #[cfg(target_os = "linux")]
    {
        use tauri::Emitter;

        #[cfg(not(debug_assertions))]
        let my_pid = std::process::id();
        #[cfg(not(debug_assertions))]
        let appimage_path = std::env::var("APPIMAGE").ok().map(PathBuf::from);
        #[cfg(not(debug_assertions))]
        let my_start_time = std::fs::metadata(format!("/proc/{}", my_pid))
            .ok()
            .and_then(|m| m.modified().ok())
            .unwrap_or_else(SystemTime::now);

        tauri::async_runtime::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(15));
            loop {
                interval.tick().await;

                // En mode debug (cargo run / tauri dev), ignorer la détection outdated pour éviter les faux positifs lors des recompilations
                #[cfg(debug_assertions)]
                let is_outdated = false;

                #[cfg(not(debug_assertions))]
                let is_outdated = if let Some(ref img_path) = appimage_path {
                    if let Ok(meta) = std::fs::metadata(img_path) {
                        if let Ok(mtime) = meta.modified() {
                            mtime > my_start_time + std::time::Duration::from_secs(3)
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else if let Ok(exe_path) = std::fs::read_link("/proc/self/exe") {
                    if let Ok(meta) = std::fs::metadata(&exe_path) {
                        if let Ok(mtime) = meta.modified() {
                            mtime > my_start_time + std::time::Duration::from_secs(3)
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    false
                };

                if is_outdated {
                    eprintln!("[ProcessGuard] Nouvelle version détectée sur disque ! Le processus actif est outdated.");
                    let _ = app.emit("loom:update-detected", ());
                    // Check again in 60s
                    tokio::time::sleep(std::time::Duration::from_secs(45)).await;
                }
            }
        });
    }
}

/// Relaunches the updated application and exits the current process cleanly
#[tauri::command]
pub fn restart_application() -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        let executable = if let Ok(appimage) = std::env::var("APPIMAGE") {
            PathBuf::from(appimage)
        } else if let Ok(exe) = std::fs::read_link("/proc/self/exe") {
            exe
        } else {
            return Err("Impossible de déterminer le chemin d'exécution".to_string());
        };

        if !executable.exists() {
            return Err(format!("L'exécutable n'existe pas : {:?}", executable));
        }

        let args: Vec<String> = std::env::args().skip(1).collect();
        let mut cmd = std::process::Command::new(&executable);
        cmd.args(&args);

        // Nettoyage explicite des variables d'environnement pour que la nouvelle instance reparte sur un état sain
        cmd.env_remove("LIBGL_ALWAYS_SOFTWARE");
        cmd.env_remove("GALLIUM_DRIVER");
        cmd.env_remove("WEBKIT_DISABLE_DMABUF_RENDERER");
        cmd.env_remove("WEBKIT_DISABLE_COMPOSITING_MODE");
        cmd.env_remove("WEBKIT_FORCE_COMPOSITING_MODE");

        match cmd.spawn() {
            Ok(_) => {
                eprintln!("[ProcessGuard] Redémarrage effectué avec succès vers {:?}", executable);
                std::process::exit(0);
            }
            Err(e) => Err(format!("Échec du redémarrage : {}", e)),
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        Err("Non supporté sur cette plateforme".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cleanup_outdated_or_ghost_instances_runs_cleanly() {
        // Must run safely and not panic
        cleanup_outdated_or_ghost_instances();
    }
}
