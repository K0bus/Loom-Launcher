use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha1::{Digest, Sha1};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryDownload {
    pub path: String,
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionMetadata {
    pub id: String,
    pub main_class: String,
    pub client_jar_url: String,
    pub client_jar_sha1: String,
    pub asset_index_url: String,
    pub asset_index_id: String,
    pub asset_index_sha1: String,
    pub libraries: Vec<LibraryDownload>,
    pub minecraft_arguments: Option<String>,
    pub java_version_major: u32,
}

pub fn get_shared_common_dir() -> PathBuf {
    let base = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string())).join(".local/share"))
        .join("loom")
        .join("common");
    let _ = fs::create_dir_all(&base);
    base
}

pub fn get_shared_assets_dir() -> PathBuf {
    let base = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string())).join(".local/share"))
        .join("loom")
        .join("common")
        .join("assets");
    let _ = fs::create_dir_all(&base);
    base
}

pub fn get_shared_libraries_dir() -> PathBuf {
    let base = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string())).join(".local/share"))
        .join("loom")
        .join("common")
        .join("libraries");
    let _ = fs::create_dir_all(&base);
    base
}

pub fn get_shared_versions_dir() -> PathBuf {
    let base = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string())).join(".local/share"))
        .join("loom")
        .join("common")
        .join("versions");
    let _ = fs::create_dir_all(&base);
    base
}

pub fn verify_sha1(bytes: &[u8], expected_hash: &str) -> bool {
    if expected_hash.is_empty() {
        return true;
    }
    let mut hasher = Sha1::new();
    hasher.update(bytes);
    let result = hasher.finalize();
    let calculated = format!("{:x}", result);
    calculated.eq_ignore_ascii_case(expected_hash)
}

pub async fn fetch_version_manifest(client: &Client) -> Result<Value, String> {
    let url = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch Mojang version manifest: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Mojang manifest error status: {}", resp.status()));
    }

    resp.json().await.map_err(|e| e.to_string())
}

