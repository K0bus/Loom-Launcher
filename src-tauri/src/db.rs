use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinecraftInstanceRecord {
    pub id: String,
    pub name: String,
    pub mc_version: String,
    pub modloader: String, // 'vanilla', 'fabric', 'forge', 'neoforge', 'quilt'
    pub loader_version: Option<String>,
    pub min_ram_mb: u32,
    pub max_ram_mb: u32,
    pub java_path: Option<String>,
    pub jvm_args: String,
    pub created_at: i64,
    #[serde(default)]
    pub banner_url: Option<String>,
    #[serde(default)]
    pub cover_url: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub playtime_seconds: Option<u64>,
    #[serde(default)]
    pub last_played: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameProfileRecord {
    pub id: String,
    pub game_id: String,
    pub name: String,
    pub game_version: String,
    pub modloader: Option<String>,
    #[serde(default)]
    pub deployment_path: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileModRecord {
    pub id: String,
    pub profile_id: String,
    pub provider: String, // 'modrinth', 'curseforge', 'nexus', 'wago', 'local'
    pub remote_mod_id: String,
    pub remote_file_id: String,
    pub name: String,
    pub file_path: String,
    pub filename: String,
    #[serde(default)]
    pub staged_path: Option<String>,
    pub load_priority: i32,
    pub is_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModDependencyRecord {
    pub mod_id: String,
    pub depends_on_remote_id: String,
    pub is_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileConflictRuleRecord {
    pub profile_id: String,
    pub relative_path: String,
    pub winning_mod_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictingMod {
    pub mod_id: String,
    pub mod_name: String,
    pub load_priority: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileConflict {
    pub relative_path: String,
    pub conflicting_mods: Vec<ConflictingMod>,
    pub current_winner_mod_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BethesdaPluginRecord {
    pub id: String,
    pub profile_id: String,
    pub mod_id: Option<String>,
    pub filename: String,
    pub load_order_index: i32,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BattleNetConfigRecord {
    pub id: String,
    pub wine_prefix_path: String,
    pub runner_executable: String,
    pub battlenet_exe_path: String,
    pub enable_gamemode: bool,
    pub enable_mangohud: bool,
    pub custom_env_vars: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BattleNetGameRecord {
    pub product_uid: String,
    pub name: String,
    pub install_path: String,
    pub executable_relative_path: String,
    pub version_build: Option<String>,
    pub is_external: bool,
    pub is_installed: bool,
    pub last_played: Option<i64>,
    pub playtime_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BattleNetAccountRecord {
    pub account_id: String,
    pub battletag: String,
    pub avatar_url: Option<String>,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: i64,
}

pub fn get_db_path() -> PathBuf {
    let base = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string())).join(".local/share"))
        .join("loom");
    let _ = fs::create_dir_all(&base);
    base.join("loom.db")
}

pub fn open_db() -> Result<Connection> {
    let path = get_db_path();
    let conn = Connection::open(path)?;
    conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")?;
    init_schema(&conn)?;
    Ok(conn)
}

pub fn init_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS game_profiles (
            id TEXT PRIMARY KEY,
            game_id TEXT NOT NULL,
            name TEXT NOT NULL,
            game_version TEXT NOT NULL,
            modloader TEXT,
            deployment_path TEXT,
            created_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS minecraft_instances (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            mc_version TEXT NOT NULL,
            modloader TEXT NOT NULL,
            loader_version TEXT,
            min_ram_mb INTEGER NOT NULL DEFAULT 2048,
            max_ram_mb INTEGER NOT NULL DEFAULT 4096,
            java_path TEXT,
            jvm_args TEXT NOT NULL DEFAULT '["-XX:+UseG1GC","-XX:+UnlockExperimentalVMOptions","-XX:G1NewSizePercent=20","-XX:G1ReservePercent=20","-XX:MaxGCPauseMillis=50","-XX:G1HeapRegionSize=32M"]',
            created_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS profile_mods (
            id TEXT PRIMARY KEY,
            profile_id TEXT NOT NULL,
            provider TEXT NOT NULL,
            remote_mod_id TEXT NOT NULL,
            remote_file_id TEXT NOT NULL,
            name TEXT NOT NULL,
            file_path TEXT NOT NULL DEFAULT '',
            filename TEXT NOT NULL DEFAULT '',
            staged_path TEXT,
            load_priority INTEGER NOT NULL DEFAULT 0,
            is_enabled INTEGER NOT NULL DEFAULT 1
        );

        CREATE TABLE IF NOT EXISTS mod_dependencies (
            mod_id TEXT NOT NULL,
            depends_on_remote_id TEXT NOT NULL,
            is_required INTEGER NOT NULL DEFAULT 1,
            FOREIGN KEY(mod_id) REFERENCES profile_mods(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS file_conflict_rules (
            profile_id TEXT NOT NULL,
            relative_path TEXT NOT NULL,
            winning_mod_id TEXT NOT NULL,
            PRIMARY KEY(profile_id, relative_path)
        );

        -- Plugins spécifiques aux moteurs Bethesda (.esp, .esm, .esl)
        CREATE TABLE IF NOT EXISTS bethesda_plugins (
            id TEXT PRIMARY KEY,
            profile_id TEXT NOT NULL,
            mod_id TEXT,
            filename TEXT NOT NULL,
            load_order_index INTEGER NOT NULL,
            is_active INTEGER NOT NULL DEFAULT 1,
            FOREIGN KEY(profile_id) REFERENCES game_profiles(id) ON DELETE CASCADE,
            FOREIGN KEY(mod_id) REFERENCES profile_mods(id) ON DELETE SET NULL
        );

        -- Configuration du runner et de l'environnement Wine Battle.net
        CREATE TABLE IF NOT EXISTS battlenet_config (
            id TEXT PRIMARY KEY DEFAULT 'default',
            wine_prefix_path TEXT NOT NULL,
            runner_executable TEXT NOT NULL DEFAULT 'wine',
            battlenet_exe_path TEXT NOT NULL,
            enable_gamemode INTEGER NOT NULL DEFAULT 1,
            enable_mangohud INTEGER NOT NULL DEFAULT 0,
            custom_env_vars TEXT -- Format JSON key-value
        );

        -- Cache des jeux Battle.net (détectés localement et/ou externes)
        CREATE TABLE IF NOT EXISTS battlenet_games (
            product_uid TEXT PRIMARY KEY,       -- Ex: 'wow', 'fenris', 'pro', 'w3'
            name TEXT NOT NULL,
            install_path TEXT NOT NULL,
            executable_relative_path TEXT NOT NULL,
            version_build TEXT,
            is_external INTEGER NOT NULL DEFAULT 0,
            is_installed INTEGER NOT NULL DEFAULT 1,
            last_played INTEGER,
            playtime_seconds INTEGER NOT NULL DEFAULT 0
        );

        -- Compte Blizzard lié via OAuth2
        CREATE TABLE IF NOT EXISTS battlenet_accounts (
            account_id TEXT PRIMARY KEY,
            battletag TEXT NOT NULL,
            avatar_url TEXT,
            access_token TEXT NOT NULL,
            refresh_token TEXT NOT NULL,
            expires_at INTEGER NOT NULL
        );

        -- Identifiants API Développeur Blizzard (OAuth Client ID & Secret)
        CREATE TABLE IF NOT EXISTS battlenet_credentials (
            id TEXT PRIMARY KEY DEFAULT 'default',
            client_id TEXT NOT NULL,
            client_secret TEXT NOT NULL,
            updated_at INTEGER NOT NULL
        );
        "#,
    )?;

    // Safe migration: add columns if they don't exist
    let _ = conn.execute("ALTER TABLE game_profiles ADD COLUMN deployment_path TEXT;", []);
    let _ = conn.execute("ALTER TABLE profile_mods ADD COLUMN filename TEXT NOT NULL DEFAULT '';", []);
    let _ = conn.execute("ALTER TABLE profile_mods ADD COLUMN file_path TEXT NOT NULL DEFAULT '';", []);
    let _ = conn.execute("ALTER TABLE profile_mods ADD COLUMN staged_path TEXT;", []);
    let _ = conn.execute("ALTER TABLE battlenet_games ADD COLUMN playtime_seconds INTEGER NOT NULL DEFAULT 0;", []);

    Ok(())
}

pub fn get_profiles(conn: &Connection, game_id: Option<&str>) -> Result<Vec<GameProfileRecord>> {
    if let Some(gid) = game_id {
        let mut s = conn.prepare("SELECT id, game_id, name, game_version, modloader, deployment_path, created_at FROM game_profiles WHERE game_id = ? ORDER BY created_at DESC")?;
        let rows = s.query_map([gid], |row| {
            Ok(GameProfileRecord {
                id: row.get(0)?,
                game_id: row.get(1)?,
                name: row.get(2)?,
                game_version: row.get(3)?,
                modloader: row.get(4)?,
                deployment_path: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;
        rows.collect()
    } else {
        let mut s = conn.prepare("SELECT id, game_id, name, game_version, modloader, deployment_path, created_at FROM game_profiles ORDER BY created_at DESC")?;
        let rows = s.query_map([], |row| {
            Ok(GameProfileRecord {
                id: row.get(0)?,
                game_id: row.get(1)?,
                name: row.get(2)?,
                game_version: row.get(3)?,
                modloader: row.get(4)?,
                deployment_path: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;
        rows.collect()
    }
}

pub fn insert_profile(conn: &Connection, profile: &GameProfileRecord) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO game_profiles (id, game_id, name, game_version, modloader, deployment_path, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            profile.id,
            profile.game_id,
            profile.name,
            profile.game_version,
            profile.modloader,
            profile.deployment_path,
            profile.created_at
        ],
    )?;
    Ok(())
}

pub fn delete_profile(conn: &Connection, profile_id: &str) -> Result<()> {
    conn.execute("DELETE FROM game_profiles WHERE id = ?", params![profile_id])?;
    Ok(())
}

pub fn get_minecraft_instances(conn: &Connection) -> Result<Vec<MinecraftInstanceRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, mc_version, modloader, loader_version, min_ram_mb, max_ram_mb, java_path, jvm_args, created_at
         FROM minecraft_instances ORDER BY created_at DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(MinecraftInstanceRecord {
            id: row.get(0)?,
            name: row.get(1)?,
            mc_version: row.get(2)?,
            modloader: row.get(3)?,
            loader_version: row.get(4)?,
            min_ram_mb: row.get(5)?,
            max_ram_mb: row.get(6)?,
            java_path: row.get(7)?,
            jvm_args: row.get(8)?,
            created_at: row.get(9)?,
            banner_url: None,
            cover_url: None,
            icon: None,
            playtime_seconds: None,
            last_played: None,
        })
    })?;
    rows.collect()
}

pub fn get_minecraft_instance(conn: &Connection, id: &str) -> Result<Option<MinecraftInstanceRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, mc_version, modloader, loader_version, min_ram_mb, max_ram_mb, java_path, jvm_args, created_at
         FROM minecraft_instances WHERE id = ?",
    )?;
    let mut rows = stmt.query_map([id], |row| {
        Ok(MinecraftInstanceRecord {
            id: row.get(0)?,
            name: row.get(1)?,
            mc_version: row.get(2)?,
            modloader: row.get(3)?,
            loader_version: row.get(4)?,
            min_ram_mb: row.get(5)?,
            max_ram_mb: row.get(6)?,
            java_path: row.get(7)?,
            jvm_args: row.get(8)?,
            created_at: row.get(9)?,
            banner_url: None,
            cover_url: None,
            icon: None,
            playtime_seconds: None,
            last_played: None,
        })
    })?;
    if let Some(res) = rows.next() {
        Ok(Some(res?))
    } else {
        Ok(None)
    }
}

pub fn insert_minecraft_instance(conn: &Connection, inst: &MinecraftInstanceRecord) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO minecraft_instances (id, name, mc_version, modloader, loader_version, min_ram_mb, max_ram_mb, java_path, jvm_args, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            inst.id,
            inst.name,
            inst.mc_version,
            inst.modloader,
            inst.loader_version,
            inst.min_ram_mb,
            inst.max_ram_mb,
            inst.java_path,
            inst.jvm_args,
            inst.created_at,
        ],
    )?;
    Ok(())
}

pub fn delete_minecraft_instance(conn: &Connection, id: &str) -> Result<()> {
    conn.execute("DELETE FROM minecraft_instances WHERE id = ?", params![id])?;
    conn.execute("DELETE FROM profile_mods WHERE profile_id = ?", params![id])?;
    Ok(())
}

pub fn get_profile_mods(conn: &Connection, profile_id: &str) -> Result<Vec<ProfileModRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, profile_id, provider, remote_mod_id, remote_file_id, name, COALESCE(file_path, ''), COALESCE(filename, ''), staged_path, load_priority, is_enabled
         FROM profile_mods WHERE profile_id = ? ORDER BY load_priority ASC, name ASC",
    )?;
    let rows = stmt.query_map([profile_id], |row| {
        let is_enabled_int: i32 = row.get(10)?;
        let mut file_path: String = row.get(6)?;
        let mut filename: String = row.get(7)?;
        if filename.is_empty() && !file_path.is_empty() {
            filename = std::path::Path::new(&file_path)
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| file_path.clone());
        }
        if file_path.is_empty() && !filename.is_empty() {
            file_path = filename.clone();
        }

        Ok(ProfileModRecord {
            id: row.get(0)?,
            profile_id: row.get(1)?,
            provider: row.get(2)?,
            remote_mod_id: row.get(3)?,
            remote_file_id: row.get(4)?,
            name: row.get(5)?,
            file_path,
            filename,
            staged_path: row.get(8)?,
            load_priority: row.get(9)?,
            is_enabled: is_enabled_int == 1,
        })
    })?;
    rows.collect()
}

pub fn insert_profile_mod(conn: &Connection, pmod: &ProfileModRecord) -> Result<()> {
    let filename = if pmod.filename.is_empty() {
        &pmod.file_path
    } else {
        &pmod.filename
    };
    let file_path = if pmod.file_path.is_empty() {
        filename
    } else {
        &pmod.file_path
    };

    conn.execute(
        "INSERT OR REPLACE INTO profile_mods (id, profile_id, provider, remote_mod_id, remote_file_id, name, file_path, filename, staged_path, load_priority, is_enabled)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            pmod.id,
            pmod.profile_id,
            pmod.provider,
            pmod.remote_mod_id,
            pmod.remote_file_id,
            pmod.name,
            file_path,
            filename,
            pmod.staged_path,
            pmod.load_priority,
            if pmod.is_enabled { 1 } else { 0 },
        ],
    )?;
    Ok(())
}

pub fn toggle_profile_mod(conn: &Connection, profile_id: &str, mod_id: &str, is_enabled: bool) -> Result<()> {
    conn.execute(
        "UPDATE profile_mods SET is_enabled = ?1 WHERE profile_id = ?2 AND id = ?3",
        params![if is_enabled { 1 } else { 0 }, profile_id, mod_id],
    )?;
    Ok(())
}

pub fn update_mod_priority(conn: &Connection, profile_id: &str, mod_id: &str, priority: i32) -> Result<()> {
    conn.execute(
        "UPDATE profile_mods SET load_priority = ?1 WHERE profile_id = ?2 AND id = ?3",
        params![priority, profile_id, mod_id],
    )?;
    Ok(())
}

pub fn delete_profile_mod(conn: &Connection, profile_id: &str, mod_id: &str) -> Result<()> {
    conn.execute(
        "DELETE FROM profile_mods WHERE profile_id = ?1 AND id = ?2",
        params![profile_id, mod_id],
    )?;
    Ok(())
}

pub fn insert_mod_dependency(conn: &Connection, dep: &ModDependencyRecord) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO mod_dependencies (mod_id, depends_on_remote_id, is_required) VALUES (?1, ?2, ?3)",
        params![dep.mod_id, dep.depends_on_remote_id, if dep.is_required { 1 } else { 0 }],
    )?;
    Ok(())
}

pub fn get_mod_dependencies(conn: &Connection, mod_id: &str) -> Result<Vec<ModDependencyRecord>> {
    let mut stmt = conn.prepare("SELECT mod_id, depends_on_remote_id, is_required FROM mod_dependencies WHERE mod_id = ?")?;
    let rows = stmt.query_map([mod_id], |row| {
        let req_int: i32 = row.get(2)?;
        Ok(ModDependencyRecord {
            mod_id: row.get(0)?,
            depends_on_remote_id: row.get(1)?,
            is_required: req_int == 1,
        })
    })?;
    rows.collect()
}

pub fn set_file_conflict_rule(conn: &Connection, rule: &FileConflictRuleRecord) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO file_conflict_rules (profile_id, relative_path, winning_mod_id) VALUES (?1, ?2, ?3)",
        params![rule.profile_id, rule.relative_path, rule.winning_mod_id],
    )?;
    Ok(())
}

