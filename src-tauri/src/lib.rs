use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct LaunchResult {
    pub success: bool,
    pub pid: Option<u32>,
    pub message: String,
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

#[tauri::command]
fn launch_game(game_id: String, runner: Option<String>) -> Result<LaunchResult, String> {
    println!("Launching game {} with runner {:?}", game_id, runner);
    Ok(LaunchResult {
        success: true,
        pid: Some(1337),
        message: format!("Jeu {} démarré avec succès", game_id),
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
        .invoke_handler(tauri::generate_handler![launch_game, get_system_stats])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
