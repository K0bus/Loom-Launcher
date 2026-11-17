//! Page Monitoring : collecte périodique, historiques 60 points, tris/filtres, actions.

use crate::monitoring::*;
use crate::pages::with_app;
use crate::{toast, AppWindow, ctx};
use crate::{MonCore, MonDisk, MonGpuProc, MonIface, MonPart, MonPkg, MonProc, Monitor};
use slint::{Color, ComponentHandle, ModelRc, SharedString, VecModel};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

const HIST: usize = 60;
const INTERVALS_MS: [u64; 4] = [500, 1000, 2000, 5000];
const PAGE_SIZE: usize = 50;

#[derive(Default)]
struct Hist { cpu: VecDeque<f32>, gpu: VecDeque<f32>, ram: VecDeque<f32>, dl: VecDeque<f32>, ul: VecDeque<f32>, dr: VecDeque<f32>, dw: VecDeque<f32> }

#[derive(Default)]
struct State {
    hist: Hist,
    last: Option<SystemMetricsSnapshot>,
    paused: bool,
    interval_idx: usize,
    inflight: bool,
    last_poll: Option<Instant>,
    initialized: bool,
    // processus
    sort_field: i32, sort_desc: bool,
    all: Vec<ProcessSnapshot>, all_page: usize, all_sort_field: i32, all_sort_desc: bool,
    gpu: Vec<GpuProcessInfo>,
    smart: Vec<SmartDiskInfo>,
    dnf: Option<DnfUpdateStatus>,
    kill_pid: Option<u32>,
}

static ST: OnceLock<Mutex<State>> = OnceLock::new();
static COLLECTOR: OnceLock<SharedCollector> = OnceLock::new();
fn st() -> std::sync::MutexGuard<'static, State> { ST.get().unwrap().lock().unwrap() }
fn collector() -> SharedCollector { COLLECTOR.get().unwrap().clone() }

// ---------------------------------------------------------------- formatage

fn col(hex: u32) -> Color { Color::from_rgb_u8((hex >> 16) as u8, (hex >> 8) as u8, hex as u8) }
fn metric_color(p: f32) -> Color { if p >= 90.0 { col(0xf43f5e) } else if p >= 75.0 { col(0xf59e0b) } else { col(0x10b981) } }
fn ss(s: impl Into<String>) -> SharedString { SharedString::from(s.into()) }

fn bytes(b: u64) -> String {
    let (kb, mb, gb) = (b as f64 / 1024.0, b as f64 / 1048576.0, b as f64 / 1073741824.0);
    if gb >= 1.0 { format!("{gb:.2} Go") } else if mb >= 1.0 { format!("{mb:.1} Mo") } else { format!("{kb:.0} Ko") }
}
fn gb1(b: u64) -> String { format!("{:.1} Go", b as f64 / 1073741824.0) }
fn rate(b: u64) -> String {
    let mb = b as f64 / 1048576.0;
    if mb >= 10.0 { format!("{mb:.1} Mo/s") } else if mb >= 1.0 { format!("{mb:.2} Mo/s") }
    else if b >= 1024 { format!("{:.1} Ko/s", b as f64 / 1024.0) } else { format!("{b} B/s") }
}
fn mem(b: u64) -> String { let mb = b as f64 / 1048576.0; if mb >= 1024.0 { format!("{:.1} Go", mb / 1024.0) } else { format!("{mb:.0} Mo") } }
fn io(r: u64, w: u64) -> String {
    let t = r + w; if t == 0 { return "-".into(); }
    let mb = t as f64 / 1048576.0; if mb >= 1.0 { format!("{mb:.1} Mo/s") } else { format!("{:.0} Ko/s", t as f64 / 1024.0) }
}
fn freq(mhz: u64) -> String { if mhz >= 1000 { format!("{:.2} GHz", mhz as f64 / 1000.0) } else { format!("{mhz} MHz") } }
fn opt<T: std::fmt::Display>(v: Option<T>, f: impl Fn(T) -> String) -> String { v.map(f).unwrap_or_else(|| "--".into()) }

fn push(q: &mut VecDeque<f32>, v: f32) { q.push_back(v); while q.len() > HIST { q.pop_front(); } }

