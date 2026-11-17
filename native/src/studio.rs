//! Modding Studio : Minecraft, World of Warcraft, Bethesda (câblage des globals Slint).

use crate::minecraft::{self, instance, launcher::{LaunchEvents, MinecraftExitEvent, MinecraftLogEvent}, mods as mc_mods};
use crate::modding::{self, GameAdapter};
use crate::pages::with_app;
use crate::{battlenet::wow, db, images, toast, ctx, AppWindow};
use crate::{Beth, BethConflict, BethConflictMod, BethMod, BethPlugin, McCat, McInst, McLog, McMod, Minecraft, Modding, Wow, WowAddon, WowCat, WowChar, WowInst};
use slint::{Color, ComponentHandle, ModelRc, SharedString, VecModel};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

fn ss(s: impl Into<String>) -> SharedString { SharedString::from(s.into()) }
fn model<T: Clone + 'static>(v: Vec<T>) -> ModelRc<T> { ModelRc::new(VecModel::from(v)) }
fn now_secs() -> i64 { chrono::Utc::now().timestamp() }
fn fmt_duration(secs: u64) -> String {
    if secs == 0 { return "0 min".into(); }
    let (h, m) = (secs / 3600, (secs % 3600) / 60);
    if h > 0 { format!("{h}h {m:02}m") } else { format!("{m} min") }
}
fn fmt_ago(ts: Option<i64>) -> String {
    let Some(mut t) = ts.filter(|t| *t > 0) else { return "Jamais jouée".into() };
    if t > 100_000_000_000 { t /= 1000; }
    let d = now_secs() - t;
    if d < 120 { "À l'instant".into() } else if d < 3600 { format!("Il y a {} min", d / 60) }
    else if d < 86400 { format!("Il y a {} h", d / 3600) } else { format!("Il y a {} j", d / 86400) }
}
fn compact(n: u64) -> String { if n >= 1_000_000 { format!("{:.1}M", n as f64 / 1e6) } else if n >= 1000 { format!("{}k", n / 1000) } else { n.to_string() } }
fn hue_for(s: &str) -> Color {
    let h = s.bytes().fold(7u32, |a, b| a.wrapping_mul(31).wrapping_add(b as u32));
    let pal = [0x14532d, 0x1e3a5f, 0x4c1d95, 0x78350f, 0x7f1d1d, 0x134e4a];
    let c = pal[h as usize % pal.len()];
    Color::from_rgb_u8((c >> 16) as u8, (c >> 8) as u8, c as u8)
}
fn load_img(url: &Option<String>) -> (slint::Image, bool) {
    match crate::load_image_pub(url) { Some(i) => (i, true), None => (slint::Image::default(), false) }
}
fn download(urls: Vec<String>) {
    for u in urls {
        if images::cached(&u).is_some() { continue; }
        ctx().rt.spawn(async move { if images::fetch(&ctx().client, &u).await.is_some() { crate::request_refresh(); } });
    }
}

// =============================================================== Minecraft

#[derive(Clone)]
struct McLogLine { time: String, level: String, source: String, msg: String }

#[derive(Default)]
struct McState {
    instances: Vec<crate::db::MinecraftInstanceRecord>,
    running: HashMap<String, Instant>,
    launching: Option<String>,
    logs: HashMap<String, VecDeque<McLogLine>>,
    selected: Option<String>,
    catalog: Vec<crate::modding::UnifiedModSearchResult>,
    versions: Vec<String>,
    installing: HashSet<String>,
}
static MC: OnceLock<Mutex<McState>> = OnceLock::new();
fn mc() -> std::sync::MutexGuard<'static, McState> { MC.get().unwrap().lock().unwrap() }

#[derive(Clone)]
struct NativeEvents;
impl LaunchEvents for NativeEvents {
    fn emit_log(&self, e: MinecraftLogEvent) {
        {
            let mut s = mc();
            if s.launching.as_deref() == Some(e.instance_id.as_str()) { s.launching = None; }
            s.running.entry(e.instance_id.clone()).or_insert_with(Instant::now);
            let q = s.logs.entry(e.instance_id.clone()).or_default();
            q.push_back(McLogLine { time: e.timestamp, level: e.level, source: e.source, msg: e.message });
            while q.len() > 500 { q.pop_front(); }
        }
        mc_refresh_soon();
    }
    fn emit_exit(&self, e: MinecraftExitEvent) {
        let elapsed = mc().running.remove(&e.instance_id).map(|t| t.elapsed().as_secs()).unwrap_or(0);
        if let Ok(conn) = db::open_db() {
            if let Ok(Some(mut inst)) = db::get_minecraft_instance(&conn, &e.instance_id) {
                inst.playtime_seconds = Some(inst.playtime_seconds.unwrap_or(0) + elapsed);
                inst.last_played = Some(now_secs() as u64);
                let _ = db::insert_minecraft_instance(&conn, &inst);
            }
        }
        toast(format!("Minecraft fermé (code {})", e.exit_code.map(|c| c.to_string()).unwrap_or_else(|| "?".into())), 0);
        mc_reload();
    }
}

fn loader_variant(l: &str) -> i32 { match l { "fabric" => 2, "neoforge" => 1, "forge" => 3, "quilt" => 5, _ => 0 } }

fn mc_inst(i: &db::MinecraftInstanceRecord, s: &McState) -> McInst {
    let (banner, has) = load_img(&i.banner_url.clone().or(i.cover_url.clone()));
    let mods = mc_mods::get_installed_mods(&i.id).map(|m| m.len()).unwrap_or(0);
    McInst {
        id: ss(i.id.clone()), name: ss(i.name.clone()), loader: ss(i.modloader.to_uppercase()), loader_variant: loader_variant(&i.modloader),
        version: ss(i.mc_version.clone()), mods: mods as i32, playtime: ss(fmt_duration(i.playtime_seconds.unwrap_or(0))),
        last_played: ss(fmt_ago(i.last_played.map(|v| v as i64))),
        state: if s.running.contains_key(&i.id) { 1 } else { 0 }, banner, has_banner: has, hue: hue_for(&i.id),
    }
}

