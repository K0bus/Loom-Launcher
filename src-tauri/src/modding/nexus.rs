use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::modding::symlink::get_mods_cache_dir;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NxmUriInfo {
    pub raw_uri: String,
    pub game_domain: String,
    pub mod_id: u64,
    pub file_id: u64,
    pub key: Option<String>,
    pub expires: Option<u64>,
    pub user_id: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NexusDownloadLink {
    pub name: String,
    pub short_name: Option<String>,
    #[serde(alias = "URI", alias = "uri")]
    pub uri: String,
}

/// Parses an NXM URI:
/// `nxm://skyrimspecialedition/mods/2014/files/8542?key=abcdef123456&expires=1700000000&user_id=12345`
pub fn parse_nxm_uri(uri_str: &str) -> Result<NxmUriInfo, String> {
    let trimmed = uri_str.trim();
    if !trimmed.starts_with("nxm://") {
        return Err(format!("Invalid NXM scheme: {}", trimmed));
    }

    let without_scheme = &trimmed["nxm://".len()..];
    let mut parts = without_scheme.splitn(2, '?');
    let path_part = parts.next().unwrap_or("");
    let query_part = parts.next().unwrap_or("");

    // Path should look like: {game_domain}/mods/{mod_id}/files/{file_id}
    let segments: Vec<&str> = path_part.split('/').filter(|s| !s.is_empty()).collect();
    if segments.len() < 4 || segments[1] != "mods" || segments[3] != "files" && segments.len() < 4 {
        // Fallback: segments = [game_domain, "mods", mod_id, "files", file_id]
    }

    let game_domain = segments.first().ok_or("Missing game domain in NXM URI")?.to_string();
    let mod_id_idx = segments.iter().position(|&s| s == "mods").ok_or("Missing /mods/ segment")? + 1;
    let mod_id: u64 = segments
        .get(mod_id_idx)
        .ok_or("Missing mod ID")?
        .parse()
        .map_err(|e| format!("Invalid mod ID: {}", e))?;

    let file_id_idx = segments.iter().position(|&s| s == "files").ok_or("Missing /files/ segment")? + 1;
    let file_id: u64 = segments
        .get(file_id_idx)
        .ok_or("Missing file ID")?
        .parse()
        .map_err(|e| format!("Invalid file ID: {}", e))?;

    let mut key = None;
    let mut expires = None;
    let mut user_id = None;

    for param in query_part.split('&') {
        let mut kv = param.splitn(2, '=');
        let k = kv.next().unwrap_or("");
        let v = kv.next().unwrap_or("");
        match k {
            "key" => key = Some(v.to_string()),
            "expires" => expires = v.parse().ok(),
            "user_id" => user_id = v.parse().ok(),
            _ => {}
        }
    }

    Ok(NxmUriInfo {
        raw_uri: trimmed.to_string(),
        game_domain,
        mod_id,
        file_id,
        key,
        expires,
        user_id,
    })
}

/// Resolves mirror download links from the Nexus Mods v1 API
pub async fn resolve_nexus_download_links(
    client: &Client,
    info: &NxmUriInfo,
    api_key: Option<&str>,
) -> Result<Vec<NexusDownloadLink>, String> {
    let mut url = format!(
        "https://api.nexusmods.com/v1/games/{}/mods/{}/files/{}/download_link.json",
        info.game_domain, info.mod_id, info.file_id
    );

    let mut query_params = Vec::new();
    if let Some(ref k) = info.key {
        query_params.push(format!("key={}", k));
    }
    if let Some(exp) = info.expires {
        query_params.push(format!("expires={}", exp));
    }
    if let Some(uid) = info.user_id {
        query_params.push(format!("user_id={}", uid));
    }

    if !query_params.is_empty() {
        url = format!("{}?{}", url, query_params.join("&"));
    }

    let mut req = client.get(&url);
    if let Some(ak) = api_key {
        req = req.header("apikey", ak);
    }
    req = req.header("Application-Name", "Loom Launcher");
    req = req.header("Application-Version", "0.1.0");

    let resp = req
        .send()
        .await
        .map_err(|e| format!("Failed to call Nexus API: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Nexus API returned error {}: {}", status, body));
    }

    let links: Vec<NexusDownloadLink> = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse Nexus download links: {}", e))?;

    Ok(links)
}

