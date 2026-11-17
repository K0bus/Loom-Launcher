//! Liens `nxm://` (Nexus Mods « Mod Manager Download ») :
//! - enregistrement de Loom natif comme gestionnaire par défaut,
//! - instance unique (un second lancement transmet ses liens à l'instance ouverte),
//! - téléchargement, extraction, installation dans le profil Bethesda et déploiement
//!   (port de `handle_nxm_uri_internal`, src-tauri/src/lib.rs).

use crate::modding::nexus::{download_nexus_file, fetch_nexus_mod_details, parse_nxm_uri, resolve_nexus_download_links};
use crate::modding::{BethesdaAdapter, GameAdapter};
use crate::pages::with_app;
use crate::{ctx, db, toast, Beth, Modding};
use slint::ComponentHandle;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};

const DESKTOP_ID: &str = "loom-native.desktop";

fn socket_path() -> PathBuf {
    std::env::var("XDG_RUNTIME_DIR").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("/tmp")).join("loom-native.sock")
}

pub fn uris_from_args() -> Vec<String> {
    std::env::args().skip(1).filter(|a| a.to_lowercase().starts_with("nxm://")).collect()
}

/// Si une instance tourne déjà : lui transmet les liens (ou une demande d'affichage) et renvoie `true`.
pub fn forward_to_running(uris: &[String]) -> bool {
    let Ok(mut s) = UnixStream::connect(socket_path()) else { log(&format!("lancement à froid ({} lien(s))", uris.len())); return false };
    log(&format!("instance existante : transmission de {} lien(s)", uris.len()));
    if uris.is_empty() { let _ = writeln!(s, "RAISE"); } else { for u in uris { let _ = writeln!(s, "{u}"); } }
    true
}

/// Écoute les liens envoyés par les invocations suivantes.
pub fn start_server() {
    let path = socket_path();
    let _ = std::fs::remove_file(&path); // socket orpheline d'une session précédente
    let Ok(listener) = UnixListener::bind(&path) else { eprintln!("[nxm] impossible d'ouvrir {}", path.display()); return };
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            for line in BufReader::new(stream).lines().flatten() {
                let line = line.trim().to_string();
                log(&format!("socket : {}", if line.starts_with("nxm://") { "lien transmis" } else { "demande d'affichage" }));
                if line == "RAISE" { with_app(|a| { a.window().set_minimized(false); a.window().show().ok(); }); }
                else if line.to_lowercase().starts_with("nxm://") { handle(line); }
            }
        }
    });
}

/// Enregistre Loom natif comme gestionnaire de `x-scheme-handler/nxm` (si ce n'est pas déjà le cas).
pub fn register_handler(force: bool) -> Result<(), String> {
    let home = dirs::home_dir().ok_or("HOME introuvable")?;
    // `xdg-mime query` est peu fiable sous KDE (qtpaths absent) : on lit mimeapps.list directement
    let mimeapps = std::fs::read_to_string(home.join(".config/mimeapps.list")).unwrap_or_default();
    let nxm_lines: Vec<&str> = mimeapps.lines().filter(|l| l.trim_start().starts_with("x-scheme-handler/nxm=")).collect();
    let ours_only = !nxm_lines.is_empty() && nxm_lines.iter().all(|l| l.contains(DESKTOP_ID) && !l.contains("loom-launcher"));
    let tauri_claims = std::fs::read_to_string(home.join(".local/share/applications/loom-launcher.desktop")).map(|t| t.contains("x-scheme-handler/nxm")).unwrap_or(false);
    if !force && ours_only && !tauri_claims { return Ok(()); }

    let exe = std::env::var("APPIMAGE").map(PathBuf::from).or_else(|_| std::env::current_exe()).map_err(|e| e.to_string())?;
    let bin_dir = home.join(".local/bin");
    let _ = std::fs::create_dir_all(&bin_dir);
    let link = bin_dir.join("loom-native");
    let _ = std::fs::remove_file(&link);
    let exec = if std::os::unix::fs::symlink(&exe, &link).is_ok() { link } else { exe };

    let apps = home.join(".local/share/applications");
    let _ = std::fs::create_dir_all(&apps);
    let content = format!("[Desktop Entry]\nName=Loom Launcher (natif)\nExec=\"{}\" %u\nType=Application\nMimeType=x-scheme-handler/nxm;\nTerminal=false\nCategories=Game;\nNoDisplay=true\n", exec.display());
    std::fs::write(apps.join(DESKTOP_ID), content).map_err(|e| e.to_string())?;

    // Le .desktop du Loom Tauri revendique aussi nxm:// : on retire cette revendication
    let tauri_desktop = apps.join("loom-launcher.desktop");
    if let Ok(t) = std::fs::read_to_string(&tauri_desktop) {
        if t.contains("x-scheme-handler/nxm") {
            let kept: Vec<&str> = t.lines().filter(|l| !l.starts_with("MimeType=") || !l.contains("x-scheme-handler/nxm")).collect();
            let _ = std::fs::write(&tauri_desktop, kept.join("\n") + "\n");
        }
    }
    // Retire les anciennes associations nxm (celles du Loom Tauri) qui passeraient avant la nôtre
    for f in [home.join(".config/mimeapps.list"), apps.join("mimeapps.list"), apps.join("defaults.list")] {
        if let Ok(txt) = std::fs::read_to_string(&f) {
            if txt.lines().any(|l| l.trim_start().starts_with("x-scheme-handler/nxm=")) {
                let kept: Vec<&str> = txt.lines().filter(|l| !l.trim_start().starts_with("x-scheme-handler/nxm=")).collect();
                let _ = std::fs::write(&f, kept.join("\n") + "\n");
            }
        }
    }
    let run = |bin: &str, args: &[&str]| { let _ = std::process::Command::new(bin).args(args).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).output(); };
    run("update-desktop-database", &[&apps.to_string_lossy()]);
    run("xdg-mime", &["default", DESKTOP_ID, "x-scheme-handler/nxm"]);
    run("gio", &["mime", "x-scheme-handler/nxm", DESKTOP_ID]);
    run("kbuildsycoca6", &["--noincremental"]);
    Ok(())
}

