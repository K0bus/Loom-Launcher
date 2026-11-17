//! Câblage des pages Paramètres et Comptes (globals Slint `Settings` / `Accounts`).

use crate::{accounts, ctx, images, library, mc_auth, reload, settings, toast, AppWindow};
use crate::{Accounts, ListRow, Settings, Theme};
use slint::{Color, ComponentHandle, Image, ModelRc, SharedString, VecModel, Weak};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

static APP: OnceLock<Mutex<Weak<AppWindow>>> = OnceLock::new();
static LAST: Mutex<Option<Snapshot>> = Mutex::new(None);

/// Ré-applique le dernier instantané de comptes (appelé quand la bibliothèque change).
pub fn reapply_accounts() {
    with_app(|a| { let snap = LAST.lock().unwrap().clone(); if let Some(s) = snap { apply_accounts(a, s); } });
}

/// Exécute `f` sur le thread UI (depuis une tâche async).
pub fn with_app(f: impl FnOnce(&AppWindow) + Send + 'static) {
    if let Some(w) = APP.get() {
        let weak = w.lock().unwrap().clone();
        let _ = weak.upgrade_in_event_loop(move |a| f(&a));
    }
}

// ---------------------------------------------------------------- Paramètres

const PALETTES: [[u32; 5]; 5] = [
    [0xf59e0b, 0xfbbf24, 0xfcd34d, 0xfde68a, 0xd97706], // amber
    [0x10b981, 0x34d399, 0x6ee7b7, 0xa7f3d0, 0x059669], // emerald
    [0x8b5cf6, 0xa78bfa, 0xc4b5fd, 0xddd6fe, 0x7c3aed], // violet
    [0x0ea5e9, 0x38bdf8, 0x7dd3fc, 0xbae6fd, 0x0284c7], // sky
    [0xf43f5e, 0xfb7185, 0xfda4af, 0xfecdd3, 0xe11d48], // rose
];

fn col(c: u32) -> Color { Color::from_rgb_u8((c >> 16) as u8, (c >> 8) as u8, c as u8) }

pub fn apply_accent(app: &AppWindow, idx: usize) {
    let p = PALETTES[idx.min(4)];
    let t = app.global::<Theme>();
    t.set_amber_500(col(p[0]));
    t.set_amber_400(col(p[1]));
    t.set_amber_300(col(p[2]));
    t.set_amber_200(col(p[3]));
    t.set_amber_600(col(p[4]));
}

fn push_settings(app: &AppWindow, s: &settings::AppSettings) {
    let g = app.global::<Settings>();
    g.set_autostart(s.autostart);
    g.set_start_minimized(s.start_minimized);
    g.set_close_to_tray(s.close_to_tray);
    g.set_after_launch(s.after_launch as i32);
    g.set_inst_steam(s.inst[0]);
    g.set_inst_bnet(s.inst[1]);
    g.set_inst_epic(s.inst[2]);
    g.set_inst_gog(s.inst[3]);
    g.set_inst_silent(s.inst_silent);
    g.set_accent(s.accent as i32);
    g.set_notifications(s.notifications);
    g.set_auto_sync(s.auto_sync);
    g.set_runner(s.runner as i32);
    g.set_gamemode(s.gamemode);
    g.set_mangohud(s.mangohud);
    g.set_dir_games(s.dir_games.clone().into());
    g.set_dir_minecraft(s.dir_minecraft.clone().into());
    g.set_dir_wine(s.dir_wine.clone().into());
    g.set_check_updates(s.check_updates);
    g.set_igdb_id(s.igdb_id.clone().into());
    g.set_igdb_token(s.igdb_token.clone().into());
    g.set_igdb_secret(s.igdb_secret.clone().into());
    let i = settings::sys_info();
    g.set_sys_version(i.version.into());
    g.set_sys_display(i.display.into());
    g.set_sys_kernel(i.kernel.into());
    g.set_sys_ram(i.ram.into());
}

fn pull_settings(app: &AppWindow) -> settings::AppSettings {
    let g = app.global::<Settings>();
    settings::AppSettings {
        autostart: g.get_autostart(),
        start_minimized: g.get_start_minimized(),
        close_to_tray: g.get_close_to_tray(),
        after_launch: g.get_after_launch() as usize,
        inst: [g.get_inst_steam(), g.get_inst_bnet(), g.get_inst_epic(), g.get_inst_gog()],
        inst_silent: g.get_inst_silent(),
        accent: g.get_accent() as usize,
        notifications: g.get_notifications(),
        auto_sync: g.get_auto_sync(),
        runner: g.get_runner() as usize,
        gamemode: g.get_gamemode(),
        mangohud: g.get_mangohud(),
        dir_games: g.get_dir_games().to_string(),
        dir_minecraft: g.get_dir_minecraft().to_string(),
        dir_wine: g.get_dir_wine().to_string(),
        check_updates: g.get_check_updates(),
        igdb_id: g.get_igdb_id().to_string(),
        igdb_token: g.get_igdb_token().to_string(),
        igdb_secret: g.get_igdb_secret().to_string(),
    }
}