/// Downloads a mod file via the mirror URI, following HTTP 302 redirects into `cache/mods/`
pub async fn download_nexus_file(
    client: &Client,
    download_url: &str,
    target_filename: Option<&str>,
) -> Result<PathBuf, String> {
    let cache_dir = get_mods_cache_dir();
    let resp = client
        .get(download_url)
        .send()
        .await
        .map_err(|e| format!("Failed to initiate Nexus file download: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Nexus download server returned HTTP {}", resp.status()));
    }

    // Determine filename from header, url, or fallback
    let filename = if let Some(custom) = target_filename {
        custom.to_string()
    } else {
        resp.url()
            .path_segments()
            .and_then(|mut segs| segs.next_back())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("nexus_mod_{}.zip", uuid::Uuid::new_v4().simple()))
    };

    let target_path = cache_dir.join(&filename);
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("Failed to download Nexus file body: {}", e))?;

    let mut file = File::create(&target_path)
        .map_err(|e| format!("Failed to create destination file {}: {}", target_path.display(), e))?;

    file.write_all(&bytes)
        .map_err(|e| format!("Failed to write file contents: {}", e))?;

    Ok(target_path)
}

/// Registers the `x-scheme-handler/nxm` MIME type for Loom on Linux
pub fn register_linux_desktop_mime() -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        let home = dirs::home_dir().ok_or("Cannot determine HOME directory")?;
        let apps_dir = home.join(".local/share/applications");
        let _ = fs::create_dir_all(&apps_dir);

        let local_bin_dir = home.join(".local/bin");
        let _ = fs::create_dir_all(&local_bin_dir);
        let bin_link = local_bin_dir.join("loom-launcher");

        let current_exe = std::env::current_exe()
            .unwrap_or_else(|_| PathBuf::from("loom-launcher"));

        // If in development target directory, ensure ~/.local/bin/loom-launcher points to it
        if current_exe.exists() {
            #[cfg(unix)]
            {
                let _ = fs::remove_file(&bin_link);
                let _ = std::os::unix::fs::symlink(&current_exe, &bin_link);
            }
        }

        let exec_target = if bin_link.exists() {
            bin_link.to_string_lossy().to_string()
        } else {
            current_exe.to_string_lossy().to_string()
        };

        let desktop_file = apps_dir.join("loom-launcher.desktop");
        let desktop_content = format!(
            r#"[Desktop Entry]
Name=Loom Launcher
Exec="{}" %u
Type=Application
MimeType=x-scheme-handler/nxm;
Terminal=false
Categories=Game;
"#,
            exec_target
        );
        fs::write(&desktop_file, desktop_content)
            .map_err(|e| format!("Failed to write desktop file: {}", e))?;

        // Update ~/.config/mimeapps.list explicitly
        let mimeapps = home.join(".config/mimeapps.list");
        if mimeapps.exists() {
            if let Ok(content) = fs::read_to_string(&mimeapps) {
                let mut lines: Vec<String> = Vec::new();
                for line in content.lines() {
                    if line.starts_with("x-scheme-handler/nxm=") {
                        continue;
                    }
                    lines.push(line.to_string());
                    if line.trim() == "[Default Applications]" || line.trim() == "[Added Associations]" {
                        lines.push("x-scheme-handler/nxm=loom-launcher.desktop;".to_string());
                    }
                }
                let _ = fs::write(&mimeapps, lines.join("\n"));
            }
        }

        // Also write ~/.local/share/applications/defaults.list for KDE Plasma fallback
        let defaults_list = apps_dir.join("defaults.list");
        let _ = fs::write(&defaults_list, "[Default Applications]\nx-scheme-handler/nxm=loom-launcher.desktop;\n");

        // Rebuild desktop database and KDE sycoca cache if available
        let _ = std::process::Command::new("update-desktop-database")
            .arg(&apps_dir)
            .output();
        let _ = std::process::Command::new("kbuildsycoca6")
            .arg("--noincremental")
            .output();

        // Register default with both gio and xdg-mime
        let _ = std::process::Command::new("gio")
            .args(["mime", "x-scheme-handler/nxm", "loom-launcher.desktop"])
            .output();

        let _ = std::process::Command::new("xdg-mime")
            .args(["default", "loom-launcher.desktop", "x-scheme-handler/nxm"])
            .output();
    }
    Ok(())
}
