export interface CoreUsage {
  id: number
  name: string
  usage: number
  frequencyMhz: number
  temperatureC?: number
}

export interface CpuSnapshot {
  model: string
  physicalCores: number
  logicalCores: number
  globalUsage: number
  packageTemperatureC?: number
  cores: CoreUsage[]
  powerGovernor?: string
}

export interface GpuSnapshot {
  name: string
  vendor: string
  driverVersion?: string
  usagePercent?: number
  memoryUsedMb?: number
  memoryTotalMb?: number
  coreTempC?: number
  hotspotTempC?: number
  vramTempC?: number
  fanSpeedPercent?: number
  fanSpeedRpm?: number
  powerWatts?: number
}

export interface MemorySnapshot {
  totalBytes: number
  usedBytes: number
  freeBytes: number
  availableBytes: number
  buffersBytes: number
  cachedBytes: number
  usagePercent: number
  swapTotalBytes: number
  swapUsedBytes: number
  swapFreeBytes: number
  swapUsagePercent: number
}

export interface BatterySnapshot {
  present: boolean
  percentage: number
  state: string
  powerWatts?: number
  healthPercent?: number
  timeRemainingMins?: number
}

export interface DiskPartitionSnapshot {
  name: string
  mountPoint: string
  fileSystem: string
  totalBytes: number
  availableBytes: number
  usedBytes: number
  usagePercent: number
  isRemovable: boolean
  temperatureC?: number
}

export interface DiskActivitySnapshot {
  readBytesPerSec: number
  writeBytesPerSec: number
  partitions: DiskPartitionSnapshot[]
}

export interface NetworkInterfaceSnapshot {
  name: string
  isUp: boolean
  ipAddress?: string
  macAddress?: string
  isWireless: boolean
  rxBytesPerSec: number
  txBytesPerSec: number
  totalRxBytes: number
  totalTxBytes: number
}

export interface NetworkSnapshot {
  rxBytesPerSec: number
  txBytesPerSec: number
  defaultGateway?: string
  localIp?: string
  wifiSsid?: string
  wifiSignalPercent?: number
  wifiSignalDbm?: number
  interfaces: NetworkInterfaceSnapshot[]
}

export interface ProcessSnapshot {
  pid: number
  name: string
  command: string
  user?: string
  cpuUsage: number
  memoryBytes: number
  diskReadBytesPerSec: number
  diskWriteBytesPerSec: number
  status: string
}

export interface OsSnapshot {
  osName: string
  osVersion: string
  kernelVersion: string
  hostname: string
  uptimeSeconds: number
  uptimeFormatted: string
  displayServer: string
  loadAverage1m: number
  loadAverage5m: number
  loadAverage15m: number
}

export interface SystemMetricsSnapshot {
  timestampMs: number
  cpu: CpuSnapshot
  gpu?: GpuSnapshot | null
  memory: MemorySnapshot
  disks: DiskActivitySnapshot
  network: NetworkSnapshot
  battery?: BatterySnapshot | null
  os: OsSnapshot
  topProcesses: ProcessSnapshot[]
}

export interface SystemSummary {
  osName: string
  osVersion: string
  kernelVersion: string
  hostname: string
  displayServer: string
  cpuModel: string
  physicalCores: number
  logicalCores: number
  totalMemoryBytes: number
  gpuName?: string
  networkInterfaces: string[]
}
