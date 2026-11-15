<script setup lang="ts">
import { onMounted, watch } from 'vue'
import DesktopWindowHeader from '@/components/layout/DesktopWindowHeader.vue'
import { useSystemStore } from '@/stores/systemStore'
import { useSettingsStore } from '@/stores/settingsStore'
import { useToastStore } from '@/stores/toastStore'
import { ipc } from '@/services/ipc'

const systemStore = useSystemStore()
const settingsStore = useSettingsStore()
const toast = useToastStore()

watch(
  () => settingsStore.settings.themeAccent,
  (val) => {
    settingsStore.applyThemeAccent(val)
  },
  { immediate: true }
)

onMounted(async () => {
  systemStore.refreshStats()

  // 1. Gestion du démarrage minimisé
  try {
    const shouldStartMinimized = await ipc.isStartMinimized()
    if (shouldStartMinimized || settingsStore.settings.startMinimized) {
      if (ipc.isTauri()) {
        const { getCurrentWindow } = await import('@tauri-apps/api/window')
        getCurrentWindow().minimize().catch(() => {})
      }
    }
  } catch {}

  // 2. Initialisation des instances externes configurées pour l'autostart (Steam, Battle.net, Epic, GOG)
  settingsStore.runAutostartInstances()

  // 3. Détection et écoute de l'état maximisé de la fenêtre pour adapter les bordures arrondies
  if (ipc.isTauri()) {
    try {
      const { getCurrentWindow } = await import('@tauri-apps/api/window')
      const win = getCurrentWindow()
      systemStore.setMaximized(await win.isMaximized())
      win.onResized(async () => {
        try {
          systemStore.setMaximized(await win.isMaximized())
        } catch {}
      }).catch(() => {})
    } catch {}

    // 4. Détection automatique des mises à jour sur disque pour les processus en cours
    try {
      const { listen } = await import('@tauri-apps/api/event')
      await listen('loom:update-detected', () => {
        toast.show(
          'Une nouvelle version de Loom Launcher a été déployée sur le disque.',
          'info'
        )
      })
    } catch {}
  }
})
</script>

<template>
  <div
    :class="[
      'flex flex-col h-screen w-screen overflow-hidden bg-zinc-950 font-sans text-zinc-100 select-none transition-all duration-150',
      systemStore.isMaximized
        ? 'rounded-none border-0'
        : 'rounded-none border border-zinc-800/80 shadow-2xl'
    ]"
  >
    <!-- Top Floating Desktop Titlebar & Navigation -->
    <DesktopWindowHeader />

    <!-- Main Desktop Content Area sans Transition (navigation instantanée sans flash) -->
    <main class="flex-1 overflow-y-auto overflow-x-hidden relative bg-zinc-950">
      <router-view />
    </main>

    <!-- Global Desktop Toast Notification Container -->
    <div
      :class="[
        'fixed bottom-6 right-6 z-50 flex items-center gap-3 px-4 py-3 rounded-xl bg-zinc-900 border border-amber-500/40 text-xs text-white shadow-2xl transition-all duration-300',
        toast.visible ? 'translate-y-0 opacity-100 pointer-events-auto' : 'translate-y-12 opacity-0 pointer-events-none'
      ]"
    >
      <i
        :class="[
          'text-base',
          toast.type === 'success' ? 'ph-fill ph-check-circle text-amber-400' :
          toast.type === 'error' ? 'ph-fill ph-warning-circle text-rose-400' :
          'ph-fill ph-info text-sky-400'
        ]"
      />
      <span class="font-medium">{{ toast.message }}</span>
    </div>
  </div>
</template>

<style>
/* Global background */
html, body {
  background-color: #09090b;
  overflow: hidden;
}
</style>
