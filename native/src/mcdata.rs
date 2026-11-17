//! Données d'instance Minecraft : serveurs (servers.dat), ping, cartes (saves/), modpacks.
//! Le backend Tauri/Vue n'implémentait pas ces routes (le front retombait sur des valeurs vides) :
//! tout est ici lu/écrit pour de vrai (NBT, protocole Server List Ping, archives zip).

use crate::minecraft::instance::get_instance_minecraft_dir;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

// ---------------------------------------------------------------- serveurs (servers.dat)

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Server {
    pub name: String,
    pub ip: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hidden: Option<i8>,
    #[serde(default, rename = "acceptTextures", skip_serializing_if = "Option::is_none")]
    pub accept_textures: Option<i8>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ServersFile { servers: Vec<Server> }

pub fn servers_path(instance_id: &str) -> PathBuf { get_instance_minecraft_dir(instance_id).join("servers.dat") }

pub fn read_servers_file(path: &Path) -> Result<Vec<Server>, String> {
    if !path.exists() { return Ok(Vec::new()); }
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    if bytes.is_empty() { return Ok(Vec::new()); }
    fastnbt::from_bytes::<ServersFile>(&bytes).map(|f| f.servers).map_err(|e| format!("servers.dat illisible : {e}"))
}

pub fn write_servers_file(path: &Path, servers: &[Server]) -> Result<(), String> {
    if let Some(p) = path.parent() { let _ = std::fs::create_dir_all(p); }
    let bytes = fastnbt::to_bytes(&ServersFile { servers: servers.to_vec() }).map_err(|e| e.to_string())?;
    std::fs::write(path, bytes).map_err(|e| e.to_string())
}

pub fn get_servers(instance_id: &str) -> Result<Vec<Server>, String> { read_servers_file(&servers_path(instance_id)) }
pub fn save_servers(instance_id: &str, servers: &[Server]) -> Result<(), String> { write_servers_file(&servers_path(instance_id), servers) }

/// Sépare « hôte:port » (port par défaut 25565).
pub fn split_address(ip: &str) -> (String, u16) {
    match ip.rsplit_once(':') {
        Some((h, p)) if !h.contains(':') => (h.to_string(), p.parse().unwrap_or(25565)),
        _ => (ip.to_string(), 25565),
    }
}

// ---------------------------------------------------------------- ping (Server List Ping)

#[derive(Debug, Clone, Default, PartialEq)]
pub struct PingResult {
    pub online: bool,
    pub ping_ms: u32,
    pub motd: String,
    pub players_online: u32,
    pub players_max: u32,
    pub version: String,
}

fn write_varint(buf: &mut Vec<u8>, mut v: u32) {
    loop {
        let mut b = (v & 0x7f) as u8;
        v >>= 7;
        if v != 0 { b |= 0x80; }
        buf.push(b);
        if v == 0 { break; }
    }
}

fn read_varint(s: &mut impl Read) -> std::io::Result<u32> {
    let (mut n, mut shift) = (0u32, 0);
    loop {
        let mut b = [0u8; 1];
        s.read_exact(&mut b)?;
        n |= ((b[0] & 0x7f) as u32) << shift;
        if b[0] & 0x80 == 0 { return Ok(n); }
        shift += 7;
        if shift > 35 { return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "varint trop long")); }
    }
}

fn motd_text(d: &serde_json::Value) -> String {
    match d {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Object(_) => {
            let mut out = d["text"].as_str().unwrap_or("").to_string();
            for e in d["extra"].as_array().cloned().unwrap_or_default() { out.push_str(&motd_text(&e)); }
            out
        }
        _ => String::new(),
    }
}

/// Retire les codes de formatage Minecraft (§a, §l…).
pub fn clean_motd(s: &str) -> String {
    let mut out = String::new();
    let mut skip = false;
    for c in s.chars() { if skip { skip = false; continue; } if c == '§' { skip = true; continue; } out.push(c); }
    out.trim().to_string()
}

