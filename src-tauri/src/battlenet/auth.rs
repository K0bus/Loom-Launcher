use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use crate::db::{self, BattleNetAccountRecord, BattleNetGameRecord};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlizzardGameEntitlement {
    pub product_uid: String,
    pub name: String,
    pub is_owned: bool,
    pub is_installed: bool,
    pub install_path: Option<String>,
    pub version_build: Option<String>,
    pub cover_url: Option<String>,
    pub banner_url: Option<String>,
    pub summary: Option<String>,
    pub stats: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct BlizzardTokenResponse {
    access_token: String,
    token_type: String,
    expires_in: i64,
    refresh_token: Option<String>,
    scope: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BlizzardUserInfo {
    sub: Option<String>,
    id: Option<Value>,
    battletag: Option<String>,
}

/// Helper to read Blizzard Client ID & Secret from database, environment, or Loom's config files
pub fn get_blizzard_credentials() -> (Option<String>, Option<String>) {
    let mut client_id = std::env::var("BLIZZARD_CLIENT_ID").ok();
    let mut client_secret = std::env::var("BLIZZARD_CLIENT_SECRET").ok();

    // 1. Check SQLite database (Primary persistent storage for desktop app)
    if client_id.is_none() || client_secret.is_none() {
        if let Ok(conn) = db::open_db() {
            if let Ok(Some((cid, csec))) = db::get_battlenet_credentials_db(&conn) {
                if client_id.is_none() && !cid.trim().is_empty() {
                    std::env::set_var("BLIZZARD_CLIENT_ID", &cid);
                    client_id = Some(cid);
                }
                if client_secret.is_none() && !csec.trim().is_empty() {
                    std::env::set_var("BLIZZARD_CLIENT_SECRET", &csec);
                    client_secret = Some(csec);
                }
            }
        }
    }

    // 2. Check JSON credentials file in local data dir (~/.local/share/loom/blizzard_credentials.json)
    if client_id.is_none() || client_secret.is_none() {
        let base_dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string())).join(".local/share"))
            .join("loom");
        let creds_file = base_dir.join("blizzard_credentials.json");
        if creds_file.exists() {
            if let Ok(content) = fs::read_to_string(&creds_file) {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                    if client_id.is_none() {
                        if let Some(cid) = val.get("client_id").and_then(|v| v.as_str()) {
                            if !cid.trim().is_empty() {
                                std::env::set_var("BLIZZARD_CLIENT_ID", cid);
                                client_id = Some(cid.to_string());
                            }
                        }
                    }
                    if client_secret.is_none() {
                        if let Some(csec) = val.get("client_secret").and_then(|v| v.as_str()) {
                            if !csec.trim().is_empty() {
                                std::env::set_var("BLIZZARD_CLIENT_SECRET", csec);
                                client_secret = Some(csec.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    // 3. Look for .env in current or parent dirs (Dev fallback)
    if client_id.is_none() || client_secret.is_none() {
        for env_path in &[PathBuf::from(".env"), PathBuf::from("../.env")] {
            if let Ok(content) = fs::read_to_string(env_path) {
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with('#') || trimmed.is_empty() {
                        continue;
                    }
                    if let Some((k, v)) = trimmed.split_once('=') {
                        let key = k.trim();
                        let val = v.trim().trim_matches('"').trim_matches('\'').to_string();
                        if key == "BLIZZARD_CLIENT_ID" && client_id.is_none() && !val.is_empty() {
                            client_id = Some(val);
                        } else if key == "BLIZZARD_CLIENT_SECRET" && client_secret.is_none() && !val.is_empty() {
                            client_secret = Some(val);
                        }
                    }
                }
            }
        }
    }

    (client_id, client_secret)
}

/// Helper to write Blizzard Client ID & Secret to SQLite and persistent JSON configuration
pub fn save_blizzard_credentials(client_id: &str, client_secret: &str) -> Result<(), String> {
    let client_id = client_id.trim();
    let client_secret = client_secret.trim();

    if client_id.is_empty() || client_secret.is_empty() {
        return Err("Le Client ID et le Client Secret ne peuvent pas être vides".to_string());
    }

    // 1. Persist to SQLite Database
    let conn = db::open_db().map_err(|e| format!("Impossible d'ouvrir la base de données: {}", e))?;
    db::save_battlenet_credentials_db(&conn, client_id, client_secret)
        .map_err(|e| format!("Impossible d'enregistrer les identifiants dans SQLite: {}", e))?;

    // 2. Persist to JSON config file in user data directory (~/.local/share/loom/blizzard_credentials.json)
    let base_dir = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string())).join(".local/share"))
        .join("loom");
    let _ = fs::create_dir_all(&base_dir);
    let creds_file = base_dir.join("blizzard_credentials.json");
    let creds_json = serde_json::json!({
        "client_id": client_id,
        "client_secret": client_secret,
    });
    if let Ok(json_str) = serde_json::to_string_pretty(&creds_json) {
        let _ = fs::write(&creds_file, json_str);
    }

    // 3. Update memory environment variables
    std::env::set_var("BLIZZARD_CLIENT_ID", client_id);
    std::env::set_var("BLIZZARD_CLIENT_SECRET", client_secret);

    // 4. Update .env file if it exists (for dev environment convenience)
    for env_path in &[PathBuf::from(".env"), PathBuf::from("../.env")] {
        if env_path.exists() {
            if let Ok(content) = fs::read_to_string(env_path) {
                let mut lines = Vec::new();
                let mut found_id = false;
                let mut found_secret = false;
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("BLIZZARD_CLIENT_ID=") {
                        lines.push(format!("BLIZZARD_CLIENT_ID=\"{}\"", client_id));
                        found_id = true;
                    } else if trimmed.starts_with("BLIZZARD_CLIENT_SECRET=") {
                        lines.push(format!("BLIZZARD_CLIENT_SECRET=\"{}\"", client_secret));
                        found_secret = true;
                    } else {
                        lines.push(line.to_string());
                    }
                }
                if !found_id {
                    lines.push(format!("BLIZZARD_CLIENT_ID=\"{}\"", client_id));
                }
                if !found_secret {
                    lines.push(format!("BLIZZARD_CLIENT_SECRET=\"{}\"", client_secret));
                }
                let new_content = lines.join("\n") + "\n";
                let _ = fs::write(env_path, new_content);
            }
        }
    }

    println!("[Loom::BlizzardOAuth] Identifiants Blizzard enregistrés avec succès (SQLite & config)");
    Ok(())
}

/// Helper to get configured Blizzard Client ID status
pub fn get_blizzard_credentials_status() -> (bool, Option<String>) {
    let (client_id, client_secret) = get_blizzard_credentials();
    match (client_id, client_secret) {
        (Some(id), Some(sec)) if !id.trim().is_empty() && !sec.trim().is_empty() => (true, Some(id)),
        (Some(id), _) if !id.trim().is_empty() => (false, Some(id)),
        _ => (false, None),
    }
}

/// Starts the Blizzard OAuth2 authorization flow.
/// Listens locally on port 8999 (or ephemeral fallback), opens the browser,
/// captures the authorization code, exchanges it for tokens, fetches userinfo,
/// and persists the account in SQLite.
pub async fn start_blizzard_oauth_flow() -> Result<BattleNetAccountRecord, String> {
    let (client_id_opt, client_secret_opt) = get_blizzard_credentials();

    let (client_id, client_secret) = match (client_id_opt, client_secret_opt) {
        (Some(id), Some(sec)) if !id.trim().is_empty() && !sec.trim().is_empty() => (id, sec),
        _ => {
            return Err("CREDENTIALS_REQUIRED".to_string());
        }
    };

    // Bind local TCP listener - prefer port 8999 for static redirect URI match on develop.battle.net
    let listener = match TcpListener::bind("127.0.0.1:8999").await {
        Ok(l) => l,
        Err(_) => TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|e| format!("Impossible d'ouvrir l'écouteur local pour OAuth2: {}", e))?,
    };

    let port = listener
        .local_addr()
        .map_err(|e| format!("Impossible de déterminer le port local: {}", e))?
        .port();

    let redirect_uri = format!("http://localhost:{}/callback", port);
    let state = uuid::Uuid::new_v4().to_string();

    let auth_url = format!(
        "https://oauth.battle.net/authorize?client_id={}&redirect_uri={}&response_type=code&scope=openid%20wow.profile%20sc2.profile%20d3.profile&state={}",
        urlencoding::encode(&client_id),
        urlencoding::encode(&redirect_uri),
        state
    );

    println!("[Loom::BlizzardOAuth] Ouverture du navigateur sur: {}", auth_url);

    // Open browser via unified opener
    if let Err(e) = crate::open_url_in_default_browser(&auth_url) {
        eprintln!("[Loom::BlizzardOAuth] Erreur lors de l'ouverture du navigateur: {}", e);
    }

    // Await incoming HTTP connection with 120s timeout
    let accept_future = listener.accept();
    let (mut stream, _) = tokio::time::timeout(Duration::from_secs(120), accept_future)
        .await
        .map_err(|_| "Délai d'attente de l'authentification Blizzard dépassé (120s)".to_string())?
        .map_err(|e| format!("Erreur lors de la réception de la redirection OAuth: {}", e))?;

    let mut buf = vec![0u8; 4096];
    let n = stream
        .read(&mut buf)
        .await
        .map_err(|e| format!("Erreur lecture requête HTTP: {}", e))?;

    let req_str = String::from_utf8_lossy(&buf[..n]);

    // Parse HTTP request line: GET /callback?code=...&state=... HTTP/1.1
    let first_line = req_str.lines().next().unwrap_or("");
    let mut code_opt = None;
    let mut returned_state_opt = None;

    if let Some(pos) = first_line.find("/callback?") {
        let query_part = &first_line[pos + 10..];
        let end = query_part.find(' ').unwrap_or(query_part.len());
        let query = &query_part[..end];

        for pair in query.split('&') {
            if let Some((k, v)) = pair.split_once('=') {
                if k == "code" {
                    code_opt = Some(v.to_string());
                } else if k == "state" {
                    returned_state_opt = Some(v.to_string());
                }
            }
        }
    }

    // Send styled response page back to browser
    let html_body = r#"<!DOCTYPE html>
<html lang="fr">
<head>
  <meta charset="utf-8">
  <title>Loom - Authentification Blizzard Réussie</title>
  <style>
    body {
      margin: 0;
      background: #09090b;
      color: #f4f4f5;
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
      display: flex;
      align-items: center;
      justify-content: center;
      height: 100vh;
    }
    .card {
      background: #18181b;
      border: 1px solid #27272a;
      border-radius: 16px;
      padding: 32px 40px;
      max-width: 440px;
      text-align: center;
      box-shadow: 0 20px 40px rgba(0,0,0,0.6);
    }
    .badge {
      display: inline-block;
      padding: 4px 12px;
      border-radius: 9999px;
      font-size: 12px;
      font-weight: 700;
      text-transform: uppercase;
      background: rgba(245, 158, 11, 0.15);
      color: #f59e0b;
      border: 1px solid rgba(245, 158, 11, 0.4);
      margin-bottom: 16px;
    }
    h1 {
      margin: 0 0 8px;
      font-size: 22px;
      font-weight: 800;
      color: #fafafa;
    }
    p {
      color: #a1a1aa;
      font-size: 14px;
      line-height: 1.5;
      margin: 0 0 20px;
    }
    .hint {
      color: #71717a;
      font-size: 12px;
    }
  </style>
</head>
<body>
  <div class="card">
    <div class="badge">Battle.net & Loom</div>
    <h1>Connexion Réussie !</h1>
    <p>Votre compte Blizzard a été authentifié et lié avec succès à Loom Launcher.</p>
    <div class="hint">Vous pouvez désormais fermer cet onglet et retourner dans l'application.</div>
  </div>
</body>
</html>"#;

    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=UTF-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        html_body.len(),
        html_body
    );

    let _ = stream.write_all(response.as_bytes()).await;
    let _ = stream.flush().await;

    let code = code_opt.ok_or_else(|| "Paramètre 'code' introuvable dans le retour OAuth Blizzard".to_string())?;

    if let Some(ret_state) = returned_state_opt {
        if ret_state != state {
            return Err("État OAuth non valide (échec vérification CSRF)".to_string());
        }
    }

    // Exchange authorization code for token
    let http_client = Client::new();
    let token_resp = http_client
        .post("https://oauth.battle.net/token")
        .basic_auth(&client_id, Some(&client_secret))
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", &code),
            ("redirect_uri", &redirect_uri),
        ])
        .send()
        .await
        .map_err(|e| format!("Erreur requête échange token Blizzard: {}", e))?;

    if !token_resp.status().is_success() {
        let err_text = token_resp.text().await.unwrap_or_default();
        return Err(format!("Échec échange de code OAuth Blizzard: {}", err_text));
    }

    let token_data: BlizzardTokenResponse = token_resp
        .json()
        .await
        .map_err(|e| format!("Impossible de décoder la réponse token Blizzard: {}", e))?;

    let now_ts = chrono::Utc::now().timestamp();
    let expires_at = now_ts + token_data.expires_in;

    // Fetch user info
    let userinfo_resp = http_client
        .get("https://oauth.battle.net/oauth/userinfo")
        .bearer_auth(&token_data.access_token)
        .send()
        .await
        .map_err(|e| format!("Erreur récupération profil Blizzard: {}", e))?;

    let battletag: String;
    let account_id: String;

    if userinfo_resp.status().is_success() {
        if let Ok(info) = userinfo_resp.json::<BlizzardUserInfo>().await {
            battletag = info.battletag.unwrap_or_else(|| "BlizzardPlayer".to_string());
            account_id = info
                .sub
                .or_else(|| info.id.map(|v| v.to_string()))
                .unwrap_or_else(|| battletag.clone());
        } else {
            battletag = "BlizzardPlayer".to_string();
            account_id = "blizzard_user".to_string();
        }
    } else {
        battletag = "BlizzardPlayer".to_string();
        account_id = "blizzard_user".to_string();
    }

    let btag_name = battletag.split('#').next().unwrap_or(&battletag);
    let mut avatar_url = Some(format!(
        "https://ui-avatars.com/api/?name={}&background=0284c7&color=ffffff&bold=true&size=128",
        urlencoding::encode(btag_name)
    ));

    // Try to resolve 3D render avatar from user's highest WoW character if present
    if let Ok(wow_res) = http_client
        .get("https://eu.api.blizzard.com/profile/user/wow?namespace=profile-eu&locale=fr_FR")
        .bearer_auth(&token_data.access_token)
        .send()
        .await
    {
        if wow_res.status().is_success() {
            if let Ok(val) = wow_res.json::<Value>().await {
                if let Some(waccs) = val.get("wow_accounts").and_then(|v| v.as_array()) {
                    let mut highest_char: Option<&Value> = None;
                    for wacc in waccs {
                        if let Some(chars) = wacc.get("characters").and_then(|c| c.as_array()) {
                            for c in chars {
                                let lvl = c.get("level").and_then(|l| l.as_i64()).unwrap_or(0);
                                let prev_lvl = highest_char.and_then(|hc| hc.get("level")).and_then(|l| l.as_i64()).unwrap_or(0);
                                if highest_char.is_none() || lvl > prev_lvl {
                                    highest_char = Some(c);
                                }
                            }
                        }
                    }
                    if let Some(hc) = highest_char {
                        let realm_slug = hc.get("realm").and_then(|r| r.get("slug")).and_then(|s| s.as_str()).unwrap_or("");
                        let name = hc.get("name").and_then(|n| n.as_str()).unwrap_or("");
                        if !realm_slug.is_empty() && !name.is_empty() {
                            let media_url = format!(
                                "https://eu.api.blizzard.com/profile/wow/character/{}/{}/character-media?namespace=profile-eu",
                                realm_slug,
                                urlencoding::encode(&name.to_lowercase())
                            );
                            if let Ok(media_res) = http_client.get(&media_url).bearer_auth(&token_data.access_token).send().await {
                                if let Ok(mval) = media_res.json::<Value>().await {
                                    if let Some(assets) = mval.get("assets").and_then(|a| a.as_array()) {
                                        for asset in assets {
                                            if asset.get("key").and_then(|k| k.as_str()) == Some("avatar") {
                                                if let Some(val_url) = asset.get("value").and_then(|v| v.as_str()) {
                                                    avatar_url = Some(val_url.to_string());
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let record = BattleNetAccountRecord {
        account_id,
        battletag,
        avatar_url,
        access_token: token_data.access_token,
        refresh_token: token_data.refresh_token.unwrap_or_default(),
        expires_at,
    };

    // Persist in SQLite
    let conn = db::open_db().map_err(|e| e.to_string())?;
    db::save_battlenet_account(&conn, &record).map_err(|e| e.to_string())?;

    Ok(record)
}

/// Refreshes the Blizzard access token if expired or near expiry (within 5 minutes).
pub async fn ensure_valid_blizzard_account() -> Result<Option<BattleNetAccountRecord>, String> {
    let conn = db::open_db().map_err(|e| e.to_string())?;
    let account_opt = db::get_battlenet_account(&conn).map_err(|e| e.to_string())?;

    let mut account = match account_opt {
        Some(a) => a,
        None => return Ok(None),
    };

    if account.account_id.starts_with("demo-")
        || account.battletag.contains("LoomPlayer")
        || account.battletag.contains("LoomChampion")
    {
        let _ = db::delete_battlenet_account(&conn);
        return Ok(None);
    }

    let now_ts = chrono::Utc::now().timestamp();
    if account.expires_at > now_ts + 300 {
        return Ok(Some(account));
    }

    // Expired or close to expiry: refresh
    let (client_id_opt, client_secret_opt) = get_blizzard_credentials();
    let (client_id, client_secret) = match (client_id_opt, client_secret_opt) {
        (Some(id), Some(sec)) if !account.refresh_token.is_empty() => (id, sec),
        _ => return Ok(Some(account)),
    };

    println!("[Loom::BlizzardOAuth] Rafraîchissement du jeton d'accès pour {}", account.battletag);

    let client = Client::new();
    let resp = client
        .post("https://oauth.battle.net/token")
        .basic_auth(&client_id, Some(&client_secret))
        .form(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", &account.refresh_token),
        ])
        .send()
        .await;

    match resp {
        Ok(res) if res.status().is_success() => {
            if let Ok(token_data) = res.json::<BlizzardTokenResponse>().await {
                account.access_token = token_data.access_token;
                if let Some(new_rt) = token_data.refresh_token {
                    account.refresh_token = new_rt;
                }
                account.expires_at = now_ts + token_data.expires_in;
                let _ = db::save_battlenet_account(&conn, &account);
                println!("[Loom::BlizzardOAuth] Jeton rafraîchi avec succès pour {}", account.battletag);
            }
        }
        _ => {
            println!("[Loom::BlizzardOAuth] Impossible de rafraîchir le jeton Blizzard, conservation de l'état actuel.");
        }
    }

    Ok(Some(account))
}

/// Disconnects the linked Blizzard account.
pub fn disconnect_blizzard_account() -> Result<(), String> {
    let conn = db::open_db().map_err(|e| e.to_string())?;
    db::delete_battlenet_account(&conn).map_err(|e| e.to_string())
}

/// Returns the owned Blizzard games catalog with installation status and rich metadata.
pub async fn get_blizzard_owned_games() -> Result<Vec<BlizzardGameEntitlement>, String> {
    let conn = db::open_db().map_err(|e| e.to_string())?;
    let local_games = db::get_battlenet_games(&conn).unwrap_or_default();
    let local_map: HashMap<String, BattleNetGameRecord> = local_games
        .into_iter()
        .map(|g| (g.product_uid.to_lowercase(), g))
        .collect();

    // Standard high-quality Blizzard / Battle.net catalogue games
    let catalogue = vec![
        (
            "pro",
            "Overwatch 2",
            "https://images.igdb.com/igdb/image/upload/t_cover_big/co885f.jpg",
            "https://shared.fastly.steamstatic.com/store_item_assets/steam/apps/2357570/library_hero.jpg",
            "Jeu de tir par équipe palpitant mettant en scène des héros aux pouvoirs uniques. Gratuit pour tous.",
        ),
        (
            "hs",
            "Hearthstone",
            "https://images.igdb.com/igdb/image/upload/t_cover_big/co1sh2.jpg",
            "https://blz-contentstack-images.akamaized.net/v3/assets/bltc965041283bac56c/blt2a00940d7f28339b/64bea585e26c5443e04bc2a4/Masthead.png",
            "Jeu de cartes stratégique effréné dans la taverne de Warcraft. Duels intenses, sorts dévastateurs et serviteurs légendaires.",
        ),
        (
            "sc2",
            "StarCraft II",
            "https://images.igdb.com/igdb/image/upload/t_cover_big/co1tnn.jpg",
            "https://blz-contentstack-images.akamaized.net/v3/assets/blt9c12f249ac15c7ec/bltb017c0048472fd04/69646379b1b0ff9682e39241/terran_background_2600.webp",
            "Le monument mondial de la stratégie en temps réel. Menez Terrans, Protoss ou Zergs vers la victoire. Campagne & Multi gratuits.",
        ),
        (
            "hero",
            "Heroes of the Storm",
            "https://lutris.net/media/igdb/hots_grid_264x352.webp",
            "https://blz-contentstack-images.akamaized.net/v3/assets/blt9c12f249ac15c7ec/blta8d048de9718bc10/6a97a0ae2924f59b018aa920/masthead_desktop.png",
            "Les héros emblématiques des univers Warcraft, StarCraft et Diablo réunis dans des batailles d'arène intenses.",
        ),
        (
            "anbs",
            "Diablo Immortal",
            "https://images.igdb.com/igdb/image/upload/t_cover_big/co2pwy.jpg",
            "https://blz-contentstack-images.akamaized.net/v3/assets/blt9c12f249ac15c7ec/bltd1d54d0ae64bba83/6a2053f8927dbbbf1e03bc77/di-masthead-bg_XL_warlock_2600.webp",
            "MMO Action-RPG impitoyable se déroulant entre Diablo II et Diablo III. Affrontez les serviteurs de Skarn.",
        ),
        (
            "odin",
            "Call of Duty: Warzone",
            "https://images.igdb.com/igdb/image/upload/t_cover_big/coa8id.jpg",
            "https://shared.fastly.steamstatic.com/store_item_assets/steam/apps/1938090/library_hero.jpg",
            "L'expérience Battle Royale massive et gratuite de Call of Duty. Combats tactiques et guerre totale.",
        ),
        (
            "wow",
            "World of Warcraft",
            "https://images.igdb.com/igdb/image/upload/t_cover_big/co8b7k.jpg",
            "https://blz-contentstack-images.akamaized.net/v3/assets/blt9c12f249ac15c7ec/bltfe806acfa8672377/6a87bb854b66aa218691cf56/BG_Masthead_2600_Desktop.jpg",
            "Plongez dans Azeroth avec The War Within. Explorez les profondeurs inconnues et défendez votre faction.",
        ),
        (
            "wow_classic",
            "World of Warcraft Classic",
            "https://images.igdb.com/igdb/image/upload/t_cover_big/co1trz.jpg",
            "https://blz-contentstack-images.akamaized.net/v3/assets/blt9c12f249ac15c7ec/bltfe806acfa8672377/6a87bb854b66aa218691cf56/BG_Masthead_2600_Desktop.jpg",
            "Revivez l'expérience authentique de WoW originel, Season of Discovery et les serveurs Hardcore.",
        ),
        (
            "fenris",
            "Diablo IV",
            "https://images.igdb.com/igdb/image/upload/t_cover_big/co69sm.jpg",
            "https://shared.fastly.steamstatic.com/store_item_assets/steam/apps/2344520/library_hero.jpg",
            "L'obscurité s'étend sur Sanctuaire. Combattez les hordes de Lilith dans cet Action-RPG d'anthologie.",
        ),
        (
            "osi",
            "Diablo II: Resurrected",
            "https://images.igdb.com/igdb/image/upload/t_cover_big/co3gbk.jpg",
            "https://blz-contentstack-images.akamaized.net/v3/assets/blt9c12f249ac15c7ec/blt6c60900c9bb4c4d8/6972a545d855f0a07c5d7824/masthead_overview_2600.png",
            "La renaissance fidèle du maître incontesté du hack'n'slash avec audio et graphismes 4K.",
        ),
        (
            "diablo3",
            "Diablo III",
            "https://images.igdb.com/igdb/image/upload/t_cover_big/co2l1u.jpg",
            "https://blz-contentstack-images.akamaized.net/v3/assets/blt9c12f249ac15c7ec/blt9843230b12d957ea/6a8f5e2b2924f5fb788aa851/masthead_2600.png",
            "Défiez les Seigneurs des Enfers et amassez un butin légendaire à travers Sanctuaire.",
        ),
        (
            "w3",
            "Warcraft III: Reforged",
            "https://images.igdb.com/igdb/image/upload/t_cover_big/co1rrv.jpg",
            "https://blz-contentstack-images.akamaized.net/v3/assets/blt9c12f249ac15c7ec/blt668d39048ffb606f/6a9792a96cec0d4229b141a2/masthead_desktop.png",
            "Revivez l'épopée fondatrice de Warcraft avec des graphismes remasterisés et l'éditeur de cartes culte.",
        ),
        (
            "s1",
            "StarCraft: Remastered",
            "https://images.igdb.com/igdb/image/upload/t_cover_big/co3wal.jpg",
            "https://blz-contentstack-images.akamaized.net/v3/assets/blt9c12f249ac15c7ec/bltb017c0048472fd04/69646379b1b0ff9682e39241/terran_background_2600.webp",
            "Le classique originel remasterisé en 4K avec audio haute fidélité.",
        ),
        (
            "w1r",
            "Warcraft I: Remastered",
            "https://images.igdb.com/igdb/image/upload/t_cover_big/co1xup.jpg",
            "https://blz-contentstack-images.akamaized.net/v3/assets/blt9c12f249ac15c7ec/blt668d39048ffb606f/6a9792a96cec0d4229b141a2/masthead_desktop.png",
            "L'origine légendaire de la guerre entre Humains et Orcs, remasterisée en haute définition.",
        ),
        (
            "w2r",
            "Warcraft II: Remastered",
            "https://images.igdb.com/igdb/image/upload/t_cover_big/co3wix.jpg",
            "https://blz-contentstack-images.akamaized.net/v3/assets/blt9c12f249ac15c7ec/blt668d39048ffb606f/6a9792a96cec0d4229b141a2/masthead_desktop.png",
            "Tides of Darkness et Beyond the Dark Portal dans une version remasterisée moderne.",
        ),
        (
            "rtro",
            "Blizzard Arcade Collection",
            "https://lutris.net/media/igdb/blizarcade.jpg",
            "https://blz-contentstack-images.akamaized.net/v3/assets/blt9c12f249ac15c7ec/blta8d048de9718bc10/6a97a0ae2924f59b018aa920/masthead_desktop.png",
            "The Lost Vikings, Blackthorne et Rock N' Roll Racing dans leurs versions définitives.",
        ),
    ];

    // Attempt real WoW profile stats if account is connected with valid token
    let real_wow_stats = if let Ok(Some(acc)) = ensure_valid_blizzard_account().await {
        let http_client = Client::new();
        if let Ok(res) = http_client
            .get("https://eu.api.blizzard.com/profile/user/wow?namespace=profile-eu&locale=fr_FR")
            .bearer_auth(&acc.access_token)
            .send()
            .await
        {
            if res.status().is_success() {
                if let Ok(val) = res.json::<serde_json::Value>().await {
                    let mut char_count = 0;
                    let mut highest_level: i64 = 0;
                    let mut highest_char_desc = None;
                    if let Some(wow_accounts) = val.get("wow_accounts").and_then(|v| v.as_array()) {
                        for wacc in wow_accounts {
                            if let Some(chars) = wacc.get("characters").and_then(|c| c.as_array()) {
                                char_count += chars.len();
                                for c in chars {
                                    let lvl = c.get("level").and_then(|l| l.as_i64()).unwrap_or(0);
                                    if lvl > highest_level || highest_char_desc.is_none() {
                                        highest_level = lvl;
                                        let c_name = c.get("name").and_then(|n| n.as_str()).unwrap_or("");
                                        let c_class = c.get("playable_class").and_then(|p| p.get("name")).and_then(|n| n.as_str()).unwrap_or("");
                                        let c_realm = c.get("realm").and_then(|r| r.get("name")).and_then(|n| n.as_str()).unwrap_or("");
                                        highest_char_desc = Some(format!("{} ({} Niv. {}) - {}", c_name, c_class, lvl, c_realm));
                                    }
                                }
                            }
                        }
                    }
                    if char_count > 0 {
                        Some(serde_json::json!({
                            "charactersCount": char_count,
                            "character": highest_char_desc.unwrap_or_else(|| "Personnage WoW".to_string()),
                            "profileSynced": true
                        }))
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    let mut entitlements = Vec::new();

    for (uid, name, cover, banner, summary) in catalogue {
        let is_installed;
        let install_path;
        let version_build;

        if let Some(installed_rec) = local_map.get(uid) {
            is_installed = installed_rec.is_installed;
            install_path = Some(installed_rec.install_path.clone());
            version_build = installed_rec.version_build.clone();
        } else {
            is_installed = false;
            install_path = None;
            version_build = None;
        }

        let stats = match uid {
            "wow" => real_wow_stats.clone(),
            _ => None,
        };

        entitlements.push(BlizzardGameEntitlement {
            product_uid: uid.to_string(),
            name: name.to_string(),
            is_owned: true,
            is_installed,
            install_path,
            version_build,
            cover_url: Some(cover.to_string()),
            banner_url: Some(banner.to_string()),
            summary: Some(summary.to_string()),
            stats,
        });
    }

    Ok(entitlements)
}
