//! Steam : scan de la bibliothèque locale, détection du jeu en cours, lancement,
//! arrêt, installation et désinstallation.
//! Port fidèle de `scan_local_steam_library` / `get_running_steam_app` / `launch_game`
//! (src-tauri/src/lib.rs, privés et couplés à Tauri) et des routes `/api/steam/*`.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct SteamGame {
    pub appid: String,
    pub name: String,
    pub installed: bool,
    pub playtime_minutes: u32,
    pub last_played: i64,
    pub size_gb: f32,
    pub install_path: Option<PathBuf>,
}

const IGNORED: &[&str] = &[
    "228980", "1070560", "1391110", "1628350", "2180100", "1493710", "1826330", "3658110",
    "1161040", "4183110", "2805730", "4628710",
];

fn home() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

pub fn steam_base() -> PathBuf {
    let h = home();
    for p in [".local/share/Steam", ".steam/steam", ".var/app/com.valvesoftware.Steam/.local/share/Steam"] {
        let c = h.join(p);
        if c.exists() {
            return c;
        }
    }
    h.join(".local/share/Steam")
}

/// Noms connus (fichier déjà utilisé par le front Vue pour les jeux non installés).
fn known_names() -> HashMap<String, String> {
    #[derive(serde::Deserialize)]
    struct E {
        appid: String,
        name: String,
    }
    let raw = include_str!("../../src/services/steam_detected_games.json");
    serde_json::from_str::<Vec<E>>(raw)
        .map(|v| v.into_iter().map(|e| (e.appid, e.name)).collect())
        .unwrap_or_default()
}

fn vdf_value(line: &str) -> Option<String> {
    let parts: Vec<&str> = line.trim().split('"').collect();
    if parts.len() >= 4 { Some(parts[3].to_string()) } else { None }
}

pub fn scan_library() -> Vec<SteamGame> { scan_library_at(&steam_base()) }