pub fn get_file_conflict_rules(conn: &Connection, profile_id: &str) -> Result<Vec<FileConflictRuleRecord>> {
    let mut stmt = conn.prepare(
        "SELECT profile_id, relative_path, winning_mod_id FROM file_conflict_rules WHERE profile_id = ?"
    )?;
    let rows = stmt.query_map([profile_id], |row| {
        Ok(FileConflictRuleRecord {
            profile_id: row.get(0)?,
            relative_path: row.get(1)?,
            winning_mod_id: row.get(2)?,
        })
    })?;
    rows.collect()
}

pub fn delete_file_conflict_rule(conn: &Connection, profile_id: &str, relative_path: &str) -> Result<()> {
    conn.execute(
        "DELETE FROM file_conflict_rules WHERE profile_id = ?1 AND relative_path = ?2",
        params![profile_id, relative_path],
    )?;
    Ok(())
}

pub fn get_bethesda_plugins(conn: &Connection, profile_id: &str) -> Result<Vec<BethesdaPluginRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, profile_id, mod_id, filename, load_order_index, is_active
         FROM bethesda_plugins WHERE profile_id = ? ORDER BY load_order_index ASC"
    )?;
    let rows = stmt.query_map([profile_id], |row| {
        let is_active_int: i32 = row.get(5)?;
        Ok(BethesdaPluginRecord {
            id: row.get(0)?,
            profile_id: row.get(1)?,
            mod_id: row.get(2)?,
            filename: row.get(3)?,
            load_order_index: row.get(4)?,
            is_active: is_active_int == 1,
        })
    })?;
    rows.collect()
}

