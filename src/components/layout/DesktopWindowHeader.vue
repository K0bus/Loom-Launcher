<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useToastStore } from '@/stores/toastStore'
import { useGamesStore } from '@/stores/gamesStore'
import { useDownloadsStore } from '@/stores/downloadsStore'
import { useAccountsStore } from '@/stores/accountsStore'
import { useSettingsStore } from '@/stores/settingsStore'
import { useSystemStore } from '@/stores/systemStore'
import DownloadsPopover from '@/components/layout/DownloadsPopover.vue'
import { ipc } from '@/services/ipc'

const route = useRoute()
const router = useRouter()
const toast = useToastStore()
const gamesStore = useGamesStore()
const downloadsStore = useDownloadsStore()
const accountsStore = useAccountsStore()
const settingsStore = useSettingsStore()
const systemStore = useSystemStore()

const downloadsPopoverOpen = ref(false)
const searchQuery = ref('')
const searchInputRef = ref<HTMLInputElement | null>(null)
const avatarImgFailed = ref(false)

const userAvatar = computed(() => {
  const bnet = accountsStore.getAccount('battlenet')
  if (bnet?.connected && bnet.avatarUrl) return bnet.avatarUrl
  const xbox = accountsStore.getAccount('xbox')
  if (xbox?.connected && xbox.avatarUrl) return xbox.avatarUrl
  const steam = accountsStore.getAccount('steam')
  if (steam?.connected && steam.avatarUrl) return steam.avatarUrl
  return null
})

watch(userAvatar, () => {
  avatarImgFailed.value = false
})

const connectedCount = computed(() => {
  return accountsStore.accounts.filter((a) => a.connected).length
})

function goToAccounts() {
  if (route.path !== '/accounts') {
    router.push('/accounts')
  }
}

function goBack() {
  window.history.length > 1 ? router.back() : router.push('/')
}

function goForward() {
  router.forward()
}

function handleSearchEnter() {
  if (searchQuery.value.trim()) {
    router.push({
      path: '/library',
      query: { q: searchQuery.value }
    })
  }
}

function handleKeydown(e: KeyboardEvent) {
  if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
    e.preventDefault()
    searchInputRef.value?.focus()
  }
}

onMounted(() => {
  window.addEventListener('keydown', handleKeydown)
})

onUnmounted(() => {
  window.removeEventListener('keydown', handleKeydown)
})

// Window controls (Desktop native, Tauri or fallback)
async function minimizeWindow() {
  if ((window as any).desktopWindow) {
    ;(window as any).desktopWindow.minimize()
    return
  }
  if (ipc.isTauri()) {
    const { getCurrentWindow } = await import('@tauri-apps/api/window')
    getCurrentWindow().minimize()
  } else {
    toast.show('Fenêtre minimisée (simulation)', 'info')
  }
}

async function toggleMaximizeWindow() {
  if ((window as any).desktopWindow) {
    ;(window as any).desktopWindow.toggleMaximize()
    systemStore.setMaximized(!systemStore.isMaximized)
    return
  }
  if (ipc.isTauri()) {
    try {
      const { getCurrentWindow } = await import('@tauri-apps/api/window')
      const win = getCurrentWindow()
      await win.toggleMaximize()
      systemStore.setMaximized(await win.isMaximized())
    } catch {}
  } else {
    systemStore.setMaximized(!systemStore.isMaximized)
    toast.show('Plein écran basculé (simulation)', 'info')
  }
}

async function closeWindow() {
  if (settingsStore.settings.closeToTray) {
    try {
      await ipc.hideWindow()
      toast.show('Loom réduit dans la barre d\'état. Clic droit sur l\'icône pour quitter.', 'info')
      return
    } catch (err) {
      console.warn('[DesktopWindowHeader] hideWindow failed, falling back to exit:', err)
    }
  }

  if ((window as any).desktopWindow) {
    ;(window as any).desktopWindow.close()
    return
  }

  if (ipc.isTauri()) {
    try {
      await ipc.exitApp()
      return
    } catch {
      const { getCurrentWindow } = await import('@tauri-apps/api/window')
      getCurrentWindow().close()
    }
  } else {
    toast.show('Fermeture de l\'application (simulation)', 'info')
  }
}
</script>

