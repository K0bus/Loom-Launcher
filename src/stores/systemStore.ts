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
        mountPoint: '/mnt/games-nvme',
        label: 'NVMe Jeux (Samsung 990 Pro)',
        freeGb: 482,
        totalGb: 2000,
        usedPercentage: 76,
        isGamePartition: true
      },
      {
        mountPoint: '/',
        label: 'Système Linux (Arch)',
        freeGb: 124,
        totalGb: 500,
        usedPercentage: 75,
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

  async function refreshStats() {
    try {
      const data = await ipc.invoke<SystemStats>('get_system_stats')
      if (data) {
        stats.value = {
          ...stats.value,
          ...data,
          tauriDetected: ipc.isTauri()
        }
      }
    } catch (err) {
      console.warn('Could not refresh stats:', err)
    }
  }

  function markNotificationsAsRead() {
    notifications.value.forEach((n) => (n.read = true))
    stats.value.notificationsCount = 0
  }

  return {
    stats,
    notifications,
    refreshStats,
    markNotificationsAsRead
  }
})
