//! Modèle de bibliothèque unifié (Steam + Battle.net), favoris persistés et actions système.

use crate::{battlenet, db, steam};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Steam,
    BattleNet,
}

impl Game {
    pub fn platform_label(&self) -> &'static str { match self.platform { Platform::Steam => "Steam", Platform::BattleNet => "Battle.net" } } }

#[derive(Debug, Clone)]
pub struct Game {
    pub id: String, // "steam-<appid>" | "bnet-<uid>"
    pub title: String,
    pub subtitle: String,
    pub platform: Platform,
    pub installed: bool,
    pub playtime_min: u32,
    pub last_played: i64,
    pub size_gb: f32,
    pub genre: String,
    pub cover_url: Option<String>,
    pub banner_url: Option<String>,
    pub install_path: Option<PathBuf>,
    pub description: Option<String>,
    pub ach: Option<(u32, u32)>,
    pub igdb_slug: Option<String>,
    pub igdb_id: Option<i64>,
}

#[derive(serde::Deserialize)]
struct Art {
    cover: Option<String>,
    banner: Option<String>,
    genre: Option<Vec<String>>,
    desc: Option<String>,
    igdb_id: Option<i64>,
    igdb_slug: Option<String>,
}

fn blizzard_art() -> HashMap<String, Art> {
    serde_json::from_str(include_str!("blizzard_art.json")).unwrap_or_default()
}

/// Charge Steam + Battle.net (même sources que le front Vue : scan local, base SQLite
/// partagée avec l'application Tauri, jeux possédés via l'API Blizzard si connecté).
pub async fn load_all() -> Vec<Game> {
    let steam_task = tokio::task::spawn_blocking(steam::scan_library);
    let summaries = tokio::task::spawn_blocking(steam::achievement_summaries).await.unwrap_or_default();
    let mut games: Vec<Game> = Vec::new();

    if let Ok(list) = steam_task.await {
        for g in list {
            games.push(Game {
                id: format!("steam-{}", g.appid),
                title: g.name,
                subtitle: String::new(),
                platform: Platform::Steam,
                installed: g.installed,
                playtime_min: g.playtime_minutes,
                last_played: g.last_played,
                size_gb: g.size_gb,
                genre: "Steam".into(),
                cover_url: Some(steam::cover_url(&g.appid)),
                banner_url: Some(steam::banner_url(&g.appid)),
                install_path: g.install_path,
                description: None,
                ach: summaries.get(&g.appid).cloned(),
                igdb_slug: None,
                igdb_id: None,
            });
        }
    }

    games.extend(load_battlenet().await);
    // Steam : images et infos viennent d'IGDB (cache disque) quand une fiche est associée à l'AppID
    for g in games.iter_mut().filter(|g| g.platform == Platform::Steam) {
        if let Some(ig) = g.id.strip_prefix("steam-").and_then(crate::igdb::cached_steam) {
            if let Some(c) = ig.cover_url() { g.cover_url = Some(c); }
            if let Some(b) = ig.banner_url() { g.banner_url = Some(b); }
            if let Some(first) = ig.genres.first() { g.genre = first.clone(); }
        }
    }
    // Jaquette / bannière issues d'IGDB (cache disque) quand elles sont connues
    for g in games.iter_mut().filter(|g| g.platform == Platform::BattleNet) {
        if let Some(ig) = g.igdb_slug.as_deref().and_then(crate::igdb::cached).or(g.igdb_id.and_then(|i| crate::igdb::cached(&i.to_string()))) {
            if let Some(c) = ig.cover_url() { g.cover_url = Some(c); }
            if let Some(b) = ig.banner_url() { g.banner_url = Some(b); }
            if !ig.genres.is_empty() { g.genre = ig.genres[0].clone(); }
        }
    }

    games.sort_by(|a, b| b.last_played.cmp(&a.last_played).then(a.title.cmp(&b.title)));
    games
}

async fn load_battlenet() -> Vec<Game> {
    let art = blizzard_art();
    let mut out: Vec<Game> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    let local = tokio::task::spawn_blocking(|| -> Result<Vec<db::BattleNetGameRecord>, String> {
        let conn = db::open_db().map_err(|e| e.to_string())?;
        let cfg = db::get_battlenet_config(&conn).map_err(|e| e.to_string())?;
        battlenet::detector::scan_prefix_and_external_games(&PathBuf::from(&cfg.wine_prefix_path), &conn)
    })
    .await;

    let make = |uid: &str, name: String, version: Option<String>, cover: Option<String>, banner: Option<String>| {
        let a = art.get(&uid.to_lowercase());
        let subtitle = version.map(|v| format!("Build {v}")).unwrap_or_else(|| "Plateforme Battle.net".into());
        (
            a.and_then(|a| a.cover.clone()).or(cover),
            a.and_then(|a| a.banner.clone()).or(banner),
            a.and_then(|a| a.genre.as_ref()).and_then(|g| g.first().cloned()).unwrap_or_else(|| "Battle.net".into()),
            subtitle,
            name,
        )
    };

    if let Ok(Ok(list)) = local {
        for g in list {
            let uid = g.product_uid.to_lowercase();
            seen.insert(uid.clone());
            let (cover, banner, genre, subtitle, name) = make(&uid, g.name.clone(), g.version_build.clone(), None, None);
            out.push(Game {
                id: format!("bnet-{uid}"),
                title: name,
                subtitle,
                platform: Platform::BattleNet,
                installed: g.is_installed,
                playtime_min: (g.playtime_seconds / 60) as u32,
                last_played: g.last_played.unwrap_or(0),
                size_gb: 0.0,
                genre,
                cover_url: cover,
                banner_url: banner,
                install_path: Some(PathBuf::from(g.install_path)),
                description: art.get(&uid).and_then(|a| a.desc.clone()),
                ach: None,
                igdb_slug: art.get(&uid).and_then(|a| a.igdb_slug.clone()),
                igdb_id: art.get(&uid).and_then(|a| a.igdb_id),
            });
        }
    }

    // Jeux possédés dans le compte Blizzard (uniquement si un compte est connecté)
    if let Ok(Some(_)) = battlenet::auth::ensure_valid_blizzard_account().await {
        if let Ok(owned) = battlenet::auth::get_blizzard_owned_games().await {
            for og in owned {
                let uid = og.product_uid.to_lowercase();
                if !seen.insert(uid.clone()) { continue; }
                let (cover, banner, genre, subtitle, name) =
                    make(&uid, og.name.clone(), og.version_build.clone(), og.cover_url.clone(), og.banner_url.clone());
                out.push(Game {
                    id: format!("bnet-{uid}"),
                    title: name,
                    subtitle,
                    platform: Platform::BattleNet,
                    installed: og.is_installed,
                    playtime_min: 0,
                    last_played: 0,
                    size_gb: 0.0,
                    genre,
                    cover_url: cover,
                    banner_url: banner,
                    install_path: og.install_path.map(PathBuf::from),
                    description: art.get(&uid).and_then(|a| a.desc.clone()).or(og.summary.clone()),
                    ach: None,
                    igdb_slug: art.get(&uid).and_then(|a| a.igdb_slug.clone()),
                    igdb_id: art.get(&uid).and_then(|a| a.igdb_id),
                });
            }
        }
    }
    out
}

// ---------------- Favoris ----------------

fn favorites_path() -> PathBuf {
    let dir = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from(".")).join("loom");
    let _ = std::fs::create_dir_all(&dir);
    dir.join("native_favorites.json")
}