pub fn insert_or_replace_bethesda_plugin(conn: &Connection, plugin: &BethesdaPluginRecord) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO bethesda_plugins (id, profile_id, mod_id, filename, load_order_index, is_active)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            plugin.id,
            plugin.profile_id,
            plugin.mod_id,
            plugin.filename,
            plugin.load_order_index,
            if plugin.is_active { 1 } else { 0 },
        ],
    )?;
    Ok(())
}

pub fn update_plugin_load_order(conn: &Connection, profile_id: &str, ordered_filenames: &[String]) -> Result<()> {
    for (idx, filename) in ordered_filenames.iter().enumerate() {
        conn.execute(
            "UPDATE bethesda_plugins SET load_order_index = ?1 WHERE profile_id = ?2 AND filename = ?3",
            params![idx as i32, profile_id, filename],
        )?;
    }
    Ok(())
}

pub fn toggle_bethesda_plugin(conn: &Connection, profile_id: &str, filename: &str, is_active: bool) -> Result<()> {
    conn.execute(
        "UPDATE bethesda_plugins SET is_active = ?1 WHERE profile_id = ?2 AND filename = ?3",
        params![if is_active { 1 } else { 0 }, profile_id, filename],
    )?;
    Ok(())
}

pub fn delete_bethesda_plugin(conn: &Connection, profile_id: &str, filename: &str) -> Result<()> {
    conn.execute(
        "DELETE FROM bethesda_plugins WHERE profile_id = ?1 AND filename = ?2",
        params![profile_id, filename],
    )?;
    Ok(())
}