fn nexus_api_key() -> Option<String> {
    let p = dirs::data_local_dir()?.join("loom/settings.json");
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(p).ok()?).ok()?;
    v["nexusApiKey"].as_str().map(|k| k.trim().to_string()).filter(|k| !k.is_empty())
}

fn extract_archive(archive: &Path, dest: &Path) -> Result<(), String> {
    let (a, d) = (archive.to_string_lossy().to_string(), dest.to_string_lossy().to_string());
    let ok = |o: std::io::Result<std::process::Output>| o.map(|o| o.status.success()).unwrap_or(false);
    if ok(std::process::Command::new("7z").args(["x", "-y", &format!("-o{d}"), &a]).output()) { return Ok(()); }
    if a.ends_with(".zip") && ok(std::process::Command::new("unzip").args(["-o", &a, "-d", &d]).output()) { return Ok(()); }
    if ok(std::process::Command::new("unar").args(["-f", "-o", &d, &a]).output()) { return Ok(()); }
    if let Ok(f) = std::fs::File::open(archive) { if let Ok(mut z) = zip::ZipArchive::new(f) { if z.extract(dest).is_ok() { return Ok(()); } } }
    Err(format!("Échec de l'extraction de l'archive : {}", archive.display()))
}

/// Ouvre le studio Bethesda du bon jeu (sur l'onglet des réglages si la clé API manque).
fn show_studio(game_idx: i32, tab: i32) {
    with_app(move |a| {
        a.set_page(1);
        a.global::<Modding>().set_screen(4);
        let b = a.global::<Beth>();
        b.set_game(game_idx);
        b.set_tab(tab);
        b.invoke_select_game(game_idx);
        b.set_tab(tab);
    });
}

/// Journal persistant (`~/.local/share/loom/logs/native-nxm.log`) — la clé de téléchargement est masquée.
pub fn log(msg: &str) {
    let dir = dirs::data_local_dir().unwrap_or_default().join("loom/logs");
    let _ = std::fs::create_dir_all(&dir);
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(dir.join("native-nxm.log")) {
        let masked = msg.split("key=").next().unwrap_or(msg).to_string() + if msg.contains("key=") { "key=***" } else { "" };
        let _ = writeln!(f, "{} {}", chrono::Local::now().format("%H:%M:%S"), masked);
    }
}

fn notify_user(body: &str) { let _ = crate::settings::notify("Loom — Nexus Mods", body); }

