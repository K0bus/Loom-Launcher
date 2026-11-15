use reqwest::Client;
use serde_json::Value;
use std::fs::{self, File};
use std::io::Read;
use std::path::Path;
use uuid::Uuid;
use zip::ZipArchive;

use crate::db::{self, GameProfileRecord, ProfileModRecord};
use crate::modding::providers::curseforge::CurseForgeProvider;
use crate::modding::providers::ModProvider;
use crate::modding::symlink::{create_atomic_symlink, get_mods_cache_dir};

use super::adapter::MinecraftAdapter;
use super::manifest::verify_sha1;

pub struct ModpackInstaller;

impl ModpackInstaller {
    /// Installs a Modrinth modpack (`.mrpack`) from a local file archive or downloaded file
    pub async fn install_mrpack(
        client: &Client,
        archive_path: &Path,
        custom_name: Option<&str>,
    ) -> Result<String, String> {
        let file = File::open(archive_path)
            .map_err(|e| format!("Failed to open .mrpack file: {}", e))?;
        let mut archive = ZipArchive::new(file)
            .map_err(|e| format!("Failed to read .mrpack ZIP archive: {}", e))?;

        // 1. Read modrinth.index.json
        let mut index_file = archive
            .by_name("modrinth.index.json")
            .map_err(|_| "Invalid .mrpack: missing modrinth.index.json".to_string())?;

        let mut index_str = String::new();
        index_file
            .read_to_string(&mut index_str)
            .map_err(|e| e.to_string())?;
        drop(index_file);

        let index_json: Value = serde_json::from_str(&index_str)
            .map_err(|e| format!("Invalid JSON in modrinth.index.json: {}", e))?;

        let pack_name = custom_name.unwrap_or_else(|| {
            index_json["name"].as_str().unwrap_or("Modrinth Modpack")
        });

        let mc_version = index_json["dependencies"]["minecraft"]
            .as_str()
            .unwrap_or("1.20.1")
            .to_string();

        let mut modloader = "fabric".to_string();
        if index_json["dependencies"]["neoforge"].is_string() {
            modloader = "neoforge".to_string();
        } else if index_json["dependencies"]["forge"].is_string() {
            modloader = "forge".to_string();
        } else if index_json["dependencies"]["quilt-loader"].is_string() {
            modloader = "quilt".to_string();
        }

        // 2. Create Instance Directory
        let instance_id = format!("inst-{}", Uuid::new_v4().simple());
        let instance_dir = MinecraftAdapter::get_instance_dir(&instance_id);
        let mods_dir = instance_dir.join("mods");
        let _ = fs::create_dir_all(&mods_dir);

        // 3. Extract overrides/ folder
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
            let entry_name = entry.name().to_string();

            if entry_name.starts_with("overrides/") && !entry.is_dir() {
                let rel_path = entry_name.trim_start_matches("overrides/");
                let dest_path = instance_dir.join(rel_path);
                if let Some(parent) = dest_path.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                let mut out_file = File::create(&dest_path).map_err(|e| e.to_string())?;
                std::io::copy(&mut entry, &mut out_file).map_err(|e| e.to_string())?;
            }
        }

        // 4. Download and symlink mod files
        let cache_dir = get_mods_cache_dir();
        let conn = db::open_db().map_err(|e| e.to_string())?;

        let profile_record = GameProfileRecord {
            id: instance_id.clone(),
            game_id: "minecraft".to_string(),
            name: pack_name.to_string(),
            game_version: mc_version.clone(),
            modloader: Some(modloader.clone()),
            created_at: chrono::Utc::now().timestamp(),
        };
        db::insert_profile(&conn, &profile_record).map_err(|e| e.to_string())?;

