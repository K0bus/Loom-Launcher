<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'
import { useRouter } from 'vue-router'
import { useIgdbStore, type IGDBTab } from '@/stores/igdbStore'
import { useGamesStore } from '@/stores/gamesStore'
import type { IGDBGame } from '@/models/igdb'
import { getIGDBCoverUrl, getIGDBArtworkUrl, getIGDBScreenshotUrl, mapIGDBGameToLoomGame } from '@/models/igdb'
import UiModal from '@/components/ui/UiModal.vue'
import UiButton from '@/components/ui/UiButton.vue'
import UiBadge from '@/components/ui/UiBadge.vue'

const router = useRouter()

interface Props {
  open: boolean
  targetSyncGameId?: string
}

const props = defineProps<Props>()
const emit = defineEmits<{
  (e: 'close'): void
  (e: 'imported', gameId: string): void
  (e: 'synced', gameId: string): void
}>()

const igdbStore = useIgdbStore()
const gamesStore = useGamesStore()

const localQuery = ref('')
const selectedGame = ref<IGDBGame | null>(null)
let debounceTimeout: any = null

function onTabChange(tab: IGDBTab) {
  igdbStore.loadCatalog(tab)
}

function handleInput() {
  clearTimeout(debounceTimeout)
  debounceTimeout = setTimeout(() => {
    igdbStore.search(localQuery.value)
  }, 350)
}

function selectGame(game: IGDBGame) {
  selectedGame.value = game
}

function isAlreadyInLibrary(game: IGDBGame): boolean {
  return gamesStore.games.some(
    (g) => g.igdbId === game.id || g.slug === game.slug || g.title.toLowerCase() === game.name.toLowerCase()
  )
}

function handleImport(game: IGDBGame) {
  const success = igdbStore.importToLibrary(game)
  if (success) {
    emit('imported', game.slug || `igdb-${game.id}`)
  }
}

function handleSyncWithTarget(game: IGDBGame) {
  if (!props.targetSyncGameId) return
  igdbStore.syncGameMetadata(props.targetSyncGameId, game)
  emit('synced', props.targetSyncGameId)
  emit('close')
}

function openFullscreenPage(game: IGDBGame) {
  const isOwned = isAlreadyInLibrary(game)
  const mapped = mapIGDBGameToLoomGame(game, {
    inLibrary: isOwned
  })
  gamesStore.registerDiscoveredGame(mapped)
  emit('close')
  router.push(`/games/${game.slug || `igdb-${game.id}`}`)
}

onMounted(() => {
  if (igdbStore.searchResults.length === 0) {
    igdbStore.loadCatalog('popular')
  }
})

watch(
  () => props.open,
  (isOpen) => {
    if (isOpen && igdbStore.searchResults.length === 0) {
      igdbStore.loadCatalog('popular')
    }
  }
)
</script>