pub fn handle(uri: String) {
    log(&format!("lien reçu : {uri}"));
    // Retour immédiat et visible même si la fenêtre est cachée
    notify_user("Lien Nexus reçu, téléchargement en cours…");
    with_app(|a| { a.window().set_minimized(false); let _ = a.window().show(); });
    ctx().rt.spawn(async move {
        match process(&uri).await {
            Ok(_) => { log("terminé : mod installé et déployé"); notify_user("Mod installé et déployé"); }
            Err(e) => { eprintln!("[nxm] {e}"); log(&format!("ÉCHEC : {e}")); notify_user(&format!("Échec : {e}")); toast(e, 2); }
        }
    });
}

async fn process(uri: &str) -> Result<(), String> {
    let info = parse_nxm_uri(uri)?;
    let domain = info.game_domain.to_lowercase();
    let (game_id, game_idx) = match domain.as_str() {
        "skyrimspecialedition" | "skyrimse" | "skyrim" => ("skyrim_se", 0),
        "fallout4" => ("fallout4", 1),
        "oblivion" => ("oblivion", 2),
        other => return Err(format!("Jeu Nexus « {other} » non pris en charge par le studio Bethesda")),
    };
    show_studio(game_idx, 0);

    let Some(api_key) = nexus_api_key() else {
        show_studio(game_idx, 3);
        return Err("Clé API Nexus Mods requise : renseignez-la dans Réglages, puis recliquez sur « Mod Manager Download ».".into());
    };
    toast(format!("Téléchargement Nexus : mod {} / fichier {}…", info.mod_id, info.file_id), 0);

    let client = reqwest::Client::new();
    let links = resolve_nexus_download_links(&client, &info, Some(&api_key)).await?;
    let link = links.first().ok_or("Aucun miroir de téléchargement renvoyé par Nexus")?;
    let archive = download_nexus_file(&client, &link.uri, None).await?;

    // Profil du jeu (créé si absent)
    let conn = db::open_db().map_err(|e| e.to_string())?;
    let profile = match db::get_profiles(&conn, Some(game_id)).unwrap_or_default().into_iter().next() {
        Some(p) => p,
        None => {
            let rec = db::GameProfileRecord { id: format!("{game_id}-default"), game_id: game_id.into(), name: "Profil Principal (Par Défaut)".into(), game_version: "1.6+".into(), modloader: Some("skse64_loader.exe".into()), deployment_path: None, created_at: chrono::Utc::now().timestamp() };
            let _ = db::insert_profile(&conn, &rec);
            rec
        }
    };

    let staged = dirs::data_local_dir().unwrap_or_default().join("loom/profiles").join(game_id).join(&profile.id).join("staged").join(format!("{}_{}", info.mod_id, info.file_id));
    let _ = std::fs::create_dir_all(&staged);
    {
        let (a, s) = (archive.clone(), staged.clone());
        tokio::task::spawn_blocking(move || extract_archive(&a, &s)).await.map_err(|e| e.to_string())??;
    }
    let adapter: Box<dyn GameAdapter> = match game_id { "fallout4" => Box::new(BethesdaAdapter::new_fallout_4()), "oblivion" => Box::new(BethesdaAdapter::new_oblivion()), _ => Box::new(BethesdaAdapter::new_skyrim_se()) };
    let _ = adapter.normalize_staged_mod(&staged);

    let details = fetch_nexus_mod_details(&client, &info.game_domain, info.mod_id, Some(&api_key)).await.ok();
    let filename = archive.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_else(|| format!("nexus_mod_{}", info.mod_id));
    let name = details.map(|d| d.name).unwrap_or_else(|| filename.trim_end_matches(".zip").trim_end_matches(".7z").trim_end_matches(".rar").to_string());
    let pmod = db::ProfileModRecord {
        id: format!("pmod-{}", uuid::Uuid::new_v4().simple()), profile_id: profile.id.clone(), provider: "nexusmods".into(),
        remote_mod_id: info.mod_id.to_string(), remote_file_id: info.file_id.to_string(), name: name.clone(),
        file_path: archive.to_string_lossy().to_string(), filename, staged_path: Some(staged.to_string_lossy().to_string()), load_priority: 0, is_enabled: true,
    };
    db::insert_profile_mod(&conn, &pmod).map_err(|e| e.to_string())?;
    let _ = adapter.deploy_profile(&profile.id);

    toast(format!("« {name} » installé et déployé"), 1);
    crate::studio::beth_reload();
    Ok(())
}