/// Scan d'une installation Steam donnée (testable avec un faux dossier).
pub fn scan_library_at(base: &Path) -> Vec<SteamGame> {
    let base = base.to_path_buf();
    if !base.exists() {
        return Vec::new();
    }

    let mut libraries: Vec<PathBuf> = vec![base.clone()];
    if let Ok(txt) = fs::read_to_string(base.join("config/libraryfolders.vdf")) {
        for line in txt.lines() {
            if line.trim().starts_with("\"path\"") {
                if let Some(v) = vdf_value(line) {
                    let p = PathBuf::from(v);
                    if p.exists() && !libraries.contains(&p) {
                        libraries.push(p);
                    }
                }
            }
        }
    }

    // 1. Temps de jeu / dernier lancement réels (localconfig.vdf, suivi de profondeur)
    let mut playtime: HashMap<String, (u64, u32)> = HashMap::new();
    if let Ok(users) = fs::read_dir(base.join("userdata")) {
        for u in users.flatten() {
            let Ok(txt) = fs::read_to_string(u.path().join("config/localconfig.vdf")) else { continue };
            let (mut pending, mut active): (Option<String>, Option<String>) = (None, None);
            let (mut active_depth, mut depth) = (0usize, 0usize);
            for line in txt.lines() {
                let t = line.trim();
                if t.is_empty() { continue; }
                if t == "{" && pending.is_some() {
                    active = pending.take();
                    active_depth = depth + 1;
                }
                let parts: Vec<&str> = t.split('"').collect();
                if parts.len() >= 2 && !parts[1].is_empty() && parts[1].chars().all(|c| c.is_ascii_digit())
                    && t == format!("\"{}\"", parts[1])
                {
                    pending = Some(parts[1].to_string());
                }
                if let Some(aid) = &active {
                    if t.starts_with("\"LastPlayed\"") {
                        if let Some(v) = vdf_value(t).and_then(|v| v.parse::<u64>().ok()) {
                            let e = playtime.entry(aid.clone()).or_insert((0, 0));
                            if v > e.0 { e.0 = v; }
                        }
                    } else if t.starts_with("\"Playtime\"") {
                        if let Some(v) = vdf_value(t).and_then(|v| v.parse::<u32>().ok()) {
                            let e = playtime.entry(aid.clone()).or_insert((0, 0));
                            if v > e.1 { e.1 = v; }
                        }
                    }
                }
                depth = depth.saturating_add(t.matches('{').count()).saturating_sub(t.matches('}').count());
                if active.is_some() && depth < active_depth { active = None; }
            }
        }
    }

    // 2. Jeux installés (appmanifest_*.acf de chaque bibliothèque)
    let mut games = Vec::new();
    let mut installed_ids = HashSet::new();
    for lib in &libraries {
        let steamapps = lib.join("steamapps");
        let Ok(entries) = fs::read_dir(&steamapps) else { continue };
        for e in entries.flatten() {
            let f = e.file_name().to_string_lossy().to_string();
            if !(f.starts_with("appmanifest_") && f.ends_with(".acf")) { continue; }
            let Ok(content) = fs::read_to_string(e.path()) else { continue };
            let (mut aid, mut name, mut dir, mut flags, mut size) =
                (String::new(), String::new(), String::new(), String::new(), 0u64);
            for line in content.lines() {
                let t = line.trim();
                if t.starts_with("\"appid\"") { aid = vdf_value(t).unwrap_or_default(); }
                else if t.starts_with("\"name\"") { name = vdf_value(t).unwrap_or_default(); }
                else if t.starts_with("\"installdir\"") { dir = vdf_value(t).unwrap_or_default(); }
                else if t.starts_with("\"StateFlags\"") { flags = vdf_value(t).unwrap_or_default(); }
                else if t.starts_with("\"SizeOnDisk\"") { size = vdf_value(t).and_then(|v| v.parse().ok()).unwrap_or(0); }
            }
            if aid.is_empty() || IGNORED.contains(&aid.as_str()) { continue; }
            if name.starts_with("Steam Linux Runtime") || name.starts_with("Proton") || name.contains("Redistributable") { continue; }
            let installed = flags == "4";
            if !installed { continue; }
            let (lp, pt) = playtime.get(&aid).cloned().unwrap_or((0, 0));
            installed_ids.insert(aid.clone());
            games.push(SteamGame {
                appid: aid,
                name,
                installed: true,
                playtime_minutes: pt,
                last_played: lp as i64,
                size_gb: (size as f32 / 1024.0 / 1024.0 / 1024.0 * 10.0).round() / 10.0,
                install_path: if dir.is_empty() { None } else { Some(steamapps.join("common").join(dir)) },
            });
        }
    }

    // 3. Jeux possédés/joués mais non installés
    let names = known_names();
    for (aid, (lp, pt)) in &playtime {
        if installed_ids.contains(aid) || IGNORED.contains(&aid.as_str()) { continue; }
        games.push(SteamGame {
            appid: aid.clone(),
            name: names.get(aid).cloned().unwrap_or_else(|| format!("Steam App {}", aid)),
            installed: false,
            playtime_minutes: *pt,
            last_played: *lp as i64,
            size_gb: 0.0,
            install_path: None,
        });
    }
    games
}

pub fn cover_url(appid: &str) -> String {
    format!("https://shared.steamstatic.com/store_item_assets/steam/apps/{appid}/library_600x900.jpg")
}
pub fn banner_url(appid: &str) -> String {
    format!("https://shared.steamstatic.com/store_item_assets/steam/apps/{appid}/library_hero.jpg")
}

