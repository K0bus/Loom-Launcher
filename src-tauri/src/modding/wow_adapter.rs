use std::fs;
use std::path::PathBuf;

use crate::db::{open_db, get_profile_mods, get_profiles};
use crate::modding::adapter::{GameAdapter, GameModdingManifest, LaunchExecutable, ProviderKind};
use crate::modding::symlink::create_atomic_symlink;

const WOW_TRACKING_FILE: &str = ".loom_deployed_addons.json";

pub struct WowAdapter {
    pub branch: String, // "_retail_", "_classic_", "_classic_era_"
}

impl Default for WowAdapter {
    fn default() -> Self {
        Self::new("_retail_")
    }
}

impl WowAdapter {
    pub fn new(branch: impl Into<String>) -> Self {
        Self {
            branch: branch.into(),
        }
    }

    /// Resolves the Battle.net / WoW AddOns directory in Wine prefix
    pub fn resolve_default_addons_dir(&self) -> PathBuf {
        let prefix = crate::db::get_default_battlenet_prefix();
        let target = prefix
            .join("drive_c/Program Files (x86)/World of Warcraft")
            .join(&self.branch)
            .join("Interface/AddOns");
        let _ = fs::create_dir_all(&target);
        target
    }
}

impl GameAdapter for WowAdapter {
    fn manifest(&self) -> GameModdingManifest {
        GameModdingManifest::new(
            "wow",
            format!("World of Warcraft ({})", self.branch),
            vec![
                ProviderKind::CurseForge,
                ProviderKind::Wago,
                ProviderKind::LocalArchive,
            ],
            false, // allows_load_order
            false, // allows_conflict_override
            false, // supports_modpacks
            vec![
                LaunchExecutable::new("wow", "World of Warcraft", "Wow.exe", true, false),
                LaunchExecutable::new("battlenet", "Battle.net Launcher", "Battle.net.exe", false, false),
            ],
        )
    }

    fn get_staging_directory(&self, profile_id: &str) -> PathBuf {
        let base = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string())).join(".local/share"))
            .join("loom")
            .join("profiles")
            .join("wow")
            .join(profile_id)
            .join("staged");
        let _ = fs::create_dir_all(&base);
        base
    }

    fn get_deployment_directory(&self, profile_id: &str) -> PathBuf {
        if let Ok(conn) = open_db() {
            if let Ok(profiles) = get_profiles(&conn, Some("wow")) {
                if let Some(prof) = profiles.into_iter().find(|p| p.id == profile_id) {
                    if let Some(dep_path) = prof.deployment_path {
                        let path = PathBuf::from(&dep_path);
                        if path.exists() {
                            let addons = path.join("Interface/AddOns");
                            let _ = fs::create_dir_all(&addons);
                            return addons;
                        }
                    }
                }
            }
        }

        self.resolve_default_addons_dir()
    }

    fn normalize_staged_mod(&self, mod_staged_path: &PathBuf) -> Result<(), String> {
        if !mod_staged_path.is_dir() {
            return Err(format!("Staged addon path is not a directory: {}", mod_staged_path.display()));
        }

        // Check if there is any .toc file in this directory or subdirectories
        let mut has_toc = false;
        if let Ok(entries) = fs::read_dir(mod_staged_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("toc") {
                    has_toc = true;
                    break;
                }
                if path.is_dir() {
                    if let Ok(sub) = fs::read_dir(&path) {
                        for sub_entry in sub.flatten() {
                            if sub_entry.path().extension().and_then(|s| s.to_str()) == Some("toc") {
                                has_toc = true;
                                break;
                            }
                        }
                    }
                }
            }
        }

        if !has_toc {
            eprintln!("[WowAdapter] Warning: no .toc file found in {}", mod_staged_path.display());
        }

        Ok(())
    }

    fn deploy_profile(&self, profile_id: &str) -> Result<(), String> {
        let staging_dir = self.get_staging_directory(profile_id);
        let deployment_dir = self.get_deployment_directory(profile_id);

        self.undeploy_profile(profile_id)?;

        let conn = open_db().map_err(|e| e.to_string())?;
        let mods = get_profile_mods(&conn, profile_id).map_err(|e| e.to_string())?;

        let mut deployed_links: Vec<String> = Vec::new();

        for pmod in mods {
            if !pmod.is_enabled {
                continue;
            }

            let mod_staged = if let Some(ref sp) = pmod.staged_path {
                PathBuf::from(sp)
            } else {
                staging_dir.join(&pmod.id)
            };

            if !mod_staged.is_dir() {
                continue;
            }

            let _ = self.normalize_staged_mod(&mod_staged);

            // In WoW, archives often contain 1 or more top-level addon folders (each with a .toc)
            if let Ok(entries) = fs::read_dir(&mod_staged) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let folder_name = entry.file_name();
                        let target_symlink = deployment_dir.join(&folder_name);

                        if let Ok(()) = create_atomic_symlink(&path, &target_symlink) {
                            deployed_links.push(target_symlink.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }

        // Save deployed links to tracking file
        let tracking_file = deployment_dir.join(WOW_TRACKING_FILE);
        if let Ok(json) = serde_json::to_string_pretty(&deployed_links) {
            let _ = fs::write(tracking_file, json);
        }

        Ok(())
    }

    fn undeploy_profile(&self, profile_id: &str) -> Result<(), String> {
        let deployment_dir = self.get_deployment_directory(profile_id);
        let tracking_file = deployment_dir.join(WOW_TRACKING_FILE);

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