/// Chemins SVG (viewbox 600×52) : (ligne, aire)
fn spark(data: &VecDeque<f32>, min: f32, max: Option<f32>) -> (SharedString, SharedString) {
    let (w, h) = (600.0f32, 52.0f32);
    let n = data.len();
    if n < 2 { return (ss(format!("M 0 {h} L {w} {h}")), ss(format!("M 0 {h} L {w} {h} Z"))); }
    let mx = max.unwrap_or_else(|| data.iter().cloned().fold(0.0, f32::max).max(0.001) * 1.15);
    let pts: Vec<(f32, f32)> = data.iter().enumerate().map(|(i, v)| {
        let x = i as f32 / (n - 1) as f32 * w;
        let y = h - ((v - min) / (mx - min)).clamp(0.0, 1.0) * (h - 4.0) - 2.0;
        (x, y)
    }).collect();
    let mut line = format!("M {:.1} {:.1}", pts[0].0, pts[0].1);
    for p in &pts[1..] { line.push_str(&format!(" L {:.1} {:.1}", p.0, p.1)); }
    let area = format!("{line} L {w} {h} L 0 {h} Z");
    (ss(line), ss(area))
}

fn model<T: Clone + 'static>(v: Vec<T>) -> ModelRc<T> { ModelRc::new(VecModel::from(v)) }

fn proc_row(p: &ProcessSnapshot) -> MonProc {
    MonProc {
        pid: ss(p.pid.to_string()), name: ss(p.name.clone()), command: ss(p.command.clone()),
        user: ss(p.user.clone().unwrap_or_else(|| "root".into())), cpu: ss(format!("{:.1}%", p.cpu_usage)),
        mem: ss(mem(p.memory_bytes)), io: ss(io(p.disk_read_bytes_per_sec, p.disk_write_bytes_per_sec)),
        status: ss(p.status.clone()),
        killable: p.pid > 1 && !p.name.to_lowercase().contains("loom"),
    }
}

fn sort_procs(list: &mut Vec<ProcessSnapshot>, field: i32, desc: bool) {
    list.sort_by(|a, b| {
        let o = match field {
            0 => a.pid.cmp(&b.pid),
            2 => a.memory_bytes.cmp(&b.memory_bytes),
            3 => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            4 => a.user.cmp(&b.user),
            _ => a.cpu_usage.partial_cmp(&b.cpu_usage).unwrap_or(std::cmp::Ordering::Equal),
        };
        if desc { o.reverse() } else { o }
    });
}
fn matches(p: &ProcessSnapshot, q: &str) -> bool {
    q.is_empty() || p.name.to_lowercase().contains(q) || p.command.to_lowercase().contains(q)
        || p.pid.to_string().contains(q) || p.user.as_deref().map(|u| u.to_lowercase().contains(q)).unwrap_or(false)
}

// ---------------------------------------------------------------- application des données