pub async fn get_version_metadata(client: &Client, version_id: &str) -> Result<VersionMetadata, String> {
    let manifest = fetch_version_manifest(client).await?;
    let versions = manifest["versions"].as_array().ok_or("Invalid versions array")?;

    let version_entry = versions
        .iter()
        .find(|v| v["id"].as_str() == Some(version_id))
        .ok_or_else(|| format!("Minecraft version '{}' not found in manifest", version_id))?;

    let version_url = version_entry["url"]
        .as_str()
        .ok_or("Missing version JSON url")?;

    let v_resp = client
        .get(version_url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch version JSON from {}: {}", version_url, e))?;

    let v_json: Value = v_resp.json().await.map_err(|e| e.to_string())?;

    let main_class = v_json["mainClass"]
        .as_str()
        .unwrap_or("net.minecraft.client.main.Main")
        .to_string();

    let client_jar_url = v_json["downloads"]["client"]["url"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let client_jar_sha1 = v_json["downloads"]["client"]["sha1"]
        .as_str()
        .unwrap_or("")
        .to_string();

    let asset_index_url = v_json["assetIndex"]["url"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let asset_index_id = v_json["assetIndex"]["id"]
        .as_str()
        .unwrap_or(version_id)
        .to_string();
    let asset_index_sha1 = v_json["assetIndex"]["sha1"]
        .as_str()
        .unwrap_or("")
        .to_string();

    let java_version_major = v_json["javaVersion"]["majorVersion"]
        .as_u64()
        .unwrap_or_else(|| {
            // Default inference: 1.20.5+ -> 21, 1.17+ -> 17, <= 1.16.5 -> 8
            if version_id.starts_with("1.21") || version_id.starts_with("1.20.5") || version_id.starts_with("1.20.6") {
                21
            } else if version_id.starts_with("1.17") || version_id.starts_with("1.18") || version_id.starts_with("1.19") || version_id.starts_with("1.20") {
                17
            } else {
                8
            }
        }) as u32;

    let minecraft_arguments = v_json["minecraftArguments"].as_str().map(String::from);

    // Extract libraries applicable to Linux
    let mut libraries = Vec::new();
    if let Some(lib_arr) = v_json["libraries"].as_array() {
        for lib in lib_arr {
            // Check OS rules
            if let Some(rules) = lib["rules"].as_array() {
                let mut allowed = false;
                for rule in rules {
                    let action = rule["action"].as_str().unwrap_or("");
                    let os_name = rule["os"]["name"].as_str();

                    if action == "allow" {
                        if os_name.is_none() || os_name == Some("linux") {
                            allowed = true;
                        }
                    } else if action == "disallow" {
                        if os_name == Some("linux") {
                            allowed = false;
                        }
                    }
                }
                if !allowed {
                    continue;
                }
            }

            if let Some(artifact) = lib["downloads"]["artifact"].as_object() {
                let path = artifact.get("path").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let url = artifact.get("url").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let sha1 = artifact.get("sha1").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let size = artifact.get("size").and_then(|v| v.as_u64()).unwrap_or(0);

                if !url.is_empty() && !path.is_empty() {
                    libraries.push(LibraryDownload {
                        path,
                        sha1,
                        size,
                        url,
                    });
                }
            }
        }
    }

    Ok(VersionMetadata {
        id: version_id.to_string(),
        main_class,
        client_jar_url,
        client_jar_sha1,
        asset_index_url,
        asset_index_id,
        asset_index_sha1,
        libraries,
        minecraft_arguments,
        java_version_major,
    })
}

pub async fn download_client_jar(client: &Client, meta: &VersionMetadata) -> Result<PathBuf, String> {
    let versions_dir = get_shared_versions_dir().join(&meta.id);
    fs::create_dir_all(&versions_dir).map_err(|e| e.to_string())?;

    let jar_path = versions_dir.join(format!("{}.jar", meta.id));
    if jar_path.exists() {
        if let Ok(bytes) = fs::read(&jar_path) {
            if verify_sha1(&bytes, &meta.client_jar_sha1) {
                return Ok(jar_path);
            }
        }
    }

    if meta.client_jar_url.is_empty() {
        return Err("Client jar URL is missing from version metadata".to_string());
    }

    let bytes = client
        .get(&meta.client_jar_url)
        .send()
        .await
        .map_err(|e| format!("Failed to download client jar: {}", e))?
        .bytes()
        .await
        .map_err(|e| format!("Failed reading client jar bytes: {}", e))?;

    if !verify_sha1(&bytes, &meta.client_jar_sha1) {
        return Err("Client jar SHA-1 verification failed".to_string());
    }

    fs::write(&jar_path, &bytes).map_err(|e| e.to_string())?;
    Ok(jar_path)
}

pub async fn download_libraries(client: &Client, meta: &VersionMetadata) -> Result<Vec<PathBuf>, String> {
    let lib_base = get_shared_libraries_dir();
    let mut paths = Vec::new();

    for lib in &meta.libraries {
        let dest = lib_base.join(&lib.path);
        if let Some(parent) = dest.parent() {
            let _ = fs::create_dir_all(parent);
        }

        let needs_download = if dest.exists() {
            if let Ok(b) = fs::read(&dest) {
                !verify_sha1(&b, &lib.sha1)
            } else {
                true
            }
        } else {
            true
        };

        if needs_download {
            if let Ok(resp) = client.get(&lib.url).send().await {
                if resp.status().is_success() {
                    if let Ok(bytes) = resp.bytes().await {
                        if verify_sha1(&bytes, &lib.sha1) {
                            let _ = fs::write(&dest, &bytes);
                        }
                    }
                }
            }
        }

        if dest.exists() {
            paths.push(dest);
        }
    }

    Ok(paths)
}

pub async fn download_assets(client: &Client, meta: &VersionMetadata) -> Result<PathBuf, String> {
    let assets_dir = get_shared_assets_dir();
    let indexes_dir = assets_dir.join("indexes");
    let objects_dir = assets_dir.join("objects");
    let _ = fs::create_dir_all(&indexes_dir);
    let _ = fs::create_dir_all(&objects_dir);

    let index_file = indexes_dir.join(format!("{}.json", meta.asset_index_id));
    let mut index_json_str = String::new();

    if index_file.exists() {
        if let Ok(content) = fs::read_to_string(&index_file) {
            if verify_sha1(content.as_bytes(), &meta.asset_index_sha1) {
                index_json_str = content;
            }
        }
    }

    if index_json_str.is_empty() && !meta.asset_index_url.is_empty() {
        let bytes = client
            .get(&meta.asset_index_url)
            .send()
            .await
            .map_err(|e| format!("Failed to download asset index: {}", e))?
            .bytes()
            .await
            .map_err(|e| e.to_string())?;

        let _ = fs::write(&index_file, &bytes);
        index_json_str = String::from_utf8_lossy(&bytes).to_string();
    }

    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&index_json_str) {
        if let Some(objects) = parsed.get("objects").and_then(|o| o.as_object()) {
            let mut missing: Vec<(String, PathBuf)> = Vec::new();
            for (_name, obj) in objects {
                if let Some(hash) = obj.get("hash").and_then(|h| h.as_str()) {
                    if hash.len() >= 2 {
                        let prefix = &hash[..2];
                        let dest_path = objects_dir.join(prefix).join(hash);
                        if !dest_path.exists() {
                            let url = format!("https://resources.download.minecraft.net/{}/{}", prefix, hash);
                            missing.push((url, dest_path));
                        }
                    }
                }
            }

            if !missing.is_empty() {
                // Download missing objects concurrently
                let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(20));
                let mut handles = Vec::new();

                for (url, dest_path) in missing {
                    let sem = semaphore.clone();
                    let cl = client.clone();
                    handles.push(tokio::spawn(async move {
                        let _permit = sem.acquire().await;
                        if let Some(parent) = dest_path.parent() {
                            let _ = fs::create_dir_all(parent);
                        }
                        if let Ok(res) = cl.get(&url).send().await {
                            if let Ok(bytes) = res.bytes().await {
                                let _ = fs::write(&dest_path, &bytes);
                            }
                        }
                    }));
                }

                for h in handles {
                    let _ = h.await;
                }
            }
        }
    }

    Ok(assets_dir)
}
