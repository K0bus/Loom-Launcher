import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { SystemStats, AppNotification } from '@/models/system'
import { ipc } from '@/services/ipc'

export const useSystemStore = defineStore('system', () => {
  const stats = ref<SystemStats>({
    cpuUsage: 18.2,
    ramUsedGb: 14.2,
    ramTotalGb: 32.0,
    networkDownKbps: 4820,
    networkUpKbps: 340,
    disks: [
      {
        mountPoint: '/home/k0bus/.local/share/Steam',
        label: 'Disque Principal (Steam Défaut)',
        freeGb: 389,
        totalGb: 895,
        usedPercentage: 57,
        isGamePartition: true
      },
      {
        mountPoint: '/',
        label: 'Système Linux (Racine)',
        freeGb: 389,
        totalGb: 895,
        usedPercentage: 57,
        isGamePartition: false
      }
    ],
    weeklyPlaytimeHours: 28.5,
    activeDownloadsCount: 0,
    notificationsCount: 2,
    appVersion: 'v0.1.0-alpha (Tauri v2)',
    tauriDetected: ipc.isTauri()
  })

  const notifications = ref<AppNotification[]>([
    {
      id: 'notif-1',
      title: 'Proton GE 9-20 Disponible',
      message: 'Une nouvelle version de compatibilité Wine/Proton est prête pour l\'installation.',
      type: 'info',
      timestamp: 'Il y a 10 min',
      read: false
    },
    {
      id: 'notif-2',
      title: 'Synchronisation Cloud Terminée',
      message: 'Vos sauvegardes Cyberpunk 2077 et Elden Ring sont à jour sur Steam Cloud.',
      type: 'success',
      timestamp: 'Il y a 1h',
      read: false
    }
  ])

  const isMaximized = ref(false)

  function setMaximized(value: boolean) {
    isMaximized.value = value
  }

  async function refreshStats() {
    try {
      const data = await ipc.invoke<any>('get_system_stats')
      if (data) {
        const cpuUsage = data.cpuUsage ?? data.cpu_usage ?? stats.value.cpuUsage
        const ramUsedGb = data.ramUsedGb ?? data.ram_used_gb ?? stats.value.ramUsedGb
        const ramTotalGb = data.ramTotalGb ?? data.ram_total_gb ?? stats.value.ramTotalGb
        const networkDown = data.networkDownKbps ?? data.networkDownKb ?? data.network_down_kb ?? stats.value.networkDownKbps
        const networkUp = data.networkUpKbps ?? data.networkUpKb ?? data.network_up_kb ?? stats.value.networkUpKbps
        const displayServer = data.displayServer ?? data.display_server ?? stats.value.displayServer
        const kernelVersion = data.kernelVersion ?? data.kernel_version ?? stats.value.kernelVersion

        const df = data.diskFreeGb ?? data.disk_free_gb
        const dt = data.diskTotalGb ?? data.disk_total_gb
        if (typeof df === 'number' && typeof dt === 'number' && dt > 0) {
          const rootDisk = stats.value.disks.find((d) => d.mountPoint === '/')
          if (rootDisk) {
            rootDisk.freeGb = Math.round(df)
            rootDisk.totalGb = Math.round(dt)
            rootDisk.usedPercentage = Math.round(((dt - df) / dt) * 100)
          }
        }

        stats.value = {
          ...stats.value,
          cpuUsage,
          ramUsedGb,
          ramTotalGb,
          networkDownKbps: networkDown,
          networkUpKbps: networkUp,
          displayServer,
          kernelVersion,
          tauriDetected: ipc.isTauri()
        }
      }
    } catch (err) {
      console.warn('Could not refresh stats:', err)
    }
  }

  async function refreshDiskLibraries() {
    try {
      const steamLibsRes = await ipc.invoke<any>('get_steam_libraries')
      const libs = Array.isArray(steamLibsRes) ? steamLibsRes : steamLibsRes?.libraries
      if (Array.isArray(libs) && libs.length > 0) {
        const availableLibs = libs.filter((l: any) => l.available !== false && l.totalGb > 0)
        if (availableLibs.length > 0) {
          stats.value.disks = availableLibs.map((l: any) => ({
            mountPoint: l.path,
            label: l.label || (l.isDefault ? 'Disque Steam Principal' : 'Bibliothèque Steam'),
            freeGb: Math.round(l.freeGb),
            totalGb: Math.round(l.totalGb),
            usedPercentage: Math.round(l.usedPercent),
            isGamePartition: true
          }))
        }
      }
    } catch {}
  }

  // Automatic real-time polling for system and resource stats (4s interval)
  if (typeof window !== 'undefined') {
    refreshStats()
    refreshDiskLibraries()
    setInterval(refreshStats, 4000)
    setInterval(refreshDiskLibraries, 30000)
  }

  function markNotificationsAsRead() {
    notifications.value.forEach((n) => (n.read = true))
    stats.value.notificationsCount = 0
  }

  return {
    stats,
    notifications,
    isMaximized,
    setMaximized,
    refreshStats,
    refreshDiskLibraries,
    markNotificationsAsRead
  }
})
