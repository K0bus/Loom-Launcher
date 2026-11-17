//! Page détail d'un jeu : métadonnées Steam (API publique), actualités, succès (cache local Steam).

use crate::library::{Game, Platform};
use crate::pages::with_app;
use crate::{ctx, images, steam, toast, AppWindow};
use crate::{AchRow, Detail, NewsRow, ShotRow};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

fn ss(s: impl Into<String>) -> SharedString { SharedString::from(s.into()) }
fn model<T: Clone + 'static>(v: Vec<T>) -> ModelRc<T> { ModelRc::new(VecModel::from(v)) }

#[derive(Clone, Default)]
struct NewsItem { tag: String, patch: bool, author: String, date: String, title: String, summary: String, url: String }

#[derive(Clone, Default)]
struct Extra {
    description: String,
    genres: Vec<String>,
    developer: String,
    release: String,
    rating: String,
    shots: Vec<(String, String)>, // (miniature, pleine)
    news: Vec<NewsItem>,
    ach: Vec<steam::Achievement>,
    banner_url: Option<String>,
    igdb_note: String,
    loaded: bool,
}

static EXTRA: OnceLock<Mutex<HashMap<String, Extra>>> = OnceLock::new();
static CURRENT: Mutex<Option<String>> = Mutex::new(None);
fn extra() -> std::sync::MutexGuard<'static, HashMap<String, Extra>> { EXTRA.get().unwrap().lock().unwrap() }

