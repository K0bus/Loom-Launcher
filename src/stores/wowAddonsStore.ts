import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { useToastStore } from '@/stores/toastStore'
import { ipc } from '@/services/ipc'
import type {
  WowInstance,
  InstalledWowAddon,
  WowAddonSearchResult,
  WowAddonFullDetails,
  WowAddonCategory
} from '@/models/wowAddon'

export const WOW_CATEGORIES: WowAddonCategory[] = [
  { id: 0, name: 'Tous', slug: 'all' },
  { id: 1019, name: 'Combat & Dégâts', slug: 'combat' },
  { id: 1014, name: 'Boss & Raids (DBM)', slug: 'boss-encounters' },
  { id: 1013, name: 'Quêtes & XP (Questie)', slug: 'quests-leveling' },
  { id: 1009, name: 'Sacs & Inventaire', slug: 'bags-inventory' },
  { id: 1018, name: 'Barres d\'action & UI', slug: 'action-bars' },
  { id: 1011, name: 'Carte & Mini-map', slug: 'map-minimap' },
  { id: 1002, name: 'Hôtel des Ventes (AH)', slug: 'auction-economy' },
  { id: 1005, name: 'Buffs & Debuffs', slug: 'buffs-debuffs' },
  { id: 1015, name: 'Métiers & Artisanat', slug: 'professions' },
  { id: 1001, name: 'Chat & Social', slug: 'chat-communication' },
  { id: 1010, name: 'Bibliothèques (Libs)', slug: 'libraries' }
]

