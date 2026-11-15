import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type {
  UnifiedModSearchResult,
  RemoteDependency,
  GameModdingManifest
} from '@/models/modding'
import { ipc } from '@/services/ipc'

export interface ModDownloadProgress {
  id: string
  name: string
  progress: number
  status: 'pending' | 'downloading' | 'completed' | 'error'
  error?: string
}

export const useModdingStore = defineStore('modding', () => {
  // State
  const searchQuery = ref('')
  const selectedGameId = ref('minecraft')
  const selectedProviders = ref<string[]>(['all'])
  const selectedVersion = ref('')
  const selectedLoader = ref('')
  const searchResults = ref<UnifiedModSearchResult[]>([])
  const isSearching = ref(false)
  const manifest = ref<GameModdingManifest | null>(null)

  // Downloads state
  const activeDownloads = ref<Map<string, ModDownloadProgress>>(new Map())

  // Interactive Dependency Modal State
  const dependencyModalOpen = ref(false)
  const pendingModToInstall = ref<UnifiedModSearchResult | null>(null)
  const targetProfileId = ref<string>('')
  const detectedDependencies = ref<RemoteDependency[]>([])
  const selectedDependencies = ref<string[]>([])
  const isResolvingDeps = ref(false)
  const isBatchInstalling = ref(false)

  // Getters
  const hasActiveDownloads = computed(() => activeDownloads.value.size > 0)
  const activeDownloadsList = computed(() => Array.from(activeDownloads.value.values()))

  // Actions
  async function loadManifest(gameId: string = 'minecraft') {
    try {
      manifest.value = await ipc.getGameModdingManifest(gameId)
    } catch (err) {
      console.warn(`[ModdingStore] Could not load manifest for ${gameId}:`, err)
    }
  }

  async function searchMods() {
    isSearching.value = true
    try {
      searchResults.value = await ipc.searchUnifiedMods(
        searchQuery.value,
        selectedGameId.value,
        selectedProviders.value,
        selectedVersion.value || undefined,
        selectedLoader.value || undefined
      )
    } catch (err) {
      console.error('[ModdingStore] Unified mod search failed:', err)
      searchResults.value = []
    } finally {
      isSearching.value = false
    }
  }

  function setProviderFilter(prov: 'all' | 'modrinth' | 'curseforge') {
    if (prov === 'all') {
      selectedProviders.value = ['all']
    } else {
      selectedProviders.value = [prov]
    }
    searchMods()
  }

  async function requestInstallMod(profileId: string, mod: UnifiedModSearchResult) {
    targetProfileId.value = profileId
    pendingModToInstall.value = mod
    isResolvingDeps.value = true

    try {
      // Check for dependencies
      const deps = await ipc.resolveRemoteDependencies(
        mod.provider,
        mod.id,
        mod.latest_version || mod.id
      )

      const requiredDeps = deps.filter((d) => d.is_required)

      if (requiredDeps.length > 0) {
        detectedDependencies.value = requiredDeps
        selectedDependencies.value = requiredDeps.map((d) => d.remote_mod_id)
        dependencyModalOpen.value = true
      } else {
        // No required dependencies, direct 1-click install
        await directInstall(profileId, mod)
      }
    } catch (err) {
      console.warn('[ModdingStore] Dependency resolution error, proceeding with direct install:', err)
      await directInstall(profileId, mod)
    } finally {
      isResolvingDeps.value = false
    }
  }

  async function directInstall(profileId: string, mod: UnifiedModSearchResult) {
    const downloadId = `${mod.provider}-${mod.id}`
    activeDownloads.value.set(downloadId, {
      id: downloadId,
      name: mod.name,
      progress: 30,
      status: 'downloading'
    })

    try {
      await ipc.installModOrPack(
        profileId,
        mod.provider,
        mod.latest_version || mod.id,
        mod.id,
        mod.name
      )

      activeDownloads.value.set(downloadId, {
        id: downloadId,
        name: mod.name,
        progress: 100,
        status: 'completed'
      })

      setTimeout(() => {
        activeDownloads.value.delete(downloadId)
      }, 3000)
    } catch (err: any) {
      activeDownloads.value.set(downloadId, {
        id: downloadId,
        name: mod.name,
        progress: 0,
        status: 'error',
        error: String(err?.message || err)
      })
    }
  }

  async function confirmCascadeInstall() {
    if (!pendingModToInstall.value || !targetProfileId.value) return

    isBatchInstalling.value = true
    const parentMod = pendingModToInstall.value
    const profileId = targetProfileId.value

    try {
      // 1. Install selected dependencies first
      for (const depId of selectedDependencies.value) {
        const dep = detectedDependencies.value.find((d) => d.remote_mod_id === depId)
        const depName = dep?.name || depId

        await ipc.installModOrPack(
          profileId,
          parentMod.provider,
          depId,
          depId,
          depName
        )
      }

      // 2. Install target mod
      await directInstall(profileId, parentMod)

      dependencyModalOpen.value = false
      pendingModToInstall.value = null
    } catch (err) {
      console.error('[ModdingStore] Cascade install error:', err)
    } finally {
      isBatchInstalling.value = false
    }
  }

  function closeDependencyModal() {
    dependencyModalOpen.value = false
    pendingModToInstall.value = null
    detectedDependencies.value = []
    selectedDependencies.value = []
  }

  return {
    searchQuery,
    selectedGameId,
    selectedProviders,
    selectedVersion,
    selectedLoader,
    searchResults,
    isSearching,
    manifest,
    activeDownloads,
    hasActiveDownloads,
    activeDownloadsList,
    dependencyModalOpen,
    pendingModToInstall,
    targetProfileId,
    detectedDependencies,
    selectedDependencies,
    isResolvingDeps,
    isBatchInstalling,
    loadManifest,
    searchMods,
    setProviderFilter,
    requestInstallMod,
    directInstall,
    confirmCascadeInstall,
    closeDependencyModal
  }
})
