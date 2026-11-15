use std::fs;
use std::path::{Path, PathBuf};

use crate::db::ProfileModRecord;
use crate::modding::adapter::{GameAdapter, GameModdingManifest, LaunchExecutable, ProviderKind};
use crate::modding::symlink::{clear_symlinks_in_dir, create_atomic_symlink, get_mods_cache_dir};

pub struct MinecraftAdapter;

impl Default for MinecraftAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl MinecraftAdapter {
    pub fn new() -> Self {
        Self
    }

    /// Returns the instance root directory:
    /// `~/.local/share/loom/instances/<instance_id>/.minecraft/`
    pub fn get_instance_dir(instance_id: &str) -> PathBuf {
        let base = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string())).join(".local/share"))
            .join("loom")
            .join("instances")
            .join(instance_id)
            .join(".minecraft");
        let _ = fs::create_dir_all(&base);
        base
    }
}

impl GameAdapter for MinecraftAdapter {
    fn manifest(&self) -> GameModdingManifest {
        GameModdingManifest {
            game_id: "minecraft".to_string(),
            game_name: "Minecraft: Java Edition".to_string(),
            supported_providers: vec![
                ProviderKind::Modrinth,
                ProviderKind::CurseForge,
                ProviderKind::Local,
            ],
            supports_load_order: true,
            supports_conflict_resolution: true,
            mods_directory: "mods".to_string(),
            file_extensions: vec![".jar".to_string(), ".zip".to_string()],
            executables: vec![
                LaunchExecutable {
                    id: "client".to_string(),
                    label: "Client Minecraft (JVM)".to_string(),
                    description: Some("Lancement standard du client Minecraft".to_string()),
                    is_default: true,
                },
                LaunchExecutable {
                    id: "client-debug".to_string(),
                    label: "Client Minecraft (Debug / Logs détaillés)".to_string(),
                    description: Some("Lancement avec logs JVM étendus".to_string()),
                    is_default: false,
                },
            ],
        }
    }

    fn get_profile_mods_path(&self, profile_id: &str) -> PathBuf {
        let inst_dir = Self::get_instance_dir(profile_id);
        let mods_dir = inst_dir.join("mods");
        let _ = fs::create_dir_all(&mods_dir);
        mods_dir
    }

    fn deploy_profile(&self, profile_id: &str, mods: &[ProfileModRecord]) -> Result<(), String> {
        let mods_dir = self.get_profile_mods_path(profile_id);
        let cache_dir = get_mods_cache_dir();

        // 1. Clear previous symlinks
        clear_symlinks_in_dir(&mods_dir)
            .map_err(|e| format!("Failed to clear existing symlinks: {}", e))?;

        // 2. Deploy symlinks for all enabled mods
        for pmod in mods {
            if !pmod.is_enabled {
                continue;
            }

            // Determine source binary location
            let source_path = if Path::new(&pmod.file_path).is_absolute() && Path::new(&pmod.file_path).exists() {
                PathBuf::from(&pmod.file_path)
            } else {
                cache_dir.join(&pmod.file_path)
            };

            if !source_path.exists() {
                // If not found in cache, continue or log warning
                eprintln!("[MinecraftAdapter] Cache file not found: {}", source_path.display());
                continue;
            }

            let file_name = source_path
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_else(|| format!("{}.jar", pmod.name));

            let link_target = mods_dir.join(file_name);

            create_atomic_symlink(&source_path, &link_target)
                .map_err(|e| format!("Failed to create symlink for {}: {}", pmod.name, e))?;
        }

        Ok(())
    }

    fn undeploy_profile(&self, profile_id: &str) -> Result<(), String> {
        let mods_dir = self.get_profile_mods_path(profile_id);
        clear_symlinks_in_dir(&mods_dir)
            .map_err(|e| format!("Failed to clear symlinks in {}: {}", mods_dir.display(), e))?;
        Ok(())
    }
}
