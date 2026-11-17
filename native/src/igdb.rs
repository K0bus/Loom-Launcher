//! Client IGDB v4 (identifiants Twitch saisis dans Paramètres → Métadonnées IGDB).
//! Résultats mis en cache sur disque pour ne pas solliciter l'API à chaque ouverture.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IgdbGame {
    pub name: String,
    pub slug: String,
    pub summary: String,
    pub genres: Vec<String>,
    pub developer: String,
    pub publisher: String,
    pub release_year: Option<i32>,
    pub rating: Option<f64>,
    pub cover_id: Option<String>,
    pub artwork_ids: Vec<String>,
    pub screenshot_ids: Vec<String>,
}

impl IgdbGame {
    pub fn cover_url(&self) -> Option<String> { self.cover_id.as_ref().map(|i| format!("https://images.igdb.com/igdb/image/upload/t_cover_big_2x/{i}.jpg")) }
    pub fn banner_url(&self) -> Option<String> {
        self.artwork_ids.first().or(self.screenshot_ids.first()).map(|i| format!("https://images.igdb.com/igdb/image/upload/t_1080p/{i}.jpg"))
    }
    pub fn shots(&self) -> Vec<(String, String)> {
        self.screenshot_ids.iter().take(8).map(|i| (
            format!("https://images.igdb.com/igdb/image/upload/t_screenshot_big/{i}.jpg"),
            format!("https://images.igdb.com/igdb/image/upload/t_1080p/{i}.jpg"),
        )).collect()
    }
}

static MEM: Mutex<Option<HashMap<String, IgdbGame>>> = Mutex::new(None);

fn cache_path() -> PathBuf {
    let d = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from(".")).join("loom");
    let _ = std::fs::create_dir_all(&d);
    d.join("native_igdb_cache.json")
}

fn with_cache<R>(f: impl FnOnce(&mut HashMap<String, IgdbGame>) -> R) -> R {
    let mut g = MEM.lock().unwrap();
    let m = g.get_or_insert_with(|| std::fs::read_to_string(cache_path()).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default());
    f(m)
}

pub fn cached(key: &str) -> Option<IgdbGame> { with_cache(|m| m.get(key).cloned()) }

fn store(key: &str, g: &IgdbGame) {
    with_cache(|m| { m.insert(key.to_string(), g.clone()); let _ = std::fs::write(cache_path(), serde_json::to_string(m).unwrap_or_default()); });
}

pub fn is_configured() -> bool {
    let s = crate::settings::load();
    !s.igdb_id.is_empty() && (!s.igdb_token.is_empty() || !s.igdb_secret.is_empty())
}

async fn credentials(client: &reqwest::Client) -> Result<(String, String), String> {
    let s = crate::settings::load();
    if s.igdb_id.is_empty() { return Err("Identifiants IGDB non configurés (Paramètres → Métadonnées IGDB)".into()); }
    let token = if s.igdb_token.is_empty() {
        if s.igdb_secret.is_empty() { return Err("Token IGDB manquant".into()); }
        crate::settings::twitch_token(client, &s.igdb_id, &s.igdb_secret).await?
    } else { s.igdb_token };
    Ok((s.igdb_id, token))
}

const FIELDS: &str = "name,slug,summary,total_rating,first_release_date,genres.name,cover.image_id,artworks.image_id,screenshots.image_id,involved_companies.developer,involved_companies.publisher,involved_companies.company.name";

fn parse_game(g: &serde_json::Value) -> IgdbGame {
    let ids = |k: &str| g[k].as_array().map(|a| a.iter().filter_map(|x| x["image_id"].as_str().map(String::from)).collect::<Vec<_>>()).unwrap_or_default();
    let company = |role: &str| g["involved_companies"].as_array().and_then(|a| a.iter().find(|c| c[role].as_bool() == Some(true))).and_then(|c| c["company"]["name"].as_str()).unwrap_or("").to_string();
    IgdbGame {
        name: g["name"].as_str().unwrap_or("").to_string(),
        slug: g["slug"].as_str().unwrap_or("").to_string(),
        summary: g["summary"].as_str().unwrap_or("").to_string(),
        genres: g["genres"].as_array().map(|a| a.iter().filter_map(|x| x["name"].as_str().map(String::from)).collect()).unwrap_or_default(),
        developer: company("developer"),
        publisher: company("publisher"),
        release_year: g["first_release_date"].as_i64().and_then(|t| chrono::DateTime::from_timestamp(t, 0)).map(|d| chrono::Datelike::year(&d)),
        rating: g["total_rating"].as_f64(),
        cover_id: g["cover"]["image_id"].as_str().map(String::from),
        artwork_ids: ids("artworks"),
        screenshot_ids: ids("screenshots"),
    }
}

async fn query(client: &reqwest::Client, creds: &(String, String), endpoint: &str, body: String) -> Result<serde_json::Value, String> {
    let resp = client.post(format!("https://api.igdb.com/v4/{endpoint}")).header("Client-ID", &creds.0).bearer_auth(&creds.1).header("Accept", "application/json").body(body).send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() { return Err(format!("IGDB a répondu HTTP {}", resp.status())); }
    resp.json().await.map_err(|e| e.to_string())
}

