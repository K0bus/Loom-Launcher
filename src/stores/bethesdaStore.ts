import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type {
  GameProfileRecord,
  ProfileModRecord,
  BethesdaPluginRecord,
  FileConflict,
  GameModdingManifest
} from '@/models/modding'
import { ipc } from '@/services/ipc'

export interface BethesdaGameMeta {
  gameId: string
  title: string
  subtitle: string
  icon: string
  bannerGradient: string
  defaultExecutable: string
  defaultFolder: string
}

export const BETHESDA_GAMES: Record<string, BethesdaGameMeta> = {
  skyrim_se: {
    gameId: 'skyrim_se',
    title: 'The Elder Scrolls V: Skyrim Special Edition',
    subtitle: 'Creation Engine • Modding avancé, SKSE64, Load Order & Conflits',
    icon: '🐉',
    bannerGradient: 'from-amber-950/60 via-zinc-900 to-zinc-950',
    defaultExecutable: 'skse64_loader.exe',
    defaultFolder: 'Skyrim Special Edition'
  },
  fallout4: {
    gameId: 'fallout4',
    title: 'Fallout 4',
    subtitle: 'Creation Engine • F4SE, Plugins ESM/ESL/ESP & Normalisation Staging',
    icon: '⚡',
    bannerGradient: 'from-blue-950/60 via-zinc-900 to-zinc-950',
    defaultExecutable: 'f4se_loader.exe',
    defaultFolder: 'Fallout4'
  },
  oblivion: {
    gameId: 'oblivion',
    title: 'The Elder Scrolls IV: Oblivion',
    subtitle: 'Gamebryo Engine • OBSE, Textures DDS & Load Order',
    icon: '⚔️',
    bannerGradient: 'from-rose-950/60 via-zinc-900 to-zinc-950',
    defaultExecutable: 'obse_loader.exe',
    defaultFolder: 'Oblivion'
  }
}