fn mc_apply(a: &AppWindow) {
    let g = a.global::<Minecraft>();
    let s = mc();
    let q = g.get_inst_search().to_lowercase();
    g.set_instances(model(s.instances.iter().filter(|i| q.is_empty() || i.name.to_lowercase().contains(&q)).map(|i| mc_inst(i, &s)).collect()));
    g.set_versions(model(s.versions.iter().map(|v| ss(v.clone())).collect()));
    g.set_launching(s.launching.is_some());
    let m = a.global::<Modding>();
    m.set_mc_instances(s.instances.len() as i32);
    let total: u64 = s.instances.iter().map(|i| i.playtime_seconds.unwrap_or(0)).sum();
    m.set_mc_playtime(ss(fmt_duration(total)));
    let active = s.instances.iter().max_by_key(|i| i.last_played.unwrap_or(0));
    m.set_mc_active_name(ss(active.map(|i| i.name.clone()).unwrap_or_default()));
    if let Some(sel) = s.selected.as_ref().and_then(|id| s.instances.iter().find(|i| &i.id == id)) {
        g.set_sel(mc_inst(sel, &s));
        g.set_sel_path(ss(instance::get_instance_dir(&sel.id).to_string_lossy().to_string()));
        let lines = s.logs.get(&sel.id).map(|q| q.iter().map(|l| McLog { time: ss(l.time.clone()), level: ss(l.level.clone()), source: ss(l.source.clone()), msg: ss(l.msg.clone()) }).collect::<Vec<_>>()).unwrap_or_default();
        g.set_logs(model(lines));
        let inst_mods = mc_mods::get_installed_mods(&sel.id).unwrap_or_default();
        let names: Vec<String> = inst_mods.iter().map(|m| m.name.to_lowercase()).collect();
        g.set_mods(model(inst_mods.iter().map(|m| McMod { id: ss(m.id.clone()), name: ss(m.name.clone()), file: ss(m.filename.clone()), provider: ss(m.provider.to_uppercase()), enabled: m.is_enabled }).collect()));
        g.set_catalog(model(s.catalog.iter().map(|c| {
            let (icon, has) = load_img(&c.icon_url);
            McCat { id: ss(c.id.clone()), name: ss(c.name.clone()), summary: ss(c.summary.clone()), author: ss(c.author.clone()), downloads: ss(compact(c.downloads)),
                provider: ss(format!("{:?}", c.provider).to_uppercase()), icon, has_icon: has,
                installed: names.contains(&c.name.to_lowercase()), installing: s.installing.contains(&c.id) }
        }).collect()));
    }
}

fn mc_refresh_soon() { crate::request_refresh(); }

pub fn mc_reload() {
    ctx().rt.spawn(async {
        let list = tokio::task::spawn_blocking(|| instance::get_instances().unwrap_or_default()).await.unwrap_or_default();
        download(list.iter().flat_map(|i| [i.banner_url.clone(), i.cover_url.clone()]).flatten().collect());
        mc().instances = list;
        mc_refresh_soon();
    });
}

fn mc_account() {
    ctx().rt.spawn(async {
        let acc = crate::accounts::microsoft().await;
        if let Some((_, uuid)) = &acc { let _ = images::fetch(&ctx().client, &format!("https://mc-heads.net/avatar/{uuid}/64")).await; }
        with_app(move |a| {
            let g = a.global::<Minecraft>();
            g.set_account_connected(acc.is_some());
            if let Some((n, uuid)) = &acc {
                g.set_account_name(ss(n.clone()));
                if let Some(img) = crate::load_image_pub(&Some(format!("https://mc-heads.net/avatar/{uuid}/64"))) { g.set_account_avatar(img); g.set_has_avatar(true); }
            }
            a.global::<Modding>().set_mc_account(ss(acc.map(|(n, _)| n).unwrap_or_else(|| "Aucun compte lié (mode hors-ligne)".into())));
        });
    });
}

fn mc_versions() {
    ctx().rt.spawn(async {
        if let Ok(v) = minecraft::manifest::fetch_version_manifest(&ctx().client).await {
            let list: Vec<String> = v["versions"].as_array().map(|a| a.iter().filter(|x| x["type"] == "release").filter_map(|x| x["id"].as_str().map(String::from)).take(80).collect()).unwrap_or_default();
            mc().versions = list;
            mc_refresh_soon();
        }
    });
}

fn mc_launch(id: String) {
    if mc().running.contains_key(&id) { return; }
    mc().launching = Some(id.clone());
    mc_refresh_soon();
    toast("Préparation de l'instance (téléchargement Java / bibliothèques si nécessaire)…", 0);
    ctx().rt.spawn(async move {
        if let Err(e) = minecraft::launcher::launch_instance(NativeEvents, id, None).await {
            mc().launching = None;
            toast(format!("Échec du lancement : {e}"), 2);
            mc_refresh_soon();
        }
    });
}

fn mc_stop(id: String) {
    // Termine la JVM dont la ligne de commande référence le dossier de l'instance
    let needle = instance::get_instance_dir(&id).to_string_lossy().to_string();
    let mut killed = 0;
    if let Ok(rd) = std::fs::read_dir("/proc") {
        for e in rd.flatten() {
            let Ok(pid) = e.file_name().to_string_lossy().parse::<i32>() else { continue };
            if let Ok(c) = std::fs::read(e.path().join("cmdline")) {
                let s = String::from_utf8_lossy(&c);
                if s.contains(&needle) && s.contains("java") { unsafe { libc::kill(pid, libc::SIGTERM); } killed += 1; }
            }
        }
    }
    toast(if killed > 0 { "Arrêt de Minecraft demandé" } else { "Aucun processus Minecraft trouvé" }, 0);
}

