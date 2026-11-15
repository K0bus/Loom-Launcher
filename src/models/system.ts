export interface DiskPartition {
  mountPoint: string
  label: string
  freeGb: number
  totalGb: number
  usedPercentage: number
  isGamePartition: boolean
}

export interface SystemStats {
  cpuUsage: number
  ramUsedGb: number
  ramTotalGb: number
  networkDownKbps: number
  networkUpKbps: number
  disks: DiskPartition[]
  weeklyPlaytimeHours: number
  activeDownloadsCount: number
  notificationsCount: number
  appVersion: string
  tauriDetected: boolean
  displayServer?: string
  kernelVersion?: string
}

export interface AppNotification {
  id: string
  title: string
  message: string
  type: 'info' | 'success' | 'warning' | 'error'
  timestamp: string
  read: boolean
}