<template>
  <header
    class="relative z-30 px-4 sm:px-6 py-2.5 flex items-center justify-between gap-3 bg-zinc-950/90 border-b border-zinc-800/80 select-none flex-shrink-0"
  >
    <!-- Left: Brand Logo + Back/Forward Navigation + Clean Menu Pills (Bibliothèque, Minecraft) -->
    <div class="flex items-center gap-3">
      <!-- Brand Logo -->
      <button
        type="button"
        @click="router.push('/library')"
        class="flex items-center justify-center p-0.5 rounded-lg hover:opacity-90 hover:brightness-110 active:scale-95 transition focus:outline-none"
        title="Loom Launcher"
      >
        <img
          src="/logo.png"
          alt="Loom Logo"
          class="w-7 h-7 object-contain drop-shadow-[0_2px_8px_rgba(245,158,11,0.25)] select-none pointer-events-none"
        />
      </button>

      <!-- History navigation -->
      <div class="flex items-center text-zinc-400 mr-1">
        <button
          type="button"
          @click="goBack"
          class="p-1.5 rounded-full hover:bg-zinc-800/60 hover:text-white transition disabled:opacity-40"
          title="Précédent"
        >
          <i class="ph ph-arrow-left text-base"></i>
        </button>
        <button
          type="button"
          @click="goForward"
          class="p-1.5 rounded-full hover:bg-zinc-800/60 hover:text-white transition disabled:opacity-40"
          title="Suivant"
        >
          <i class="ph ph-arrow-right text-base"></i>
        </button>
      </div>

      <!-- Section Switcher Pills (Épuré : Bibliothèque, Minecraft) -->
      <nav class="flex items-center gap-1 bg-zinc-900/80 p-1 rounded-full border border-zinc-800/80">
        <button
          type="button"
          @click="router.push('/library')"
          :class="[
            'px-4 py-1 rounded-full text-xs font-semibold transition',
            route.path === '/library'
              ? 'bg-amber-500 text-zinc-950 font-bold shadow-sm'
              : 'text-zinc-400 hover:text-zinc-200'
          ]"
        >
          Bibliothèque
        </button>

        <!-- Special tab: Modding integration -->
        <button
          type="button"
          @click="router.push('/modding')"
          :class="[
            'px-3.5 py-1 rounded-full text-xs font-medium transition flex items-center gap-1.5',
            route.path.startsWith('/modding')
              ? 'bg-amber-500 text-zinc-950 font-bold shadow-sm'
              : 'text-amber-400/90 hover:text-amber-300'
          ]"
        >
          <i class="ph-fill ph-stack text-xs"></i> Modding
        </button>
      </nav>
    </div>

    <!-- Drag region handle gauche -->
    <div data-tauri-drag-region class="flex-1 h-8 cursor-default select-none min-w-[12px]"></div>

    <!-- Center: Fuzzy Search Bar with ⌘K -->
    <div class="flex-1 max-w-md hidden md:block">
      <div class="relative group">
        <i class="ph ph-magnifying-glass absolute left-3.5 top-1/2 -translate-y-1/2 text-zinc-400 group-focus-within:text-amber-400 transition"></i>
        <input
          ref="searchInputRef"
          v-model="searchQuery"
          @keydown.enter="handleSearchEnter"
          type="text"
          :placeholder="gamesStore.games.length > 0 ? `Rechercher parmi vos ${gamesStore.games.length} jeux...` : 'Rechercher un jeu (Ctrl+K)...'"
          class="w-full bg-zinc-900/70 border border-zinc-800/80 rounded-full pl-9 pr-14 py-1.5 text-xs text-zinc-200 placeholder-zinc-400 focus:outline-none focus:border-zinc-600 focus:bg-zinc-900 transition"
        />
        <kbd class="absolute right-3 top-1/2 -translate-y-1/2 px-1.5 py-0.5 text-[10px] font-mono text-zinc-400 bg-zinc-800/90 border border-zinc-700/60 rounded">
          ⌘K
        </kbd>
      </div>
    </div>

    <!-- Drag region handle droite -->
    <div data-tauri-drag-region class="flex-1 h-8 cursor-default select-none min-w-[12px]"></div>

    <!-- Right: Downloads + Alerts + Paramètres (Écrou) + Profil (Compte) + Contrôles Fenêtre (Linux/Windows) -->
    <div class="flex items-center gap-2">
      <!-- Active Running Game Indicator if any -->
      <div
        v-if="gamesStore.runningGameId"
        class="hidden lg:flex items-center gap-2 px-3 py-1 rounded-full bg-amber-500/15 border border-amber-500/40 text-amber-300 text-xs font-medium"
      >
        <span class="w-2 h-2 rounded-full bg-amber-400" />
        <span>En cours : {{ gamesStore.getGameById(gamesStore.runningGameId)?.title }}</span>
      </div>

      <!-- Download Manager with Popover -->
      <div class="relative">
        <button
          type="button"
          @click="downloadsPopoverOpen = !downloadsPopoverOpen"
          :class="[
            'p-2 rounded-full transition relative flex items-center gap-1.5',
            downloadsPopoverOpen || downloadsStore.isDownloading
              ? 'text-amber-400 bg-amber-500/15 border border-amber-500/30 shadow-sm'
              : 'text-zinc-400 hover:text-zinc-200 hover:bg-zinc-900/80'
          ]"
          :title="downloadsStore.isDownloading ? `Téléchargements en cours (${downloadsStore.activeCount} actif${downloadsStore.activeCount > 1 ? 's' : ''} • ${downloadsStore.formattedSpeed})` : 'Gestionnaire de téléchargements'"
        >
          <i
            class="ph ph-download-simple text-base"
            :class="{ 'animate-bounce text-amber-400': downloadsStore.isDownloading }"
          ></i>
          <span
            v-if="downloadsStore.isDownloading"
            class="text-[11px] font-bold font-mono text-amber-300 pr-0.5"
          >
            {{ downloadsStore.activeCount }}
          </span>
          <span
            v-else-if="downloadsStore.completedDownloads.length > 0 || downloadsStore.completedModDownloads.length > 0"
            class="absolute top-1.5 right-1.5 w-1.5 h-1.5 rounded-full bg-emerald-400"
          ></span>
        </button>

        <!-- Dropdown Popover -->
        <DownloadsPopover
          :open="downloadsPopoverOpen"
          @close="downloadsPopoverOpen = false"
        />
      </div>

      <!-- Notifications -->
      <button
        type="button"
        @click="toast.show('Aucune nouvelle notification système', 'info')"
        class="p-2 rounded-full text-zinc-400 hover:text-zinc-200 hover:bg-zinc-900/80 transition"
        title="Notifications"
      >
        <i class="ph ph-bell text-base"></i>
      </button>

      <!-- Écrou / Settings en haut à droite -->
      <button
        type="button"
        @click="router.push('/settings')"
        :class="[
          'p-2 rounded-full transition text-base',
          route.path === '/settings'
            ? 'text-amber-400 bg-zinc-900 border border-amber-500/40 shadow-sm'
            : 'text-zinc-400 hover:text-zinc-200 hover:bg-zinc-900/80'
        ]"
        title="Paramètres de l'application"
      >
        <i class="ph ph-gear"></i>
      </button>

      <!-- User profile button (Hub de Comptes & Services liés) -->
      <button
        type="button"
        role="button"
        @click="goToAccounts"
        :class="[
          'flex items-center gap-2 pl-1.5 pr-3 py-1 rounded-full transition cursor-pointer border select-none focus:outline-none focus:ring-2 focus:ring-amber-500/50 active:scale-95 group no-drag',
          route.path === '/accounts'
            ? 'bg-amber-500/15 border-amber-500/50 shadow-sm text-amber-300 font-semibold'
            : 'bg-zinc-900/80 border-zinc-800/80 hover:border-amber-500/50 hover:bg-zinc-800/90 text-zinc-300 hover:text-white'
        ]"
        style="-webkit-app-region: no-drag;"
        data-tauri-drag-region="false"
        title="Mon Compte & Services liés (Accéder aux différents comptes)"
      >
        <div class="w-6 h-6 rounded-full bg-amber-500/20 text-amber-300 font-semibold text-xs flex items-center justify-center border border-amber-500/40 overflow-hidden flex-shrink-0 pointer-events-none">
          <img
            v-if="userAvatar && !avatarImgFailed"
            :src="userAvatar"
            alt="Avatar"
            class="w-full h-full object-cover"
            @error="avatarImgFailed = true"
          />
          <span v-else>K</span>
        </div>
        <span class="text-xs font-medium pointer-events-none group-hover:text-amber-300 transition-colors">k0bus</span>
        <span
          v-if="connectedCount > 0"
          class="w-1.5 h-1.5 rounded-full bg-emerald-400 pointer-events-none shadow-[0_0_6px_rgba(52,211,153,0.8)]"
          :title="`${connectedCount} plateforme(s) connectée(s)`"
        />
      </button>

      <!-- Séparateur vertical -->
      <div class="h-4 w-px bg-zinc-800 mx-1 hidden sm:block" />

      <!-- Contrôles de Fenêtre Bureau (Linux / Windows à droite) -->
      <div class="flex items-center gap-1">
        <!-- Réduire (Minimize) -->
        <button
          type="button"
          @click="minimizeWindow"
          class="w-7 h-7 flex items-center justify-center rounded-lg hover:bg-zinc-800/80 text-zinc-400 hover:text-zinc-100 transition-colors"
          title="Réduire la fenêtre"
        >
          <i class="ph ph-minus text-xs"></i>
        </button>

        <!-- Agrandir / Restaurer (Maximize) -->
        <button
          type="button"
          @click="toggleMaximizeWindow"
          class="w-7 h-7 flex items-center justify-center rounded-lg hover:bg-zinc-800/80 text-zinc-400 hover:text-zinc-100 transition-colors"
          :title="systemStore.isMaximized ? 'Restaurer la fenêtre' : 'Agrandir la fenêtre'"
        >
          <i :class="['ph text-xs', systemStore.isMaximized ? 'ph-copy' : 'ph-square']"></i>
        </button>

        <!-- Fermer (Close) -->
        <button
          type="button"
          @click="closeWindow"
          class="w-7 h-7 flex items-center justify-center rounded-lg hover:bg-rose-600/90 text-zinc-400 hover:text-white transition-colors"
          title="Fermer l'application"
        >
          <i class="ph ph-x text-xs"></i>
        </button>
      </div>
    </div>
  </header>
</template>
