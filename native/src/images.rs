//! Cache disque des jaquettes / bannières (Slint ne charge pas d'URL).
use std::path::PathBuf;

fn cache_dir() -> PathBuf {
    let d = dirs::cache_dir().unwrap_or_else(|| PathBuf::from(".")).join("loom/native-images");
    let _ = std::fs::create_dir_all(&d);
    d
}

fn stem_for(url: &str) -> String {
    use sha1::{Digest, Sha1};
    let h = Sha1::digest(url.as_bytes());
    h.iter().map(|b| format!("{b:02x}")).collect()
}

/// Chemin local si déjà en cache.
pub fn cached(url: &str) -> Option<PathBuf> {
    let stem = stem_for(url);
    ["jpg", "png", "webp"]
        .iter()
        .map(|e| cache_dir().join(format!("{stem}.{e}")))
        .find(|p| p.metadata().map(|m| m.len() > 0).unwrap_or(false))
}

pub async fn fetch(client: &reqwest::Client, url: &str) -> Option<PathBuf> {
    if let Some(p) = cached(url) { return Some(p); }
    let resp = client.get(url).send().await.ok()?.error_for_status().ok()?;
    let bytes = resp.bytes().await.ok()?;
    // Valide que c'est une image décodable par Slint (évite d'écrire du HTML d'erreur)
    let ext = image_ext(&bytes)?;
    let p = cache_dir().join(format!("{}.{ext}", stem_for(url)));
    std::fs::write(&p, &bytes).ok()?;
    Some(p)
}

fn image_ext(b: &[u8]) -> Option<&'static str> {
    if b.starts_with(&[0xFF, 0xD8]) { Some("jpg") }
    else if b.starts_with(&[0x89, b'P', b'N', b'G']) { Some("png") }
    else if b.len() > 12 && &b[0..4] == b"RIFF" && &b[8..12] == b"WEBP" { Some("webp") }
    else { None }
}