pub fn cached_steam(appid: &str) -> Option<IgdbGame> { cached(&format!("steam:{appid}")).filter(|g| !g.name.is_empty()) }
pub fn steam_known(appid: &str) -> bool { cached(&format!("steam:{appid}")).is_some() }

/// Associe des AppID Steam à leur fiche IGDB (external_games, source Steam) — 2 requêtes par lot.
/// Les jeux sans correspondance sont mémorisés (fiche vide) pour ne pas être redemandés.
pub async fn fetch_steam(client: &reqwest::Client, appids: &[String]) -> Result<usize, String> {
    let todo: Vec<&String> = appids.iter().filter(|a| !steam_known(a)).collect();
    if todo.is_empty() { return Ok(0); }
    let creds = credentials(client).await?;
    let mut found = 0;
    for chunk in todo.chunks(250) {
        let list = chunk.iter().map(|a| format!("\"{a}\"")).collect::<Vec<_>>().join(",");
        // `external_game_source` (récent) puis repli sur `category` (déprécié)
        let ext = match query(client, &creds, "external_games", format!("fields uid,game; where uid = ({list}) & external_game_source = 1; limit 500;")).await {
            Ok(v) if v.as_array().map(|a| !a.is_empty()).unwrap_or(false) => v,
            _ => query(client, &creds, "external_games", format!("fields uid,game; where uid = ({list}) & category = 1; limit 500;")).await?,
        };
        let mut by_game: HashMap<i64, Vec<String>> = HashMap::new();
        for e in ext.as_array().cloned().unwrap_or_default() {
            if let (Some(g), Some(u)) = (e["game"].as_i64(), e["uid"].as_str()) { by_game.entry(g).or_default().push(u.to_string()); }
        }
        let ids = by_game.keys().map(|i| i.to_string()).collect::<Vec<_>>();
        let mut games: HashMap<i64, IgdbGame> = HashMap::new();
        for part in ids.chunks(200) {
            let v = query(client, &creds, "games", format!("fields {FIELDS}; where id = ({}); limit 500;", part.join(","))).await?;
            for g in v.as_array().cloned().unwrap_or_default() { if let Some(id) = g["id"].as_i64() { games.insert(id, parse_game(&g)); } }
        }
        with_cache(|m| {
            for a in chunk { m.insert(format!("steam:{a}"), IgdbGame::default()); } // « pas de correspondance » par défaut
            for (gid, uids) in &by_game { if let Some(g) = games.get(gid) { for u in uids { m.insert(format!("steam:{u}"), g.clone()); found += 1; } } }
            let _ = std::fs::write(cache_path(), serde_json::to_string(m).unwrap_or_default());
        });
    }
    Ok(found)
}

/// Interroge IGDB par slug (prioritaire) ou par identifiant.
pub async fn fetch(client: &reqwest::Client, slug: Option<&str>, id: Option<i64>) -> Result<IgdbGame, String> {
    let key = slug.map(String::from).or(id.map(|i| i.to_string())).ok_or("Aucune référence IGDB")?;
    if let Some(g) = cached(&key) { return Ok(g); }
    let creds = credentials(client).await?;
    let filter = match (slug, id) { (Some(s), _) => format!("slug = \"{s}\""), (None, Some(i)) => format!("id = {i}"), _ => unreachable!() };
    let v = query(client, &creds, "games", format!("fields {FIELDS}; where {filter}; limit 1;")).await?;
    let g = v.get(0).ok_or_else(|| format!("Aucun jeu IGDB pour « {key} »"))?;
    let out = parse_game(g);
    store(&key, &out);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn igdb_payload_is_parsed() {
        let v = serde_json::json!({
            "name": "World of Warcraft: Forever", "slug": "world-of-warcraft-forever", "summary": "Résumé",
            "total_rating": 74.6, "first_release_date": 1_700_000_000,
            "genres": [{"name": "MMORPG"}, {"name": "Adventure"}],
            "cover": {"image_id": "co1"}, "artworks": [{"image_id": "ar1"}], "screenshots": [{"image_id": "sc1"}, {"image_id": "sc2"}],
            "involved_companies": [{"developer": false, "company": {"name": "Éditeur"}}, {"developer": true, "company": {"name": "Blizzard"}}]
        });
        let g = parse_game(&v);
        assert_eq!(g.name, "World of Warcraft: Forever");
        assert_eq!(g.genres, vec!["MMORPG", "Adventure"]);
        assert_eq!(g.developer, "Blizzard");
        assert_eq!(g.release_year, Some(2023));
        assert_eq!(g.cover_url().unwrap(), "https://images.igdb.com/igdb/image/upload/t_cover_big_2x/co1.jpg");
        assert_eq!(g.banner_url().unwrap(), "https://images.igdb.com/igdb/image/upload/t_1080p/ar1.jpg");
        assert_eq!(g.shots().len(), 2);
    }

    #[test]
    fn banner_falls_back_to_a_screenshot() {
        let g = IgdbGame { screenshot_ids: vec!["s".into()], ..Default::default() };
        assert!(g.banner_url().unwrap().contains("/s.jpg"));
        assert!(IgdbGame::default().banner_url().is_none());
    }
}