// ==========================================
// BATTLENET PERSISTENCE HELPERS
// ==========================================

pub fn get_default_battlenet_prefix() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| {
        PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string()))
    });
    // Check candidate existing prefixes on Linux
    let lutris_prefix = home.join("Games").join("battlenet");
    if lutris_prefix.join("drive_c").exists() {
        return lutris_prefix;
    }
    let bottles_prefix = home
        .join(".local/share/bottles/bottles/Battle-net");
    if bottles_prefix.join("drive_c").exists() {
        return bottles_prefix;
    }

    dirs::data_local_dir()
        .unwrap_or_else(|| home.join(".local/share"))
        .join("loom")
        .join("prefixes")
        .join("battlenet")
}

pub fn get_battlenet_config(conn: &Connection) -> Result<BattleNetConfigRecord> {
    let mut stmt = conn.prepare(
        "SELECT id, wine_prefix_path, runner_executable, battlenet_exe_path, enable_gamemode, enable_mangohud, custom_env_vars
         FROM battlenet_config WHERE id = 'default'",
    )?;
    let mut rows = stmt.query([])?;
    if let Some(row) = rows.next()? {
        let mut prefix_path: String = row.get(1)?;
        let mut exe_path: String = row.get(3)?;

        // If currently configured prefix path does not exist, check default smart detection
        if !std::path::Path::new(&prefix_path).exists() {
            let smart_pfx = get_default_battlenet_prefix();
            if smart_pfx.exists() {
                prefix_path = smart_pfx.to_string_lossy().to_string();
                let cand_exe = smart_pfx
                    .join("drive_c")
                    .join("Program Files (x86)")
                    .join("Battle.net")
                    .join("Battle.net.exe");
                if cand_exe.exists() {
                    exe_path = cand_exe.to_string_lossy().to_string();
                }
            }
        }

        let gm: i32 = row.get(4)?;
        let mh: i32 = row.get(5)?;
        Ok(BattleNetConfigRecord {
            id: row.get(0)?,
            wine_prefix_path: prefix_path,
            runner_executable: row.get(2)?,
            battlenet_exe_path: exe_path,
            enable_gamemode: gm == 1,
            enable_mangohud: mh == 1,
            custom_env_vars: row.get(6)?,
        })
    } else {
        let def_prefix = get_default_battlenet_prefix();
        let def_prefix_str = def_prefix.to_string_lossy().to_string();
        let def_exe = def_prefix
            .join("drive_c")
            .join("Program Files (x86)")
            .join("Battle.net")
            .join("Battle.net.exe")
            .to_string_lossy()
            .to_string();

        let cfg = BattleNetConfigRecord {
            id: "default".to_string(),
            wine_prefix_path: def_prefix_str,
            runner_executable: "wine".to_string(),
            battlenet_exe_path: def_exe,
            enable_gamemode: true,
            enable_mangohud: false,
            custom_env_vars: Some("{}".to_string()),
        };
        let _ = save_battlenet_config(conn, &cfg);
        Ok(cfg)
    }
}

