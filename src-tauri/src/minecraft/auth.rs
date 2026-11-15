use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

const DEFAULT_CLIENT_ID: &str = "c36a9fb6-4f2a-41ff-90bd-ae7cc92031eb"; // Default public client ID for Minecraft Device Code flow

pub fn get_client_id() -> String {
    std::env::var("MICROSOFT_CLIENT_ID").unwrap_or_else(|_| DEFAULT_CLIENT_ID.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCodeResponse {
    pub user_code: String,
    pub device_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinecraftAccount {
    pub uuid: String,
    pub name: String,
    pub skin_url: Option<String>,
    pub raw_skin_url: Option<String>,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: i64,
    pub xuid: Option<String>,
    pub client_id: Option<String>,
    #[serde(default)]
    pub user_properties: Option<serde_json::Value>,
}

pub fn get_auth_file_path() -> PathBuf {
    let base = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string())).join(".local/share"))
        .join("loom");
    let _ = fs::create_dir_all(&base);
    base.join("minecraft_account.json")
}

pub fn load_stored_account() -> Option<MinecraftAccount> {
    let path = get_auth_file_path();
    if !path.exists() {
        return None;
    }
    let content = fs::read_to_string(path).ok()?;
    let mut acc: MinecraftAccount = serde_json::from_str(&content).ok()?;

    // Ensure skin_url points to the Minecraft player's head avatar, not raw flat texture
    let needs_migration = match &acc.skin_url {
        Some(url) => url.contains("textures.minecraft.net") || !url.contains("/avatar/"),
        None => true,
    };

    if needs_migration {
        if acc.raw_skin_url.is_none() {
            acc.raw_skin_url = acc.skin_url.clone();
        }
        acc.skin_url = Some(format!("https://mc-heads.net/avatar/{}/64", acc.uuid));
        let _ = save_stored_account(&acc);
    }

    Some(acc)
}

pub fn save_stored_account(acc: &MinecraftAccount) -> Result<(), String> {
    let path = get_auth_file_path();
    let json = serde_json::to_string_pretty(acc).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())
}

