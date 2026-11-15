use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::db::{open_db, get_bethesda_plugins, get_profile_mods, get_profiles};
use crate::modding::adapter::{GameAdapter, GameModdingManifest, LaunchExecutable, ProviderKind};
use crate::modding::conflicts::scan_and_resolve_conflicts;
use crate::modding::fomod::find_fomod_dir;
use crate::modding::plugins::{format_plugins_txt, scan_plugins_in_directory, sync_bethesda_plugins, write_plugins_txt};
use crate::modding::symlink::create_atomic_symlink;

const SYMLINK_TRACKING_FILE: &str = ".loom_deployed_symlinks.json";

pub struct BethesdaAdapter {
    pub game_id: String,
    pub display_name: String,
    pub executable_name: String,
    pub script_extender_name: String,
    pub appdata_folder_name: String,
}

impl BethesdaAdapter {
    pub fn new_skyrim_se() -> Self {
        Self {
            game_id: "skyrim_se".to_string(),
            display_name: "The Elder Scrolls V: Skyrim Special Edition".to_string(),
            executable_name: "SkyrimSELauncher.exe".to_string(),
            script_extender_name: "skse64_loader.exe".to_string(),
            appdata_folder_name: "Skyrim Special Edition".to_string(),
        }
    }

    pub fn new_fallout_4() -> Self {
        Self {
            game_id: "fallout4".to_string(),
            display_name: "Fallout 4".to_string(),
            executable_name: "Fallout4Launcher.exe".to_string(),
            script_extender_name: "f4se_loader.exe".to_string(),
            appdata_folder_name: "Fallout4".to_string(),
        }
    }

    pub fn new_oblivion() -> Self {
        Self {
            game_id: "oblivion".to_string(),
            display_name: "The Elder Scrolls IV: Oblivion".to_string(),
            executable_name: "OblivionLauncher.exe".to_string(),
            script_extender_name: "obse_loader.exe".to_string(),
            appdata_folder_name: "Oblivion".to_string(),
        }
    }

    /// Resolves the Wine/Proton AppData directory for plugins.txt
    pub fn resolve_wine_appdata_dir(&self, deployment_dir: &Path) -> Option<PathBuf> {
        // Look up standard Proton compatdata hierarchy if inside steamapps
        let dep_str = deployment_dir.to_string_lossy();
        if let Some(idx) = dep_str.find("steamapps/common") {
            let steamapps_dir = PathBuf::from(&dep_str[..idx + "steamapps".len()]);
            // Search compatdata subfolders
            let compat_dir = steamapps_dir.join("compatdata");
            if compat_dir.is_dir() {
                if let Ok(entries) = fs::read_dir(&compat_dir) {
                    for entry in entries.flatten() {
                        let candidate = entry
                            .path()
                            .join("pfx/drive_c/users/steamuser/AppData/Local")
                            .join(&self.appdata_folder_name);
                        if candidate.exists() {
                            return Some(candidate);
                        }
                    }
                }
            }
        }

        // Generic Wine fallback
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let default_wine = home
            .join(".wine/drive_c/users")
            .join(std::env::var("USER").unwrap_or_else(|_| "steamuser".to_string()))
            .join("AppData/Local")
            .join(&self.appdata_folder_name);

        if default_wine.exists() {
            Some(default_wine)
        } else {
            None
        }
    }
}

impl GameAdapter for BethesdaAdapter {
    fn manifest(&self) -> GameModdingManifest {
        GameModdingManifest::new(
            &self.game_id,
            &self.display_name,
            vec![
                ProviderKind::NexusMods,
                ProviderKind::CurseForge,
                ProviderKind::LocalArchive,
            ],
            true, // allows_load_order
            true, // allows_conflict_override
            false, // supports_modpacks
            vec![
                LaunchExecutable::new(
                    "skse",
                    format!("{} (Script Extender)", self.script_extender_name),
                    &self.script_extender_name,
                    true,
                    true,
                ),
                LaunchExecutable::new(
                    "launcher",
                    format!("{} (Vanilla)", self.executable_name),
                    &self.executable_name,
                    false,
                    true,
                ),
            ],
        )
    }

