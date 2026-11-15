use reqwest::Client;
use serde_json::Value;
use std::fs;
use std::path::Path;
use uuid::Uuid;

use crate::db::{self, ProfileModRecord};
use crate::modding::adapter::{ProviderKind, UnifiedModSearchResult};
use super::instance::{get_cache_mods_dir, get_instance_mods_dir};

fn create_symlink_atomic(src: &Path, target: &Path) -> Result<(), String> {
    if let Some(parent) = target.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if target.exists() || target.is_symlink() {
        let _ = fs::remove_file(target);
    }

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(src, target)
            .map_err(|e| format!("Erreur création symlink: {}", e))?;
        return Ok(());
    }

    #[cfg(windows)]
    {
        std::os::windows::fs::symlink_file(src, target)
            .or_else(|_| fs::copy(src, target).map(|_| ()))
            .map_err(|e| format!("Erreur liaison fichier: {}", e))?;
        return Ok(());
    }

    #[allow(unreachable_code)]
    {
        fs::copy(src, target).map(|_| ()).map_err(|e| e.to_string())
    }
}

pub async fn search_mods(
    client: &Client,
    query: &str,
    mc_version: Option<&str>,
    modloader: Option<&str>,
    provider: Option<&str>,
) -> Result<Vec<UnifiedModSearchResult>, String> {
    let mut results = Vec::new();
    let prov = provider.unwrap_or("all");
    let do_modrinth = prov == "all" || prov == "modrinth";
    let do_curseforge = prov == "all" || prov == "curseforge";

    // 1. Modrinth API v2
    if do_modrinth {
        let mut facets: Vec<Vec<String>> = vec![vec!["project_type:mod".to_string()]];
        if let Some(v) = mc_version {
            if !v.is_empty() {
                facets.push(vec![format!("versions:{}", v)]);
            }
        }
        if let Some(l) = modloader {
            if !l.is_empty() && l != "vanilla" {
                facets.push(vec![format!("categories:{}", l.to_lowercase())]);
            }
        }

        let facets_json = serde_json::to_string(&facets).unwrap_or_else(|_| "[]".to_string());
        let url = format!(
            "https://api.modrinth.com/v2/search?query={}&limit=25&facets={}",
            urlencoding::encode(query),
            urlencoding::encode(&facets_json)
        );

        if let Ok(resp) = client.get(&url).header("User-Agent", "LoomLauncher/0.1.0").send().await {
            if resp.status().is_success() {
                if let Ok(json) = resp.json::<Value>().await {
                    if let Some(hits) = json["hits"].as_array() {
                        for h in hits {
                            results.push(UnifiedModSearchResult {
                                id: h["project_id"].as_str().unwrap_or("").to_string(),
                                slug: h["slug"].as_str().unwrap_or("").to_string(),
                                name: h["title"].as_str().unwrap_or("").to_string(),
                                summary: h["description"].as_str().unwrap_or("").to_string(),
                                author: h["author"].as_str().unwrap_or("").to_string(),
                                icon_url: h["icon_url"].as_str().map(String::from),
                                downloads: h["downloads"].as_u64().unwrap_or(0),
                                provider: ProviderKind::Modrinth,
                                latest_version: h["latest_version"].as_str().map(String::from),
                                categories: h["categories"].as_array().map(|a| {
                                    a.iter().filter_map(|s| s.as_str().map(String::from)).collect()
                                }).unwrap_or_default(),
                                loaders: h["loaders"].as_array().map(|a| {
                                    a.iter().filter_map(|s| s.as_str().map(String::from)).collect()
                                }).unwrap_or_default(),
                                game_versions: h["versions"].as_array().map(|a| {
                                    a.iter().filter_map(|s| s.as_str().map(String::from)).collect()
                                }).unwrap_or_default(),
                                project_type: Some("mod".to_string()),
                            });
                        }
                    }
                }
            }
        }
    }

    // 2. CurseForge API
    if do_curseforge {
        let cf_loader = match modloader.unwrap_or("") {
            "forge" => "1",
            "fabric" => "4",
            "quilt" => "5",
            "neoforge" => "6",
            _ => "",
        };

        let mut url = format!(
            "https://api.curseforge.com/v1/mods/search?gameId=432&classId=6&pageSize=30&sortField=2&sortOrder=desc"
        );
        let q_trimmed = query.trim();
        if !q_trimmed.is_empty() {
            url.push_str(&format!("&searchFilter={}", urlencoding::encode(q_trimmed)));
        }
        if let Some(v) = mc_version {
            if !v.is_empty() {
                url.push_str(&format!("&gameVersion={}", urlencoding::encode(v)));
            }
        }
        if !cf_loader.is_empty() {
            url.push_str(&format!("&modLoaderType={}", cf_loader));
        }

        let cf_api_key = std::env::var("CURSEFORGE_API_KEY").unwrap_or_else(|_| {
            "$2a$10$bL4bIL5pUWqfcO7KQtnMReakwtfHbNKh6v1uTpKlzhwoueEJQnPnm".to_string()
        });
        const CF_FALLBACK_KEY: &str = "$2a$10$bL4bIL5pUWqfcO7KQtnMReakwtfHbNKh6v1uTpKlzhwoueEJQnPnm";

        let mut cf_response = client
            .get(&url)
            .header("x-api-key", &cf_api_key)
            .header("Accept", "application/json")
            .send()
            .await;

        if let Ok(ref resp) = cf_response {
            if resp.status().as_u16() == 403 && cf_api_key != CF_FALLBACK_KEY {
                cf_response = client
                    .get(&url)
                    .header("x-api-key", CF_FALLBACK_KEY)
                    .header("Accept", "application/json")
                    .send()
                    .await;
            }
        }

        if let Ok(resp) = cf_response {
            if resp.status().is_success() {
                if let Ok(json) = resp.json::<Value>().await {
                    if let Some(data) = json["data"].as_array() {
                        for item in data {
                            results.push(UnifiedModSearchResult {
                                id: item["id"].to_string(),
                                slug: item["slug"].as_str().unwrap_or("").to_string(),
                                name: item["name"].as_str().unwrap_or("").to_string(),
                                summary: item["summary"].as_str().unwrap_or("").to_string(),
                                author: item["authors"][0]["name"].as_str().unwrap_or("Inconnu").to_string(),
                                icon_url: item["logo"]["url"].as_str().map(String::from),
                                downloads: item["downloadCount"].as_u64().unwrap_or(0),
                                provider: ProviderKind::CurseForge,
                                latest_version: None,
                                categories: item["categories"].as_array().map(|a| {
                                    a.iter().filter_map(|c| c["name"].as_str().map(String::from)).collect()
                                }).unwrap_or_default(),
                                loaders: Vec::new(),
                                game_versions: Vec::new(),
                                project_type: Some("mod".to_string()),
                            });
                        }
                    }
                }
            }
        }
    }

    results.sort_by(|a, b| b.downloads.cmp(&a.downloads));
    Ok(results)
}