fn apply_overview(a: &AppWindow) {
    let g = a.global::<Monitor>();
    let s = st();
    let Some(m) = s.last.as_ref() else { return };
    g.set_has_metrics(true);
    g.set_loading(false);
    g.set_paused(s.paused);

    // OS
    g.set_os_line(ss(format!("{} {}", m.os.os_name, m.os.os_version)));
    g.set_kernel(ss(m.os.kernel_version.clone()));
    g.set_session(ss(m.os.display_server.clone()));
    g.set_uptime(ss(m.os.uptime_formatted.clone()));
    g.set_host(ss(m.os.hostname.clone()));
    g.set_load1(ss(format!("{:.2}", m.os.load_average_1m)));
    g.set_load5(ss(format!("{:.2}", m.os.load_average_5m)));
    g.set_load15(ss(format!("{:.2}", m.os.load_average_15m)));

    // CPU
    let cu = m.cpu.global_usage;
    g.set_cpu_model(ss(m.cpu.model.clone()));
    g.set_cpu_governor(ss(m.cpu.power_governor.clone().unwrap_or_default()));
    g.set_cpu_usage(ss(format!("{:.0}", cu)));
    g.set_cpu_color(metric_color(cu));
    g.set_cpu_topology(ss(format!("{}C / {}T", m.cpu.physical_cores, m.cpu.logical_cores)));
    g.set_cpu_temp(ss(m.cpu.package_temperature_c.map(|t| format!("{t:.0}°C")).unwrap_or_default()));
    g.set_cpu_cores(model(m.cpu.cores.iter().map(|c| MonCore {
        id: ss(c.id.to_string()), pct: c.usage.clamp(0.0, 100.0) / 100.0, label: ss(format!("{:.0}%", c.usage)),
        freq: ss(freq(c.frequency_mhz)), temp: ss(c.temperature_c.map(|t| format!("{t:.0}°")).unwrap_or_default()), color: metric_color(c.usage),
    }).collect()));
    let (l, ar) = spark(&s.hist.cpu, 0.0, Some(100.0)); g.set_cpu_line(l); g.set_cpu_area(ar);
    match &m.battery {
        Some(b) if b.present => {
            g.set_has_battery(true);
            g.set_battery_label(ss(format!("Batterie ({})", b.state)));
            g.set_battery_power(ss(b.power_watts.map(|w| format!("{w:.1}W")).unwrap_or_default()));
            g.set_battery_pct(ss(format!("{:.0}%", b.percentage)));
        }
        _ => g.set_has_battery(false),
    }

    // GPU
    match &m.gpu {
        Some(gp) => {
            g.set_has_gpu(true);
            let u = gp.usage_percent;
            g.set_gpu_name(ss(gp.name.clone())); g.set_gpu_vendor(ss(gp.vendor.clone()));
            g.set_gpu_usage(ss(u.map(|v| format!("{v:.0}")).unwrap_or_else(|| "--".into())));
            g.set_gpu_color(metric_color(u.unwrap_or(0.0)));
            g.set_gpu_driver(ss(gp.driver_version.clone().unwrap_or_default()));
            g.set_gpu_temp(ss(gp.core_temp_c.map(|t| format!("{t:.0}°C")).unwrap_or_default()));
            let (l, ar) = spark(&s.hist.gpu, 0.0, Some(100.0)); g.set_gpu_line(l); g.set_gpu_area(ar);
            match (gp.memory_used_mb, gp.memory_total_mb) {
                (Some(u), Some(t)) if t > 0 => {
                    g.set_gpu_has_vram(true);
                    g.set_gpu_vram_label(ss(format!("{:.1} / {:.1} Go ({:.0}%)", u as f64 / 1024.0, t as f64 / 1024.0, u as f64 / t as f64 * 100.0)));
                    g.set_gpu_vram_pct(u as f32 / t as f32);
                }
                _ => g.set_gpu_has_vram(false),
            }
            g.set_gpu_power(ss(opt(gp.power_watts, |w| format!("{w:.0} W"))));
            g.set_gpu_fan(ss(gp.fan_speed_percent.map(|p| format!("{p}%")).or(gp.fan_speed_rpm.map(|r| format!("{r} RPM"))).unwrap_or_else(|| "--".into())));
            g.set_gpu_hotspot(ss(opt(gp.hotspot_temp_c, |t| format!("{t:.0}°C"))));
            g.set_gpu_vram_temp(ss(opt(gp.vram_temp_c, |t| format!("{t:.0}°C"))));
        }
        None => g.set_has_gpu(false),
    }

    // Mémoire
    let me = &m.memory;
    let ru = me.usage_percent;
    g.set_ram_line_text(ss(format!("{} utilisés sur {} installés", gb1(me.used_bytes), gb1(me.total_bytes))));
    g.set_ram_usage(ss(format!("{ru:.0}")));
    g.set_ram_color(metric_color(ru));
    g.set_ram_avail(ss(gb1(me.available_bytes)));
    let (l, ar) = spark(&s.hist.ram, 0.0, Some(100.0)); g.set_ram_line(l); g.set_ram_area(ar);
    let cached = me.cached_bytes + me.buffers_bytes;
    let tot = me.total_bytes.max(1) as f32;
    g.set_ram_used_f(me.used_bytes as f32 / tot);
    g.set_ram_cache_f((cached as f32 / tot).min(1.0 - me.used_bytes as f32 / tot));
    g.set_ram_used_gb(ss(gb1(me.used_bytes)));
    g.set_ram_cache_gb(ss(gb1(cached)));
    g.set_swap_used(ss(gb1(me.swap_used_bytes))); g.set_swap_total(ss(gb1(me.swap_total_bytes)));
    g.set_swap_pct_label(ss(format!("{:.0}%", me.swap_usage_percent)));
    g.set_swap_pct(me.swap_usage_percent / 100.0);
    g.set_swap_color(metric_color(me.swap_usage_percent));

    // Stockage
    g.set_disk_read(ss(rate(m.disks.read_bytes_per_sec)));
    g.set_disk_write(ss(rate(m.disks.write_bytes_per_sec)));
    g.set_disk_count(ss(format!("{} volumes et points de montage détectés", m.disks.partitions.len())));
    g.set_partitions(model(m.disks.partitions.iter().map(|p| MonPart {
        name: ss(p.name.clone()), mount: ss(p.mount_point.clone()), fs: ss(p.file_system.clone()),
        used: ss(gb1(p.used_bytes)), total: ss(gb1(p.total_bytes)), pct: p.usage_percent / 100.0,
        color: metric_color(p.usage_percent), removable: p.is_removable,
    }).collect()));
    let (l, ar) = spark(&s.hist.dr, 0.0, None); g.set_dr_line(l); g.set_dr_area(ar);
    let (l, ar) = spark(&s.hist.dw, 0.0, None); g.set_dw_line(l); g.set_dw_area(ar);

    // Réseau
    let p = m.network.primary_interface.as_ref();
    g.set_net_name(ss(p.map(|p| p.interface_name.clone()).unwrap_or_else(|| "Interface par défaut".into())));
    g.set_net_conn(ss(p.and_then(|p| p.connection_name.clone()).unwrap_or_default()));
    g.set_net_product(ss(p.map(|p| format!("{}{}{}",
        p.vendor.as_deref().map(|v| format!("{v} ")).unwrap_or_default(), p.product.clone().unwrap_or_default(),
        p.driver.as_deref().map(|d| format!(" (Pilote : {d})")).unwrap_or_default())).unwrap_or_default()));
    g.set_net_latency(ss(p.and_then(|p| p.latency_ms).map(|l| format!("{l:.1} ms")).unwrap_or_default()));
    let (txt, var) = match p.map(|p| p.connectivity.as_str()) {
        Some("full") => ("Internet Plein Accès", 2), Some("limited") => ("Réseau Local Uniquement", 1),
        Some("none") => ("Déconnecté", 4), _ => ("État inconnu", 0),
    };
    g.set_net_status(ss(txt)); g.set_net_status_variant(var);
    let (rx, tx) = p.map(|p| (p.rx_bytes_per_sec, p.tx_bytes_per_sec)).unwrap_or((m.network.rx_bytes_per_sec, m.network.tx_bytes_per_sec));
    g.set_net_down(ss(rate(rx))); g.set_net_up(ss(rate(tx)));
    g.set_net_total_rx(ss(bytes(p.map(|p| p.total_rx_bytes).unwrap_or(0))));
    g.set_net_total_tx(ss(bytes(p.map(|p| p.total_tx_bytes).unwrap_or(0))));
    let (l, ar) = spark(&s.hist.dl, 0.0, None); g.set_nd_line(l); g.set_nd_area(ar);
    let (l, ar) = spark(&s.hist.ul, 0.0, None); g.set_nu_line(l); g.set_nu_area(ar);
    let kv = |a: &str, b: String| model(vec![ss(a), ss(b)]);
    let speed = p.and_then(|p| p.speed_mbps).map(|s| if s >= 1000 { format!("{} Gbps", s as f64 / 1000.0) } else { format!("{s} Mbps") }).unwrap_or_else(|| "--".into());
    let props = vec![
        kv("ADRESSE IPV4", p.and_then(|p| p.ip_address.clone()).or(m.network.local_ip.clone()).unwrap_or_else(|| "Non assignée".into())),
        kv("PASSERELLE (GATEWAY)", p.and_then(|p| p.gateway.clone()).or(m.network.default_gateway.clone()).unwrap_or_else(|| "Non configurée".into())),
        kv("SERVEURS DNS", p.map(|p| p.dns_servers.join(", ")).filter(|d| !d.is_empty()).unwrap_or_else(|| "Par défaut (DHCP)".into())),
        kv("ADRESSE MAC", p.and_then(|p| p.mac_address.clone()).unwrap_or_else(|| "Non communiquée".into())),
        kv("VITESSE DE LIAISON", format!("{}{}", speed, p.and_then(|p| p.duplex.clone()).map(|d| format!(" ({d})")).unwrap_or_default())),
        kv("TAILLE MTU", p.and_then(|p| p.mtu).map(|m| format!("{m} octets")).unwrap_or_else(|| "1500 octets".into())),
        kv("ADRESSE IPV6", p.and_then(|p| p.ipv6_address.clone()).unwrap_or_else(|| "Désactivée / Non routée".into())),
        kv("PERTES / ERREURS PAQUETS", p.map(|p| format!("{} err ({} rejetés)", p.rx_errors.unwrap_or(0) + p.tx_errors.unwrap_or(0), p.rx_dropped.unwrap_or(0) + p.tx_dropped.unwrap_or(0))).unwrap_or_else(|| "--".into())),
    ];
    g.set_net_props(model(props));
    let wifi = p.map(|p| p.is_wireless).unwrap_or(false);
    g.set_net_wifi(wifi);
    if wifi {
        let p = p.unwrap();
        g.set_wifi_props(model(vec![
            kv("RÉSEAU WI-FI CONNECTÉ", p.wifi_ssid.clone().or(m.network.wifi_ssid.clone()).unwrap_or_else(|| "Réseau sans fil".into())),
            kv("FRÉQUENCE", p.wifi_frequency_mhz.map(|f| format!("{:.1} GHz", f as f64 / 1000.0)).unwrap_or_else(|| "--".into())),
            kv("PUISSANCE SIGNAL", p.wifi_signal_dbm.or(m.network.wifi_signal_dbm).map(|d| format!("{d} dBm")).unwrap_or_else(|| "--".into())),
            kv("QUALITÉ RÉCEPTION", p.wifi_signal_percent.or(m.network.wifi_signal_percent).map(|d| format!("{d}%")).unwrap_or_else(|| "--".into())),
        ]));
    }
    let main = p.map(|p| p.interface_name.clone());
    g.set_interfaces(model(m.network.interfaces.iter().filter(|i| Some(&i.name) != main.as_ref()).map(|i| MonIface {
        name: ss(i.name.clone()),
        addr: ss(i.ip_address.clone().or(i.mac_address.clone()).unwrap_or_else(|| if i.is_up { "Actif (Sans IP)".into() } else { "Inactif".into() })),
        up: i.is_up, wifi: i.is_wireless, rx: ss(rate(i.rx_bytes_per_sec)), tx: ss(rate(i.tx_bytes_per_sec)),
    }).collect()));

    // Processus (aperçu)
    let q = g.get_proc_filter().trim().to_lowercase();
    let mut list: Vec<ProcessSnapshot> = m.top_processes.iter().filter(|p| matches(p, &q)).cloned().collect();
    sort_procs(&mut list, s.sort_field, s.sort_desc);
    g.set_procs_count(list.len() as i32);
    g.set_procs(model(list.iter().map(proc_row).collect()));
    g.set_sort_field(s.sort_field);
    g.set_sort_desc(s.sort_desc);
}

