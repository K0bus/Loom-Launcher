<script setup lang="ts">
import { ref, computed } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useGamesStore } from '@/stores/gamesStore'
import { useToastStore } from '@/stores/toastStore'
import type { Game, GameFilter } from '@/models/game'
import GameCard from '@/components/games/GameCard.vue'
import GameFilterModal from '@/components/games/GameFilterModal.vue'
import GameRunnerModal from '@/components/games/GameRunnerModal.vue'
import UiButton from '@/components/ui/UiButton.vue'
import UiBadge from '@/components/ui/UiBadge.vue'

const router = useRouter()
const route = useRoute()
const gamesStore = useGamesStore()
const toast = useToastStore()

const filterModalOpen = ref(false)
const runnerModalOpen = ref(false)
const selectedGameForRunner = ref<Game | null>(null)

// 1. HERO : Dernier jeu joué
const heroGame = computed<Game>(() => {
  return gamesStore.recentGames[0] || gamesStore.heroGame || gamesStore.games[0]
})

const isHeroRunning = computed(() => {
  return gamesStore.runningGameId === heroGame.value?.id
})

async function handlePlayHero() {
  if (!heroGame.value) return
  if (!isHeroRunning.value) {
    await gamesStore.launchGame(heroGame.value.id)
    toast.show(`Lancement de ${heroGame.value.title}...`, 'success')
  } else {
    await gamesStore.stopGame(heroGame.value.id)
    toast.show('Session arrêtée.', 'info')
  }
}

// 2. RECENT GAMES (exclut optionnellement le hero pour le carrousel)
const recentPlayedGames = computed(() => {
  return gamesStore.recentGames
})

// 3. SEARCH, QUICK TAGS & FILTER STATE
const initialSearch = (route.query.q as string) || ''
const selectedTag = ref<string>('all')

const quickTags = [
  { id: 'all', label: 'Tous', icon: 'ph-squares-four' },
  { id: 'favorites', label: 'Favoris ⭐', icon: 'ph-star' },
  { id: 'Metroidvania', label: 'Metroidvania', icon: 'ph-sword' },
  { id: 'Action RPG', label: 'Action RPG', icon: 'ph-shield' },
  { id: 'Sandbox', label: 'Sandbox / Mods', icon: 'ph-cube' },
  { id: 'Souls-like', label: 'Souls-like', icon: 'ph-skull' },
  { id: 'proton-ge', label: 'Proton-GE', icon: 'ph-cpu' },
  { id: 'native', label: 'Linux Natif', icon: 'ph-terminal' },
  { id: 'steam', label: 'Steam', icon: 'ph-game-controller' }
]

const filters = ref<GameFilter>({
  search: initialSearch,
  platform: 'all',
  genre: 'all',
  runner: 'all',
  sortBy: 'lastPlayed',
  sortOrder: 'desc'
})

function selectQuickTag(tagId: string) {
  selectedTag.value = tagId

  // Apply to filters reactively
  if (tagId === 'all') {
    filters.value.genre = 'all'
    filters.value.platform = 'all'
    filters.value.runner = 'all'
  } else if (tagId === 'favorites') {
    // handled in filteredGames
  } else if (tagId === 'proton-ge' || tagId === 'native') {
    filters.value.runner = tagId
    filters.value.genre = 'all'
    filters.value.platform = 'all'
  } else if (tagId === 'steam') {
    filters.value.platform = tagId
    filters.value.genre = 'all'
    filters.value.runner = 'all'
  } else {
    filters.value.genre = tagId
    filters.value.platform = 'all'
    filters.value.runner = 'all'
  }
}

