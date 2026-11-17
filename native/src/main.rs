slint::include_modules!();

// Cœur métier de src-tauri monté tel quel (aucune copie).
#[allow(dead_code)]
#[path = "../../src-tauri/src/db.rs"]
mod db;
#[allow(dead_code)]
mod battlenet;
mod images;
mod accounts;
mod detail;
mod game_config;
mod igdb;
mod library;
mod mcdata;
mod nxm;
mod mon;
#[allow(dead_code)]
mod monitoring;
mod pages;
mod settings;
mod steam;
mod studio;
#[allow(dead_code, unused_imports)]
#[path = "../../src-tauri/src/minecraft/mod.rs"]
mod minecraft;
#[allow(dead_code, unused_imports)]
#[path = "../../src-tauri/src/modding/mod.rs"]
mod modding;
use minecraft::auth as mc_auth;

use library::{Actions, Game, Platform};
use slint::winit_030::WinitWindowAccessor;
use slint::{Color, ComponentHandle, Image, ModelRc, SharedString, VecModel};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

/// Requis par battlenet/auth.rs (`crate::open_url_in_default_browser`).
pub fn open_url_in_default_browser(url: &str) -> Result<(), String> {
    let u = url.trim();
    if !(u.starts_with("http://") || u.starts_with("https://") || u.starts_with("steam://") || u.starts_with("mailto:")) {
        return Err("URL invalide: le protocole doit être http, https, steam ou mailto".into());
    }
    open::that_detached(u).map_err(|e| e.to_string())
}

const PAGE_SIZE: usize = 48;
static LAST_PAGE: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0);

// ---------------------------------------------------------------- état partagé

#[derive(Default)]
struct Shared {
    games: Vec<Game>,
    favorites: HashSet<String>,
    running: Option<String>,
    launching: Option<(String, Instant)>,
    loading: bool,
    displayed: usize,
    toast: Option<(String, i32, Instant)>,
    bnet_active: Option<(String, Instant)>,
    bnet_carry: u64,
}

struct Ctx {
    shared: Arc<Mutex<Shared>>,
    dirty: Arc<AtomicBool>,
    actions: Arc<Actions>,
    client: reqwest::Client,
    rt: tokio::runtime::Handle,
}

static CTX: OnceLock<Ctx> = OnceLock::new();
fn ctx() -> &'static Ctx { CTX.get().expect("ctx") }

thread_local! {
    static IMAGES: RefCell<HashMap<PathBuf, Image>> = RefCell::new(HashMap::new());
}

pub(crate) fn load_image_pub(url: &Option<String>) -> Option<Image> { load_image(url) }

pub(crate) fn request_refresh() { ctx().dirty.store(true, Ordering::Relaxed); }

fn load_image(url: &Option<String>) -> Option<Image> {
    let path = images::cached(url.as_ref()?)?;
    IMAGES.with(|c| {
        let mut c = c.borrow_mut();
        if let Some(i) = c.get(&path) { return Some(i.clone()); }
        let img = Image::load_from_path(&path).ok()?;
        c.insert(path, img.clone());
        Some(img)
    })
}

fn toast(text: impl Into<String>, kind: i32) {
    let mut s = ctx().shared.lock().unwrap();
    s.toast = Some((text.into(), kind, Instant::now() + Duration::from_secs(4)));
    ctx().dirty.store(true, Ordering::Relaxed);
}

// ---------------------------------------------------------------- présentation

fn hue_for(id: &str) -> Color {
    let h = id.bytes().fold(7u32, |a, b| a.wrapping_mul(31).wrapping_add(b as u32));
    let pal = [0x78350f, 0x4c1d95, 0x7f1d1d, 0x9f1239, 0x1e3a5f, 0x14532d, 0x854d0e, 0x1e293b, 0x9a3412, 0x365314];
    let c = pal[(h as usize) % pal.len()];
    Color::from_rgb_u8((c >> 16) as u8, (c >> 8) as u8, c as u8)
}

fn hours(min: u32) -> String {
    if min > 0 { format!("{:.1} h", min as f32 / 60.0) } else { "Non joué".into() }
}

