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

export interface ModDownloadItem {
  id: string
  instanceId: string
  instanceName?: string
  remoteModId: string
  name: string
  iconUrl?: string
  provider: string
  fileName?: string
  bytesDownloaded: number
  bytesTotal: number
  progressPercent: number
  downloadRateBytesPerSec: number
  status: 'pending' | 'downloading' | 'installing' | 'completed' | 'error'
  error?: string
  completedAt?: number
}

export const useDownloadsStore = defineStore('downloads', () => {
  // Steam Downloads State
  const activeDownloads = ref<SteamDownloadItem[]>([])
  const completedDownloads = ref<SteamDownloadItem[]>([])
  const totalRateMbps = ref<number>(0)
  const lastPolledAt = ref<Date | null>(null)

  // Minecraft Mod Downloads State
  const activeModDownloads = ref<ModDownloadItem[]>([])
  const completedModDownloads = ref<ModDownloadItem[]>([])

  // Combined Active State
  const isDownloading = computed(
    () => activeDownloads.value.length > 0 || activeModDownloads.value.length > 0
  )
  const activeCount = computed(
    () => activeDownloads.value.length + activeModDownloads.value.length
  )

  // Primary active download (Steam or Mod)
  const primaryDownload = computed<SteamDownloadItem | null>(() => {
    return activeDownloads.value[0] || null
  })

  // Format speed into readable string (e.g. "62.5 Mo/s" or "1.2 Go/s")
  const formattedSpeed = computed(() => {
    // Check steam speed
    let totalBytesPerSec = (totalRateMbps.value * 1024 * 1024) / 8
    // Add mod download speed
    for (const m of activeModDownloads.value) {
      totalBytesPerSec += m.downloadRateBytesPerSec || 0
    }

    if (totalBytesPerSec <= 0) return '0 Mo/s'
    const mb = totalBytesPerSec / (1024 * 1024)
    if (mb >= 1000) {
      return `${(mb / 1024).toFixed(2)} Go/s`
    }
    if (mb < 0.1) {
      const kb = totalBytesPerSec / 1024
      return `${kb.toFixed(0)} Ko/s`
    }
    return `${mb.toFixed(1)} Mo/s`
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

  function getModDownload(remoteModIdOrSlug: string, instanceId?: string): ModDownloadItem | undefined {
    if (!remoteModIdOrSlug) return undefined
    const q = remoteModIdOrSlug.toLowerCase()
    return activeModDownloads.value.find(
      (d) =>
        (!instanceId || d.instanceId === instanceId) &&
        (d.remoteModId.toLowerCase() === q ||
          d.name.toLowerCase() === q ||
          (d.fileName && d.fileName.toLowerCase().replace(/\.jar$/, '') === q) ||
          d.id.toLowerCase().includes(q))
    )
  }

  function updateModDownload(item: any) {
    if (!item || !item.id) return
    const key = item.id
    const isMatching = (d: ModDownloadItem) =>
      d.id === key ||
      (d.remoteModId === item.remoteModId && d.instanceId === item.instanceId) ||
      (Boolean(item.instanceName) && d.instanceName === item.instanceName && (d.remoteModId?.startsWith('pack') || item.remoteModId?.startsWith('pack') || d.id?.startsWith('pack') || item.id?.startsWith('pack')))

    const existingIdx = activeModDownloads.value.findIndex(isMatching)

    if (item.status === 'completed') {
      activeModDownloads.value = activeModDownloads.value.filter((d) => !isMatching(d))
      completedModDownloads.value.unshift({
        id: item.id,
        instanceId: item.instanceId,
        instanceName: item.instanceName,
        remoteModId: item.remoteModId,
        name: item.modName || item.name || 'Mod Minecraft',
        iconUrl: item.iconUrl,
        provider: item.provider,
        fileName: item.fileName,
        bytesDownloaded: item.bytesDownloaded || item.bytesTotal || 0,
        bytesTotal: item.bytesTotal || 0,
        progressPercent: 100,
        downloadRateBytesPerSec: 0,
        status: 'completed',
        completedAt: Math.floor(Date.now() / 1000)
      })
      if (completedModDownloads.value.length > 8) {
        completedModDownloads.value = completedModDownloads.value.slice(0, 8)
      }
      return
    }

    if (item.status === 'error') {
      activeModDownloads.value = activeModDownloads.value.filter((d) => !isMatching(d))
      return
    }

    const mapped: ModDownloadItem = {
      id: item.id,
      instanceId: item.instanceId,
      instanceName: item.instanceName,
      remoteModId: item.remoteModId,
      name: item.modName || item.name || 'Mod Minecraft',
      iconUrl: item.iconUrl,
      provider: item.provider,
      fileName: item.fileName,
      bytesDownloaded: item.bytesDownloaded || 0,
      bytesTotal: item.bytesTotal || 0,
      progressPercent: item.progressPercent || 0,
      downloadRateBytesPerSec: item.downloadRateBytesPerSec || 0,
      status: item.status || 'downloading',
      error: item.error
    }

    if (existingIdx !== -1) {
      activeModDownloads.value[existingIdx] = mapped
    } else {
      activeModDownloads.value.unshift(mapped)
    }
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
        const newActive = res.active || []
        const newCompleted = res.completed || []
        const newRate = res.totalRateMbps || 0

        // Only mutate if changed to prevent unnecessary reactive cascade
        const activeChanged =
          newActive.length !== activeDownloads.value.length ||
          newActive.some((na, idx) => {
            const cur = activeDownloads.value[idx]
            return !cur || cur.appid !== na.appid || cur.progressPercent !== na.progressPercent || cur.state !== na.state
          })
        if (activeChanged) {
          activeDownloads.value = newActive
        }

        const completedChanged =
          newCompleted.length !== completedDownloads.value.length ||
          newCompleted.some((nc, idx) => {
            const cur = completedDownloads.value[idx]
            return !cur || cur.appid !== nc.appid
          })
        if (completedChanged) {
          completedDownloads.value = newCompleted
        }

        if (totalRateMbps.value !== newRate) {
          totalRateMbps.value = newRate
        }
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
        const currentActiveAppids = new Set((res.active || []).map((d) => d.appid))
        for (const prevId of prevActiveAppids) {
          if (!currentActiveAppids.has(prevId)) {
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

  // Automatic adaptive polling interval & SSE Progress Listener
  let pollTimer: any = null
  function scheduleNextPoll() {
    if (pollTimer) clearTimeout(pollTimer)
    const delay = activeDownloads.value.length > 0 ? 1500 : 4000
    pollTimer = setTimeout(async () => {
      await pollDownloads()
      scheduleNextPoll()
    }, delay)
  }

  if (typeof window !== 'undefined') {
    pollDownloads().finally(() => scheduleNextPoll())

    // Listen to real-time mod download progress events
    ipc.listen('minecraft-download-progress', (e: any) => {
      updateModDownload(e?.payload || e)
    }).catch(() => {})

    // Catch up on initial active mod downloads
    ipc.getMinecraftActiveDownloadsCmd().then((downloads) => {
      if (Array.isArray(downloads)) {
        activeModDownloads.value = downloads
          .filter((dl: any) => dl && dl.status !== 'completed' && dl.status !== 'error')
          .map((dl: any) => ({
            id: dl.id,
            instanceId: dl.instanceId,
            instanceName: dl.instanceName,
            remoteModId: dl.remoteModId,
            name: dl.modName || dl.name || 'Mod Minecraft',
            iconUrl: dl.iconUrl,
            provider: dl.provider,
            fileName: dl.fileName,
            bytesDownloaded: dl.bytesDownloaded || 0,
            bytesTotal: dl.bytesTotal || 0,
            progressPercent: dl.progressPercent || 0,
            downloadRateBytesPerSec: dl.downloadRateBytesPerSec || 0,
            status: dl.status || 'downloading',
            error: dl.error
          }))
      }
    }).catch(() => {})
  }

  return {
    activeDownloads,
    completedDownloads,
    activeModDownloads,
    completedModDownloads,
    totalRateMbps,
    isDownloading,
    activeCount,
    primaryDownload,
    formattedSpeed,
    lastPolledAt,
    pollDownloads,
    getDownloadForGame,
    getModDownload,
    updateModDownload
  }
})