pub fn save_battlenet_config(conn: &Connection, cfg: &BattleNetConfigRecord) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO battlenet_config (id, wine_prefix_path, runner_executable, battlenet_exe_path, enable_gamemode, enable_mangohud, custom_env_vars)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            cfg.id,
            cfg.wine_prefix_path,
            cfg.runner_executable,
            cfg.battlenet_exe_path,
            if cfg.enable_gamemode { 1 } else { 0 },
            if cfg.enable_mangohud { 1 } else { 0 },
            cfg.custom_env_vars
        ],
    )?;
    Ok(())
}

pub fn get_battlenet_games(conn: &Connection) -> Result<Vec<BattleNetGameRecord>> {
    let mut stmt = conn.prepare(
        "SELECT product_uid, name, install_path, executable_relative_path, version_build, is_external, is_installed, last_played, playtime_seconds
         FROM battlenet_games ORDER BY last_played DESC NULLS LAST, name ASC",
    )?;
    let rows = stmt.query_map([], |row| {
        let ext: i32 = row.get(5)?;
        let inst: i32 = row.get(6)?;
        Ok(BattleNetGameRecord {
            product_uid: row.get(0)?,
            name: row.get(1)?,
            install_path: row.get(2)?,
            executable_relative_path: row.get(3)?,
            version_build: row.get(4)?,
            is_external: ext == 1,
            is_installed: inst == 1,
            last_played: row.get(7)?,
            playtime_seconds: row.get(8)?,
        })
    })?;
    rows.collect()
}

