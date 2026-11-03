<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useGamesStore } from '@/stores/gamesStore'
import { useIgdbStore } from '@/stores/igdbStore'
import { useToastStore } from '@/stores/toastStore'
import type { Game, GameFilter } from '@/models/game'
import GameCard from '@/components/games/GameCard.vue'
import GameFilterModal from '@/components/games/GameFilterModal.vue'
import GameRunnerModal from '@/components/games/GameRunnerModal.vue'
import GameInstallModal from '@/components/games/GameInstallModal.vue'
import IgdbSearchModal from '@/components/igdb/IgdbSearchModal.vue'
import UiButton from '@/components/ui/UiButton.vue'
import UiBadge from '@/components/ui/UiBadge.vue'
import { SteamIcon } from '@/components/icons/platforms'

const router = useRouter()
const route = useRoute()
const gamesStore = useGamesStore()
const igdbStore = useIgdbStore()
const toast = useToastStore()

const filterModalOpen = ref(false)
const runnerModalOpen = ref(false)
const igdbModalOpen = ref(false)
const installModalOpen = ref(false)
const selectedGameForInstall = ref<Game | null>(null)
const selectedGameForRunner = ref<Game | null>(null)

// Chargement progressif au scroll (évite d'instancier 800+ composants d'un coup)
const PAGE_SIZE = 36
const displayedCount = ref(PAGE_SIZE)
const sentinelRef = ref<HTMLElement | null>(null)
let scrollObserver: IntersectionObserver | null = null

function loadMoreGames() {
  if (displayedCount.value < filteredGames.value.length) {
    displayedCount.value = Math.min(displayedCount.value + PAGE_SIZE, filteredGames.value.length)
  }
}

onMounted(() => {
  scrollObserver = new IntersectionObserver(
    (entries) => {
      if (entries[0]?.isIntersecting) {
        loadMoreGames()
      }
    },
    { rootMargin: '300px' }
  )

  if (sentinelRef.value) {
    scrollObserver.observe(sentinelRef.value)
  }
})

onUnmounted(() => {
  scrollObserver?.disconnect()
})

watch(sentinelRef, (newEl) => {
  if (scrollObserver && newEl) {
    scrollObserver.disconnect()
    scrollObserver.observe(newEl)
  }
})

// 1. HERO : Dernier jeu joué (null si bibliothèque vide)
const heroGame = computed<Game | null>(() => {
  return gamesStore.heroGame
})

const isHeroRunning = computed(() => {
  return heroGame.value ? gamesStore.runningGameId === heroGame.value.id : false
})

const isHeroStarting = computed(() => {
  return heroGame.value ? gamesStore.launchingGameId === heroGame.value.id : false
})

async function handlePlayHero() {
  if (!heroGame.value) return
  if (!isHeroRunning.value) {
    if (!heroGame.value.installed) {
      selectedGameForInstall.value = heroGame.value
      installModalOpen.value = true
      return
    }
    await gamesStore.launchGame(heroGame.value.id)
    toast.show(`Transmission de l'ordre de lancement à Steam pour ${heroGame.value.title}...`, 'info')
  } else {
    await gamesStore.stopGame(heroGame.value.id)
    toast.show('Demande d\'arrêt transmise.', 'info')
  }
}

// Avancement réel des succès du Hero
const heroUnlockedAchievements = computed(() => {
  if (!heroGame.value) return 0
  const realUnlocked = heroGame.value.achievements?.filter((a) => a.unlocked).length || 0
  if (realUnlocked > 0) return realUnlocked
  return heroGame.value.unlockedAchievementsCount || 0
})

const heroTotalAchievements = computed(() => {
  if (!heroGame.value) return 0
  return heroGame.value.totalAchievementsCount || heroGame.value.achievements?.length || 0
})

const heroAchievementPercent = computed(() => {
  if (!heroTotalAchievements.value) return 0
  return Math.round((heroUnlockedAchievements.value / heroTotalAchievements.value) * 100)
})

