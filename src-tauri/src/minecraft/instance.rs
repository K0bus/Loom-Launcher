use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

use crate::db::{self, MinecraftInstanceRecord};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceConfig {
    pub id: String,
    pub name: String,
    pub mc_version: String,
    pub modloader: String, // 'vanilla', 'fabric', 'forge', 'neoforge', 'quilt'
    pub loader_version: Option<String>,
    pub min_ram_mb: u32,
    pub max_ram_mb: u32,
    pub java_path: Option<String>,
    pub jvm_args: Vec<String>,
    pub created_at: i64,
}

pub fn get_loom_base_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string())).join(".local/share"))
        .join("loom")
}

pub fn get_cache_mods_dir() -> PathBuf {
    let dir = get_loom_base_dir().join("cache").join("mods");
    let _ = fs::create_dir_all(&dir);
    dir
}

pub fn get_common_assets_dir() -> PathBuf {
    let dir = get_loom_base_dir().join("common").join("assets");
    let _ = fs::create_dir_all(&dir);
    dir
}

pub fn get_common_libraries_dir() -> PathBuf {
    let dir = get_loom_base_dir().join("common").join("libraries");
    let _ = fs::create_dir_all(&dir);
    dir
}

pub fn get_instances_dir() -> PathBuf {
    let dir = get_loom_base_dir().join("instances");
    let _ = fs::create_dir_all(&dir);
    dir
}

pub fn get_instance_dir(instance_id: &str) -> PathBuf {
    get_instances_dir().join(instance_id)
}

pub fn get_instance_minecraft_dir(instance_id: &str) -> PathBuf {
    get_instance_dir(instance_id).join(".minecraft")
}

pub fn get_instance_mods_dir(instance_id: &str) -> PathBuf {
    get_instance_minecraft_dir(instance_id).join("mods")
}

pub fn default_jvm_args() -> Vec<String> {
    vec![
        "-XX:+UseG1GC".to_string(),
        "-XX:+UnlockExperimentalVMOptions".to_string(),
        "-XX:G1NewSizePercent=20".to_string(),
        "-XX:G1ReservePercent=20".to_string(),
        "-XX:MaxGCPauseMillis=50".to_string(),
        "-XX:G1HeapRegionSize=32M".to_string(),
    ]
}

#[derive(Debug, Deserialize)]
pub struct CreateInstanceParams {
    pub name: String,
    pub mc_version: String,
    pub modloader: String,
    pub loader_version: Option<String>,
    pub min_ram_mb: Option<u32>,
    pub max_ram_mb: Option<u32>,
    pub java_path: Option<String>,
    pub jvm_args: Option<Vec<String>>,
}

/// Creates a new compartmentalized Minecraft instance with isolated .minecraft folder
pub fn create_instance(params: CreateInstanceParams) -> Result<MinecraftInstanceRecord, String> {
    let instance_id = format!("inst-{}", Uuid::new_v4().to_string().replace('-', "")[..12].to_string());
    let instance_dir = get_instance_dir(&instance_id);
    let mc_dir = get_instance_minecraft_dir(&instance_id);

    // 1. Create sealed directory hierarchy
    fs::create_dir_all(&instance_dir).map_err(|e| format!("Impossible de créer le dossier d'instance: {}", e))?;
    fs::create_dir_all(mc_dir.join("mods")).map_err(|e| format!("Impossible de créer le dossier mods: {}", e))?;
    fs::create_dir_all(mc_dir.join("resourcepacks")).map_err(|e| format!("Impossible de créer resourcepacks: {}", e))?;
    fs::create_dir_all(mc_dir.join("shaderpacks")).map_err(|e| format!("Impossible de créer shaderpacks: {}", e))?;
    fs::create_dir_all(mc_dir.join("config")).map_err(|e| format!("Impossible de créer config: {}", e))?;

    // Create options.txt if not exists
    let options_file = mc_dir.join("options.txt");
    if !options_file.exists() {
        let _ = fs::write(&options_file, "lang:fr_fr\nfullscreen:false\n");
    }

    let min_ram = params.min_ram_mb.unwrap_or(2048);
    let max_ram = params.max_ram_mb.unwrap_or(4096);
    let jvm_args_list = params.jvm_args.unwrap_or_else(default_jvm_args);
    let jvm_args_json = serde_json::to_string(&jvm_args_list).unwrap_or_else(|_| "[]".to_string());
    let now = chrono::Utc::now().timestamp();

    // 2. Write instance.json metadata
    let config = InstanceConfig {
        id: instance_id.clone(),
        name: params.name.clone(),
        mc_version: params.mc_version.clone(),
        modloader: params.modloader.clone(),
        loader_version: params.loader_version.clone(),
        min_ram_mb: min_ram,
        max_ram_mb: max_ram,
        java_path: params.java_path.clone(),
        jvm_args: jvm_args_list,
        created_at: now,
    };

    let config_json = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
    fs::write(instance_dir.join("instance.json"), config_json)
        .map_err(|e| format!("Erreur d'écriture instance.json: {}", e))?;

    // 3. Persist into SQLite
    let record = MinecraftInstanceRecord {
        id: instance_id.clone(),
        name: params.name.clone(),
        mc_version: params.mc_version.clone(),
        modloader: params.modloader.clone(),
        loader_version: params.loader_version.clone(),
        min_ram_mb: min_ram,
        max_ram_mb: max_ram,
        java_path: params.java_path.clone(),
        jvm_args: jvm_args_json,
        created_at: now,
    };

    let conn = db::open_db().map_err(|e| e.to_string())?;
    db::insert_minecraft_instance(&conn, &record).map_err(|e| e.to_string())?;

    // Also insert into game_profiles for backward compatibility
    let _ = db::insert_profile(
        &conn,
        &db::GameProfileRecord {
            id: instance_id.clone(),
            game_id: "minecraft".to_string(),
            name: params.name,
            game_version: params.mc_version,
            modloader: Some(params.modloader),
            deployment_path: None,
            created_at: now,
        },
    );

    Ok(record)
}