fn wire_mc(app: &AppWindow) {
    let g = app.global::<Minecraft>();
    let w = app.as_weak();
    g.on_open_instance(move |id| {
        mc().selected = Some(id.to_string());
        mc().catalog.clear();
        if let Some(a) = w.upgrade() {
            a.global::<Modding>().set_screen(2);
            let g = a.global::<Minecraft>(); g.set_tab(0);
            let s = mc();
            if let Some(i) = s.instances.iter().find(|i| i.id == id.as_str()) {
                g.set_set_min(ss(i.min_ram_mb.to_string())); g.set_set_max(ss(i.max_ram_mb.to_string()));
                g.set_set_java(ss(i.java_path.clone().unwrap_or_default()));
                let args: Vec<String> = serde_json::from_str(&i.jvm_args).unwrap_or_default();
                g.set_set_jvm(ss(args.join(" ")));
            }
        }
        mc_refresh_soon();
    });
    let w = app.as_weak();
    g.on_back_hub(move || { if let Some(a) = w.upgrade() { a.global::<Modding>().set_screen(1); } });
    g.on_launch(|id| mc_launch(id.to_string()));
    g.on_stop(|id| mc_stop(id.to_string()));
    g.on_login_ms(|| { crate::pages::start_microsoft_login_pub(); });
    g.on_clear_logs(|| { let sel = mc().selected.clone(); if let Some(id) = sel { mc().logs.remove(&id); } mc_refresh_soon(); });
    g.on_open_folder(|| {
        let sel = mc().selected.clone();
        if let Some(id) = sel { let _ = open::that_detached(instance::get_instance_dir(&id)); }
    });
    let w = app.as_weak();
    g.on_delete_instance(move |id| {
        match instance::delete_instance(&id) {
            Ok(_) => { toast("Instance supprimée", 1); if let Some(a) = w.upgrade() { a.global::<Modding>().set_screen(1); } mc_reload(); }
            Err(e) => toast(e, 2),
        }
    });
    let w = app.as_weak();
    g.on_create_submit(move || {
        let Some(a) = w.upgrade() else { return };
        let g = a.global::<Minecraft>();
        let name = g.get_create_name().to_string();
        let versions = mc().versions.clone();
        let version = versions.get(g.get_create_version() as usize).cloned().unwrap_or_else(|| "1.20.1".into());
        let loader = ["fabric", "neoforge", "forge", "quilt", "vanilla"][g.get_create_loader().clamp(0, 4) as usize].to_string();
        let ram: u32 = g.get_create_ram().parse().unwrap_or(6);
        g.set_creating(true);
        ctx().rt.spawn(async move {
            let r = tokio::task::spawn_blocking(move || instance::create_instance(instance::CreateInstanceParams {
                name, mc_version: version, modloader: loader, loader_version: None,
                min_ram_mb: Some(1024), max_ram_mb: Some(ram * 1024), java_path: None, jvm_args: None,
            })).await.map_err(|e| e.to_string()).and_then(|r| r);
            with_app(move |a| {
                let g = a.global::<Minecraft>();
                g.set_creating(false);
                match r { Ok(i) => { g.set_create_open(false); g.set_create_name("".into()); toast(format!("Instance « {} » créée", i.name), 1); mc_reload(); } Err(e) => toast(e, 2) }
            });
        });
    });
    g.on_toggle_mod(|id| {
        let sel = mc().selected.clone();
        if let Some(inst) = sel {
            let cur = mc_mods::get_installed_mods(&inst).unwrap_or_default().into_iter().find(|m| m.id == id.as_str()).map(|m| m.is_enabled).unwrap_or(true);
            if let Err(e) = mc_mods::toggle_mod(&inst, &id, !cur) { toast(e, 2); }
            mc_refresh_soon();
        }
    });
    g.on_delete_mod(|id| {
        let sel = mc().selected.clone();
        if let Some(inst) = sel { match mc_mods::delete_mod(&inst, &id) { Ok(_) => toast("Mod supprimé", 1), Err(e) => toast(e, 2) } mc_refresh_soon(); }
    });
    let w = app.as_weak();
    g.on_search(move || {
        let Some(a) = w.upgrade() else { return };
        let g = a.global::<Minecraft>();
        let q = g.get_cat_query().to_string();
        let prov = ["all", "modrinth", "curseforge"][g.get_cat_provider().clamp(0, 2) as usize];
        let Some(sel) = mc().selected.clone() else { return };
        let (ver, loader) = { let s = mc(); s.instances.iter().find(|i| i.id == sel).map(|i| (i.mc_version.clone(), i.modloader.clone())).unwrap_or_default() };
        g.set_cat_loading(true);
        ctx().rt.spawn(async move {
            let loader_opt = if loader == "vanilla" { None } else { Some(loader.as_str()) };
            let r = mc_mods::search_mods(&ctx().client, &q, Some(&ver), loader_opt, Some(prov)).await;
            let list = r.unwrap_or_default();
            download(list.iter().filter_map(|c| c.icon_url.clone()).collect());
            mc().catalog = list;
            with_app(|a| a.global::<Minecraft>().set_cat_loading(false));
            mc_refresh_soon();
        });
    });
    g.on_install(|idx| {
        let Some(inst) = mc().selected.clone() else { return };
        let Some(m) = mc().catalog.get(idx as usize).cloned() else { return };
        mc().installing.insert(m.id.clone());
        mc_refresh_soon();
        ctx().rt.spawn(async move {
            let prov = format!("{:?}", m.provider).to_lowercase();
            let r = mc_mods::install_mod(&ctx().client, &inst, &prov, &m.id, &m.latest_version.clone().unwrap_or_else(|| m.id.clone()), None, "", Some(&m.name)).await;
            mc().installing.remove(&m.id);
            match r { Ok(_) => toast(format!("{} installé", m.name), 1), Err(e) => toast(format!("Échec : {e}"), 2) }
            mc_refresh_soon();
        });
    });
    let w = app.as_weak();
    g.on_save_settings(move || {
        let Some(a) = w.upgrade() else { return };
        let g = a.global::<Minecraft>();
        let Some(id) = mc().selected.clone() else { return };
        let (min, max) = (g.get_set_min().parse().unwrap_or(1024), g.get_set_max().parse().unwrap_or(4096));
        let java = g.get_set_java().trim().to_string();
        let args: Vec<String> = g.get_set_jvm().split_whitespace().map(String::from).collect();
        match instance::update_instance_settings(&id, min, max, args, if java.is_empty() { None } else { Some(java) }) {
            Ok(_) => { toast("Réglages de l'instance enregistrés", 1); mc_reload(); }
            Err(e) => toast(e, 2),
        }
    });
    g.on_load_tab(|_| mc_refresh_soon());
}

