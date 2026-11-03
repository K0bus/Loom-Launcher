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

  // Format speed into readable string (e.g. "62.5 Mo/s" or "1.2 Go/s")
  const formattedSpeed = computed(() => {
    const mbps = totalRateMbps.value
    if (mbps <= 0) return '0 Mo/s'
    const mbytePerSec = mbps / 8
    if (mbytePerSec >= 1000) {
      return `${(mbytePerSec / 1000).toFixed(2)} Go/s`
    }
    if (mbytePerSec < 0.1) {
      const kbytePerSec = (mbps * 1024) / 8
      return `${kbytePerSec.toFixed(0)} Ko/s`
    }
    return `${mbytePerSec.toFixed(1)} Mo/s`
  })

  function getDownloadForGame(gameIdOrAppid: string, slug?: string): SteamDownloadItem | undefined {
    const clean = gameIdOrAppid.replace(/^steam-/, '')
    return activeDownloads.value.find(
      (d) =>
        d.appid === clean ||
        d.gameId === gameIdOrAppid ||
        d.gameId === `steam-${clean}` ||
        (slug && d.gameId.includes(slug))
    )
  }

  async function pollDownloads(): Promise<void> {
    try {
      const res = await ipc.invoke<{
        success: boolean
        active: SteamDownloadItem[]
        completed: SteamDownloadItem[]
        totalRateMbps: number
        installedAppids?: string[]
      }>('get_steam_downloads')

      if (res && res.success) {
        const prevActiveAppids = new Set(activeDownloads.value.map((d) => d.appid))
        activeDownloads.value = res.active || []
        completedDownloads.value = res.completed || []
        totalRateMbps.value = res.totalRateMbps || 0
        lastPolledAt.value = new Date()

        // Sync installed state across library if Steam returns installedAppids
        if (Array.isArray(res.installedAppids) && res.installedAppids.length > 0) {
          try {
            const gamesStore = useGamesStore()
            const installedSet = new Set(res.installedAppids)
            let changed = false
            for (const game of gamesStore.games) {
              if (game.platform === 'steam') {
                const aid = game.id.replace(/^steam-/, '')
                const isNowInstalled = installedSet.has(aid)
                if (game.installed !== isNowInstalled) {
                  game.installed = isNowInstalled
                  changed = true
                }
              }
            }
            if (changed) {
              gamesStore.saveLibrary?.()
            }
          } catch {}
        }

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

  // Automatic polling interval (1.0 second for true real-time)
  if (typeof window !== 'undefined') {
    pollDownloads()
    setInterval(pollDownloads, 1000)
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
