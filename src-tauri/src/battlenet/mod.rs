pub mod auth;
pub mod detector;
pub mod launcher;
pub mod wow;

use std::path::PathBuf;
use tauri::State;

use crate::db::{
    self, BattleNetAccountRecord as BattleNetAccount, BattleNetConfigRecord as BattleNetConfig,
    BattleNetGameRecord as BattleNetGame,
};
pub use auth::BlizzardGameEntitlement;
pub use detector::BattleNetGameMetadata;
pub use launcher::BattleNetLauncherState;
pub use wow::{InstallAddonResult, InstalledWowAddonRecord, WowCharacterRecord, WowInstanceRecord};

// ==========================================
// TAURI V2 IPC COMMANDS FOR BATTLE.NET
// ==========================================

#[tauri::command]
pub async fn scan_battlenet_games() -> Result<Vec<BattleNetGame>, String> {
    let conn = db::open_db().map_err(|e| e.to_string())?;
    let config = db::get_battlenet_config(&conn).map_err(|e| e.to_string())?;
    let prefix = PathBuf::from(&config.wine_prefix_path);

    detector::scan_prefix_and_external_games(&prefix, &conn)
}

#[tauri::command]
pub async fn add_external_battlenet_game(path: String) -> Result<BattleNetGame, String> {
    let dir_path = PathBuf::from(&path);
    let meta = detector::inspect_game_directory(&dir_path)?;

    let conn = db::open_db().map_err(|e| e.to_string())?;
    let last_played = detector::detect_directory_last_modified(&dir_path);
    let record = BattleNetGame {
        product_uid: meta.product_uid,
        name: meta.name,
        install_path: meta.install_path,
        executable_relative_path: meta.executable_relative_path,
        version_build: meta.version_build,
        is_external: true,
        is_installed: true,
        last_played,
        playtime_seconds: 0,
    };

    db::insert_or_replace_battlenet_game(&conn, &record).map_err(|e| e.to_string())?;

    Ok(record)
}

#[tauri::command]
pub async fn launch_battlenet_game(
    state: State<'_, BattleNetLauncherState>,
    product_uid: String,
) -> Result<(), String> {
    launcher::launch_battlenet_game(&state, &product_uid).await
}

#[tauri::command]
pub async fn kill_battlenet_game(
    state: State<'_, BattleNetLauncherState>,
    product_uid: String,
) -> Result<(), String> {
    launcher::kill_battlenet_game(&state, &product_uid).await
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct BlizzardCredentialsStatus {
    pub configured: bool,
    pub client_id: Option<String>,
}

#[tauri::command]
pub fn save_blizzard_credentials(client_id: String, client_secret: String) -> Result<(), String> {
    auth::save_blizzard_credentials(&client_id, &client_secret)
}

#[tauri::command]
pub fn get_blizzard_credentials_status() -> Result<BlizzardCredentialsStatus, String> {
    let (configured, client_id) = auth::get_blizzard_credentials_status();
    Ok(BlizzardCredentialsStatus {
        configured,
        client_id,
    })
}

#[tauri::command]
pub async fn start_blizzard_oauth_flow() -> Result<BattleNetAccount, String> {
    auth::start_blizzard_oauth_flow().await
}

#[tauri::command]
pub async fn get_blizzard_account_info() -> Result<Option<BattleNetAccount>, String> {
    auth::ensure_valid_blizzard_account().await
}

#[tauri::command]
pub fn disconnect_blizzard_account() -> Result<(), String> {
    auth::disconnect_blizzard_account()
}

#[tauri::command]
pub async fn get_blizzard_owned_games() -> Result<Vec<BlizzardGameEntitlement>, String> {
    auth::get_blizzard_owned_games().await
}

#[tauri::command]
pub fn get_battlenet_config() -> Result<BattleNetConfig, String> {
    let conn = db::open_db().map_err(|e| e.to_string())?;
    db::get_battlenet_config(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_battlenet_config(config: BattleNetConfig) -> Result<(), String> {
    let conn = db::open_db().map_err(|e| e.to_string())?;
    db::save_battlenet_config(&conn, &config).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_running_battlenet_games(
    state: State<'_, BattleNetLauncherState>,
) -> Result<Vec<String>, String> {
    Ok(state.get_running_games().await)
}

#[tauri::command]
pub fn scan_wow_instances() -> Result<Vec<WowInstanceRecord>, String> {
    Ok(wow::scan_wow_instances_impl())
}

#[tauri::command]
pub async fn search_wow_addons(
    query: Option<String>,
    category_id: Option<u32>,
    game_version_type_id: Option<i64>,
    sort_field: Option<u32>,
    page_size: Option<u32>,
) -> Result<serde_json::Value, String> {
    wow::search_wow_addons_impl(query, category_id, game_version_type_id, sort_field, page_size).await
}

#[tauri::command]
pub async fn get_wow_addon_details(
    mod_id: i64,
) -> Result<serde_json::Value, String> {
    wow::get_wow_addon_details_impl(mod_id).await
}

#[tauri::command]
pub async fn get_wow_addon_description(
    mod_id: i64,
) -> Result<serde_json::Value, String> {
    wow::get_wow_addon_description_impl(mod_id).await
}

#[tauri::command]
pub async fn get_wow_addon_files(
    mod_id: i64,
    page_size: Option<u32>,
) -> Result<serde_json::Value, String> {
    wow::get_wow_addon_files_impl(mod_id, page_size).await
}

#[tauri::command]
pub async fn get_wow_addons(
    instance_path: String,
    check_updates: bool,
    game_version_type_id: Option<i64>,
) -> Result<Vec<InstalledWowAddonRecord>, String> {
    wow::get_wow_addons_impl(instance_path, check_updates, game_version_type_id).await
}

#[tauri::command]
pub async fn install_wow_addon(
    instance_path: String,
    download_url: String,
    mod_id: Option<i64>,
    file_id: Option<i64>,
    mod_name: Option<String>,
    version: Option<String>,
    icon_url: Option<String>,
) -> Result<InstallAddonResult, String> {
    wow::install_wow_addon_impl(instance_path, download_url, mod_id, file_id, mod_name, version, icon_url).await
}

#[tauri::command]
pub fn uninstall_wow_addon(
    instance_path: String,
    folder_name: String,
    secondary_folders: Vec<String>,
) -> Result<(), String> {
    wow::uninstall_wow_addon_impl(instance_path, folder_name, secondary_folders)
}

#[tauri::command]
pub fn toggle_wow_addon(
    instance_path: String,
    folder_name: String,
    secondary_folders: Vec<String>,
    enabled: bool,
) -> Result<bool, String> {
    wow::toggle_wow_addon_impl(instance_path, folder_name, secondary_folders, enabled)
}

#[tauri::command]
pub fn open_wow_addons_folder(instance_path: String) -> Result<(), String> {
    wow::open_wow_addons_folder_impl(instance_path)
}

#[tauri::command]
pub fn export_wow_addons(instance_path: String, flavor: Option<String>) -> Result<String, String> {
    wow::export_wow_addons_impl(instance_path, flavor)
}

#[tauri::command]
pub fn get_wow_characters(instance_path: String) -> Result<Vec<WowCharacterRecord>, String> {
    Ok(wow::get_wow_characters_impl(&instance_path))
}

#[tauri::command]
pub fn update_battlenet_game_playtime(
    product_uid: String,
    delta_seconds: u64,
    last_played: Option<i64>,
) -> Result<(), String> {
    let conn = db::open_db().map_err(|e| e.to_string())?;
    db::update_battlenet_game_playtime(&conn, &product_uid, delta_seconds, last_played).map_err(|e| e.to_string())
}