fn wire_settings(app: &AppWindow) {
    let g = app.global::<Settings>();

    let w = app.as_weak();
    g.on_save(move || {
        let Some(a) = w.upgrade() else { return };
        let s = pull_settings(&a);
        match settings::save(&s) {
            Ok(_) => {
                a.global::<Settings>().set_saved(true);
                toast("Paramètres de Loom enregistrés avec succès !", 1);
                ctx().rt.spawn(async {
                    tokio::time::sleep(Duration::from_millis(2500)).await;
                    with_app(|a| a.global::<Settings>().set_saved(false));
                });
            }
            Err(e) => toast(format!("Échec de l'enregistrement : {e}"), 2),
        }
    });

    let w = app.as_weak();
    g.on_reset(move || {
        let Some(a) = w.upgrade() else { return };
        let mut d = settings::AppSettings::default();
        // Les identifiants IGDB ne font pas partie des réglages « par défaut »
        let cur = pull_settings(&a);
        (d.igdb_id, d.igdb_token, d.igdb_secret) = (cur.igdb_id, cur.igdb_token, cur.igdb_secret);
        push_settings(&a, &d);
        apply_accent(&a, d.accent);
        let _ = settings::save(&d);
        toast("Paramètres réinitialisés aux valeurs recommandées", 0);
    });

    let w = app.as_weak();
    g.on_accent_changed(move |i| { if let Some(a) = w.upgrade() { apply_accent(&a, i as usize); } });

    let w = app.as_weak();
    g.on_pick_dir(move |i| {
        let Some(a) = w.upgrade() else { return };
        let s = pull_settings(&a);
        let (title, cur) = match i {
            0 => ("Sélectionner le dossier Bibliothèque Jeux", s.dir_games),
            1 => ("Sélectionner le dossier des instances Minecraft", s.dir_minecraft),
            _ => ("Sélectionner le dossier des préfixes Wine", s.dir_wine),
        };
        ctx().rt.spawn(async move {
            let picked = tokio::task::spawn_blocking(move || settings::pick_directory(title, &cur)).await.ok().flatten();
            match picked {
                Some(p) => with_app(move |a| {
                    let g = a.global::<Settings>();
                    match i { 0 => g.set_dir_games(p.into()), 1 => g.set_dir_minecraft(p.into()), _ => g.set_dir_wine(p.into()) }
                    toast("Répertoire mis à jour (pensez à enregistrer)", 1);
                }),
                None => toast("Aucun dossier sélectionné (zenity / kdialog requis)", 0),
            }
        });
    });

    let w = app.as_weak();
    g.on_open_dir(move |i| {
        let Some(a) = w.upgrade() else { return };
        let s = pull_settings(&a);
        let p = match i { 0 => s.dir_games, 1 => s.dir_minecraft, _ => s.dir_wine };
        match settings::open_directory(&p) {
            Ok(_) => toast("Dossier ouvert dans le gestionnaire de fichiers", 0),
            Err(e) => toast(e, 2),
        }
    });

    g.on_test_notification(|| {
        match settings::notify("Loom Launcher", "Test réussi : Les notifications de bureau sont pleinement opérationnelles !") {
            Ok(_) => toast("Notification de bureau envoyée !", 1),
            Err(e) => toast(e, 2),
        }
    });

    let w = app.as_weak();
    g.on_gen_token(move || {
        let Some(a) = w.upgrade() else { return };
        let s = pull_settings(&a);
        a.global::<Settings>().set_igdb_busy(true);
        ctx().rt.spawn(async move {
            let r = settings::twitch_token(&ctx().client, &s.igdb_id, &s.igdb_secret).await;
            with_app(move |a| {
                let g = a.global::<Settings>();
                g.set_igdb_busy(false);
                match r {
                    Ok(t) => { g.set_igdb_token(t.into()); toast("Token IGDB généré automatiquement ! (pensez à enregistrer)", 1); }
                    Err(e) => toast(format!("Échec de génération du token : {e}"), 2),
                }
            });
        });
    });
}

// ---------------------------------------------------------------- Comptes

