use tauri::State;

use super::collector::SharedCollector;
use super::models::{SystemMetricsSnapshot, SystemSummary};

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