fn apply_all(a: &AppWindow) {
    let g = a.global::<Monitor>();
    let s = st();
    let q = g.get_all_filter().trim().to_lowercase();
    let mut list: Vec<ProcessSnapshot> = s.all.iter().filter(|p| matches(p, &q)).cloned().collect();
    sort_procs(&mut list, s.all_sort_field, s.all_sort_desc);
    let pages = ((list.len() + PAGE_SIZE - 1) / PAGE_SIZE).max(1);
    let page = s.all_page.clamp(1, pages);
    g.set_all_total(list.len() as i32);
    g.set_all_pages(pages as i32);
    g.set_all_page(page as i32);
    g.set_all_procs(model(list.iter().skip((page - 1) * PAGE_SIZE).take(PAGE_SIZE).map(proc_row).collect()));
    g.set_all_sort_field(s.all_sort_field);
    g.set_all_sort_desc(s.all_sort_desc);
    g.set_all_procs_count(s.all.len() as i32);
}

fn apply_gpu(a: &AppWindow) {
    let g = a.global::<Monitor>();
    let s = st();
    let q = g.get_gpu_filter().trim().to_lowercase();
    let list: Vec<&GpuProcessInfo> = s.gpu.iter().filter(|p| q.is_empty() || p.name.to_lowercase().contains(&q) || p.command.to_lowercase().contains(&q) || p.pid.to_string().contains(&q)).collect();
    let total: u64 = s.gpu.iter().filter_map(|p| p.vram_used_mb).sum();
    g.set_gpu_total(s.gpu.len() as i32);
    g.set_gpu_procs_count(s.gpu.len() as i32);
    g.set_gpu_vram_total_label(ss(format!("{:.2} Go ({} Mo)", total as f64 / 1024.0, total)));
    g.set_gpu_procs(model(list.iter().map(|p| MonGpuProc {
        pid: ss(p.pid.to_string()), name: ss(p.name.clone()), command: ss(p.command.clone()), ptype: ss(p.process_type.clone()),
        vram: ss(p.vram_used_mb.map(|v| format!("{v} Mo")).unwrap_or_else(|| "--".into())),
        sm: ss(p.sm_util_percent.map(|v| format!("{v:.0}%")).unwrap_or_else(|| "-".into())),
        ram: ss(mem(p.ram_memory_bytes)), cpu: ss(format!("{:.1}%", p.cpu_usage)),
    }).collect()));
}