fn strip_markup(s: &str) -> String {
    let mut out = String::new();
    let mut depth = 0;
    for c in s.chars() {
        match c { '[' | '<' => depth += 1, ']' | '>' => { if depth > 0 { depth -= 1 } } _ if depth == 0 => out.push(c), _ => {} }
    }
    let out = out.replace("&quot;", "\"").replace("&amp;", "&").replace("&#39;", "'").replace("&nbsp;", " ");
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn ago_date(ts: i64) -> String {
    chrono::DateTime::from_timestamp(ts, 0).map(|t| t.with_timezone(&chrono::Local).format("%d/%m/%Y").to_string()).unwrap_or_default()
}

fn appid(g: &Game) -> Option<String> { g.id.strip_prefix("steam-").map(String::from) }

async fn fetch_extra(g: Game) {
    let mut ex = Extra { description: g.description.clone().unwrap_or_default(), genres: vec![g.genre.clone()], ..Default::default() };
    if let Some(aid) = appid(&g) {
        ex.ach = tokio::task::spawn_blocking({ let a = aid.clone(); move || steam::achievements(&a) }).await.unwrap_or_default();
        // Fiche boutique
        let url = format!("https://store.steampowered.com/api/appdetails?appids={aid}&l=french");
        if let Ok(r) = ctx().client.get(&url).send().await {
            if let Ok(v) = r.json::<serde_json::Value>().await {
                let d = &v[&aid]["data"];
                if d.is_object() {
                    let about = strip_markup(d["about_the_game"].as_str().unwrap_or(""));
                    let short = d["short_description"].as_str().unwrap_or("").to_string();
                    ex.description = if about.len() > short.len() { about.chars().take(1100).collect::<String>() } else { short };
                    ex.genres = d["genres"].as_array().map(|a| a.iter().filter_map(|x| x["description"].as_str().map(String::from)).collect()).unwrap_or_default();
                    ex.developer = d["developers"][0].as_str().unwrap_or("").to_string();
                    ex.release = d["release_date"]["date"].as_str().unwrap_or("").to_string();
                    ex.rating = d["metacritic"]["score"].as_i64().map(|s| format!("{s}/100")).unwrap_or_default();
                    ex.shots = d["screenshots"].as_array().map(|a| a.iter().take(8).filter_map(|s| Some((s["path_thumbnail"].as_str()?.to_string(), s["path_full"].as_str()?.to_string()))).collect()).unwrap_or_default();
                }
            }
        }
        // Actualités
        let url = format!("https://api.steampowered.com/ISteamNews/GetNewsForApp/v2/?appid={aid}&count=8&maxlength=500&format=json");
        if let Ok(r) = ctx().client.get(&url).send().await {
            if let Ok(v) = r.json::<serde_json::Value>().await {
                ex.news = v["appnews"]["newsitems"].as_array().map(|a| a.iter().map(|n| {
                    let title = n["title"].as_str().unwrap_or("").to_string();
                    let low = title.to_lowercase();
                    let patch = low.contains("patch") || low.contains("update") || low.contains("hotfix") || low.contains("mise à jour");
                    NewsItem { tag: n["feedlabel"].as_str().unwrap_or("").to_string(), patch, author: n["author"].as_str().unwrap_or("").to_string(),
                        date: ago_date(n["date"].as_i64().unwrap_or(0)), title, summary: strip_markup(n["contents"].as_str().unwrap_or("")).chars().take(320).collect(), url: n["url"].as_str().unwrap_or("").to_string() }
                }).collect()).unwrap_or_default();
            }
        }
    }
    // Steam : images et informations viennent d'IGDB (description et actus restent ceux de Steam)
    if let Some(aid) = appid(&g) {
        if crate::igdb::is_configured() {
            let _ = crate::igdb::fetch_steam(&ctx().client, &[aid.clone()]).await;
            if let Some(ig) = crate::igdb::cached_steam(&aid) {
                if !ig.genres.is_empty() { ex.genres = ig.genres.clone(); }
                if !ig.developer.is_empty() { ex.developer = ig.developer.clone(); }
                if let Some(y) = ig.release_year { ex.release = y.to_string(); }
                if let Some(r) = ig.rating { ex.rating = format!("{:.0}/100", r); }
                let shots = ig.shots();
                if !shots.is_empty() { ex.shots = shots; }
                ex.banner_url = ig.banner_url();
                for u in [ig.banner_url(), ig.cover_url()].into_iter().flatten() { let _ = images::fetch(&ctx().client, &u).await; }
                ex.igdb_note = format!("IGDB : {}", ig.name);
            }
        }
    }
    // IGDB : source de vérité pour les jeux Blizzard référencés (slug prioritaire)
    if g.platform == Platform::BattleNet && (g.igdb_slug.is_some() || g.igdb_id.is_some()) {
        match crate::igdb::fetch(&ctx().client, g.igdb_slug.as_deref(), g.igdb_id).await {
            Ok(ig) => {
                if !ig.summary.is_empty() { ex.description = ig.summary.clone(); }
                if !ig.genres.is_empty() { ex.genres = ig.genres.clone(); }
                if !ig.developer.is_empty() { ex.developer = ig.developer.clone(); }
                if let Some(y) = ig.release_year { ex.release = y.to_string(); }
                if let Some(r) = ig.rating { ex.rating = format!("{:.0}/100", r); }
                let shots = ig.shots();
                if !shots.is_empty() { ex.shots = shots; }
                ex.banner_url = ig.banner_url();
                for u in [ig.banner_url(), ig.cover_url()].into_iter().flatten() { let _ = images::fetch(&ctx().client, &u).await; }
                ex.igdb_note = format!("IGDB : {}", ig.name);
            }
            Err(e) => { eprintln!("[igdb] {e}"); toast(format!("IGDB indisponible : {e}"), 2); }
        }
    }
    // Miniatures + icônes de succès
    let mut urls: Vec<String> = ex.shots.iter().map(|s| s.0.clone()).collect();
    for a in ex.ach.iter().take(60) { if let Some(u) = ach_icon(&g, &a.icon) { urls.push(u); } }
    for u in urls { let _ = images::fetch(&ctx().client, &u).await; }
    ex.loaded = true;
    extra().insert(g.id.clone(), ex);
    crate::request_refresh();
}

fn ach_icon(g: &Game, icon: &str) -> Option<String> {
    if icon.is_empty() { return None; }
    if icon.starts_with("http") { return Some(icon.to_string()); }
    let aid = appid(g)?;
    let file = if icon.contains('.') { icon.to_string() } else { format!("{icon}.jpg") };
    Some(format!("https://cdn.cloudflare.steamstatic.com/steamcommunity/public/images/apps/{aid}/{file}"))
}

fn links_for(g: &Game) -> Vec<Vec<SharedString>> {
    let l = |a: &str, b: String| vec![ss(a), ss(b)];
    match appid(g) {
        Some(a) => vec![
            l("Page boutique", format!("https://store.steampowered.com/app/{a}/")),
            l("DLC", format!("https://store.steampowered.com/dlc/{a}/")),
            l("Hub communautaire", format!("https://steamcommunity.com/app/{a}")),
            l("Discussions", format!("https://steamcommunity.com/app/{a}/discussions/")),
            l("Guides", format!("https://steamcommunity.com/app/{a}/guides/")),
            l("Support", format!("https://help.steampowered.com/fr/wizard/HelpWithGame/?appid={a}")),
        ],
        None => vec![l("Battle.net", "https://shop.battle.net/".into()), l("Support Blizzard", "https://support.blizzard.com/".into())],
    }
}

fn runner_label() -> String {
    let s = crate::settings::load();
    ["GE-Proton 9-20", "Proton Experimental", "Linux Natif", "Wine Staging"][s.runner.min(3)].to_string()
}

pub fn apply(a: &AppWindow) {
    let Some(id) = CURRENT.lock().unwrap().clone() else { return };
    let sh = ctx().shared.lock().unwrap();
    let Some(g) = sh.games.iter().find(|g| g.id == id) else { return };
    let d = a.global::<Detail>();
    let data = crate::to_data(g, &sh, false);
    d.set_game(data);
    let ex_banner = extra().get(&id).and_then(|e| e.banner_url.clone());
    let (banner, has) = match crate::load_image_pub(&ex_banner.or(g.banner_url.clone())) { Some(i) => (i, true), None => (slint::Image::default(), false) };
    d.set_banner(banner); d.set_has_banner(has);
    d.set_size(ss(if g.size_gb > 0.0 { format!("{} Go", g.size_gb) } else { "—".into() }));
    d.set_last_played(ss(if g.last_played > 0 {
        chrono::DateTime::from_timestamp(g.last_played, 0).map(|t| t.with_timezone(&chrono::Local).format("%d/%m/%Y").to_string()).unwrap_or_default()
    } else { "Jamais".into() }));
    d.set_runner(ss(if g.platform == Platform::BattleNet { "Loom Wine 64".to_string() } else { runner_label() }));
    d.set_links(model(links_for(g).into_iter().map(|v| model(v)).collect()));
    d.set_stores(model(vec![]));
    let settings = crate::settings::load();
    let prefix = match g.platform {
        Platform::BattleNet => crate::db::open_db().ok().and_then(|c| crate::db::get_battlenet_config(&c).ok()).map(|c| c.wine_prefix_path).unwrap_or_default(),
        Platform::Steam => appid(g).map(|a| steam::steam_base().join(format!("steamapps/compatdata/{a}/pfx"))).filter(|p| p.exists()).map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|| "~/.local/share/wine/default".into()),
    };
    let kv = |a: &str, b: String| model(vec![ss(a), ss(b)]);
    let mut rows = vec![
        kv(if g.platform == Platform::BattleNet { "Runner Wine" } else { "Compatibility tool" }, runner_label()),
        kv("GameMode / MangoHUD", format!("{} / {}", if settings.gamemode { "GameMode ON" } else { "GameMode OFF" }, if settings.mangohud { "MangoHud ON" } else { "MangoHud OFF" })),
        kv("WINEPREFIX", prefix),
    ];
    if let Some(p) = &g.install_path { rows.push(kv("Emplacement disque", p.to_string_lossy().to_string())); }
    d.set_cfg_rows(model(rows));

    let ex = extra().get(&id).cloned().unwrap_or_default();
    d.set_loading(!ex.loaded);
    d.set_developer(ss(if ex.developer.is_empty() { g.platform_label().to_string() } else { ex.developer.clone() }));
    d.set_genres(ss(if ex.genres.is_empty() { g.genre.clone() } else { ex.genres.join(" • ") }));
    d.set_description(ss(ex.description.clone()));
    d.set_release(ss(ex.release.clone()));
    d.set_rating(ss(ex.rating.clone()));
    d.set_shots(model(ex.shots.iter().map(|(t, _)| match crate::load_image_pub(&Some(t.clone())) { Some(i) => ShotRow { img: i, has: true }, None => ShotRow { img: slint::Image::default(), has: false } }).collect()));
    d.set_news(model(ex.news.iter().map(|n| NewsRow { tag: ss(n.tag.clone()), patch: n.patch, author: ss(n.author.clone()), date: ss(n.date.clone()), title: ss(n.title.clone()), summary: ss(n.summary.clone()) }).collect()));

    // Succès : source locale Steam, sinon résumé de la bibliothèque
    let total = if ex.ach.is_empty() { g.ach.map(|a| a.1).unwrap_or(0) } else { ex.ach.len() as u32 };
    let unlocked = if ex.ach.is_empty() { g.ach.map(|a| a.0).unwrap_or(0) } else { ex.ach.iter().filter(|x| x.unlocked).count() as u32 };
    d.set_ach_total(total as i32); d.set_ach_unlocked(unlocked as i32);
    d.set_ach_pct(if total > 0 { unlocked as f32 / total as f32 } else { 0.0 });
    let row = |x: &steam::Achievement| {
        let (icon, has) = match crate::load_image_pub(&ach_icon(g, &x.icon)) { Some(i) => (i, true), None => (slint::Image::default(), false) };
        AchRow { name: ss(x.name.clone()), desc: ss(x.description.clone()), rarity: ss(if x.rarity > 0.0 { format!("{:.1}%", x.rarity) } else { String::new() }), unlocked: x.unlocked, icon, has_icon: has }
    };
    d.set_ach_preview(model(ex.ach.iter().filter(|x| x.unlocked).take(3).map(row).collect()));
    let mut all: Vec<&steam::Achievement> = ex.ach.iter().collect();
    all.sort_by_key(|x| !x.unlocked);
    d.set_ach_all(model(all.into_iter().map(row).collect()));
}