pub fn get_instances() -> Result<Vec<MinecraftInstanceRecord>, String> {
    let conn = db::open_db().map_err(|e| e.to_string())?;
    let mut list = db::get_minecraft_instances(&conn).map_err(|e| e.to_string())?;

    // If list is empty, verify if any legacy profiles exist and migrate them
    if list.is_empty() {
        if let Ok(profiles) = db::get_profiles(&conn, Some("minecraft")) {
            for p in profiles {
                let rec = MinecraftInstanceRecord {
                    id: p.id.clone(),
                    name: p.name.clone(),
                    mc_version: p.game_version.clone(),
                    modloader: p.modloader.clone().unwrap_or_else(|| "fabric".to_string()),
                    loader_version: None,
                    min_ram_mb: 2048,
                    max_ram_mb: 4096,
                    java_path: None,
                    jvm_args: serde_json::to_string(&default_jvm_args()).unwrap_or_else(|_| "[]".to_string()),
                    created_at: p.created_at,
                };
                let _ = db::insert_minecraft_instance(&conn, &rec);
                list.push(rec);
            }
        }
    }

    Ok(list)
}

pub fn get_instance(id: &str) -> Result<Option<MinecraftInstanceRecord>, String> {
    let conn = db::open_db().map_err(|e| e.to_string())?;
    db::get_minecraft_instance(&conn, id).map_err(|e| e.to_string())
}

pub fn delete_instance(id: &str) -> Result<(), String> {
    let conn = db::open_db().map_err(|e| e.to_string())?;
    db::delete_minecraft_instance(&conn, id).map_err(|e| e.to_string())?;
    let _ = db::delete_profile(&conn, id);

    // Delete directory recursively
    let dir = get_instance_dir(id);
    if dir.exists() {
        let _ = fs::remove_dir_all(dir);
    }

    Ok(())
}

pub fn update_instance_settings(
    id: &str,
    min_ram_mb: u32,
    max_ram_mb: u32,
    jvm_args: Vec<String>,
    java_path: Option<String>,
) -> Result<(), String> {
    let conn = db::open_db().map_err(|e| e.to_string())?;
    let mut inst = match db::get_minecraft_instance(&conn, id).map_err(|e| e.to_string())? {
        Some(i) => i,
        None => return Err(format!("Instance introuvable: {}", id)),
    };

    inst.min_ram_mb = min_ram_mb;
    inst.max_ram_mb = max_ram_mb;
    inst.jvm_args = serde_json::to_string(&jvm_args).unwrap_or_else(|_| "[]".to_string());
    inst.java_path = java_path.clone();

    db::insert_minecraft_instance(&conn, &inst).map_err(|e| e.to_string())?;

    // Update instance.json on disk if exists
    let instance_dir = get_instance_dir(id);
    let cfg_path = instance_dir.join("instance.json");
    if cfg_path.exists() {
        let config = InstanceConfig {
            id: inst.id,
            name: inst.name,
            mc_version: inst.mc_version,
            modloader: inst.modloader,
            loader_version: inst.loader_version,
            min_ram_mb,
            max_ram_mb,
            java_path,
            jvm_args,
            created_at: inst.created_at,
        };
        if let Ok(json) = serde_json::to_string_pretty(&config) {
            let _ = fs::write(cfg_path, json);
        }
    }

    Ok(())
}