fn to_data(g: &Game, s: &Shared, hero: bool) -> GameData {
    let img = if hero { load_image(&g.banner_url).or_else(|| load_image(&g.cover_url)) } else { load_image(&g.cover_url) };
    let running = s.running.as_deref() == Some(g.id.as_str());
    let starting = !running && s.launching.as_ref().map(|(i, _)| i == &g.id).unwrap_or(false);
    GameData {
        id: g.id.clone().into(),
        title: g.title.clone().into(),
        subtitle: if hero { g.subtitle.clone().into() } else { "".into() },
        platform: match g.platform { Platform::Steam => "STEAM", Platform::BattleNet => "BATTLE.NET" }.into(),
        platform_variant: match g.platform { Platform::Steam => 3, Platform::BattleNet => 1 },
        genre: g.genre.clone().into(),
        playtime: hours(g.playtime_min).into(),
        size_label: if !g.installed { "À télécharger".to_string() }
            else if g.size_gb > 0.0 { format!("{} Go", g.size_gb) }
            else { match g.platform { Platform::Steam => "proton", Platform::BattleNet => "wine" }.to_string() }.into(),
        installed: g.installed,
        favorite: s.favorites.contains(&g.id),
        running,
        starting,
        has_cover: img.is_some(),
        cover: img.unwrap_or_default(),
        hue: hue_for(&g.id),
    }
}

fn passes(g: &Game, s: &Shared, filter: i32) -> bool {
    match filter {
        1 => g.installed,
        2 => !g.installed,
        3 => s.favorites.contains(&g.id),
        4 => g.platform == Platform::Steam,
        5 => g.platform == Platform::BattleNet,
        _ => true,
    }
}

fn refresh(app: &AppWindow) {
    let s = ctx().shared.lock().unwrap();
    let q = app.get_search().to_lowercase();
    let filter = app.get_filter();

    let mut filtered: Vec<&Game> = s.games.iter()
        .filter(|g| passes(g, &s, filter) && (q.is_empty() || g.title.to_lowercase().contains(&q)))
        .collect();
    // Le jeu en cours remonte en tête (comportement `recentGames` du store Vue)
    filtered.sort_by_key(|g| std::cmp::Reverse((s.running.as_deref() == Some(g.id.as_str()), g.last_played)));
    let total_filtered = filtered.len();
    let shown: Vec<GameData> = filtered.iter().take(s.displayed.max(PAGE_SIZE)).map(|g| to_data(g, &s, false)).collect();
    app.set_games(ModelRc::new(VecModel::from(shown)));
    app.set_filtered_total(total_filtered as i32);

    // Récents (8 max), jeu en cours en premier
    let mut recent: Vec<&Game> = s.games.iter().filter(|g| g.last_played > 0).collect();
    recent.sort_by_key(|g| std::cmp::Reverse((s.running.as_deref() == Some(g.id.as_str()), g.last_played)));
    app.set_recent(ModelRc::new(VecModel::from(
        recent.iter().take(8).map(|g| to_data(g, &s, false)).collect::<Vec<_>>(),
    )));

    // Héros : jeu en cours sinon dernier joué sinon premier
    let hero = s.running.as_ref().and_then(|id| s.games.iter().find(|g| &g.id == id))
        .or(recent.first().copied())
        .or(s.games.first());
    match hero {
        Some(g) => {
            app.set_has_hero(true);
            app.set_hero(to_data(g, &s, true));
            app.set_hero_playtime(hours(g.playtime_min).into());
            app.set_hero_unlocked(g.ach.map(|a| a.0 as i32).unwrap_or(0));
            app.set_hero_total(g.ach.map(|a| a.1 as i32).unwrap_or(0));
        }
        None => app.set_has_hero(false),
    }

    let count = |f: &dyn Fn(&Game) -> bool| s.games.iter().filter(|g| f(g)).count();
    let installed = count(&|g| g.installed);
    app.set_installed_count(installed as i32);
    app.set_total_count(s.games.len() as i32);
    app.set_tag_labels(ModelRc::new(VecModel::from(vec![
        SharedString::from(format!("Tous ({})", s.games.len())),
        format!("Installés ({installed})").into(),
        format!("Non installés ({})", s.games.len() - installed).into(),
        "Favoris".into(),
        format!("Steam ({})", count(&|g| g.platform == Platform::Steam)).into(),
        format!("Battle.net ({})", count(&|g| g.platform == Platform::BattleNet)).into(),
    ])));
    app.set_loading(s.loading);
    app.set_running_title(s.running.as_ref().and_then(|id| s.games.iter().find(|g| &g.id == id)).map(|g| g.title.as_str()).unwrap_or("").into());
    app.set_connected_count(0);
    match &s.toast {
        Some((t, k, until)) if *until > Instant::now() => {
            app.set_toast_text(t.clone().into());
            app.set_toast_kind(*k);
            app.set_toast_visible(true);
        }
        _ => app.set_toast_visible(false),
    }
    // Les vues annexes relisent l'état partagé : le verrou doit être libéré avant
    drop(s);
    drop_studio_hook(app);
}