pub fn load_favorites() -> HashSet<String> {
    std::fs::read_to_string(favorites_path())
        .ok()
        .and_then(|s| serde_json::from_str::<Vec<String>>(&s).ok())
        .map(|v| v.into_iter().collect())
        .unwrap_or_default()
}

pub fn save_favorites(f: &HashSet<String>) {
    let v: Vec<&String> = f.iter().collect();
    if let Ok(s) = serde_json::to_string(&v) {
        let _ = std::fs::write(favorites_path(), s);
    }
}

// ---------------- Actions ----------------

pub struct Actions {
    pub bnet: battlenet::launcher::BattleNetLauncherState,
}

impl Actions {
    pub fn new() -> Self {
        Self { bnet: battlenet::launcher::BattleNetLauncherState::new() }
    }

    pub async fn launch(&self, g: &Game) -> Result<String, String> {
        match g.platform {
            Platform::Steam => {
                steam::launch(g.id.trim_start_matches("steam-"))?;
                Ok(format!("Lancement de {} transmis à Steam", g.title))
            }
            Platform::BattleNet => {
                battlenet::launcher::launch_battlenet_game(&self.bnet, g.id.trim_start_matches("bnet-")).await?;
                Ok(format!("Lancement de {} via Battle.net", g.title))
            }
        }
    }

    pub async fn stop(&self, g: &Game) -> Result<String, String> {
        match g.platform {
            Platform::Steam => {
                tokio::task::spawn_blocking(steam::stop).await.map_err(|e| e.to_string())?;
            }
            Platform::BattleNet => {
                battlenet::launcher::kill_battlenet_game(&self.bnet, g.id.trim_start_matches("bnet-")).await?;
            }
        }
        Ok(format!("{} arrêté", g.title))
    }

    pub async fn install(&self, g: &Game) -> Result<String, String> {
        match g.platform {
            Platform::Steam => {
                steam::install(g.id.trim_start_matches("steam-"))?;
                Ok(format!("Assistant d'installation Steam ouvert pour {}", g.title))
            }
            // Comme le front Vue : l'installation passe par le client Battle.net
            Platform::BattleNet => {
                battlenet::launcher::launch_battlenet_game(&self.bnet, g.id.trim_start_matches("bnet-")).await?;
                Ok(format!("Ouverture de Battle.net pour {}", g.title))
            }
        }
    }

    pub async fn uninstall(&self, g: &Game) -> Result<String, String> {
        match g.platform {
            Platform::Steam => {
                steam::uninstall(g.id.trim_start_matches("steam-"))?;
                Ok(format!("Désinstallation de {} demandée à Steam", g.title))
            }
            Platform::BattleNet => {
                battlenet::launcher::kill_battlenet_game(&self.bnet, g.id.trim_start_matches("bnet-")).await?;
                Ok("Désinstallation Battle.net : à effectuer depuis le client Battle.net".into())
            }
        }
    }
}

pub fn open_folder(g: &Game) -> Result<String, String> {
    let p = g.install_path.clone().filter(|p| p.exists()).ok_or("Dossier d'installation introuvable")?;
    open::that_detached(&p).map_err(|e| e.to_string())?;
    Ok("Dossier ouvert".into())
}

/// Jeux en cours : (steam appid complet "steam-…", bnet uids "bnet-…")
pub async fn running_ids() -> Vec<String> {
    let mut ids = Vec::new();
    if let Ok((Some(aid), _)) = tokio::task::spawn_blocking(steam::running_app).await {
        ids.push(format!("steam-{aid}"));
    }
    for uid in battlenet::launcher::scan_running_battlenet_processes().into_keys() {
        ids.push(format!("bnet-{}", uid.to_lowercase()));
    }
    ids
}

pub fn persist_bnet_playtime(uid: &str, delta: u64, now: i64) {
    if let Ok(conn) = db::open_db() {
        let _ = db::update_battlenet_game_playtime(&conn, uid, delta, Some(now));
    }
}