    fn get_staging_directory(&self, profile_id: &str) -> PathBuf {
        let base = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string())).join(".local/share"))
            .join("loom")
            .join("profiles")
            .join(&self.game_id)
            .join(profile_id)
            .join("staged");
        let _ = fs::create_dir_all(&base);
        base
    }

    fn get_deployment_directory(&self, profile_id: &str) -> PathBuf {
        if let Ok(conn) = open_db() {
            if let Ok(profiles) = get_profiles(&conn, Some(&self.game_id)) {
                if let Some(prof) = profiles.into_iter().find(|p| p.id == profile_id) {
                    if let Some(dep_path) = prof.deployment_path {
                        let path = PathBuf::from(&dep_path);
                        if path.exists() {
                            let data_dir = path.join("Data");
                            let _ = fs::create_dir_all(&data_dir);
                            return data_dir;
                        }
                    }
                }
            }
        }

        // Fallback: Local profile Data directory
        let base = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string())).join(".local/share"))
            .join("loom")
            .join("profiles")
            .join(&self.game_id)
            .join(profile_id)
            .join("Data");
        let _ = fs::create_dir_all(&base);
        base
    }

    fn normalize_staged_mod(&self, mod_staged_path: &PathBuf) -> Result<(), String> {
        if !mod_staged_path.is_dir() {
            return Err(format!("Staged mod path is not a directory: {}", mod_staged_path.display()));
        }

        // 1. Check for FOMOD
        let _ = find_fomod_dir(mod_staged_path);

        // 2. Flatten root if it contains a single "Data" subfolder
        let mut top_entries = Vec::new();
        if let Ok(entries) = fs::read_dir(mod_staged_path) {
            for entry in entries.flatten() {
                top_entries.push(entry.path());
            }
        }

        if top_entries.len() == 1 {
            let single = &top_entries[0];
            if single.is_dir() {
                let name = single.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
                if name == "data" {
                    // Move all children of Data/ to mod_staged_path/
                    if let Ok(children) = fs::read_dir(single) {
                        for child in children.flatten() {
                            let target = mod_staged_path.join(child.file_name());
                            let _ = fs::rename(child.path(), target);
                        }
                    }
                    let _ = fs::remove_dir_all(single);
                }
            }
        }

        Ok(())
    }

    fn deploy_profile(&self, profile_id: &str) -> Result<(), String> {
        let staging_dir = self.get_staging_directory(profile_id);
        let deployment_data_dir = self.get_deployment_directory(profile_id);
        let game_root_dir = deployment_data_dir.parent().unwrap_or(&deployment_data_dir);

        // 1. Undeploy previous symlinks cleanly
        self.undeploy_profile(profile_id)?;

        let conn = open_db().map_err(|e| e.to_string())?;
        let mods = get_profile_mods(&conn, profile_id).map_err(|e| e.to_string())?;

        // 2. Scan and resolve file conflicts
        let conflicts = scan_and_resolve_conflicts(profile_id, &staging_dir)?;
        let conflict_winner_map: HashMap<String, String> = conflicts
            .into_iter()
            .map(|c| (c.relative_path, c.current_winner_mod_id))
            .collect();

        // 3. Collect active enabled mods
        let mut enabled_mods: Vec<_> = mods.into_iter().filter(|m| m.is_enabled).collect();
        // Lower priority first so higher priority files can override if not in conflict map
        enabled_mods.sort_by_key(|m| m.load_priority);

        let mut deployed_links: Vec<String> = Vec::new();
        let mut all_discovered_plugins: HashSet<String> = HashSet::new();

        for pmod in &enabled_mods {
            let mod_staged = if let Some(ref sp) = pmod.staged_path {
                PathBuf::from(sp)
            } else {
                staging_dir.join(&pmod.id)
            };

            if !mod_staged.is_dir() {
                continue;
            }

            // Normalisation on-the-fly if needed
            let _ = self.normalize_staged_mod(&mod_staged);

            // Scan files in this staged mod
            let rel_files = crate::modding::conflicts::collect_relative_files(&mod_staged)?;
            for rel in rel_files {
                // If there's a conflict rule/resolution, only deploy if this mod is the winner
                if let Some(winner_id) = conflict_winner_map.get(&rel) {
                    if winner_id != &pmod.id {
                        continue;
                    }
                }

                let src_path = mod_staged.join(&rel);
                let is_root_builder = is_root_target_file(&rel);
                let target_path = if is_root_builder {
                    game_root_dir.join(&rel)
                } else {
                    deployment_data_dir.join(&rel)
                };

                // Track and create symlink
                if let Ok(()) = create_atomic_symlink(&src_path, &target_path) {
                    deployed_links.push(target_path.to_string_lossy().to_string());
                }
            }

            // Detect plugins (.esm, .esl, .esp) in this mod
            let plugins = scan_plugins_in_directory(&mod_staged);
            for p in &plugins {
                all_discovered_plugins.insert(p.clone());
            }
            let _ = sync_bethesda_plugins(profile_id, &plugins, Some(&pmod.id));
        }

        // 4. Save tracking manifest for safe undeploy
        let tracking_file = deployment_data_dir.join(SYMLINK_TRACKING_FILE);
        let json = serde_json::to_string_pretty(&deployed_links).map_err(|e| e.to_string())?;
        let _ = fs::write(tracking_file, json);

        // 5. Format and sync plugins.txt
        let plugins_records = get_bethesda_plugins(&conn, profile_id).unwrap_or_default();
        let plugins_content = format_plugins_txt(&plugins_records);
        let profile_base = staging_dir.parent().unwrap_or(&staging_dir);
        let wine_appdata = self.resolve_wine_appdata_dir(&deployment_data_dir);
        let _ = write_plugins_txt(profile_base, wine_appdata.as_deref(), &plugins_content);

        Ok(())
    }

    fn undeploy_profile(&self, profile_id: &str) -> Result<(), String> {
        let deployment_data_dir = self.get_deployment_directory(profile_id);
        let tracking_file = deployment_data_dir.join(SYMLINK_TRACKING_FILE);

        if tracking_file.exists() {
            if let Ok(content) = fs::read_to_string(&tracking_file) {
                if let Ok(links) = serde_json::from_str::<Vec<String>>(&content) {
                    for link_str in links {
                        let link_path = PathBuf::from(link_str);
                        if link_path.is_symlink() {
                            let _ = fs::remove_file(link_path);
                        }
                    }
                }
            }
            let _ = fs::remove_file(tracking_file);
        }

        Ok(())
    }
}

/// Identifies files that target the root game directory instead of Data/
fn is_root_target_file(rel_path: &str) -> bool {
    let lower = rel_path.to_lowercase();
    lower.contains("skse64_")
        || lower.contains("f4se_")
        || lower.contains("obse_")
        || lower.ends_with(".exe")
        || lower.ends_with("d3d11.dll")
        || lower.ends_with("dxgi.dll")
        || lower.ends_with("enbseries.ini")
}