#[derive(Clone)]
struct Snapshot {
    steam: accounts::SteamAccount,
    bnet: accounts::BnetInfo,
    ms: Option<(String, String)>,
}

fn rows(games: &[&crate::Game], installed_badge: &str) -> Vec<ListRow> {
    games.iter().map(|g| {
        let (badge, variant, detail) = if g.installed {
            let b = if g.size_gb > 0.0 { format!("{} Go", g.size_gb) } else { installed_badge.to_string() };
            (b, 1, String::new())
        } else if g.playtime_min > 0 {
            (format!("{:.1} h", g.playtime_min as f32 / 60.0), 0, String::new())
        } else { ("Non installé".to_string(), 0, String::new()) };
        ListRow { name: g.title.clone().into(), detail: detail.into(), badge: badge.into(), badge_variant: variant }
    }).collect()
}

fn apply_accounts(app: &AppWindow, snap: Snapshot) {
    *LAST.lock().unwrap() = Some(snap.clone());
    let g = app.global::<Accounts>();
    let s = ctx().shared.lock().unwrap();
    let steam: Vec<&crate::Game> = s.games.iter().filter(|g| g.platform == library::Platform::Steam).collect();
    let bnet: Vec<&crate::Game> = s.games.iter().filter(|g| g.platform == library::Platform::BattleNet).collect();
    let (st_in, st_out): (Vec<_>, Vec<_>) = steam.iter().copied().partition(|g| g.installed);
    let (bn_in, bn_out): (Vec<_>, Vec<_>) = bnet.iter().copied().partition(|g| g.installed);

    g.set_steam_connected(snap.steam.connected);
    g.set_steam_name(snap.steam.persona.into());
    g.set_steam_user(snap.steam.user.into());
    g.set_steam_id(snap.steam.id64.into());
    g.set_steam_path(snap.steam.path.into());
    g.set_steam_installed(st_in.len() as i32);
    g.set_steam_total(steam.len() as i32);
    g.set_steam_installed_list(ModelRc::new(VecModel::from(rows(&st_in, "installé"))));
    g.set_steam_other_list(ModelRc::new(VecModel::from(rows(&st_out, ""))));

    g.set_bnet_connected(snap.bnet.connected);
    g.set_bnet_tag(snap.bnet.tag.into());
    g.set_bnet_prefix(snap.bnet.prefix.into());
    g.set_bnet_runner(snap.bnet.runner.into());
    g.set_bnet_options(snap.bnet.options.into());
    g.set_blizz_configured(snap.bnet.credentials_configured);
    if g.get_blizz_id().is_empty() { g.set_blizz_id(snap.bnet.client_id.into()); }
    g.set_bnet_installed(bn_in.len() as i32);
    g.set_bnet_owned(bn_out.len() as i32);
    g.set_bnet_installed_list(ModelRc::new(VecModel::from(
        bn_in.iter().map(|x| ListRow { name: x.title.clone().into(), detail: x.subtitle.clone().into(), badge: "Local".into(), badge_variant: 1 }).collect::<Vec<_>>(),
    )));
    g.set_bnet_owned_list(ModelRc::new(VecModel::from(
        bn_out.iter().map(|x| ListRow { name: x.title.clone().into(), detail: "".into(), badge: "Possédé".into(), badge_variant: 3 }).collect::<Vec<_>>(),
    )));
    drop(s);

    g.set_ms_connected(snap.ms.is_some());
    if let Some((name, uuid)) = &snap.ms {
        g.set_ms_name(name.clone().into());
        let url = Some(format!("https://mc-heads.net/avatar/{uuid}/64"));
        match crate::load_image_pub(&url) {
            Some(img) => { g.set_ms_avatar(img); g.set_ms_has_avatar(true); }
            None => g.set_ms_has_avatar(false),
        }
    } else {
        g.set_ms_name(SharedString::new());
        g.set_ms_has_avatar(false);
    }
    match crate::load_image_pub(&snap.bnet.avatar_url) {
        Some(img) => { g.set_bnet_avatar(img); g.set_bnet_has_avatar(true); }
        None => g.set_bnet_has_avatar(false),
    }
}

pub fn refresh_accounts() {
    ctx().rt.spawn(async {
        let steam = tokio::task::spawn_blocking(accounts::steam_account).await.unwrap_or_default();
        let bnet = accounts::bnet_info().await;
        let ms = accounts::microsoft().await;
        // Pré-télécharge les avatars pour l'affichage
        if let Some(u) = &bnet.avatar_url { let _ = images::fetch(&ctx().client, u).await; }
        if let Some((_, uuid)) = &ms { let _ = images::fetch(&ctx().client, &format!("https://mc-heads.net/avatar/{uuid}/64")).await; }
        with_app(move |a| apply_accounts(a, Snapshot { steam, bnet, ms }));
    });
}

