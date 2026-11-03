<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useToastStore } from '@/stores/toastStore'
import { useGamesStore } from '@/stores/gamesStore'
import { useDownloadsStore } from '@/stores/downloadsStore'
import DownloadsPopover from '@/components/layout/DownloadsPopover.vue'
import { ipc } from '@/services/ipc'

const route = useRoute()
const router = useRouter()
const toast = useToastStore()
const gamesStore = useGamesStore()
const downloadsStore = useDownloadsStore()

const downloadsPopoverOpen = ref(false)
const searchQuery = ref('')
const searchInputRef = ref<HTMLInputElement | null>(null)

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

// Window controls (Tauri native or mock)
async function minimizeWindow() {
  if (ipc.isTauri()) {
    const { getCurrentWindow } = await import('@tauri-apps/api/window')
    getCurrentWindow().minimize()
  } else {
    toast.show('Fenêtre minimisée (simulation)', 'info')
  }
}

async function toggleMaximizeWindow() {
  if (ipc.isTauri()) {
    const { getCurrentWindow } = await import('@tauri-apps/api/window')
    getCurrentWindow().toggleMaximize()
  } else {
    toast.show('Plein écran basculé (simulation)', 'info')
  }
}

async function closeWindow() {
  if (ipc.isTauri()) {
    const { getCurrentWindow } = await import('@tauri-apps/api/window')
    getCurrentWindow().close()
  } else {
    toast.show('Fermeture de l\'application (simulation)', 'info')
  }
}
</script>

<template>
  <header
    data-tauri-drag-region
    class="relative z-30 px-4 sm:px-6 py-2.5 flex items-center justify-between gap-4 backdrop-blur-md bg-zinc-950/60 border-b border-zinc-800/60 select-none flex-shrink-0"
  >
    <!-- Left: Back/Forward Navigation + Clean Menu Pills (Bibliothèque, Fiche Jeu, Studio) -->
    <div class="flex items-center gap-3">
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

      <!-- Section Switcher Pills (Épuré : Bibliothèque, Fiche Jeu, Studio) -->
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

        <button
          type="button"
          @click="router.push('/games/hollow-knight-silksong')"
          :class="[
            'px-3.5 py-1 rounded-full text-xs font-medium transition flex items-center gap-1',
            route.path.startsWith('/games')
              ? 'bg-amber-500 text-zinc-950 font-bold shadow-sm'
              : 'text-zinc-400 hover:text-zinc-200'
          ]"
        >
          Fiche Jeu
        </button>

        <!-- Special tab: Minecraft Studio integration -->
        <button
          type="button"
          @click="router.push('/minecraft')"
          :class="[
            'px-3.5 py-1 rounded-full text-xs font-medium transition flex items-center gap-1.5',
            route.path === '/minecraft'
              ? 'bg-amber-500 text-zinc-950 font-bold shadow-sm'
              : 'text-amber-400/90 hover:text-amber-300'
          ]"
        >
          <i class="ph-fill ph-cube text-xs"></i> Studio
        </button>
      </nav>
    </div>

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

    <!-- Right: Downloads + Alerts + Paramètres (Écrou) + Profil (Compte) + Contrôles Fenêtre (Linux/Windows) -->
    <div class="flex items-center gap-2">
      <!-- Active Running Game Indicator if any -->
      <div
        v-if="gamesStore.runningGameId"
        class="hidden lg:flex items-center gap-2 px-3 py-1 rounded-full bg-amber-500/15 border border-amber-500/40 text-amber-300 text-xs font-medium"
      >
        <span class="w-2 h-2 rounded-full bg-amber-400 animate-pulse" />
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
          :title="downloadsStore.isDownloading ? `Téléchargement Steam en cours (${downloadsStore.formattedSpeed})` : 'Gestionnaire de téléchargements'"
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
            v-else-if="downloadsStore.completedDownloads.length > 0"
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

      <!-- User profile pill (Hub de Comptes déjà accessible ici) -->
      <div
        @click="router.push('/accounts')"
        :class="[
          'flex items-center gap-2 pl-1 pr-3 py-1 rounded-full transition cursor-pointer border',
          route.path === '/accounts'
            ? 'bg-zinc-900 border-amber-500/50 shadow-sm'
            : 'bg-zinc-900/80 border-zinc-800/80 hover:border-zinc-700'
        ]"
        title="Mon Compte & Services liés"
      >
        <div class="w-6 h-6 rounded-full bg-amber-500/20 text-amber-300 font-semibold text-xs flex items-center justify-center border border-amber-500/40">
          K
        </div>
        <span class="text-xs font-medium text-zinc-300">k0bus</span>
      </div>

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
          title="Agrandir / Restaurer"
        >
          <i class="ph ph-square text-xs"></i>
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