/// Downloads mod archive into shared cache and deploys symlink to instance
pub async fn install_mod(
    client: &Client,
    instance_id: &str,
    provider: &str,
    remote_mod_id: &str,
    remote_file_id: &str,
    download_url: Option<&str>,
    filename: &str,
    mod_name: Option<&str>,
) -> Result<ProfileModRecord, String> {
    let cache_dir = get_cache_mods_dir();
    let safe_filename = if filename.is_empty() {
        format!("{}.jar", remote_mod_id)
    } else {
        filename.to_string()
    };
    let cached_path = cache_dir.join(&safe_filename);

    // 1. Download if not already in cache
    if !cached_path.exists() {
        let final_url = if let Some(url) = download_url {
            if !url.is_empty() {
                Some(url.to_string())
            } else {
                None
            }
        } else {
            None
        };

        let resolved_url = match final_url {
            Some(u) => u,
            None => {
                if provider == "modrinth" {
                    // Resolve file from Modrinth version API
                    let ver_url = if !remote_file_id.is_empty() && remote_file_id != remote_mod_id {
                        format!("https://api.modrinth.com/v2/version/{}", remote_file_id)
                    } else {
                        format!("https://api.modrinth.com/v2/project/{}/version", remote_mod_id)
                    };

                    let v_resp = client
                        .get(&ver_url)
                        .header("User-Agent", "LoomLauncher/0.1.0")
                        .send()
                        .await
                        .map_err(|e| format!("Erreur résolution version Modrinth: {}", e))?;

                    let v_json: Value = v_resp.json().await.map_err(|e| e.to_string())?;
                    let primary_file = if v_json.is_array() {
                        &v_json[0]["files"][0]
                    } else {
                        &v_json["files"][0]
                    };
                    primary_file["url"]
                        .as_str()
                        .ok_or("URL de téléchargement Modrinth introuvable")?
                        .to_string()
                } else if provider == "curseforge" {
                    let cf_url = format!(
                        "https://api.curseforge.com/v1/mods/{}/files/{}/download-url",
                        remote_mod_id, remote_file_id
                    );
                    let cf_api_key = std::env::var("CURSEFORGE_API_KEY").unwrap_or_else(|_| {
                        "$2a$10$bL4bIL5pUWqfcO7KQtnMReakwtfHbNKh6v1uTpKlzhwoueEJQnPnm".to_string()
                    });
                    let cf_resp = client
                        .get(&cf_url)
                        .header("x-api-key", &cf_api_key)
                        .send()
                        .await
                        .map_err(|e| format!("Erreur résolution download CurseForge: {}", e))?;
                    let cf_json: Value = cf_resp.json().await.map_err(|e| e.to_string())?;
                    cf_json["data"]
                        .as_str()
                        .ok_or("URL de téléchargement CurseForge introuvable")?
                        .to_string()
                } else {
                    return Err(format!("Fournisseur inconnu ou URL manquante: {}", provider));
                }
            }
        };

        println!(
            "[Loom::Mods] Downloading mod {} to cache: {}",
            safe_filename,
            cached_path.display()
        );

        let resp = client
            .get(&resolved_url)
            .send()
            .await
            .map_err(|e| format!("Erreur lors du téléchargement du mod: {}", e))?;

        if !resp.status().is_success() {
            return Err(format!("Téléchargement échoué avec statut {}", resp.status()));
        }

        let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
        fs::write(&cached_path, &bytes).map_err(|e| format!("Erreur écriture cache: {}", e))?;
    }

    // 2. Deploy symlink into instance mods folder
    let target_symlink = get_instance_mods_dir(instance_id).join(&safe_filename);
    create_symlink_atomic(&cached_path, &target_symlink)?;

    // 3. Insert record into SQLite
    let mod_record_id = format!("pmod-{}", Uuid::new_v4().to_string().replace('-', "")[..10].to_string());
    let display_name = mod_name.unwrap_or(&safe_filename).to_string();

    let record = ProfileModRecord {
        id: mod_record_id,
        profile_id: instance_id.to_string(),
        provider: provider.to_string(),
        remote_mod_id: remote_mod_id.to_string(),
        remote_file_id: remote_file_id.to_string(),
        name: display_name,
        file_path: safe_filename.clone(),
        filename: safe_filename,
        load_priority: 0,
        is_enabled: true,
    };

    let conn = db::open_db().map_err(|e| e.to_string())?;
    db::insert_profile_mod(&conn, &record).map_err(|e| e.to_string())?;

    Ok(record)
}