// =============================================================== WoW

#[derive(Default)]
struct WowState {
    instances: Vec<wow::WowInstanceRecord>,
    addons: Vec<wow::InstalledWowAddonRecord>,
    catalog: Vec<serde_json::Value>,
    installing: HashSet<i64>,
    chars: Vec<wow::WowCharacterRecord>,
}
static WOW: OnceLock<Mutex<WowState>> = OnceLock::new();
fn ws() -> std::sync::MutexGuard<'static, WowState> { WOW.get().unwrap().lock().unwrap() }

fn wow_apply(a: &AppWindow) {
    let g = a.global::<Wow>();
    let s = ws();
    g.set_instances(model(s.instances.iter().map(|i| WowInst { id: ss(i.id.clone()), name: ss(i.name.clone()), flavor: ss(i.flavor.clone()), version: ss(i.version.clone()), addons: i.addons_count as i32, installed: i.installed }).collect()));
    let q = g.get_addon_filter().to_lowercase();
    let list: Vec<&wow::InstalledWowAddonRecord> = s.addons.iter().filter(|x| q.is_empty() || x.title.to_lowercase().contains(&q) || x.folder_name.to_lowercase().contains(&q)).collect();
    g.set_addons(model(list.iter().map(|x| WowAddon {
        id: ss(x.folder_name.clone()), folder: ss(x.folder_name.clone()), title: ss(x.title.clone()), version: ss(x.version.clone()),
        author: ss(x.author.clone().unwrap_or_default()), notes: ss(x.notes.clone().unwrap_or_default()), enabled: x.enabled,
        update: x.update_available.unwrap_or(false), latest: ss(x.latest_version.clone().unwrap_or_default()), size: ss(x.size_formatted.clone()),
    }).collect()));
    g.set_updates(s.addons.iter().filter(|x| x.update_available.unwrap_or(false)).count() as i32);
    g.set_catalog(model(s.catalog.iter().map(|m| {
        let id = m["id"].as_i64().unwrap_or(0);
        let slug = m["slug"].as_str().unwrap_or("").to_lowercase();
        let icon_url = m["logo"]["thumbnailUrl"].as_str().or(m["logo"]["url"].as_str()).map(String::from);
        let (icon, has) = load_img(&icon_url);
        WowCat { name: ss(m["name"].as_str().unwrap_or("")), summary: ss(m["summary"].as_str().unwrap_or("")),
            author: ss(m["authors"][0]["name"].as_str().unwrap_or("Communauté")), downloads: ss(compact(m["downloadCount"].as_f64().unwrap_or(0.0) as u64)),
            icon, has_icon: has, installed: s.addons.iter().any(|x| x.curseforge_id == Some(id) || x.folder_name.to_lowercase() == slug), installing: s.installing.contains(&id) }
    }).collect()));
    g.set_chars(model(s.chars.iter().map(|c| WowChar { name: ss(c.name.clone()), realm: ss(c.realm.clone()), level: ss(c.level.map(|l| l.to_string()).unwrap_or_default()), cls: ss(c.class.clone().unwrap_or_default()), faction: ss(c.faction.clone().unwrap_or_default()), race: ss(c.race.clone().unwrap_or_default()) }).collect()));
    let m = a.global::<Modding>();
    m.set_wow_instances(s.instances.iter().filter(|i| i.installed).count() as i32);
    m.set_wow_addons(s.instances.iter().map(|i| i.addons_count as i32).sum());
}

fn wow_sel() -> Option<wow::WowInstanceRecord> {
    let idx = APP_WOW_SEL.lock().unwrap().clone();
    ws().instances.get(idx).cloned()
}
static APP_WOW_SEL: Mutex<usize> = Mutex::new(0);

fn wow_load_instances() {
    ctx().rt.spawn(async {
        let list = tokio::task::spawn_blocking(wow::scan_wow_instances_impl).await.unwrap_or_default();
        ws().instances = list;
        crate::request_refresh();
        wow_load_addons(false);
    });
}

fn wow_load_addons(check_updates: bool) {
    let Some(inst) = wow_sel() else { return };
    with_app(|a| a.global::<Wow>().set_loading(true));
    ctx().rt.spawn(async move {
        let r = wow::get_wow_addons_impl(inst.path.clone(), check_updates, Some(inst.game_version_type_id)).await.unwrap_or_default();
        download(r.iter().filter_map(|a| a.icon_url.clone()).collect());
        let chars = wow::get_wow_characters_impl(&inst.path);
        { let mut s = ws(); s.addons = r; s.chars = chars; }
        with_app(|a| a.global::<Wow>().set_loading(false));
        crate::request_refresh();
    });
}

