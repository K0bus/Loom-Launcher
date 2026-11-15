use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use sysinfo::{
    Components, Disks, Networks, ProcessesToUpdate, System,
};

use super::models::*;

pub struct SystemMetricsCollector {
    sys: System,
    disks: Disks,
    networks: Networks,
    components: Components,
    last_tick: Instant,
    last_diskstats: Option<(u64, u64)>, // (read_sectors, write_sectors)
    cached_gpu_name: Option<String>,
}

impl SystemMetricsCollector {
    pub fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();
        let disks = Disks::new_with_refreshed_list();
        let networks = Networks::new_with_refreshed_list();
        let components = Components::new_with_refreshed_list();

        let initial_diskstats = Self::read_diskstats_sectors();

        Self {
            sys,
            disks,
            networks,
            components,
            last_tick: Instant::now(),
            last_diskstats: initial_diskstats,
            cached_gpu_name: None,
        }
    }

    pub fn collect_snapshot(&mut self) -> SystemMetricsSnapshot {
        let now = Instant::now();
        let elapsed_secs = (now.duration_since(self.last_tick).as_millis() as f64 / 1000.0).max(0.1);
        self.last_tick = now;

        // 1. Refresh sysinfo components
        self.sys.refresh_cpu_usage();
        self.sys.refresh_memory();
        self.sys.refresh_processes(ProcessesToUpdate::All);
        self.disks.refresh();
        self.networks.refresh();
        self.components.refresh();

        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_millis() as u64;

        // 2. CPU
        let cpu_snapshot = self.collect_cpu();

        // 3. GPU
        let gpu_snapshot = self.collect_gpu();
        if let Some(ref gpu) = gpu_snapshot {
            self.cached_gpu_name = Some(gpu.name.clone());
        }

        // 4. Memory & Swap
        let memory_snapshot = self.collect_memory();

        // 5. Disks & I/O
        let disks_snapshot = self.collect_disks(elapsed_secs);

        // 6. Network
        let network_snapshot = self.collect_network(elapsed_secs);

        // 7. Battery
        let battery_snapshot = self.collect_battery();

        // 8. OS, Kernel & Session
        let os_snapshot = self.collect_os();

        // 9. Top Processes
        let top_processes = self.collect_top_processes();

        SystemMetricsSnapshot {
            timestamp_ms,
            cpu: cpu_snapshot,
            gpu: gpu_snapshot,
            memory: memory_snapshot,
            disks: disks_snapshot,
            network: network_snapshot,
            battery: battery_snapshot,
            os: os_snapshot,
            top_processes,
        }
    }

    pub fn get_system_summary(&mut self) -> SystemSummary {
        let os_name = System::name().unwrap_or_else(|| "Linux".into());
        let os_version = System::os_version().unwrap_or_default();
        let kernel_version = System::kernel_version().unwrap_or_default();
        let hostname = System::host_name().unwrap_or_else(|| "localhost".into());
        let display_server = Self::detect_display_server();

        let cpu_model = self
            .sys
            .cpus()
            .first()
            .map(|c| c.brand().trim().to_string())
            .unwrap_or_else(|| "Unknown CPU".into());
        let physical_cores = self.sys.physical_core_count().unwrap_or(1);
        let logical_cores = self.sys.cpus().len();
        let total_memory_bytes = self.sys.total_memory();

        let gpu_name = if let Some(ref name) = self.cached_gpu_name {
            Some(name.clone())
        } else if let Some(gpu) = self.collect_gpu() {
            Some(gpu.name)
        } else {
            None
        };

        let network_interfaces = self.networks.iter().map(|(k, _)| k.clone()).collect();

        SystemSummary {
            os_name,
            os_version,
            kernel_version,
            hostname,
            display_server,
            cpu_model,
            physical_cores,
            logical_cores,
            total_memory_bytes,
            gpu_name,
            network_interfaces,
        }
    }

    // --- CPU Collection ---
    fn collect_cpu(&self) -> CpuSnapshot {
        let model = self
            .sys
            .cpus()
            .first()
            .map(|c| c.brand().trim().to_string())
            .unwrap_or_else(|| "Unknown CPU".into());
        let physical_cores = self.sys.physical_core_count().unwrap_or(1);
        let logical_cores = self.sys.cpus().len();
        let global_usage = self.sys.global_cpu_usage();

        // Find package temp from Components or sysfs
        let package_temperature_c = self.find_cpu_package_temperature();

        // Core usage details
        let mut cores = Vec::with_capacity(self.sys.cpus().len());
        for (i, cpu) in self.sys.cpus().iter().enumerate() {
            let core_temp = self.find_core_temperature(i);
            cores.push(CoreUsage {
                id: i,
                name: cpu.name().to_string(),
                usage: cpu.cpu_usage(),
                frequency_mhz: cpu.frequency(),
                temperature_c: core_temp.or(package_temperature_c),
            });
        }

        let power_governor = Self::read_cpu_governor();

        CpuSnapshot {
            model,
            physical_cores,
            logical_cores,
            global_usage,
            package_temperature_c,
            cores,
            power_governor,
        }
    }

    fn find_cpu_package_temperature(&self) -> Option<f32> {
        // Search in sysinfo Components
        for comp in &self.components {
            let label = comp.label().to_lowercase();
            if label.contains("package id")
                || label.contains("tctl")
                || label.contains("tdie")
                || (label.contains("cpu") && !label.contains("fan"))
                || label.contains("coretemp")
                || label.contains("k10temp")
            {
                let temp = comp.temperature();
                if temp > 0.0 && temp < 130.0 {
                    return Some(temp);
                }
            }
        }

        // Fallback: Linux thermal zones
        if let Ok(entries) = fs::read_dir("/sys/class/thermal") {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("thermal_zone") {
                    let temp_path = entry.path().join("temp");
                    if let Ok(content) = fs::read_to_string(temp_path) {
                        if let Ok(milli_c) = content.trim().parse::<f32>() {
                            let c = milli_c / 1000.0;
                            if c > 0.0 && c < 125.0 {
                                return Some(c);
                            }
                        }
                    }
                }
            }
        }
        None
    }

    fn find_core_temperature(&self, core_idx: usize) -> Option<f32> {
        let target_label = format!("core {}", core_idx);
        for comp in &self.components {
            if comp.label().to_lowercase().contains(&target_label) {
                let t = comp.temperature();
                if t > 0.0 && t < 130.0 {
                    return Some(t);
                }
            }
        }
        None
    }

    fn read_cpu_governor() -> Option<String> {
        if let Ok(gov) = fs::read_to_string("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor") {
            let trimmed = gov.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
        if let Ok(prof) = fs::read_to_string("/sys/firmware/acpi/platform_profile") {
            let trimmed = prof.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
        None
    }

    // --- GPU Collection (Nvidia, AMD, Intel) ---
    fn collect_gpu(&self) -> Option<GpuSnapshot> {
        // Try Nvidia first
        if let Some(gpu) = Self::collect_nvidia_gpu() {
            return Some(gpu);
        }

        // Try AMD via sysfs
        if let Some(gpu) = Self::collect_amd_gpu() {
            return Some(gpu);
        }

        // Try Intel via sysfs
        if let Some(gpu) = Self::collect_intel_gpu() {
            return Some(gpu);
        }

        None
    }

    fn collect_nvidia_gpu() -> Option<GpuSnapshot> {
        let output = Command::new("nvidia-smi")
            .args([
                "--query-gpu=name,driver_version,utilization.gpu,memory.used,memory.total,temperature.gpu,fan.speed,power.draw",
                "--format=csv,noheader,nounits",
            ])
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let stdout = String::from_utf8(output.stdout).ok()?;
        let line = stdout.lines().next()?;
        let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();

        if parts.len() < 8 {
            return None;
        }

        let name = parts[0].to_string();
        let driver_version = Some(parts[1].to_string());
        let usage_percent = parts[2].parse::<f32>().ok();
        let memory_used_mb = parts[3].parse::<u64>().ok();
        let memory_total_mb = parts[4].parse::<u64>().ok();
        let core_temp_c = parts[5].parse::<f32>().ok();
        let fan_speed_percent = parts[6].parse::<u32>().ok();
        let power_watts = parts[7].parse::<f32>().ok();

        Some(GpuSnapshot {
            name,
            vendor: "NVIDIA".into(),
            driver_version,
            usage_percent,
            memory_used_mb,
            memory_total_mb,
            core_temp_c,
            hotspot_temp_c: None,
            vram_temp_c: None,
            fan_speed_percent,
            fan_speed_rpm: None,
            power_watts,
        })
    }

    fn collect_amd_gpu() -> Option<GpuSnapshot> {
        let drm_dir = Path::new("/sys/class/drm");
        if !drm_dir.exists() {
            return None;
        }

        let entries = fs::read_dir(drm_dir).ok()?;
        for entry in entries.flatten() {
            let fname = entry.file_name().to_string_lossy().to_string();
            if fname.starts_with("card") && !fname.contains('-') {
                let dev_path = entry.path().join("device");
                if dev_path.exists() {
                    // Check vendor (0x1002 is AMD)
                    let vendor_str = fs::read_to_string(dev_path.join("vendor")).unwrap_or_default();
                    if vendor_str.trim().contains("0x1002") || dev_path.join("gpu_busy_percent").exists() {
                        let usage_percent = fs::read_to_string(dev_path.join("gpu_busy_percent"))
                            .ok()
                            .and_then(|s| s.trim().parse::<f32>().ok());

                        let mem_used_bytes = fs::read_to_string(dev_path.join("mem_info_vram_used"))
                            .ok()
                            .and_then(|s| s.trim().parse::<u64>().ok());
                        let mem_total_bytes = fs::read_to_string(dev_path.join("mem_info_vram_total"))
                            .ok()
                            .and_then(|s| s.trim().parse::<u64>().ok());

                        let memory_used_mb = mem_used_bytes.map(|b| b / 1024 / 1024);
                        let memory_total_mb = mem_total_bytes.map(|b| b / 1024 / 1024);

                        // Temps and fans via hwmon
                        let (core_temp, hotspot_temp, vram_temp, fan_speed_rpm, fan_speed_percent, power_watts) =
                            Self::read_drm_hwmon(&dev_path);

                        let name = if let Ok(product) = fs::read_to_string(dev_path.join("product_name")) {
                            product.trim().to_string()
                        } else {
                            "AMD Radeon GPU".to_string()
                        };

                        return Some(GpuSnapshot {
                            name,
                            vendor: "AMD".into(),
                            driver_version: None,
                            usage_percent,
                            memory_used_mb,
                            memory_total_mb,
                            core_temp_c: core_temp,
                            hotspot_temp_c: hotspot_temp,
                            vram_temp_c: vram_temp,
                            fan_speed_percent,
                            fan_speed_rpm,
                            power_watts,
                        });
                    }
                }
            }
        }
        None
    }

    fn collect_intel_gpu() -> Option<GpuSnapshot> {
        let drm_dir = Path::new("/sys/class/drm");
        if !drm_dir.exists() {
            return None;
        }

        let entries = fs::read_dir(drm_dir).ok()?;
        for entry in entries.flatten() {
            let fname = entry.file_name().to_string_lossy().to_string();
            if fname.starts_with("card") && !fname.contains('-') {
                let dev_path = entry.path().join("device");
                if dev_path.exists() {
                    let vendor_str = fs::read_to_string(dev_path.join("vendor")).unwrap_or_default();
                    if vendor_str.trim().contains("0x8086") {
                        let (core_temp, _, _, _, _, power_watts) = Self::read_drm_hwmon(&dev_path);
                        return Some(GpuSnapshot {
                            name: "Intel HD / Iris / Arc Graphics".into(),
                            vendor: "Intel".into(),
                            driver_version: None,
                            usage_percent: None,
                            memory_used_mb: None,
                            memory_total_mb: None,
                            core_temp_c: core_temp,
                            hotspot_temp_c: None,
                            vram_temp_c: None,
                            fan_speed_percent: None,
                            fan_speed_rpm: None,
                            power_watts,
                        });
                    }
                }
            }
        }
        None
    }

    fn read_drm_hwmon(dev_path: &Path) -> (Option<f32>, Option<f32>, Option<f32>, Option<u32>, Option<u32>, Option<f32>) {
        let mut core_temp = None;
        let mut hotspot_temp = None;
        let mut vram_temp = None;
        let mut fan_rpm = None;
        let mut fan_pct = None;
        let mut power_watts = None;

        let hwmon_dir = dev_path.join("hwmon");
        if let Ok(entries) = fs::read_dir(hwmon_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                // temp1: edge/core
                if let Ok(val) = fs::read_to_string(path.join("temp1_input")) {
                    if let Ok(m) = val.trim().parse::<f32>() {
                        core_temp = Some(m / 1000.0);
                    }
                }
                // temp2: junction/hotspot
                if let Ok(val) = fs::read_to_string(path.join("temp2_input")) {
                    if let Ok(m) = val.trim().parse::<f32>() {
                        hotspot_temp = Some(m / 1000.0);
                    }
                }
                // temp3: mem
                if let Ok(val) = fs::read_to_string(path.join("temp3_input")) {
                    if let Ok(m) = val.trim().parse::<f32>() {
                        vram_temp = Some(m / 1000.0);
                    }
                }
                // fan speed
                if let Ok(val) = fs::read_to_string(path.join("fan1_input")) {
                    if let Ok(rpm) = val.trim().parse::<u32>() {
                        fan_rpm = Some(rpm);
                    }
                }
                if let Ok(val) = fs::read_to_string(path.join("pwm1")) {
                    if let Ok(pwm) = val.trim().parse::<f32>() {
                        fan_pct = Some(((pwm / 255.0) * 100.0).round() as u32);
                    }
                }
                // power
                if let Ok(val) = fs::read_to_string(path.join("power1_average")) {
                    if let Ok(u) = val.trim().parse::<f32>() {
                        power_watts = Some(u / 1_000_000.0);
                    }
                } else if let Ok(val) = fs::read_to_string(path.join("power1_input")) {
                    if let Ok(u) = val.trim().parse::<f32>() {
                        power_watts = Some(u / 1_000_000.0);
                    }
                }
            }
        }
        (core_temp, hotspot_temp, vram_temp, fan_rpm, fan_pct, power_watts)
    }

    // --- Memory & Swap ---
    fn collect_memory(&self) -> MemorySnapshot {
        let total_bytes = self.sys.total_memory();
        let available_bytes = self.sys.available_memory();
        let free_bytes = self.sys.free_memory();
        let used_bytes = self.sys.used_memory();

        let (buffers_bytes, cached_bytes) = Self::read_meminfo_buffers_cached();

        let usage_percent = if total_bytes > 0 {
            (used_bytes as f32 / total_bytes as f32) * 100.0
        } else {
            0.0
        };

        let swap_total_bytes = self.sys.total_swap();
        let swap_used_bytes = self.sys.used_swap();
        let swap_free_bytes = self.sys.free_swap();
        let swap_usage_percent = if swap_total_bytes > 0 {
            (swap_used_bytes as f32 / swap_total_bytes as f32) * 100.0
        } else {
            0.0
        };

        MemorySnapshot {
            total_bytes,
            used_bytes,
            free_bytes,
            available_bytes,
            buffers_bytes,
            cached_bytes,
            usage_percent,
            swap_total_bytes,
            swap_used_bytes,
            swap_free_bytes,
            swap_usage_percent,
        }
    }

    fn read_meminfo_buffers_cached() -> (u64, u64) {
        let mut buffers = 0u64;
        let mut cached = 0u64;

        if let Ok(content) = fs::read_to_string("/proc/meminfo") {
            for line in content.lines() {
                if line.starts_with("Buffers:") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        buffers = parts[1].parse::<u64>().unwrap_or(0) * 1024;
                    }
                } else if line.starts_with("Cached:") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        cached = parts[1].parse::<u64>().unwrap_or(0) * 1024;
                    }
                }
            }
        }
        (buffers, cached)
    }

    // --- Disks & Storage ---
    fn collect_disks(&mut self, elapsed_secs: f64) -> DiskActivitySnapshot {
        let mut partitions = Vec::new();

        for disk in &self.disks {
            let total = disk.total_space();
            let avail = disk.available_space();
            let used = total.saturating_sub(avail);
            let usage_percent = if total > 0 {
                (used as f32 / total as f32) * 100.0
            } else {
                0.0
            };

            let name = disk.name().to_string_lossy().to_string();
            let mount_point = disk.mount_point().to_string_lossy().to_string();
            let file_system = disk.file_system().to_string_lossy().to_string();

            // Check if disk temperature is found in Components
            let temp = self.find_disk_temperature(&name);

            partitions.push(DiskPartitionSnapshot {
                name,
                mount_point,
                file_system,
                total_bytes: total,
                available_bytes: avail,
                used_bytes: used,
                usage_percent,
                is_removable: disk.is_removable(),
                temperature_c: temp,
            });
        }

        // Calculate I/O read/write throughput from /proc/diskstats
        let (read_bytes_per_sec, write_bytes_per_sec) = self.calculate_disk_io(elapsed_secs);

        DiskActivitySnapshot {
            read_bytes_per_sec,
            write_bytes_per_sec,
            partitions,
        }
    }

    fn find_disk_temperature(&self, disk_name: &str) -> Option<f32> {
        let clean_name = disk_name.trim_start_matches("/dev/").to_lowercase();
        for comp in &self.components {
            let label = comp.label().to_lowercase();
            if (label.contains("nvme") || label.contains(&clean_name)) && !label.contains("fan") {
                let t = comp.temperature();
                if t > 0.0 && t < 110.0 {
                    return Some(t);
                }
            }
        }
        None
    }

    fn calculate_disk_io(&mut self, elapsed_secs: f64) -> (u64, u64) {
        let current_stats = Self::read_diskstats_sectors();
        if let (Some((cur_r, cur_w)), Some((last_r, last_w))) = (current_stats, self.last_diskstats) {
            self.last_diskstats = current_stats;
            let delta_r_sectors = cur_r.saturating_sub(last_r);
            let delta_w_sectors = cur_w.saturating_sub(last_w);
            // 1 sector = 512 bytes on Linux
            let read_bps = (delta_r_sectors as f64 * 512.0 / elapsed_secs).round() as u64;
            let write_bps = (delta_w_sectors as f64 * 512.0 / elapsed_secs).round() as u64;
            (read_bps, write_bps)
        } else {
            self.last_diskstats = current_stats;
            (0, 0)
        }
    }

    fn read_diskstats_sectors() -> Option<(u64, u64)> {
        let content = fs::read_to_string("/proc/diskstats").ok()?;
        let mut total_read_sectors = 0u64;
        let mut total_write_sectors = 0u64;

        for line in content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 14 {
                let dev_name = parts[2];
                // Only count primary block devices (nvme0n1, sda, sdb, vda, etc.)
                let is_primary_disk = (dev_name.starts_with("sd") && dev_name.len() == 3)
                    || (dev_name.starts_with("nvme") && dev_name.contains('n') && !dev_name.contains('p'))
                    || (dev_name.starts_with("vd") && dev_name.len() == 3);

                if is_primary_disk {
                    let r_sec = parts[5].parse::<u64>().unwrap_or(0);
                    let w_sec = parts[9].parse::<u64>().unwrap_or(0);
                    total_read_sectors += r_sec;
                    total_write_sectors += w_sec;
                }
            }
        }
        Some((total_read_sectors, total_write_sectors))
    }

    // --- Network Collection ---
    fn collect_network(&self, elapsed_secs: f64) -> NetworkSnapshot {
        let mut total_rx_bps = 0u64;
        let mut total_tx_bps = 0u64;
        let mut interfaces = Vec::new();

        let (default_gw, active_iface_name) = Self::detect_default_gateway();
        let mut local_ip = None;

        for (name, net_data) in &self.networks {
            // In sysinfo 0.31, received() is the delta bytes since last refresh
            let rx_bytes = net_data.received();
            let tx_bytes = net_data.transmitted();

            let rx_bps = (rx_bytes as f64 / elapsed_secs).round() as u64;
            let tx_bps = (tx_bytes as f64 / elapsed_secs).round() as u64;

            // Only sum up non-loopback interfaces for global traffic
            if name != "lo" && !name.starts_with("veth") && !name.starts_with("br-") {
                total_rx_bps += rx_bps;
                total_tx_bps += tx_bps;
            }

            let is_wireless = Path::new(&format!("/sys/class/net/{}/wireless", name)).exists();
            let mac_address = fs::read_to_string(format!("/sys/class/net/{}/address", name))
                .ok()
                .map(|s| s.trim().to_string());

            let operstate = fs::read_to_string(format!("/sys/class/net/{}/operstate", name))
                .unwrap_or_default();
            let is_up = operstate.trim() == "up" || net_data.total_received() > 0;

            let ip_addr = Self::get_interface_ip(name);
            if let (Some(ref active_name), Some(ref ip)) = (&active_iface_name, &ip_addr) {
                if name == active_name {
                    local_ip = Some(ip.clone());
                }
            } else if local_ip.is_none() && ip_addr.is_some() && name != "lo" {
                local_ip = ip_addr.clone();
            }

            interfaces.push(NetworkInterfaceSnapshot {
                name: name.clone(),
                is_up,
                ip_address: ip_addr,
                mac_address,
                is_wireless,
                rx_bytes_per_sec: rx_bps,
                tx_bytes_per_sec: tx_bps,
                total_rx_bytes: net_data.total_received(),
                total_tx_bytes: net_data.total_transmitted(),
            });
        }

        // Wi-Fi details if wireless present
        let (wifi_ssid, wifi_signal_percent, wifi_signal_dbm) = Self::detect_wifi_info();

        NetworkSnapshot {
            rx_bytes_per_sec: total_rx_bps,
            tx_bytes_per_sec: total_tx_bps,
            default_gateway: default_gw,
            local_ip,
            wifi_ssid,
            wifi_signal_percent,
            wifi_signal_dbm,
            interfaces,
        }
    }

    fn detect_default_gateway() -> (Option<String>, Option<String>) {
        if let Ok(content) = fs::read_to_string("/proc/net/route") {
            for line in content.lines().skip(1) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 && parts[1] == "00000000" {
                    let iface = parts[0].to_string();
                    if let Ok(gw_hex) = u32::from_str_radix(parts[2], 16) {
                        let ip = format!(
                            "{}.{}.{}.{}",
                            gw_hex & 0xFF,
                            (gw_hex >> 8) & 0xFF,
                            (gw_hex >> 16) & 0xFF,
                            (gw_hex >> 24) & 0xFF
                        );
                        return (Some(ip), Some(iface));
                    }
                }
            }
        }
        (None, None)
    }

    fn get_interface_ip(iface: &str) -> Option<String> {
        // Query via ip addr command or parse /proc/net/fib_trie
        let output = Command::new("ip")
            .args(["-o", "-4", "addr", "show", iface])
            .output()
            .ok()?;

        if output.status.success() {
            let txt = String::from_utf8(output.stdout).ok()?;
            for line in txt.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if let Some(pos) = parts.iter().position(|&x| x == "inet") {
                    if let Some(cidr) = parts.get(pos + 1) {
                        return Some(cidr.split('/').next().unwrap_or(*cidr).to_string());
                    }
                }
            }
        }
        None
    }

    fn detect_wifi_info() -> (Option<String>, Option<u8>, Option<i32>) {
        // First try reading /proc/net/wireless for signal quality
        let mut signal_dbm: Option<i32> = None;
        let mut signal_pct: Option<u8> = None;

        if let Ok(content) = fs::read_to_string("/proc/net/wireless") {
            for line in content.lines().skip(2) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 4 {
                    if let Ok(level) = parts[3].trim_end_matches('.').parse::<f32>() {
                        let dbm = level as i32;
                        signal_dbm = Some(dbm);
                        // Approximate percentage from dBm: -100 dBm = 0%, -50 dBm = 100%
                        let pct = ((dbm + 100) * 2).clamp(0, 100) as u8;
                        signal_pct = Some(pct);
                        break;
                    }
                }
            }
        }

        // Get SSID via iwgetid or nmcli
        let ssid = Command::new("iwgetid")
            .args(["-r"])
            .output()
            .ok()
            .and_then(|out| {
                if out.status.success() {
                    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    if !s.is_empty() {
                        Some(s)
                    } else {
                        None
                    }
                } else {
                    None
                }
            });

        (ssid, signal_pct, signal_dbm)
    }

    // --- Battery Collection ---
    fn collect_battery(&self) -> Option<BatterySnapshot> {
        let base_path = Path::new("/sys/class/power_supply");
        if !base_path.exists() {
            return None;
        }

        let entries = fs::read_dir(base_path).ok()?;
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("BAT") {
                let path = entry.path();
                let pct = fs::read_to_string(path.join("capacity"))
                    .ok()
                    .and_then(|s| s.trim().parse::<f32>().ok())
                    .unwrap_or(0.0);

                let state = fs::read_to_string(path.join("status"))
                    .ok()
                    .map(|s| s.trim().to_string())
                    .unwrap_or_else(|| "Unknown".into());

                let power_now = fs::read_to_string(path.join("power_now"))
                    .ok()
                    .and_then(|s| s.trim().parse::<f32>().ok())
                    .map(|u| u / 1_000_000.0);

                let energy_full = fs::read_to_string(path.join("energy_full"))
                    .ok()
                    .and_then(|s| s.trim().parse::<f32>().ok());
                let energy_design = fs::read_to_string(path.join("energy_full_design"))
                    .ok()
                    .and_then(|s| s.trim().parse::<f32>().ok());

                let health_percent = match (energy_full, energy_design) {
                    (Some(full), Some(design)) if design > 0.0 => Some((full / design * 100.0).clamp(0.0, 100.0)),
                    _ => None,
                };

                return Some(BatterySnapshot {
                    present: true,
                    percentage: pct,
                    state,
                    power_watts: power_now,
                    health_percent,
                    time_remaining_mins: None,
                });
            }
        }
        None
    }

    // --- OS, Kernel & Session ---
    fn collect_os(&self) -> OsSnapshot {
        let os_name = System::name().unwrap_or_else(|| "Linux".into());
        let os_version = System::os_version().unwrap_or_default();
        let kernel_version = System::kernel_version().unwrap_or_default();
        let hostname = System::host_name().unwrap_or_else(|| "localhost".into());
        let uptime_seconds = System::uptime();
        let uptime_formatted = Self::format_uptime(uptime_seconds);
        let display_server = Self::detect_display_server();

        let load = System::load_average();

        OsSnapshot {
            os_name,
            os_version,
            kernel_version,
            hostname,
            uptime_seconds,
            uptime_formatted,
            display_server,
            load_average_1m: load.one,
            load_average_5m: load.five,
            load_average_15m: load.fifteen,
        }
    }

    fn detect_display_server() -> String {
        if let Ok(val) = std::env::var("WAYLAND_DISPLAY") {
            if !val.trim().is_empty() {
                return "Wayland".into();
            }
        }
        if let Ok(val) = std::env::var("XDG_SESSION_TYPE") {
            let lower = val.to_lowercase();
            if lower.contains("wayland") {
                return "Wayland".into();
            }
            if lower.contains("x11") {
                return "X11".into();
            }
        }
        if std::env::var("DISPLAY").is_ok() {
            return "X11".into();
        }
        "Unknown".into()
    }

    fn format_uptime(secs: u64) -> String {
        let days = secs / 86400;
        let hours = (secs % 86400) / 3600;
        let mins = (secs % 3600) / 60;
        let s = secs % 60;

        if days > 0 {
            format!("{}j {}h {}m {}s", days, hours, mins, s)
        } else if hours > 0 {
            format!("{}h {}m {}s", hours, mins, s)
        } else if mins > 0 {
            format!("{}m {}s", mins, s)
        } else {
            format!("{}s", s)
        }
    }

    // --- Top Processes ---
    fn collect_top_processes(&self) -> Vec<ProcessSnapshot> {
        let mut list: Vec<ProcessSnapshot> = self
            .sys
            .processes()
            .iter()
            .map(|(pid, proc)| {
                let name = proc.name().to_string_lossy().to_string();
                let command = proc
                    .cmd()
                    .iter()
                    .map(|c| c.to_string_lossy().to_string())
                    .collect::<Vec<_>>()
                    .join(" ");

                let cmd_display = if !command.is_empty() { command } else { name.clone() };

                let disk_usage = proc.disk_usage();
                let status_str = format!("{:?}", proc.status());

                ProcessSnapshot {
                    pid: pid.as_u32(),
                    name,
                    command: cmd_display,
                    user: proc.user_id().map(|u| u.to_string()),
                    cpu_usage: proc.cpu_usage(),
                    memory_bytes: proc.memory(),
                    disk_read_bytes_per_sec: disk_usage.read_bytes,
                    disk_write_bytes_per_sec: disk_usage.written_bytes,
                    status: status_str,
                }
            })
            .collect();

        // Sort by CPU usage descending by default
        list.sort_by(|a, b| b.cpu_usage.partial_cmp(&a.cpu_usage).unwrap_or(std::cmp::Ordering::Equal));

        // Limit to top 50
        list.truncate(50);
        list
    }
}

pub type SharedCollector = Arc<Mutex<SystemMetricsCollector>>;