export const useBethesdaStore = defineStore('bethesda', () => {
  // State
  const currentGameId = ref<string>('skyrim_se')
  const manifest = ref<GameModdingManifest | null>(null)
  const profiles = ref<GameProfileRecord[]>([])
  const activeProfileId = ref<string>('')
  const mods = ref<ProfileModRecord[]>([])
  const plugins = ref<BethesdaPluginRecord[]>([])
  const conflicts = ref<FileConflict[]>([])

  const activeTab = ref<'mods' | 'conflicts' | 'plugins' | 'settings'>('mods')
  const isLoading = ref<boolean>(false)
  const isDeploying = ref<boolean>(false)
  const deploySuccessMessage = ref<string | null>(null)
  const deployErrorMessage = ref<string | null>(null)

  // Getters
  const currentGame = computed<BethesdaGameMeta>(() => {
    return BETHESDA_GAMES[currentGameId.value] || BETHESDA_GAMES.skyrim_se
  })

  const activeProfile = computed<GameProfileRecord | undefined>(() => {
    return profiles.value.find((p) => p.id === activeProfileId.value)
  })

  const enabledModsCount = computed<number>(() => {
    return mods.value.filter((m) => m.is_enabled).length
  })

  const activePluginsCount = computed<number>(() => {
    return plugins.value.filter((p) => p.is_active).length
  })

  const conflictsCount = computed<number>(() => {
    return conflicts.value.length
  })

  // Actions
  async function loadGame(gameId: string = 'skyrim_se') {
    currentGameId.value = gameId
    isLoading.value = true
    try {
      // 1. Load manifest
      manifest.value = await ipc.getGameManifest(gameId)

      // 2. Load profiles
      await loadProfiles()

      // 3. If no profile exists, create a default profile automatically
      if (profiles.value.length === 0) {
        await createDefaultProfile()
      } else if (!activeProfileId.value || !profiles.value.some((p) => p.id === activeProfileId.value)) {
        activeProfileId.value = profiles.value[0].id
      }

      // 4. Load details for active profile
      if (activeProfileId.value) {
        await refreshActiveProfileData()
      }
    } catch (err) {
      console.error(`[BethesdaStore] Error loading game ${gameId}:`, err)
    } finally {
      isLoading.value = false
    }
  }

  async function loadProfiles() {
    try {
      profiles.value = await ipc.getGameProfiles(currentGameId.value)
    } catch (err) {
      console.error('[BethesdaStore] Failed to load profiles:', err)
      profiles.value = []
    }
  }

  async function createDefaultProfile() {
    const defaultRecord: GameProfileRecord = {
      id: `${currentGameId.value}-default`,
      game_id: currentGameId.value,
      name: 'Profil Principal (Par Défaut)',
      game_version: '1.6+',
      modloader: currentGame.value.defaultExecutable,
      created_at: Math.floor(Date.now() / 1000)
    }

    try {
      await ipc.createGameProfile(defaultRecord)
      await loadProfiles()
      activeProfileId.value = defaultRecord.id
    } catch (err) {
      console.error('[BethesdaStore] Failed to create default profile:', err)
    }
  }

  async function createProfile(name: string, deploymentPath?: string) {
    const id = `${currentGameId.value}-${Date.now()}`
    const record: GameProfileRecord = {
      id,
      game_id: currentGameId.value,
      name,
      game_version: '1.6+',
      modloader: currentGame.value.defaultExecutable,
      deployment_path: deploymentPath,
      created_at: Math.floor(Date.now() / 1000)
    }

    await ipc.createGameProfile(record)
    await loadProfiles()
    activeProfileId.value = id
    await refreshActiveProfileData()
  }

  async function selectProfile(id: string) {
    activeProfileId.value = id
    await refreshActiveProfileData()
  }

  async function refreshActiveProfileData() {
    if (!activeProfileId.value) return

    try {
      const [m, p, c] = await Promise.all([
        ipc.getProfileMods(activeProfileId.value),
        ipc.getBethesdaPlugins(activeProfileId.value),
        ipc.scanAndResolveConflicts(activeProfileId.value)
      ])
      mods.value = m
      plugins.value = p
      conflicts.value = c
    } catch (err) {
      console.error('[BethesdaStore] Failed to refresh profile data:', err)
    }
  }

  async function toggleMod(modId: string, enabled: boolean) {
    if (!activeProfileId.value) return
    try {
      await ipc.toggleModState(activeProfileId.value, modId, enabled)
      const found = mods.value.find((m) => m.id === modId)
      if (found) found.is_enabled = enabled
      await refreshActiveProfileData()
    } catch (err) {
      console.error('[BethesdaStore] Failed to toggle mod:', err)
    }
  }

  async function reorderMod(modId: string, priority: number) {
    if (!activeProfileId.value) return
    try {
      await ipc.reorderMod(activeProfileId.value, modId, priority)
      await refreshActiveProfileData()
    } catch (err) {
      console.error('[BethesdaStore] Failed to reorder mod:', err)
    }
  }

  async function deleteMod(modId: string) {
    if (!activeProfileId.value) return
    try {
      await ipc.deleteMod(activeProfileId.value, modId)
      await refreshActiveProfileData()
    } catch (err) {
      console.error('[BethesdaStore] Failed to delete mod:', err)
    }
  }

  async function setConflictWinner(relativePath: string, winningModId: string) {
    if (!activeProfileId.value) return
    try {
      await ipc.setConflictWinner(activeProfileId.value, relativePath, winningModId)
      // Update local state immediately
      const found = conflicts.value.find((c) => c.relative_path === relativePath)
      if (found) found.current_winner_mod_id = winningModId
    } catch (err) {
      console.error('[BethesdaStore] Failed to set conflict winner:', err)
    }
  }

  async function togglePlugin(filename: string, isActive: boolean) {
    if (!activeProfileId.value) return
    try {
      await ipc.toggleBethesdaPlugin(activeProfileId.value, filename, isActive)
      const found = plugins.value.find((p) => p.filename === filename)
      if (found) found.is_active = isActive
    } catch (err) {
      console.error('[BethesdaStore] Failed to toggle plugin:', err)
    }
  }

  async function reorderPlugins(orderedFilenames: string[]) {
    if (!activeProfileId.value) return
    try {
      await ipc.updatePluginsOrder(activeProfileId.value, orderedFilenames)
      await refreshActiveProfileData()
    } catch (err) {
      console.error('[BethesdaStore] Failed to reorder plugins:', err)
    }
  }

  async function deployActiveProfile() {
    if (!activeProfileId.value) return
    isDeploying.value = true
    deploySuccessMessage.value = null
    deployErrorMessage.value = null

    try {
      await ipc.deployActiveProfile(activeProfileId.value)
      deploySuccessMessage.value = 'Profil et liens symboliques déployés avec succès !'
      setTimeout(() => {
        deploySuccessMessage.value = null
      }, 5000)
    } catch (err: any) {
      console.error('[BethesdaStore] Deploy error:', err)
      deployErrorMessage.value = String(err?.message || err)
    } finally {
      isDeploying.value = false
    }
  }

  async function handleNxmUri(uri: string) {
    try {
      await ipc.handleNxmUri(uri)
      await refreshActiveProfileData()
    } catch (err) {
      console.error('[BethesdaStore] Failed to handle NXM URI:', err)
    }
  }

  return {
    currentGameId,
    currentGame,
    manifest,
    profiles,
    activeProfileId,
    activeProfile,
    mods,
    plugins,
    conflicts,
    activeTab,
    isLoading,
    isDeploying,
    deploySuccessMessage,
    deployErrorMessage,
    enabledModsCount,
    activePluginsCount,
    conflictsCount,
    loadGame,
    loadProfiles,
    createProfile,
    selectProfile,
    refreshActiveProfileData,
    toggleMod,
    reorderMod,
    deleteMod,
    setConflictWinner,
    togglePlugin,
    reorderPlugins,
    deployActiveProfile,
    handleNxmUri
  }
})