pub fn ping_server(host: &str, port: u16, timeout: Duration) -> PingResult {
    let started = Instant::now();
    let Some(addr) = (host, port).to_socket_addrs().ok().and_then(|mut a| a.next()) else { return PingResult::default() };
    let Ok(mut s) = TcpStream::connect_timeout(&addr, timeout) else { return PingResult::default() };
    let _ = s.set_read_timeout(Some(timeout));
    let _ = s.set_write_timeout(Some(timeout));

    let mut hs = Vec::new();
    write_varint(&mut hs, 0); // id handshake
    write_varint(&mut hs, 765); // protocole (1.20.4) : le serveur répond quel que soit le protocole
    write_varint(&mut hs, host.len() as u32);
    hs.extend_from_slice(host.as_bytes());
    hs.extend_from_slice(&port.to_be_bytes());
    write_varint(&mut hs, 1); // état « status »
    let mut pkt = Vec::new();
    write_varint(&mut pkt, hs.len() as u32);
    pkt.extend_from_slice(&hs);
    pkt.extend_from_slice(&[1, 0]); // status request
    if s.write_all(&pkt).is_err() { return PingResult::default(); }

    let mut parse = || -> std::io::Result<serde_json::Value> {
        let _len = read_varint(&mut s)?;
        let _id = read_varint(&mut s)?;
        let jlen = read_varint(&mut s)? as usize;
        if jlen > 4 * 1024 * 1024 { return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "réponse trop grande")); }
        let mut json = vec![0u8; jlen];
        s.read_exact(&mut json)?;
        serde_json::from_slice(&json).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    };
    let Ok(v) = parse() else { return PingResult::default() };
    PingResult {
        online: true,
        ping_ms: started.elapsed().as_millis().min(u32::MAX as u128) as u32,
        motd: clean_motd(&motd_text(&v["description"])),
        players_online: v["players"]["online"].as_u64().unwrap_or(0) as u32,
        players_max: v["players"]["max"].as_u64().unwrap_or(0) as u32,
        version: v["version"]["name"].as_str().unwrap_or("").to_string(),
    }
}

// ---------------------------------------------------------------- cartes (saves/)

#[derive(Debug, Clone, PartialEq)]
pub struct SaveInfo {
    pub folder: String,
    pub name: String,
    pub last_played_ms: i64,
    pub game_mode: String,
    pub version: String,
    pub size_bytes: u64,
}

