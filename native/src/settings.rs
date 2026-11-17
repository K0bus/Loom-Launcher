//! Réglages : partage `~/.local/share/loom/settings.json` avec l'app Tauri (champs backend,
//! autres champs conservés tels quels) et stocke les réglages propres au front dans
//! `native_settings.json` (le front Vue les garde dans le localStorage, inaccessible d'ici).

use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

pub const RUNNERS: [&str; 4] = ["proton-ge", "proton-experimental", "native", "wine-staging"];
pub const ACCENTS: [&str; 5] = ["amber", "emerald", "violet", "sky", "rose"];
pub const AFTER_LAUNCH: [&str; 3] = ["minimize", "hide", "keep"];

#[derive(Debug, Clone)]
pub struct AppSettings {
    pub autostart: bool,
    pub start_minimized: bool,
    pub close_to_tray: bool,
    pub after_launch: usize,
    pub inst: [bool; 4], // steam, battlenet, epic, gog
    pub inst_silent: bool,
    pub accent: usize,
    pub notifications: bool,
    pub auto_sync: bool,
    pub runner: usize,
    pub gamemode: bool,
    pub mangohud: bool,
    pub dir_games: String,
    pub dir_minecraft: String,
    pub dir_wine: String,
    pub check_updates: bool,
    pub igdb_id: String,
    pub igdb_token: String,
    pub igdb_secret: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            autostart: false,
            start_minimized: false,
            close_to_tray: true,
            after_launch: 0,
            inst: [true, true, false, false],
            inst_silent: true,
            accent: 0,
            notifications: true,
            auto_sync: true,
            runner: 0,
            gamemode: true,
            mangohud: true,
            dir_games: crate::steam::steam_base().to_string_lossy().to_string(),
            dir_minecraft: "~/.local/share/loom/instances".into(),
            dir_wine: "~/.local/share/loom/prefixes".into(),
            check_updates: true,
            igdb_id: String::new(),
            igdb_token: String::new(),
            igdb_secret: String::new(),
        }
    }
}

fn loom_dir() -> PathBuf {
    let d = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from(".")).join("loom");
    let _ = fs::create_dir_all(&d);
    d
}
fn backend_path() -> PathBuf { loom_dir().join("settings.json") }
fn native_path() -> PathBuf { loom_dir().join("native_settings.json") }
fn igdb_path() -> PathBuf { loom_dir().join("native_igdb.json") }
fn autostart_file() -> Option<PathBuf> {
    dirs::config_dir().map(|c| c.join("autostart").join("com.loom.launcher.desktop"))
}

fn read_json(p: &PathBuf) -> Value {
    fs::read_to_string(p).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or(Value::Null)
}
fn idx(list: &[&str], v: Option<&str>, d: usize) -> usize {
    v.and_then(|v| list.iter().position(|x| *x == v)).unwrap_or(d)
}

pub fn load() -> AppSettings {
    let mut s = AppSettings::default();
    let b = read_json(&backend_path());
    let n = read_json(&native_path());
    let gb = |v: &Value, k: &str| v.get(k).and_then(|x| x.as_bool());
    let gs = |v: &Value, k: &str| v.get(k).and_then(|x| x.as_str()).map(String::from);

    if let Some(v) = gb(&b, "closeToTray") { s.close_to_tray = v; }
    if let Some(v) = gb(&b, "startMinimized") { s.start_minimized = v; }
    if let Some(v) = gs(&b, "gamesDirectory") { s.dir_games = v; }
    if let Some(v) = gs(&b, "minecraftDirectory") { s.dir_minecraft = v; }
    if let Some(v) = gs(&b, "winePrefixesDirectory") { s.dir_wine = v; }
    s.runner = idx(&RUNNERS, b.get("defaultRunner").and_then(|x| x.as_str()), 0);
    if let Some(v) = gb(&b, "globalGamemode") { s.gamemode = v; }
    if let Some(v) = gb(&b, "globalMangohud") { s.mangohud = v; }

    s.after_launch = idx(&AFTER_LAUNCH, n.get("afterGameLaunch").and_then(|x| x.as_str()), 0);
    if let Some(i) = n.get("autostartInstances") {
        for (k, key) in ["steam", "battlenet", "epic", "gog"].iter().enumerate() {
            if let Some(v) = gb(i, key) { s.inst[k] = v; }
        }
    }
    if let Some(v) = gb(&n, "startInstancesMinimized") { s.inst_silent = v; }
    s.accent = idx(&ACCENTS, n.get("themeAccent").and_then(|x| x.as_str()), 0);
    if let Some(v) = gb(&n, "desktopNotifications") { s.notifications = v; }
    if let Some(v) = gb(&n, "autoSyncCloudSaves") { s.auto_sync = v; }
    if let Some(v) = gb(&n, "checkUpdatesAuto") { s.check_updates = v; }

    // Autostart réel = présence du .desktop
    s.autostart = autostart_file().map(|p| p.exists()).unwrap_or(false);

    let ig = read_json(&igdb_path());
    s.igdb_id = gs(&ig, "clientId").unwrap_or_default();
    s.igdb_token = gs(&ig, "accessToken").unwrap_or_default();
    s.igdb_secret = gs(&ig, "clientSecret").unwrap_or_default();
    s
}

