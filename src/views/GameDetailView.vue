<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useGamesStore } from '@/stores/gamesStore'
import { useToastStore } from '@/stores/toastStore'
import { useDownloadsStore } from '@/stores/downloadsStore'
import type { Game, RunnerType } from '@/models/game'
import UiModal from '@/components/ui/UiModal.vue'
import UiButton from '@/components/ui/UiButton.vue'
import IgdbSearchModal from '@/components/igdb/IgdbSearchModal.vue'
import AchievementsModal from '@/components/games/AchievementsModal.vue'

const route = useRoute()
const router = useRouter()
const gamesStore = useGamesStore()
const toast = useToastStore()
const downloadsStore = useDownloadsStore()

function formatBytes(bytes: number): string {
  if (!bytes || bytes <= 0) return '0 Go'
  const gb = bytes / (1024 ** 3)
  if (gb < 0.1) {
    const mb = bytes / (1024 ** 2)
    return `${mb.toFixed(0)} Mo`
  }
  return `${gb.toFixed(1)} Go`
}

function formatSpeed(mbps: number): string {
  if (!mbps || mbps <= 0) return '0 Mo/s'
  const mbytePerSec = mbps / 8
  if (mbytePerSec >= 1000) {
    return `${(mbytePerSec / 1000).toFixed(2)} Go/s`
  }
  if (mbytePerSec < 0.1) {
    const kbytePerSec = (mbps * 1024) / 8
    return `${Math.max(1, Math.round(kbytePerSec))} Ko/s`
  }
  return `${mbytePerSec.toFixed(1)} Mo/s`
}

const igdbModalOpen = ref(false)
const buyModalOpen = ref(false)
const achievementsModalOpen = ref(false)
const isLoadingAchievements = ref(false)
const selectedScreenshot = ref<string | null>(null)
const isLoadingGame = ref(false)

const gameSlug = computed(() => (route.params.slug as string) || (route.params.id as string) || '')
const game = computed<Game | undefined>(() => {
  if (!gameSlug.value) return undefined
  return gamesStore.getGameBySlug(gameSlug.value) || gamesStore.getGameById(gameSlug.value)
})

const currentDownload = computed(() => {
  if (!game.value) return undefined
  return downloadsStore.getDownloadForGame(game.value.id, game.value.slug)
})

async function loadGameDetails() {
  if (!gameSlug.value) return
  isLoadingGame.value = true
  try {
    const existing = gamesStore.getGameBySlug(gameSlug.value) || gamesStore.getGameById(gameSlug.value)
    if (existing) {
      await gamesStore.enrichGameWithIGDB(existing)
    } else {
      await gamesStore.fetchAndRegisterGame(gameSlug.value)
    }
  } finally {
    isLoadingGame.value = false
  }
}

async function loadAchievements() {
  if (!game.value) return
  isLoadingAchievements.value = true
  try {
    await gamesStore.loadGameAchievements(game.value.id)
  } finally {
    isLoadingAchievements.value = false
  }
}

onMounted(async () => {
  await loadGameDetails()
  loadAchievements()
})

watch(
  () => gameSlug.value,
  async () => {
    await loadGameDetails()
    loadAchievements()
  }
)

function handleAddToLibrary() {
  if (!game.value) return
  game.value.inLibrary = true
  gamesStore.addToLibrary(game.value)
  toast.show(`${game.value.title} ajouté à votre bibliothèque Loom !`, 'success')
}

// Steam status simulation
const isSteamRunning = ref(false)
function toggleSteam() {
  isSteamRunning.value = !isSteamRunning.value
  if (isSteamRunning.value) {
    toast.show('Steam Client démarré silencieusement en arrière-plan', 'success')
  } else {
    toast.show('Steam Client arrêté', 'info')
  }
}

// Play / Stop game state
const isRunning = computed(() => game.value && gamesStore.runningGameId === game.value.id)
const isStarting = computed(() => game.value && gamesStore.launchingGameId === game.value.id)

async function handlePlay() {
  if (!game.value) return
  if (!isRunning.value) {
    if (!game.value.installed) {
      toast.show(`Ouverture de l'assistant d'installation Steam pour ${game.value.title}...`, 'info')
      await gamesStore.installGame(game.value.id)
      return
    }
    await gamesStore.launchGame(game.value.id)
    toast.show(`Transmission de l'ordre de lancement à Steam pour ${game.value.title}...`, 'info')
  } else {
    await gamesStore.stopGame(game.value.id)
    toast.show('Demande d\'arrêt transmise à Steam...', 'info')
  }
}

// News filter tabs: 'all' | 'news' | 'patches'
const currentNewsFilter = ref<'all' | 'news' | 'patches'>('all')

const filteredNews = computed(() => {
  if (!game.value || !game.value.news) return []
  if (currentNewsFilter.value === 'all') return game.value.news
  return game.value.news.filter((n) => n.type === currentNewsFilter.value)
})

// Settings Modal
const settingsModalOpen = ref(false)
const localRunner = ref<RunnerType>('proton-ge')
const localGamemode = ref(true)
const localMangohud = ref(true)
const localEnvVars = ref('VKD3D_CONFIG=dxr MANGOHUD=1')

function openSettingsModal() {
  if (game.value) {
    localRunner.value = game.value.config.runner
    localGamemode.value = game.value.config.gamemode
    localMangohud.value = game.value.config.mangohud
  }
  settingsModalOpen.value = true
}

function saveRunnerSettings() {
  if (game.value) {
    gamesStore.updateGameConfig(game.value.id, {
      runner: localRunner.value,
      gamemode: localGamemode.value,
      mangohud: localMangohud.value
    })
  }
  settingsModalOpen.value = false
  toast.show('Paramètres du runner GE-Proton enregistrés !', 'success')
}

