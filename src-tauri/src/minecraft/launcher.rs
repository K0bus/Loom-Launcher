use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::process::Stdio;
use tauri::Emitter;
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

pub async fn launch_instance(
    app_handle: tauri::AppHandle,
    instance_id: String,
    allocated_ram_gb: Option<u32>,
) -> Result<(), String> {
    let client = Client::new();

    // 1. Fetch instance info from DB
    let conn = db::open_db().map_err(|e| e.to_string())?;
    let inst_opt = db::get_minecraft_instance(&conn, &instance_id).map_err(|e| e.to_string())?;

    let (game_version, modloader, min_ram_mb, max_ram_mb, java_path, custom_jvm_args) = match inst_opt {
        Some(i) => {
            let jargs: Vec<String> = serde_json::from_str(&i.jvm_args).unwrap_or_else(|_| default_jvm_args());
            (i.mc_version, i.modloader, i.min_ram_mb, i.max_ram_mb, i.java_path, jargs)
        }
        None => {
            let profiles = db::get_profiles(&conn, Some("minecraft")).unwrap_or_default();
            let p = profiles.iter().find(|p| p.id == instance_id).cloned();
            let (gv, ml) = match p {
                Some(p) => (p.game_version, p.modloader.unwrap_or_else(|| "fabric".to_string())),
                None => ("1.20.1".to_string(), "fabric".to_string()),
            };
            let ram = allocated_ram_gb.unwrap_or(4) * 1024;
            (gv, ml, 2048, ram, None, default_jvm_args())
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

    // 6. Build Classpath
    let mut cp_entries: Vec<String> = libraries
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();
    cp_entries.push(client_jar.to_string_lossy().to_string());
    let classpath = cp_entries.join(":");

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
    let main_class = if modloader == "fabric" || modloader == "quilt" {
        "net.fabricmc.loader.impl.launch.knot.KnotClient"
    } else {
        &version_meta.main_class
    };
    cmd.arg(main_class);

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
            let _ = app_handle_out.emit("minecraft-log", evt);
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
            let _ = app_handle_err.emit("minecraft-log", evt);
        }
    });

    // Monitor process exit
    let app_handle_exit = app_handle.clone();
    let inst_id_exit = instance_id.clone();

    tokio::spawn(async move {
        match child.wait().await {
            Ok(status) => {
                let code = status.code();
                let _ = app_handle_exit.emit(
                    "minecraft-exit",
                    MinecraftExitEvent {
                        instance_id: inst_id_exit,
                        exit_code: code,
                    },
                );
            }
            Err(e) => {
                eprintln!("[Loom::Launcher] Error waiting on child: {}", e);
            }
        }
    });

    Ok(())
}