pub fn save(s: &AppSettings) -> Result<(), String> {
    // settings.json partagé : on fusionne pour ne pas écraser les champs inconnus (ex. nexusApiKey)
    let mut b = match read_json(&backend_path()) { Value::Object(m) => m, _ => Default::default() };
    for (k, v) in [
        ("closeToTray", json!(s.close_to_tray)),
        ("startMinimized", json!(s.start_minimized)),
        ("autostartWithSystem", json!(s.autostart)),
        ("gamesDirectory", json!(s.dir_games)),
        ("minecraftDirectory", json!(s.dir_minecraft)),
        ("winePrefixesDirectory", json!(s.dir_wine)),
        ("defaultRunner", json!(RUNNERS[s.runner.min(3)])),
        ("globalGamemode", json!(s.gamemode)),
        ("globalMangohud", json!(s.mangohud)),
    ] {
        b.insert(k.into(), v);
    }
    fs::write(backend_path(), serde_json::to_string_pretty(&Value::Object(b)).unwrap()).map_err(|e| e.to_string())?;

    let n = json!({
        "afterGameLaunch": AFTER_LAUNCH[s.after_launch.min(2)],
        "autostartInstances": { "steam": s.inst[0], "battlenet": s.inst[1], "epic": s.inst[2], "gog": s.inst[3] },
        "startInstancesMinimized": s.inst_silent,
        "desktopNotifications": s.notifications,
        "themeAccent": ACCENTS[s.accent.min(4)],
        "autoSyncCloudSaves": s.auto_sync,
        "checkUpdatesAuto": s.check_updates,
    });
    fs::write(native_path(), serde_json::to_string_pretty(&n).unwrap()).map_err(|e| e.to_string())?;

    let ig = json!({ "clientId": s.igdb_id, "accessToken": s.igdb_token, "clientSecret": s.igdb_secret });
    fs::write(igdb_path(), serde_json::to_string_pretty(&ig).unwrap()).map_err(|e| e.to_string())?;

    set_autostart(s.autostart, s.start_minimized)
}

/// Écrit/supprime l'entrée `com.loom.launcher.desktop` (même fichier que l'app Tauri).
pub fn set_autostart(enabled: bool, minimized: bool) -> Result<(), String> {
    let Some(file) = autostart_file() else { return Ok(()) };
    if !enabled {
        if file.exists() { let _ = fs::remove_file(file); }
        return Ok(());
    }
    let _ = fs::create_dir_all(file.parent().unwrap());
    let exe = std::env::var("APPIMAGE").map(PathBuf::from).or_else(|_| std::env::current_exe()).map_err(|e| e.to_string())?;
    let exec = if minimized { format!("\"{}\" --minimized", exe.display()) } else { format!("\"{}\"", exe.display()) };
    let content = format!(
        "[Desktop Entry]\nType=Application\nName=Loom Launcher\nGenericName=Universal Gaming Launcher\nComment=Universal Gaming and Modding Platform\nExec={exec}\nIcon=loom-launcher\nTerminal=false\nCategories=Game;\nStartupNotify=false\nX-GNOME-Autostart-enabled=true\n"
    );
    fs::write(file, content).map_err(|e| e.to_string())
}

pub fn expand(p: &str) -> PathBuf {
    if let Some(rest) = p.strip_prefix("~/") {
        dirs::home_dir().unwrap_or_default().join(rest)
    } else { PathBuf::from(p) }
}

pub fn pick_directory(title: &str, default: &str) -> Option<String> {
    let def = expand(default);
    let out = Command::new("zenity")
        .args(["--file-selection", "--directory", &format!("--title={title}"), &format!("--filename={}/", def.display())])
        .output().ok().filter(|o| o.status.success())
        .or_else(|| Command::new("kdialog").args(["--getexistingdirectory", &def.to_string_lossy()]).output().ok().filter(|o| o.status.success()))?;
    let p = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if p.is_empty() { None } else { Some(p) }
}