fn cdn_url(id: i64, name: &str) -> String { format!("https://edge.forgecdn.net/files/{}/{}/{}", id / 1000, id % 1000, name) }

fn wire_wow(app: &AppWindow) {
    let g = app.global::<Wow>();
    g.on_select_instance(|i| { *APP_WOW_SEL.lock().unwrap() = i as usize; with_app(move |a| a.global::<Wow>().set_sel(i)); wow_load_addons(false); });
    g.on_refresh(|| wow_load_addons(true));
    g.on_filter_changed(|| crate::request_refresh());
    g.on_open_folder(|| { if let Some(i) = wow_sel() { if let Err(e) = wow::open_wow_addons_folder_impl(i.path) { toast(e, 2) } } });
    g.on_export(|| {
        let Some(i) = wow_sel() else { return };
        with_app(|a| a.global::<Wow>().set_exporting(true));
        ctx().rt.spawn(async move {
            let r = tokio::task::spawn_blocking(move || wow::export_wow_addons_impl(i.path, Some(i.flavor))).await.map_err(|e| e.to_string()).and_then(|r| r);
            with_app(move |a| { a.global::<Wow>().set_exporting(false); match r { Ok(p) => toast(format!("Export créé : {p}"), 1), Err(e) => toast(e, 2) } });
        });
    });
    g.on_toggle(|folder| {
        let Some(i) = wow_sel() else { return };
        let (sec, enabled) = { let s = ws(); s.addons.iter().find(|a| a.folder_name == folder.as_str()).map(|a| (a.secondary_folders.clone(), a.enabled)).unwrap_or_default() };
        match wow::toggle_wow_addon_impl(i.path, folder.to_string(), sec, !enabled) { Ok(_) => wow_load_addons(false), Err(e) => toast(e, 2) }
    });
    g.on_uninstall(|folder| {
        let Some(i) = wow_sel() else { return };
        let sec = { ws().addons.iter().find(|a| a.folder_name == folder.as_str()).map(|a| a.secondary_folders.clone()).unwrap_or_default() };
        match wow::uninstall_wow_addon_impl(i.path, folder.to_string(), sec) { Ok(_) => { toast("Addon désinstallé", 1); wow_load_addons(false); } Err(e) => toast(e, 2) }
    });
    g.on_update_addon(|folder| {
        let Some(i) = wow_sel() else { return };
        let Some(a) = ws().addons.iter().find(|a| a.folder_name == folder.as_str()).cloned() else { return };
        let (Some(url), Some(mid)) = (a.download_url.clone(), a.curseforge_id) else { toast("Lien de mise à jour indisponible", 2); return };
        ctx().rt.spawn(async move {
            match wow::install_wow_addon_impl(i.path, url, Some(mid), a.latest_file_id, Some(a.title.clone()), a.latest_version.clone(), a.icon_url.clone()).await {
                Ok(_) => { toast(format!("{} mis à jour", a.title), 1); wow_load_addons(false); }
                Err(e) => toast(e, 2),
            }
        });
    });
    let w = app.as_weak();
    g.on_search(move || {
        let Some(a) = w.upgrade() else { return };
        let g = a.global::<Wow>();
        let q = g.get_cat_query().to_string();
        let sort = [2u32, 3, 6][g.get_cat_sort().clamp(0, 2) as usize];
        let gv = wow_sel().map(|i| i.game_version_type_id);
        g.set_cat_loading(true);
        ctx().rt.spawn(async move {
            let r = wow::search_wow_addons_impl(if q.is_empty() { None } else { Some(q) }, None, gv, Some(sort), Some(36)).await;
            match r {
                Ok(v) => {
                    let list = v["data"].as_array().cloned().unwrap_or_default();
                    download(list.iter().filter_map(|m| m["logo"]["thumbnailUrl"].as_str().or(m["logo"]["url"].as_str()).map(String::from)).collect());
                    ws().catalog = list;
                }
                Err(e) => toast(e, 2),
            }
            with_app(|a| a.global::<Wow>().set_cat_loading(false));
            crate::request_refresh();
        });
    });
    g.on_install(|idx| {
        let Some(i) = wow_sel() else { return };
        let Some(m) = ws().catalog.get(idx as usize).cloned() else { return };
        let mid = m["id"].as_i64().unwrap_or(0);
        let name = m["name"].as_str().unwrap_or("").to_string();
        let icon = m["logo"]["thumbnailUrl"].as_str().map(String::from);
        ws().installing.insert(mid);
        crate::request_refresh();
        ctx().rt.spawn(async move {
            let files = wow::get_wow_addon_files_impl(mid, Some(15)).await.ok().and_then(|v| v["data"].as_array().cloned()).unwrap_or_default();
            let best = files.iter().find(|f| f["sortableGameVersions"].as_array().map(|g| g.iter().any(|x| x["gameVersionTypeId"].as_i64() == Some(i.game_version_type_id))).unwrap_or(false)).or(files.first()).cloned();
            let r = match best {
                Some(f) => {
                    let fid = f["id"].as_i64().unwrap_or(0);
                    let fname = f["fileName"].as_str().unwrap_or("addon.zip");
                    let url = f["downloadUrl"].as_str().map(String::from).unwrap_or_else(|| cdn_url(fid, fname));
                    wow::install_wow_addon_impl(i.path.clone(), url, Some(mid), Some(fid), Some(name.clone()), f["displayName"].as_str().map(String::from), icon).await.map(|_| ())
                }
                None => Err("Lien de téléchargement de l'addon introuvable".into()),
            };
            ws().installing.remove(&mid);
            match r { Ok(_) => { toast(format!("{name} installé"), 1); wow_load_addons(false); } Err(e) => toast(e, 2) }
            crate::request_refresh();
        });
    });
}

