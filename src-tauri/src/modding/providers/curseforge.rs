use std::fs;
use std::path::{Path, PathBuf};
use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue};
use serde_json::Value;

use super::super::adapter::{
    ModFileInfo, ModVersionInfo, ProviderKind, RemoteDependency, UnifiedModSearchResult,
};
use super::ModProvider;

pub struct CurseForgeProvider {
    client: reqwest::Client,
    base_url: String,
}

impl Default for CurseForgeProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl CurseForgeProvider {
    pub fn new() -> Self {
        let api_key = std::env::var("CURSEFORGE_API_KEY").unwrap_or_else(|_| {
            // Standard CurseForge Core API key for open-source launchers
            "$2a$10$bL4bIL5pUWqfcO7KQtnMReakwtfHbNKh6v1uTpKlzhwoueEJQnPnm".to_string()
        });

        let mut headers = HeaderMap::new();
        if let Ok(val) = HeaderValue::from_str(&api_key) {
            headers.insert("x-api-key", val);
        }

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            client,
            base_url: "https://api.curseforge.com/v1".to_string(),
        }
    }

    fn loader_to_type_int(loader: Option<&str>) -> Option<u32> {
        match loader.map(|s| s.to_lowercase()).as_deref() {
            Some("forge") => Some(1),
            Some("fabric") => Some(4),
            Some("neoforge") => Some(6),
            Some("quilt") => Some(5),
            _ => None,
        }
    }

    pub async fn search_content(
        &self,
        query: &str,
        _game_id: &str,
        game_version: Option<&str>,
        loader: Option<&str>,
        content_type: Option<&str>,
        limit: usize,
    ) -> Result<Vec<UnifiedModSearchResult>, String> {
        let p_type = content_type.unwrap_or("mod");
        let class_id = match p_type {
            "resourcepack" => "12",
            "shader" => "6552",
            "modpack" => "4471",
            _ => "6",
        };

        let url = format!("{}/mods/search", self.base_url);
        let mut req = self.client.get(&url).query(&[
            ("gameId", "432"),    // Minecraft
            ("classId", class_id),
            ("pageSize", &limit.to_string()),
            ("sortField", "2"),   // Popularity
            ("sortOrder", "desc"),
        ]);

        let q_trimmed = query.trim();
        if !q_trimmed.is_empty() {
            req = req.query(&[("searchFilter", q_trimmed)]);
        }

        if p_type == "mod" {
            if let Some(gv) = game_version {
                if !gv.is_empty() {
                    req = req.query(&[("gameVersion", gv)]);
                }
            }

            if let Some(type_int) = Self::loader_to_type_int(loader) {
                req = req.query(&[("modLoaderType", &type_int.to_string())]);
            }
        }

        let resp = req
            .send()
            .await
            .map_err(|e| format!("CurseForge search error: {}", e))?;

        if !resp.status().is_success() {
            return Err(format!("CurseForge API error status: {}", resp.status()));
        }

        let body: Value = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse CurseForge JSON: {}", e))?;

        let data = body["data"].as_array().cloned().unwrap_or_default();
        let mut results = Vec::new();

        for item in data {
            let id = item["id"].as_i64().unwrap_or(0).to_string();
            let slug = item["slug"].as_str().unwrap_or("").to_string();
            let name = item["name"].as_str().unwrap_or("").to_string();
            let summary = item["summary"].as_str().unwrap_or("").to_string();
            let downloads = item["downloadCount"].as_u64().unwrap_or(0);

            let author = item["authors"]
                .as_array()
                .and_then(|a| a.first())
                .and_then(|a| a["name"].as_str())
                .unwrap_or("Inconnu")
                .to_string();

            let icon_url = item["logo"]["url"].as_str().map(|s| s.to_string());

            let categories = item["categories"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|c| c["name"].as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();

            // Extract game versions and loaders from latestFilesIndexes
            let mut game_versions = Vec::new();
            let mut loaders = Vec::new();

            if let Some(indexes) = item["latestFilesIndexes"].as_array() {
                for idx in indexes {
                    if let Some(gv) = idx["gameVersion"].as_str() {
                        if !game_versions.contains(&gv.to_string()) {
                            game_versions.push(gv.to_string());
                        }
                    }
                    if let Some(type_id) = idx["modLoaderType"].as_u64() {
                        let l_str = match type_id {
                            1 => "forge",
                            4 => "fabric",
                            5 => "quilt",
                            6 => "neoforge",
                            _ => "",
                        };
                        if !l_str.is_empty() && !loaders.contains(&l_str.to_string()) {
                            loaders.push(l_str.to_string());
                        }
                    }
                }
            }

            results.push(UnifiedModSearchResult {
                id,
                slug,
                name,
                summary,
                author,
                icon_url,
                downloads,
                provider: ProviderKind::CurseForge,
                latest_version: None,
                categories,
                loaders,
                game_versions,
                project_type: Some(p_type.to_string()),
            });
        }

        Ok(results)
    }
}