export const useWowAddonsStore = defineStore('wowAddons', () => {
  // Instances
  const instances = ref<WowInstance[]>([])
  const selectedInstanceId = ref<string>('forever')
  const isLoadingInstances = ref<boolean>(false)

  // Installed Addons
  const installedAddons = ref<InstalledWowAddon[]>([])
  const installedFilter = ref<string>('')
  const isLoadingInstalled = ref<boolean>(false)
  const isCheckingUpdates = ref<boolean>(false)

  // Catalog
  const catalogAddons = ref<WowAddonSearchResult[]>([])
  const searchQuery = ref<string>('')
  const selectedCategory = ref<number | null>(null)
  const sortField = ref<number>(2) // 2 = Popularity, 3 = Updated, 1 = Featured, 4 = Name
  const isLoadingCatalog = ref<boolean>(false)

  // Modals & Details
  const detailsModalOpen = ref<boolean>(false)
  const selectedAddonForDetails = ref<WowAddonFullDetails | null>(null)
  const detailsFiles = ref<any[]>([])
  const isLoadingDetails = ref<boolean>(false)

  const toast = useToastStore()

  // Tracking operations in-flight
  const installingMap = ref<Record<string | number, boolean>>({})
  const uninstallingMap = ref<Record<string, boolean>>({})
  const togglingMap = ref<Record<string, boolean>>({})
  const isUpdatingAll = ref<boolean>(false)
  const isExporting = ref<boolean>(false)

  // Active sub-tab
  const activeTab = ref<'installed' | 'catalog'>('installed')

  // Getters
  const selectedInstance = computed(() => {
    return (
      instances.value.find((i) => i.id === selectedInstanceId.value) ||
      instances.value.find((i) => i.installed) ||
      instances.value[0] ||
      null
    )
  })

  const installedCount = computed(() => installedAddons.value.length)
  const totalFoldersCount = computed(() => {
    return installedAddons.value.reduce((acc, curr) => acc + 1 + (curr.secondaryFolders?.length || 0), 0)
  })
  const enabledCount = computed(() => installedAddons.value.filter((a) => a.enabled).length)
  const updatesAvailableCount = computed(() => installedAddons.value.filter((a) => a.updateAvailable).length)

  const filteredInstalledAddons = computed(() => {
    const q = installedFilter.value.trim().toLowerCase()
    if (!q) return installedAddons.value
    return installedAddons.value.filter(
      (a) =>
        a.title.toLowerCase().includes(q) ||
        a.folderName.toLowerCase().includes(q) ||
        (a.author && a.author.toLowerCase().includes(q)) ||
        (a.notes && a.notes.toLowerCase().includes(q))
    )
  })

  // Helper to construct fallback CDN download URL if missing
  function buildCdnDownloadUrl(fileId: number, fileName: string): string {
    const p1 = Math.floor(fileId / 1000)
    const p2 = fileId % 1000
    return `https://edge.forgecdn.net/files/${p1}/${p2}/${encodeURIComponent(fileName)}`
  }

  // Load instances from local detection
  async function loadInstances(force = false) {
    if (instances.value.length > 0 && !force) return
    isLoadingInstances.value = true
    try {
      const data = await ipc.scanWowInstances()
      instances.value = data
      // Select first installed instance by default
      const active = data.find((i) => i.installed)
      if (active && !instances.value.some((i) => i.id === selectedInstanceId.value && i.installed)) {
        selectedInstanceId.value = active.id
      }
    } catch (err) {
      console.warn('[WowAddonsStore] Failed to load instances:', err)
    } finally {
      isLoadingInstances.value = false
    }
  }

  function selectInstance(id: string) {
    if (selectedInstanceId.value === id) return
    selectedInstanceId.value = id
    loadInstalledAddons(true)
    if (activeTab.value === 'catalog') {
      searchCatalog()
    }
  }

  // Load installed addons for current instance
  async function loadInstalledAddons(checkUpdates = true) {
    if (!selectedInstance.value) return
    isLoadingInstalled.value = true
    if (checkUpdates) isCheckingUpdates.value = true

    try {
      const data = await ipc.getWowAddons(
        selectedInstance.value.path,
        checkUpdates,
        selectedInstance.value.gameVersionTypeId
      )
      installedAddons.value = data

      // Update addonsCount on selected instance
      if (selectedInstance.value) {
        selectedInstance.value.addonsCount = data.length
      }
    } catch (err) {
      console.error('[WowAddonsStore] Failed to load installed addons:', err)
    } finally {
      isLoadingInstalled.value = false
      isCheckingUpdates.value = false
    }
  }

  // Helper to pick best file for instance flavor
  function getBestFileForInstance(files: any[], targetTypeId?: number): any {
    if (!files || files.length === 0) return null
    const typeId = targetTypeId || selectedInstance.value?.gameVersionTypeId

    // Sort files by id descending to prefer newest releases
    const sorted = [...files].sort((a, b) => (b.id || 0) - (a.id || 0))

    if (typeId) {
      const match = sorted.find((f) => {
        const vTypes = f.sortableGameVersions?.map((v: any) => v.gameVersionTypeId) || []
        return vTypes.includes(typeId)
      })
      if (match) return match

      // If looking for Forever (88568), fallback to Classic (67408)
      if (typeId === 88568) {
        const classicMatch = sorted.find((f) => {
          const vTypes = f.sortableGameVersions?.map((v: any) => v.gameVersionTypeId) || []
          return vTypes.includes(67408)
        })
        if (classicMatch) return classicMatch
      }
    }

    // Default to the first release or first available file
    return sorted.find((f) => f.releaseType === 1) || sorted[0]
  }

  // Search catalog from CurseForge
  async function searchCatalog(_resetPage = true) {
    isLoadingCatalog.value = true
    try {
      const res = await ipc.searchWowAddons({
        query: searchQuery.value.trim() || undefined,
        categoryId: (selectedCategory.value && selectedCategory.value > 0) ? selectedCategory.value : undefined,
        gameVersionTypeId: selectedInstance.value?.gameVersionTypeId || undefined,
        sortField: sortField.value || undefined,
        pageSize: 36
      }).catch((err) => {
        console.warn('[WowAddonsStore] searchWowAddons IPC error:', err)
        return { data: [] }
      })

      const rawMods = res?.data || []

      const mapped: WowAddonSearchResult[] = rawMods.map((m: any) => {
        const isInst = installedAddons.value.some(
          (inst) => inst.curseforgeId === m.id || inst.folderName.toLowerCase() === m.slug.toLowerCase()
        )
        const instMatch = installedAddons.value.find(
          (inst) => inst.curseforgeId === m.id || inst.folderName.toLowerCase() === m.slug.toLowerCase()
        )

        return {
          id: m.id,
          gameId: m.gameId,
          name: m.name,
          slug: m.slug,
          summary: m.summary || '',
          downloadCount: m.downloadCount || 0,
          websiteUrl: m.links?.websiteUrl || `https://www.curseforge.com/wow/addons/${m.slug}`,
          iconUrl: m.logo?.thumbnailUrl || m.logo?.url || '',
          author: m.authors?.[0]?.name || 'Communauté',
          authors: m.authors || [],
          categories: m.categories || [],
          latestFiles: m.latestFiles || [],
          dateModified: m.dateModified,
          isInstalled: isInst,
          installedVersion: instMatch?.version,
          updateAvailable: instMatch?.updateAvailable
        }
      })

      catalogAddons.value = mapped
    } catch (err) {
      console.error('[WowAddonsStore] Error searching catalog:', err)
      catalogAddons.value = []
    } finally {
      isLoadingCatalog.value = false
    }
  }

  // Open addon details modal
  async function openDetailsModal(addon: WowAddonSearchResult | InstalledWowAddon) {
    detailsModalOpen.value = true
    isLoadingDetails.value = true
    detailsFiles.value = []

    let modId = (addon as any).curseforgeId || (addon as any).id
    if (!modId && (addon as InstalledWowAddon).folderName) {
      // Find in catalog or use folder
      const found = catalogAddons.value.find((c) => c.slug === (addon as InstalledWowAddon).folderName.toLowerCase())
      if (found) modId = found.id
    }

    if (!modId) {
      isLoadingDetails.value = false
      return
    }

    try {
      // Fetch mod details
      const detailsRes = await ipc.getWowAddonDetails(modId).catch(() => null)
      const m = detailsRes?.data
      if (m) {
        const instMatch = installedAddons.value.find((inst) => inst.curseforgeId === m.id)
        selectedAddonForDetails.value = {
          id: m.id,
          gameId: m.gameId,
          name: m.name,
          slug: m.slug,
          summary: m.summary || '',
          downloadCount: m.downloadCount || 0,
          websiteUrl: m.links?.websiteUrl,
          iconUrl: m.logo?.thumbnailUrl || m.logo?.url,
          author: m.authors?.[0]?.name || '',
          authors: m.authors || [],
          categories: m.categories || [],
          latestFiles: m.latestFiles || [],
          dateModified: m.dateModified,
          screenshots: m.screenshots || [],
          isInstalled: !!instMatch,
          installedVersion: instMatch?.version,
          updateAvailable: instMatch?.updateAvailable
        }
      }

      // Fetch description HTML
      const descRes = await ipc.getWowAddonDescription(modId).catch(() => null)
      if (selectedAddonForDetails.value && descRes?.data) {
        selectedAddonForDetails.value.descriptionHtml = descRes.data
      }

      // Fetch mod files
      const filesRes = await ipc.getWowAddonFiles(modId, 20).catch(() => null)
      if (filesRes?.data) {
        detailsFiles.value = filesRes.data
      }
    } catch (err) {
      console.warn('[WowAddonsStore] Failed loading addon details:', err)
    } finally {
      isLoadingDetails.value = false
    }
  }

  function closeDetailsModal() {
    detailsModalOpen.value = false
    selectedAddonForDetails.value = null
    detailsFiles.value = []
  }

  // Install addon
  async function installAddon(params: {
    modId: number
    fileId?: number
    downloadUrl?: string
    modName: string
    version?: string
    iconUrl?: string
  }): Promise<boolean> {
    if (!selectedInstance.value) return false
    const key = params.modId

    installingMap.value[key] = true
    try {
      let dlUrl = params.downloadUrl
      let finalFileId = params.fileId
      let finalVer = params.version || 'Latest'

      // If downloadUrl is not provided, fetch files to pick the best file
      if (!dlUrl) {
        const fRes = await ipc.getWowAddonFiles(params.modId, 15).catch(() => null)
        const files = fRes?.data || []
        const best = getBestFileForInstance(files)
        if (best) {
          finalFileId = best.id
          finalVer = best.displayName || best.fileName || finalVer
          dlUrl = best.downloadUrl || buildCdnDownloadUrl(best.id, best.fileName)
        }
      }

      if (!dlUrl) {
        throw new Error("Lien de téléchargement de l'addon introuvable")
      }

      await ipc.installWowAddon({
        instancePath: selectedInstance.value.path,
        downloadUrl: dlUrl,
        modId: params.modId,
        fileId: finalFileId,
        modName: params.modName,
        version: finalVer,
        iconUrl: params.iconUrl
      })

      // Refresh installed addons and update status
      await loadInstalledAddons(true)

      // Sync state in catalog search results
      const catItem = catalogAddons.value.find((c) => c.id === params.modId)
      if (catItem) {
        catItem.isInstalled = true
        catItem.installedVersion = finalVer
        catItem.updateAvailable = false
      }

      if (selectedAddonForDetails.value && selectedAddonForDetails.value.id === params.modId) {
        selectedAddonForDetails.value.isInstalled = true
        selectedAddonForDetails.value.installedVersion = finalVer
        selectedAddonForDetails.value.updateAvailable = false
      }

      return true
    } catch (err: any) {
      console.error('[WowAddonsStore] Install failed:', err)
      alert(`Erreur d'installation : ${err?.message || err}`)
      return false
    } finally {
      installingMap.value[key] = false
    }
  }

  // Update a single addon
  async function updateAddon(addon: InstalledWowAddon): Promise<boolean> {
    if (!addon.curseforgeId) return false
    return installAddon({
      modId: addon.curseforgeId,
      fileId: addon.latestFileId,
      downloadUrl: addon.downloadUrl,
      modName: addon.title,
      version: addon.latestVersion,
      iconUrl: addon.iconUrl
    })
  }

  // Update all addons with pending updates
  async function updateAllAddons(): Promise<void> {
    const toUpdate = installedAddons.value.filter((a) => a.updateAvailable && a.curseforgeId)
    if (toUpdate.length === 0) return

    isUpdatingAll.value = true
    try {
      for (const a of toUpdate) {
        await updateAddon(a)
      }
    } finally {
      isUpdatingAll.value = false
      await loadInstalledAddons(true)
    }
  }

  // Uninstall addon
  async function uninstallAddon(addon: InstalledWowAddon): Promise<boolean> {
    if (!selectedInstance.value) return false
    uninstallingMap.value[addon.folderName] = true

    try {
      await ipc.uninstallWowAddon(
        selectedInstance.value.path,
        addon.folderName,
        addon.secondaryFolders || []
      )

      installedAddons.value = installedAddons.value.filter((a) => a.folderName !== addon.folderName)

      // Update catalog state
      const catItem = catalogAddons.value.find(
        (c) => c.id === addon.curseforgeId || c.slug === addon.folderName.toLowerCase()
      )
      if (catItem) {
        catItem.isInstalled = false
        catItem.installedVersion = undefined
        catItem.updateAvailable = false
      }

      if (selectedAddonForDetails.value && selectedAddonForDetails.value.id === addon.curseforgeId) {
        selectedAddonForDetails.value.isInstalled = false
        selectedAddonForDetails.value.installedVersion = undefined
        selectedAddonForDetails.value.updateAvailable = false
      }

      return true
    } catch (err: any) {
      console.error('[WowAddonsStore] Uninstall failed:', err)
      alert(`Erreur de désinstallation : ${err?.message || err}`)
      return false
    } finally {
      uninstallingMap.value[addon.folderName] = false
    }
  }

  // Toggle enable / disable
  async function toggleAddon(addon: InstalledWowAddon): Promise<boolean> {
    if (!selectedInstance.value) return false
    const nextState = !addon.enabled
    togglingMap.value[addon.folderName] = true

    try {
      const ok = await ipc.toggleWowAddon(
        selectedInstance.value.path,
        addon.folderName,
        addon.secondaryFolders || [],
        nextState
      )

      if (ok) {
        addon.enabled = nextState
        return true
      }
      return false
    } catch (err) {
      console.error('[WowAddonsStore] Toggle failed:', err)
      return false
    } finally {
      togglingMap.value[addon.folderName] = false
    }
  }

  // Open directory in native file manager
  async function openAddonsFolder(): Promise<void> {
    if (!selectedInstance.value?.addonsPath) return
    try {
      await ipc.openWowAddonsFolder(selectedInstance.value.addonsPath)
    } catch (err) {
      console.warn('[WowAddonsStore] Open folder error:', err)
    }
  }

  // Export installed addons & WTF configuration as backup zip
  async function exportBackup(): Promise<boolean> {
    if (!selectedInstance.value?.path) {
      toast.show('Aucune instance World of Warcraft sélectionnée', 'error')
      return false
    }

    const inst = selectedInstance.value
    isExporting.value = true
    toast.show(`Préparation de l'export (${inst.name})...`, 'info', 3000)

    try {
      const zipPath = await ipc.exportWowAddons(inst.path, inst.flavor)
      toast.show(`Configuration et addons exportés dans : ${zipPath}`, 'success', 5000)
      return true
    } catch (err: any) {
      console.error('[WowAddonsStore] exportBackup error:', err)
      toast.show(`Erreur lors de l'exportation : ${err?.message || 'Erreur inconnue'}`, 'error')
      return false
    } finally {
      isExporting.value = false
    }
  }

  return {
    // State
    instances,
    selectedInstanceId,
    isLoadingInstances,
    installedAddons,
    installedFilter,
    isLoadingInstalled,
    isCheckingUpdates,
    catalogAddons,
    searchQuery,
    selectedCategory,
    sortField,
    isLoadingCatalog,
    detailsModalOpen,
    selectedAddonForDetails,
    detailsFiles,
    isLoadingDetails,
    installingMap,
    uninstallingMap,
    togglingMap,
    isUpdatingAll,
    isExporting,
    activeTab,

    // Getters
    selectedInstance,
    installedCount,
    totalFoldersCount,
    enabledCount,
    updatesAvailableCount,
    filteredInstalledAddons,

    // Actions
    loadInstances,
    selectInstance,
    loadInstalledAddons,
    searchCatalog,
    openDetailsModal,
    closeDetailsModal,
    getBestFileForInstance,
    installAddon,
    updateAddon,
    updateAllAddons,
    uninstallAddon,
    toggleAddon,
    openAddonsFolder,
    exportBackup
  }
})