// =============================================================== Bethesda

const GAME_IDS: [&str; 3] = ["skyrim_se", "fallout4", "oblivion"];
fn adapter(game: usize) -> modding::BethesdaAdapter {
    match game { 1 => modding::BethesdaAdapter::new_fallout_4(), 2 => modding::BethesdaAdapter::new_oblivion(), _ => modding::BethesdaAdapter::new_skyrim_se() }
}
#[derive(Default)]
struct BethState { game: usize, profiles: Vec<db::GameProfileRecord>, sel: HashMap<usize, String>, profile: Option<db::GameProfileRecord>, mods: Vec<db::ProfileModRecord>, plugins: Vec<db::BethesdaPluginRecord>, conflicts: Vec<db::FileConflict>, nexus: bool }
static BETH: OnceLock<Mutex<BethState>> = OnceLock::new();
fn bs() -> std::sync::MutexGuard<'static, BethState> { BETH.get().unwrap().lock().unwrap() }

pub fn beth_reload() { beth_load() }

fn beth_load() {
    let (game, want) = { let s = bs(); (s.game, s.sel.get(&s.game).cloned()) };
    ctx().rt.spawn(async move {
        let r = tokio::task::spawn_blocking(move || -> Result<_, String> {
            let conn = db::open_db().map_err(|e| e.to_string())?;
            let mut all = db::get_profiles(&conn, Some(GAME_IDS[game])).map_err(|e| e.to_string())?;
            for pr in all.iter_mut() {
                if pr.deployment_path.is_none() {
                    if let Some(d) = adapter(game).auto_detect_game_directory() { pr.deployment_path = Some(d.to_string_lossy().to_string()); let _ = db::insert_profile(&conn, pr); }
                }
            }
            let idx = want.and_then(|w| all.iter().position(|p| p.id == w)).unwrap_or(0);
            let p = all.get(idx).cloned();
            let (mods, plugins) = match &p {
                Some(pr) => (db::get_profile_mods(&conn, &pr.id).unwrap_or_default(), db::get_bethesda_plugins(&conn, &pr.id).unwrap_or_default()),
                None => (vec![], vec![]),
            };
            Ok((all, p, mods, plugins))
        }).await.map_err(|e| e.to_string()).and_then(|r| r);
        match r {
            Ok((all, p, mut mods, plugins)) => { mods.sort_by_key(|m| m.load_priority); let mut s = bs(); s.profiles = all; s.profile = p; s.mods = mods; s.plugins = plugins; s.conflicts.clear(); }
            Err(e) => toast(e, 2),
        }
        crate::request_refresh();
    });
}

fn beth_apply(a: &AppWindow) {
    let g = a.global::<Beth>();
    let s = bs();
    g.set_has_profile(s.profile.is_some());
    g.set_profiles(model(s.profiles.iter().map(|p| ss(p.name.clone())).collect()));
    g.set_profile_idx(s.profile.as_ref().and_then(|p| s.profiles.iter().position(|x| x.id == p.id)).unwrap_or(0) as i32);
    g.set_skse_cmd(ss(match s.game { 1 => "echo \"%command%\" | sed \"s/Fallout4.exe/f4se_loader.exe/\" | sh", 2 => "echo \"%command%\" | sed \"s/Oblivion.exe/obse_loader.exe/\" | sh", _ => "echo \"%command%\" | sed \"s/SkyrimSE.exe/skse64_loader.exe/\" | sh" }));
    g.set_nexus_configured(s.nexus);
    if let Some(p) = &s.profile {
        g.set_profile_name(ss(p.name.clone()));
        let path = p.deployment_path.clone().unwrap_or_default();
        g.set_path_ok(!path.is_empty() && std::path::Path::new(&path).exists());
        if g.get_path_input().is_empty() { g.set_path_input(ss(path.clone())); }
        g.set_path(ss(path));
    }
    g.set_mods(model(s.mods.iter().map(|m| BethMod { id: ss(m.id.clone()), name: ss(m.name.clone()), file: ss(m.filename.clone()), provider: ss(m.provider.to_uppercase()), enabled: m.is_enabled, priority: m.load_priority }).collect()));
    g.set_plugins(model(s.plugins.iter().enumerate().map(|(i, p)| BethPlugin { name: ss(p.filename.clone()), active: p.is_active, idx: i as i32 }).collect()));
    g.set_conflicts(model(s.conflicts.iter().map(|c| BethConflict {
        path: ss(c.relative_path.clone()),
        mods: model(c.conflicting_mods.iter().map(|m| BethConflictMod { id: ss(m.mod_id.clone()), name: ss(m.mod_name.clone()), winner: m.mod_id == c.current_winner_mod_id }).collect()),
    }).collect()));
    a.global::<Modding>().set_beth_profiles(if s.profile.is_some() { 1 } else { 0 });
}

fn settings_path() -> std::path::PathBuf { dirs::data_local_dir().unwrap_or_default().join("loom/settings.json") }