#[async_trait]
impl ModProvider for CurseForgeProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::CurseForge
    }

    async fn search(
        &self,
        query: &str,
        game_id: &str,
        game_version: Option<&str>,
        loader: Option<&str>,
        limit: usize,
    ) -> Result<Vec<UnifiedModSearchResult>, String> {
        self.search_content(query, game_id, game_version, loader, None, limit).await
    }

    async fn get_versions(
        &self,
        remote_mod_id: &str,
        game_version: Option<&str>,
        loader: Option<&str>,
    ) -> Result<Vec<ModVersionInfo>, String> {
        let url = format!("{}/mods/{}/files", self.base_url, remote_mod_id);
        let mut req = self.client.get(&url);

        if let Some(gv) = game_version {
            if !gv.is_empty() {
                req = req.query(&[("gameVersion", gv)]);
            }
        }
        if let Some(type_int) = Self::loader_to_type_int(loader) {
            req = req.query(&[("modLoaderType", &type_int.to_string())]);
        }

        let resp = req
            .send()
            .await
            .map_err(|e| format!("CurseForge get_versions error: {}", e))?;

        if !resp.status().is_success() {
            return Err(format!("CurseForge API files error status: {}", resp.status()));
        }

        let body: Value = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse CurseForge files JSON: {}", e))?;

        let files = body["data"].as_array().cloned().unwrap_or_default();
        let mut results = Vec::new();

        for f in files {
            let id = f["id"].as_i64().unwrap_or(0).to_string();
            let name = f["displayName"].as_str().unwrap_or("").to_string();
            let filename = f["fileName"].as_str().unwrap_or("").to_string();
            let download_url = f["downloadUrl"].as_str().unwrap_or("").to_string();
            let file_length = f["fileLength"].as_u64().unwrap_or(0);

            let game_versions = f["gameVersions"]
                .as_array()
                .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
                .unwrap_or_default();

            let mut dependencies = Vec::new();
            if let Some(deps_arr) = f["dependencies"].as_array() {
                for d in deps_arr {
                    let rel_type = d["relationType"].as_u64().unwrap_or(0);
                    let dep_mod_id = d["modId"].as_i64().unwrap_or(0).to_string();

                    // Relation 3 = Required, 2 = Optional
                    let is_required = rel_type == 3;
                    let dep_type_str = match rel_type {
                        3 => "required",
                        2 => "optional",
                        1 => "embedded",
                        5 => "incompatible",
                        _ => "dependency",
                    };

                    dependencies.push(RemoteDependency {
                        remote_mod_id: dep_mod_id,
                        remote_version_id: None,
                        name: None,
                        is_required,
                        dependency_type: dep_type_str.to_string(),
                    });
                }
            }

            let file_info = ModFileInfo {
                id: id.clone(),
                filename,
                url: download_url,
                size_bytes: file_length,
                sha1: None,
                sha512: None,
                is_primary: true,
            };

            results.push(ModVersionInfo {
                id,
                mod_id: remote_mod_id.to_string(),
                name: name.clone(),
                version_number: name,
                game_versions,
                loaders: Vec::new(),
                files: vec![file_info],
                dependencies,
            });
        }

        Ok(results)
    }

    async fn resolve_dependencies(
        &self,
        remote_mod_id: &str,
        remote_file_id: &str,
    ) -> Result<Vec<RemoteDependency>, String> {
        let url = format!("{}/mods/{}/files/{}", self.base_url, remote_mod_id, remote_file_id);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("CurseForge file lookup error: {}", e))?;

        if !resp.status().is_success() {
            return Err(format!("CurseForge API file status: {}", resp.status()));
        }

        let body: Value = resp.json().await.map_err(|e| e.to_string())?;
        let mut deps = Vec::new();

        if let Some(arr) = body["data"]["dependencies"].as_array() {
            for d in arr {
                let rel_type = d["relationType"].as_u64().unwrap_or(0);
                let dep_mod_id = d["modId"].as_i64().unwrap_or(0).to_string();
                let is_required = rel_type == 3;

                // Lookup name of dep_mod_id
                let name = match self.get_mod_name(&dep_mod_id).await {
                    Ok(n) => Some(n),
                    Err(_) => None,
                };

                deps.push(RemoteDependency {
                    remote_mod_id: dep_mod_id,
                    remote_version_id: None,
                    name,
                    is_required,
                    dependency_type: if is_required { "required".to_string() } else { "optional".to_string() },
                });
            }
        }

        Ok(deps)
    }

    async fn download_file(
        &self,
        url: &str,
        destination_dir: &Path,
        expected_filename: &str,
    ) -> Result<PathBuf, String> {
        fs::create_dir_all(destination_dir).map_err(|e| e.to_string())?;
        let target_path = destination_dir.join(expected_filename);

        if target_path.exists() {
            return Ok(target_path);
        }

        let bytes = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| format!("Failed to download CurseForge file from {}: {}", url, e))?
            .bytes()
            .await
            .map_err(|e| format!("Failed reading bytes: {}", e))?;

        fs::write(&target_path, &bytes).map_err(|e| format!("Failed writing file {}: {}", target_path.display(), e))?;
        Ok(target_path)
    }
}

impl CurseForgeProvider {
    pub async fn get_mod_name(&self, mod_id: &str) -> Result<String, String> {
        let url = format!("{}/mods/{}", self.base_url, mod_id);
        let resp = self.client.get(&url).send().await.map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Ok(format!("Mod #{}", mod_id));
        }
        let body: Value = resp.json().await.map_err(|e| e.to_string())?;
        let name = body["data"]["name"].as_str().unwrap_or(mod_id).to_string();
        Ok(name)
    }
}
