import { defineStore } from 'pinia'
import { ref } from 'vue'
import { ipc } from '@/services/ipc'
import { getCompleteGpuDiagnostic, type FullGpuDiagnostic } from '@/services/gpuDiagnostic'

export interface AutostartInstances {
  steam: boolean
  battlenet: boolean
  epic: boolean
  gog: boolean
}

export interface LoomSettingsState {
  // Catégorie 1 : Paramètres propres à Loom
  hardwareAcceleration: boolean
  autostartWithSystem: boolean
  startMinimized: boolean
  closeToTray: boolean
  afterGameLaunch: 'minimize' | 'hide' | 'keep'
  autostartInstances: AutostartInstances
  startInstancesMinimized: boolean
  desktopNotifications: boolean
  themeAccent: 'amber' | 'emerald' | 'violet' | 'sky' | 'rose'
  checkUpdatesAuto: boolean
  autoSyncCloudSaves: boolean

  // Catégorie 2 : Environnement Linux & Compatibilité
  defaultRunner: string
  globalGamemode: boolean
  globalMangohud: boolean

  // Catégorie 3 : Emplacements des fichiers
  gamesDirectory: string
  minecraftDirectory: string
  winePrefixesDirectory: string
}

const STORAGE_KEY = 'loom_launcher_settings'

const defaultSettings: LoomSettingsState = {
  hardwareAcceleration: true,
  autostartWithSystem: false,
  startMinimized: false,
  closeToTray: true,
  afterGameLaunch: 'minimize',
  autostartInstances: {
    steam: true,
    battlenet: true,
    epic: false,
    gog: false
  },
  startInstancesMinimized: true,
  desktopNotifications: true,
  themeAccent: 'amber',
  checkUpdatesAuto: true,
  autoSyncCloudSaves: true,

  defaultRunner: 'proton-ge',
  globalGamemode: true,
  globalMangohud: true,

  gamesDirectory: '/mnt/games-nvme/SteamLibrary',
  minecraftDirectory: '~/.local/share/loom/instances',
  winePrefixesDirectory: '~/.local/share/loom/prefixes'
}

