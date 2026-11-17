use reqwest::Client;
use tauri::AppHandle;

use crate::db::{MinecraftInstanceRecord, ProfileModRecord};
use crate::minecraft::instance::{
    create_instance, delete_instance, get_instances, update_instance_settings, CreateInstanceParams,
};
use crate::minecraft::launcher::{launch_instance, LaunchEvents, MinecraftExitEvent, MinecraftLogEvent};
use crate::minecraft::mods::{
    delete_mod, install_local_mod, install_mod, search_mods, toggle_mod,
};
use crate::modding::adapter::UnifiedModSearchResult;

impl LaunchEvents for AppHandle {
    fn emit_log(&self, evt: MinecraftLogEvent) {
        let _ = tauri::Emitter::emit(self, "minecraft-log", evt);
    }
    fn emit_exit(&self, evt: MinecraftExitEvent) {
        let _ = tauri::Emitter::emit(self, "minecraft-exit", evt);
    }
}

#[tauri::command]
pub fn create_minecraft_instance_cmd(
    name: String,
    mc_version: String,
    modloader: String,
    loader_version: Option<String>,
    min_ram_mb: Option<u32>,
    max_ram_mb: Option<u32>,
    java_path: Option<String>,
    jvm_args: Option<Vec<String>>,
) -> Result<MinecraftInstanceRecord, String> {
    create_instance(CreateInstanceParams {
        name,
        mc_version,
        modloader,
        loader_version,
        min_ram_mb,
        max_ram_mb,
        java_path,
        jvm_args,
    })
}

#[tauri::command]
pub fn get_minecraft_instances_cmd() -> Result<Vec<MinecraftInstanceRecord>, String> {
    get_instances()
}

#[tauri::command]
pub fn delete_minecraft_instance_cmd(instance_id: String) -> Result<(), String> {
    delete_instance(&instance_id)
}

#[tauri::command]
pub fn update_minecraft_instance_settings_cmd(
    instance_id: String,
    min_ram_mb: u32,
    max_ram_mb: u32,
    jvm_args: Vec<String>,
    java_path: Option<String>,
) -> Result<(), String> {
    update_instance_settings(&instance_id, min_ram_mb, max_ram_mb, jvm_args, java_path)
}

#[tauri::command]
pub async fn launch_minecraft_instance_cmd(
    app: AppHandle,
    instance_id: String,
    allocated_ram_gb: Option<u32>,
) -> Result<(), String> {
    launch_instance(app, instance_id, allocated_ram_gb).await
}

#[tauri::command]
pub async fn search_minecraft_mods_cmd(
    query: String,
    mc_version: Option<String>,
    modloader: Option<String>,
    provider: Option<String>,
) -> Result<Vec<UnifiedModSearchResult>, String> {
    let client = Client::new();
    search_mods(
        &client,
        &query,
        mc_version.as_deref(),
        modloader.as_deref(),
        provider.as_deref(),
    )
    .await
}

#[tauri::command]
pub async fn install_minecraft_mod_cmd(
    instance_id: String,
    provider: String,
    remote_mod_id: String,
    remote_file_id: String,
    download_url: Option<String>,
    filename: String,
    mod_name: Option<String>,
) -> Result<ProfileModRecord, String> {
    let client = Client::new();
    install_mod(
        &client,
        &instance_id,
        &provider,
        &remote_mod_id,
        &remote_file_id,
        download_url.as_deref(),
        &filename,
        mod_name.as_deref(),
    )
    .await
}

#[tauri::command]
pub fn toggle_minecraft_mod_cmd(
    instance_id: String,
    mod_id: String,
    enabled: bool,
) -> Result<(), String> {
    toggle_mod(&instance_id, &mod_id, enabled)
}

#[tauri::command]
pub fn delete_minecraft_mod_cmd(instance_id: String, mod_id: String) -> Result<(), String> {
    delete_mod(&instance_id, &mod_id)
}

#[tauri::command]
pub fn install_local_minecraft_mod_cmd(
    instance_id: String,
    source_path: String,
    filename: String,
) -> Result<ProfileModRecord, String> {
    install_local_mod(&instance_id, &source_path, &filename)
}

