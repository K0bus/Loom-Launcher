use std::path::{Path, PathBuf};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use crate::db;
use crate::modding::adapter::GameAdapter;

use super::adapter::MinecraftAdapter;
use super::instance::{get_instance_minecraft_dir, default_jvm_args};
use super::manifest::{
    download_assets, download_client_jar, download_libraries, get_version_metadata,
};
use super::runtime::{ensure_java_runtime, validate_java_binary};

async fn setup_fabric(
    client: &Client,
    mc_version: &str,
    loader_version: Option<&str>,
) -> Result<(String, Vec<PathBuf>), String> {
    let loader_ver = loader_version.unwrap_or("0.16.9");
    let url = format!(
        "https://meta.fabricmc.net/v2/versions/loader/{}/{}/profile/json",
        mc_version, loader_ver
    );
    let res = client.get(&url).send().await.map_err(|e| e.to_string())?;
    if !res.status().is_success() {
        return Ok(("net.fabricmc.loader.impl.launch.knot.KnotClient".to_string(), vec![]));
    }
    let profile: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
    let main_class = profile
        .get("mainClass")
        .and_then(|m| m.as_str())
        .unwrap_or("net.fabricmc.loader.impl.launch.knot.KnotClient")
        .to_string();

    let lib_dir = super::manifest::get_shared_libraries_dir();
    let mut fabric_libs = Vec::new();

    if let Some(libs) = profile.get("libraries").and_then(|l| l.as_array()) {
        for lib in libs {
            if let Some(name) = lib.get("name").and_then(|n| n.as_str()) {
                let parts: Vec<&str> = name.split(':').collect();
                if parts.len() >= 3 {
                    let grp = parts[0].replace('.', "/");
                    let art = parts[1];
                    let ver = parts[2];
                    let rel_path = format!("{}/{}/{}/{}-{}.jar", grp, art, ver, art, ver);
                    let dest_path = lib_dir.join(&rel_path);
                    fabric_libs.push(dest_path.clone());

                    if !dest_path.exists() {
                        if let Some(parent) = dest_path.parent() {
                            let _ = std::fs::create_dir_all(parent);
                        }
                        let base_url = lib.get("url").and_then(|u| u.as_str()).unwrap_or("https://maven.fabricmc.net/");
                        let dl_url = format!("{}{}", base_url, rel_path);
                        if let Ok(dl_res) = client.get(&dl_url).send().await {
                            if let Ok(bytes) = dl_res.bytes().await {
                                let _ = std::fs::write(&dest_path, &bytes);
                            }
                        }
                    }
                }
            }
        }
    }

    Ok((main_class, fabric_libs))
}