fn wire_beth(app: &AppWindow) {
    let g = app.global::<Beth>();
    g.on_select_game(|i| { bs().game = i as usize; with_app(move |a| { let g = a.global::<Beth>(); g.set_game(i); g.set_path_input("".into()); g.set_tab(0); }); beth_load(); });
    g.on_load_tab(|_| beth_load());
    let w = app.as_weak();
    g.on_create_profile(move || {
        let Some(a) = w.upgrade() else { return };
        let g = a.global::<Beth>();
        let name = g.get_new_name().trim().to_string();
        let path = g.get_new_path().trim().to_string();
        if name.is_empty() { return; }
        let game = bs().game;
        let det = if path.is_empty() { adapter(game).auto_detect_game_directory().map(|p| p.to_string_lossy().to_string()) } else { Some(path) };
        let rec = db::GameProfileRecord { id: uuid::Uuid::new_v4().to_string(), game_id: GAME_IDS[game].into(), name: name.clone(), game_version: "latest".into(), modloader: None, deployment_path: det, created_at: now_secs() };
        let id = rec.id.clone();
        match db::open_db().map_err(|e| e.to_string()).and_then(|c| db::insert_profile(&c, &rec).map_err(|e| e.to_string())) {
            Ok(_) => { bs().sel.insert(game, id); g.set_create_open(false); g.set_new_name("".into()); g.set_new_path("".into()); g.set_path_input("".into()); toast(format!("Profil « {name} » créé"), 1); beth_load(); }
            Err(e) => toast(e, 2),
        }
    });
    let w = app.as_weak();
    g.on_select_profile(move |i| {
        let id = { let s = bs(); s.profiles.get(i as usize).map(|p| p.id.clone()) };
        if let Some(id) = id { let game = bs().game; bs().sel.insert(game, id); if let Some(a) = w.upgrade() { a.global::<Beth>().set_path_input("".into()); } beth_load(); }
    });
    g.on_delete_profile(|| {
        let (game, p) = { let s = bs(); (s.game, s.profile.clone()) };
        let Some(p) = p else { return };
        let r = adapter(game).undeploy_profile(&p.id).and_then(|_| Ok(())).ok();
        let _ = r;
        match db::open_db().map_err(|e| e.to_string()).and_then(|c| db::delete_profile(&c, &p.id).map_err(|e| e.to_string())) {
            Ok(_) => { bs().sel.remove(&game); toast(format!("Profil « {} » supprimé", p.name), 1); beth_load(); }
            Err(e) => toast(e, 2),
        }
    });
    g.on_launch(|| {
        let (game, pid) = { let s = bs(); (s.game, s.profile.as_ref().map(|p| p.id.clone())) };
        let steam_id = ["steam-489830", "steam-377160", "steam-22330"][game.min(2)].to_string();
        with_app(|a| a.global::<Beth>().set_launching(true));
        ctx().rt.spawn(async move {
            // Comme le Vue : on déploie le profil (liens symboliques à jour) puis on lance le jeu Steam
            if let Some(pid) = pid {
                let r = tokio::task::spawn_blocking(move || adapter(game).deploy_profile(&pid)).await.map_err(|e| e.to_string()).and_then(|r| r);
                if let Err(e) = r { toast(format!("Déploiement : {e}"), 2); }
            }
            crate::do_launch(steam_id);
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            with_app(|a| a.global::<Beth>().set_launching(false));
        });
    });
    g.on_copy_skse(|| {
        let cmd = ["echo \"%command%\" | sed \"s/SkyrimSE.exe/skse64_loader.exe/\" | sh", "echo \"%command%\" | sed \"s/Fallout4.exe/f4se_loader.exe/\" | sh", "echo \"%command%\" | sed \"s/Oblivion.exe/obse_loader.exe/\" | sh"][bs().game.min(2)];
        if crate::accounts::copy_to_clipboard(cmd) { toast("Commande copiée", 1) } else { toast("Aucun utilitaire de presse-papiers (wl-copy / xclip)", 2) }
    });
    g.on_mod_toggle(|id| {
        let (pid, cur) = { let s = bs(); (s.profile.as_ref().map(|p| p.id.clone()), s.mods.iter().find(|m| m.id == id.as_str()).map(|m| m.is_enabled)) };
        if let (Some(pid), Some(cur)) = (pid, cur) { if let Ok(c) = db::open_db() { let _ = db::toggle_profile_mod(&c, &pid, &id, !cur); } beth_load(); }
    });
    g.on_mod_delete(|id| {
        // Le verrou doit être relâché avant beth_load() (qui le reprend) : sinon l'app se fige
        let pid = bs().profile.as_ref().map(|p| p.id.clone());
        if let Some(pid) = pid {
            if let Ok(c) = db::open_db() { let _ = db::delete_profile_mod(&c, &pid, &id); }
            beth_load();
        }
    });
    g.on_mod_move(|id, dir| {
        let (pid, mods) = { let s = bs(); (s.profile.as_ref().map(|p| p.id.clone()), s.mods.clone()) };
        let Some(pid) = pid else { return };
        let Some(pos) = mods.iter().position(|m| m.id == id.as_str()) else { return };
        let other = pos as i32 + dir;
        if other < 0 || other as usize >= mods.len() { return; }
        if let Ok(c) = db::open_db() {
            let (a, b) = (&mods[pos], &mods[other as usize]);
            let _ = db::update_mod_priority(&c, &pid, &a.id, b.load_priority);
            let _ = db::update_mod_priority(&c, &pid, &b.id, a.load_priority);
        }
        beth_load();
    });
    g.on_plugin_toggle(|name| {
        let (pid, cur) = { let s = bs(); (s.profile.as_ref().map(|p| p.id.clone()), s.plugins.iter().find(|p| p.filename == name.as_str()).map(|p| p.is_active)) };
        if let (Some(pid), Some(cur)) = (pid, cur) { if let Ok(c) = db::open_db() { let _ = db::toggle_bethesda_plugin(&c, &pid, &name, !cur); } beth_load(); }
    });
    g.on_plugin_move(|idx, dir| {
        let (pid, mut names) = { let s = bs(); (s.profile.as_ref().map(|p| p.id.clone()), s.plugins.iter().map(|p| p.filename.clone()).collect::<Vec<_>>()) };
        let Some(pid) = pid else { return };
        let (i, j) = (idx as usize, idx + dir);
        if j < 0 || j as usize >= names.len() { return; }
        names.swap(i, j as usize);
        if let Ok(c) = db::open_db() { let _ = db::update_plugin_load_order(&c, &pid, &names); }
        beth_load();
    });
    g.on_scan_conflicts(|| {
        let (game, pid) = { let s = bs(); (s.game, s.profile.as_ref().map(|p| p.id.clone())) };
        let Some(pid) = pid else { return };
        ctx().rt.spawn(async move {
            let r = tokio::task::spawn_blocking(move || { let dir = adapter(game).get_staging_directory(&pid); modding::conflicts::scan_and_resolve_conflicts(&pid, &dir) }).await.map_err(|e| e.to_string()).and_then(|r| r);
            match r { Ok(c) => bs().conflicts = c, Err(e) => toast(format!("Scan des conflits : {e}"), 2) }
            crate::request_refresh();
        });
    });
    g.on_set_winner(|ci, mod_id| {
        let (pid, path) = { let s = bs(); (s.profile.as_ref().map(|p| p.id.clone()), s.conflicts.get(ci as usize).map(|c| c.relative_path.clone())) };
        if let (Some(pid), Some(path)) = (pid, path) {
            if let Ok(c) = db::open_db() { let _ = db::set_file_conflict_rule(&c, &db::FileConflictRuleRecord { profile_id: pid, relative_path: path, winning_mod_id: mod_id.to_string() }); }
            { let mut s = bs(); if let Some(c) = s.conflicts.get_mut(ci as usize) { c.current_winner_mod_id = mod_id.to_string(); } }
            crate::request_refresh();
        }
    });
    g.on_deploy(|| {
        let (game, pid) = { let s = bs(); (s.game, s.profile.as_ref().map(|p| p.id.clone())) };
        let Some(pid) = pid else { return };
        with_app(|a| a.global::<Beth>().set_busy(true));
        ctx().rt.spawn(async move {
            let r = tokio::task::spawn_blocking(move || adapter(game).deploy_profile(&pid)).await.map_err(|e| e.to_string()).and_then(|r| r);
            with_app(move |a| { a.global::<Beth>().set_busy(false); match r { Ok(_) => toast("Profil déployé (liens symboliques à jour)", 1), Err(e) => toast(format!("Déploiement : {e}"), 2) } });
            beth_load();
        });
    });
    let w = app.as_weak();
    g.on_pick_path(move || {
        let Some(a) = w.upgrade() else { return };
        let cur = a.global::<Beth>().get_path_input().to_string();
        ctx().rt.spawn(async move {
            if let Some(p) = tokio::task::spawn_blocking(move || crate::settings::pick_directory("Sélectionner le dossier du jeu", &cur)).await.ok().flatten() {
                with_app(move |a| a.global::<Beth>().set_path_input(ss(p)));
            }
        });
    });
    let w = app.as_weak();
    g.on_save_path(move || {
        let Some(a) = w.upgrade() else { return };
        let path = a.global::<Beth>().get_path_input().to_string();
        let cur = bs().profile.clone();
        if let Some(mut p) = cur {
            p.deployment_path = Some(path);
            match db::open_db().map_err(|e| e.to_string()).and_then(|c| db::insert_profile(&c, &p).map_err(|e| e.to_string())) { Ok(_) => { toast("Dossier du jeu enregistré", 1); beth_load(); } Err(e) => toast(e, 2) }
        }
    });
    g.on_register_nxm(|| match crate::nxm::register_handler(true) { Ok(_) => toast("Liens nxm:// associés à Loom natif", 1), Err(e) => toast(e, 2) });
    let w = app.as_weak();
    g.on_save_nexus(move || {
        let Some(a) = w.upgrade() else { return };
        let key = a.global::<Beth>().get_nexus_key().to_string();
        let mut v: serde_json::Value = std::fs::read_to_string(settings_path()).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or(serde_json::json!({}));
        v["nexusApiKey"] = serde_json::Value::String(key.trim().to_string());
        match std::fs::write(settings_path(), serde_json::to_string_pretty(&v).unwrap()) { Ok(_) => { bs().nexus = !key.trim().is_empty(); toast("Clé API Nexus Mods enregistrée", 1); crate::request_refresh(); } Err(e) => toast(e.to_string(), 2) }
    });
}