/// Jeu Steam en cours d'exécution : (appid, pids).
pub fn running_app() -> (Option<String>, Vec<u32>) {
    if let Ok(entries) = fs::read_dir("/proc") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let Ok(pid) = name.parse::<u32>() else { continue };
            if let Ok(cmd) = fs::read(entry.path().join("cmdline")) {
                let s = String::from_utf8_lossy(&cmd);
                // Seul le « reaper » de Steam (`SteamLaunch AppId=NNN -- <jeu>`) désigne un jeu réellement lancé.
                // `rungameid/NNN` apparaît aussi dans xdg-open / steam (la demande de lancement) : à ignorer,
                // sinon Loom affiche « en jeu » avant même que le jeu existe.
                if let Some(pos) = s.find("SteamLaunch") {
                    if let Some(p2) = s[pos..].find("AppId=") {
                        let start = pos + p2 + 6;
                        let aid: String = s[start..].chars().take_while(|c| c.is_ascii_digit()).collect();
                        if !aid.is_empty() && aid != "0" { return (Some(aid), vec![pid]); }
                    }
                }
            }
            if let Ok(env) = fs::read(entry.path().join("environ")) {
                if let Some(pos) = env.windows(11).position(|w| w == b"SteamAppId=") {
                    let sub = &env[pos + 11..];
                    let end = sub.iter().position(|&b| b == 0).unwrap_or(sub.len());
                    if let Ok(v) = std::str::from_utf8(&sub[..end]) {
                        let v = v.trim();
                        if !v.is_empty() && v != "0" && v.chars().all(|c| c.is_ascii_digit()) {
                            return (Some(v.to_string()), vec![pid]);
                        }
                    }
                }
            }
        }
    }

    let log = base_log();
    let Ok(content) = fs::read_to_string(&log) else { return (None, Vec::new()) };
    let lines: Vec<&str> = content.lines().collect();
    let recent = &lines[lines.len().saturating_sub(5000)..];
    let mut app_pids: HashMap<String, HashSet<u32>> = HashMap::new();
    for line in recent {
        if line.contains("Remove ") && line.contains(" from running list") {
            if let Some(pos) = line.find("Remove ") {
                if let Some(end) = line[pos + 7..].find(' ') {
                    app_pids.remove(&line[pos + 7..pos + 7 + end]);
                }
            }
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if line.contains("adding PID") {
            for i in 0..parts.len() {
                if parts[i] == "AppID" && i + 4 < parts.len() && parts[i + 3] == "PID" {
                    if let Ok(pid) = parts[i + 4].parse::<u32>() {
                        app_pids.entry(parts[i + 1].to_string()).or_default().insert(pid);
                    }
                }
            }
        } else if line.contains("no longer tracking PID") {
            for i in 0..parts.len() {
                if parts[i] == "AppID" && i + 6 < parts.len() && parts[i + 5] == "PID" {
                    if let Ok(pid) = parts[i + 6].trim_matches(',').parse::<u32>() {
                        if let Some(set) = app_pids.get_mut(parts[i + 1]) {
                            set.remove(&pid);
                            if set.is_empty() { app_pids.remove(parts[i + 1]); }
                        }
                    }
                }
            }
        }
    }
    for (aid, pids) in app_pids {
        let alive: Vec<u32> = pids.into_iter().filter(|p| PathBuf::from(format!("/proc/{p}")).exists()).collect();
        if !alive.is_empty() { return (Some(aid), alive); }
    }
    (None, Vec::new())
}

fn base_log() -> PathBuf {
    steam_base().join("logs/gameprocess_log.txt")
}

fn steam_uri(action: &str, appid: &str) -> Result<(), String> {
    use std::process::Stdio;
    let uri = format!("steam://{action}/{appid}");
    let detached = |mut c: Command| c.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().is_ok();
    // 1. Le client Steam lui-même : transmet l'URL à l'instance déjà ouverte (ou démarre Steam).
    //    On évite le `xdg-open` du PATH : selon l'environnement (wrapper Claude Desktop, etc.) il ne
    //    transmet pas les URL steam:// tout en renvoyant un code de succès.
    for bin in ["/usr/bin/steam", "/usr/games/steam", "steam"] {
        let mut c = Command::new(bin);
        c.arg(&uri);
        if detached(c) { return Ok(()); }
    }
    // 2. Steam Flatpak
    let mut f = Command::new("flatpak");
    f.args(["run", "com.valvesoftware.Steam", &uri]);
    if detached(f) { return Ok(()); }
    // 3. Gestionnaire d'URL système (chemin absolu)
    let mut x = Command::new("/usr/bin/xdg-open");
    x.arg(&uri);
    if detached(x) { return Ok(()); }
    Err("Impossible de joindre Steam (aucun binaire `steam` trouvé)".into())
}