/// Sélecteur de fichier (zenity puis kdialog) — archives de mods.
pub fn pick_file(title: &str, patterns: &str) -> Option<String> {
    let out = Command::new("zenity")
        .args(["--file-selection", &format!("--title={title}"), &format!("--file-filter=Archives | {patterns}")])
        .output().ok().filter(|o| o.status.success())
        .or_else(|| Command::new("kdialog").args(["--getopenfilename", &dirs::home_dir().unwrap_or_default().to_string_lossy(), patterns]).output().ok().filter(|o| o.status.success()))?;
    let p = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if p.is_empty() { None } else { Some(p) }
}

pub fn open_directory(path: &str) -> Result<(), String> {
    let p = expand(path);
    let _ = fs::create_dir_all(&p);
    Command::new("xdg-open").arg(&p).spawn().map(|_| ()).map_err(|e| e.to_string())
}

pub fn notify(title: &str, body: &str) -> Result<(), String> {
    Command::new("notify-send")
        .args(["-a", "Loom Launcher", "-h", "string:x-canonical-private-synchronous:loom", title, body])
        .stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null())
        .spawn().map(|_| ()).map_err(|e| format!("notify-send indisponible : {e}"))
}

pub async fn twitch_token(client: &reqwest::Client, id: &str, secret: &str) -> Result<String, String> {
    let resp = client
        .post("https://id.twitch.tv/oauth2/token")
        .query(&[("client_id", id), ("client_secret", secret), ("grant_type", "client_credentials")])
        .send().await.map_err(|e| e.to_string())?;
    let status = resp.status();
    let v: Value = resp.json().await.map_err(|e| e.to_string())?;
    match v.get("access_token").and_then(|t| t.as_str()) {
        Some(t) => Ok(t.to_string()),
        None => Err(format!("{} : {}", status, v.get("message").and_then(|m| m.as_str()).unwrap_or("réponse invalide"))),
    }
}

/// Port de `autostart_platform_instance` (src-tauri/src/lib.rs)
pub fn autostart_platform(platform: &str, minimized: bool) -> bool {
    let has = |bin: &str| Command::new("which").arg(bin).output().map(|o| o.status.success()).unwrap_or(false);
    let running = |args: &[&str]| Command::new("pgrep").args(args).output().map(|o| o.status.success()).unwrap_or(false);
    match platform {
        "steam" => {
            if !has("steam") { return false; }
            if !running(&["-x", "steam"]) {
                let mut c = Command::new("steam");
                if minimized { c.arg("-silent"); }
                let _ = c.spawn();
            }
            true
        }
        "battlenet" => { let _ = fs::create_dir_all(loom_dir().join("prefixes/battlenet")); true }
        "epic" => {
            if !has("heroic") { return false; }
            if !running(&["-f", "heroic"]) {
                let mut c = Command::new("heroic");
                if minimized { c.arg("--minimized"); }
                let _ = c.spawn();
            }
            true
        }
        "gog" => true,
        _ => false,
    }
}

pub struct SysInfo { pub version: String, pub display: String, pub kernel: String, pub ram: String }

pub fn sys_info() -> SysInfo {
    let kernel = fs::read_to_string("/proc/sys/kernel/osrelease").map(|s| s.trim().to_string()).unwrap_or_else(|_| "Linux x86_64".into());
    let mem = fs::read_to_string("/proc/meminfo").unwrap_or_default();
    let kb = |key: &str| mem.lines().find(|l| l.starts_with(key)).and_then(|l| l.split_whitespace().nth(1)).and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
    let total = kb("MemTotal:") / 1024.0 / 1024.0;
    let used = total - kb("MemAvailable:") / 1024.0 / 1024.0;
    let display = match std::env::var("XDG_SESSION_TYPE").unwrap_or_default().as_str() {
        "wayland" => "Wayland (Session native)".to_string(),
        "x11" => "X11".to_string(),
        o if !o.is_empty() => o.to_string(),
        _ => "Inconnu".to_string(),
    };
    SysInfo {
        version: format!("v{} (Slint natif)", env!("CARGO_PKG_VERSION")),
        display,
        kernel,
        ram: format!("{:.1} GB / {:.1} GB", used, total),
    }
}