const filteredGames = computed(() => {
  let list = gamesStore.filterGames(filters.value)
  if (selectedTag.value === 'favorites') {
    list = list.filter((g) => g.isFavorite)
  }
  return list
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
  selectedTag.value = 'all'
}

const activeFiltersCount = computed(() => {
  let count = 0
  if (filters.value.platform && filters.value.platform !== 'all') count++
  if (filters.value.genre && filters.value.genre !== 'all') count++
  if (filters.value.runner && filters.value.runner !== 'all') count++
  if (selectedTag.value !== 'all') count++
  return count
})

function resetFilters() {
  selectedTag.value = 'all'
  filters.value = {
    search: '',
    platform: 'all',
    genre: 'all',
    runner: 'all',
    sortBy: 'lastPlayed',
    sortOrder: 'desc'
  }
}
</script>

<template>
  <div class="max-w-7xl mx-auto px-6 sm:px-10 lg:px-14 py-6 space-y-10 pb-20">
    <!-- ========================================================================= -->
    <!-- 1. HERO DU DERNIER JEU JOUÉ                                              -->
    <!-- ========================================================================= -->
    <section v-if="heroGame" class="relative rounded-3xl overflow-hidden border border-zinc-800/80 shadow-2xl bg-zinc-950 group">
      <!-- Background Banner 21:9 format -->
      <div class="relative w-full aspect-[21/9] min-h-[360px] max-h-[460px] overflow-hidden">
        <img
          :src="heroGame.bannerUrl"
          :alt="heroGame.title"
          class="w-full h-full object-cover object-[65%_25%] transform scale-100 group-hover:scale-105 transition-transform duration-1000 ease-out"
        />

        <!-- Ambient glowing overlays & vignettes matching Aureus reference -->
        <div class="absolute inset-0 hero-glow mix-blend-screen pointer-events-none" />
        <div class="absolute inset-0 hero-vignette pointer-events-none" />
        <div class="absolute inset-0 bg-[radial-gradient(#ffffff08_1px,transparent_1px)] [background-size:24px_24px] pointer-events-none" />
      </div>

      <!-- Hero Content Overlay -->
      <div class="absolute inset-0 p-8 sm:p-10 flex flex-col justify-end">
        <div class="max-w-3xl space-y-4">
          <!-- Badges -->
          <div class="flex items-center gap-2">
            <UiBadge variant="amber" size="sm" :dot="true">
              Dernier Jeu Lancé
            </UiBadge>
            <UiBadge variant="zinc" size="sm">
              {{ heroGame.config.runner === 'proton-ge' ? 'Proton GE-9-20' : heroGame.config.runner }}
              <span v-if="heroGame.config.gamemode">• GameMode</span>
            </UiBadge>
            <UiBadge variant="zinc" size="sm" class="uppercase">
              {{ heroGame.platform }}
            </UiBadge>
          </div>

          <!-- Title -->
          <div class="space-y-1">
            <h1 class="font-cinzel text-3xl sm:text-5xl lg:text-6xl font-black tracking-wider text-white drop-shadow-[0_4px_24px_rgba(0,0,0,0.8)]">
              {{ heroGame.title }}
            </h1>
            <div v-if="heroGame.subtitle" class="flex items-center justify-between -mt-1 sm:-mt-2 max-w-sm">
              <span class="h-[1px] flex-1 bg-gradient-to-r from-transparent via-zinc-400 to-transparent"></span>
              <span class="font-cinzel text-base sm:text-xl font-bold tracking-[0.3em] text-amber-200 px-3 drop-shadow-md">
                {{ heroGame.subtitle }}
              </span>
              <span class="h-[1px] flex-1 bg-gradient-to-r from-transparent via-zinc-400 to-transparent"></span>
            </div>
          </div>

          <!-- Quick Metrics & Info -->
          <div class="flex flex-wrap items-center gap-4 text-xs font-medium text-zinc-300 font-mono">
            <span class="flex items-center gap-1.5 bg-zinc-900/80 backdrop-blur-md px-3 py-1 rounded-full border border-zinc-700/60 font-sans">
              <i class="ph ph-clock text-amber-400"></i>
              {{ (heroGame.playtimeMinutes / 60).toFixed(1) }} heures de jeu
            </span>
            <span class="flex items-center gap-1.5 bg-zinc-900/80 backdrop-blur-md px-3 py-1 rounded-full border border-zinc-700/60 font-sans">
              <i class="ph ph-trophy text-amber-400"></i>
              {{ heroGame.achievements.filter(a => a.unlocked).length }} / {{ heroGame.totalAchievementsCount || heroGame.achievements.length }} Succès
            </span>
            <span class="text-zinc-400 hidden md:inline">
              Dernière session : {{ heroGame.lastPlayed }}
            </span>
          </div>

          <!-- Call to action buttons -->
          <div class="pt-2 flex flex-wrap items-center gap-3.5">
            <!-- Big Gold Play Button -->
            <button
              type="button"
              @click="handlePlayHero"
              :class="[
                'flex items-center gap-2.5 px-7 py-3 rounded-xl font-bold text-sm tracking-wide active:scale-95 transition-all duration-150',
                !isHeroRunning
                  ? 'bg-brand-gold text-zinc-950 shadow-[0_0_24px_rgba(245,158,11,0.35)] hover:bg-brand-gold-hover hover:shadow-[0_0_32px_rgba(245,158,11,0.5)]'
                  : 'bg-rose-600 text-white shadow-[0_0_24px_rgba(225,29,72,0.35)] hover:bg-rose-500'
              ]"
            >
              <i :class="!isHeroRunning ? 'ph-fill ph-play text-lg' : 'ph-fill ph-stop text-lg'"></i>
              <span>{{ !isHeroRunning ? 'JOUER MAINTENANT' : 'ARRÊTER LA SESSION' }}</span>
            </button>

            <!-- Detailed Game Page Button -->
            <button
              type="button"
              @click="openGameDetail(heroGame)"
              class="px-5 py-3 rounded-xl bg-zinc-900/80 hover:bg-zinc-800 border border-zinc-700/70 text-zinc-200 hover:text-white transition backdrop-blur-md text-sm font-semibold flex items-center gap-2"
            >
              <i class="ph ph-article text-base text-amber-400"></i>
              Fiche Détaillée & Runner
            </button>

            <!-- Minecraft Studio button if Minecraft -->
            <button
              v-if="heroGame.id === 'minecraft' || heroGame.platform === 'minecraft'"
              type="button"
              @click="router.push('/minecraft')"
              class="px-5 py-3 rounded-xl bg-amber-500/15 hover:bg-amber-500/25 border border-amber-500/40 text-amber-300 font-semibold text-sm transition flex items-center gap-2"
            >
              <i class="ph-fill ph-cube text-base text-amber-400"></i>
              Accéder au Minecraft Studio
            </button>
          </div>
        </div>
      </div>
    </section>

    <!-- ========================================================================= -->
    <!-- 2. RANGÉE DES DERNIERS JEUX JOUÉS (CARROUSEL HORIZONTAL)                  -->
    <!-- ========================================================================= -->
    <section class="space-y-4">
      <div class="flex items-center justify-between">
        <div class="flex items-center gap-2.5">
          <i class="ph-fill ph-fire text-amber-400 text-lg"></i>
          <h2 class="text-base font-bold text-zinc-100 tracking-tight">
            Reprendre une Partie — Derniers Jeux Joués
          </h2>
        </div>
        <span class="text-xs text-zinc-500 font-mono">{{ recentPlayedGames.length }} titres récents</span>
      </div>

      <!-- Horizontal row of 3:4 poster cards -->
      <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-4">
        <div
          v-for="game in recentPlayedGames"
          :key="game.id"
          @click="openGameDetail(game)"
          class="group relative rounded-2xl overflow-hidden bg-zinc-900/60 border border-zinc-800/80 hover:border-amber-500/50 transition-all duration-300 cursor-pointer shadow-lg hover:-translate-y-1 select-none flex flex-col"
        >
          <!-- 3:4 poster ratio -->
          <div class="relative w-full aspect-[3/4] overflow-hidden bg-zinc-950">
            <img
              :src="game.coverUrl"
              :alt="game.title"
              class="w-full h-full object-cover transition-transform duration-500 group-hover:scale-105"
            />
            <div class="absolute inset-0 bg-gradient-to-t from-zinc-950 via-zinc-950/20 to-transparent" />

            <!-- Platform badge -->
            <div class="absolute top-2.5 left-2.5">
              <UiBadge variant="zinc" size="xs">
                {{ game.platform }}
              </UiBadge>
            </div>

            <!-- Running indicator if active -->
            <div
              v-if="gamesStore.runningGameId === game.id"
              class="absolute top-2.5 right-2.5 px-2 py-0.5 rounded-full bg-amber-500 text-zinc-950 text-[10px] font-bold font-mono uppercase flex items-center gap-1 shadow"
            >
              <span class="w-1.5 h-1.5 rounded-full bg-zinc-950 animate-ping" />
              Actif
            </div>

            <!-- Quick play hover button -->
            <button
              type="button"
              @click.stop="gamesStore.launchGame(game.id)"
              class="absolute bottom-3 right-3 p-2.5 rounded-full bg-brand-gold text-zinc-950 shadow-lg opacity-0 group-hover:opacity-100 transform translate-y-2 group-hover:translate-y-0 transition-all hover:bg-brand-gold-hover hover:scale-110 active:scale-95"
              title="Lancer immédiatement"
            >
              <i class="ph-fill ph-play text-sm"></i>
            </button>
          </div>

          <div class="p-3 space-y-1">
            <h4 class="font-bold text-xs text-zinc-200 group-hover:text-amber-400 transition-colors truncate">
              {{ game.title }}
            </h4>
            <div class="flex items-center justify-between text-[11px] text-zinc-500 font-mono">
              <span>{{ (game.playtimeMinutes / 60).toFixed(0) }} h</span>
              <span>{{ game.config.runner }}</span>
            </div>
          </div>
        </div>
      </div>
    </section>

    <!-- ========================================================================= -->
    <!-- 3. BIBLIOTHÈQUE VIGNETTES AVEC RECHERCHE FUZZY, TAGS & FILTRE MODAL       -->
    <!-- ========================================================================= -->
    <section class="space-y-6 pt-4 border-t border-zinc-800/80">
      <!-- Search Toolbar & Advanced Filters Button -->
      <div class="flex flex-col md:flex-row items-start md:items-center justify-between gap-4">
        <div>
          <h2 class="text-xl font-black text-zinc-100 tracking-tight font-display flex items-center gap-2.5">
            <i class="ph-fill ph-game-controller text-amber-400 text-2xl"></i>
            Catalogue Complet des Jeux
          </h2>
          <p class="text-xs text-zinc-400 mt-0.5">
            {{ filteredGames.length }} titres affichés • Lancement natif Linux & Proton
          </p>
        </div>

        <!-- Right: Search Bar & Modal Filter Trigger -->
        <div class="flex items-center gap-3 w-full md:w-auto">
          <!-- Fuzzy Search Bar -->
          <div class="relative flex-1 md:w-80">
            <i class="ph ph-magnifying-glass absolute left-3.5 top-1/2 -translate-y-1/2 text-zinc-400"></i>
            <input
              v-model="filters.search"
              type="text"
              placeholder="Rechercher par titre, genre, runner..."
              class="w-full bg-zinc-900/80 text-zinc-100 placeholder-zinc-500 text-xs rounded-xl pl-9 pr-4 py-2.5 border border-zinc-800 focus:border-amber-500/70 focus:ring-2 focus:ring-amber-500/20 outline-none transition-all"
            />
          </div>

          <!-- Advanced Filter Modal Button -->
          <UiButton
            variant="secondary"
            size="md"
            @click="filterModalOpen = true"
            class="relative"
          >
            <template #prefix>
              <i class="ph ph-sliders-horizontal text-amber-400 text-base"></i>
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

      <!-- Quick Interactive Clickable Tags (Tag Bar) -->
      <div class="flex items-center gap-2 overflow-x-auto pb-1 no-scrollbar">
        <button
          v-for="tag in quickTags"
          :key="tag.id"
          type="button"
          @click="selectQuickTag(tag.id)"
          :class="[
            'px-3.5 py-1.5 rounded-full text-xs font-medium transition-all whitespace-nowrap flex items-center gap-1.5 border select-none',
            selectedTag === tag.id
              ? 'bg-brand-gold text-zinc-950 font-bold border-amber-400 shadow-[0_0_12px_rgba(245,158,11,0.3)]'
              : 'bg-zinc-900/70 text-zinc-400 border-zinc-800/80 hover:text-zinc-200 hover:border-zinc-700'
          ]"
        >
          <i :class="['ph', tag.icon, 'text-xs']"></i>
          <span>{{ tag.label }}</span>
        </button>

        <button
          v-if="activeFiltersCount > 0"
          type="button"
          @click="resetFilters"
          class="px-3 py-1.5 rounded-full text-xs font-medium text-rose-400 hover:text-rose-300 hover:bg-rose-500/10 transition-colors ml-auto flex-shrink-0"
        >
          Réinitialiser filtres
        </button>
      </div>

      <!-- 3:4 Movie Poster Grid -->
      <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-5">
        <GameCard
          v-for="game in filteredGames"
          :key="game.id"
          :game="game"
          @select="openGameDetail"
          @configure="configureRunner"
        />
      </div>

      <!-- Empty State -->
      <div
        v-if="filteredGames.length === 0"
        class="py-16 text-center space-y-3 rounded-2xl bg-zinc-900/30 border border-zinc-800/60"
      >
        <div class="w-12 h-12 rounded-full bg-zinc-800/80 mx-auto flex items-center justify-center text-zinc-400">
          <i class="ph ph-magnifying-glass text-xl"></i>
        </div>
        <p class="text-zinc-300 font-medium text-sm">
          Aucun jeu ne correspond à vos filtres actuels.
        </p>
        <p class="text-zinc-500 text-xs">
          Essayez de modifier votre terme de recherche ou de réinitialiser les filtres appliqués.
        </p>
        <UiButton
          variant="outline"
          size="sm"
          @click="resetFilters"
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