pub fn get_battlenet_game(conn: &Connection, product_uid: &str) -> Result<Option<BattleNetGameRecord>> {
    let mut stmt = conn.prepare(
        "SELECT product_uid, name, install_path, executable_relative_path, version_build, is_external, is_installed, last_played, playtime_seconds
         FROM battlenet_games WHERE product_uid = ?",
    )?;
    let mut rows = stmt.query([product_uid])?;
    if let Some(row) = rows.next()? {
        let ext: i32 = row.get(5)?;
        let inst: i32 = row.get(6)?;
        Ok(Some(BattleNetGameRecord {
            product_uid: row.get(0)?,
            name: row.get(1)?,
            install_path: row.get(2)?,
            executable_relative_path: row.get(3)?,
            version_build: row.get(4)?,
            is_external: ext == 1,
            is_installed: inst == 1,
            last_played: row.get(7)?,
            playtime_seconds: row.get(8)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn insert_or_replace_battlenet_game(conn: &Connection, game: &BattleNetGameRecord) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO battlenet_games (product_uid, name, install_path, executable_relative_path, version_build, is_external, is_installed, last_played, playtime_seconds)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            game.product_uid,
            game.name,
            game.install_path,
            game.executable_relative_path,
            game.version_build,
            if game.is_external { 1 } else { 0 },
            if game.is_installed { 1 } else { 0 },
            game.last_played,
            game.playtime_seconds
        ],
    )?;
    Ok(())
}

pub fn update_battlenet_game_last_played(conn: &Connection, product_uid: &str, timestamp: i64) -> Result<()> {
    conn.execute(
        "UPDATE battlenet_games SET last_played = ?1 WHERE product_uid = ?2",
        params![timestamp, product_uid],
    )?;
    Ok(())
}

pub fn update_battlenet_game_playtime(conn: &Connection, product_uid: &str, delta_seconds: u64, last_played: Option<i64>) -> Result<()> {
    if let Some(ts) = last_played {
        conn.execute(
            "UPDATE battlenet_games SET playtime_seconds = playtime_seconds + ?1, last_played = ?2 WHERE product_uid = ?3",
            params![delta_seconds, ts, product_uid],
        )?;
    } else {
        conn.execute(
            "UPDATE battlenet_games SET playtime_seconds = playtime_seconds + ?1 WHERE product_uid = ?2",
            params![delta_seconds, product_uid],
        )?;
    }
    Ok(())
}

pub fn get_battlenet_account(conn: &Connection) -> Result<Option<BattleNetAccountRecord>> {
    let mut stmt = conn.prepare(
        "SELECT account_id, battletag, avatar_url, access_token, refresh_token, expires_at
         FROM battlenet_accounts LIMIT 1",
    )?;
    let mut rows = stmt.query([])?;
    if let Some(row) = rows.next()? {
        let account_id: String = row.get(0)?;
        let battletag: String = row.get(1)?;
        if account_id.starts_with("demo-")
            || battletag.contains("LoomPlayer")
            || battletag.contains("LoomChampion")
        {
            let _ = conn.execute("DELETE FROM battlenet_accounts", []);
            return Ok(None);
        }
        Ok(Some(BattleNetAccountRecord {
            account_id,
            battletag,
            avatar_url: row.get(2)?,
            access_token: row.get(3)?,
            refresh_token: row.get(4)?,
            expires_at: row.get(5)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn save_battlenet_account(conn: &Connection, acc: &BattleNetAccountRecord) -> Result<()> {
    conn.execute("DELETE FROM battlenet_accounts", [])?;
    conn.execute(
        "INSERT INTO battlenet_accounts (account_id, battletag, avatar_url, access_token, refresh_token, expires_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            acc.account_id,
            acc.battletag,
            acc.avatar_url,
            acc.access_token,
            acc.refresh_token,
            acc.expires_at
        ],
    )?;
    Ok(())
}

pub fn delete_battlenet_account(conn: &Connection) -> Result<()> {
    conn.execute("DELETE FROM battlenet_accounts", [])?;
    Ok(())
}

pub fn get_battlenet_credentials_db(conn: &Connection) -> Result<Option<(String, String)>> {
    let mut stmt = conn.prepare(
        "SELECT client_id, client_secret FROM battlenet_credentials WHERE id = 'default' LIMIT 1"
    )?;
    let mut rows = stmt.query([])?;
    if let Some(row) = rows.next()? {
        let client_id: String = row.get(0)?;
        let client_secret: String = row.get(1)?;
        if !client_id.trim().is_empty() && !client_secret.trim().is_empty() {
            return Ok(Some((client_id, client_secret)));
        }
    }
    Ok(None)
}

pub fn save_battlenet_credentials_db(conn: &Connection, client_id: &str, client_secret: &str) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    conn.execute(
        "INSERT INTO battlenet_credentials (id, client_id, client_secret, updated_at)
         VALUES ('default', ?1, ?2, ?3)
         ON CONFLICT(id) DO UPDATE SET client_id = ?1, client_secret = ?2, updated_at = ?3",
        params![client_id, client_secret, now],
    )?;
    Ok(())
}