pub fn toggle_mod(instance_id: &str, mod_id: &str, enabled: bool) -> Result<(), String> {
    let conn = db::open_db().map_err(|e| e.to_string())?;
    let mods = db::get_profile_mods(&conn, instance_id).map_err(|e| e.to_string())?;
    let target_mod = mods.iter().find(|m| m.id == mod_id).ok_or("Mod introuvable")?;

    let filename = if !target_mod.filename.is_empty() {
        &target_mod.filename
    } else {
        &target_mod.file_path
    };

    let mods_dir = get_instance_mods_dir(instance_id);
    let active_path = mods_dir.join(filename);
    let disabled_path = mods_dir.join(format!("{}.disabled", filename));

    if enabled {
        // Enable: ensure active symlink exists
        if disabled_path.exists() || disabled_path.is_symlink() {
            let _ = fs::rename(&disabled_path, &active_path);
        } else {
            let cache_path = get_cache_mods_dir().join(filename);
            if cache_path.exists() {
                let _ = create_symlink_atomic(&cache_path, &active_path);
            }
        }
    } else {
        // Disable: rename to .disabled
        if active_path.exists() || active_path.is_symlink() {
            let _ = fs::rename(&active_path, &disabled_path);
        }
    }

    db::toggle_profile_mod(&conn, instance_id, mod_id, enabled).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn delete_mod(instance_id: &str, mod_id: &str) -> Result<(), String> {
    let conn = db::open_db().map_err(|e| e.to_string())?;
    let mods = db::get_profile_mods(&conn, instance_id).map_err(|e| e.to_string())?;
    if let Some(target_mod) = mods.iter().find(|m| m.id == mod_id) {
        let filename = if !target_mod.filename.is_empty() {
            &target_mod.filename
        } else {
            &target_mod.file_path
        };

        let mods_dir = get_instance_mods_dir(instance_id);
        let active_path = mods_dir.join(filename);
        let disabled_path = mods_dir.join(format!("{}.disabled", filename));

        let _ = fs::remove_file(active_path);
        let _ = fs::remove_file(disabled_path);

        db::delete_profile_mod(&conn, instance_id, mod_id).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn install_local_mod(
    instance_id: &str,
    source_path: &str,
    filename: &str,
) -> Result<ProfileModRecord, String> {
    let cache_dir = get_cache_mods_dir();
    let safe_filename = if filename.is_empty() {
        Path::new(source_path)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "mod.jar".to_string())
    } else {
        filename.to_string()
    };

    let cache_path = cache_dir.join(&safe_filename);
    let src = Path::new(source_path);

    if src.exists() && src != cache_path {
        fs::copy(src, &cache_path).map_err(|e| format!("Erreur copie dans le cache: {}", e))?;
    }

    let target_symlink = get_instance_mods_dir(instance_id).join(&safe_filename);
    create_symlink_atomic(&cache_path, &target_symlink)?;

    let mod_id = format!("pmod-{}", Uuid::new_v4().to_string().replace('-', "")[..10].to_string());
    let name = safe_filename.replace(".jar", "").replace(".zip", "");

    let record = ProfileModRecord {
        id: mod_id,
        profile_id: instance_id.to_string(),
        provider: "local".to_string(),
        remote_mod_id: safe_filename.clone(),
        remote_file_id: safe_filename.clone(),
        name,
        file_path: safe_filename.clone(),
        filename: safe_filename,
        load_priority: 0,
        is_enabled: true,
    };

    let conn = db::open_db().map_err(|e| e.to_string())?;
    db::insert_profile_mod(&conn, &record).map_err(|e| e.to_string())?;

    Ok(record)
}

pub fn get_installed_mods(instance_id: &str) -> Result<Vec<ProfileModRecord>, String> {
    let conn = db::open_db().map_err(|e| e.to_string())?;
    db::get_profile_mods(&conn, instance_id).map_err(|e| e.to_string())
}