pub fn launch(appid: &str) -> Result<(), String> { steam_uri("rungameid", appid) }
pub fn install(appid: &str) -> Result<(), String> { steam_uri("install", appid) }
pub fn uninstall(appid: &str) -> Result<(), String> { steam_uri("uninstall", appid) }

pub fn stop() {
    let (_, pids) = running_app();
    for pid in pids {
        let _ = Command::new("kill").arg("-15").arg(pid.to_string()).status();
    }
}

// ---------------- Succès (cache local Steam : userdata/*/config/librarycache) ----------------

#[derive(Debug, Clone)]
pub struct Achievement {
    pub name: String,
    pub description: String,
    pub unlocked: bool,
    pub icon: String,
    pub rarity: f64,
}

fn librarycache_dirs() -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Ok(users) = fs::read_dir(steam_base().join("userdata")) {
        for u in users.flatten() {
            let d = u.path().join("config/librarycache");
            if d.exists() { v.push(d); }
        }
    }
    v
}

/// appid -> (débloqués, total)
pub fn achievement_summaries() -> HashMap<String, (u32, u32)> {
    let mut out = HashMap::new();
    for dir in librarycache_dirs() {
        if let Ok(txt) = fs::read_to_string(dir.join("achievement_progress.json")) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&txt) {
                if let Some(arr) = v["mapCache"].as_array() {
                    for e in arr {
                        if let (Some(id), Some(info)) = (e.get(0), e.get(1)) {
                            let aid = id.as_u64().map(|x| x.to_string()).or(id.as_str().map(String::from));
                            let (u, t) = (info["unlocked"].as_u64().unwrap_or(0) as u32, info["total"].as_u64().unwrap_or(0) as u32);
                            if let Some(a) = aid { if t > 0 { out.insert(a, (u, t)); } }
                        }
                    }
                }
            }
        }
        if let Ok(rd) = fs::read_dir(&dir) {
            for f in rd.flatten() {
                let name = f.file_name().to_string_lossy().to_string();
                let Some(aid) = name.strip_suffix(".json") else { continue };
                if aid.is_empty() || !aid.chars().all(|c| c.is_ascii_digit()) || out.contains_key(aid) { continue; }
                if let Some((u, t)) = achievements_from_file(&f.path()).map(|v| (v.iter().filter(|a| a.unlocked).count() as u32, v.len() as u32)) {
                    if t > 0 { out.insert(aid.to_string(), (u, t)); }
                }
            }
        }
    }
    out
}

fn achievements_from_file(path: &std::path::Path) -> Option<Vec<Achievement>> {
    let v: serde_json::Value = serde_json::from_str(&fs::read_to_string(path).ok()?).ok()?;
    for item in v.as_array()? {
        let arr = item.as_array()?;
        if arr.first().and_then(|x| x.as_str()) == Some("achievements") {
            let data = &arr.get(1)?["data"];
            let mut seen = HashSet::new();
            let mut out = Vec::new();
            for key in ["vecHighlight", "vecAchieved", "vecUnachieved"] {
                for a in data[key].as_array().cloned().unwrap_or_default() {
                    let id = a["strID"].as_str().unwrap_or("").to_string();
                    if id.is_empty() || !seen.insert(id.clone()) { continue; }
                    out.push(Achievement {
                        name: a["strName"].as_str().unwrap_or(&id).to_string(),
                        description: a["strDescription"].as_str().unwrap_or("").to_string(),
                        unlocked: a["bAchieved"].as_bool().unwrap_or(false),
                        icon: a["strImage"].as_str().unwrap_or("").to_string(),
                        rarity: a["flAchieved"].as_f64().unwrap_or(0.0),
                    });
                }
            }
            return Some(out);
        }
    }
    None
}