fn drop_studio_hook(app: &AppWindow) { match app.get_page() { 1 => studio::apply_all(app), 5 => detail::apply(app), _ => {} } }

// ---------------------------------------------------------------- tâches async

fn find(id: &str) -> Option<Game> {
    ctx().shared.lock().unwrap().games.iter().find(|g| g.id == id).cloned()
}

fn spawn_cover_downloads(games: &[Game]) {
    let client = ctx().client.clone();
    let sem = Arc::new(tokio::sync::Semaphore::new(6));
    for url in games.iter().flat_map(|g| [g.cover_url.clone(), g.banner_url.clone()]).flatten() {
        if images::cached(&url).is_some() { continue; }
        let (client, sem) = (client.clone(), sem.clone());
        ctx().rt.spawn(async move {
            let _p = sem.acquire().await;
            if images::fetch(&client, &url).await.is_some() {
                ctx().dirty.store(true, Ordering::Relaxed);
            }
        });
    }
}

fn enrich_igdb(games: &[Game]) {
    static DONE: AtomicBool = AtomicBool::new(false);
    if DONE.swap(true, Ordering::Relaxed) || !igdb::is_configured() { return; }
    let bnet: Vec<(Option<String>, Option<i64>)> = games.iter().filter(|g| g.platform == Platform::BattleNet && (g.igdb_slug.is_some() || g.igdb_id.is_some())).map(|g| (g.igdb_slug.clone(), g.igdb_id)).collect();
    let steam_ids: Vec<String> = games.iter().filter_map(|g| g.id.strip_prefix("steam-").map(String::from)).collect();
    ctx().rt.spawn(async move {
        let mut any = false;
        for (slug, id) in bnet { if igdb::fetch(&ctx().client, slug.as_deref(), id).await.is_ok() { any = true; } }
        match igdb::fetch_steam(&ctx().client, &steam_ids).await {
            Ok(n) => { eprintln!("[igdb] {n} jeux Steam associés"); if n > 0 { any = true; } }
            Err(e) => eprintln!("[igdb] {e}"),
        }
        if any { reload(); }
    });
}

fn reload() {
    {
        let mut s = ctx().shared.lock().unwrap();
        if s.loading { return; }
        s.loading = true;
    }
    ctx().dirty.store(true, Ordering::Relaxed);
    ctx().rt.spawn(async {
        let games = library::load_all().await;
        eprintln!("[loom] bibliothèque chargée : {} jeux ({} Steam, {} Battle.net)", games.len(), games.iter().filter(|g| g.platform == Platform::Steam).count(), games.iter().filter(|g| g.platform == Platform::BattleNet).count());
        spawn_cover_downloads(&games);
        enrich_igdb(&games);
        let mut s = ctx().shared.lock().unwrap();
        // Conserve un « dernier lancement » local plus récent que celui du scan
        let old: HashMap<String, i64> = s.games.iter().map(|g| (g.id.clone(), g.last_played)).collect();
        s.games = games;
        for g in s.games.iter_mut() {
            if let Some(&t) = old.get(&g.id) { if t > g.last_played { g.last_played = t; } }
        }
        s.loading = false;
        drop(s);
        ctx().dirty.store(true, Ordering::Relaxed);
        pages::reapply_accounts();
    });
}

