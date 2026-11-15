use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CoreUsage {
    pub id: usize,
    pub name: String,
    pub usage: f32,
    pub frequency_mhz: u64,
    pub temperature_c: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CpuSnapshot {
    pub model: String,
    pub physical_cores: usize,
    pub logical_cores: usize,
    pub global_usage: f32,
    pub package_temperature_c: Option<f32>,
    pub cores: Vec<CoreUsage>,
    pub power_governor: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GpuSnapshot {
    pub name: String,
    pub vendor: String,
    pub driver_version: Option<String>,
    pub usage_percent: Option<f32>,
    pub memory_used_mb: Option<u64>,
    pub memory_total_mb: Option<u64>,
    pub core_temp_c: Option<f32>,
    pub hotspot_temp_c: Option<f32>,
    pub vram_temp_c: Option<f32>,
    pub fan_speed_percent: Option<u32>,
    pub fan_speed_rpm: Option<u32>,
    pub power_watts: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MemorySnapshot {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub free_bytes: u64,
    pub available_bytes: u64,
    pub buffers_bytes: u64,
    pub cached_bytes: u64,
    pub usage_percent: f32,
    pub swap_total_bytes: u64,
    pub swap_used_bytes: u64,
    pub swap_free_bytes: u64,
    pub swap_usage_percent: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BatterySnapshot {
    pub present: bool,
    pub percentage: f32,
    pub state: String,
    pub power_watts: Option<f32>,
    pub health_percent: Option<f32>,
    pub time_remaining_mins: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DiskPartitionSnapshot {
    pub name: String,
    pub mount_point: String,
    pub file_system: String,
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub used_bytes: u64,
    pub usage_percent: f32,
    pub is_removable: bool,
    pub temperature_c: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DiskActivitySnapshot {
    pub read_bytes_per_sec: u64,
    pub write_bytes_per_sec: u64,
    pub partitions: Vec<DiskPartitionSnapshot>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NetworkInterfaceSnapshot {
    pub name: String,
    pub is_up: bool,
    pub ip_address: Option<String>,
    pub mac_address: Option<String>,
    pub is_wireless: bool,
    pub rx_bytes_per_sec: u64,
    pub tx_bytes_per_sec: u64,
    pub total_rx_bytes: u64,
    pub total_tx_bytes: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NetworkSnapshot {
    pub rx_bytes_per_sec: u64,
    pub tx_bytes_per_sec: u64,
    pub default_gateway: Option<String>,
    pub local_ip: Option<String>,
    pub wifi_ssid: Option<String>,
    pub wifi_signal_percent: Option<u8>,
    pub wifi_signal_dbm: Option<i32>,
    pub interfaces: Vec<NetworkInterfaceSnapshot>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProcessSnapshot {
    pub pid: u32,
    pub name: String,
    pub command: String,
    pub user: Option<String>,
    pub cpu_usage: f32,
    pub memory_bytes: u64,
    pub disk_read_bytes_per_sec: u64,
    pub disk_write_bytes_per_sec: u64,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OsSnapshot {
    pub os_name: String,
    pub os_version: String,
    pub kernel_version: String,
    pub hostname: String,
    pub uptime_seconds: u64,
    pub uptime_formatted: String,
    pub display_server: String,
    pub load_average_1m: f64,
    pub load_average_5m: f64,
    pub load_average_15m: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SystemMetricsSnapshot {
    pub timestamp_ms: u64,
    pub cpu: CpuSnapshot,
    pub gpu: Option<GpuSnapshot>,
    pub memory: MemorySnapshot,
    pub disks: DiskActivitySnapshot,
    pub network: NetworkSnapshot,
    pub battery: Option<BatterySnapshot>,
    pub os: OsSnapshot,
    pub top_processes: Vec<ProcessSnapshot>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SystemSummary {
    pub os_name: String,
    pub os_version: String,
    pub kernel_version: String,
    pub hostname: String,
    pub display_server: String,
    pub cpu_model: String,
    pub physical_cores: usize,
    pub logical_cores: usize,
    pub total_memory_bytes: u64,
    pub gpu_name: Option<String>,
    pub network_interfaces: Vec<String>,
}
