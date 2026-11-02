<script setup lang="ts">
import { ref, computed } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import {
  Search,
  SlidersHorizontal,
  Flame,
  Gamepad2
} from 'lucide-vue-next'
import { useGamesStore } from '@/stores/gamesStore'
import type { Game, GameFilter } from '@/models/game'
import GameCard from '@/components/games/GameCard.vue'
import GameFilterModal from '@/components/games/GameFilterModal.vue'
import GameRunnerModal from '@/components/games/GameRunnerModal.vue'
import UiButton from '@/components/ui/UiButton.vue'

const router = useRouter()
const route = useRoute()
const gamesStore = useGamesStore()

const filterModalOpen = ref(false)
const runnerModalOpen = ref(false)
const selectedGameForRunner = ref<Game | null>(null)

// Search & filters state
const initialSearch = (route.query.q as string) || ''
const filters = ref<GameFilter>({
  search: initialSearch,
  platform: 'all',
  genre: 'all',
  runner: 'all',
  sortBy: 'lastPlayed',
  sortOrder: 'desc'
})

const filteredGames = computed(() => {
  return gamesStore.filterGames(filters.value)
})

function openGameDetail(game: Game) {
  router.push(`/games/${game.id}`)
}

function configureRunner(game: Game) {
  selectedGameForRunner.value = game
  runnerModalOpen.value = true
}

function handleApplyFilters(newFilters: GameFilter) {
  filters.value = newFilters
}

const activeFiltersCount = computed(() => {
  let count = 0
  if (filters.value.platform && filters.value.platform !== 'all') count++
  if (filters.value.genre && filters.value.genre !== 'all') count++
  if (filters.value.runner && filters.value.runner !== 'all') count++
  return count
})
</script>

<template>
  <div class="max-w-7xl mx-auto px-6 sm:px-10 lg:px-14 py-6 space-y-8 pb-16">
    <!-- 1. HEADER & SEARCH & ADVANCED FILTERS MODAL BUTTON -->
    <div class="flex flex-col md:flex-row items-start md:items-center justify-between gap-4">
      <div>
        <h1 class="text-3xl font-black text-zinc-100 tracking-tight font-display flex items-center gap-3">
          <Gamepad2 class="w-8 h-8 text-amber-400" />
          Bibliothèque de Jeux
        </h1>
        <p class="text-xs text-zinc-400 mt-1">
          {{ filteredGames.length }} jeux disponibles • Gestion unifiée Linux Natif, Proton et Wine
        </p>
      </div>

      <!-- Search & Filters Trigger -->
      <div class="flex items-center gap-3 w-full md:w-auto">
        <div class="relative flex-1 md:w-72">
          <Search class="absolute left-3.5 top-1/2 -translate-y-1/2 w-4 h-4 text-zinc-500" />
          <input
            v-model="filters.search"
            type="text"
            placeholder="Filtrer par titre ou genre..."
            class="w-full bg-zinc-900/80 text-zinc-100 placeholder-zinc-500 text-xs rounded-xl pl-10 pr-4 py-2.5 border border-zinc-800 focus:border-amber-500/70 focus:ring-2 focus:ring-amber-500/20 outline-none transition-all"
          />
        </div>

        <UiButton
          variant="secondary"
          size="md"
          @click="filterModalOpen = true"
          class="relative"
        >
          <template #prefix>
            <SlidersHorizontal class="w-4 h-4 text-amber-400" />
          </template>
          Filtres
          <span
            v-if="activeFiltersCount > 0"
            class="ml-1 px-1.5 py-0.5 rounded-full text-[10px] font-bold bg-amber-500 text-zinc-950 font-mono"
          >
            {{ activeFiltersCount }}
          </span>
        </UiButton>
      </div>
    </div>

    <!-- 2. RAPPELS RÉCENTS (BANDEAU COMPACT) -->
    <section class="p-4 rounded-2xl bg-zinc-900/40 border border-zinc-800/80 space-y-3">
      <div class="flex items-center gap-2 text-xs font-semibold text-zinc-300">
        <Flame class="w-4 h-4 text-amber-400" />
        <span>Rappels Récents — Lancements Fréquents</span>
      </div>

      <div class="flex items-center gap-3 overflow-x-auto pb-1 no-scrollbar">
        <button
          v-for="g in gamesStore.recentGames"
          :key="g.id"
          type="button"
          @click="openGameDetail(g)"
          class="flex items-center gap-3 p-2 pr-4 rounded-xl bg-zinc-950/70 hover:bg-zinc-900 border border-zinc-800 hover:border-amber-500/40 transition-all flex-shrink-0 text-left group"
        >
          <img
            :src="g.coverUrl"
            :alt="g.title"
            class="w-9 h-12 object-cover rounded-lg border border-zinc-700/60"
          />
          <div>
            <div class="text-xs font-bold text-zinc-200 group-hover:text-amber-400 transition-colors line-clamp-1 max-w-[140px]">
              {{ g.title }}
            </div>
            <div class="text-[10px] text-zinc-500 font-mono mt-0.5">
              {{ g.config.runner }} • {{ (g.playtimeMinutes / 60).toFixed(0) }}h
            </div>
          </div>
        </button>
      </div>
    </section>

    <!-- 3. GRILLE DE VIGNETTES 3:4 FORMAT AFFICHE DE FILM -->
    <section class="space-y-4">
      <div class="flex items-center justify-between text-xs text-zinc-400 font-mono">
        <span>Résultats de recherche : {{ filteredGames.length }}</span>
        <span>Tri par : {{ filters.sortBy }} ({{ filters.sortOrder }})</span>
      </div>

      <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-5">
        <GameCard
          v-for="game in filteredGames"
          :key="game.id"
          :game="game"
          @select="openGameDetail"
          @configure="configureRunner"
        />
      </div>

      <!-- Empty state -->
      <div
        v-if="filteredGames.length === 0"
        class="py-16 text-center space-y-3 rounded-2xl bg-zinc-900/30 border border-zinc-800/60"
      >
        <p class="text-zinc-400 text-sm">
          Aucun jeu ne correspond à vos filtres actuels.
        </p>
        <UiButton
          variant="outline"
          size="sm"
          @click="filters = { search: '', platform: 'all', genre: 'all', runner: 'all', sortBy: 'lastPlayed', sortOrder: 'desc' }"
        >
          Réinitialiser tous les filtres
        </UiButton>
      </div>
    </section>

    <!-- Modals -->
    <GameFilterModal
      :open="filterModalOpen"
      :filters="filters"
      @close="filterModalOpen = false"
      @apply="handleApplyFilters"
    />

    <GameRunnerModal
      :open="runnerModalOpen"
      :game="selectedGameForRunner"
      @close="runnerModalOpen = false"
    />
  </div>
</template>
