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
    serve(socket_path(), |line| {
        log(&format!("socket : {}", if line.to_lowercase().starts_with("nxm://") { "lien transmis" } else { "demande d'affichage" }));
        if line == "RAISE" { with_app(|a| { a.window().set_minimized(false); a.window().show().ok(); }); }
        else if line.to_lowercase().starts_with("nxm://") { handle(line); }
    });
}

/// Écoute `path` et appelle `on_line` pour chaque ligne reçue (un lien ou « RAISE »).
pub fn serve(path: PathBuf, on_line: impl Fn(String) + Send + 'static) {
    let _ = std::fs::remove_file(&path); // socket orpheline d'une session précédente
    let Ok(listener) = UnixListener::bind(&path) else { eprintln!("[nxm] impossible d'ouvrir {}", path.display()); return };
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            for line in BufReader::new(stream).lines().flatten() {
                let line = line.trim().to_string();
                if !line.is_empty() { on_line(line); }
            }
        }
    });
}

/// Envoie des lignes à une instance qui écoute sur `path`.
pub fn send_to(path: &Path, lines: &[String]) -> bool {
    let Ok(mut s) = UnixStream::connect(path) else { return false };
    for l in lines { let _ = writeln!(s, "{l}"); }
    true
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

async fn details_name(client: &reqwest::Client, info: &crate::modding::nexus::NxmUriInfo, key: &str) -> Option<String> {
    fetch_nexus_mod_details(client, &info.game_domain, info.mod_id, Some(key)).await.ok().map(|d| d.name)
}

/// Installe une archive de mod dans le profil du jeu : profil (créé si absent), extraction dans le
/// staging, normalisation selon le moteur du jeu, enregistrement en base puis déploiement.
/// Commun aux liens Nexus et à l'installation depuis un fichier local.
pub async fn install_archive(game_id: &str, archive: PathBuf, display_name: Option<String>, provider: &str, remote: (u64, u64)) -> Result<String, String> {
    let profile = {
        let conn = db::open_db().map_err(|e| e.to_string())?;
        match db::get_profiles(&conn, Some(game_id)).unwrap_or_default().into_iter().next() {
            Some(p) => p,
            None => {
                let rec = db::GameProfileRecord { id: format!("{game_id}-default"), game_id: game_id.into(), name: "Profil Principal (Par Défaut)".into(), game_version: "1.6+".into(), modloader: Some("skse64_loader.exe".into()), deployment_path: None, created_at: chrono::Utc::now().timestamp() };
                let _ = db::insert_profile(&conn, &rec);
                rec
            }
        }
    };
    install_archive_in_profile(&profile, game_id, archive, display_name, provider, remote).await
}

pub async fn install_archive_in_profile(profile: &db::GameProfileRecord, game_id: &str, archive: PathBuf, display_name: Option<String>, provider: &str, remote: (u64, u64)) -> Result<String, String> {
    let staged = dirs::data_local_dir().unwrap_or_default().join("loom/profiles").join(game_id).join(&profile.id).join("staged").join(format!("{}_{}", remote.0, remote.1));
    let _ = std::fs::create_dir_all(&staged);
    {
        let (a, s) = (archive.clone(), staged.clone());
        tokio::task::spawn_blocking(move || extract_archive(&a, &s)).await.map_err(|e| e.to_string())??;
    }
    let adapter: Box<dyn GameAdapter> = match game_id { "fallout4" => Box::new(BethesdaAdapter::new_fallout_4()), "oblivion" => Box::new(BethesdaAdapter::new_oblivion()), _ => Box::new(BethesdaAdapter::new_skyrim_se()) };
    let _ = adapter.normalize_staged_mod(&staged);

    let filename = archive.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_else(|| format!("mod_{}", remote.0));
    let name = display_name.unwrap_or_else(|| filename.trim_end_matches(".zip").trim_end_matches(".7z").trim_end_matches(".rar").to_string());
    let pmod = db::ProfileModRecord {
        id: format!("pmod-{}", uuid::Uuid::new_v4().simple()), profile_id: profile.id.clone(), provider: provider.into(),
        remote_mod_id: remote.0.to_string(), remote_file_id: remote.1.to_string(), name: name.clone(),
        file_path: archive.to_string_lossy().to_string(), filename, staged_path: Some(staged.to_string_lossy().to_string()), load_priority: 0, is_enabled: true,
    };
    {
        let conn = db::open_db().map_err(|e| e.to_string())?;
        db::insert_profile_mod(&conn, &pmod).map_err(|e| e.to_string())?;
    }
    let _ = adapter.deploy_profile(&profile.id);
    Ok(name)
}

/// Installation d'un fichier choisi sur le disque (zip / 7z / rar) dans le profil actif du jeu.
pub async fn install_local(game_id: &str, profile_id: &str, archive: PathBuf) -> Result<String, String> {
    if !archive.is_file() { return Err(format!("Fichier introuvable : {}", archive.display())); }
    let profile = {
        let conn = db::open_db().map_err(|e| e.to_string())?;
        db::get_profiles(&conn, Some(game_id)).unwrap_or_default().into_iter().find(|p| p.id == profile_id).ok_or("Profil introuvable")?
    };
    // Identifiant « distant » stable dérivé du chemin (évite les collisions de staging)
    let h = { use sha1::{Digest, Sha1}; let d = Sha1::digest(archive.to_string_lossy().as_bytes()); u64::from_be_bytes(d[..8].try_into().unwrap()) };
    install_archive_in_profile(&profile, game_id, archive, None, "local", (h, 0)).await
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

    let archive_name = details_name(&client, &info, &api_key).await;
    let name = install_archive(game_id, archive, archive_name, "nexusmods", (info.mod_id, info.file_id)).await?;
    toast(format!("« {name} » installé et déployé"), 1);
    crate::studio::beth_reload();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::Duration;

    fn sock(name: &str) -> PathBuf { std::env::temp_dir().join(format!("loom-test-{name}-{}.sock", std::process::id())) }

    #[test]
    fn links_are_forwarded_to_the_running_instance() {
        let path = sock("fwd");
        let (tx, rx) = mpsc::channel();
        serve(path.clone(), move |l| { let _ = tx.send(l); });
        std::thread::sleep(Duration::from_millis(100));
        let uri = "nxm://skyrimspecialedition/mods/12604/files/749043?key=abc&expires=1&user_id=2".to_string();
        assert!(send_to(&path, &[uri.clone()]));
        assert_eq!(rx.recv_timeout(Duration::from_secs(2)).unwrap(), uri);
    }

    #[test]
    fn several_links_arrive_in_order() {
        let path = sock("multi");
        let (tx, rx) = mpsc::channel();
        serve(path.clone(), move |l| { let _ = tx.send(l); });
        std::thread::sleep(Duration::from_millis(100));
        send_to(&path, &["RAISE".into(), "nxm://a/mods/1/files/2".into()]);
        assert_eq!(rx.recv_timeout(Duration::from_secs(2)).unwrap(), "RAISE");
        assert_eq!(rx.recv_timeout(Duration::from_secs(2)).unwrap(), "nxm://a/mods/1/files/2");
    }

    #[test]
    fn nothing_listening_means_cold_start() {
        assert!(!send_to(&sock("absent"), &["x".into()]));
    }

    #[test]
    fn nxm_uri_is_parsed() {
        let i = parse_nxm_uri("nxm://skyrimspecialedition/mods/2014/files/8542?key=k&expires=1700000000&user_id=12").unwrap();
        assert_eq!(i.game_domain, "skyrimspecialedition");
        assert_eq!((i.mod_id, i.file_id), (2014, 8542));
        assert_eq!(i.key.as_deref(), Some("k"));
        assert!(parse_nxm_uri("https://exemple.org").is_err());
    }

    #[test]
    fn archives_without_extractor_report_an_error() {
        let d = std::env::temp_dir().join(format!("loom-test-ex-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&d);
        let bad = d.join("pas_une_archive.zip");
        std::fs::write(&bad, b"ceci n'est pas un zip").unwrap();
        assert!(extract_archive(&bad, &d.join("out")).is_err());
    }

    fn make_zip(path: &Path, files: &[(&str, &[u8])]) {
        use std::io::Write;
        let mut z = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
        for (name, data) in files {
            z.start_file(*name, zip::write::SimpleFileOptions::default()).unwrap();
            z.write_all(data).unwrap();
        }
        z.finish().unwrap();
    }

    #[tokio::test]
    async fn local_archive_is_extracted_registered_and_listed_in_the_profile() {
        let (_g, root) = crate::settings::temp_xdg_guard();
        let conn = db::open_db().unwrap();
        let profile = db::GameProfileRecord { id: "test-profile".into(), game_id: "skyrim_se".into(), name: "Test".into(), game_version: "1".into(), modloader: None, deployment_path: None, created_at: 1 };
        db::insert_profile(&conn, &profile).unwrap();
        drop(conn);
        let zip = root.join("MonSuperMod.zip");
        make_zip(&zip, &[("MonMod.esp", b"TES4"), ("textures/a.dds", b"x")]);

        let name = install_local("skyrim_se", "test-profile", zip.clone()).await.expect("installation");
        assert_eq!(name, "MonSuperMod");

        let conn = db::open_db().unwrap();
        let mods = db::get_profile_mods(&conn, "test-profile").unwrap();
        assert_eq!(mods.len(), 1);
        assert_eq!((mods[0].provider.as_str(), mods[0].is_enabled), ("local", true));
        let staged = PathBuf::from(mods[0].staged_path.clone().unwrap());
        assert!(staged.join("MonMod.esp").exists() || staged.join("Data/MonMod.esp").exists(), "plugin extrait dans le staging : {:?}", std::fs::read_dir(&staged).map(|d| d.flatten().map(|e| e.file_name()).collect::<Vec<_>>()));
    }

    #[tokio::test]
    async fn installing_the_same_archive_twice_does_not_collide() {
        let (_g, root) = crate::settings::temp_xdg_guard();
        let conn = db::open_db().unwrap();
        db::insert_profile(&conn, &db::GameProfileRecord { id: "p".into(), game_id: "skyrim_se".into(), name: "P".into(), game_version: "1".into(), modloader: None, deployment_path: None, created_at: 1 }).unwrap();
        drop(conn);
        let zip = root.join("m.zip");
        make_zip(&zip, &[("a.esp", b"1")]);
        install_local("skyrim_se", "p", zip.clone()).await.unwrap();
        install_local("skyrim_se", "p", zip).await.unwrap();
        let mods = db::get_profile_mods(&db::open_db().unwrap(), "p").unwrap();
        assert_eq!(mods.len(), 2, "chaque installation crée sa ligne (ids uniques)");
        assert_ne!(mods[0].id, mods[1].id);
    }

    #[tokio::test]
    async fn missing_file_or_profile_is_reported() {
        let (_g, _root) = crate::settings::temp_xdg_guard();
        assert!(install_local("skyrim_se", "x", PathBuf::from("/inexistant.zip")).await.unwrap_err().contains("introuvable"));
    }
}