// Minecraft Quick Jump
function openMinecraftStudio() {
  router.push('/minecraft')
}

// Achievements calculation (real data only)
const unlockedCount = computed(() => {
  if (!game.value) return 0
  const realUnlocked = game.value.achievements?.filter((a) => a.unlocked).length || 0
  if (realUnlocked > 0) return realUnlocked
  return game.value.unlockedAchievementsCount || 0
})

const totalAchievements = computed(() => {
  if (!game.value) return 0
  return game.value.totalAchievementsCount || game.value.achievements?.length || 0
})

const achievementPercent = computed(() => {
  if (!totalAchievements.value) return 0
  return Math.round((unlockedCount.value / totalAchievements.value) * 100)
})
</script>

<template>
  <div v-if="game" class="min-h-full pb-16 selection:bg-amber-500/30 selection:text-amber-200">
    <!-- TOP WRAPPER WITH CINEMATIC BACKGROUND -->
    <div class="relative w-full min-h-[560px] lg:min-h-[620px] flex flex-col justify-between overflow-hidden">
      <!-- Background Artwork -->
      <div class="absolute inset-0 z-0 select-none">
        <img
          :src="game.bannerUrl"
          :alt="game.title"
          class="w-full h-full object-cover object-[65%_25%] scale-105 transition-transform duration-1000 ease-out"
        />
        <!-- Dramatic red/gold glow layer -->
        <div class="absolute inset-0 hero-glow mix-blend-screen pointer-events-none" />
        <!-- Vignette and dark fade to zinc-950 -->
        <div class="absolute inset-0 hero-vignette pointer-events-none" />
        <!-- Subtle particle/dust effect hint -->
        <div class="absolute inset-0 bg-[radial-gradient(#ffffff08_1px,transparent_1px)] [background-size:24px_24px] pointer-events-none" />
      </div>

      <!-- 1. STATUS BANNER (Steam status pill) -->
      <div class="relative z-20 px-6 sm:px-10 pt-4 pb-2">
        <div class="inline-flex items-center gap-3 px-4 py-1.5 rounded-full bg-zinc-900/75 border border-zinc-800/80 backdrop-blur-md text-xs text-zinc-300 shadow-lg">
          <span class="flex items-center gap-2">
            <span
              :class="[
                'w-2 h-2 rounded-full transition-colors',
                isSteamRunning ? 'bg-emerald-500 animate-pulse' : 'bg-zinc-500'
              ]"
            />
            <span>
              {{ isSteamRunning ? 'Steam is active and running in background' : 'Steam isn\'t running — Play starts it silently' }}
            </span>
          </span>
          <button
            type="button"
            @click="toggleSteam"
            class="px-2.5 py-0.5 rounded-full bg-white text-zinc-950 font-semibold text-[11px] hover:bg-zinc-200 transition"
          >
            {{ isSteamRunning ? 'Stop Steam' : 'Start Steam' }}
          </button>
        </div>
      </div>

      <!-- 2. HERO CINEMATIC (Game Logo, Categories, Play CTA & Metrics) -->
      <div class="relative z-20 px-6 sm:px-10 lg:px-14 pb-8 pt-6 md:pt-14 max-w-7xl w-full">
        <!-- Hero Title Row with 3:4 Box Cover Art -->
        <div class="flex flex-col sm:flex-row items-start sm:items-end gap-5 mb-5">
          <!-- 3:4 Box Art Cover Poster -->
          <div class="w-24 sm:w-28 md:w-32 aspect-[3/4] rounded-2xl overflow-hidden border-2 border-zinc-700/80 shadow-[0_12px_36px_rgba(0,0,0,0.85)] flex-shrink-0 bg-zinc-950 hidden sm:block">
            <img
              :src="game.coverUrl"
              :alt="game.title"
              class="w-full h-full object-cover"
              loading="lazy"
            />
          </div>

          <!-- Typographic Stylized Logo -->
          <div class="space-y-1">
            <div class="flex items-center gap-2 text-zinc-400 text-xs tracking-widest font-mono uppercase">
              <span>{{ game.developer }}</span>
              <span>•</span>
              <span class="text-amber-400/90 font-medium font-sans">
                {{ game.config.runner === 'proton-ge' ? 'Proton GE-9-20' : game.config.runner }}
              </span>
            </div>

            <div class="relative inline-block">
              <h1 class="font-cinzel text-3xl sm:text-5xl lg:text-6xl font-black tracking-wider text-white drop-shadow-[0_4px_24px_rgba(0,0,0,0.8)]">
                {{ game.title }}
              </h1>
              <div v-if="game.subtitle" class="flex items-center justify-between -mt-1 sm:-mt-2">
                <span class="h-[1px] flex-1 bg-gradient-to-r from-transparent via-zinc-400 to-transparent"></span>
                <span class="font-cinzel text-lg sm:text-2xl font-bold tracking-[0.3em] text-amber-200 px-3 drop-shadow-md">
                  {{ game.subtitle }}
                </span>
                <span class="h-[1px] flex-1 bg-gradient-to-r from-transparent via-zinc-400 to-transparent"></span>
              </div>
            </div>

            <p class="text-xs sm:text-sm text-zinc-400 font-medium pt-1">
              {{ game.genre.join(' • ') }}
            </p>
          </div>
        </div>

        <!-- Play CTA + Runner Settings + Metrics pill bar -->
        <div class="flex flex-wrap items-center gap-3">
          <!-- Fallback : Bouton Acheter le Jeu si hors bibliothèque -->
          <button
            v-if="game.inLibrary === false"
            type="button"
            @click="buyModalOpen = true"
            class="flex items-center gap-2.5 px-7 py-3 rounded-xl font-bold text-sm tracking-wide active:scale-95 transition-all duration-150 btn-gold shadow-lg"
          >
            <i class="ph-bold ph-shopping-bag-open text-lg"></i>
            <span>Acheter le Jeu</span>
          </button>

          <!-- Bouton secondaire : Ajouter à la bibliothèque -->
          <button
            v-if="game.inLibrary === false"
            type="button"
            @click="handleAddToLibrary"
            class="flex items-center gap-2 px-5 py-3 rounded-xl font-semibold text-xs bg-zinc-900/90 hover:bg-zinc-800 border border-zinc-700/80 text-zinc-200 hover:text-amber-400 transition"
          >
            <i class="ph-bold ph-plus-circle text-base text-amber-400"></i>
            <span>Ajouter à la Bibliothèque</span>
          </button>

          <!-- Bouton d'action dynamique : Barre de progression Téléchargement -->
          <div
            v-if="currentDownload"
            class="relative overflow-hidden min-w-[260px] sm:min-w-[320px] px-5 py-2.5 rounded-xl bg-zinc-950/90 border border-amber-500/50 shadow-[0_0_24px_rgba(245,158,11,0.2)] flex flex-col justify-center select-none"
          >
            <!-- Fond animé de progression -->
            <div
              class="absolute inset-y-0 left-0 bg-gradient-to-r from-amber-500/20 via-amber-400/15 to-sky-400/20 transition-all duration-300 pointer-events-none"
              :style="{ width: `${Math.max(2, currentDownload.progressPercent)}%` }"
            />

            <!-- Ligne supérieure : Titre + Pourcentage -->
            <div class="relative z-10 flex items-center justify-between gap-3">
              <div class="flex items-center gap-2">
                <i
                  :class="[
                    'text-base',
                    currentDownload.state === 'staging'
                      ? 'ph ph-arrows-clockwise animate-spin text-sky-400'
                      : 'ph-bold ph-download-simple animate-bounce text-amber-400'
                  ]"
                />
                <span class="text-sm font-bold text-white tracking-wide">
                  {{ currentDownload.state === 'staging' ? 'Finalisation...' : 'Téléchargement' }}
                </span>
              </div>
              <span class="text-sm font-black font-mono text-amber-400">
                {{ currentDownload.progressPercent }}%
              </span>
            </div>

            <!-- Ligne inférieure : Évolution détaillée (octets + vitesse) -->
            <div class="relative z-10 flex items-center justify-between text-[11px] text-zinc-400 font-mono pt-0.5">
              <span>
                {{ formatBytes(currentDownload.bytesDownloaded) }} / {{ formatBytes(currentDownload.bytesTotal) }}
              </span>
              <span v-if="currentDownload.downloadRateMbps > 0" class="text-sky-400 font-semibold flex items-center gap-1">
                <i class="ph-fill ph-lightning text-[10px]"></i>
                {{ formatSpeed(currentDownload.downloadRateMbps) }}
              </span>
            </div>

            <!-- Micro-barre de progression lumineuse en bas du bouton -->
            <div class="absolute bottom-0 left-0 right-0 h-[3px] bg-zinc-800/80 overflow-hidden">
              <div
                class="h-full bg-gradient-to-r from-amber-500 via-amber-400 to-sky-400 transition-all duration-300 rounded-full"
                :style="{ width: `${Math.max(1, currentDownload.progressPercent)}%` }"
              />
            </div>
          </div>

          <!-- Bouton Normal JOUER / STOP si déjà en bibliothèque -->
          <button
            v-else
            type="button"
            @click="handlePlay"
            :class="[
              'flex items-center gap-2.5 px-7 py-3 rounded-xl font-bold text-sm tracking-wide active:scale-95 transition-all duration-150',
              isRunning
                ? 'bg-rose-600 hover:bg-rose-500 text-white shadow-[0_0_24px_rgba(225,29,72,0.35)]'
                : isStarting
                  ? 'bg-amber-500 text-zinc-950 shadow-[0_0_24px_rgba(245,158,11,0.35)]'
                  : game.installed
                    ? 'btn-gold shadow-lg'
                    : 'bg-[#1a9fff] hover:bg-[#158ce0] text-white shadow-[0_0_24px_rgba(26,159,255,0.35)]'
            ]"
          >
            <i v-if="isRunning" class="ph-fill ph-stop text-lg"></i>
            <i v-else-if="isStarting" class="ph ph-spinner animate-spin text-lg"></i>
            <i v-else-if="game.installed" class="ph-fill ph-play text-lg"></i>
            <i v-else class="ph-bold ph-download-simple text-lg"></i>

            <span>
              {{
                isRunning
                  ? 'Arrêter'
                  : isStarting
                    ? 'Démarrage Steam...'
                    : game.installed
                      ? 'Lancer'
                      : 'Installer via Steam'
              }}
            </span>
          </button>

          <!-- Runner / Config Cog Button -->
          <button
            type="button"
            @click="openSettingsModal"
            class="p-3 rounded-xl bg-zinc-900/80 hover:bg-zinc-800 border border-zinc-800/80 text-zinc-300 hover:text-white transition backdrop-blur-md"
            title="Runner & Graphics Settings"
          >
            <i class="ph ph-sliders-horizontal text-lg"></i>
          </button>

          <!-- Minecraft Studio Quick Jump (if Minecraft instance) -->
          <button
            v-if="game.id === 'minecraft' || game.platform === 'minecraft'"
            type="button"
            @click="openMinecraftStudio"
            class="p-3 rounded-xl bg-zinc-900/80 hover:bg-amber-500/10 hover:border-amber-500/40 border border-zinc-800/80 text-zinc-300 hover:text-amber-300 transition backdrop-blur-md flex items-center gap-2 text-xs font-medium"
            title="Ouvrir les profils Minecraft Studio"
          >
            <i class="ph-fill ph-cube text-base text-amber-400"></i>
            <span class="hidden sm:inline">Mods & Instances</span>
          </button>

          <!-- IGDB Sync Button -->
          <button
            type="button"
            @click="igdbModalOpen = true"
            class="p-3 rounded-xl bg-zinc-900/80 hover:bg-amber-500/10 hover:border-amber-500/40 border border-zinc-800/80 text-zinc-300 hover:text-amber-300 transition backdrop-blur-md flex items-center gap-2 text-xs font-medium"
            title="Synchroniser les métadonnées officielles via l'API IGDB"
          >
            <i class="ph-fill ph-database text-base text-amber-400"></i>
            <span class="hidden sm:inline">Sync IGDB</span>
          </button>

          <!-- Horizontal Metrics Bar -->
          <div class="flex flex-wrap items-center gap-6 sm:gap-8 px-5 py-2.5 rounded-xl bg-zinc-900/80 border border-zinc-800/80 backdrop-blur-md text-xs">
            <!-- IGDB Rating (if available) -->
            <div v-if="game.rating" class="space-y-0.5">
              <span class="text-[10px] tracking-wider uppercase font-medium text-zinc-400">Note IGDB</span>
              <p class="font-bold text-amber-400 flex items-center gap-1">
                <i class="ph-fill ph-star text-xs"></i> {{ game.rating }}/100
              </p>
            </div>
            <!-- Last Played -->
            <div class="space-y-0.5">
              <span class="text-[10px] tracking-wider uppercase font-medium text-zinc-400">Last Played</span>
              <p class="font-semibold text-zinc-200">{{ game.lastPlayed || 'Never' }}</p>
            </div>

            <!-- Play Time -->
            <div class="space-y-0.5">
              <span class="text-[10px] tracking-wider uppercase font-medium text-zinc-400">Play Time</span>
              <p class="font-semibold text-zinc-200">{{ (game.playtimeMinutes / 60).toFixed(1) }} h</p>
            </div>

            <!-- Achievements with micro progress bar -->
            <div v-if="totalAchievements > 0" class="space-y-1">
              <div class="flex items-center justify-between gap-3">
                <span class="text-[10px] tracking-wider uppercase font-medium text-zinc-400">Achievements</span>
                <span class="font-semibold text-zinc-200 text-[11px]">{{ unlockedCount }}/{{ totalAchievements }}</span>
              </div>
              <div class="w-24 h-1.5 rounded-full bg-zinc-800 overflow-hidden">
                <div
                  class="h-full bg-brand-gold rounded-full shadow-[0_0_8px_rgba(245,158,11,0.5)] transition-all duration-500"
                  :style="{ width: `${achievementPercent}%` }"
                />
              </div>
            </div>
            <div v-else class="space-y-0.5">
              <span class="text-[10px] tracking-wider uppercase font-medium text-zinc-400">Succès</span>
              <p class="font-semibold text-zinc-500">Non répertoriés</p>
            </div>

            <!-- Cloud Saves -->
            <div class="space-y-0.5">
              <span class="text-[10px] tracking-wider uppercase font-medium text-zinc-400">Cloud Saves</span>
              <p class="font-semibold text-emerald-400 flex items-center gap-1">
                <i class="ph-bold ph-check text-xs"></i> Up to date
              </p>
            </div>
          </div>
          </div>
      </div>
    </div>

    <!-- 3. SUB-NAVIGATION CONTEXTUAL LINKS -->
    <section class="border-b border-zinc-800/60 bg-zinc-950 px-6 sm:px-10 lg:px-14 py-3">
      <div class="flex items-center gap-6 text-xs text-zinc-400 overflow-x-auto whitespace-nowrap">
        <a href="#" @click.prevent="toast.show('Ouverture de la page du magasin (Steam)', 'info')" class="hover:text-zinc-200 transition flex items-center gap-1 group">
          Store page <i class="ph ph-arrow-up-right text-[10px] group-hover:translate-x-0.5 group-hover:-translate-y-0.5 transition-transform"></i>
        </a>
        <a href="#" @click.prevent="toast.show('Aucun DLC disponible', 'info')" class="hover:text-zinc-200 transition flex items-center gap-1 group">
          DLC <i class="ph ph-arrow-up-right text-[10px] group-hover:translate-x-0.5 group-hover:-translate-y-0.5 transition-transform"></i>
        </a>
        <a href="#" @click.prevent="toast.show('Ouverture du hub communautaire', 'info')" class="hover:text-zinc-200 transition flex items-center gap-1 group">
          Community hub <i class="ph ph-arrow-up-right text-[10px] group-hover:translate-x-0.5 group-hover:-translate-y-0.5 transition-transform"></i>
        </a>
        <a href="#" @click.prevent="toast.show('Ouverture des discussions Steam', 'info')" class="hover:text-zinc-200 transition flex items-center gap-1 group">
          Discussions <i class="ph ph-arrow-up-right text-[10px] group-hover:translate-x-0.5 group-hover:-translate-y-0.5 transition-transform"></i>
        </a>
        <a href="#" @click.prevent="toast.show('Guides de la communauté', 'info')" class="hover:text-zinc-200 transition flex items-center gap-1 group">
          Guides <i class="ph ph-arrow-up-right text-[10px] group-hover:translate-x-0.5 group-hover:-translate-y-0.5 transition-transform"></i>
        </a>
        <a href="#" @click.prevent="toast.show('Support technique Loom', 'info')" class="hover:text-zinc-200 transition flex items-center gap-1 group">
          Support <i class="ph ph-arrow-up-right text-[10px] group-hover:translate-x-0.5 group-hover:-translate-y-0.5 transition-transform"></i>
        </a>
      </div>
    </section>

    <!-- 4. MAIN CONTENT FEED & WIDGETS SECTION (12 COLS GRID) -->
    <main class="max-w-7xl mx-auto px-6 sm:px-10 lg:px-14 py-8">
      <div class="grid grid-cols-1 lg:grid-cols-12 gap-8">
        <!-- Left Column: Activity & News timeline (8 cols) -->
        <div class="lg:col-span-8 space-y-6">
          <!-- Description & Synopsis -->
          <section v-if="game.description" class="p-6 rounded-2xl bg-zinc-900/50 border border-zinc-800/60 space-y-3">
            <h3 class="text-sm font-bold text-white flex items-center gap-2">
              <i class="ph-bold ph-book-open text-amber-400"></i>
              À propos de {{ game.title }}
            </h3>
            <p class="text-sm text-zinc-300 leading-relaxed whitespace-pre-line">
              {{ game.description }}
            </p>
          </section>

          <!-- Galerie de Captures d'Écran Officielles (1080p) -->
          <section v-if="game.screenshots && game.screenshots.length > 0" class="p-6 rounded-2xl bg-zinc-900/50 border border-zinc-800/60 space-y-4">
            <div class="flex items-center justify-between">
              <h3 class="text-sm font-bold text-white flex items-center gap-2">
                <i class="ph-bold ph-image text-amber-400"></i>
                Captures d'écran officielles
              </h3>
              <span class="text-xs text-zinc-500 font-mono">{{ game.screenshots.length }} images HD</span>
            </div>
            <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
              <div
                v-for="(shot, idx) in game.screenshots"
                :key="idx"
                class="group/shot relative aspect-video rounded-xl overflow-hidden border border-zinc-800 hover:border-amber-500/50 transition-all cursor-pointer bg-zinc-950"
                @click="selectedScreenshot = shot"
              >
                <img
                  :src="shot"
                  :alt="`${game.title} capture ${idx + 1}`"
                  class="w-full h-full object-cover group-hover/shot:scale-105 transition-transform duration-500"
                  loading="lazy"
                />
                <div class="absolute inset-0 bg-black/40 opacity-0 group-hover/shot:opacity-100 transition-opacity flex items-center justify-center">
                  <span class="p-2.5 rounded-full bg-zinc-900/90 text-amber-400 border border-amber-500/40 shadow-xl">
                    <i class="ph-bold ph-magnifying-glass-plus text-base"></i>
                  </span>
                </div>
              </div>
            </div>
          </section>

          <!-- Filter Tabs (All / News / Updates & patches) -->
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-2 bg-zinc-900/60 p-1 rounded-full border border-zinc-800/80">
              <button
                type="button"
                @click="currentNewsFilter = 'all'"
                :class="[
                  'px-4 py-1 rounded-full text-xs font-semibold transition',
                  currentNewsFilter === 'all'
                    ? 'bg-white text-zinc-950'
                    : 'text-zinc-400 hover:text-white'
                ]"
              >
                All
              </button>
              <button
                type="button"
                @click="currentNewsFilter = 'news'"
                :class="[
                  'px-3.5 py-1 rounded-full text-xs font-medium transition',
                  currentNewsFilter === 'news'
                    ? 'bg-white text-zinc-950 font-semibold'
                    : 'text-zinc-400 hover:text-white'
                ]"
              >
                News
              </button>
              <button
                type="button"
                @click="currentNewsFilter = 'patches'"
                :class="[
                  'px-3.5 py-1 rounded-full text-xs font-medium transition',
                  currentNewsFilter === 'patches'
                    ? 'bg-white text-zinc-950 font-semibold'
                    : 'text-zinc-400 hover:text-white'
                ]"
              >
                Updates & patches
              </button>
            </div>

            <span class="text-xs text-zinc-400 font-medium">Newest first</span>
          </div>

          <!-- News / Patch Cards -->
          <div v-if="filteredNews.length > 0" class="space-y-4">
            <article
              v-for="item in filteredNews"
              :key="item.id"
              class="p-6 rounded-2xl bg-zinc-900/50 border border-zinc-800/60 hover:border-zinc-700/80 transition group space-y-4"
            >
              <div class="flex items-center justify-between text-xs">
                <div class="flex items-center gap-2">
                  <span
                    v-if="item.tag"
                    :class="[
                      'px-2.5 py-0.5 rounded-full text-[11px] font-medium border',
                      item.type === 'patches'
                        ? 'bg-amber-500/10 border-amber-500/30 text-amber-300'
                        : 'bg-zinc-800 border-zinc-700 text-zinc-300'
                    ]"
                  >
                    {{ item.tag }}
                  </span>
                  <span class="text-zinc-400">{{ item.author }}</span>
                </div>
                <time class="text-zinc-400">{{ item.date }}</time>
              </div>

              <div class="space-y-2">
                <h3 class="text-lg font-bold text-white group-hover:text-amber-300 transition">
                  {{ item.title }}
                </h3>
                <p class="text-sm text-zinc-300 line-clamp-3 leading-relaxed">
                  {{ item.summary }}
                </p>
              </div>

              <div class="flex items-center justify-between pt-2 border-t border-zinc-800/40 text-xs">
                <div class="flex items-center gap-4 text-zinc-400">
                  <span v-if="item.hearts" class="flex items-center gap-1">
                    <i class="ph ph-heart text-rose-400"></i>
                    {{ item.hearts }}
                  </span>
                  <span v-if="item.comments" class="flex items-center gap-1">
                    <i class="ph ph-chat-circle"></i>
                    {{ item.comments }} commentaires
                  </span>
                </div>
                <button
                  type="button"
                  @click="toast.show(`Ouverture de : ${item.title}`, 'info')"
                  class="text-amber-400 hover:text-amber-300 font-medium flex items-center gap-1 ml-auto"
                >
                  Lire l'annonce <i class="ph ph-arrow-right"></i>
                </button>
              </div>
            </article>
          </div>
          <div v-else class="p-8 rounded-2xl bg-zinc-900/30 border border-zinc-800/60 text-center space-y-2">
            <i class="ph ph-newspaper text-zinc-500 text-2xl"></i>
            <p class="text-xs text-zinc-400 font-medium">Aucune actualité officielle répertoriée</p>
            <p class="text-[11px] text-zinc-500">Les annonces et patch notes synchronisés apparaîtront ici.</p>
          </div>
        </div>

        <!-- Right Column: Achievements & Runner Info (4 cols) -->
        <aside class="lg:col-span-4 space-y-6">
          <!-- Achievements Card -->
          <div class="p-6 rounded-2xl bg-zinc-900/50 border border-zinc-800/60 space-y-5">
            <div class="flex items-center justify-between">
              <h3 class="text-base font-bold text-white flex items-center gap-2">
                <i class="ph ph-trophy text-amber-400"></i>
                Succès
              </h3>
              <div class="flex items-center gap-2">
                <span v-if="isLoadingAchievements" class="w-3.5 h-3.5 rounded-full border-2 border-amber-500 border-t-transparent animate-spin" />
                <span v-if="totalAchievements > 0" class="text-xs text-zinc-400 font-mono">
                  {{ unlockedCount }}/{{ totalAchievements }}
                </span>
              </div>
            </div>

            <!-- Progress display if achievements exist -->
            <div v-if="totalAchievements > 0" class="space-y-4">
              <div class="flex items-center gap-4 p-3 rounded-xl bg-zinc-950/60 border border-zinc-800/60">
                <!-- Progress Circle SVG -->
                <div class="relative w-16 h-16 flex items-center justify-center flex-shrink-0">
                  <svg class="w-full h-full -rotate-90" viewBox="0 0 36 36">
                    <path
                      class="text-zinc-800"
                      stroke-width="3.5"
                      stroke="currentColor"
                      fill="none"
                      d="M18 2.0845 a 15.9155 15.9155 0 0 1 0 31.831 a 15.9155 15.9155 0 0 1 0 -31.831"
                    />
                    <path
                      class="text-amber-400"
                      stroke-width="3.5"
                      :stroke-dasharray="`${achievementPercent}, 100`"
                      stroke-linecap="round"
                      stroke="currentColor"
                      fill="none"
                      d="M18 2.0845 a 15.9155 15.9155 0 0 1 0 31.831 a 15.9155 15.9155 0 0 1 0 -31.831"
                    />
                  </svg>
                  <span class="absolute font-bold text-xs text-white">{{ achievementPercent }}%</span>
                </div>

                <div>
                  <p class="text-sm font-semibold text-zinc-200">
                    {{ unlockedCount }} sur {{ totalAchievements }} débloqués
                  </p>
                  <p class="text-xs text-zinc-400">
                    {{ totalAchievements - unlockedCount }} succès restants
                  </p>
                </div>
              </div>

              <!-- Recent achievements badge list preview -->
              <div v-if="game.achievements && game.achievements.length > 0" class="space-y-2">
                <span class="text-[11px] uppercase tracking-wider text-zinc-400 font-semibold block">
                  Aperçu des succès
                </span>

                <div
                  v-for="ach in game.achievements.filter(a => a.unlocked).slice(0, 3)"
                  :key="ach.id"
                  class="flex items-center gap-3 p-2.5 rounded-lg bg-zinc-800/30 hover:bg-zinc-800/50 transition border border-zinc-800/40"
                >
                  <div class="w-9 h-9 rounded overflow-hidden bg-zinc-950 border border-zinc-800 flex-shrink-0 flex items-center justify-center">
                    <img
                      v-if="ach.icon && (ach.icon.startsWith('http') || ach.icon.startsWith('/'))"
                      :src="ach.icon"
                      :alt="ach.name"
                      class="w-full h-full object-cover"
                    />
                    <i v-else class="ph ph-trophy text-amber-300"></i>
                  </div>
                  <div class="flex-1 min-w-0">
                    <div class="flex items-center justify-between gap-1">
                      <p class="text-xs font-semibold text-zinc-200 truncate">{{ ach.name }}</p>
                      <span v-if="ach.rarityPercentage" class="text-[10px] text-amber-400/90 font-mono">{{ ach.rarityPercentage.toFixed(1) }}%</span>
                    </div>
                    <p class="text-[10px] text-zinc-400 truncate">{{ ach.description }}</p>
                  </div>
                </div>
              </div>

              <!-- Button to Open Full Modal -->
              <UiButton
                variant="secondary"
                size="sm"
                class="w-full mt-2"
                @click="achievementsModalOpen = true"
              >
                <template #prefix>
                  <i class="ph ph-list-dashes text-sm text-amber-400"></i>
                </template>
                Voir tous les succès ({{ totalAchievements }})
              </UiButton>
            </div>

            <!-- Honest Empty State for achievements -->
            <div v-else class="py-4 text-center space-y-2">
              <div class="w-10 h-10 rounded-full bg-zinc-800/80 mx-auto flex items-center justify-center text-zinc-500">
                <i class="ph ph-trophy text-lg"></i>
              </div>
              <p class="text-xs text-zinc-400 font-medium">Aucun succès répertorié</p>
              <p class="text-[11px] text-zinc-500">Ce titre ne comporte aucun succès officiel sur cette plateforme.</p>
            </div>
          </div>

          <!-- System & Runner Environment Card -->
          <div class="p-6 rounded-2xl bg-zinc-900/50 border border-zinc-800/60 space-y-4">
            <h3 class="text-sm font-bold text-white flex items-center justify-between">
              <span>Runner Configuration</span>
              <span class="text-[10px] font-mono px-2 py-0.5 rounded bg-zinc-800 text-zinc-300">Loom Core</span>
            </h3>

            <div class="space-y-2 text-xs">
              <div class="flex justify-between py-1.5 border-b border-zinc-800/40 text-zinc-300">
                <span class="text-zinc-400">Compatibility tool</span>
                <span class="font-medium">
                  {{ game.config.runner === 'proton-ge' ? 'GE-Proton 9-20' : game.config.runner }}
                </span>
              </div>
              <div class="flex justify-between py-1.5 border-b border-zinc-800/40 text-zinc-300">
                <span class="text-zinc-400">GameMode / MangoHUD</span>
                <span class="font-medium text-emerald-400">
                  {{ game.config.gamemode ? 'Enabled' : 'Disabled' }}
                </span>
              </div>
              <div class="flex justify-between py-1.5 border-b border-zinc-800/40 text-zinc-300">
                <span class="text-zinc-400">WINEPREFIX location</span>
                <span class="font-mono text-[10px] text-zinc-400 truncate max-w-[140px]">
                  {{ game.config.winePrefix || '~/.local/share/wine/default' }}
                </span>
              </div>
              <div class="flex justify-between py-1.5 text-zinc-300">
                <span class="text-zinc-400">Storage Footprint</span>
                <span class="font-medium">{{ game.sizeGb }} GB on /nvme0n1</span>
              </div>
            </div>
          </div>

          <!-- Official Store Links -->
          <div v-if="game.storeLinks && game.storeLinks.length > 0" class="p-6 rounded-2xl bg-zinc-900/50 border border-zinc-800/60 space-y-4">
            <h3 class="text-sm font-bold text-white flex items-center gap-2">
              <i class="ph-bold ph-shopping-bag text-amber-400"></i>
              Boutiques Officielles
            </h3>
            <div class="space-y-2">
              <a
                v-for="store in game.storeLinks"
                :key="store.store"
                :href="store.url"
                target="_blank"
                rel="noopener noreferrer"
                class="flex items-center justify-between p-3 rounded-xl bg-zinc-950/60 hover:bg-zinc-800/60 border border-zinc-800 hover:border-amber-500/40 transition group text-xs"
              >
                <div class="flex items-center gap-2.5">
                  <i :class="[store.icon || 'ph-fill ph-storefront', 'text-base text-amber-400']"></i>
                  <span class="font-semibold text-zinc-200 group-hover:text-amber-300 transition-colors">{{ store.store }}</span>
                </div>
                <i class="ph ph-arrow-up-right text-zinc-500 group-hover:text-amber-400 group-hover:translate-x-0.5 group-hover:-translate-y-0.5 transition-transform"></i>
              </a>
            </div>
          </div>
        </aside>
      </div>
    </main>

    <!-- MODAL: LIGHTBOX CAPTURE D'ÉCRAN -->
    <UiModal
      :open="Boolean(selectedScreenshot)"
      title="Aperçu HD"
      max-width="3xl"
      @close="selectedScreenshot = null"
    >
      <div v-if="selectedScreenshot" class="aspect-video w-full rounded-xl overflow-hidden bg-zinc-950 border border-zinc-800">
        <img :src="selectedScreenshot" :alt="game.title" class="w-full h-full object-contain" />
      </div>
    </UiModal>

    <!-- MODAL: RUNNER SETTINGS (MATCHING REFERENCE DESIGN) -->
    <UiModal
      :open="settingsModalOpen"
      title="Configuration du Runner"
      max-width="lg"
      @close="settingsModalOpen = false"
    >
      <template #title>
        <span class="flex items-center gap-2">
          <i class="ph ph-sliders-horizontal text-amber-400"></i> Configuration du Runner
        </span>
      </template>

      <div class="space-y-4 text-xs">
        <div>
          <label class="block text-zinc-400 mb-1.5 font-medium">Version Proton / Wine</label>
          <select
            v-model="localRunner"
            class="w-full bg-zinc-950 border border-zinc-700/80 rounded-lg p-2.5 text-zinc-200 focus:outline-none focus:border-amber-500"
          >
            <option value="proton-ge">GE-Proton 9-20 (Recommandé)</option>
            <option value="proton-experimental">Proton Experimental</option>
            <option value="wine-staging">Wine Staging (Système WINE 9.0)</option>
            <option value="native">Linux Natif</option>
          </select>
        </div>

        <div class="flex items-center justify-between p-3 rounded-xl bg-zinc-950/60 border border-zinc-800">
          <div>
            <p class="font-semibold text-zinc-200">Activer Feral GameMode</p>
            <p class="text-[11px] text-zinc-400">Optimise les fréquences CPU & I/O scheduler</p>
          </div>
          <input
            v-model="localGamemode"
            type="checkbox"
            class="w-4 h-4 accent-amber-500 rounded cursor-pointer"
          />
        </div>

        <div class="flex items-center justify-between p-3 rounded-xl bg-zinc-950/60 border border-zinc-800">
          <div>
            <p class="font-semibold text-zinc-200">Affichage MangoHud (FPS / Temp)</p>
            <p class="text-[11px] text-zinc-400">Overlay Vulkan en jeu</p>
          </div>
          <input
            v-model="localMangohud"
            type="checkbox"
            class="w-4 h-4 accent-amber-500 rounded cursor-pointer"
          />
        </div>

        <div>
          <label class="block text-zinc-400 mb-1.5 font-medium">Variables d'environnement personnalisées</label>
          <input
            v-model="localEnvVars"
            type="text"
            class="w-full bg-zinc-950 border border-zinc-700/80 rounded-lg p-2.5 text-zinc-200 font-mono text-[11px] focus:outline-none focus:border-amber-500"
          />
        </div>
      </div>

      <template #footer>
        <UiButton variant="ghost" size="sm" @click="settingsModalOpen = false">
          Annuler
        </UiButton>
        <UiButton variant="primary" size="sm" @click="saveRunnerSettings">
          Enregistrer
        </UiButton>
      </template>
    </UiModal>

    <!-- IGDB Sync Modal -->
    <IgdbSearchModal
      :open="igdbModalOpen"
      :target-sync-game-id="game?.id"
      @close="igdbModalOpen = false"
    />

    <!-- MODALE : OÙ ACHETER CE JEU -->
    <UiModal
      :open="buyModalOpen"
      title="Acheter ou Obtenir le Jeu"
      :subtitle="`Choisissez votre boutique officielle pour acquérir ${game.title}`"
      max-width="md"
      @close="buyModalOpen = false"
    >
      <div class="space-y-4 py-2">
        <p class="text-xs text-zinc-400 leading-relaxed">
          Ce jeu ne fait pas encore partie de votre bibliothèque locale. Procurez-vous la licence officielle sur l'une des plateformes ci-dessous :
        </p>

        <!-- Store Links List -->
        <div class="space-y-2.5">
          <a
            v-for="(store, idx) in (game.storeLinks || [])"
            :key="idx"
            :href="store.url"
            target="_blank"
            rel="noopener noreferrer"
            class="flex items-center justify-between p-3.5 rounded-xl bg-zinc-950/80 border border-zinc-800 hover:border-amber-500/50 hover:bg-zinc-900 transition-all group"
          >
            <div class="flex items-center gap-3">
              <div class="w-10 h-10 rounded-lg bg-zinc-900 border border-zinc-800 flex items-center justify-center text-amber-400 group-hover:scale-110 transition-transform">
                <i :class="store.icon || 'ph-fill ph-shopping-bag'" class="text-xl"></i>
              </div>
              <div>
                <h4 class="font-bold text-xs text-zinc-100 group-hover:text-amber-400 transition-colors">
                  {{ store.store }}
                </h4>
                <p class="text-[11px] text-zinc-500">
                  Version officielle compatible PC & Linux
                </p>
              </div>
            </div>

            <div class="flex items-center gap-1.5 text-xs text-amber-400 font-semibold group-hover:translate-x-1 transition-transform">
              <span>Visiter</span>
              <i class="ph ph-arrow-square-out text-sm"></i>
            </div>
          </a>
        </div>

        <!-- Déjà possédé ? -->
        <div class="pt-3 border-t border-zinc-800/80 flex items-center justify-between">
          <span class="text-xs text-zinc-400">Vous possédez déjà ce jeu ?</span>
          <UiButton
            variant="secondary"
            size="sm"
            @click="handleAddToLibrary(); buyModalOpen = false"
          >
            <template #prefix>
              <i class="ph-bold ph-plus text-xs"></i>
            </template>
            L'ajouter à Loom
          </UiButton>
        </div>
      </div>
    </UiModal>

    <!-- Modal complète des Succès -->
    <AchievementsModal
      :open="achievementsModalOpen"
      :game="game"
      :achievements="game.achievements || []"
      :loading="isLoadingAchievements"
      @close="achievementsModalOpen = false"
    />
  </div>

  <!-- Loading or Not Found State -->
  <div v-else class="min-h-[75vh] flex flex-col items-center justify-center px-6 text-center">
    <div v-if="isLoadingGame" class="space-y-4">
      <div class="w-12 h-12 rounded-full border-2 border-amber-500/20 border-t-amber-500 animate-spin mx-auto" />
      <div class="space-y-1">
        <h2 class="text-lg font-bold text-white">Recherche sur IGDB...</h2>
        <p class="text-xs text-zinc-400 font-mono">Chargement des métadonnées officielles pour "{{ gameSlug }}"</p>
      </div>
    </div>
    <div v-else class="space-y-5 max-w-md">
      <div class="w-16 h-16 rounded-2xl bg-zinc-900 border border-zinc-800 mx-auto flex items-center justify-center text-zinc-500 text-3xl">
        <i class="ph ph-game-controller"></i>
      </div>
      <div class="space-y-1.5">
        <h2 class="text-xl font-bold text-white">Jeu introuvable</h2>
        <p class="text-xs text-zinc-400 leading-relaxed">
          Le jeu <span class="text-amber-300 font-mono">"{{ gameSlug }}"</span> n'a pas été trouvé dans votre bibliothèque locale ni dans le catalogue IGDB.
        </p>
      </div>
      <div class="flex items-center justify-center gap-3 pt-2">
        <UiButton variant="secondary" size="sm" @click="router.push('/library')">
          <template #prefix>
            <i class="ph ph-arrow-left text-xs"></i>
          </template>
          Bibliothèque
        </UiButton>
        <UiButton variant="primary" size="sm" @click="igdbModalOpen = true">
          <template #prefix>
            <i class="ph-bold ph-magnifying-glass text-xs"></i>
          </template>
          Rechercher sur IGDB
        </UiButton>
      </div>
    </div>
  </div>
</template>