#[tauri::command]
pub fn open_minecraft_instance_folder_cmd(
    instance_id: String,
    sub_dir: Option<String>,
) -> Result<(), String> {
    let mut base = dirs::data_local_dir()
        .map(|d| d.join("loom").join("instances").join(&instance_id))
        .ok_or_else(|| "Chemin instances introuvable".to_string())?;

    if let Some(sub) = sub_dir {
        base = base.join(sub);
    }
    let _ = std::fs::create_dir_all(&base);

    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("explorer.exe").arg(&base).spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(&base).spawn();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open").arg(&base).spawn();
    }

    Ok(())
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModpackCatalogItem {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub author: String,
    pub downloads: u64,
    pub likes: u64,
    pub icon_url: String,
    pub banner_url: String,
    pub summary: String,
    pub loader: String,
    pub mc_version: String,
    pub source: String,
    pub categories: Vec<String>,
}

#[tauri::command]
pub async fn search_modpacks_cmd(
    query: Option<String>,
    provider: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<ModpackCatalogItem>, String> {
    let client = Client::builder()
        .user_agent("LoomLauncher/0.1.0")
        .build()
        .unwrap_or_else(|_| Client::new());

    let q = query.unwrap_or_default().trim().to_string();
    let prov = provider.unwrap_or_else(|| "all".to_string()).to_lowercase();
    let max_results = limit.unwrap_or(24);
    let mut results = Vec::new();

    let do_modrinth = prov == "all" || prov == "modrinth";
    let do_curseforge = prov == "all" || prov == "curseforge";

    // 1. Modrinth Modpacks
    if do_modrinth {
        let sort_index = if q.is_empty() { "downloads" } else { "relevance" };
        let facets = "[[\"project_type:modpack\"]]";
        let mut req = client.get("https://api.modrinth.com/v2/search")
            .query(&[
                ("limit", &max_results.to_string()),
                ("index", &sort_index.to_string()),
                ("facets", &facets.to_string()),
            ]);
        if !q.is_empty() {
            req = req.query(&[("query", &q)]);
        }

        if let Ok(resp) = req.send().await {
            if resp.status().is_success() {
                if let Ok(json) = resp.json::<serde_json::Value>().await {
                    if let Some(hits) = json["hits"].as_array() {
                        for h in hits {
                            let categories: Vec<String> = h["categories"]
                                .as_array()
                                .map(|arr| arr.iter().filter_map(|c| c.as_str().map(String::from)).collect())
                                .unwrap_or_default();

                            let mut detected_loader = "fabric".to_string();
                            for cat in &categories {
                                let l = cat.to_lowercase();
                                if ["fabric", "neoforge", "forge", "quilt"].contains(&l.as_str()) {
                                    detected_loader = l;
                                    break;
                                }
                            }

                            let mc_version = h["versions"]
                                .as_array()
                                .and_then(|a| a.iter().filter_map(|v| v.as_str()).last())
                                .unwrap_or("1.20.1")
                                .to_string();

                            let icon = h["icon_url"].as_str().unwrap_or("").to_string();
                            let banner = h["gallery"]
                                .as_array()
                                .and_then(|a| a.first())
                                .and_then(|g| g.as_str().or_else(|| g["url"].as_str()))
                                .unwrap_or(&icon)
                                .to_string();

                            results.push(ModpackCatalogItem {
                                id: h["project_id"].as_str().unwrap_or("").to_string(),
                                slug: h["slug"].as_str().unwrap_or("").to_string(),
                                title: h["title"].as_str().unwrap_or("").to_string(),
                                author: h["author"].as_str().unwrap_or("").to_string(),
                                downloads: h["downloads"].as_u64().unwrap_or(0),
                                likes: h["follows"].as_u64().unwrap_or(0),
                                icon_url: icon,
                                banner_url: banner,
                                summary: h["description"].as_str().unwrap_or("").to_string(),
                                loader: detected_loader,
                                mc_version,
                                source: "modrinth".to_string(),
                                categories,
                            });
                        }
                    }
                }
            }
        }
    }

    // 2. CurseForge Modpacks
    if do_curseforge {
        let cf_api_key = std::env::var("CURSEFORGE_API_KEY").unwrap_or_else(|_| {
            "$2a$10$bL4bIL5pUWqfcO7KQtnMReakwtfHbNKh6v1uTpKlzhwoueEJQnPnm".to_string()
        });

        let mut req = client.get("https://api.curseforge.com/v1/mods/search")
            .header("x-api-key", &cf_api_key)
            .header("Accept", "application/json")
            .query(&[
                ("gameId", "432"),
                ("classId", "4471"),
                ("pageSize", &max_results.to_string()),
                ("sortField", "2"),
                ("sortOrder", "desc"),
            ]);

        if !q.is_empty() {
            req = req.query(&[("searchFilter", &q)]);
        }

        if let Ok(resp) = req.send().await {
            if resp.status().is_success() {
                if let Ok(json) = resp.json::<serde_json::Value>().await {
                    if let Some(items) = json["data"].as_array() {
                        for item in items {
                            let mut loader = "fabric".to_string();
                            let mut mc_version = "1.20.1".to_string();

                            if let Some(l_idx) = item["latestFilesIndexes"].as_array().and_then(|a| a.first()) {
                                if let Some(gv) = l_idx["gameVersion"].as_str() {
                                    mc_version = gv.to_string();
                                }
                                if let Some(m_type) = l_idx["modLoader"].as_u64().or_else(|| l_idx["modLoaderType"].as_u64()) {
                                    loader = match m_type {
                                        1 => "forge".to_string(),
                                        4 => "fabric".to_string(),
                                        5 => "quilt".to_string(),
                                        6 => "neoforge".to_string(),
                                        _ => "fabric".to_string(),
                                    };
                                }
                            }

                            let icon = item["logo"]["url"].as_str().unwrap_or("").to_string();
                            let banner = item["screenshots"]
                                .as_array()
                                .and_then(|a| a.first())
                                .and_then(|s| s["url"].as_str())
                                .unwrap_or(&icon)
                                .to_string();

                            let author = item["authors"]
                                .as_array()
                                .and_then(|a| a.first())
                                .and_then(|a| a["name"].as_str())
                                .unwrap_or("Inconnu")
                                .to_string();

                            let categories: Vec<String> = item["categories"]
                                .as_array()
                                .map(|arr| arr.iter().filter_map(|c| c["name"].as_str().map(String::from)).collect())
                                .unwrap_or_default();

                            results.push(ModpackCatalogItem {
                                id: item["id"].to_string(),
                                slug: item["slug"].as_str().unwrap_or("").to_string(),
                                title: item["name"].as_str().unwrap_or("").to_string(),
                                author,
                                downloads: item["downloadCount"].as_u64().unwrap_or(0),
                                likes: item["thumbsUpCount"].as_u64().unwrap_or(0),
                                icon_url: icon,
                                banner_url: banner,
                                summary: item["summary"].as_str().unwrap_or("").to_string(),
                                loader,
                                mc_version,
                                source: "curseforge".to_string(),
                                categories,
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

#[tauri::command]
pub async fn get_minecraft_mod_details_cmd(
    provider: String,
    mod_id: String,
) -> Result<serde_json::Value, String> {
    let client = Client::builder()
        .user_agent("LoomLauncher/0.1.0")
        .build()
        .unwrap_or_else(|_| Client::new());

    if provider.to_lowercase() == "curseforge" {
        let cf_api_key = std::env::var("CURSEFORGE_API_KEY").unwrap_or_else(|_| {
            "$2a$10$bL4bIL5pUWqfcO7KQtnMReakwtfHbNKh6v1uTpKlzhwoueEJQnPnm".to_string()
        });

        // 1. Mod info
        let mod_url = format!("https://api.curseforge.com/v1/mods/{}", mod_id);
        let mod_res = client.get(&mod_url)
            .header("x-api-key", &cf_api_key)
            .send()
            .await
            .map_err(|e| format!("CurseForge request error: {}", e))?;

        if !mod_res.status().is_success() {
            return Err(format!("CurseForge error status: {}", mod_res.status()));
        }

        let mod_json: serde_json::Value = mod_res.json().await.map_err(|e| e.to_string())?;
        let data = &mod_json["data"];

        // 2. Description (HTML)
        let desc_url = format!("https://api.curseforge.com/v1/mods/{}/description", mod_id);
        let desc_html = match client.get(&desc_url).header("x-api-key", &cf_api_key).send().await {
            Ok(r) if r.status().is_success() => {
                let d_json: serde_json::Value = r.json().await.unwrap_or_default();
                d_json["data"].as_str().unwrap_or("").to_string()
            }
            _ => data["summary"].as_str().unwrap_or("").to_string(),
        };

        // 3. Files / versions
        let files_url = format!("https://api.curseforge.com/v1/mods/{}/files?pageSize=50", mod_id);
        let mut versions = Vec::new();
        if let Ok(r) = client.get(&files_url).header("x-api-key", &cf_api_key).send().await {
            if r.status().is_success() {
                if let Ok(f_json) = r.json::<serde_json::Value>().await {
                    if let Some(files) = f_json["data"].as_array() {
                        for f in files {
                            let f_id = f["id"].to_string();
                            let f_name = f["fileName"].as_str().unwrap_or("").to_string();
                            let d_url = f["downloadUrl"].as_str()
                                .map(String::from)
                                .unwrap_or_else(|| {
                                    let id_num = f["id"].as_u64().unwrap_or(0);
                                    format!("https://edge.forgecdn.net/files/{}/{}/{}", id_num / 1000, id_num % 1000, f_name)
                                });

                            let mut gv_list = Vec::new();
                            let mut loaders = Vec::new();
                            if let Some(gvs) = f["gameVersions"].as_array() {
                                for gv in gvs {
                                    if let Some(s) = gv.as_str() {
                                        let lower = s.to_lowercase();
                                        if ["forge", "fabric", "neoforge", "quilt"].contains(&lower.as_str()) {
                                            if !loaders.contains(&lower) { loaders.push(lower); }
                                        } else if s.contains('.') {
                                            if !gv_list.contains(&s.to_string()) { gv_list.push(s.to_string()); }
                                        }
                                    }
                                }
                            }

                            let rel_type = match f["releaseType"].as_u64().unwrap_or(1) {
                                1 => "release",
                                2 => "beta",
                                3 => "alpha",
                                _ => "release",
                            };

                            versions.push(serde_json::json!({
                                "id": f_id,
                                "name": f["displayName"].as_str().unwrap_or(&f_name),
                                "version_number": f_name,
                                "release_type": rel_type,
                                "game_versions": gv_list,
                                "loaders": if loaders.is_empty() { vec!["forge".to_string(), "fabric".to_string()] } else { loaders },
                                "date_published": f["fileDate"],
                                "downloads": f["downloadCount"].as_u64().unwrap_or(0),
                                "filename": f_name,
                                "files": [{
                                    "id": f_id,
                                    "filename": f_name,
                                    "url": d_url,
                                    "size_bytes": f["fileLength"].as_u64().unwrap_or(0),
                                    "primary": true
                                }]
                            }));
                        }
                    }
                }
            }
        }

        let categories: Vec<String> = data["categories"].as_array()
            .map(|arr| arr.iter().filter_map(|c| c["name"].as_str().map(String::from)).collect())
            .unwrap_or_default();

        let gallery: Vec<serde_json::Value> = data["screenshots"].as_array()
            .map(|arr| arr.iter().map(|s| serde_json::json!({
                "url": s["url"].as_str().or_else(|| s["thumbnailUrl"].as_str()).unwrap_or(""),
                "title": s["title"].as_str().unwrap_or(""),
                "description": s["description"].as_str().unwrap_or("")
            })).collect())
            .unwrap_or_default();

        let author = data["authors"].as_array()
            .and_then(|a| a.first())
            .and_then(|a| a["name"].as_str())
            .unwrap_or("Inconnu")
            .to_string();

        Ok(serde_json::json!({
            "id": data["id"].to_string(),
            "slug": data["slug"].as_str().unwrap_or(""),
            "name": data["name"].as_str().unwrap_or(""),
            "summary": data["summary"].as_str().unwrap_or(""),
            "description": desc_html,
            "icon_url": data["logo"]["url"].as_str(),
            "banner_url": data["screenshots"].as_array().and_then(|a| a.first()).and_then(|s| s["url"].as_str()),
            "author": author,
            "authors": [{ "name": author }],
            "provider": "curseforge",
            "downloads": data["downloadCount"].as_u64().unwrap_or(0),
            "follows": data["thumbsUpCount"].as_u64().unwrap_or(0),
            "categories": categories,
            "loaders": ["fabric", "forge", "neoforge"],
            "game_versions": [],
            "gallery": gallery,
            "versions": versions
        }))
    } else {
        // Modrinth
        let proj_url = format!("https://api.modrinth.com/v2/project/{}", mod_id);
        let proj_res = client.get(&proj_url)
            .send()
            .await
            .map_err(|e| format!("Modrinth request error: {}", e))?;

        if !proj_res.status().is_success() {
            return Err(format!("Modrinth error status: {}", proj_res.status()));
        }

        let p: serde_json::Value = proj_res.json().await.map_err(|e| e.to_string())?;

        // Versions
        let ver_url = format!("https://api.modrinth.com/v2/project/{}/version", mod_id);
        let mut versions = Vec::new();
        if let Ok(v_res) = client.get(&ver_url).send().await {
            if v_res.status().is_success() {
                if let Ok(v_json) = v_res.json::<serde_json::Value>().await {
                    if let Some(v_list) = v_json.as_array() {
                        for v in v_list {
                            let f_first = v["files"].as_array().and_then(|a| a.first());
                            let f_name = f_first.and_then(|f| f["filename"].as_str()).unwrap_or("mod.jar").to_string();
                            let f_url = f_first.and_then(|f| f["url"].as_str()).unwrap_or("").to_string();
                            let f_size = f_first.and_then(|f| f["size"].as_u64()).unwrap_or(0);

                            versions.push(serde_json::json!({
                                "id": v["id"].as_str().unwrap_or(""),
                                "name": v["name"].as_str().unwrap_or(&f_name),
                                "version_number": v["version_number"].as_str().unwrap_or(""),
                                "release_type": v["version_type"].as_str().unwrap_or("release"),
                                "game_versions": v["game_versions"].as_array().unwrap_or(&vec![]),
                                "loaders": v["loaders"].as_array().unwrap_or(&vec![]),
                                "date_published": v["date_published"],
                                "downloads": v["downloads"].as_u64().unwrap_or(0),
                                "changelog": v["changelog"].as_str(),
                                "filename": f_name,
                                "files": [{
                                    "id": v["id"].as_str().unwrap_or(""),
                                    "filename": f_name,
                                    "url": f_url,
                                    "size_bytes": f_size,
                                    "primary": true
                                }]
                            }));
                        }
                    }
                }
            }
        }

        let gallery: Vec<serde_json::Value> = p["gallery"].as_array()
            .map(|arr| arr.iter().map(|g| serde_json::json!({
                "url": g["url"].as_str().or_else(|| g["raw_url"].as_str()).unwrap_or(""),
                "title": g["title"].as_str().unwrap_or(""),
                "description": g["description"].as_str().unwrap_or("")
            })).collect())
            .unwrap_or_default();

        let categories: Vec<String> = p["categories"].as_array()
            .map(|arr| arr.iter().filter_map(|c| c.as_str().map(String::from)).collect())
            .unwrap_or_default();

        let loaders: Vec<String> = p["loaders"].as_array()
            .map(|arr| arr.iter().filter_map(|c| c.as_str().map(String::from)).collect())
            .unwrap_or_default();

        let game_versions: Vec<String> = p["game_versions"].as_array()
            .map(|arr| arr.iter().filter_map(|c| c.as_str().map(String::from)).collect())
            .unwrap_or_default();

        Ok(serde_json::json!({
            "id": p["id"].as_str().unwrap_or(""),
            "slug": p["slug"].as_str().unwrap_or(""),
            "name": p["title"].as_str().unwrap_or(""),
            "summary": p["description"].as_str().unwrap_or(""),
            "description": p["body"].as_str().unwrap_or(""),
            "icon_url": p["icon_url"].as_str(),
            "banner_url": gallery.first().and_then(|g| g["url"].as_str()),
            "author": "Modrinth Creator",
            "provider": "modrinth",
            "downloads": p["downloads"].as_u64().unwrap_or(0),
            "follows": p["followers"].as_u64().unwrap_or(0),
            "categories": categories,
            "loaders": loaders,
            "game_versions": game_versions,
            "gallery": gallery,
            "versions": versions
        }))
    }
}

#[tauri::command]
pub async fn get_minecraft_version_changelog_cmd(
    provider: String,
    mod_id: String,
    file_id: String,
) -> Result<serde_json::Value, String> {
    let client = Client::builder()
        .user_agent("LoomLauncher/0.1.0")
        .build()
        .unwrap_or_else(|_| Client::new());

    if provider.to_lowercase() == "curseforge" {
        let cf_api_key = std::env::var("CURSEFORGE_API_KEY").unwrap_or_else(|_| {
            "$2a$10$bL4bIL5pUWqfcO7KQtnMReakwtfHbNKh6v1uTpKlzhwoueEJQnPnm".to_string()
        });
        let url = format!("https://api.curseforge.com/v1/mods/{}/files/{}/changelog", mod_id, file_id);
        let resp = client.get(&url).header("x-api-key", &cf_api_key).send().await
            .map_err(|e| e.to_string())?;
        if resp.status().is_success() {
            let json: serde_json::Value = resp.json().await.unwrap_or_default();
            let changelog = json["data"].as_str().unwrap_or("").to_string();
            return Ok(serde_json::json!({ "changelog": changelog }));
        }
        Ok(serde_json::json!({ "changelog": "" }))
    } else {
        let url = format!("https://api.modrinth.com/v2/version/{}", file_id);
        let resp = client.get(&url).send().await.map_err(|e| e.to_string())?;
        if resp.status().is_success() {
            let json: serde_json::Value = resp.json().await.unwrap_or_default();
            let changelog = json["changelog"].as_str().unwrap_or("").to_string();
            return Ok(serde_json::json!({ "changelog": changelog }));
        }
        Ok(serde_json::json!({ "changelog": "" }))
    }
}