pub fn remove_stored_account() -> Result<(), String> {
    let path = get_auth_file_path();
    if path.exists() {
        fs::remove_file(path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Returns a valid account, automatically refreshing the tokens if near expiry
pub async fn get_valid_account(client: &Client) -> Option<MinecraftAccount> {
    let mut account = load_stored_account()?;
    let now = chrono::Utc::now().timestamp();

    // If token expires in less than 5 minutes or is already expired, refresh it
    if account.expires_at <= now + 300 {
        println!(
            "[Loom::Auth] Jeton Minecraft expirant prochainement (expires_at: {}, now: {}). Rafraîchissement automatique...",
            account.expires_at, now
        );

        match refresh_account_token(client, &account.refresh_token).await {
            Ok(refreshed) => {
                println!(
                    "[Loom::Auth] Jeton rafraîchi avec succès pour le joueur {}. Persistance mise à jour.",
                    refreshed.name
                );
                account = refreshed;
            }
            Err(err) => {
                eprintln!(
                    "[Loom::Auth] Échec du rafraîchissement ({}), conservation du compte pour session hors-ligne.",
                    err
                );
            }
        }
    }

    Some(account)
}

/// Initiates Device Code Flow with Microsoft identity provider
pub async fn initiate_device_code(client: &Client) -> Result<DeviceCodeResponse, String> {
    let url = "https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode";
    let client_id = get_client_id();
    let params = [
        ("client_id", client_id.as_str()),
        ("scope", "XboxLive.SignIn XboxLive.offline_access"),
    ];

    let resp = client
        .post(url)
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("Erreur lors de la requête devicecode Microsoft: {}", e))?;

    if !resp.status().is_success() {
        let err_body = resp.text().await.unwrap_or_default();
        return Err(format!("L'API Microsoft a retourné une erreur: {}", err_body));
    }

    let val: Value = resp.json().await.map_err(|e| e.to_string())?;

    let user_code = val["user_code"].as_str().unwrap_or("").to_string();
    let device_code = val["device_code"].as_str().unwrap_or("").to_string();
    let verification_uri = val["verification_uri"]
        .as_str()
        .unwrap_or("https://microsoft.com/link")
        .to_string();
    let expires_in = val["expires_in"].as_u64().unwrap_or(900);
    let interval = val["interval"].as_u64().unwrap_or(5);
    let message = val["message"].as_str().unwrap_or("").to_string();

    Ok(DeviceCodeResponse {
        user_code,
        device_code,
        verification_uri,
        expires_in,
        interval,
        message,
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PollAuthResult {
    pub pending: bool,
    pub account: Option<MinecraftAccount>,
    pub error: Option<String>,
}

/// Polls Microsoft token endpoint until user approves or declines
pub async fn poll_device_code(client: &Client, device_code: &str) -> Result<PollAuthResult, String> {
    let url = "https://login.microsoftonline.com/consumers/oauth2/v2.0/token";
    let client_id = get_client_id();
    let params = [
        ("client_id", client_id.as_str()),
        ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
        ("device_code", device_code),
    ];

    let resp = client
        .post(url)
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("Erreur réseau polling Microsoft: {}", e))?;

    let body: Value = resp.json().await.map_err(|e| e.to_string())?;

    if let Some(err) = body["error"].as_str() {
        if err == "authorization_pending" {
            return Ok(PollAuthResult {
                pending: true,
                account: None,
                error: None,
            });
        }
        if err == "slow_down" {
            return Ok(PollAuthResult {
                pending: true,
                account: None,
                error: None,
            });
        }
        if err == "authorization_declined" {
            return Ok(PollAuthResult {
                pending: false,
                account: None,
                error: Some("Authentification refusée par l'utilisateur.".to_string()),
            });
        }
        if err == "expired_token" {
            return Ok(PollAuthResult {
                pending: false,
                account: None,
                error: Some("Le code d'authentification a expiré. Veuillez réessayer.".to_string()),
            });
        }
        return Ok(PollAuthResult {
            pending: false,
            account: None,
            error: Some(format!("Erreur Microsoft: {}", err)),
        });
    }

    let ms_access_token = match body["access_token"].as_str() {
        Some(t) => t.to_string(),
        None => {
            return Ok(PollAuthResult {
                pending: true,
                account: None,
                error: None,
            })
        }
    };
    let ms_refresh_token = body["refresh_token"].as_str().unwrap_or("").to_string();

    // Authenticate through Xbox Live, XSTS, and Mojang
    let account = exchange_ms_token_for_minecraft(client, &ms_access_token, &ms_refresh_token).await?;
    save_stored_account(&account)?;

    Ok(PollAuthResult {
        pending: false,
        account: Some(account),
        error: None,
    })
}

/// Uses Microsoft refresh_token to obtain a fresh token set and updates persistence
pub async fn refresh_account_token(client: &Client, refresh_token: &str) -> Result<MinecraftAccount, String> {
    let url = "https://login.microsoftonline.com/consumers/oauth2/v2.0/token";
    let client_id = get_client_id();
    let params = [
        ("client_id", client_id.as_str()),
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
        ("scope", "XboxLive.SignIn XboxLive.offline_access"),
    ];

    let resp = client
        .post(url)
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("Erreur rafraîchissement jeton Microsoft: {}", e))?;

    if !resp.status().is_success() {
        let err_text = resp.text().await.unwrap_or_default();
        return Err(format!("Échec du rafraîchissement Microsoft: {}", err_text));
    }

    let body: Value = resp.json().await.map_err(|e| e.to_string())?;

    let new_access_token = body["access_token"]
        .as_str()
        .ok_or_else(|| "Jeton d'accès manquant dans la réponse de rafraîchissement".to_string())?;

    let new_refresh_token = body["refresh_token"]
        .as_str()
        .unwrap_or(refresh_token);

    let account = exchange_ms_token_for_minecraft(client, new_access_token, new_refresh_token).await?;
    save_stored_account(&account)?;

    Ok(account)
}

/// Full OAuth -> XBL -> XSTS -> Mojang token exchange pipeline
pub async fn exchange_ms_token_for_minecraft(
    client: &Client,
    ms_token: &str,
    ms_refresh: &str,
) -> Result<MinecraftAccount, String> {
    // 1. Xbox Live User Authentication (XBL)
    let xbl_payload = serde_json::json!({
        "Properties": {
            "AuthMethod": "RPS",
            "SiteName": "user.auth.xboxlive.com",
            "RpsTicket": format!("d={}", ms_token)
        },
        "RelyingParty": "http://auth.xboxlive.com",
        "TokenType": "JWT"
    });

    let xbl_resp = client
        .post("https://user.auth.xboxlive.com/user/authenticate")
        .json(&xbl_payload)
        .send()
        .await
        .map_err(|e| format!("Erreur connexion Xbox Live (XBL): {}", e))?;

    if !xbl_resp.status().is_success() {
        let status = xbl_resp.status();
        let body = xbl_resp.text().await.unwrap_or_default();
        return Err(format!("Échec XBL (status {}): {}", status, body));
    }

    let xbl_json: Value = xbl_resp.json().await.map_err(|e| e.to_string())?;

    let xbl_token = xbl_json["Token"]
        .as_str()
        .ok_or_else(|| "Jeton XBL manquant".to_string())?;
    let uhs = xbl_json["DisplayClaims"]["xui"][0]["uhs"]
        .as_str()
        .ok_or_else(|| "Hash utilisateur XBL (uhs) manquant".to_string())?;
    let xuid = xbl_json["DisplayClaims"]["xui"][0]["xid"]
        .as_str()
        .map(String::from);

    // 2. Xbox Security Token Service (XSTS)
    let xsts_payload = serde_json::json!({
        "Properties": {
            "SandboxId": "RETAIL",
            "UserTokens": [xbl_token]
        },
        "RelyingParty": "rp://api.minecraftservices.com/",
        "TokenType": "JWT"
    });

    let xsts_resp = client
        .post("https://xsts.auth.xboxlive.com/xsts/authorize")
        .json(&xsts_payload)
        .send()
        .await
        .map_err(|e| format!("Erreur autorisation XSTS: {}", e))?;

    let xsts_json: Value = xsts_resp.json().await.map_err(|e| e.to_string())?;

    if let Some(xerr) = xsts_json["XErr"].as_u64() {
        match xerr {
            2148916233 => return Err("Ce compte Microsoft ne possède pas de profil Xbox Live. Créez-en un sur xbox.com.".to_string()),
            2148916238 => return Err("Ce compte est un compte enfant nécessitant l'accord d'un tuteur légal.".to_string()),
            2148916236 | 2148916237 => return Err("Ce compte nécessite une vérification d'âge sur Xbox Live.".to_string()),
            _ => return Err(format!("Erreur XSTS code {}", xerr)),
        }
    }

    let xsts_token = xsts_json["Token"]
        .as_str()
        .ok_or_else(|| "Jeton XSTS manquant".to_string())?;

    // 3. Mojang Login With Xbox
    let mojang_payload = serde_json::json!({
        "identityToken": format!("XBL3.0 x={};{}", uhs, xsts_token)
    });

    let mojang_resp = client
        .post("https://api.minecraftservices.com/authentication/login_with_xbox")
        .json(&mojang_payload)
        .send()
        .await
        .map_err(|e| format!("Erreur authentification Mojang: {}", e))?;

    if !mojang_resp.status().is_success() {
        let status = mojang_resp.status();
        let body = mojang_resp.text().await.unwrap_or_default();
        return Err(format!("Échec Mojang login (status {}): {}", status, body));
    }

    let mojang_json: Value = mojang_resp.json().await.map_err(|e| e.to_string())?;

    let mojang_token = mojang_json["access_token"]
        .as_str()
        .ok_or_else(|| "Jeton d'accès Mojang manquant".to_string())?;
    let expires_in = mojang_json["expires_in"].as_i64().unwrap_or(86400);

    // 4. Mojang Profile (Player UUID, Gamertag, Skin)
    let profile_resp = client
        .get("https://api.minecraftservices.com/minecraft/profile")
        .bearer_auth(mojang_token)
        .send()
        .await
        .map_err(|e| format!("Erreur récupération profil Minecraft: {}", e))?;

    if !profile_resp.status().is_success() {
        let status = profile_resp.status();
        if status.as_u16() == 404 {
            return Err("Ce compte Microsoft ne possède pas de licence Minecraft: Java Edition active.".to_string());
        }
        return Err(format!("Erreur lors de la lecture du profil Minecraft: status {}", status));
    }

    let profile_json: Value = profile_resp.json().await.map_err(|e| e.to_string())?;

    let uuid = profile_json["id"]
        .as_str()
        .ok_or_else(|| "UUID manquant dans le profil Minecraft".to_string())?
        .to_string();

    let name = profile_json["name"]
        .as_str()
        .ok_or_else(|| "Pseudo joueur manquant dans le profil Minecraft".to_string())?
        .to_string();

    let raw_skin_url = profile_json["skins"]
        .as_array()
        .and_then(|a| a.iter().find(|s| s["state"].as_str() == Some("ACTIVE")).or_else(|| a.first()))
        .and_then(|s| s["url"].as_str())
        .map(String::from);

    let dashed_uuid = if uuid.contains('-') {
        uuid
    } else if uuid.len() == 32 {
        format!("{}-{}-{}-{}-{}", &uuid[0..8], &uuid[8..12], &uuid[12..16], &uuid[16..20], &uuid[20..32])
    } else {
        uuid
    };

    let head_avatar_url = format!("https://mc-heads.net/avatar/{}/64", dashed_uuid);
    let now_sec = chrono::Utc::now().timestamp();

    Ok(MinecraftAccount {
        uuid: dashed_uuid,
        name,
        skin_url: Some(head_avatar_url),
        raw_skin_url,
        access_token: mojang_token.to_string(),
        refresh_token: ms_refresh.to_string(),
        expires_at: now_sec + expires_in,
        xuid,
        client_id: Some(get_client_id()),
        user_properties: None,
    })
}