fn apply_smart(a: &AppWindow) {
    let g = a.global::<Monitor>();
    let s = st();
    let admin = s.smart.iter().any(|d| d.has_admin_data);
    g.set_smart_admin(admin);
    g.set_smart_count(s.smart.len() as i32);
    let n = |v: Option<u64>, suf: &str| v.map(|v| format!("{v}{suf}")).unwrap_or_else(|| "--".into());
    g.set_smart(model(s.smart.iter().map(|d| {
        let health = d.health_status.clone();
        let ok = matches!(health.to_lowercase().as_str(), "passed" | "ok" | "good" | "healthy" | "sain");
        MonDisk {
            device: ss(d.device.clone()), dtype: ss(d.disk_type.to_uppercase()), model: ss(d.model.clone()),
            meta: ss(format!("S/N : {} • Capacité : {}", if d.serial.is_empty() { "Non communiqué" } else { &d.serial }, gb1(d.size_bytes))),
            health: ss(health), health_ok: ok,
            temp: ss(d.temperature_c.map(|t| format!("{t:.0}°C")).unwrap_or_else(|| "--".into())),
            hours: ss(n(d.power_on_hours, " h")),
            wear: ss(d.percentage_used.map(|p| format!("{p}%")).unwrap_or_else(|| "Normal".into())),
            read: ss(n(d.data_units_read_gb, " Go")), written: ss(n(d.data_units_written_gb, " Go")),
            cycles: ss(n(d.power_cycles, "")),
            realloc: ss(d.reallocated_sectors.map(|v| v.to_string()).unwrap_or_else(|| if d.has_admin_data { "0".into() } else { "--".into() })),
            media: ss(d.media_errors.map(|v| v.to_string()).unwrap_or_else(|| if d.has_admin_data { "0".into() } else { "--".into() })),
            warning: ss(d.critical_warning.clone().unwrap_or_default()),
            source: ss(if d.has_admin_data { "✓ Registres SMART complets (Root)" } else { "Données sysfs standard" }),
        }
    }).collect()));
}