pub fn achievements(appid: &str) -> Vec<Achievement> {
    for dir in librarycache_dirs() {
        if let Some(v) = achievements_from_file(&dir.join(format!("{appid}.json"))) { if !v.is_empty() { return v; } }
    }
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("loom-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    fn fake_steam() -> PathBuf {
        let base = tmp("steam");
        fs::create_dir_all(base.join("steamapps")).unwrap();
        fs::create_dir_all(base.join("config")).unwrap();
        fs::create_dir_all(base.join("userdata/123/config")).unwrap();
        fs::write(base.join("steamapps/appmanifest_489830.acf"),
            "\"AppState\"\n{\n\t\"appid\"\t\t\"489830\"\n\t\"name\"\t\t\"Skyrim SE\"\n\t\"installdir\"\t\t\"Skyrim Special Edition\"\n\t\"StateFlags\"\t\t\"4\"\n\t\"SizeOnDisk\"\t\t\"2147483648\"\n}\n").unwrap();
        // Runtime Proton et jeu non terminé : doivent être ignorés
        fs::write(base.join("steamapps/appmanifest_1493710.acf"), "\"AppState\"\n{\n\"appid\" \"1493710\"\n\"name\" \"Proton Experimental\"\n\"StateFlags\" \"4\"\n}\n").unwrap();
        fs::write(base.join("steamapps/appmanifest_999.acf"), "\"AppState\"\n{\n\"appid\" \"999\"\n\"name\" \"Pas fini\"\n\"StateFlags\" \"1026\"\n}\n").unwrap();
        fs::write(base.join("userdata/123/config/localconfig.vdf"),
            "\"UserLocalConfigStore\"\n{\n\"Software\"\n{\n\"apps\"\n{\n\"489830\"\n{\n\"LastPlayed\"\t\"1700000000\"\n\"Playtime\"\t\"600\"\n}\n\"4000\"\n{\n\"LastPlayed\"\t\"1600000000\"\n\"Playtime\"\t\"30\"\n}\n}\n}\n}\n").unwrap();
        base
    }

    #[test]
    fn scan_reads_installed_games_and_playtime() {
        let games = scan_library_at(&fake_steam());
        let sky = games.iter().find(|g| g.appid == "489830").expect("Skyrim détecté");
        assert!(sky.installed);
        assert_eq!(sky.name, "Skyrim SE");
        assert_eq!(sky.playtime_minutes, 600);
        assert_eq!(sky.last_played, 1_700_000_000);
        assert_eq!(sky.size_gb, 2.0);
        assert!(sky.install_path.as_ref().unwrap().ends_with("steamapps/common/Skyrim Special Edition"));
    }

    #[test]
    fn scan_ignores_runtimes_and_unfinished_installs() {
        let games = scan_library_at(&fake_steam());
        assert!(!games.iter().any(|g| g.appid == "1493710"), "runtime Proton ignoré");
        assert!(!games.iter().any(|g| g.appid == "999" && g.installed), "installation incomplète non installée");
    }

    #[test]
    fn scan_lists_played_but_uninstalled_games() {
        let games = scan_library_at(&fake_steam());
        let g = games.iter().find(|g| g.appid == "4000").expect("jeu joué non installé");
        assert!(!g.installed);
        assert_eq!(g.playtime_minutes, 30);
    }

    #[test]
    fn scan_of_missing_steam_is_empty() {
        assert!(scan_library_at(Path::new("/chemin/inexistant/steam")).is_empty());
    }

    #[test]
    fn achievements_are_parsed_from_librarycache() {
        let d = tmp("ach");
        let f = d.join("489830.json");
        fs::write(&f, r#"[["achievements",{"data":{
            "vecHighlight":[{"strID":"A","strName":"Premier","strDescription":"d","bAchieved":true,"flAchieved":12.5,"strImage":"abc"}],
            "vecAchieved":[{"strID":"A","strName":"doublon","bAchieved":true}],
            "vecUnachieved":[{"strID":"B","strName":"Second","bAchieved":false}]}}]]"#).unwrap();
        let ach = achievements_from_file(&f).unwrap();
        assert_eq!(ach.len(), 2, "les doublons sont ignorés");
        assert_eq!(ach.iter().filter(|a| a.unlocked).count(), 1);
        assert_eq!(ach[0].name, "Premier");
        assert_eq!(ach[0].rarity, 12.5);
    }

    #[test]
    fn urls_follow_steam_cdn_layout() {
        assert!(cover_url("10").ends_with("/apps/10/library_600x900.jpg"));
        assert!(banner_url("10").ends_with("/apps/10/library_hero.jpg"));
    }
}