fn run_blizzard_oauth(close_modal: bool) {
    with_app(|a| {
        let g = a.global::<Accounts>();
        g.set_bnet_busy(true);
        g.set_blizz_error(SharedString::new());
        if g.get_blizz_open() { g.set_blizz_authenticating(true); }
    });
    ctx().rt.spawn(async move {
        let r = crate::battlenet::auth::start_blizzard_oauth_flow().await;
        with_app(move |a| {
            let g = a.global::<Accounts>();
            g.set_bnet_busy(false);
            g.set_blizz_authenticating(false);
            match r {
                Ok(acc) => {
                    if close_modal { g.set_blizz_open(false); }
                    toast(format!("Compte Blizzard lié : {}", acc.battletag), 1);
                    refresh_accounts();
                    reload();
                }
                Err(e) => { g.set_blizz_error(e.clone().into()); toast(format!("Échec de la liaison Blizzard : {e}"), 2); }
            }
        });
    });
}

pub fn start_microsoft_login_pub() { start_microsoft_login() }

fn start_microsoft_login() {
    with_app(|a| {
        let g = a.global::<Accounts>();
        g.set_ms_state(0);
        g.set_ms_code("".into());
        g.set_ms_uri("".into());
        g.set_ms_open(true);
    });
    ctx().rt.spawn(async {
        let client = reqwest::Client::new();
        let dc = match mc_auth::initiate_device_code(&client).await {
            Ok(d) => d,
            Err(e) => {
                with_app(move |a| { let g = a.global::<Accounts>(); g.set_ms_error(e.into()); g.set_ms_state(2); });
                return;
            }
        };
        let (code, uri) = (dc.user_code.clone(), dc.verification_uri.clone());
        with_app(move |a| { let g = a.global::<Accounts>(); g.set_ms_code(code.into()); g.set_ms_uri(uri.into()); });
        let _ = crate::open_url_in_default_browser(&dc.verification_uri);

        let deadline = std::time::Instant::now() + Duration::from_secs(dc.expires_in.max(60));
        let every = Duration::from_secs(dc.interval.max(3));
        loop {
            tokio::time::sleep(every).await;
            if std::time::Instant::now() > deadline {
                with_app(|a| { let g = a.global::<Accounts>(); g.set_ms_error("Code expiré, veuillez recommencer.".into()); g.set_ms_state(2); });
                return;
            }
            match mc_auth::poll_device_code(&client, &dc.device_code).await {
                Ok(r) if r.pending => continue,
                Ok(r) => {
                    if let Some(acc) = r.account {
                        let _ = mc_auth::save_stored_account(&acc);
                        with_app(move |a| {
                            let g = a.global::<Accounts>();
                            g.set_ms_name(acc.name.into());
                            g.set_ms_state(1);
                            refresh_accounts();
                        });
                    } else {
                        let e = r.error.unwrap_or_else(|| "Authentification refusée".into());
                        with_app(move |a| { let g = a.global::<Accounts>(); g.set_ms_error(e.into()); g.set_ms_state(2); });
                    }
                    return;
                }
                Err(e) => {
                    with_app(move |a| { let g = a.global::<Accounts>(); g.set_ms_error(e.into()); g.set_ms_state(2); });
                    return;
                }
            }
        }
    });
}