/// Verrou partagé par les tests qui redirigent XDG_DATA_HOME / XDG_CONFIG_HOME.
#[cfg(test)]
pub(crate) static TEST_ENV: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Variante pour les tests async : renvoie le verrou et la racine temporaire.
#[cfg(test)]
pub(crate) fn temp_xdg_guard() -> (std::sync::MutexGuard<'static, ()>, std::path::PathBuf) {
    let g = TEST_ENV.lock().unwrap_or_else(|e| e.into_inner());
    let root = std::env::temp_dir().join(format!("loom-xdg-{}-{}", std::process::id(), uuid::Uuid::new_v4().simple()));
    std::fs::create_dir_all(&root).unwrap();
    std::env::set_var("XDG_DATA_HOME", root.join("data"));
    std::env::set_var("XDG_CONFIG_HOME", root.join("config"));
    (g, root)
}

#[cfg(test)]
pub(crate) fn with_temp_xdg<R>(f: impl FnOnce(&std::path::Path) -> R) -> R {
    let _g = TEST_ENV.lock().unwrap_or_else(|e| e.into_inner());
    let root = std::env::temp_dir().join(format!("loom-xdg-{}-{}", std::process::id(), uuid::Uuid::new_v4().simple()));
    std::fs::create_dir_all(&root).unwrap();
    std::env::set_var("XDG_DATA_HOME", root.join("data"));
    std::env::set_var("XDG_CONFIG_HOME", root.join("config"));
    let r = f(&root);
    let _ = std::fs::remove_dir_all(&root);
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_the_vue_app() {
        let d = AppSettings::default();
        assert!(d.close_to_tray && d.gamemode && d.mangohud && d.notifications && d.check_updates);
        assert!(!d.autostart && !d.start_minimized);
        assert_eq!(d.inst, [true, true, false, false]);
        assert_eq!((d.runner, d.accent, d.after_launch), (0, 0, 0));
    }

    #[test]
    fn save_then_load_roundtrips_every_field() {
        with_temp_xdg(|_| {
            let mut s = AppSettings::default();
            s.start_minimized = true; s.close_to_tray = false; s.after_launch = 2; s.accent = 3; s.runner = 2;
            s.inst = [false, true, true, true]; s.dir_games = "/mnt/jeux".into(); s.igdb_id = "cid".into(); s.igdb_secret = "sec".into();
            save(&s).unwrap();
            let l = load();
            assert!(l.start_minimized && !l.close_to_tray);
            assert_eq!((l.after_launch, l.accent, l.runner), (2, 3, 2));
            assert_eq!(l.inst, [false, true, true, true]);
            assert_eq!(l.dir_games, "/mnt/jeux");
            assert_eq!((l.igdb_id.as_str(), l.igdb_secret.as_str()), ("cid", "sec"));
        });
    }

    #[test]
    fn shared_settings_json_keeps_fields_written_by_the_tauri_app() {
        with_temp_xdg(|_| {
            let path = backend_path();
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, r#"{"nexusApiKey":"CLE","champInconnu":42,"startMinimized":false}"#).unwrap();
            let mut s = load();
            s.start_minimized = true;
            save(&s).unwrap();
            let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
            assert_eq!(v["nexusApiKey"], "CLE", "clé Nexus conservée");
            assert_eq!(v["champInconnu"], 42, "champs inconnus conservés");
            assert_eq!(v["startMinimized"], true);
            assert_eq!(v["defaultRunner"], "proton-ge");
        });
    }

    #[test]
    fn autostart_is_the_presence_of_the_desktop_file() {
        with_temp_xdg(|_| {
            assert!(!load().autostart);
            set_autostart(true, true).unwrap();
            let f = autostart_file().unwrap();
            let txt = std::fs::read_to_string(&f).unwrap();
            assert!(txt.contains("--minimized") && txt.contains("Type=Application"));
            assert!(load().autostart);
            set_autostart(false, false).unwrap();
            assert!(!f.exists() && !load().autostart);
        });
    }

    #[test]
    fn tilde_is_expanded() {
        assert!(expand("~/x").ends_with("x") && !expand("~/x").to_string_lossy().starts_with('~'));
        assert_eq!(expand("/abs"), PathBuf::from("/abs"));
    }

    #[test]
    fn unknown_choices_fall_back_to_defaults() {
        assert_eq!(idx(&RUNNERS, Some("inconnu"), 1), 1);
        assert_eq!(idx(&RUNNERS, Some("native"), 0), 2);
        assert_eq!(idx(&ACCENTS, None, 4), 4);
    }

    #[test]
    fn system_info_is_filled() {
        let i = sys_info();
        assert!(i.version.contains("Slint") && !i.kernel.is_empty() && i.ram.contains("GB"));
    }
}