fn do_launch(id: String) {
    let Some(g) = find(&id) else { return };
    if !g.installed { return do_install(id); }
    ctx().shared.lock().unwrap().launching = Some((id.clone(), Instant::now()));
    ctx().dirty.store(true, Ordering::Relaxed);
    ctx().rt.spawn(async move {
        match ctx().actions.launch(&g).await {
            Ok(m) => {
                toast(m, 1);
                // Comportement « après le lancement d'un jeu » (Paramètres)
                let after = settings::load().after_launch;
                if after == 0 { pages::with_app(|a| a.window().set_minimized(true)); } else if after == 1 { pages::with_app(|a| { let _ = a.window().hide(); }); }
            }
            Err(e) => {
                ctx().shared.lock().unwrap().launching = None;
                toast(format!("Échec du lancement : {e}"), 2);
            }
        }
    });
}

fn do_stop(id: String) {
    let Some(g) = find(&id) else { return };
    ctx().rt.spawn(async move {
        match ctx().actions.stop(&g).await {
            Ok(m) => toast(m, 1),
            Err(e) => toast(format!("Échec de l'arrêt : {e}"), 2),
        }
    });
}

fn do_install(id: String) {
    let Some(g) = find(&id) else { return };
    ctx().rt.spawn(async move {
        match ctx().actions.install(&g).await {
            Ok(m) => toast(m, 0),
            Err(e) => toast(format!("Échec de l'installation : {e}"), 2),
        }
    });
}

fn do_uninstall(id: String) {
    let Some(g) = find(&id) else { return };
    ctx().rt.spawn(async move {
        match ctx().actions.uninstall(&g).await {
            Ok(m) => toast(m, 0),
            Err(e) => toast(format!("Échec de la désinstallation : {e}"), 2),
        }
    });
}

/// Sonde l'état réel (processus Steam / Battle.net) toutes les 2 s.
async fn poll_loop() {
    let mut tick = tokio::time::interval(Duration::from_secs(2));
    loop {
        tick.tick().await;
        let running = library::running_ids().await;
        let now = chrono::Utc::now().timestamp();
        let mut stopped = false;
        {
            let mut s = ctx().shared.lock().unwrap();
            let prev = s.running.clone();
            // Battle.net prioritaire comme dans checkRunningGames()
            let cur = running.iter().find(|i| i.starts_with("bnet-")).or(running.first()).cloned();

            if let Some(id) = &cur {
                if let Some(g) = s.games.iter_mut().find(|g| &g.id == id) { g.last_played = now; }
                if s.launching.as_ref().map(|(l, _)| l == id).unwrap_or(false) { s.launching = None; }
            }
            // Temps de jeu Battle.net : delta persisté en base
            match (&cur, s.bnet_active.clone()) {
                (Some(id), Some((aid, t0))) if id.starts_with("bnet-") && &aid == id => {
                    let delta = t0.elapsed().as_secs();
                    if delta > 0 && delta < 30 {
                        library::persist_bnet_playtime(id.trim_start_matches("bnet-"), delta, now);
                        s.bnet_carry += delta;
                        if s.bnet_carry >= 60 {
                            let m = (s.bnet_carry / 60) as u32;
                            s.bnet_carry %= 60;
                            if let Some(g) = s.games.iter_mut().find(|g| &g.id == id) { g.playtime_min += m; }
                        }
                    }
                    s.bnet_active = Some((aid, Instant::now()));
                }
                (Some(id), _) if id.starts_with("bnet-") => s.bnet_active = Some((id.clone(), Instant::now())),
                (_, Some((aid, _))) => {
                    library::persist_bnet_playtime(aid.trim_start_matches("bnet-"), 0, now);
                    s.bnet_active = None;
                }
                _ => {}
            }
            if let Some((_, t)) = &s.launching { if t.elapsed() > Duration::from_secs(90) { s.launching = None; } }
            if prev != cur { stopped = prev.is_some() && cur.is_none(); s.running = cur; }
        }
        ctx().dirty.store(true, Ordering::Relaxed);
        if stopped { reload(); } // récupère le temps de jeu Steam à jour
    }
}

// ---------------------------------------------------------------- main