#[derive(Deserialize)]
struct LevelFile { #[serde(rename = "Data")] data: LevelData }
#[derive(Deserialize)]
struct LevelData {
    #[serde(rename = "LevelName", default)] name: String,
    #[serde(rename = "LastPlayed", default)] last_played: i64,
    #[serde(rename = "GameType", default)] game_type: i32,
    #[serde(rename = "Version", default)] version: Option<LevelVersion>,
}
#[derive(Deserialize)]
struct LevelVersion { #[serde(rename = "Name", default)] name: String }

pub fn saves_dir(instance_id: &str) -> PathBuf { get_instance_minecraft_dir(instance_id).join("saves") }

fn dir_size(p: &Path) -> u64 { walkdir::WalkDir::new(p).into_iter().flatten().filter_map(|e| e.metadata().ok()).filter(|m| m.is_file()).map(|m| m.len()).sum() }

impl SaveInfo { pub fn mode_label(&self) -> String { self.game_mode.clone() } }

pub fn game_mode_name(t: i32) -> &'static str { match t { 0 => "Survie", 1 => "Créatif", 2 => "Aventure", 3 => "Spectateur", _ => "Inconnu" } }

pub fn read_level_dat(path: &Path) -> Option<(String, i64, i32, String)> {
    let mut gz = flate2::read::GzDecoder::new(std::fs::File::open(path).ok()?);
    let mut bytes = Vec::new();
    gz.read_to_end(&mut bytes).ok()?;
    let l: LevelFile = fastnbt::from_bytes(&bytes).ok()?;
    Some((l.data.name, l.data.last_played, l.data.game_type, l.data.version.map(|v| v.name).unwrap_or_default()))
}

pub fn list_saves_in(dir: &Path) -> Vec<SaveInfo> {
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir(dir) else { return out };
    for e in rd.flatten() {
        let p = e.path();
        if !p.is_dir() { continue; }
        let folder = e.file_name().to_string_lossy().to_string();
        let (name, last, gt, ver) = read_level_dat(&p.join("level.dat")).unwrap_or((folder.clone(), 0, -1, String::new()));
        out.push(SaveInfo { folder: folder.clone(), name: if name.is_empty() { folder } else { name }, last_played_ms: last, game_mode: game_mode_name(gt).to_string(), version: ver, size_bytes: dir_size(&p) });
    }
    out.sort_by(|a, b| b.last_played_ms.cmp(&a.last_played_ms));
    out
}

pub fn list_saves(instance_id: &str) -> Vec<SaveInfo> { list_saves_in(&saves_dir(instance_id)) }

/// Nom de dossier sûr (pas de séparateur, pas de « .. »).
fn safe_folder(name: &str) -> Result<&str, String> {
    if name.is_empty() || name.contains('/') || name.contains('\\') || name == ".." || name == "." { Err("Nom de carte invalide".into()) } else { Ok(name) }
}

pub fn backup_save_in(saves: &Path, backups: &Path, folder: &str) -> Result<PathBuf, String> {
    let src = saves.join(safe_folder(folder)?);
    if !src.is_dir() { return Err("Carte introuvable".into()); }
    std::fs::create_dir_all(backups).map_err(|e| e.to_string())?;
    let dest = backups.join(format!("{}-{}.zip", folder, chrono::Local::now().format("%Y%m%d-%H%M%S")));
    let mut z = zip::ZipWriter::new(std::fs::File::create(&dest).map_err(|e| e.to_string())?);
    let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for e in walkdir::WalkDir::new(&src).into_iter().flatten() {
        let rel = e.path().strip_prefix(&src).unwrap().to_string_lossy().replace('\\', "/");
        if rel.is_empty() { continue; }
        let name = format!("{folder}/{rel}");
        if e.path().is_dir() { z.add_directory(name, opts).map_err(|e| e.to_string())?; }
        else { z.start_file(name, opts).map_err(|e| e.to_string())?; z.write_all(&std::fs::read(e.path()).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?; }
    }
    z.finish().map_err(|e| e.to_string())?;
    Ok(dest)
}

pub fn backup_save(instance_id: &str, folder: &str) -> Result<PathBuf, String> {
    backup_save_in(&saves_dir(instance_id), &get_instance_minecraft_dir(instance_id).join("backups"), folder)
}

pub fn delete_save_in(saves: &Path, folder: &str) -> Result<(), String> {
    let p = saves.join(safe_folder(folder)?);
    if !p.is_dir() { return Err("Carte introuvable".into()); }
    std::fs::remove_dir_all(p).map_err(|e| e.to_string())
}

pub fn delete_save(instance_id: &str, folder: &str) -> Result<(), String> { delete_save_in(&saves_dir(instance_id), folder) }

// ---------------------------------------------------------------- modpacks

#[derive(Debug, Clone, PartialEq)]
pub struct Modpack {
    pub id: String,
    pub title: String,
    pub author: String,
    pub downloads: u64,
    pub summary: String,
    pub loader: String,
    pub mc_version: String,
    pub source: String, // "modrinth" | "curseforge"
    pub icon_url: String,
}

fn cf_loader(n: u64) -> &'static str { match n { 1 => "forge", 5 => "quilt", 6 => "neoforge", _ => "fabric" } }

pub fn parse_modrinth_hit(h: &serde_json::Value) -> Modpack {
    let categories: Vec<String> = h["categories"].as_array().map(|a| a.iter().filter_map(|c| c.as_str().map(|s| s.to_lowercase())).collect()).unwrap_or_default();
    let loader = categories.iter().find(|c| ["fabric", "neoforge", "forge", "quilt"].contains(&c.as_str())).cloned().unwrap_or_else(|| "fabric".into());
    Modpack {
        id: h["project_id"].as_str().unwrap_or("").into(), title: h["title"].as_str().unwrap_or("").into(), author: h["author"].as_str().unwrap_or("").into(),
        downloads: h["downloads"].as_u64().unwrap_or(0), summary: h["description"].as_str().unwrap_or("").into(), loader,
        mc_version: h["versions"].as_array().and_then(|a| a.iter().filter_map(|v| v.as_str()).last()).unwrap_or("1.20.1").into(),
        source: "modrinth".into(), icon_url: h["icon_url"].as_str().unwrap_or("").into(),
    }
}

pub fn parse_curseforge_item(i: &serde_json::Value) -> Modpack {
    let idx = i["latestFilesIndexes"].as_array().and_then(|a| a.first());
    Modpack {
        id: i["id"].to_string(), title: i["name"].as_str().unwrap_or("").into(),
        author: i["authors"][0]["name"].as_str().unwrap_or("Inconnu").into(), downloads: i["downloadCount"].as_u64().unwrap_or(0),
        summary: i["summary"].as_str().unwrap_or("").into(),
        loader: cf_loader(idx.and_then(|x| x["modLoader"].as_u64().or(x["modLoaderType"].as_u64())).unwrap_or(4)).into(),
        mc_version: idx.and_then(|x| x["gameVersion"].as_str()).unwrap_or("1.20.1").into(),
        source: "curseforge".into(), icon_url: i["logo"]["url"].as_str().unwrap_or("").into(),
    }
}

pub async fn search_modpacks(client: &reqwest::Client, query: &str, provider: &str, limit: usize) -> Vec<Modpack> {
    let mut out = Vec::new();
    let q = query.trim();
    if provider == "all" || provider == "modrinth" {
        let mut req = client.get("https://api.modrinth.com/v2/search").query(&[("limit", limit.to_string()), ("index", (if q.is_empty() { "downloads" } else { "relevance" }).to_string()), ("facets", "[[\"project_type:modpack\"]]".to_string())]);
        if !q.is_empty() { req = req.query(&[("query", q)]); }
        if let Ok(r) = req.send().await { if let Ok(v) = r.json::<serde_json::Value>().await { out.extend(v["hits"].as_array().cloned().unwrap_or_default().iter().map(parse_modrinth_hit)); } }
    }
    if provider == "all" || provider == "curseforge" {
        let key = std::env::var("CURSEFORGE_API_KEY").unwrap_or_else(|_| "$2a$10$bL4bIL5pUWqfcO7KQtnMReakwtfHbNKh6v1uTpKlzhwoueEJQnPnm".to_string());
        let mut req = client.get("https://api.curseforge.com/v1/mods/search").header("x-api-key", key).header("Accept", "application/json")
            .query(&[("gameId", "432"), ("classId", "4471"), ("pageSize", &limit.to_string()), ("sortField", "2"), ("sortOrder", "desc")]);
        if !q.is_empty() { req = req.query(&[("searchFilter", q)]); }
        if let Ok(r) = req.send().await { if let Ok(v) = r.json::<serde_json::Value>().await { out.extend(v["data"].as_array().cloned().unwrap_or_default().iter().map(parse_curseforge_item)); } }
    }
    out
}

/// Télécharge le dernier fichier du modpack puis l'installe (nouvelle instance). Renvoie l'id d'instance.
pub async fn install_modpack(client: &reqwest::Client, pack: &Modpack) -> Result<String, String> {
    use crate::modding::{CurseForgeProvider, ModProvider, ModrinthProvider};
    let cache = crate::modding::symlink::get_mods_cache_dir();
    let (versions, provider): (_, Box<dyn ModProvider>) = if pack.source == "modrinth" {
        let p = ModrinthProvider::new();
        (p.get_versions(&pack.id, None, None).await?, Box::new(p))
    } else {
        let p = CurseForgeProvider::new();
        (p.get_versions(&pack.id, None, None).await?, Box::new(p))
    };
    let version = versions.first().ok_or("Aucune version disponible pour ce modpack")?;
    let file = version.files.iter().find(|f| f.is_primary).or_else(|| version.files.first()).ok_or("Aucun fichier pour cette version")?;
    let path = provider.download_file(&file.url, &cache, &file.filename).await?;
    if file.filename.ends_with(".mrpack") {
        crate::minecraft::ModpackInstaller::install_mrpack(client, &path, Some(&pack.title)).await
    } else {
        crate::minecraft::ModpackInstaller::install_curseforge_pack(&path, Some(&pack.title)).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("loom-mc-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn servers_dat_roundtrips_through_nbt() {
        let f = tmp("servers").join("servers.dat");
        assert!(read_servers_file(&f).unwrap().is_empty(), "fichier absent = aucun serveur");
        let list = vec![
            Server { name: "Survie".into(), ip: "play.exemple.fr".into(), icon: None, hidden: None, accept_textures: Some(1) },
            Server { name: "Créatif".into(), ip: "10.0.0.2:25570".into(), icon: Some("aWNvbg==".into()), hidden: Some(0), accept_textures: None },
        ];
        write_servers_file(&f, &list).unwrap();
        assert_eq!(read_servers_file(&f).unwrap(), list);
    }

    #[test]
    fn corrupted_servers_dat_is_an_error_not_a_panic() {
        let f = tmp("servers-bad").join("servers.dat");
        std::fs::write(&f, b"pas du nbt").unwrap();
        assert!(read_servers_file(&f).is_err());
    }

    #[test]
    fn addresses_are_split_with_default_port() {
        assert_eq!(split_address("play.exemple.fr"), ("play.exemple.fr".into(), 25565));
        assert_eq!(split_address("play.exemple.fr:25570"), ("play.exemple.fr".into(), 25570));
        assert_eq!(split_address("hote:abc"), ("hote".into(), 25565));
    }

    #[test]
    fn varints_roundtrip() {
        for v in [0u32, 1, 127, 128, 300, 25565, 2_097_151] {
            let mut b = Vec::new();
            write_varint(&mut b, v);
            assert_eq!(read_varint(&mut &b[..]).unwrap(), v);
        }
    }

    #[test]
    fn motd_formatting_codes_are_removed() {
        assert_eq!(clean_motd("§a§lBienvenue §rsur le serveur"), "Bienvenue sur le serveur");
        let d = serde_json::json!({"text": "Hello ", "extra": [{"text": "World"}]});
        assert_eq!(motd_text(&d), "Hello World");
    }

    /// Faux serveur Minecraft : répond à un Server List Ping.
    fn fake_server() -> u16 {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        std::thread::spawn(move || {
            if let Ok((mut s, _)) = l.accept() {
                let mut buf = [0u8; 512];
                let _ = s.read(&mut buf);
                let json = serde_json::json!({"version": {"name": "1.20.1"}, "players": {"online": 3, "max": 20}, "description": {"text": "§6Serveur de test"}}).to_string();
                let mut body = Vec::new();
                write_varint(&mut body, 0);
                write_varint(&mut body, json.len() as u32);
                body.extend_from_slice(json.as_bytes());
                let mut pkt = Vec::new();
                write_varint(&mut pkt, body.len() as u32);
                pkt.extend_from_slice(&body);
                let _ = s.write_all(&pkt);
            }
        });
        port
    }

    #[test]
    fn ping_reads_the_status_of_a_live_server() {
        let port = fake_server();
        let r = ping_server("127.0.0.1", port, Duration::from_secs(3));
        assert!(r.online);
        assert_eq!((r.players_online, r.players_max), (3, 20));
        assert_eq!((r.motd.as_str(), r.version.as_str()), ("Serveur de test", "1.20.1"));
    }

    #[test]
    fn ping_of_a_closed_port_is_offline() {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        drop(l);
        assert!(!ping_server("127.0.0.1", port, Duration::from_millis(500)).online);
    }

    fn write_level_dat(dir: &Path, name: &str, last: i64, game_type: i32) {
        #[derive(Serialize)] struct V { #[serde(rename = "Name")] n: String }
        #[derive(Serialize)] struct D { #[serde(rename = "LevelName")] name: String, #[serde(rename = "LastPlayed")] last: i64, #[serde(rename = "GameType")] gt: i32, #[serde(rename = "Version")] v: V }
        #[derive(Serialize)] struct F { #[serde(rename = "Data")] d: D }
        let bytes = fastnbt::to_bytes(&F { d: D { name: name.into(), last, gt: game_type, v: V { n: "1.20.1".into() } } }).unwrap();
        let mut gz = flate2::write::GzEncoder::new(std::fs::File::create(dir.join("level.dat")).unwrap(), flate2::Compression::default());
        gz.write_all(&bytes).unwrap();
        gz.finish().unwrap();
    }

    #[test]
    fn saves_are_listed_newest_first_with_metadata() {
        let saves = tmp("saves");
        for (folder, name, last, gt) in [("a", "Vieille carte", 1_000, 0), ("b", "Récente", 9_000, 1)] {
            let d = saves.join(folder);
            std::fs::create_dir_all(&d).unwrap();
            write_level_dat(&d, name, last, gt);
        }
        std::fs::create_dir_all(saves.join("sans_level")).unwrap();
        let list = list_saves_in(&saves);
        assert_eq!(list.len(), 3);
        assert_eq!((list[0].name.as_str(), list[0].game_mode.as_str(), list[0].version.as_str()), ("Récente", "Créatif", "1.20.1"));
        assert_eq!(list[1].name, "Vieille carte");
        assert_eq!(list[2].name, "sans_level", "dossier sans level.dat : nom du dossier");
        assert!(list[0].size_bytes > 0);
    }

    #[test]
    fn backup_creates_a_zip_and_delete_removes_the_save() {
        let root = tmp("backup");
        let (saves, backups) = (root.join("saves"), root.join("backups"));
        std::fs::create_dir_all(saves.join("monde/region")).unwrap();
        write_level_dat(&saves.join("monde"), "Monde", 1, 0);
        std::fs::write(saves.join("monde/region/r.0.0.mca"), b"donnees").unwrap();

        let zip = backup_save_in(&saves, &backups, "monde").unwrap();
        let mut za = zip::ZipArchive::new(std::fs::File::open(&zip).unwrap()).unwrap();
        assert!(za.by_name("monde/level.dat").is_ok() && za.by_name("monde/region/r.0.0.mca").is_ok());

        delete_save_in(&saves, "monde").unwrap();
        assert!(!saves.join("monde").exists());
    }

    #[test]
    fn path_traversal_in_save_names_is_rejected() {
        let root = tmp("traversal");
        assert!(delete_save_in(&root, "../etc").is_err());
        assert!(backup_save_in(&root, &root, "a/b").is_err());
        assert!(delete_save_in(&root, "..").is_err());
    }

    #[test]
    fn modrinth_and_curseforge_results_are_normalised() {
        let m = parse_modrinth_hit(&serde_json::json!({"project_id": "abc", "title": "Pack", "author": "me", "downloads": 10, "description": "d", "categories": ["Neoforge", "adventure"], "versions": ["1.20.1", "1.21"], "icon_url": "u"}));
        assert_eq!((m.loader.as_str(), m.mc_version.as_str(), m.source.as_str()), ("neoforge", "1.21", "modrinth"));
        let c = parse_curseforge_item(&serde_json::json!({"id": 42, "name": "CF", "authors": [{"name": "au"}], "downloadCount": 5, "latestFilesIndexes": [{"gameVersion": "1.19.2", "modLoader": 1}], "logo": {"url": "l"}}));
        assert_eq!((c.id.as_str(), c.loader.as_str(), c.mc_version.as_str(), c.author.as_str()), ("42", "forge", "1.19.2", "au"));
        assert_eq!(parse_curseforge_item(&serde_json::json!({})).loader, "fabric", "défaut");
    }
}