pub fn open(a: &AppWindow, id: &str) {
    *CURRENT.lock().unwrap() = Some(id.to_string());
    let g = ctx().shared.lock().unwrap().games.iter().find(|g| g.id == id).cloned();
    a.set_page(5);
    a.global::<Detail>().set_news_filter(0);
    if let Some(g) = g {
        if !extra().get(id).map(|e| e.loaded).unwrap_or(false) { ctx().rt.spawn(fetch_extra(g)); }
    }
    apply(a);
}

pub fn wire(app: &AppWindow) {
    let _ = EXTRA.set(Mutex::new(HashMap::new()));
    let d = app.global::<Detail>();
    let w = app.as_weak();
    d.on_back(move || { if let Some(a) = w.upgrade() { a.set_page(0); } });
    d.on_play(|| { let id = CURRENT.lock().unwrap().clone(); if let Some(id) = id { crate::do_launch(id) } });
    d.on_stop(|| { let id = CURRENT.lock().unwrap().clone(); if let Some(id) = id { crate::do_stop(id) } });
    d.on_install(|| { let id = CURRENT.lock().unwrap().clone(); if let Some(id) = id { crate::do_install(id) } });
    let w = app.as_weak();
    d.on_uninstall_confirm(move || {
        if let Some(a) = w.upgrade() { a.global::<Detail>().set_uninstall_open(false); }
        let id = CURRENT.lock().unwrap().clone();
        if let Some(id) = id { crate::do_uninstall(id) }
    });
    d.on_open_url(|u| if let Err(e) = crate::open_url_in_default_browser(&u) { toast(e, 2) });
    d.on_toggle_favorite(|| {
        let Some(id) = CURRENT.lock().unwrap().clone() else { return };
        { let mut s = ctx().shared.lock().unwrap(); if !s.favorites.remove(&id) { s.favorites.insert(id); } crate::library::save_favorites(&s.favorites); }
        crate::request_refresh();
    });
    d.on_open_folder(|| {
        let Some(id) = CURRENT.lock().unwrap().clone() else { return };
        match crate::find(&id).ok_or("Jeu introuvable".to_string()).and_then(|g| crate::library::open_folder(&g)) { Ok(m) => toast(m, 0), Err(e) => toast(e, 2) }
    });
    d.on_news_click(|i| {
        let Some(id) = CURRENT.lock().unwrap().clone() else { return };
        // L'index est relatif à la liste complète
        if let Some(n) = extra().get(&id).and_then(|e| e.news.get(i as usize).cloned()) { let _ = crate::open_url_in_default_browser(&n.url); }
    });
    let w = app.as_weak();
    d.on_open_shot(move |i| {
        let Some(id) = CURRENT.lock().unwrap().clone() else { return };
        let Some(full) = extra().get(&id).and_then(|e| e.shots.get(i as usize).map(|s| s.1.clone())) else { return };
        if let Some(a) = w.upgrade() { let d = a.global::<Detail>(); d.set_shot_ready(false); d.set_shot_open(true); }
        ctx().rt.spawn(async move {
            let ok = images::fetch(&ctx().client, &full).await.is_some();
            with_app(move |a| { let d = a.global::<Detail>(); if ok { if let Some(img) = crate::load_image_pub(&Some(full)) { d.set_shot_big(img); d.set_shot_ready(true); } } });
        });
    });
}