export const useSettingsStore = defineStore('settings', () => {
  const settings = ref<LoomSettingsState>(loadSettings())
  const initialHwAccel = ref<boolean>(settings.value.hardwareAcceleration)
  const restartRequired = ref<boolean>(false)
  const gpuDiagnostic = ref<FullGpuDiagnostic | null>(null)
  const isRunningBenchmark = ref<boolean>(false)

  // Synchronisation initiale avec le backend Tauri
  ipc.getHardwareAcceleration().then((enabled) => {
    if (typeof enabled === 'boolean') {
      settings.value.hardwareAcceleration = enabled
      initialHwAccel.value = enabled
    }
    // Lance une première sonde WebGL & backend
    refreshGpuDiagnostic(false)
  }).catch(() => {
    refreshGpuDiagnostic(false)
  })

  async function refreshGpuDiagnostic(withBenchmark = false) {
    if (withBenchmark) {
      isRunningBenchmark.value = true
    }
    try {
      gpuDiagnostic.value = await getCompleteGpuDiagnostic(withBenchmark)
    } finally {
      if (withBenchmark) {
        isRunningBenchmark.value = false
      }
    }
  }

  ipc.getSystemAutostart().then((enabled) => {
    if (typeof enabled === 'boolean') {
      settings.value.autostartWithSystem = enabled
    }
  }).catch(() => {})

  ipc.getBackendSettings().then((backend) => {
    if (backend && typeof backend === 'object') {
      if (backend.minecraftDirectory) settings.value.minecraftDirectory = backend.minecraftDirectory
      if (backend.winePrefixesDirectory) settings.value.winePrefixesDirectory = backend.winePrefixesDirectory
      if (backend.gamesDirectory) settings.value.gamesDirectory = backend.gamesDirectory
      if (typeof backend.closeToTray === 'boolean') settings.value.closeToTray = backend.closeToTray
      if (typeof backend.startMinimized === 'boolean') settings.value.startMinimized = backend.startMinimized
      if (backend.defaultRunner) settings.value.defaultRunner = backend.defaultRunner
      if (typeof backend.globalGamemode === 'boolean') settings.value.globalGamemode = backend.globalGamemode
      if (typeof backend.globalMangohud === 'boolean') settings.value.globalMangohud = backend.globalMangohud
    }
  }).catch(() => {})

  // Applique le thème au démarrage
  applyThemeAccent(settings.value.themeAccent)

  function applyThemeAccent(accent: string) {
    if (typeof document !== 'undefined') {
      document.documentElement.setAttribute('data-theme', accent)
    }
  }

  function loadSettings(): LoomSettingsState {
    if (typeof window === 'undefined') return { ...defaultSettings }
    try {
      const raw = localStorage.getItem(STORAGE_KEY)
      if (raw) {
        const parsed = JSON.parse(raw)
        return {
          ...defaultSettings,
          ...parsed,
          autostartInstances: {
            ...defaultSettings.autostartInstances,
            ...(parsed.autostartInstances || {})
          }
        }
      }
    } catch (err) {
      console.warn('[SettingsStore] Impossible de charger les paramètres locaux:', err)
    }
    return { ...defaultSettings }
  }

  function saveSettings(newSettings?: Partial<LoomSettingsState>) {
    if (newSettings) {
      settings.value = {
        ...settings.value,
        ...newSettings
      }
    }
    // Détecte si un paramètre critique nécessitant un redémarrage a changé
    restartRequired.value = settings.value.hardwareAcceleration !== initialHwAccel.value

    applyThemeAccent(settings.value.themeAccent)

    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(settings.value))
      // Synchronise l'accélération matérielle avec le backend Tauri
      ipc.setHardwareAcceleration(settings.value.hardwareAcceleration).catch(() => {})

      // Synchronise l'autostart système réel
      ipc.setSystemAutostart(settings.value.autostartWithSystem, settings.value.startMinimized).catch(() => {})

      // Synchronise la configuration complète avec le backend Tauri
      ipc.saveBackendSettings({
        closeToTray: settings.value.closeToTray,
        startMinimized: settings.value.startMinimized,
        autostartWithSystem: settings.value.autostartWithSystem,
        gamesDirectory: settings.value.gamesDirectory,
        minecraftDirectory: settings.value.minecraftDirectory,
        winePrefixesDirectory: settings.value.winePrefixesDirectory,
        defaultRunner: settings.value.defaultRunner,
        globalGamemode: settings.value.globalGamemode,
        globalMangohud: settings.value.globalMangohud
      }).catch(() => {})
    } catch (err) {
      console.error('[SettingsStore] Erreur sauvegarde localStorage:', err)
    }
  }

  async function runAutostartInstances() {
    const instances = settings.value.autostartInstances
    const minimized = settings.value.startInstancesMinimized

    for (const [key, enabled] of Object.entries(instances)) {
      if (enabled) {
        try {
          await ipc.autostartPlatformInstance(key, minimized)
        } catch (err) {
          console.warn(`[SettingsStore] Échec démarrage instance ${key}:`, err)
        }
      }
    }
  }

  function toggleInstance(instance: keyof AutostartInstances) {
    settings.value.autostartInstances[instance] = !settings.value.autostartInstances[instance]
    saveSettings()
  }

  function resetToDefaults() {
    settings.value = JSON.parse(JSON.stringify(defaultSettings))
    saveSettings()
  }

  return {
    settings,
    restartRequired,
    gpuDiagnostic,
    isRunningBenchmark,
    refreshGpuDiagnostic,
    saveSettings,
    toggleInstance,
    resetToDefaults,
    applyThemeAccent,
    runAutostartInstances
  }
})
