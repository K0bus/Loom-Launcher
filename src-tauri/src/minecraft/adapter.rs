use std::fs;
use std::path::{Path, PathBuf};

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
        GameModdingManifest::new(
            "minecraft",
            "Minecraft: Java Edition",
            vec![
                ProviderKind::Modrinth,
                ProviderKind::CurseForge,
                ProviderKind::LocalArchive,
            ],
            true,
            true,
            true,
            vec![
                LaunchExecutable::new("client", "Client Minecraft (JVM)", "bin/java", true, false),
                LaunchExecutable::new("client-debug", "Client Minecraft (Debug / Logs détaillés)", "bin/java", false, false),
            ],
        )
    }

    fn get_staging_directory(&self, profile_id: &str) -> PathBuf {
        let base = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string())).join(".local/share"))
            .join("loom")
            .join("profiles")
            .join("minecraft")
            .join(profile_id)
            .join("staged");
        let _ = fs::create_dir_all(&base);
        base
    }

    fn get_deployment_directory(&self, profile_id: &str) -> PathBuf {
        let inst_dir = Self::get_instance_dir(profile_id);
        let mods_dir = inst_dir.join("mods");
        let _ = fs::create_dir_all(&mods_dir);
        mods_dir
    }

    fn normalize_staged_mod(&self, mod_staged_path: &PathBuf) -> Result<(), String> {
        if !mod_staged_path.exists() {
            return Err(format!("Staged mod path does not exist: {}", mod_staged_path.display()));
        }
        Ok(())
    }

    fn deploy_profile(&self, profile_id: &str) -> Result<(), String> {
        let mods_dir = self.get_deployment_directory(profile_id);
        let cache_dir = get_mods_cache_dir();

        // 1. Clear previous symlinks
        clear_symlinks_in_dir(&mods_dir)
            .map_err(|e| format!("Failed to clear existing symlinks: {}", e))?;

        let conn = crate::db::open_db().map_err(|e| e.to_string())?;
        let mods = crate::db::get_profile_mods(&conn, profile_id).unwrap_or_default();

        // 2. Deploy symlinks for all enabled mods
        for pmod in mods {
            if !pmod.is_enabled {
                continue;
            }

            let source_path = if Path::new(&pmod.file_path).is_absolute() && Path::new(&pmod.file_path).exists() {
                PathBuf::from(&pmod.file_path)
            } else if let Some(ref sp) = pmod.staged_path {
                let p = PathBuf::from(sp);
                if p.exists() { p } else { cache_dir.join(&pmod.file_path) }
            } else {
                cache_dir.join(&pmod.file_path)
            };

            if !source_path.exists() {
                eprintln!("[MinecraftAdapter] Cache file not found: {}", source_path.display());
                continue;
            }

            let file_name = source_path
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_else(|| format!("{}.jar", pmod.name));

            let link_target = mods_dir.join(file_name);

            // If the source is already the exact file in the target directory, don't create a circular symlink
            if source_path == link_target {
                continue;
            }
            if let (Ok(can_src), Ok(can_tgt)) = (source_path.canonicalize(), link_target.canonicalize()) {
                if can_src == can_tgt {
                    continue;
                }
            }

            create_atomic_symlink(&source_path, &link_target)
                .map_err(|e| format!("Failed to create symlink for {}: {}", pmod.name, e))?;
        }

        Ok(())
    }

    fn undeploy_profile(&self, profile_id: &str) -> Result<(), String> {
        let mods_dir = self.get_deployment_directory(profile_id);
        clear_symlinks_in_dir(&mods_dir)
            .map_err(|e| format!("Failed to clear symlinks in {}: {}", mods_dir.display(), e))?;
        Ok(())
    }
}