// =============================================================== init

pub fn apply_all(a: &AppWindow) { mc_apply(a); wow_apply(a); beth_apply(a); }

pub fn wire(app: &AppWindow) {
    let _ = MC.set(Mutex::new(McState::default()));
    let _ = WOW.set(Mutex::new(WowState::default()));
    let _ = BETH.set(Mutex::new(BethState::default()));
    wire_mc(app);
    wire_wow(app);
    wire_beth(app);
    let g = app.global::<Modding>();
    let w = app.as_weak();
    g.on_open(move |s| { if let Some(a) = w.upgrade() { a.global::<Modding>().set_screen(s); } });
    let w = app.as_weak();
    g.on_back(move || { if let Some(a) = w.upgrade() { a.global::<Modding>().set_screen(0); } });
    g.on_launch_active_mc(|| {
        let id = { let s = mc(); s.instances.iter().max_by_key(|i| i.last_played.unwrap_or(0)).map(|i| i.id.clone()) };
        if let Some(id) = id { mc_launch(id); }
    });
    // Chargement initial des données (légères)
    mc_reload(); mc_account(); mc_versions(); wow_load_instances();
    bs().nexus = std::fs::read_to_string(settings_path()).ok().and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok()).and_then(|v| v["nexusApiKey"].as_str().map(|k| !k.trim().is_empty())).unwrap_or(false);
    beth_load();
}