async fn setup_neoforge(
    client: &Client,
    java_bin: &Path,
    mc_version: &str,
    loader_version: Option<&str>,
    client_jar: &Path,
) -> Result<(String, Vec<String>, Vec<String>, Vec<PathBuf>), String> {
    let loader_ver = loader_version.unwrap_or("21.1.266");
    let common_dir = super::manifest::get_shared_common_dir();
    let versions_dir = super::manifest::get_shared_versions_dir();
    let libraries_dir = super::manifest::get_shared_libraries_dir();
    let nf_ver_name = format!("neoforge-{}", loader_ver);
    let nf_json_path = versions_dir.join(&nf_ver_name).join(format!("{}.json", &nf_ver_name));

    if !nf_json_path.exists() {
        let lp_path = common_dir.join("launcher_profiles.json");
        if !lp_path.exists() {
            let _ = std::fs::write(&lp_path, r#"{"profiles":{}}"#);
        }

        let v_dir = versions_dir.join(mc_version);
        let _ = std::fs::create_dir_all(&v_dir);
        let v_json_path = v_dir.join(format!("{}.json", mc_version));
        if !v_json_path.exists() {
            if let Ok(m_res) = client.get("https://piston-meta.mojang.com/mc/game/version_manifest_v2.json").send().await {
                if let Ok(m_data) = m_res.json::<serde_json::Value>().await {
                    if let Some(versions) = m_data.get("versions").and_then(|v| v.as_array()) {
                        if let Some(entry) = versions.iter().find(|v| v.get("id").and_then(|id| id.as_str()) == Some(mc_version)) {
                            if let Some(url) = entry.get("url").and_then(|u| u.as_str()) {
                                if let Ok(v_res) = client.get(url).send().await {
                                    if let Ok(bytes) = v_res.bytes().await {
                                        let _ = std::fs::write(&v_json_path, &bytes);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        let v_jar_path = v_dir.join(format!("{}.jar", mc_version));
        if !v_jar_path.exists() && client_jar.exists() {
            let _ = std::fs::copy(client_jar, &v_jar_path);
        }

        let installer_path = versions_dir.join(format!("{}-installer.jar", &nf_ver_name));
        if !installer_path.exists() {
            let installer_url = format!(
                "https://maven.neoforged.net/releases/net/neoforged/neoforge/{}/neoforge-{}-installer.jar",
                loader_ver, loader_ver
            );
            let res = client.get(&installer_url).send().await.map_err(|e| format!("Failed to download NeoForge installer: {}", e))?;
            let bytes = res.bytes().await.map_err(|e| e.to_string())?;
            std::fs::write(&installer_path, &bytes).map_err(|e| e.to_string())?;
        }

        let mut install_cmd = tokio::process::Command::new(java_bin);
        install_cmd
            .arg("-jar")
            .arg(&installer_path)
            .arg("--install-client")
            .arg(&common_dir);
        let out = install_cmd.output().await.map_err(|e| format!("Failed to run NeoForge installer: {}", e))?;
        if !out.status.success() {
            return Err(format!("NeoForge installer failed: {}", String::from_utf8_lossy(&out.stderr)));
        }
    }

    if !nf_json_path.exists() {
        return Err(format!("NeoForge version JSON missing after installation at {}", nf_json_path.display()));
    }

    let content = std::fs::read_to_string(&nf_json_path).map_err(|e| e.to_string())?;
    let nf_json: serde_json::Value = serde_json::from_str(&content).map_err(|e| e.to_string())?;

    let main_class = nf_json
        .get("mainClass")
        .and_then(|m| m.as_str())
        .unwrap_or("cpw.mods.bootstraplauncher.BootstrapLauncher")
        .to_string();

    let mut extra_jvm_args: Vec<String> = Vec::new();
    if let Some(jvm_arr) = nf_json.get("arguments").and_then(|a| a.get("jvm")).and_then(|j| j.as_array()) {
        for raw in jvm_arr {
            if let Some(raw_str) = raw.as_str() {
                let mut val = raw_str
                    .replace("${library_directory}", &libraries_dir.to_string_lossy())
                    .replace("${classpath_separator}", ":")
                    .replace("${version_name}", &nf_ver_name);
                if val.starts_with("-DignoreList=") {
                    val.push_str(&format!(",{},minecraft-{}-client.jar", mc_version, mc_version));
                }
                extra_jvm_args.push(val);
            }
        }
    }

    let mut extra_game_args: Vec<String> = Vec::new();
    if let Some(game_arr) = nf_json.get("arguments").and_then(|a| a.get("game")).and_then(|g| g.as_array()) {
        for raw in game_arr {
            if let Some(raw_str) = raw.as_str() {
                extra_game_args.push(raw_str.to_string());
            }
        }
    }

    let mut nf_libraries: Vec<PathBuf> = Vec::new();
    if let Some(libs) = nf_json.get("libraries").and_then(|l| l.as_array()) {
        for lib in libs {
            if let Some(p) = lib.get("downloads").and_then(|d| d.get("artifact")).and_then(|a| a.get("path")).and_then(|p| p.as_str()) {
                let lib_path = libraries_dir.join(p);
                if lib_path.exists() {
                    nf_libraries.push(lib_path);
                }
            } else if let Some(name) = lib.get("name").and_then(|n| n.as_str()) {
                let parts: Vec<&str> = name.split(':').collect();
                if parts.len() >= 3 {
                    let grp = parts[0].replace('.', "/");
                    let art = parts[1];
                    let ver = parts[2];
                    let classifier = if parts.len() >= 4 { format!("-{}", parts[3]) } else { String::new() };
                    let rel = format!("{}/{}/{}/{}-{}{}.jar", grp, art, ver, art, ver, classifier);
                    let lib_path = libraries_dir.join(rel);
                    if lib_path.exists() {
                        nf_libraries.push(lib_path);
                    }
                }
            }
        }
    }

    Ok((main_class, extra_jvm_args, extra_game_args, nf_libraries))
}

pub fn get_required_java_version(mc_version: &str) -> u32 {
    let parts: Vec<u32> = mc_version
        .split('.')
        .filter_map(|p| p.parse::<u32>().ok())
        .collect();

    if parts.len() >= 2 {
        let minor = parts[1];
        let patch = if parts.len() >= 3 { parts[2] } else { 0 };

        if minor < 17 {
            8
        } else if minor < 20 || (minor == 20 && patch < 5) {
            17
        } else {
            21
        }
    } else {
        17
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinecraftLogEvent {
    pub instance_id: String,
    pub timestamp: String,
    pub level: String, // 'INFO' | 'WARN' | 'ERROR' | 'DEBUG'
    pub source: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinecraftExitEvent {
    pub instance_id: String,
    pub exit_code: Option<i32>,
}

fn parse_log_line(raw: &str) -> (String, String, String) {
    // Standard format: [HH:mm:ss] [Source/LEVEL]: Message
    let mut level = "INFO".to_string();
    let mut source = "Minecraft".to_string();
    let mut message = raw.to_string();

    if let Some(first_close) = raw.find(']') {
        let remainder = &raw[first_close + 1..].trim();
        if remainder.starts_with('[') {
            if let Some(second_close) = remainder.find(']') {
                let tag = &remainder[1..second_close];
                if let Some(slash_idx) = tag.find('/') {
                    source = tag[..slash_idx].trim().to_string();
                    let lvl = tag[slash_idx + 1..].trim().to_uppercase();
                    if lvl.contains("WARN") {
                        level = "WARN".to_string();
                    } else if lvl.contains("ERROR") || lvl.contains("FATAL") {
                        level = "ERROR".to_string();
                    } else if lvl.contains("DEBUG") || lvl.contains("TRACE") {
                        level = "DEBUG".to_string();
                    }
                } else {
                    source = tag.to_string();
                }

                if let Some(colon) = remainder[second_close + 1..].find(':') {
                    message = remainder[second_close + 1 + colon + 1..].trim().to_string();
                } else {
                    message = remainder[second_close + 1..].trim().to_string();
                }
            }
        }
    } else if raw.to_uppercase().contains("ERROR") || raw.to_uppercase().contains("EXCEPTION") {
        level = "ERROR".to_string();
    } else if raw.to_uppercase().contains("WARN") {
        level = "WARN".to_string();
    }

    (level, source, message)
}

/// Destination des événements du jeu (console + fin de processus).
/// Implémentée pour `tauri::AppHandle` (commands/minecraft.rs) et par le front natif Slint.
pub trait LaunchEvents: Clone + Send + Sync + 'static {
    fn emit_log(&self, evt: MinecraftLogEvent);
    fn emit_exit(&self, evt: MinecraftExitEvent);
}

pub async fn launch_instance<E: LaunchEvents>(
    app_handle: E,
    instance_id: String,
    allocated_ram_gb: Option<u32>,
) -> Result<(), String> {
    let client = Client::new();

    // 1. Fetch instance info from DB
    let conn = db::open_db().map_err(|e| e.to_string())?;
    let inst_opt = db::get_minecraft_instance(&conn, &instance_id).map_err(|e| e.to_string())?;

    let (game_version, modloader, loader_version, min_ram_mb, max_ram_mb, java_path, custom_jvm_args) = match inst_opt {
        Some(i) => {
            let jargs: Vec<String> = serde_json::from_str(&i.jvm_args).unwrap_or_else(|_| default_jvm_args());
            (i.mc_version, i.modloader, i.loader_version, i.min_ram_mb, i.max_ram_mb, i.java_path, jargs)
        }
        None => {
            let profiles = db::get_profiles(&conn, Some("minecraft")).unwrap_or_default();
            let p = profiles.iter().find(|p| p.id == instance_id).cloned();
            let (gv, ml) = match p {
                Some(p) => (p.game_version, p.modloader.unwrap_or_else(|| "fabric".to_string())),
                None => ("1.20.1".to_string(), "fabric".to_string()),
            };
            let ram = allocated_ram_gb.unwrap_or(4) * 1024;
            (gv, ml, None, 2048, ram, None, default_jvm_args())
        }
    };

    // 2. Fetch version manifest & metadata
    let version_meta = get_version_metadata(&client, &game_version).await?;
    let req_java = get_required_java_version(&game_version);

    // 3. Ensure Java Runtime is provisioned or custom java path
    let java_bin = if let Some(ref custom_j) = java_path {
        let p = std::path::PathBuf::from(custom_j);
        if validate_java_binary(&p) {
            p
        } else {
            ensure_java_runtime(&client, req_java).await?
        }
    } else {
        ensure_java_runtime(&client, req_java).await?
    };

    // 4. Download Client Jar, Libraries, and Assets
    let client_jar = download_client_jar(&client, &version_meta).await?;
    let libraries = download_libraries(&client, &version_meta).await?;
    let assets_dir = download_assets(&client, &version_meta).await?;

    // 5. Deploy mods via symlinks into instance directory
    let adapter = MinecraftAdapter::new();
    adapter.deploy_profile(&instance_id)?;

    let mc_dir = get_instance_minecraft_dir(&instance_id);
    let _ = std::fs::create_dir_all(&mc_dir);

    // 6. Build Classpath and Loader Configuration
    let mut cp_entries: Vec<String> = libraries
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();

    let (main_class, extra_jvm_args, extra_game_args) = if modloader == "neoforge" {
        let (mc, jvms, games, nf_libs) = setup_neoforge(
            &client,
            &java_bin,
            &game_version,
            loader_version.as_deref(),
            &client_jar,
        )
        .await?;
        for lib in nf_libs {
            cp_entries.push(lib.to_string_lossy().to_string());
        }
        (mc, jvms, games)
    } else if modloader == "fabric" || modloader == "quilt" {
        let (mc, fab_libs) = setup_fabric(&client, &game_version, loader_version.as_deref()).await?;
        for lib in fab_libs {
            cp_entries.push(lib.to_string_lossy().to_string());
        }
        (mc, Vec::new(), Vec::new())
    } else {
        (version_meta.main_class.clone(), Vec::new(), Vec::new())
    };

    cp_entries.push(client_jar.to_string_lossy().to_string());

    let mut seen = std::collections::HashSet::new();
    let unique_cp: Vec<String> = cp_entries
        .into_iter()
        .filter(|p| std::path::Path::new(p).exists() && seen.insert(p.clone()))
        .collect();
    let classpath = unique_cp.join(":");

    // 7. Resolve Authentication (with automatic token refresh if needed)
    let account = super::auth::get_valid_account(&client).await;
    let (username, uuid, access_token, xuid, client_id, user_properties_json) = match &account {
        Some(acc) => {
            let xid = acc.xuid.clone().unwrap_or_default();
            let cid = acc.client_id.clone().unwrap_or_else(|| "c36a9fb6-4f2a-41ff-90bd-ae7cc92031eb".to_string());
            let u_props = match &acc.user_properties {
                Some(props) => props.to_string(),
                None => "{}".to_string(),
            };
            (acc.name.clone(), acc.uuid.clone(), acc.access_token.clone(), xid, cid, u_props)
        }
        None => (
            "LoomPlayer".to_string(),
            uuid::Uuid::new_v4().to_string(),
            "00000000000000000000000000000000".to_string(),
            String::new(),
            "c36a9fb6-4f2a-41ff-90bd-ae7cc92031eb".to_string(),
            "{}".to_string(),
        ),
    };

    let dashed_uuid = if uuid.contains('-') {
        uuid
    } else if uuid.len() == 32 {
        format!("{}-{}-{}-{}-{}", &uuid[0..8], &uuid[8..12], &uuid[12..16], &uuid[16..20], &uuid[20..32])
    } else {
        uuid
    };

    // Pre-seed usercache.json for Minecraft
    let usercache_path = mc_dir.join("usercache.json");
    let usercache_data = serde_json::json!([
        {
            "name": username,
            "uuid": dashed_uuid,
            "expiresOn": "2030-01-01 00:00:00 +0000"
        }
    ]);
    let _ = std::fs::write(&usercache_path, usercache_data.to_string());

    // 8. Build JVM Command
    let mut cmd = Command::new(&java_bin);
    cmd.current_dir(&mc_dir);

    // Memory Allocations
    cmd.arg(format!("-Xms{}M", min_ram_mb))
        .arg(format!("-Xmx{}M", max_ram_mb));

    // Loader JVM args (e.g. NeoForge module path, ignoreList, etc.)
    for arg in extra_jvm_args {
        cmd.arg(arg);
    }

    // Custom and GC args
    for arg in custom_jvm_args {
        cmd.arg(arg);
    }

    cmd.arg("-Djava.net.preferIPv4Stack=true")
        .arg("-Djava.net.preferIPv4Addresses=true")
        .arg("-Dsun.net.client.defaultConnectTimeout=10000")
        .arg("-Dsun.net.client.defaultReadTimeout=10000")
        .arg("-Dminecraft.launcher.brand=Loom")
        .arg("-Dminecraft.launcher.version=0.1.0")
        .arg("-cp")
        .arg(&classpath);

    // Main class
    cmd.arg(&main_class);

    // Modloader specific game arguments
    for arg in extra_game_args {
        cmd.arg(arg);
    }

    // Minecraft Game Arguments
    cmd.arg("--username")
        .arg(&username)
        .arg("--version")
        .arg(&game_version)
        .arg("--gameDir")
        .arg(&mc_dir)
        .arg("--assetsDir")
        .arg(&assets_dir)
        .arg("--assetIndex")
        .arg(&version_meta.asset_index_id)
        .arg("--uuid")
        .arg(&dashed_uuid)
        .arg("--accessToken")
        .arg(&access_token)
        .arg("--clientId")
        .arg(&client_id)
        .arg("--xuid")
        .arg(&xuid)
        .arg("--userType")
        .arg("msa")
        .arg("--versionType")
        .arg("release")
        .arg("--userProperties")
        .arg(&user_properties_json)
        .arg("--profileProperties")
        .arg(&user_properties_json);

    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    println!(
        "[Loom::Launcher] Spawning Minecraft instance '{}' with Java at {}",
        instance_id,
        java_bin.display()
    );

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Failed to spawn JVM process: {}", e))?;

    let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;
    let stderr = child.stderr.take().ok_or("Failed to capture stderr")?;

    let app_handle_out = app_handle.clone();
    let inst_id_out = instance_id.clone();

    // Stream STDOUT
    tokio::spawn(async move {
        let mut reader = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            let (level, source, message) = parse_log_line(&line);
            let evt = MinecraftLogEvent {
                instance_id: inst_id_out.clone(),
                timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                level,
                source,
                message,
            };
            app_handle_out.emit_log(evt);
        }
    });

    let app_handle_err = app_handle.clone();
    let inst_id_err = instance_id.clone();

    // Stream STDERR
    tokio::spawn(async move {
        let mut reader = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            let evt = MinecraftLogEvent {
                instance_id: inst_id_err.clone(),
                timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                level: "ERROR".to_string(),
                source: "JVM/stderr".to_string(),
                message: line,
            };
            app_handle_err.emit_log(evt);
        }
    });

    // Monitor process exit
    let app_handle_exit = app_handle.clone();
    let inst_id_exit = instance_id.clone();

    tokio::spawn(async move {
        match child.wait().await {
            Ok(status) => {
                let code = status.code();
                app_handle_exit.emit_exit(MinecraftExitEvent {
                    instance_id: inst_id_exit,
                    exit_code: code,
                });
            }
            Err(e) => {
                eprintln!("[Loom::Launcher] Error waiting on child: {}", e);
            }
        }
    });

    Ok(())
}