fn wire_accounts(app: &AppWindow) {
    let g = app.global::<Accounts>();

    g.on_sync_steam(|| {
        with_app(|a| a.global::<Accounts>().set_steam_syncing(true));
        reload();
        ctx().rt.spawn(async {
            // Laisse le temps au rechargement de la bibliothèque
            tokio::time::sleep(Duration::from_millis(1500)).await;
            while ctx().shared.lock().unwrap().loading { tokio::time::sleep(Duration::from_millis(300)).await; }
            let n = ctx().shared.lock().unwrap().games.iter().filter(|g| g.platform == library::Platform::Steam).count();
            with_app(|a| a.global::<Accounts>().set_steam_syncing(false));
            refresh_accounts();
            toast(format!("{n} jeux Steam réels synchronisés avec succès !"), 1);
        });
    });

    g.on_bnet_scan(|| {
        with_app(|a| a.global::<Accounts>().set_bnet_scanning(true));
        reload();
        ctx().rt.spawn(async {
            tokio::time::sleep(Duration::from_millis(1200)).await;
            while ctx().shared.lock().unwrap().loading { tokio::time::sleep(Duration::from_millis(300)).await; }
            with_app(|a| a.global::<Accounts>().set_bnet_scanning(false));
            refresh_accounts();
            toast("Scan Battle.net terminé", 1);
        });
    });

    let w = app.as_weak();
    g.on_bnet_action(move || {
        let Some(a) = w.upgrade() else { return };
        let g = a.global::<Accounts>();
        if g.get_bnet_connected() {
            match crate::battlenet::auth::disconnect_blizzard_account() {
                Ok(_) => { toast("Compte Blizzard déconnecté", 0); refresh_accounts(); reload(); }
                Err(e) => toast(e, 2),
            }
        } else if !g.get_blizz_configured() {
            g.set_blizz_open(true);
        } else {
            run_blizzard_oauth(false);
        }
    });
    let w = app.as_weak();
    g.on_bnet_open_api(move || { if let Some(a) = w.upgrade() { a.global::<Accounts>().set_blizz_open(true); } });
    let w = app.as_weak();
    g.on_blizz_save(move || {
        let Some(a) = w.upgrade() else { return };
        let g = a.global::<Accounts>();
        match crate::battlenet::auth::save_blizzard_credentials(g.get_blizz_id().trim(), g.get_blizz_secret().trim()) {
            Ok(_) => { g.set_blizz_configured(true); run_blizzard_oauth(true); }
            Err(e) => g.set_blizz_error(e.into()),
        }
    });
    g.on_blizz_connect(|| run_blizzard_oauth(true));
    g.on_open_dev_portal(|| { let _ = crate::open_url_in_default_browser("https://develop.battle.net/access/clients"); });

    g.on_external_submit(|path| {
        with_app(|a| a.global::<Accounts>().set_external_busy(true));
        let path = path.to_string();
        ctx().rt.spawn(async move {
            let r = tokio::task::spawn_blocking(move || accounts::add_external_bnet_game(&path)).await.map_err(|e| e.to_string()).and_then(|r| r);
            with_app(move |a| {
                let g = a.global::<Accounts>();
                g.set_external_busy(false);
                match r {
                    Ok(name) => { g.set_external_open(false); g.set_external_path("".into()); toast(format!("{name} ajouté"), 1); reload(); refresh_accounts(); }
                    Err(e) => toast(format!("Dossier invalide : {e}"), 2),
                }
            });
        });
    });

    let w = app.as_weak();
    g.on_ms_action(move || {
        let Some(a) = w.upgrade() else { return };
        if a.global::<Accounts>().get_ms_connected() {
            match mc_auth::remove_stored_account() {
                Ok(_) => { toast("Compte Microsoft déconnecté", 0); refresh_accounts(); }
                Err(e) => toast(e, 2),
            }
        } else {
            start_microsoft_login();
        }
    });
    let w = app.as_weak();
    g.on_ms_open_link(move || {
        if let Some(a) = w.upgrade() { let _ = crate::open_url_in_default_browser(&a.global::<Accounts>().get_ms_uri()); }
    });
    let w = app.as_weak();
    g.on_ms_copy_code(move || {
        let Some(a) = w.upgrade() else { return };
        if accounts::copy_to_clipboard(&a.global::<Accounts>().get_ms_code()) { toast("Code copié", 1) } else { toast("Aucun utilitaire de presse-papiers (wl-copy / xclip)", 2) }
    });
    g.on_not_available(|name| toast(format!("L'intégration {name} n'est pas encore disponible dans Loom"), 0));
    let w = app.as_weak();
    g.on_open_library_bnet(move || {
        if let Some(a) = w.upgrade() { a.set_page(0); a.set_filter(5); }
    });
}

// ---------------------------------------------------------------- init

pub fn wire(app: &AppWindow) {
    let _ = APP.set(Mutex::new(app.as_weak()));
    let s = settings::load();
    push_settings(app, &s);
    apply_accent(app, s.accent);
    wire_settings(app);
    wire_accounts(app);
    refresh_accounts();

    // Démarrage minimisé (réglage ou --minimized)
    if s.start_minimized || std::env::args().any(|a| a == "--minimized") {
        app.window().set_minimized(true);
    }
    // Instances de plateformes au démarrage
    for (i, key) in ["steam", "battlenet", "epic", "gog"].iter().enumerate() {
        if s.inst[i] {
            let (key, silent) = (key.to_string(), s.inst_silent);
            ctx().rt.spawn(async move {
                let _ = tokio::task::spawn_blocking(move || settings::autostart_platform(&key, silent)).await;
            });
        }
    }
}

#[allow(dead_code)]
fn _unused(_: Image) {}