        if let Some(files) = index_json["files"].as_array() {
            for (idx, f) in files.iter().enumerate() {
                let path_str = f["path"].as_str().unwrap_or("");
                let sha1_str = f["hashes"]["sha1"].as_str().unwrap_or("");
                let download_urls = f["downloads"].as_array();

                let filename = Path::new(path_str)
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| format!("mod_{}.jar", idx));

                let cache_target = cache_dir.join(&filename);

                // Download to cache if not already present with matching sha1
                let mut downloaded = false;
                if cache_target.exists() {
                    if let Ok(b) = fs::read(&cache_target) {
                        if verify_sha1(&b, sha1_str) {
                            downloaded = true;
                        }
                    }
                }

                if !downloaded {
                    if let Some(urls) = download_urls {
                        for u in urls {
                            if let Some(url_str) = u.as_str() {
                                if let Ok(resp) = client.get(url_str).send().await {
                                    if resp.status().is_success() {
                                        if let Ok(b) = resp.bytes().await {
                                            if verify_sha1(&b, sha1_str) {
                                                let _ = fs::write(&cache_target, &b);
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // If downloaded into cache, create symlink in instance directory
                if cache_target.exists() {
                    let instance_mod_target = instance_dir.join(path_str);
                    let _ = create_atomic_symlink(&cache_target, &instance_mod_target);

                    let mod_record = ProfileModRecord {
                        id: format!("pmod-{}", Uuid::new_v4().simple()),
                        profile_id: instance_id.clone(),
                        provider: "modrinth".to_string(),
                        remote_mod_id: filename.clone(),
                        remote_file_id: sha1_str.to_string(),
                        name: filename.clone(),
                        file_path: filename.clone(),
                        filename: filename.clone(),
                        load_priority: idx as i32,
                        is_enabled: true,
                    };
                    let _ = db::insert_profile_mod(&conn, &mod_record);
                }
            }
        }

        Ok(instance_id)
    }

    /// Installs a CurseForge modpack ZIP archive (manifest.json + overrides)
    pub async fn install_curseforge_pack(
        archive_path: &Path,
        custom_name: Option<&str>,
    ) -> Result<String, String> {
        let file = File::open(archive_path)
            .map_err(|e| format!("Failed to open CurseForge archive: {}", e))?;
        let mut archive = ZipArchive::new(file)
            .map_err(|e| format!("Failed to read CurseForge ZIP: {}", e))?;

        // 1. Read manifest.json
        let mut manifest_file = archive
            .by_name("manifest.json")
            .map_err(|_| "Invalid CurseForge pack: missing manifest.json".to_string())?;

        let mut manifest_str = String::new();
        manifest_file
            .read_to_string(&mut manifest_str)
            .map_err(|e| e.to_string())?;
        drop(manifest_file);

        let manifest_json: Value = serde_json::from_str(&manifest_str)
            .map_err(|e| format!("Invalid JSON in manifest.json: {}", e))?;

        let pack_name = custom_name.unwrap_or_else(|| {
            manifest_json["name"].as_str().unwrap_or("CurseForge Modpack")
        });

        let mc_version = manifest_json["minecraft"]["version"]
            .as_str()
            .unwrap_or("1.20.1")
            .to_string();

        let mut modloader = "forge".to_string();
        if let Some(loaders) = manifest_json["minecraft"]["modLoaders"].as_array() {
            if let Some(l) = loaders.first().and_then(|x| x["id"].as_str()) {
                if l.starts_with("fabric") {
                    modloader = "fabric".to_string();
                } else if l.starts_with("neoforge") {
                    modloader = "neoforge".to_string();
                } else if l.starts_with("quilt") {
                    modloader = "quilt".to_string();
                }
            }
        }

        // 2. Create Instance Directory
        let instance_id = format!("inst-{}", Uuid::new_v4().simple());
        let instance_dir = MinecraftAdapter::get_instance_dir(&instance_id);
        let mods_dir = instance_dir.join("mods");
        let _ = fs::create_dir_all(&mods_dir);

        // 3. Extract overrides/ folder
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
            let entry_name = entry.name().to_string();

            if entry_name.starts_with("overrides/") && !entry.is_dir() {
                let rel_path = entry_name.trim_start_matches("overrides/");
                let dest_path = instance_dir.join(rel_path);
                if let Some(parent) = dest_path.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                let mut out_file = File::create(&dest_path).map_err(|e| e.to_string())?;
                std::io::copy(&mut entry, &mut out_file).map_err(|e| e.to_string())?;
            }
        }

        // 4. Download and symlink files via CurseForge API
        let cf_provider = CurseForgeProvider::new();
        let cache_dir = get_mods_cache_dir();
        let conn = db::open_db().map_err(|e| e.to_string())?;

        let profile_record = GameProfileRecord {
            id: instance_id.clone(),
            game_id: "minecraft".to_string(),
            name: pack_name.to_string(),
            game_version: mc_version.clone(),
            modloader: Some(modloader.clone()),
            created_at: chrono::Utc::now().timestamp(),
        };
        db::insert_profile(&conn, &profile_record).map_err(|e| e.to_string())?;

        if let Some(files) = manifest_json["files"].as_array() {
            for (idx, f) in files.iter().enumerate() {
                let project_id = f["projectID"].as_i64().unwrap_or(0).to_string();
                let file_id = f["fileID"].as_i64().unwrap_or(0).to_string();

                if project_id == "0" || file_id == "0" {
                    continue;
                }

                // Query CurseForge file info
                if let Ok(versions) = cf_provider.get_versions(&project_id, None, None).await {
                    if let Some(v) = versions.iter().find(|x| x.id == file_id) {
                        if let Some(f_info) = v.files.first() {
                            if let Ok(cached_path) = cf_provider
                                .download_file(&f_info.url, &cache_dir, &f_info.filename)
                                .await
                            {
                                let link_target = mods_dir.join(&f_info.filename);
                                let _ = create_atomic_symlink(&cached_path, &link_target);

                                let mod_record = ProfileModRecord {
                                    id: format!("pmod-{}", Uuid::new_v4().simple()),
                                    profile_id: instance_id.clone(),
                                    provider: "curseforge".to_string(),
                                    remote_mod_id: project_id.clone(),
                                    remote_file_id: file_id.clone(),
                                    name: f_info.filename.clone(),
                                    file_path: f_info.filename.clone(),
                                    filename: f_info.filename.clone(),
                                    load_priority: idx as i32,
                                    is_enabled: true,
                                };
                                let _ = db::insert_profile_mod(&conn, &mod_record);
                            }
                        }
                    }
                }
            }
        }

        Ok(instance_id)
    }
}
