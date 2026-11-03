import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { ipc } from '@/services/ipc'
import { useGamesStore } from '@/stores/gamesStore'

export interface SteamDownloadItem {
  appid: string
  gameId: string
  name: string
  state: 'downloading' | 'staging' | 'completed' | 'paused'
  bytesDownloaded: number
  bytesTotal: number
  progressPercent: number
  downloadRateMbps: number
  coverUrl: string
  completedAt?: number
}

export const useDownloadsStore = defineStore('downloads', () => {
  const activeDownloads = ref<SteamDownloadItem[]>([])
  const completedDownloads = ref<SteamDownloadItem[]>([])
  const totalRateMbps = ref<number>(0)
  const lastPolledAt = ref<Date | null>(null)

  const isDownloading = computed(() => activeDownloads.value.length > 0)
  const activeCount = computed(() => activeDownloads.value.length)

  // Primary active download (e.g. for hero / header summary)
  const primaryDownload = computed<SteamDownloadItem | null>(() => {
    return activeDownloads.value[0] || null
  })

  // Format speed into readable string (e.g. "148.5 Mbps" or "1.4 Gbps" or "18.5 MB/s")
  const formattedSpeed = computed(() => {
    const mbps = totalRateMbps.value
    if (mbps <= 0) return '0 MB/s'
    const mbytePerSec = mbps / 8
    if (mbytePerSec >= 1000) {
      return `${(mbytePerSec / 1000).toFixed(2)} GB/s`
    }
    return `${mbytePerSec.toFixed(1)} MB/s`
  })

  function getDownloadForGame(gameIdOrAppid: string): SteamDownloadItem | undefined {
    const clean = gameIdOrAppid.replace(/^steam-/, '')
    return activeDownloads.value.find(
      (d) => d.appid === clean || d.gameId === gameIdOrAppid || d.gameId === `steam-${clean}`
    )
  }

  async function pollDownloads(): Promise<void> {
    try {
      const res = await ipc.invoke<{
        success: boolean
        active: SteamDownloadItem[]
        completed: SteamDownloadItem[]
        totalRateMbps: number
      }>('get_steam_downloads')

      if (res && res.success) {
        const prevActiveAppids = new Set(activeDownloads.value.map((d) => d.appid))
        activeDownloads.value = res.active || []
        completedDownloads.value = res.completed || []
        totalRateMbps.value = res.totalRateMbps || 0
        lastPolledAt.value = new Date()

        // Check if any active download finished
        const currentActiveAppids = new Set(activeDownloads.value.map((d) => d.appid))
        for (const prevId of prevActiveAppids) {
          if (!currentActiveAppids.has(prevId)) {
            // App finished downloading! Mark as installed in gamesStore
            try {
              const gamesStore = useGamesStore()
              const target =
                gamesStore.getGameById(`steam-${prevId}`) ||
                gamesStore.getGameById(prevId)
              if (target && !target.installed) {
                target.installed = true
                gamesStore.saveLibrary?.()
              }
            } catch {}
          }
        }
      }
    } catch (e) {
      console.warn('[DownloadsStore] Erreur polling téléchargements:', e)
    }
  }

  // Automatic polling interval (1.5 seconds)
  if (typeof window !== 'undefined') {
    pollDownloads()
    setInterval(pollDownloads, 1500)
  }

  return {
    activeDownloads,
    completedDownloads,
    totalRateMbps,
    isDownloading,
    activeCount,
    primaryDownload,
    formattedSpeed,
    lastPolledAt,
    pollDownloads,
    getDownloadForGame
  }
})