// 2. RECENT GAMES
const recentPlayedGames = computed(() => {
  return gamesStore.recentGames
})

// 3. SEARCH, QUICK TAGS & FILTER STATE
const initialSearch = (route.query.q as string) || ''
const selectedTag = ref<string>('all')

const quickTags = computed(() => [
  { id: 'all', label: `Tous (${gamesStore.totalCount})`, icon: 'ph-squares-four' },
  { id: 'installed', label: `Installés (${gamesStore.installedCount})`, icon: 'ph-hard-drive' },
  { id: 'uninstalled', label: `Non installés (${gamesStore.uninstalledCount})`, icon: 'ph-cloud-arrow-down' },
  { id: 'favorites', label: 'Favoris ⭐', icon: 'ph-star' },
  { id: 'steam', label: 'Steam', icon: 'ph-game-controller' }
])

const filters = ref<GameFilter>({
  search: initialSearch,
  status: 'all',
  platform: 'all',
  genre: 'all',
  runner: 'all',
  sortBy: 'lastPlayed',
  sortOrder: 'desc'
})

function selectQuickTag(tagId: string) {
  selectedTag.value = tagId

  if (tagId === 'all') {
    filters.value.status = 'all'
    filters.value.genre = 'all'
    filters.value.platform = 'all'
    filters.value.runner = 'all'
  } else if (tagId === 'installed') {
    filters.value.status = 'installed'
    filters.value.genre = 'all'
    filters.value.platform = 'all'
    filters.value.runner = 'all'
  } else if (tagId === 'uninstalled') {
    filters.value.status = 'uninstalled'
    filters.value.genre = 'all'
    filters.value.platform = 'all'
    filters.value.runner = 'all'
  } else if (tagId === 'favorites') {
    filters.value.status = 'all'
  } else if (tagId === 'steam') {
    filters.value.platform = tagId
    filters.value.status = 'all'
    filters.value.genre = 'all'
    filters.value.runner = 'all'
  } else {
    filters.value.genre = tagId
    filters.value.status = 'all'
    filters.value.platform = 'all'
    filters.value.runner = 'all'
  }
}

const filteredGames = computed(() => {
  let list = gamesStore.filterGames(filters.value).filter((g) => g.inLibrary !== false)
  if (selectedTag.value === 'favorites') {
    list = list.filter((g) => g.isFavorite)
  }
  return list
})

const displayedGames = computed(() => {
  return filteredGames.value.slice(0, displayedCount.value)
})

const hasMoreGames = computed(() => {
  return displayedCount.value < filteredGames.value.length
})

watch(
  [() => filters.value, () => selectedTag.value],
  () => {
    displayedCount.value = PAGE_SIZE
  },
  { deep: true }
)

function openIgdbWithSearch(query?: string) {
  igdbModalOpen.value = true
  if (query && query.trim()) {
    igdbStore.search(query.trim())
  }
}

function openGameDetail(game: Game) {
  router.push(`/games/${game.slug || game.id}`)
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
    status: 'all',
    platform: 'all',
    genre: 'all',
    runner: 'all',
    sortBy: 'lastPlayed',
    sortOrder: 'desc'
  }
}

function scrollToCatalog() {
  const el = document.getElementById('catalog-section')
  el?.scrollIntoView({ behavior: 'smooth' })
}
</script>