fn apply_dnf(a: &AppWindow) {
    let g = a.global::<Monitor>();
    let s = st();
    let Some(d) = &s.dnf else { return };
    g.set_dnf_available(d.is_available);
    g.set_dnf_badge(if d.is_available { d.upgrades_count as i32 } else { -1 });
    g.set_dnf_summary(ss(if d.upgrades_count > 0 { format!("{} mises à jour", d.upgrades_count) } else { "Système à jour".into() }));
    let when = chrono::DateTime::from_timestamp(if d.last_check_timestamp > 100_000_000_000 { (d.last_check_timestamp / 1000) as i64 } else { d.last_check_timestamp as i64 }, 0).map(|t| t.with_timezone(&chrono::Local).format("%d/%m/%Y %H:%M").to_string()).unwrap_or_default();
    g.set_dnf_last(ss(if when.is_empty() { String::new() } else { format!("Dernier scan : {when}") }));
    let q = g.get_dnf_filter().trim().to_lowercase();
    let list: Vec<&DnfPackageUpdate> = d.packages.iter().filter(|p| q.is_empty() || p.name.to_lowercase().contains(&q) || p.repository.to_lowercase().contains(&q)).collect();
    g.set_dnf_total(list.len() as i32);
    g.set_dnf_pkgs(model(list.iter().map(|p| MonPkg { name: ss(p.name.clone()), arch: ss(p.arch.clone()), evr: ss(p.evr.clone()), repo: ss(p.repository.clone()) }).collect()));
}

// ---------------------------------------------------------------- collecte

fn poll_once() {
    {
        let mut s = st();
        if s.inflight { return; }
        s.inflight = true;
        s.last_poll = Some(Instant::now());
    }
    with_app(|a| a.global::<Monitor>().set_refreshing(true));
    ctx().rt.spawn(async {
        let c = collector();
        let r = tokio::task::spawn_blocking(move || c.lock().map(|mut g| g.collect_snapshot()).map_err(|e| e.to_string())).await;
        let snap = r.map_err(|e| e.to_string()).and_then(|r| r);
        {
            let mut s = st();
            s.inflight = false;
            if let Ok(m) = &snap {
                let h = &mut s.hist;
                push(&mut h.cpu, m.cpu.global_usage);
                push(&mut h.ram, m.memory.usage_percent);
                push(&mut h.gpu, m.gpu.as_ref().and_then(|g| g.usage_percent).unwrap_or(0.0));
                let p = m.network.primary_interface.as_ref();
                push(&mut h.dl, p.map(|p| p.rx_bytes_per_sec).unwrap_or(m.network.rx_bytes_per_sec) as f32 / 1048576.0);
                push(&mut h.ul, p.map(|p| p.tx_bytes_per_sec).unwrap_or(m.network.tx_bytes_per_sec) as f32 / 1048576.0);
                push(&mut h.dr, m.disks.read_bytes_per_sec as f32 / 1048576.0);
                push(&mut h.dw, m.disks.write_bytes_per_sec as f32 / 1048576.0);
            }
            if let Ok(m) = snap.clone() { s.last = Some(m); }
        }
        with_app(move |a| {
            let g = a.global::<Monitor>();
            g.set_refreshing(false);
            match snap {
                Ok(_) => { g.set_error(SharedString::new()); apply_overview(a); }
                Err(e) => { g.set_loading(false); g.set_error(ss(e)); }
            }
        });
    });
}