fn main() {
    // Instance unique : un lien nxm:// (ou un second lancement) est transmis à l'instance déjà ouverte
    let nxm_uris = nxm::uris_from_args();
    if nxm::forward_to_running(&nxm_uris) { return; }
    let rt = tokio::runtime::Runtime::new().expect("runtime tokio");
    let _guard = rt.enter();

    let client = reqwest::Client::builder()
        .user_agent("LoomLauncher/0.1")
        .timeout(Duration::from_secs(20))
        .build()
        .expect("client http");
    let favorites = library::load_favorites();
    let _ = CTX.set(Ctx {
        shared: Arc::new(Mutex::new(Shared { favorites, displayed: PAGE_SIZE, ..Default::default() })),
        dirty: Arc::new(AtomicBool::new(true)),
        actions: Arc::new(Actions::new()),
        client,
        rt: rt.handle().clone(),
    });

    let app = AppWindow::new().unwrap();

    // --- Fenêtre ---
    // Fermer : réduit dans la barre d'état si « Close to Tray » est actif, sinon quitte
    let w = app.as_weak();
    app.on_close_clicked(move || {
        if settings::load().close_to_tray {
            if let Some(a) = w.upgrade() { let _ = a.window().hide(); }
        } else {
            let _ = slint::quit_event_loop();
        }
    });
    let w = app.as_weak();
    app.on_minimize_clicked(move || { if let Some(a) = w.upgrade() { a.window().set_minimized(true); } });
    let w = app.as_weak();
    app.on_maximize_clicked(move || {
        if let Some(a) = w.upgrade() {
            let m = !a.window().is_maximized();
            a.window().set_maximized(m);
            a.set_win_maximized(m);
        }
    });
    let w = app.as_weak();
    app.on_drag_window(move |x, y| {
        if let Some(a) = w.upgrade() {
            a.window().with_winit_window(|win| { let _ = win.drag_window(); });
            // Le compositeur capture le pointeur pendant le déplacement : Slint ne reçoit jamais
            // le relâchement et resterait « en appui » (UI figée). On l'émet nous-mêmes.
            a.window().dispatch_event(slint::platform::WindowEvent::PointerReleased {
                position: slint::LogicalPosition::new(x, y),
                button: slint::platform::PointerEventButton::Left,
            });
            a.window().dispatch_event(slint::platform::WindowEvent::PointerExited);
        }
    });
    app.on_go_back(|| {});
    app.on_go_forward(|| {});

    // --- Bibliothèque ---
    app.on_play(|id| do_launch(id.to_string()));
    app.on_stop(|id| do_stop(id.to_string()));
    app.on_install(|id| do_install(id.to_string()));
    app.on_uninstall(|id| do_uninstall(id.to_string()));
    app.on_open_folder(|id| {
        match find(&id).ok_or("Jeu introuvable".to_string()).and_then(|g| library::open_folder(&g)) {
            Ok(m) => toast(m, 0),
            Err(e) => toast(e, 2),
        }
    });
    let w = app.as_weak();
    app.on_toggle_favorite(move |id| {
        {
            let mut s = ctx().shared.lock().unwrap();
            let id = id.to_string();
            if !s.favorites.remove(&id) { s.favorites.insert(id); }
            library::save_favorites(&s.favorites);
        }
        if let Some(a) = w.upgrade() { refresh(&a); }
    });
    let w = app.as_weak();
    app.on_hero_action(move || {
        let Some(a) = w.upgrade() else { return };
        let hero = a.get_hero();
        if hero.running { do_stop(hero.id.to_string()) } else { do_launch(hero.id.to_string()) }
    });
    let w = app.as_weak();
    app.on_open_game(move |id| { if let Some(a) = w.upgrade() { detail::open(&a, &id); } });
    app.on_refresh(|| reload());
    app.on_open_igdb(|| {});
    app.on_load_more(|| {
        ctx().shared.lock().unwrap().displayed += PAGE_SIZE;
        ctx().dirty.store(true, Ordering::Relaxed);
    });
    let w = app.as_weak();
    app.on_search_changed(move |_| {
        ctx().shared.lock().unwrap().displayed = PAGE_SIZE;
        if let Some(a) = w.upgrade() { refresh(&a); }
    });

    // Timer UI : applique les changements poussés par les tâches async (dirty flag)
    let w = app.as_weak();
    let timer = slint::Timer::default();
    timer.start(slint::TimerMode::Repeated, Duration::from_millis(250), move || {
        if let Some(a) = w.upgrade() {
            let toast_active = ctx().shared.lock().unwrap().toast.as_ref().map(|t| t.2 > Instant::now()).unwrap_or(false);
            let pg = a.get_page();
            let changed = LAST_PAGE.swap(pg, Ordering::Relaxed) != pg;
            if ctx().dirty.swap(false, Ordering::Relaxed) || changed || (a.get_toast_visible() && !toast_active) {
                refresh(&a);
            }
        }
    });

    pages::wire(&app);
    mon::wire(&app);
    studio::wire(&app);
    nxm::start_server();
    if let Err(e) = nxm::register_handler(false) { eprintln!("[nxm] enregistrement du gestionnaire : {e}"); }
    for u in nxm_uris { nxm::handle(u); }
    detail::wire(&app);
    rt.spawn(poll_loop());
    reload();

    // Icône de la barre d'état (même rôle que le tray du Loom Tauri)
    let tray = Tray::new().expect("icône de zone de notification");
    let w = app.as_weak();
    tray.on_show_window(move || { if let Some(a) = w.upgrade() { let _ = a.window().show(); a.window().set_minimized(false); } });
    let w = app.as_weak();
    tray.on_hide_window(move || { if let Some(a) = w.upgrade() { let _ = a.window().hide(); } });
    let w = app.as_weak();
    tray.on_toggle_window(move || {
        if let Some(a) = w.upgrade() {
            if a.window().is_visible() && !a.window().is_minimized() { let _ = a.window().hide(); }
            else { let _ = a.window().show(); a.window().set_minimized(false); }
        }
    });
    tray.on_quit_app(|| { let _ = slint::quit_event_loop(); });
    tray.show().ok();

    app.run().unwrap();
    drop(tray);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game(id: &str, platform: Platform, installed: bool) -> Game {
        Game { id: id.into(), title: id.into(), subtitle: String::new(), platform, installed, playtime_min: 0, last_played: 0, size_gb: 0.0,
            genre: String::new(), cover_url: None, banner_url: None, install_path: None, description: None, ach: None, igdb_slug: None, igdb_id: None }
    }

    #[test]
    fn library_filters_match_the_vue_tags() {
        let mut s = Shared::default();
        s.favorites.insert("steam-1".into());
        let (a, b, c) = (game("steam-1", Platform::Steam, true), game("bnet-wow", Platform::BattleNet, false), game("steam-2", Platform::Steam, false));
        assert!(passes(&a, &s, 0) && passes(&b, &s, 0) && passes(&c, &s, 0));
        assert!(passes(&a, &s, 1) && !passes(&b, &s, 1));
        assert!(!passes(&a, &s, 2) && passes(&b, &s, 2) && passes(&c, &s, 2));
        assert!(passes(&a, &s, 3) && !passes(&b, &s, 3));
        assert!(passes(&a, &s, 4) && !passes(&b, &s, 4));
        assert!(passes(&b, &s, 5) && !passes(&a, &s, 5));
    }

    #[test]
    fn playtime_is_shown_in_hours() {
        assert_eq!(hours(0), "Non joué");
        assert_eq!(hours(90), "1.5 h");
        assert_eq!(hours(3618), "60.3 h");
    }

    #[test]
    fn placeholder_colors_are_deterministic() {
        assert_eq!(hue_for("steam-1"), hue_for("steam-1"));
    }

    #[test]
    fn card_data_reflects_running_and_starting_states() {
        let mut s = Shared::default();
        let g = game("steam-1", Platform::Steam, true);
        assert!(!to_data(&g, &s, false).running);
        s.running = Some("steam-1".into());
        let d = to_data(&g, &s, false);
        assert!(d.running && !d.starting);
        s.running = None;
        s.launching = Some(("steam-1".into(), Instant::now()));
        let d = to_data(&g, &s, false);
        assert!(d.starting && !d.running);
        assert_eq!(d.platform_variant, 3);
        let uninstalled = to_data(&game("bnet-wow", Platform::BattleNet, false), &s, false);
        assert_eq!(uninstalled.size_label, "À télécharger");
        assert_eq!(uninstalled.platform_variant, 1);
    }
}
