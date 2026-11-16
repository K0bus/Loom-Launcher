use tauri::State;

use super::collector::SharedCollector;
use super::models::{
    DnfUpdateStatus, GpuProcessInfo, ProcessSnapshot, SmartDiskInfo, SystemMetricsSnapshot,
    SystemSummary,
};

#[tauri::command]
pub async fn get_system_static_info(
    collector: State<'_, SharedCollector>,
) -> Result<SystemSummary, String> {
    let collector = collector.inner().clone();
    tokio::task::spawn_blocking(move || {
        let mut guard = collector
            .lock()
            .map_err(|e| format!("Impossible de verrouiller le collecteur: {}", e))?;
        Ok(guard.get_system_summary())
    })
    .await
    .map_err(|e| format!("Erreur tâche asynchrone: {}", e))?
}

#[tauri::command]
pub async fn get_system_metrics(
    collector: State<'_, SharedCollector>,
) -> Result<SystemMetricsSnapshot, String> {
    let collector = collector.inner().clone();
    tokio::task::spawn_blocking(move || {
        let mut guard = collector
            .lock()
            .map_err(|e| format!("Impossible de verrouiller le collecteur: {}", e))?;
        Ok(guard.collect_snapshot())
    })
    .await
    .map_err(|e| format!("Erreur tâche asynchrone: {}", e))?
}

#[tauri::command]
pub fn kill_process(pid: u32, force: bool) -> Result<(), String> {
    if pid <= 1 {
        return Err("Action interdite: impossible de terminer les processus système critiques (PID <= 1).".into());
    }

    let own_pid = std::process::id();
    if pid == own_pid {
        return Err("Action interdite: impossible de terminer le processus de l'application hôte Loom.".into());
    }

    #[cfg(unix)]
    {
        let sig = if force { libc::SIGKILL } else { libc::SIGTERM };
        let ret = unsafe { libc::kill(pid as libc::pid_t, sig) };
        if ret == 0 {
            Ok(())
        } else {
            let err = std::io::Error::last_os_error();
            Err(format!(
                "Échec de l'arrêt du processus {} (code {}): {}",
                pid, ret, err
            ))
        }
    }

    #[cfg(not(unix))]
    {
        Err("L'arrêt direct des processus n'est pas supporté sur ce système.".into())
    }
}

#[tauri::command]
pub async fn get_gpu_processes(
    collector: State<'_, SharedCollector>,
) -> Result<Vec<GpuProcessInfo>, String> {
    let collector = collector.inner().clone();
    tokio::task::spawn_blocking(move || {
        let mut guard = collector
            .lock()
            .map_err(|e| format!("Impossible de verrouiller le collecteur: {}", e))?;
        Ok(guard.collect_gpu_processes())
    })
    .await
    .map_err(|e| format!("Erreur tâche asynchrone: {}", e))?
}

#[tauri::command]
pub async fn get_all_system_processes(
    collector: State<'_, SharedCollector>,
) -> Result<Vec<ProcessSnapshot>, String> {
    let collector = collector.inner().clone();
    tokio::task::spawn_blocking(move || {
        let mut guard = collector
            .lock()
            .map_err(|e| format!("Impossible de verrouiller le collecteur: {}", e))?;
        Ok(guard.collect_all_processes())
    })
    .await
    .map_err(|e| format!("Erreur tâche asynchrone: {}", e))?
}

#[tauri::command]
pub async fn get_smart_disks_info(
    collector: State<'_, SharedCollector>,
    elevated: Option<bool>,
) -> Result<Vec<SmartDiskInfo>, String> {
    let collector = collector.inner().clone();
    let is_elevated = elevated.unwrap_or(false);
    tokio::task::spawn_blocking(move || {
        let guard = collector
            .lock()
            .map_err(|e| format!("Impossible de verrouiller le collecteur: {}", e))?;
        Ok(guard.collect_smart_disks(is_elevated))
    })
    .await
    .map_err(|e| format!("Erreur tâche asynchrone: {}", e))?
}

#[tauri::command]
pub async fn check_dnf_updates(_refresh: bool) -> Result<DnfUpdateStatus, String> {
    tokio::task::spawn_blocking(move || {
        Ok(super::collector::SystemMetricsCollector::collect_dnf_updates())
    })
    .await
    .map_err(|e| format!("Erreur vérification DNF: {}", e))?
}