fn fetch_gpu() {
    with_app(|a| a.global::<Monitor>().set_gpu_loading(true));
    ctx().rt.spawn(async {
        let c = collector();
        let r = tokio::task::spawn_blocking(move || c.lock().map(|mut g| g.collect_gpu_processes()).unwrap_or_default()).await.unwrap_or_default();
        st().gpu = r;
        with_app(|a| { a.global::<Monitor>().set_gpu_loading(false); apply_gpu(a); });
    });
}
fn fetch_all() {
    with_app(|a| a.global::<Monitor>().set_all_loading(true));
    ctx().rt.spawn(async {
        let c = collector();
        let r = tokio::task::spawn_blocking(move || c.lock().map(|mut g| g.collect_all_processes()).unwrap_or_default()).await.unwrap_or_default();
        st().all = r;
        with_app(|a| { a.global::<Monitor>().set_all_loading(false); apply_all(a); });
    });
}
fn fetch_smart(elevated: bool) {
    with_app(|a| a.global::<Monitor>().set_smart_loading(true));
    ctx().rt.spawn(async move {
        let c = collector();
        let r = tokio::task::spawn_blocking(move || c.lock().map(|g| g.collect_smart_disks(elevated)).unwrap_or_default()).await.unwrap_or_default();
        st().smart = r;
        with_app(|a| { a.global::<Monitor>().set_smart_loading(false); apply_smart(a); });
    });
}
fn fetch_dnf() {
    with_app(|a| a.global::<Monitor>().set_dnf_loading(true));
    ctx().rt.spawn(async {
        let r = tokio::task::spawn_blocking(SystemMetricsCollector::collect_dnf_updates).await.ok();
        if let Some(d) = r { st().dnf = Some(d); }
        with_app(|a| { a.global::<Monitor>().set_dnf_loading(false); apply_dnf(a); });
    });
}

fn kill_process(pid: u32, force: bool) -> Result<(), String> {
    // Mêmes garde-fous que src-tauri/src/monitoring/commands.rs
    if pid <= 1 { return Err("Action interdite: impossible de terminer les processus système critiques (PID <= 1).".into()); }
    if pid == std::process::id() { return Err("Action interdite: impossible de terminer le processus de l'application hôte Loom.".into()); }
    let sig = if force { libc::SIGKILL } else { libc::SIGTERM };
    let ret = unsafe { libc::kill(pid as libc::pid_t, sig) };
    if ret == 0 { Ok(()) } else { Err(format!("Échec de l'arrêt du processus {pid}: {}", std::io::Error::last_os_error())) }
}

fn find_proc(pid: u32) -> Option<(String, String, String)> {
    let s = st();
    s.last.as_ref().and_then(|m| m.top_processes.iter().find(|p| p.pid == pid)).or_else(|| s.all.iter().find(|p| p.pid == pid))
        .map(|p| (p.name.clone(), mem(p.memory_bytes), format!("{:.1}%", p.cpu_usage)))
        .or_else(|| s.gpu.iter().find(|p| p.pid == pid).map(|p| (p.name.clone(), mem(p.ram_memory_bytes), format!("{:.1}%", p.cpu_usage))))
}

// ---------------------------------------------------------------- câblage

