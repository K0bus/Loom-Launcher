import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type {
  MinecraftInstance,
  MinecraftMod,
  MinecraftLoader,
  MinecraftInstanceRecord,
  ModpackCatalogItem,
  MinecraftGlobalStats,
  MinecraftPlaytimeTickEvent
} from '@/models/minecraft'
import type {
  MinecraftAccount,
  DeviceCodeResponse,
  MinecraftLogEvent,
  MinecraftExitEvent,
  ProfileModRecord,
  UnifiedModSearchResult
} from '@/models/modding'
import type { IGDBGame } from '@/models/igdb'
import { igdbService } from '@/services/igdbService'
import { ipc } from '@/services/ipc'
import { useSettingsStore } from '@/stores/settingsStore'
import { useToastStore } from '@/stores/toastStore'

export const useMinecraftStore = defineStore('minecraft', () => {
  // Real instances loaded from storage / database
  const instances = ref<MinecraftInstance[]>([])
  const isLoaded = ref(false)
  const selectedInstanceId = ref<string>('')

  // Mod Search / Catalog state
  const modSearchResults = ref<UnifiedModSearchResult[]>([])
  const isSearchingMods = ref(false)
  const installingModIds = ref<Record<string, boolean>>({})

  // Modpack Search / Catalog state
  const modpackSearchResults = ref<ModpackCatalogItem[]>([])
  const isSearchingModpacks = ref(false)
  const installingModpackIds = ref<Record<string, boolean>>({})

  // Java Detection & Runtimes state
  const detectedJavaRuntimes = ref<Array<{ path: string; major: number; versionStr: string; vendor: string }>>([])
  const isDetectingJava = ref(false)

  // Interactive Dependency Modal State
  const dependencyModalOpen = ref(false)
  const pendingModToInstall = ref<UnifiedModSearchResult | null>(null)
  const targetInstanceId = ref<string>('')
  const detectedDependencies = ref<any[]>([])
  const selectedDependencies = ref<string[]>([])
  const isResolvingDeps = ref(false)
  const isBatchInstalling = ref(false)

  // Microsoft Account State
  const account = ref<MinecraftAccount | null>(null)
  const deviceCodeInfo = ref<DeviceCodeResponse | null>(null)
  const isAuthPolling = ref(false)
  const authModalOpen = ref(false)
  const authError = ref<string | null>(null)
  let authPollTimer: any = null

  // Official IGDB Metadata for Minecraft (https://www.igdb.com/games/minecraft--1)
  const igdbGame = ref<IGDBGame | null>(null)
  const isIgdbLoading = ref(false)

  const defaultCoverUrl = computed(() => {
    if (igdbGame.value?.cover?.url) {
      return igdbGame.value.cover.url
    }
    return 'https://images.igdb.com/igdb/image/upload/t_cover_big/co8fu7.jpg'
  })

  const defaultBannerUrl = computed(() => {
    if (igdbGame.value?.artworks && igdbGame.value.artworks.length > 0 && igdbGame.value.artworks[0].url) {
      return igdbGame.value.artworks[0].url
    }
    if (igdbGame.value?.cover?.url) {
      return igdbGame.value.cover.url.replace('/t_cover_big/', '/t_1080p/')
    }
    return 'https://images.igdb.com/igdb/image/upload/t_1080p/co8fu7.jpg'
  })

  const igdbSummary = computed(() => {
    return (
      igdbGame.value?.summary ||
      'Explorez des mondes infinis générés aléatoirement et construisez tout ce qui vous passe par la tête, du modeste abri au plus grandiose des châteaux. Jouez en mode créatif avec des ressources illimitées, ou creusez jusqu\'au centre de la Terre en mode survie.'
    )
  })

  const igdbRating = computed(() => {
    return igdbGame.value?.total_rating ? Math.round(igdbGame.value.total_rating) : 91
  })

  async function loadIgdbMetadata() {
    isIgdbLoading.value = true
    try {
      const g =
        (await igdbService.getGameBySlug('minecraft--1')) ||
        (await igdbService.getGameBySlug('minecraft')) ||
        (await igdbService.getGameById(135400))
      if (g) {
        igdbGame.value = g
      }
    } catch (err) {
      console.warn('[MinecraftStore] Could not load IGDB metadata:', err)
    } finally {
      isIgdbLoading.value = false
    }
  }

  // Global Minecraft Stats (persistent and independent of instance deletions)
  const globalStats = ref<MinecraftGlobalStats>({
    playtimeSeconds: 0,
    playtimeMinutes: 0,
    lastPlayed: null,
    sessionsCount: 0
  })

  function formatPlaytime(seconds: number = 0): string {
    if (!seconds || seconds <= 0) return '0 min'
    const totalMinutes = Math.floor(seconds / 60)
    if (totalMinutes < 60) {
      return `${Math.max(1, totalMinutes)} min`
    }
    const hours = Math.floor(totalMinutes / 60)
    const mins = totalMinutes % 60
    if (mins === 0) return `${hours} h`
    return `${hours} h ${mins} min`
  }

  function formatPlaytimeCompact(seconds: number = 0): string {
    if (!seconds || seconds <= 0) return '0 min'
    const totalMinutes = Math.floor(seconds / 60)
    if (totalMinutes < 60) return `${Math.max(1, totalMinutes)} min`
    const hours = (totalMinutes / 60).toFixed(1)
    return `${hours} h`
  }

  const globalPlaytimeFormatted = computed(() => {
    return formatPlaytime(globalStats.value.playtimeSeconds)
  })

  function formatLastPlayed(lastPlayed?: string | number | null): string {
    if (!lastPlayed) return 'Jamais jouée'
    let time: number
    if (typeof lastPlayed === 'number') {
      time = lastPlayed > 1e11 ? lastPlayed : lastPlayed * 1000
    } else {
      time = new Date(lastPlayed).getTime()
    }
    if (isNaN(time) || time <= 0) return 'Jamais jouée'
    const diffSec = Math.floor((Date.now() - time) / 1000)
    if (diffSec < 0) return "À l'instant"
    if (diffSec < 60) return "À l'instant"
    if (diffSec < 3600) return `Il y a ${Math.floor(diffSec / 60)} min`
    if (diffSec < 86400) return `Il y a ${Math.floor(diffSec / 3600)} h`
    if (diffSec < 86400 * 7) return `Il y a ${Math.floor(diffSec / 86400)} j`
    return new Date(time).toLocaleDateString('fr-FR', { day: 'numeric', month: 'short' })
  }

  const globalLastPlayedFormatted = computed(() => {
    return formatLastPlayed(globalStats.value.lastPlayed)
  })

  const activeInstance = computed(() => {
    return instances.value.find((i) => i.id === selectedInstanceId.value) || instances.value[0] || null
  })

  const lastPlayedInstance = computed<MinecraftInstance | undefined>(() => {
    if (instances.value.length === 0) return undefined
    const withPlayed = instances.value.filter((i) => i.lastPlayed)
    if (withPlayed.length > 0) {
      return [...withPlayed].sort(
        (a, b) => new Date(b.lastPlayed!).getTime() - new Date(a.lastPlayed!).getTime()
      )[0]
    }
    return activeInstance.value || instances.value[0]
  })

  // Initialize event listeners for JVM stdout/stderr streaming
  let eventListenersInitialized = false
  async function init() {
    if (eventListenersInitialized) return
    eventListenersInitialized = true

    // 1. Listen for real-time JVM output with batched buffer & ring cap
    const pendingLogs: Record<string, any[]> = {}
    let logBatchTimer: any = null

    function flushLogBatch() {
      if (logBatchTimer) {
        clearTimeout(logBatchTimer)
        logBatchTimer = null
      }
      for (const [instId, newItems] of Object.entries(pendingLogs)) {
        if (!newItems || newItems.length === 0) continue
        const inst = instances.value.find((i) => i.id === instId)
        if (inst) {
          inst.logs.push(...newItems)
          // Hard cap: keep maximum 500 lines for the live console to prevent DOM & memory bloat
          if (inst.logs.length > 500) {
            inst.logs.splice(0, inst.logs.length - 500)
          }
        }
      }
      for (const key of Object.keys(pendingLogs)) {
        delete pendingLogs[key]
      }
    }

    const queueLogItem = (instanceId: string, item: any) => {
      if (!pendingLogs[instanceId]) {
        pendingLogs[instanceId] = []
      }
      pendingLogs[instanceId].push(item)
      if (pendingLogs[instanceId].length >= 40) {
        flushLogBatch()
      } else if (!logBatchTimer) {
        logBatchTimer = setTimeout(flushLogBatch, 60)
      }
    }

    await ipc.listen<MinecraftLogEvent>('minecraft-log', ({ payload }) => {
      queueLogItem(payload.instance_id, {
        timestamp: payload.timestamp || new Date().toLocaleTimeString(),
        level: (payload.level as any) || 'INFO',
        source: payload.source || 'Minecraft',
        message: payload.message
      })
    })

    await ipc.listen<any>('minecraft-log-batch', ({ payload }) => {
      if (payload.entries && Array.isArray(payload.entries)) {
        for (const entry of payload.entries) {
          queueLogItem(payload.instance_id, {
            timestamp: entry.timestamp || new Date().toLocaleTimeString(),
            level: (entry.level as any) || 'INFO',
            source: entry.source || 'Minecraft',
            message: entry.message
          })
        }
      }
    })

    await ipc.listen<MinecraftExitEvent>('minecraft-exit', ({ payload }) => {
      const inst = instances.value.find((i) => i.id === payload.instance_id)
      if (inst) {
        inst.status = 'idle'
        inst.logs.push({
          timestamp: new Date().toLocaleTimeString(),
          level: 'INFO',
          source: 'LoomLauncher',
          message: `Processus Minecraft terminé (Code de sortie: ${payload.exit_code ?? 0}).`
        })
      }

      try {
        const settingsStore = useSettingsStore()
        if (settingsStore.settings.afterGameLaunch !== 'keep') {
          ipc.showWindow().catch(() => {})
        }
        if (settingsStore.settings.autoSyncCloudSaves) {
          const toast = useToastStore()
          toast.show('Mondes & sauvegardes Minecraft vérifiés et synchronisés', 'success')
        }
      } catch {}
    })

    // Listen for real-time playtime tick
    await ipc.listen<MinecraftPlaytimeTickEvent>('minecraft-playtime-tick', ({ payload }) => {
      const inst = instances.value.find((i) => i.id === payload.instance_id)
      if (inst) {
        inst.playtimeSeconds = payload.instance_playtime_seconds
        inst.playtimeMinutes = Math.floor(payload.instance_playtime_seconds / 60)
        if (payload.instance_last_played) {
          inst.lastPlayed = new Date(payload.instance_last_played * 1000).toISOString()
        }
      }
      globalStats.value.playtimeSeconds = payload.global_playtime_seconds
      globalStats.value.playtimeMinutes = Math.floor(payload.global_playtime_seconds / 60)
      if (payload.global_last_played) {
        globalStats.value.lastPlayed = new Date(payload.global_last_played * 1000).toISOString()
      }
    })

    // 2. Load real data from persistence
    await loadIgdbMetadata()
    await fetchInstances()
    await fetchGlobalStats()
    await loadAccount()
  }

  async function fetchGlobalStats() {
    try {
      const stats = await ipc.getMinecraftGlobalStatsCmd()
      globalStats.value = stats
    } catch (err) {
      console.warn('[MinecraftStore] Could not load global stats:', err)
    }
  }

  function mapProfileModRecord(m: ProfileModRecord, fallbackDescPrefix: string = 'Mod'): MinecraftMod {
    return {
      id: m.id,
      name: m.name,
      version: m.version || m.filename?.replace(/\.(jar|zip)$/i, '') || '1.0.0',
      author: m.author || m.provider,
      enabled: m.is_enabled,
      description: m.description || `${fallbackDescPrefix} ${m.name} (${m.provider})`,
      iconUrl: m.icon_url,
      updateAvailable: m.update_available,
      newVersion: m.new_version,
      newFileId: m.new_file_id,
      newDownloadUrl: m.new_download_url,
      fileName: m.filename || m.file_path,
      fileSizeMb: m.file_size_mb || 0,
      loadPriority: m.load_priority,
      provider: m.provider,
      remoteModId: m.remote_mod_id,
      remoteFileId: m.remote_file_id
    }
  }

  async function fetchInstances() {
    try {
      const records: MinecraftInstanceRecord[] = await ipc.getMinecraftInstancesCmd()
      const loadedList: MinecraftInstance[] = []

      for (const rec of records) {
        const [rawMods, rawResourcePacks, rawShaderPacks] = await Promise.all([
          ipc.getProfileMods(rec.id).catch(() => []),
          ipc.getInstanceResourcePacksCmd(rec.id).catch(() => []),
          ipc.getInstanceShaderPacksCmd(rec.id).catch(() => [])
        ])

        const mappedMods: MinecraftMod[] = rawMods.map((m) => mapProfileModRecord(m, 'Mod'))
        const mappedResourcePacks: MinecraftMod[] = rawResourcePacks.map((m) => mapProfileModRecord(m, 'Pack de ressources'))
        const mappedShaderPacks: MinecraftMod[] = rawShaderPacks.map((m) => mapProfileModRecord(m, 'Shaderpack'))

        // Keep existing logs if instance was already loaded
        const existing = instances.value.find((i) => i.id === rec.id)

        loadedList.push({
          id: rec.id,
          name: rec.name,
          icon: (rec as any).icon || '⛏️',
          bannerUrl: (rec as any).banner_url || null,
          coverUrl: (rec as any).cover_url || null,
          lastPlayed: (rec as any).last_played ? new Date((rec as any).last_played * 1000).toISOString() : existing?.lastPlayed,
          gameVersion: rec.mc_version,
          loader: (rec.modloader as MinecraftLoader) || 'fabric',
          loaderVersion: rec.loader_version || `${rec.modloader} latest`,
          playtimeSeconds: (rec as any).playtime_seconds || 0,
          playtimeMinutes: Math.floor(((rec as any).playtime_seconds || 0) / 60),
          allocatedRamGb: Math.round(rec.max_ram_mb / 1024),
          javaPath: rec.java_path,
          jvmArgs: rec.jvm_args,
          status: existing ? existing.status : 'idle',
          mods: mappedMods,
          worlds: [],
          servers: [],
          resourcePacks: mappedResourcePacks,
          shaderPacks: mappedShaderPacks,
          logs: existing ? existing.logs : []
        })
      }

      instances.value = loadedList
      if (loadedList.length > 0 && (!selectedInstanceId.value || !instances.value.find(i => i.id === selectedInstanceId.value))) {
        selectedInstanceId.value = loadedList[0].id
      }
    } catch (err) {
      console.warn('[MinecraftStore] Could not load instances from DB:', err)
    } finally {
      isLoaded.value = true
    }
  }

  function selectInstance(id: string) {
    selectedInstanceId.value = id
  }

  async function createInstance(params: {
    name: string
    mcVersion?: string
    gameVersion?: string
    modloader?: MinecraftLoader
    loader?: MinecraftLoader
    loaderVersion?: string
    minRamMb?: number
    maxRamMb?: number
    allocatedRamGb?: number
    icon?: string
    javaPath?: string
    jvmArgs?: string[]
  }): Promise<MinecraftInstance> {
    try {
      const ver = params.mcVersion || params.gameVersion || '1.20.1'
      const loader = params.modloader || params.loader || 'fabric'
      const maxRam = params.maxRamMb || (params.allocatedRamGb ? params.allocatedRamGb * 1024 : 4096)
      const minRam = params.minRamMb || 2048

      const rec = await ipc.createMinecraftInstanceCmd({
        name: params.name,
        mc_version: ver,
        modloader: loader,
        loader_version: params.loaderVersion,
        min_ram_mb: minRam,
        max_ram_mb: maxRam,
        java_path: params.javaPath,
        jvm_args: params.jvmArgs || [
          '-XX:+UseG1GC',
          '-XX:+UnlockExperimentalVMOptions',
          '-XX:G1NewSizePercent=20',
          '-XX:G1ReservePercent=20',
          '-XX:MaxGCPauseMillis=50',
          '-XX:G1HeapRegionSize=32M'
        ]
      })

      const newInst: MinecraftInstance = {
        id: rec.id,
        name: rec.name,
        icon: params.icon || '⛏️',
        gameVersion: rec.mc_version,
        loader: rec.modloader as MinecraftLoader,
        loaderVersion: rec.loader_version || `${rec.modloader} latest`,
        playtimeSeconds: 0,
        playtimeMinutes: 0,
        allocatedRamGb: Math.round(rec.max_ram_mb / 1024),
        javaPath: rec.java_path,
        jvmArgs: rec.jvm_args,
        status: 'idle',
        mods: [],
        worlds: [],
        servers: [],
        resourcePacks: [],
        shaderPacks: [],
        logs: [
          {
            timestamp: new Date().toLocaleTimeString(),
            level: 'INFO',
            source: 'LoomLauncher',
            message: `Instance "${rec.name}" (${rec.mc_version} / ${rec.modloader}) initialisée avec succès.`
          }
        ]
      }

      instances.value.unshift(newInst)
      selectedInstanceId.value = newInst.id
      return newInst
    } catch (err) {
      console.error('[MinecraftStore] Error creating instance:', err)
      throw err
    }
  }

  async function reorderMod(instanceId: string, fromIndex: number, toIndex: number) {
    const inst = instances.value.find((i) => i.id === instanceId)
    if (!inst) return
    if (toIndex < 0 || toIndex >= inst.mods.length) return

    const [movedMod] = inst.mods.splice(fromIndex, 1)
    inst.mods.splice(toIndex, 0, movedMod)

    inst.mods.forEach((m, idx) => {
      m.loadPriority = idx
      ipc.reorderMod(instanceId, m.id, idx).catch(() => {})
    })
  }

  async function searchModpacks(
    query: string = '',
    provider: 'all' | 'modrinth' | 'curseforge' = 'all'
  ) {
    isSearchingModpacks.value = true
    try {
      const results = await ipc.searchModpacksCmd(query, provider)
      modpackSearchResults.value = results
    } catch (err) {
      console.error('[MinecraftStore] Error searching modpacks:', err)
      modpackSearchResults.value = []
    } finally {
      isSearchingModpacks.value = false
    }
  }

  async function installModpack(pack: ModpackCatalogItem): Promise<MinecraftInstance> {
    installingModpackIds.value[pack.id] = true
    try {
      const res = await ipc.installModpackCmd({
        provider: pack.source,
        modpackId: pack.id,
        instanceName: pack.title,
        bannerUrl: pack.bannerUrl,
        iconUrl: pack.iconUrl,
        mcVersion: pack.mcVersion,
        modloader: pack.loader
      })
      await fetchInstances()
      const createdId = res?.instance?.id || instances.value[0]?.id
      if (createdId) {
        selectInstance(createdId)
      }
      return activeInstance.value || instances.value[0]
    } finally {
      delete installingModpackIds.value[pack.id]
    }
  }

  async function importLocalModpack(file: File): Promise<MinecraftInstance> {
    const res = await ipc.importLocalModpackCmd(file)
    await fetchInstances()
    const createdId = res?.instance?.id || instances.value[0]?.id
    if (createdId) {
      selectInstance(createdId)
    }
    return activeInstance.value || instances.value[0]
  }

  function getRequiredJavaVersion(mcVersion: string): number {
    const clean = (mcVersion || '').trim()
    const parts = clean.split('.').map((p) => parseInt(p, 10))
    if (parts.length > 0 && !isNaN(parts[0])) {
      if (parts[0] >= 25) return 25
      if (parts[0] === 1) {
        const minor = parts[1] || 0
        const patch = parts[2] || 0
        if (minor < 17) return 8
        if (minor === 17) return 17
        if (minor < 20 || (minor === 20 && patch < 5)) return 17
        if (minor === 20 && patch >= 5) return 21
        if (minor === 21) return 21
        if (minor >= 22) return 25
      }
    }
    const snapMatch = clean.match(/^(\d{2})w/i)
    if (snapMatch) {
      const year = parseInt(snapMatch[1], 10)
      if (year >= 25) return 25
      if (year === 24) return 21
    }
    return 21
  }

  async function fetchDetectedJavaRuntimes() {
    isDetectingJava.value = true
    try {
      const list = await ipc.detectSystemJavaCmd()
      detectedJavaRuntimes.value = list
      return list
    } catch (err) {
      console.warn('[MinecraftStore] Failed to detect Java runtimes:', err)
      return []
    } finally {
      isDetectingJava.value = false
    }
  }

  async function inspectJavaPath(path: string) {
    return await ipc.inspectJavaPathCmd(path)
  }

  async function deleteInstance(instanceId: string) {
    const idx = instances.value.findIndex((i) => i.id === instanceId)
    if (idx !== -1) {
      instances.value.splice(idx, 1)
      if (selectedInstanceId.value === instanceId) {
        selectedInstanceId.value = instances.value[0]?.id || ''
      }
      try {
        await ipc.deleteMinecraftInstanceCmd(instanceId)
      } catch (err) {
        console.warn('[MinecraftStore] deleteInstance error:', err)
      }
    }
  }

  async function launchInstance(instanceId: string) {
    const inst = instances.value.find((i) => i.id === instanceId)
    if (!inst) return
    inst.status = 'running'
    inst.lastPlayed = new Date().toISOString()
    globalStats.value.sessionsCount += 1
    globalStats.value.lastPlayed = new Date().toISOString()
    inst.logs.push({
      timestamp: new Date().toLocaleTimeString(),
      level: 'INFO',
      source: 'LoomLauncher',
      message: `Initialisation du lancement de "${inst.name}" (${inst.gameVersion} / ${inst.loader})...`
    })

    try {
      const settingsStore = useSettingsStore()
      if (settingsStore.settings.autoSyncCloudSaves) {
        const toast = useToastStore()
        toast.show(`Préparation & synchronisation de ${inst.name}...`, 'info')
      }

      if (settingsStore.settings.afterGameLaunch === 'minimize') {
        if (ipc.isTauri()) {
          import('@tauri-apps/api/window').then(({ getCurrentWindow }) => {
            getCurrentWindow().minimize().catch(() => {})
          })
        }
      } else if (settingsStore.settings.afterGameLaunch === 'hide') {
        ipc.hideWindow().catch(() => {})
      }

      await ipc.launchMinecraftInstanceCmd(instanceId, inst.allocatedRamGb)
    } catch (err: any) {
      inst.status = 'idle'
      inst.logs.push({
        timestamp: new Date().toLocaleTimeString(),
        level: 'ERROR',
        source: 'LoomLauncher',
        message: `Erreur critique lors du lancement: ${err?.message || err}`
      })
    }
  }

  async function stopInstance(instanceId: string) {
    const inst = instances.value.find((i) => i.id === instanceId)
    if (!inst) return
    inst.status = 'idle'
    inst.logs.push({
      timestamp: new Date().toLocaleTimeString(),
      level: 'INFO',
      source: 'LoomLauncher',
      message: `Arrêt manuel de l'instance "${inst.name}" demandé...`
    })
    try {
      await ipc.stopMinecraftInstanceCmd(instanceId)
    } catch (err: any) {
      console.warn('[MinecraftStore] Erreur lors de l\'arrêt de l\'instance:', err)
      inst.logs.push({
        timestamp: new Date().toLocaleTimeString(),
        level: 'WARN',
        source: 'LoomLauncher',
        message: `Erreur lors de l'arrêt: ${err?.message || err}`
      })
    }
  }

  async function updateInstanceSettings(
    instanceId: string,
    settings: {
      name?: string
      minRamMb?: number
      maxRamMb?: number
      jvmArgs?: string[]
      javaPath?: string
      bannerUrl?: string | null
      coverUrl?: string | null
      icon?: string | null
    }
  ) {
    const inst = instances.value.find((i) => i.id === instanceId)
    if (inst) {
      if (settings.name && settings.name.trim()) inst.name = settings.name.trim()
      if (settings.maxRamMb) inst.allocatedRamGb = Math.round(settings.maxRamMb / 1024)
      if (settings.javaPath !== undefined) inst.javaPath = settings.javaPath
      if (settings.jvmArgs) inst.jvmArgs = JSON.stringify(settings.jvmArgs)
      if (settings.bannerUrl !== undefined) inst.bannerUrl = settings.bannerUrl
      if (settings.coverUrl !== undefined) inst.coverUrl = settings.coverUrl
      if (settings.icon !== undefined) inst.icon = settings.icon || 'cube'
    }
    await ipc.updateMinecraftInstanceSettingsCmd(
      instanceId,
      settings.minRamMb || 2048,
      settings.maxRamMb || 4096,
      settings.jvmArgs || [],
      settings.javaPath,
      settings.name,
      settings.bannerUrl,
      settings.icon
    )
  }

  async function resetInstanceStatus(instanceId: string) {
    const inst = instances.value.find((i) => i.id === instanceId)
    if (inst) {
      inst.status = 'idle'
      inst.logs.push({
        timestamp: new Date().toLocaleTimeString(),
        level: 'INFO',
        source: 'LoomLauncher',
        message: `Statut de l'instance "${inst.name}" réinitialisé.`
      })
    }
    await ipc.resetMinecraftInstanceCmd(instanceId).catch(() => {})
  }

  // Mods management
  async function toggleMod(instanceId: string, modId: string) {
    const inst = instances.value.find((i) => i.id === instanceId)
    if (!inst) return
    const mod = inst.mods.find((m) => m.id === modId)
    if (mod) {
      mod.enabled = !mod.enabled
      try {
        await ipc.toggleMinecraftModCmd(instanceId, modId, mod.enabled)
      } catch (err) {
        console.warn('[MinecraftStore] toggleMod warning:', err)
      }
    }
  }

  async function deleteMod(instanceId: string, modId: string) {
    const inst = instances.value.find((i) => i.id === instanceId)
    if (!inst) return
    const idx = inst.mods.findIndex((m) => m.id === modId)
    if (idx !== -1) {
      inst.mods.splice(idx, 1)
      try {
        await ipc.deleteMinecraftModCmd(instanceId, modId)
      } catch (err) {
        console.warn('[MinecraftStore] deleteMod warning:', err)
      }
    }
  }

  async function importLocalJar(instanceId: string, file: File | { path: string; name: string }) {
    const inst = instances.value.find((i) => i.id === instanceId)
    if (!inst) return

    const fileName = file.name
    const filePath = (file as any).path || fileName

    try {
      const rec = await ipc.installLocalMinecraftModCmd(instanceId, filePath, fileName)
      const newMod: MinecraftMod = {
        id: rec.id,
        name: rec.name,
        version: '1.0.0',
        author: 'Local',
        enabled: true,
        description: `Fichier local : ${fileName}`,
        fileName,
        fileSizeMb: (file as any).size ? parseFloat(((file as any).size / (1024 * 1024)).toFixed(1)) : 2.0,
        loadPriority: inst.mods.length,
        provider: 'local'
      }
      inst.mods.unshift(newMod)
    } catch (err) {
      console.error('[MinecraftStore] Error importing local mod:', err)
    }
  }

  async function toggleResourcePack(instanceId: string, packId: string, enabled: boolean) {
    const inst = instances.value.find((i) => i.id === instanceId)
    if (!inst) return
    const target = inst.resourcePacks.find((p) => p.id === packId || p.fileName === packId)
    if (target) {
      target.enabled = enabled
    }
    await ipc.toggleResourcePackCmd(instanceId, packId, enabled)
    await refreshInstanceResourcePacks(instanceId)
  }

  async function deleteResourcePack(instanceId: string, packId: string) {
    const inst = instances.value.find((i) => i.id === instanceId)
    if (!inst) return
    const idx = inst.resourcePacks.findIndex((p) => p.id === packId || p.fileName === packId)
    if (idx !== -1) {
      inst.resourcePacks.splice(idx, 1)
      try {
        await ipc.deleteResourcePackCmd(instanceId, packId)
      } catch (err) {
        console.warn('[MinecraftStore] deleteResourcePack warning:', err)
      }
    }
  }

  async function importLocalResourcePack(instanceId: string, file: File | { path: string; name: string }) {
    const inst = instances.value.find((i) => i.id === instanceId)
    if (!inst) return
    const fileName = file.name
    try {
      await ipc.installLocalResourcePackCmd(instanceId, fileName)
      await refreshInstanceResourcePacks(instanceId)
    } catch (err) {
      console.error('[MinecraftStore] Error importing local resourcepack:', err)
    }
  }

  async function toggleShaderPack(instanceId: string, packId: string, enabled: boolean) {
    const inst = instances.value.find((i) => i.id === instanceId)
    if (!inst) return
    const target = inst.shaderPacks.find((p) => p.id === packId || p.fileName === packId)
    if (target) {
      target.enabled = enabled
    }
    await ipc.toggleShaderPackCmd(instanceId, packId, enabled)
    await refreshInstanceShaderPacks(instanceId)
  }

  async function deleteShaderPack(instanceId: string, packId: string) {
    const inst = instances.value.find((i) => i.id === instanceId)
    if (!inst) return
    const idx = inst.shaderPacks.findIndex((p) => p.id === packId || p.fileName === packId)
    if (idx !== -1) {
      inst.shaderPacks.splice(idx, 1)
      try {
        await ipc.deleteShaderPackCmd(instanceId, packId)
      } catch (err) {
        console.warn('[MinecraftStore] deleteShaderPack warning:', err)
      }
    }
  }

  async function importLocalShaderPack(instanceId: string, file: File | { path: string; name: string }) {
    const inst = instances.value.find((i) => i.id === instanceId)
    if (!inst) return
    const fileName = file.name
    try {
      await ipc.installLocalShaderPackCmd(instanceId, fileName)
      await refreshInstanceShaderPacks(instanceId)
    } catch (err) {
      console.error('[MinecraftStore] Error importing local shaderpack:', err)
    }
  }

  async function openInstanceFolder(instanceId: string, subDir?: string) {
    try {
      await ipc.openMinecraftInstanceFolder(instanceId, subDir)
    } catch (err) {
      console.warn('[MinecraftStore] Could not open instance folder:', err)
    }
  }

  // Saves / Maps
  async function getInstanceSaves(instanceId: string) {
    return await ipc.getInstanceSavesCmd(instanceId)
  }

  async function backupInstanceSave(instanceId: string, folderName: string) {
    return await ipc.backupInstanceSaveCmd(instanceId, folderName)
  }

  async function deleteInstanceSave(instanceId: string, folderName: string) {
    return await ipc.deleteInstanceSaveCmd(instanceId, folderName)
  }

  async function openInstanceSaveFolder(instanceId: string, folderName: string) {
    return await ipc.openInstanceSaveFolderCmd(instanceId, folderName)
  }

  // Servers
  async function getInstanceServers(instanceId: string) {
    return await ipc.getInstanceServersCmd(instanceId)
  }

  async function saveInstanceServers(instanceId: string, servers: any[]) {
    return await ipc.saveInstanceServersCmd(instanceId, servers)
  }

  async function deleteInstanceServer(instanceId: string, serverId: string) {
    return await ipc.deleteInstanceServerCmd(instanceId, serverId)
  }

  async function pingMinecraftServer(ip: string, port?: number) {
    return await ipc.pingMinecraftServerCmd(ip, port)
  }

  // Logs
  async function getInstanceLogFiles(instanceId: string) {
    return await ipc.getInstanceLogFilesCmd(instanceId)
  }

  async function readInstanceLogContent(instanceId: string, file: string, isCrash?: boolean) {
    return await ipc.readInstanceLogContentCmd(instanceId, file, isCrash)
  }

  async function clearDiskLogs(instanceId: string) {
    return await ipc.clearInstanceLogsCmd(instanceId)
  }

  // Configs
  async function getInstanceConfigs(instanceId: string) {
    return await ipc.getInstanceConfigsCmd(instanceId)
  }

  async function readInstanceConfig(instanceId: string, relPath: string) {
    return await ipc.readInstanceConfigCmd(instanceId, relPath)
  }

  async function saveInstanceConfig(instanceId: string, relPath: string, content: string) {
    return await ipc.saveInstanceConfigCmd(instanceId, relPath, content)
  }

  async function deleteInstanceConfig(instanceId: string, relPath: string) {
    return await ipc.deleteInstanceConfigCmd(instanceId, relPath)
  }

  async function searchContent(
    query: string,
    contentType: 'mod' | 'resourcepack' | 'shader' = 'mod',
    provider: 'all' | 'modrinth' | 'curseforge' = 'all'
  ) {
    isSearchingMods.value = true
    try {
      const inst = activeInstance.value
      const mcVer = inst?.gameVersion || '1.20.1'
      const loader = inst?.loader || 'fabric'

      const results = await ipc.searchUnifiedMods(
        query,
        'minecraft',
        provider !== 'all' ? [provider] : ['all'],
        mcVer,
        contentType === 'mod' && loader !== 'vanilla' ? loader : undefined,
        contentType
      )
      modSearchResults.value = results
    } catch (err) {
      console.error('[MinecraftStore] Error searching content:', err)
      modSearchResults.value = []
    } finally {
      isSearchingMods.value = false
    }
  }

  async function searchMods(query: string, provider: 'all' | 'modrinth' | 'curseforge' = 'all') {
    return searchContent(query, 'mod', provider)
  }

  const isCheckingUpdates = ref(false)
  const updatingModIds = ref<Record<string, boolean>>({})

  async function checkModUpdates(instanceId: string) {
    isCheckingUpdates.value = true
    try {
      await ipc.checkMinecraftModUpdates(instanceId)
      await refreshInstanceMods(instanceId)
    } catch (err) {
      console.error('[MinecraftStore] Error checking mod updates:', err)
    } finally {
      isCheckingUpdates.value = false
    }
  }

  async function updateMod(instanceId: string, modId: string, fileId?: string, downloadUrl?: string) {
    updatingModIds.value[modId] = true
    try {
      await ipc.updateMinecraftMod(instanceId, modId, fileId, downloadUrl)
      await refreshInstanceMods(instanceId)
    } catch (err) {
      console.error('[MinecraftStore] Error updating mod:', err)
      throw err
    } finally {
      updatingModIds.value[modId] = false
    }
  }

  async function getModDetails(provider: string, modId: string) {
    return await ipc.getMinecraftModDetails(provider, modId)
  }

  async function getVersionChangelog(provider: string, modId: string, fileId: string) {
    const res = await ipc.getMinecraftVersionChangelog(provider, modId, fileId)
    return res.changelog || ''
  }

  async function refreshInstanceMods(instanceId: string) {
    const inst = instances.value.find((i) => i.id === instanceId)
    if (!inst) return
    try {
      const dbMods = await ipc.getProfileMods(instanceId)
      inst.mods = dbMods.map((m: any) => mapProfileModRecord(m, 'Mod'))
    } catch (err) {
      console.warn('[MinecraftStore] Could not refresh mods for instance:', instanceId, err)
    }
  }

  async function refreshInstanceResourcePacks(instanceId: string) {
    const inst = instances.value.find((i) => i.id === instanceId)
    if (!inst) return
    try {
      const dbPacks = await ipc.getInstanceResourcePacksCmd(instanceId)
      inst.resourcePacks = dbPacks.map((m: any) => mapProfileModRecord(m, 'Pack de ressources'))
    } catch (err) {
      console.warn('[MinecraftStore] Could not refresh resourcepacks for instance:', instanceId, err)
    }
  }

  async function refreshInstanceShaderPacks(instanceId: string) {
    const inst = instances.value.find((i) => i.id === instanceId)
    if (!inst) return
    try {
      const dbShaders = await ipc.getInstanceShaderPacksCmd(instanceId)
      inst.shaderPacks = dbShaders.map((m: any) => mapProfileModRecord(m, 'Shaderpack'))
    } catch (err) {
      console.warn('[MinecraftStore] Could not refresh shaderpacks for instance:', instanceId, err)
    }
  }

  async function executeDirectModInstall(
    instanceId: string,
    mod: { id: string; provider: string; name: string; slug?: string; summary?: string; icon_url?: string; latest_version?: string; project_type?: 'mod' | 'resourcepack' | 'shader' }
  ) {
    const inst = instances.value.find((i) => i.id === instanceId)
    if (!inst) return

    const contentType = mod.project_type || 'mod'
    const ext = contentType === 'mod' ? '.jar' : '.zip'
    const safeFilename = `${mod.slug || mod.id}${ext}`

    if (contentType === 'resourcepack') {
      await fetch(`/api/minecraft/instances/${instanceId}/resourcepacks/install`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          provider: mod.provider,
          remoteModId: mod.id,
          remoteFileId: mod.latest_version || mod.id,
          filename: safeFilename,
          modName: mod.name,
          iconUrl: mod.icon_url,
          mcVersion: inst.gameVersion
        })
      }).catch(() => {})
      await refreshInstanceResourcePacks(instanceId)
    } else if (contentType === 'shader') {
      await fetch(`/api/minecraft/instances/${instanceId}/shaderpacks/install`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          provider: mod.provider,
          remoteModId: mod.id,
          remoteFileId: mod.latest_version || mod.id,
          filename: safeFilename,
          modName: mod.name,
          iconUrl: mod.icon_url,
          mcVersion: inst.gameVersion
        })
      }).catch(() => {})
      await refreshInstanceShaderPacks(instanceId)
    } else {
      await ipc.installMinecraftModCmd({
        instanceId,
        provider: mod.provider,
        remoteModId: mod.id,
        remoteFileId: mod.latest_version || mod.id,
        downloadUrl: undefined,
        filename: safeFilename,
        modName: mod.name,
        iconUrl: mod.icon_url,
        mcVersion: inst.gameVersion,
        modloader: inst.loader
      })
      await refreshInstanceMods(instanceId)
    }
  }

  async function installModFromCatalog(instanceId: string, mod: UnifiedModSearchResult) {
    const inst = instances.value.find((i) => i.id === instanceId)
    if (!inst) return

    installingModIds.value[mod.id] = true
    isResolvingDeps.value = true

    try {
      const contentType = mod.project_type || 'mod'

      // Check Java mod dependencies only if installing a Java mod
      if (contentType === 'mod') {
        const depCheck = await ipc.checkMinecraftModDependenciesCmd(
          instanceId,
          mod.provider,
          mod.id
        )

        if (depCheck?.requiredDependencies && depCheck.requiredDependencies.length > 0) {
          pendingModToInstall.value = mod
          targetInstanceId.value = instanceId
          detectedDependencies.value = depCheck.requiredDependencies
          selectedDependencies.value = depCheck.requiredDependencies.map((d: any) => d.id || d.slug)
          dependencyModalOpen.value = true
          return
        }
      }

      // Direct install
      await executeDirectModInstall(instanceId, mod)
    } catch (err) {
      console.error('[MinecraftStore] Failed to install content:', err)
      await executeDirectModInstall(instanceId, mod).catch(() => {})
    } finally {
      installingModIds.value[mod.id] = false
      isResolvingDeps.value = false
    }
  }

  async function confirmCascadeInstall() {
    if (!pendingModToInstall.value || !targetInstanceId.value) return
    isBatchInstalling.value = true
    const parentMod = pendingModToInstall.value
    const instanceId = targetInstanceId.value

    try {
      // 1. Install each selected dependency for this instance's version and loader
      for (const depId of selectedDependencies.value) {
        const dep = detectedDependencies.value.find((d: any) => d.id === depId || d.slug === depId)
        if (dep) {
          await executeDirectModInstall(instanceId, {
            id: dep.id || dep.slug,
            slug: dep.slug || dep.id,
            provider: dep.provider || parentMod.provider,
            name: dep.name,
            summary: dep.summary,
            icon_url: dep.icon_url
          })
        }
      }

      // 2. Install target mod
      await executeDirectModInstall(instanceId, parentMod)

      await refreshInstanceMods(instanceId)
      closeDependencyModal()
    } catch (err) {
      console.error('[MinecraftStore] Cascade install error:', err)
    } finally {
      isBatchInstalling.value = false
      if (parentMod?.id) {
        installingModIds.value[parentMod.id] = false
      }
    }
  }

  function closeDependencyModal() {
    dependencyModalOpen.value = false
    if (pendingModToInstall.value?.id) {
      installingModIds.value[pendingModToInstall.value.id] = false
    }
    pendingModToInstall.value = null
    targetInstanceId.value = ''
    detectedDependencies.value = []
    selectedDependencies.value = []
    isBatchInstalling.value = false
    isResolvingDeps.value = false
  }

  function clearLogs(instanceId: string) {
    const inst = instances.value.find((i) => i.id === instanceId)
    if (inst) {
      inst.logs = []
    }
  }

  // Microsoft Authentication
  async function loadAccount() {
    try {
      account.value = await ipc.getMinecraftAuth()
    } catch (e) {
      console.warn('[MinecraftStore] Could not load stored account:', e)
    }
  }

  async function startMicrosoftLogin() {
    authError.value = null
    deviceCodeInfo.value = null
    isAuthPolling.value = false
    if (authPollTimer) {
      clearInterval(authPollTimer)
      authPollTimer = null
    }

    authModalOpen.value = true

    try {
      const codeInfo = await ipc.initiateMicrosoftAuth()
      if (!codeInfo?.user_code || !codeInfo?.device_code) {
        throw new Error('Code de périphérique invalide ou non reçu de Microsoft.')
      }
      deviceCodeInfo.value = codeInfo
      isAuthPolling.value = true

      const pollInterval = Math.max(3000, (codeInfo.interval || 5) * 1000)

      authPollTimer = setInterval(async () => {
        if (!authModalOpen.value || !deviceCodeInfo.value) {
          closeAuthModal()
          return
        }

        try {
          const res = await ipc.pollMicrosoftAuth(deviceCodeInfo.value.device_code)
          if (!res.pending) {
            if (res.account) {
              account.value = res.account
              isAuthPolling.value = false
              if (authPollTimer) {
                clearInterval(authPollTimer)
                authPollTimer = null
              }
              setTimeout(() => {
                authModalOpen.value = false
              }, 1200)
            } else if (res.error) {
              authError.value = res.error
              isAuthPolling.value = false
              if (authPollTimer) {
                clearInterval(authPollTimer)
                authPollTimer = null
              }
            }
          }
        } catch (err: any) {
          console.warn('[MinecraftStore] Polling tick error:', err)
        }
      }, pollInterval)
    } catch (err: any) {
      console.error('[MinecraftStore] Failed to initiate MS auth:', err)
      authError.value = err?.message || 'Impossible d\'initier la connexion Microsoft.'
      isAuthPolling.value = false
    }
  }

  function closeAuthModal() {
    authModalOpen.value = false
    isAuthPolling.value = false
    if (authPollTimer) {
      clearInterval(authPollTimer)
      authPollTimer = null
    }
  }

  async function logoutMicrosoft() {
    await ipc.logoutMinecraftAuth()
    account.value = null
  }

  // Call init
  init()

  return {
    init,
    instances,
    selectedInstanceId,
    activeInstance,
    lastPlayedInstance,
    igdbGame,
    defaultCoverUrl,
    defaultBannerUrl,
    igdbSummary,
    igdbRating,
    loadIgdbMetadata,
    globalStats,
    globalPlaytimeFormatted,
    globalLastPlayedFormatted,
    formatPlaytime,
    formatPlaytimeCompact,
    formatLastPlayed,
    fetchGlobalStats,
    isLoaded,
    modSearchResults,
    isSearchingMods,
    installingModIds,
    modpackSearchResults,
    isSearchingModpacks,
    installingModpackIds,
    account,
    deviceCodeInfo,
    isAuthPolling,
    authModalOpen,
    authError,
    fetchInstances,
    selectInstance,
    createInstance,
    deleteInstance,
    launchInstance,
    stopInstance,
    resetInstanceStatus,
    openInstanceFolder,
    updateInstanceSettings,
    toggleMod,
    deleteMod,
    importLocalJar,
    importLocalFile: importLocalJar,
    toggleResourcePack,
    deleteResourcePack,
    importLocalResourcePack,
    refreshInstanceResourcePacks,
    toggleShaderPack,
    deleteShaderPack,
    importLocalShaderPack,
    refreshInstanceShaderPacks,
    reorderMod,
    installModpack,
    searchModpacks,
    importLocalModpack,
    detectedJavaRuntimes,
    isDetectingJava,
    getRequiredJavaVersion,
    fetchDetectedJavaRuntimes,
    inspectJavaPath,
    searchMods,
    searchContent,
    installModFromCatalog,
    clearLogs,
    getInstanceSaves,
    backupInstanceSave,
    deleteInstanceSave,
    openInstanceSaveFolder,
    getInstanceServers,
    saveInstanceServers,
    deleteInstanceServer,
    pingMinecraftServer,
    getInstanceLogFiles,
    readInstanceLogContent,
    clearDiskLogs,
    getInstanceConfigs,
    readInstanceConfig,
    saveInstanceConfig,
    deleteInstanceConfig,
    refreshInstanceMods,
    isCheckingUpdates,
    updatingModIds,
    checkModUpdates,
    updateMod,
    getModDetails,
    getVersionChangelog,
    dependencyModalOpen,
    pendingModToInstall,
    detectedDependencies,
    selectedDependencies,
    isResolvingDeps,
    isBatchInstalling,
    confirmCascadeInstall,
    closeDependencyModal,
    startMicrosoftLogin,
    closeAuthModal,
    logoutMicrosoft
  }
})
