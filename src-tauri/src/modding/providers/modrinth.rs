use std::fs;
use std::path::{Path, PathBuf};
use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use serde_json::Value;

use super::super::adapter::{
    ModFileInfo, ModVersionInfo, ProviderKind, RemoteDependency, UnifiedModSearchResult,
};
use super::ModProvider;

pub struct ModrinthProvider {
    client: reqwest::Client,
    base_url: String,
}

impl Default for ModrinthProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ModrinthProvider {
    pub fn new() -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static("LoomLauncher/0.1.0 (contact: support@loom.games)"),
        );
        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            client,
            base_url: "https://api.modrinth.com/v2".to_string(),
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
        let p_type = match content_type {
            Some("resourcepack") => "resourcepack",
            Some("shader") => "shader",
            Some("modpack") => "modpack",
            _ => "mod",
        };
        let mut facets: Vec<Vec<String>> = vec![vec![format!("project_type:{}", p_type)]];

        if p_type == "mod" {
            if let Some(gv) = game_version {
                if !gv.is_empty() {
                    facets.push(vec![format!("versions:{}", gv)]);
                }
            }

            if let Some(ld) = loader {
                if !ld.is_empty() && ld != "vanilla" {
                    facets.push(vec![format!("categories:{}", ld.to_lowercase())]);
                }
            }
        }

        let facets_json = serde_json::to_string(&facets).unwrap_or_else(|_| "[]".to_string());
        let index = if query.trim().is_empty() { "downloads" } else { "relevance" };

        let url = format!("{}/search", self.base_url);
        let mut req = self
            .client
            .get(&url)
            .query(&[
                ("limit", &limit.to_string()),
                ("index", &index.to_string()),
                ("facets", &facets_json),
            ]);

        let q_trimmed = query.trim();
        if !q_trimmed.is_empty() {
            req = req.query(&[("query", q_trimmed)]);
        }

        let response = req
            .send()
            .await
            .map_err(|e| format!("Modrinth search error: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Modrinth API returned status {}", response.status()));
        }

        let body: Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse Modrinth search JSON: {}", e))?;

        let hits = body["hits"].as_array().cloned().unwrap_or_default();
        let mut results = Vec::new();

        for hit in hits {
            let id = hit["project_id"]
                .as_str()
                .unwrap_or("")
                .to_string();
            let slug = hit["slug"].as_str().unwrap_or("").to_string();
            let name = hit["title"].as_str().unwrap_or("").to_string();
            let summary = hit["description"].as_str().unwrap_or("").to_string();
            let author = hit["author"].as_str().unwrap_or("").to_string();
            let icon_url = hit["icon_url"].as_str().map(|s| s.to_string());
            let downloads = hit["downloads"].as_u64().unwrap_or(0);
            let latest_version = hit["latest_version"].as_str().map(|s| s.to_string());

            let categories = hit["categories"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();

            let loaders = hit["loaders"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();

            let game_versions = hit["versions"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();

            results.push(UnifiedModSearchResult {
                id,
                slug,
                name,
                summary,
                author,
                icon_url,
                downloads,
                provider: ProviderKind::Modrinth,
                latest_version,
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
impl ModProvider for ModrinthProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Modrinth
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
        let url = format!("{}/project/{}/version", self.base_url, remote_mod_id);
        let mut req = self.client.get(&url);

        if let Some(gv) = game_version {
            if !gv.is_empty() {
                req = req.query(&[("game_versions", format!("[\"{}\"]", gv))]);
            }
        }
        if let Some(ld) = loader {
            if !ld.is_empty() && ld != "vanilla" {
                req = req.query(&[("loaders", format!("[\"{}\"]", ld.to_lowercase()))]);
            }
        }

        let response = req
            .send()
            .await
            .map_err(|e| format!("Modrinth get_versions error: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Modrinth API error: status {}", response.status()));
        }

        let body: Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse version list: {}", e))?;

        let versions = body.as_array().cloned().unwrap_or_default();
        let mut results = Vec::new();

        for v in versions {
            let id = v["id"].as_str().unwrap_or("").to_string();
            let mod_id = v["project_id"].as_str().unwrap_or("").to_string();
            let name = v["name"].as_str().unwrap_or("").to_string();
            let version_number = v["version_number"].as_str().unwrap_or("").to_string();

            let game_versions = v["game_versions"]
                .as_array()
                .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
                .unwrap_or_default();

            let loaders = v["loaders"]
                .as_array()
                .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
                .unwrap_or_default();

            let mut files = Vec::new();
            if let Some(f_arr) = v["files"].as_array() {
                for f in f_arr {
                    let file_url = f["url"].as_str().unwrap_or("").to_string();
                    let filename = f["filename"].as_str().unwrap_or("").to_string();
                    let is_primary = f["primary"].as_bool().unwrap_or(false);
                    let size_bytes = f["size"].as_u64().unwrap_or(0);
                    let sha1 = f["hashes"]["sha1"].as_str().map(String::from);
                    let sha512 = f["hashes"]["sha512"].as_str().map(String::from);

                    files.push(ModFileInfo {
                        id: filename.clone(),
                        filename,
                        url: file_url,
                        size_bytes,
                        sha1,
                        sha512,
                        is_primary,
                    });
                }
            }

            let mut dependencies = Vec::new();
            if let Some(d_arr) = v["dependencies"].as_array() {
                for d in d_arr {
                    let dep_type = d["dependency_type"].as_str().unwrap_or("required").to_string();
                    let is_required = dep_type == "required";
                    let remote_mod_id = d["project_id"].as_str().unwrap_or("").to_string();
                    let remote_version_id = d["version_id"].as_str().map(String::from);

                    if !remote_mod_id.is_empty() {
                        dependencies.push(RemoteDependency {
                            remote_mod_id,
                            remote_version_id,
                            name: None,
                            is_required,
                            dependency_type: dep_type,
                        });
                    }
                }
            }

            results.push(ModVersionInfo {
                id,
                mod_id,
                name,
                version_number,
                game_versions,
                loaders,
                files,
                dependencies,
            });
        }

        Ok(results)
    }

    async fn resolve_dependencies(
        &self,
        _remote_mod_id: &str,
        remote_version_id: &str,
    ) -> Result<Vec<RemoteDependency>, String> {
        let url = format!("{}/version/{}", self.base_url, remote_version_id);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Modrinth resolve_dependencies error: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Modrinth version lookup returned {}", response.status()));
        }

        let body: Value = response
            .json()
            .await
            .map_err(|e| format!("Invalid JSON from version: {}", e))?;

        let mut deps = Vec::new();
        if let Some(arr) = body["dependencies"].as_array() {
            for d in arr {
                let dep_type = d["dependency_type"].as_str().unwrap_or("required").to_string();
                let is_required = dep_type == "required";
                let pid = d["project_id"].as_str().unwrap_or("").to_string();
                let vid = d["version_id"].as_str().map(String::from);

                if !pid.is_empty() {
                    // Fetch project name if possible
                    let name = match self.get_project_title(&pid).await {
                        Ok(t) => Some(t),
                        Err(_) => None,
                    };

                    deps.push(RemoteDependency {
                        remote_mod_id: pid,
                        remote_version_id: vid,
                        name,
                        is_required,
                        dependency_type: dep_type,
                    });
                }
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
            .map_err(|e| format!("Failed to download from {}: {}", url, e))?
            .bytes()
            .await
            .map_err(|e| format!("Failed to read bytes from {}: {}", url, e))?;

        fs::write(&target_path, &bytes).map_err(|e| format!("Failed to write file {}: {}", target_path.display(), e))?;
        Ok(target_path)
    }
}

impl ModrinthProvider {
    pub async fn get_project_title(&self, project_id_or_slug: &str) -> Result<String, String> {
        let url = format!("{}/project/{}", self.base_url, project_id_or_slug);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            return Ok(project_id_or_slug.to_string());
        }

        let data: Value = resp.json().await.map_err(|e| e.to_string())?;
        let title = data["title"].as_str().unwrap_or(project_id_or_slug).to_string();
        Ok(title)
    }
}