pub fn wire(app: &AppWindow) {
    let _ = COLLECTOR.set(Arc::new(Mutex::new(SystemMetricsCollector::new())));
    let _ = ST.set(Mutex::new(State { interval_idx: 1, sort_field: 1, sort_desc: true, all_page: 1, all_sort_field: 1, all_sort_desc: true, ..Default::default() }));
    let g = app.global::<Monitor>();

    g.on_set_interval(|i| { st().interval_idx = i as usize; with_app(move |a| a.global::<Monitor>().set_interval_idx(i)); });
    g.on_toggle_pause(|| { let p = { let mut s = st(); s.paused = !s.paused; s.paused }; with_app(move |a| a.global::<Monitor>().set_paused(p)); });
    g.on_refresh(|| poll_once());
    g.on_set_tab(|t| {
        with_app(move |a| a.global::<Monitor>().set_tab(t));
        match t { 1 => fetch_gpu(), 2 => fetch_all(), 3 => fetch_smart(false), 4 => fetch_dnf(), _ => {} }
    });
    let w = app.as_weak();
    g.on_sort(move |f| {
        { let mut s = st(); if s.sort_field == f { s.sort_desc = !s.sort_desc } else { s.sort_field = f; s.sort_desc = true } }
        if let Some(a) = w.upgrade() { apply_overview(&a); }
    });
    let w = app.as_weak();
    g.on_all_sort(move |f| {
        { let mut s = st(); if s.all_sort_field == f { s.all_sort_desc = !s.all_sort_desc } else { s.all_sort_field = f; s.all_sort_desc = true } s.all_page = 1; }
        if let Some(a) = w.upgrade() { apply_all(&a); }
    });
    let w = app.as_weak();
    g.on_all_page_prev(move || { { let mut s = st(); s.all_page = s.all_page.saturating_sub(1).max(1); } if let Some(a) = w.upgrade() { apply_all(&a); } });
    let w = app.as_weak();
    g.on_all_page_next(move || { st().all_page += 1; if let Some(a) = w.upgrade() { apply_all(&a); } });
    g.on_all_refresh(|| fetch_all());
    g.on_gpu_refresh(|| fetch_gpu());
    g.on_smart_refresh(|| fetch_smart(false));
    g.on_smart_elevated(|| { toast("Demande d'accès administrateur (pkexec smartctl)…", 0); fetch_smart(true); });
    g.on_dnf_refresh(|| fetch_dnf());
    g.on_dnf_copy(|| {
        if crate::accounts::copy_to_clipboard("sudo dnf upgrade --refresh") { toast("Commande copiée", 1) } else { toast("Aucun utilitaire de presse-papiers (wl-copy / xclip)", 2) }
    });
    let w = app.as_weak();
    g.on_filters_changed(move || {
        if let Some(a) = w.upgrade() { st().all_page = 1; apply_overview(&a); apply_all(&a); apply_gpu(&a); apply_dnf(&a); }
    });
    let w = app.as_weak();
    g.on_request_kill(move |pid| {
        let Ok(pid) = pid.parse::<u32>() else { return };
        let Some(a) = w.upgrade() else { return };
        let info = find_proc(pid);
        let g = a.global::<Monitor>();
        let (name, m, c) = info.unwrap_or_else(|| (format!("PID {pid}"), "--".into(), "--".into()));
        g.set_kill_name(ss(name)); g.set_kill_mem(ss(m)); g.set_kill_cpu(ss(c)); g.set_kill_pid(ss(pid.to_string()));
        g.set_kill_force(false); g.set_kill_open(true);
        st().kill_pid = Some(pid);
    });
    let w = app.as_weak();
    g.on_confirm_kill(move || {
        let Some(a) = w.upgrade() else { return };
        let g = a.global::<Monitor>();
        let force = g.get_kill_force();
        g.set_kill_open(false);
        let target = st().kill_pid.take();
        if let Some(pid) = target {
            match kill_process(pid, force) {
                Ok(_) => { toast(format!("Signal {} envoyé au processus {pid}", if force { "SIGKILL" } else { "SIGTERM" }), 1); fetch_all(); fetch_gpu(); poll_once(); }
                Err(e) => toast(e, 2),
            }
        }
    });

    // Timer UI : ne collecte que lorsque la page Monitoring est affichée (comme onMounted/onUnmounted)
    let w = app.as_weak();
    let timer = slint::Timer::default();
    timer.start(slint::TimerMode::Repeated, Duration::from_millis(100), move || {
        let Some(a) = w.upgrade() else { return };
        if a.get_page() != 2 { st().initialized = false; return; }
        let (due, first) = {
            let mut s = st();
            let first = !s.initialized;
            s.initialized = true;
            let due = !s.paused && s.last_poll.map(|t| t.elapsed() >= Duration::from_millis(INTERVALS_MS[s.interval_idx.min(3)])).unwrap_or(true);
            (due, first)
        };
        if first { fetch_gpu(); fetch_smart(false); fetch_dnf(); }
        if due || first { poll_once(); }
    });
    std::mem::forget(timer);
}