<template>
  <UiModal
    :open="open"
    title="Rechercher un jeu hors bibliothèque (IGDB)"
    subtitle="Explorez le catalogue mondial pour découvrir, prévisualiser et ajouter des jeux à Loom"
    max-width="3xl"
    @close="emit('close')"
  >
    <div class="space-y-6">
      <!-- Search & Filters Toolbar -->
      <div class="flex flex-col md:flex-row items-stretch md:items-center justify-between gap-4">
        <!-- Search input -->
        <div class="relative flex-1">
          <i class="ph ph-magnifying-glass absolute left-3.5 top-1/2 -translate-y-1/2 text-zinc-500 text-lg"></i>
          <input
            v-model="localQuery"
            type="text"
            placeholder="Rechercher un jeu (ex: Silksong, Cyberpunk, Elden Ring, Hades...)"
            class="w-full bg-zinc-950/80 border border-zinc-800 rounded-xl pl-10 pr-10 py-2.5 text-sm text-zinc-100 placeholder-zinc-500 outline-none focus:border-amber-500 transition-colors"
            @input="handleInput"
          />
          <button
            v-if="localQuery"
            type="button"
            @click="localQuery = ''; igdbStore.loadCatalog('popular')"
            class="absolute right-3.5 top-1/2 -translate-y-1/2 text-zinc-500 hover:text-zinc-200"
          >
            <i class="ph ph-x text-base"></i>
          </button>
        </div>

        <!-- Filter tabs -->
        <div class="flex items-center gap-1.5 p-1 rounded-xl bg-zinc-950/80 border border-zinc-800/80 overflow-x-auto text-xs">
          <button
            type="button"
            :class="[
              'px-3 py-1.5 rounded-lg font-medium transition-all whitespace-nowrap flex items-center gap-1.5',
              igdbStore.activeTab === 'popular'
                ? 'tag-selected'
                : 'text-zinc-400 hover:text-zinc-200 hover:bg-zinc-900'
            ]"
            @click="onTabChange('popular')"
          >
            <i class="ph-fill ph-fire text-amber-400"></i>
            Populaires
          </button>
          <button
            type="button"
            :class="[
              'px-3 py-1.5 rounded-lg font-medium transition-all whitespace-nowrap flex items-center gap-1.5',
              igdbStore.activeTab === 'top_rated'
                ? 'tag-selected'
                : 'text-zinc-400 hover:text-zinc-200 hover:bg-zinc-900'
            ]"
            @click="onTabChange('top_rated')"
          >
            <i class="ph-fill ph-star text-amber-400"></i>
            Mieux Notés
          </button>
          <button
            type="button"
            :class="[
              'px-3 py-1.5 rounded-lg font-medium transition-all whitespace-nowrap flex items-center gap-1.5',
              igdbStore.activeTab === 'upcoming'
                ? 'tag-selected'
                : 'text-zinc-400 hover:text-zinc-200 hover:bg-zinc-900'
            ]"
            @click="onTabChange('upcoming')"
          >
            <i class="ph-fill ph-calendar text-amber-400"></i>
            Prochaines Sorties
          </button>
        </div>
      </div>

      <!-- Sync Context Banner (if opening for a specific game) -->
      <div
        v-if="targetSyncGameId"
        class="p-3.5 rounded-xl bg-amber-500/10 border border-amber-500/30 flex items-center justify-between text-xs"
      >
        <div class="flex items-center gap-2.5">
          <i class="ph-fill ph-arrows-clockwise text-amber-400 text-base"></i>
          <span class="text-amber-200">
            Mode Synchronisation : Sélectionnez un jeu IGDB pour mettre à jour automatiquement les jaquettes, résumés et métadonnées de votre jeu actuel.
          </span>
        </div>
      </div>

      <!-- Main Layout: Grid or Detail View -->
      <div v-if="!selectedGame" class="space-y-4">
        <!-- Results stats -->
        <div class="flex items-center justify-between text-xs text-zinc-500">
          <span>
            {{ igdbStore.searchResults.length }} titre{{ igdbStore.searchResults.length > 1 ? 's' : '' }} trouvé{{ igdbStore.searchResults.length > 1 ? 's' : '' }}
          </span>
          <div class="flex items-center gap-2 font-mono text-[11px]">
            <span class="w-2 h-2 rounded-full bg-emerald-500"></span>
            <span class="text-zinc-400">API IGDB Connectée (Token Public)</span>
          </div>
        </div>

        <!-- Grid Cards -->
        <div
          v-if="!igdbStore.isSearching && igdbStore.searchResults.length > 0"
          class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 gap-4 max-h-[60vh] overflow-y-auto pr-1"
        >
          <div
            v-for="game in igdbStore.searchResults"
            :key="game.id"
            class="group relative rounded-2xl bg-zinc-900/60 border border-zinc-800/80 hover:border-amber-500/50 transition-all overflow-hidden flex flex-col cursor-pointer"
            @click="selectGame(game)"
          >
            <!-- 3:4 Poster Image -->
            <div class="relative aspect-[3/4] bg-zinc-950 overflow-hidden flex items-center justify-center">
              <img
                :src="game.cover?.url || getIGDBCoverUrl(game.cover?.image_id, 'cover_big')"
                :alt="game.name"
                class="w-full h-full object-cover group-hover:scale-105 transition-transform duration-500"
                loading="lazy"
                @error="(e: any) => { e.target.style.display = 'none'; if (e.target.nextElementSibling) e.target.nextElementSibling.style.display = 'flex' }"
              />
              <div class="hidden absolute inset-0 bg-zinc-950 flex-col items-center justify-center p-3 text-center border-b border-zinc-800">
                <i class="ph-fill ph-game-controller text-2xl text-amber-400 mb-1"></i>
                <span class="text-[10px] font-bold text-zinc-300 line-clamp-2">{{ game.name }}</span>
              </div>

              <!-- Rating Badge -->
              <div
                v-if="game.total_rating"
                class="absolute top-2 right-2 px-2 py-0.5 rounded-lg bg-zinc-950/80 backdrop-blur-md border border-zinc-700/60 text-[11px] font-bold text-amber-400 font-mono flex items-center gap-1 shadow-md"
              >
                <i class="ph-fill ph-star text-xs"></i>
                {{ Math.round(game.total_rating) }}%
              </div>

              <!-- In Library Indicator -->
              <div
                v-if="isAlreadyInLibrary(game)"
                class="absolute bottom-2 left-2 px-2 py-0.5 rounded-lg bg-emerald-950/80 backdrop-blur-md border border-emerald-500/40 text-[10px] font-bold text-emerald-400 flex items-center gap-1 shadow-md"
              >
                <i class="ph-fill ph-check-circle"></i>
                En bibliothèque
              </div>
            </div>

            <!-- Card Body -->
            <div class="p-3 flex-1 flex flex-col justify-between space-y-2">
              <div>
                <h4 class="font-bold text-xs text-zinc-100 group-hover:text-amber-400 transition-colors line-clamp-1">
                  {{ game.name }}
                </h4>
                <p class="text-[10px] text-zinc-500 line-clamp-1 mt-0.5">
                  {{ game.involved_companies?.[0]?.company.name || 'Studio' }}
                  <span v-if="game.first_release_date">
                    • {{ new Date(game.first_release_date * 1000).getFullYear() }}
                  </span>
                </p>
              </div>

              <!-- Card Actions: Voir la fiche + Importer / Sync -->
              <div class="pt-1 flex items-center gap-1.5">
                <!-- Voir la page du jeu -->
                <button
                  type="button"
                  @click.stop="openFullscreenPage(game)"
                  class="p-1.5 rounded-lg bg-zinc-800/80 hover:bg-zinc-700 text-zinc-300 hover:text-amber-400 transition-colors flex-shrink-0"
                  title="Voir la page du jeu en plein écran"
                >
                  <i class="ph-bold ph-arrow-up-right text-sm"></i>
                </button>

                <!-- Quick Action Button -->
                <button
                  v-if="targetSyncGameId"
                  type="button"
                  @click.stop="handleSyncWithTarget(game)"
                  class="flex-1 py-1.5 rounded-lg text-xs font-bold btn-gold flex items-center justify-center gap-1"
                >
                  <i class="ph-fill ph-arrows-clockwise text-xs"></i>
                  Sync
                </button>
                <button
                  v-else-if="!isAlreadyInLibrary(game)"
                  type="button"
                  @click.stop="handleImport(game)"
                  class="flex-1 py-1.5 rounded-lg text-xs font-bold btn-gold flex items-center justify-center gap-1"
                >
                  <i class="ph-fill ph-plus-circle text-xs"></i>
                  Importer
                </button>
                <div
                  v-else
                  class="flex-1 py-1.5 rounded-lg text-[11px] font-medium text-center bg-zinc-800/60 text-zinc-400 border border-zinc-700/40"
                >
                  Possédé
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- Loading State -->
        <div v-if="igdbStore.isSearching" class="py-24 text-center space-y-3">
          <i class="ph ph-circle-notch animate-spin text-3xl text-amber-400"></i>
          <p class="text-xs text-zinc-400">Interrogation des endpoints IGDB en cours...</p>
        </div>

        <!-- Empty State -->
        <div
          v-if="!igdbStore.isSearching && igdbStore.searchResults.length === 0"
          class="py-20 text-center space-y-3 bg-zinc-950/40 rounded-2xl border border-zinc-800/60"
        >
          <i class="ph ph-game-controller text-3xl text-zinc-600"></i>
          <p class="text-sm font-semibold text-zinc-300">Aucun résultat trouvé pour cette recherche</p>
          <p class="text-xs text-zinc-500">Essayez un autre mot-clé ou réinitialisez les filtres.</p>
        </div>
      </div>

      <!-- Detail Preview Pane (When a Game Card is Clicked) -->
      <div v-else class="space-y-6 max-h-[65vh] overflow-y-auto pr-1">
        <!-- Back Button -->
        <button
          type="button"
          @click="selectedGame = null"
          class="text-xs text-zinc-400 hover:text-amber-400 flex items-center gap-1.5 transition-colors"
        >
          <i class="ph ph-arrow-left text-sm"></i>
          Retour aux résultats
        </button>

        <!-- Hero Header -->
        <div class="relative rounded-2xl overflow-hidden border border-zinc-800 shadow-xl min-h-[260px] flex items-end p-6">
          <img
            :src="selectedGame.artworks?.[0]?.url || getIGDBArtworkUrl(selectedGame.artworks?.[0]?.image_id, '1080p') || selectedGame.screenshots?.[0]?.url || getIGDBScreenshotUrl(selectedGame.screenshots?.[0]?.image_id, '1080p') || selectedGame.cover?.url || getIGDBCoverUrl(selectedGame.cover?.image_id, '1080p')"
            :alt="selectedGame.name"
            class="absolute inset-0 w-full h-full object-cover"
          />
          <div class="absolute inset-0 bg-gradient-to-t from-zinc-950 via-zinc-950/75 to-transparent"></div>

          <!-- Hero Details -->
          <div class="relative z-10 flex flex-col md:flex-row items-start md:items-end justify-between gap-6 w-full">
            <div class="flex items-end gap-5">
              <img
                :src="selectedGame.cover?.url || getIGDBCoverUrl(selectedGame.cover?.image_id, 'cover_big')"
                :alt="selectedGame.name"
                class="w-24 h-32 rounded-xl object-cover border-2 border-zinc-800 shadow-2xl flex-shrink-0 hidden sm:block"
              />
              <div class="space-y-1.5">
                <div class="flex items-center gap-2 flex-wrap">
                  <span
                    v-for="genre in selectedGame.genres"
                    :key="genre.id"
                    class="text-[10px] font-semibold uppercase px-2 py-0.5 rounded-full bg-zinc-800/80 text-zinc-300 border border-zinc-700/60"
                  >
                    {{ genre.name }}
                  </span>
                  <span
                    v-if="selectedGame.first_release_date"
                    class="text-[10px] font-mono px-2 py-0.5 rounded-full bg-amber-500/20 text-amber-400 border border-amber-500/40"
                  >
                    {{ new Date(selectedGame.first_release_date * 1000).getFullYear() }}
                  </span>
                </div>

                <h2 class="text-2xl sm:text-3xl font-black text-zinc-100 tracking-tight font-display">
                  {{ selectedGame.name }}
                </h2>

                <p class="text-xs text-zinc-400 flex items-center gap-2">
                  <span>Développeur : <strong class="text-zinc-200">{{ selectedGame.involved_companies?.find(c => c.developer)?.company.name || 'Indépendant' }}</strong></span>
                  <span v-if="selectedGame.total_rating">• Note IGDB : <strong class="text-amber-400">{{ Math.round(selectedGame.total_rating) }}/100</strong></span>
                </p>
              </div>
            </div>

            <!-- Action CTAs -->
            <div class="flex items-center gap-3">
              <!-- Voir la page du jeu en plein écran -->
              <UiButton
                variant="secondary"
                size="md"
                @click="openFullscreenPage(selectedGame)"
              >
                <template #prefix>
                  <i class="ph-bold ph-arrows-out-simple text-base text-amber-400"></i>
                </template>
                Voir la page du jeu
              </UiButton>

              <UiButton
                v-if="targetSyncGameId"
                variant="primary"
                size="md"
                @click="handleSyncWithTarget(selectedGame)"
              >
                <template #prefix>
                  <i class="ph-fill ph-arrows-clockwise text-base"></i>
                </template>
                Synchroniser
              </UiButton>

              <UiButton
                v-else-if="!isAlreadyInLibrary(selectedGame)"
                variant="primary"
                size="md"
                @click="handleImport(selectedGame)"
              >
                <template #prefix>
                  <i class="ph-fill ph-download-simple text-base"></i>
                </template>
                Importer dans Loom
              </UiButton>

              <UiBadge v-else variant="emerald" size="sm">
                <i class="ph-fill ph-check-circle mr-1"></i>
                En bibliothèque
              </UiBadge>
            </div>
          </div>
        </div>

        <!-- Metadata Sections -->
        <div class="grid grid-cols-1 md:grid-cols-3 gap-6">
          <!-- Left: Synopsis & Storyline -->
          <div class="md:col-span-2 space-y-4">
            <div class="p-5 rounded-2xl bg-zinc-900/60 border border-zinc-800/80 space-y-3">
              <h4 class="font-bold text-xs uppercase tracking-wider text-zinc-300 font-mono flex items-center gap-2">
                <i class="ph-fill ph-book-open text-amber-400 text-sm"></i>
                Synopsis & Histoire
              </h4>
              <p class="text-xs text-zinc-300 leading-relaxed">
                {{ selectedGame.summary || 'Aucun résumé disponible pour ce titre.' }}
              </p>
              <div v-if="selectedGame.storyline" class="pt-2 border-t border-zinc-800/80">
                <p class="text-xs text-zinc-400 italic leading-relaxed">
                  "{{ selectedGame.storyline }}"
                </p>
              </div>
            </div>

            <!-- Screenshots Gallery -->
            <div v-if="selectedGame.screenshots && selectedGame.screenshots.length > 0" class="space-y-2">
              <h4 class="font-bold text-xs uppercase tracking-wider text-zinc-300 font-mono">
                Captures d'écran HD
              </h4>
              <div class="grid grid-cols-3 gap-3">
                <div
                  v-for="(screen, idx) in selectedGame.screenshots.slice(0, 3)"
                  :key="idx"
                  class="rounded-xl overflow-hidden aspect-video bg-zinc-950 border border-zinc-800"
                >
                  <img
                    :src="getIGDBScreenshotUrl(screen.image_id, 'screenshot_huge')"
                    :alt="selectedGame.name"
                    class="w-full h-full object-cover hover:scale-105 transition-transform duration-300"
                  />
                </div>
              </div>
            </div>
          </div>

          <!-- Right: Details Card -->
          <div class="space-y-4">
            <div class="p-5 rounded-2xl bg-zinc-900/60 border border-zinc-800/80 space-y-4 text-xs font-mono">
              <h4 class="font-bold text-xs uppercase tracking-wider text-zinc-300 flex items-center gap-2">
                <i class="ph-fill ph-info text-amber-400 text-sm"></i>
                Fiche Technique IGDB
              </h4>

              <div class="space-y-2 text-zinc-400">
                <div class="flex justify-between py-1 border-b border-zinc-800/60">
                  <span>ID IGDB :</span>
                  <span class="text-zinc-200">#{{ selectedGame.id }}</span>
                </div>
                <div class="flex justify-between py-1 border-b border-zinc-800/60">
                  <span>Slug :</span>
                  <span class="text-amber-400 truncate max-w-[140px]">{{ selectedGame.slug }}</span>
                </div>
                <div class="flex justify-between py-1 border-b border-zinc-800/60">
                  <span>Modes de jeu :</span>
                  <span class="text-zinc-200">{{ selectedGame.game_modes?.map(m => m.name).join(', ') || 'Solo' }}</span>
                </div>
                <div class="flex justify-between py-1 border-b border-zinc-800/60">
                  <span>Plates-formes :</span>
                  <span class="text-zinc-200 truncate max-w-[140px]">{{ selectedGame.platforms?.map(p => p.abbreviation || p.name).join(', ') || 'PC' }}</span>
                </div>
              </div>

              <!-- Links -->
              <div v-if="selectedGame.websites && selectedGame.websites.length > 0" class="pt-2 space-y-1.5">
                <span class="text-[11px] text-zinc-500 uppercase tracking-wider block font-sans">Liens Officiels</span>
                <div class="flex flex-wrap gap-2">
                  <a
                    v-for="(w, idx) in selectedGame.websites.slice(0, 4)"
                    :key="idx"
                    :href="w.url"
                    target="_blank"
                    rel="noopener noreferrer"
                    class="px-2.5 py-1 rounded-lg bg-zinc-800 text-zinc-300 hover:text-amber-400 hover:bg-zinc-700/80 transition-colors text-[11px] flex items-center gap-1 font-sans"
                  >
                    <i class="ph ph-arrow-square-out"></i>
                    {{ w.url.includes('steam') ? 'Steam' : w.url.includes('cyberpunk') ? 'Site Officiel' : 'Lien Web' }}
                  </a>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </UiModal>
</template>
