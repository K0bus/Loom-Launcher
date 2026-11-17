//! Comptes : détection Steam locale, session Battle.net (base SQLite partagée), session Microsoft.

use crate::{battlenet, db, mc_auth};
use std::fs;
use std::path::PathBuf;

#[derive(Default, Clone)]
pub struct SteamAccount {
    pub connected: bool,
    pub persona: String,
    pub user: String,
    pub id64: String,
    pub path: String,
}

/// Compte Steam le plus récemment connecté (`config/loginusers.vdf`) — aucune valeur codée en dur.
pub fn steam_account() -> SteamAccount {
    let base = crate::steam::steam_base();
    let mut acc = SteamAccount {
        connected: base.exists(),
        path: base.to_string_lossy().replace(&dirs::home_dir().unwrap_or_default().to_string_lossy().to_string(), "~"),
        ..Default::default()
    };
    let Ok(txt) = fs::read_to_string(base.join("config/loginusers.vdf")) else { return acc };
    let (mut cur_id, mut cur_user, mut cur_persona, mut recent) = (String::new(), String::new(), String::new(), false);
    let mut best: Option<(String, String, String)> = None;
    let flush = |id: &str, u: &str, p: &str, r: bool, best: &mut Option<(String, String, String)>| {
        if !id.is_empty() && (r || best.is_none()) { *best = Some((id.into(), u.into(), p.into())); }
    };
    for line in txt.lines() {
        let parts: Vec<&str> = line.trim().split('"').collect();
        if parts.len() == 3 && parts[1].len() == 17 && parts[1].chars().all(|c| c.is_ascii_digit()) {
            flush(&cur_id, &cur_user, &cur_persona, recent, &mut best);
            cur_id = parts[1].into(); cur_user.clear(); cur_persona.clear(); recent = false;
        } else if parts.len() >= 4 {
            match parts[1] {
                "AccountName" => cur_user = parts[3].into(),
                "PersonaName" => cur_persona = parts[3].into(),
                "MostRecent" => recent = parts[3] == "1",
                _ => {}
            }
        }
    }
    flush(&cur_id, &cur_user, &cur_persona, recent, &mut best);
    if let Some((id, u, p)) = best { acc.id64 = id; acc.user = u; acc.persona = p; }
    acc
}

#[derive(Default, Clone)]
pub struct BnetInfo {
    pub connected: bool,
    pub tag: String,
    pub avatar_url: Option<String>,
    pub prefix: String,
    pub runner: String,
    pub options: String,
    pub credentials_configured: bool,
    pub client_id: String,
}

pub async fn bnet_info() -> BnetInfo {
    let mut info = BnetInfo::default();
    if let Ok(conn) = db::open_db() {
        if let Ok(cfg) = db::get_battlenet_config(&conn) {
            info.prefix = cfg.wine_prefix_path;
            info.runner = cfg.runner_executable;
            let mut o = Vec::new();
            if cfg.enable_gamemode { o.push("GameMode"); }
            if cfg.enable_mangohud { o.push("MangoHud"); }
            info.options = if o.is_empty() { "Par défaut".into() } else { o.join(" • ") };
        }
    }
    let (configured, cid) = battlenet::auth::get_blizzard_credentials_status();
    info.credentials_configured = configured;
    info.client_id = cid.unwrap_or_default();
    if let Ok(Some(acc)) = battlenet::auth::ensure_valid_blizzard_account().await {
        info.connected = true;
        info.tag = acc.battletag;
        info.avatar_url = acc.avatar_url.filter(|u| !u.is_empty());
    }
    info
}

pub async fn microsoft() -> Option<(String, String)> {
    let client = reqwest::Client::new();
    mc_auth::get_valid_account(&client).await.map(|a| (a.name, a.uuid))
}

pub fn add_external_bnet_game(path: &str) -> Result<String, String> {
    let dir = PathBuf::from(path.trim());
    let meta = battlenet::detector::inspect_game_directory(&dir)?;
    let conn = db::open_db().map_err(|e| e.to_string())?;
    let last = battlenet::detector::detect_directory_last_modified(&dir);
    let rec = db::BattleNetGameRecord {
        product_uid: meta.product_uid,
        name: meta.name.clone(),
        install_path: meta.install_path,
        executable_relative_path: meta.executable_relative_path,
        version_build: meta.version_build,
        is_external: true,
        is_installed: true,
        last_played: last,
        playtime_seconds: 0,
    };
    db::insert_or_replace_battlenet_game(&conn, &rec).map_err(|e| e.to_string())?;
    Ok(meta.name)
}

pub fn copy_to_clipboard(text: &str) -> bool {
    use std::io::Write;
    for (bin, args) in [("wl-copy", vec![]), ("xclip", vec!["-selection", "clipboard"]), ("xsel", vec!["-ib"])] {
        if let Ok(mut c) = std::process::Command::new(bin).args(&args).stdin(std::process::Stdio::piped()).spawn() {
            if let Some(mut si) = c.stdin.take() { let _ = si.write_all(text.as_bytes()); }
            return true;
        }
    }
    false
}