<template>
  <div class="w-full pb-20 selection:bg-amber-500/30 selection:text-amber-200">
    <!-- ========================================================================= -->
    <!-- 1. HERO DU DERNIER JEU JOUÉ EN FULL PAGE                                  -->
    <!--    (Appel au jeu, nombre d'heures, avancement des succès)                 -->
    <!-- ========================================================================= -->
    <section
      v-if="heroGame"
      class="relative w-full h-[calc(100vh-53px)] min-h-[580px] flex flex-col justify-end overflow-hidden select-none"
    >
      <!-- Background Artwork Full-Page -->
      <div class="absolute inset-0 z-0">
        <img
          :src="heroGame.bannerUrl"
          :alt="heroGame.title"
          class="w-full h-full object-cover object-[65%_25%] scale-105 transition-transform duration-1000 ease-out"
        />
        <!-- Ambient radial glow & dark vignettes matching Aureus reference -->
        <div class="absolute inset-0 hero-glow mix-blend-screen pointer-events-none" />
        <div class="absolute inset-0 hero-vignette pointer-events-none" />
        <div class="absolute inset-0 bg-[radial-gradient(#ffffff08_1px,transparent_1px)] [background-size:24px_24px] pointer-events-none" />
      </div>

      <!-- Hero Overlay : Appel au jeu, nombre d'heures et avancement des succès -->
      <div class="relative z-20 px-6 sm:px-12 lg:px-16 pb-16 max-w-5xl">
        <div class="space-y-6">
          <!-- Titre stylisé du jeu -->
          <div class="space-y-1">
            <h1 class="font-cinzel text-4xl sm:text-6xl lg:text-7xl font-black tracking-wider text-white drop-shadow-[0_6px_32px_rgba(0,0,0,0.9)]">
              {{ heroGame.title }}
            </h1>
            <div v-if="heroGame.subtitle" class="flex items-center justify-between -mt-1 sm:-mt-2 max-w-xs sm:max-w-sm">
              <span class="h-[1px] flex-1 bg-gradient-to-r from-transparent via-zinc-400 to-transparent"></span>
              <span class="font-cinzel text-lg sm:text-2xl font-bold tracking-[0.3em] text-amber-200 px-3 drop-shadow-md">
                {{ heroGame.subtitle }}
              </span>
              <span class="h-[1px] flex-1 bg-gradient-to-r from-transparent via-zinc-400 to-transparent"></span>
            </div>
          </div>

          <!-- Ligne épurée : Appel au Jeu (Play CTA) + Heures + Avancement des Succès -->
          <div class="flex flex-wrap items-center gap-6 pt-2">
            <!-- Appel au Jeu (Bouton Jouer / Play CTA) -->
            <button
              type="button"
              @click="handlePlayHero"
              :class="[
                'flex items-center gap-3 px-8 py-3.5 rounded-xl font-bold text-base tracking-wide active:scale-95 transition-all duration-150',
                isHeroRunning
                  ? 'bg-rose-600 hover:bg-rose-500 text-white shadow-[0_0_28px_rgba(225,29,72,0.4)]'
                  : isHeroStarting
                    ? 'bg-amber-500 text-zinc-950 shadow-[0_0_28px_rgba(245,158,11,0.4)]'
                    : heroGame.installed
                      ? 'btn-gold shadow-lg'
                      : 'bg-[#1a9fff] hover:bg-[#158ce0] text-white shadow-[0_0_28px_rgba(26,159,255,0.4)]'
              ]"
            >
              <i v-if="isHeroRunning" class="ph-fill ph-stop text-xl"></i>
              <i v-else-if="isHeroStarting" class="ph ph-spinner animate-spin text-xl"></i>
              <i v-else-if="heroGame.installed" class="ph-fill ph-play text-xl"></i>
              <i v-else class="ph-bold ph-download-simple text-xl"></i>

              <span>
                {{
                  isHeroRunning
                    ? 'ARRÊTER'
                    : isHeroStarting
                      ? 'DÉMARRAGE STEAM...'
                      : heroGame.installed
                        ? 'JOUER'
                        : 'INSTALLER'
                }}
              </span>
            </button>

            <!-- Métriques groupées : Nombre d'heures & Avancement des Succès -->
            <div class="flex flex-wrap items-center gap-6 sm:gap-8 px-6 py-3 rounded-2xl bg-zinc-900/80 border border-zinc-800/80 backdrop-blur-md">
              <!-- Nombre d'heures jouées -->
              <div class="flex items-center gap-3">
                <div class="w-10 h-10 rounded-xl bg-amber-500/10 border border-amber-500/30 flex items-center justify-center text-amber-400">
                  <i class="ph ph-clock text-xl"></i>
                </div>
                <div>
                  <span class="text-[10px] tracking-wider uppercase font-semibold text-zinc-400 block font-mono">Temps de jeu</span>
                  <p class="font-bold text-sm text-zinc-100 font-mono">
                    {{ (heroGame.playtimeMinutes / 60).toFixed(1) }} h
                  </p>
                </div>
              </div>

              <div class="h-8 w-px bg-zinc-800/80 hidden sm:block" />

              <!-- Avancement des Succès -->
              <div class="flex items-center gap-3">
                <div class="w-10 h-10 rounded-xl bg-amber-500/10 border border-amber-500/30 flex items-center justify-center text-amber-400">
                  <i class="ph ph-trophy text-xl"></i>
                </div>
                <div v-if="heroTotalAchievements > 0" class="space-y-1">
                  <div class="flex items-center justify-between gap-4">
                    <span class="text-[10px] tracking-wider uppercase font-semibold text-zinc-400 font-mono">Succès débloqués</span>
                    <span class="font-bold text-xs text-amber-300 font-mono">
                      {{ heroUnlockedAchievements }}/{{ heroTotalAchievements }} ({{ heroAchievementPercent }}%)
                    </span>
                  </div>
                  <div class="w-32 h-1.5 rounded-full bg-zinc-800 overflow-hidden">
                    <div
                      class="h-full bg-brand-gold rounded-full shadow-[0_0_8px_rgba(245,158,11,0.5)] transition-all duration-500"
                      :style="{ width: `${heroAchievementPercent}%` }"
                    />
                  </div>
                </div>
                <div v-else>
                  <span class="text-[10px] tracking-wider uppercase font-semibold text-zinc-400 block font-mono">Succès</span>
                  <p class="font-bold text-xs text-zinc-400 font-mono">Non répertoriés</p>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Indicateur de défilement vers la bibliothèque -->
      <button
        type="button"
        @click="scrollToCatalog"
        class="absolute bottom-4 left-1/2 -translate-x-1/2 z-20 flex flex-col items-center gap-1 text-xs text-zinc-400 hover:text-amber-400 transition-colors cursor-pointer group"
      >
        <span class="text-[10px] font-mono tracking-widest uppercase opacity-75 group-hover:opacity-100">BIBLIOTHÈQUE</span>
        <i class="ph ph-caret-down text-base animate-bounce"></i>
      </button>
    </section>

    <!-- ========================================================================= -->
    <!-- ÉCRAN D'ACCUEIL BIBLIOTHÈQUE VIDE (ZÉRO FAUSSE DONNÉE)                     -->
    <!-- ========================================================================= -->
    <div v-if="gamesStore.games.length === 0" class="max-w-4xl mx-auto px-6 py-16 text-center space-y-10">
      <div class="space-y-4">
        <div class="w-20 h-20 rounded-3xl bg-amber-500/10 border border-amber-500/30 flex items-center justify-center text-amber-400 mx-auto shadow-2xl">
          <i class="ph-bold ph-game-controller text-4xl"></i>
        </div>
        <div class="space-y-2">
          <h1 class="text-3xl sm:text-4xl font-black text-white tracking-tight font-cinzel">
            Votre Bibliothèque Loom est Vide
          </h1>
          <p class="text-sm text-zinc-400 max-w-xl mx-auto leading-relaxed">
            Aucun jeu n'a encore été détecté dans votre stockage local. Loom est connecté en temps réel aux métadonnées officielles IGDB sans aucune donnée fictive.
          </p>
        </div>
      </div>

      <!-- Quick Action Cards Grid -->
      <div class="grid grid-cols-1 md:grid-cols-2 gap-4 text-left">
        <!-- Carte Steam -->
        <div
          @click="router.push('/accounts')"
          class="p-6 rounded-2xl bg-zinc-900/60 border border-zinc-800 hover:border-amber-500/60 hover:bg-zinc-900/90 transition-all duration-200 cursor-pointer group flex flex-col justify-between space-y-4"
        >
          <div class="flex items-start justify-between">
            <div class="w-12 h-12 rounded-xl bg-[#1a9fff]/10 border border-[#1a9fff]/30 flex items-center justify-center text-[#1a9fff] group-hover:scale-105 transition-transform">
              <SteamIcon className="w-6 h-6" :colored="true" />
            </div>
            <span class="text-[11px] font-mono px-2.5 py-1 rounded-full bg-zinc-800 text-amber-300 border border-zinc-700">
              Prochaine étape
            </span>
          </div>
          <div>
            <h3 class="text-base font-bold text-white group-hover:text-amber-400 transition-colors">
              Lier votre compte Steam
            </h3>
            <p class="text-xs text-zinc-400 mt-1 leading-relaxed">
              Détectez automatiquement vos jeux installés, votre historique de temps de jeu et vos succès Steam officiels.
            </p>
          </div>
          <div class="flex items-center gap-1.5 text-xs font-semibold text-amber-400 group-hover:translate-x-1 transition-transform">
            <span>Configurer la liaison</span>
            <i class="ph ph-arrow-right"></i>
          </div>
        </div>

        <!-- Carte IGDB Explore -->
        <div
          @click="openIgdbWithSearch('')"
          class="p-6 rounded-2xl bg-zinc-900/60 border border-zinc-800 hover:border-amber-500/60 hover:bg-zinc-900/90 transition-all duration-200 cursor-pointer group flex flex-col justify-between space-y-4"
        >
          <div class="flex items-start justify-between">
            <div class="w-12 h-12 rounded-xl bg-purple-500/10 border border-purple-500/30 flex items-center justify-center text-purple-400 text-2xl group-hover:scale-105 transition-transform">
              <i class="ph-bold ph-globe-hemisphere-west"></i>
            </div>
            <span class="text-[11px] font-mono px-2.5 py-1 rounded-full bg-zinc-800 text-zinc-300 border border-zinc-700">
              API IGDB v4
            </span>
          </div>
          <div>
            <h3 class="text-base font-bold text-white group-hover:text-amber-400 transition-colors">
              Explorer le catalogue IGDB
            </h3>
            <p class="text-xs text-zinc-400 mt-1 leading-relaxed">
              Recherchez des fiches officielles par slug, visualisez les artworks 1080p et ajoutez vos jeux manuellement.
            </p>
          </div>
          <div class="flex items-center gap-1.5 text-xs font-semibold text-amber-400 group-hover:translate-x-1 transition-transform">
            <span>Ouvrir le catalogue</span>
            <i class="ph ph-arrow-right"></i>
          </div>
        </div>
      </div>

      <!-- Action Footer: Refresh / Rescan -->
      <div class="flex items-center justify-center gap-3 pt-2">
        <UiButton
          variant="secondary"
          size="md"
          :loading="gamesStore.isScanningLibrary"
          @click="gamesStore.fetchLibrary()"
        >
          <template #prefix>
            <i class="ph ph-arrows-clockwise text-base"></i>
          </template>
          Actualiser la détection locale
        </UiButton>
      </div>
    </div>

    <!-- ========================================================================= -->
    <!-- CONTENU SI JEUX EN BIBLIOTHÈQUE : CARROUSEL RÉCENTS & CATALOGUE COMPLET    -->
    <!-- ========================================================================= -->
    <div v-else id="catalog-section" class="max-w-7xl mx-auto px-6 sm:px-10 lg:px-14 py-10 space-y-10">
      <!-- 2. RANGÉE DES DERNIERS JEUX JOUÉS (CARROUSEL HORIZONTAL) -->
      <section v-if="recentPlayedGames.length > 0" class="space-y-4">
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-2.5">
            <i class="ph-fill ph-fire text-amber-400 text-lg"></i>
            <h2 class="text-base font-bold text-zinc-100 tracking-tight">
              Reprendre une Partie — Derniers Jeux Joués
            </h2>
          </div>
          <span class="text-xs text-zinc-500 font-mono">{{ recentPlayedGames.length }} titres récents</span>
        </div>

        <!-- Carrousel horizontal d'affiches 3:4 -->
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
                class="absolute bottom-3 right-3 p-2.5 rounded-full btn-gold shadow-lg opacity-0 group-hover:opacity-100 transform translate-y-2 group-hover:translate-y-0 transition-all hover:scale-110 active:scale-95"
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

      <!-- 3. BIBLIOTHÈQUE VIGNETTES AVEC RECHERCHE FUZZY, TAGS & FILTRE MODAL -->
      <section class="space-y-6 pt-4 border-t border-zinc-800/80">
        <!-- Search Toolbar & Advanced Filters Button -->
        <div class="flex flex-col md:flex-row items-start md:items-center justify-between gap-4">
          <div>
            <h2 class="text-xl font-black text-zinc-100 tracking-tight font-display flex items-center gap-2.5">
              <i class="ph-fill ph-game-controller text-amber-400 text-2xl"></i>
              Bibliothèque de Jeux
            </h2>
            <p class="text-xs text-zinc-400 mt-1 flex flex-wrap items-center gap-2">
              <span class="inline-flex items-center gap-1.5 font-semibold text-emerald-400">
                <span class="w-1.5 h-1.5 rounded-full bg-emerald-400" />
                {{ gamesStore.installedCount }} installés
              </span>
              <span>•</span>
              <span class="inline-flex items-center gap-1.5 font-semibold text-zinc-300">
                <span class="w-1.5 h-1.5 rounded-full bg-zinc-500" />
                {{ gamesStore.uninstalledCount }} non installés
              </span>
              <span class="text-zinc-500 font-mono">({{ filteredGames.length }} affichés sur {{ gamesStore.totalCount }})</span>
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

            <!-- Découvrir Hors Bibliothèque (IGDB) Button -->
            <UiButton
              variant="primary"
              size="md"
              @click="openIgdbWithSearch('')"
            >
              <template #prefix>
                <i class="ph-bold ph-globe-hemisphere-west text-base"></i>
              </template>
              Découvrir hors bibliothèque
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
                ? 'tag-selected'
                : 'bg-zinc-900/80 text-zinc-300 border-zinc-800 hover:text-white hover:border-zinc-700'
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
            v-for="game in displayedGames"
            :key="game.id"
            :game="game"
            @select="openGameDetail"
            @configure="configureRunner"
          />
        </div>

        <!-- Scroll Sentinel & Chargement progressif -->
        <div
          v-if="hasMoreGames"
          ref="sentinelRef"
          class="w-full py-10 flex flex-col items-center justify-center gap-2.5 text-zinc-400 select-none"
        >
          <div class="w-7 h-7 rounded-full border-2 border-amber-500/30 border-t-amber-500 animate-spin" />
          <p class="text-xs text-zinc-400 font-mono tracking-wide">
            {{ displayedGames.length }} sur {{ filteredGames.length }} jeux chargés • Défilez pour afficher la suite
          </p>
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

          <!-- Prompt to search on IGDB when not in local library -->
          <div v-if="filters.search" class="pt-2">
            <UiButton
              variant="primary"
              size="sm"
              @click="openIgdbWithSearch(filters.search)"
            >
              <template #prefix>
                <i class="ph-bold ph-magnifying-glass-plus text-base"></i>
              </template>
              Rechercher "{{ filters.search }}" hors bibliothèque sur IGDB
            </UiButton>
          </div>

          <div class="pt-1">
            <UiButton
              variant="outline"
              size="sm"
              @click="resetFilters"
            >
              Réinitialiser tous les filtres
            </UiButton>
          </div>
        </div>
      </section>
    </div>

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

    <IgdbSearchModal
      :open="igdbModalOpen"
      @close="igdbModalOpen = false"
      @imported="(id) => router.push(`/games/${id}`)"
    />

    <GameInstallModal
      v-if="selectedGameForInstall"
      :game="selectedGameForInstall"
      :is-open="installModalOpen"
      @close="installModalOpen = false"
      @installed="installModalOpen = false"
    />
  </div>
</template>
