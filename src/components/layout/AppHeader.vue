<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import {
  Search,
  Square,
  ShieldCheck,
  Zap
} from 'lucide-vue-next'
import { useGamesStore } from '@/stores/gamesStore'

const router = useRouter()
const gamesStore = useGamesStore()

const searchQuery = ref('')

function handleGlobalSearch() {
  if (searchQuery.value.trim()) {
    router.push({
      path: '/library',
      query: { q: searchQuery.value }
    })
  }
}
</script>

<template>
  <header class="h-16 px-8 flex items-center justify-between border-b border-zinc-800/80 bg-zinc-950/70 backdrop-blur-md select-none sticky top-0 z-20">
    <!-- Search Bar -->
    <div class="relative w-96">
      <Search class="absolute left-3.5 top-1/2 -translate-y-1/2 w-4 h-4 text-zinc-500" />
      <input
        v-model="searchQuery"
        @keydown.enter="handleGlobalSearch"
        type="text"
        placeholder="Rechercher un jeu, un modpack, un outil... (Ctrl+K)"
        class="w-full bg-zinc-900/90 hover:bg-zinc-900 text-zinc-100 placeholder-zinc-500 text-xs rounded-xl pl-10 pr-4 py-2 border border-zinc-800/80 focus:border-amber-500/70 focus:ring-2 focus:ring-amber-500/20 outline-none transition-all"
      />
    </div>

    <!-- Right Controls & Active Status -->
    <div class="flex items-center gap-4">
      <!-- Active Running Game Indicator -->
      <div
        v-if="gamesStore.runningGameId"
        class="flex items-center gap-2.5 px-3 py-1.5 rounded-xl bg-amber-500/10 border border-amber-500/30 text-xs animate-pulse-subtle"
      >
        <span class="w-2 h-2 rounded-full bg-amber-400 animate-ping" />
        <span class="text-amber-300 font-medium truncate max-w-[150px]">
          {{ gamesStore.getGameById(gamesStore.runningGameId)?.title }}
        </span>
        <button
          type="button"
          @click="gamesStore.stopGame(gamesStore.runningGameId!)"
          class="p-1 rounded-md text-amber-300 hover:text-white hover:bg-amber-500/30 transition-colors"
          title="Arrêter le processus"
        >
          <Square class="w-3.5 h-3.5 fill-current" />
        </button>
      </div>

      <!-- Linux Gaming Runners Quick Status -->
      <div class="hidden xl:flex items-center gap-2 text-xs font-mono text-zinc-400">
        <div class="flex items-center gap-1.5 px-2.5 py-1 rounded-lg bg-zinc-900/60 border border-zinc-800/70">
          <Zap class="w-3.5 h-3.5 text-amber-400" />
          <span>GameMode:</span>
          <span class="text-emerald-400 font-bold">Actif</span>
        </div>
        <div class="flex items-center gap-1.5 px-2.5 py-1 rounded-lg bg-zinc-900/60 border border-zinc-800/70">
          <ShieldCheck class="w-3.5 h-3.5 text-amber-400" />
          <span>Proton GE:</span>
          <span class="text-zinc-300 font-semibold">9-20</span>
        </div>
      </div>

      <!-- User Profile Badge -->
      <div
        @click="router.push('/accounts')"
        class="flex items-center gap-3 pl-2 py-1 pr-1 rounded-full bg-zinc-900/60 hover:bg-zinc-800/80 border border-zinc-800 transition-colors cursor-pointer"
      >
        <div class="flex flex-col text-right">
          <span class="text-xs font-semibold text-zinc-200">K0bus</span>
          <span class="text-[10px] text-amber-400/90 font-mono">Arch Linux x86_64</span>
        </div>
        <img
          src="https://images.unsplash.com/photo-1535713875002-d1d0cf377fde?q=80&w=120&auto=format&fit=crop"
          alt="Avatar"
          class="w-8 h-8 rounded-full border border-amber-500/40 object-cover"
        />
      </div>
    </div>
  </header>
</template>
