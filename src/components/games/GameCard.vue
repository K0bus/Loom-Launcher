<script setup lang="ts">
import { computed } from 'vue'
import { Play, Square, Download, Star, Settings2, HardDrive } from 'lucide-vue-next'
import type { Game } from '@/models/game'
import { useGamesStore } from '@/stores/gamesStore'
import { PlatformIcon } from '@/components/icons/platforms'
import UiBadge from '@/components/ui/UiBadge.vue'

interface Props {
  game: Game
  isRunning?: boolean
  isStarting?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  isRunning: undefined,
  isStarting: undefined
})

const emit = defineEmits<{
  (e: 'select', game: Game): void
  (e: 'configure', game: Game): void
}>()

const gamesStore = useGamesStore()

// Découplage de la réactivité : utilise les props fournies par le parent (LibraryView)
// pour éviter que chaque carte n'écoute individuellement le store lors des polls réguliers
const isRunning = computed(() => {
  if (props.isRunning !== undefined) return props.isRunning
  return props.game.id === gamesStore.runningGameId
})

const isStarting = computed(() => {
  if (props.isStarting !== undefined) return props.isStarting
  return props.game.id === gamesStore.launchingGameId
})

const platformBadgeColor = {
  steam: 'sky',
  epic: 'purple',
  gog: 'rose',
  minecraft: 'emerald',
  battlenet: 'amber',
  manual: 'zinc'
} as const
</script>

<template>
  <div
    class="game-card-contain group relative rounded-2xl overflow-hidden bg-zinc-900/80 border border-zinc-800/80 hover:border-amber-500/50 transition-colors duration-150 flex flex-col cursor-pointer shadow-md hover:-translate-y-0.5 select-none"
    @click="emit('select', game)"
  >
    <!-- 3:4 Poster Image Container -->
    <div class="relative w-full aspect-[3/4] overflow-hidden bg-zinc-950">
      <img
        :src="game.coverUrl"
        :alt="game.title"
        :class="[
          'w-full h-full object-cover transition-transform duration-200 group-hover:scale-105',
          !game.installed ? 'opacity-70 group-hover:opacity-100' : 'opacity-100'
        ]"
        loading="lazy"
        decoding="async"
      />

      <!-- Dark Gradient Overlay -->
      <div class="absolute inset-0 bg-gradient-to-t from-zinc-950 via-zinc-950/20 to-transparent opacity-80 group-hover:opacity-90 transition-opacity" />

      <!-- Top Badges -->
      <div class="absolute top-3 left-3 right-3 flex items-center justify-between pointer-events-none">
        <div class="flex items-center gap-1.5 pointer-events-auto">
          <UiBadge
            :variant="platformBadgeColor[game.platform]"
            size="xs"
            class="flex items-center gap-1.5"
          >
            <PlatformIcon :platform="game.platform" class-name="w-3 h-3" />
            <span class="capitalize">{{ game.platform }}</span>
          </UiBadge>

          <!-- Installed status pill -->
          <span
            v-if="game.installed"
            class="px-1.5 py-0.5 rounded-md text-[9px] font-bold font-mono uppercase bg-emerald-950/80 text-emerald-300 border border-emerald-500/40 flex items-center gap-1 shadow-sm"
            title="Installé sur votre machine"
          >
            <HardDrive class="w-2.5 h-2.5" />
            Installé
          </span>
          <span
            v-else
            class="px-1.5 py-0.5 rounded-md text-[9px] font-medium font-mono uppercase bg-zinc-950/90 text-zinc-400 border border-zinc-700/60"
            :title="game.platform === 'battlenet' ? 'Dans votre compte Blizzard (non installé)' : 'Dans votre bibliothèque Steam (non installé)'"
          >
            Bibliothèque
          </span>
        </div>

        <button
          type="button"
          @click.stop="gamesStore.toggleFavorite(game.id)"
          class="pointer-events-auto p-1.5 rounded-full bg-zinc-950/90 hover:bg-zinc-800 text-zinc-400 hover:text-amber-400 transition-colors"
          title="Favori"
        >
          <Star
            class="w-3.5 h-3.5"
            :class="{ 'fill-amber-400 text-amber-400': game.isFavorite }"
          />
        </button>
      </div>

      <!-- Center Hover Quick Action Button (Play if installed, Install if not, Stop if running) -->
      <div class="absolute inset-0 flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity duration-200">
        <!-- Stop Button if Running -->
        <button
          v-if="isRunning"
          type="button"
          @click.stop="gamesStore.stopGame(game.id)"
          class="p-4 rounded-full bg-rose-600 text-white shadow-xl shadow-rose-600/40 hover:bg-rose-500 hover:scale-110 active:scale-95 transition-all"
          title="Arrêter le jeu"
        >
          <Square class="w-6 h-6 fill-current" />
        </button>

        <!-- Starting indicator if Steam or Battle.net is launching -->
        <div
          v-else-if="isStarting"
          class="flex items-center gap-2 px-4 py-2.5 rounded-full bg-amber-500 text-zinc-950 font-bold text-xs shadow-xl shadow-amber-500/40"
        >
          <span class="w-3.5 h-3.5 rounded-full border-2 border-zinc-950 border-t-transparent animate-spin" />
          <span>{{ game.platform === 'battlenet' ? 'Lancement Battle.net...' : 'Lancement Steam...' }}</span>
        </div>

        <!-- Play Button for Installed -->
        <button
          v-else-if="game.installed"
          type="button"
          @click.stop="gamesStore.launchGame(game.id)"
          class="p-4 rounded-full bg-amber-500 text-zinc-950 shadow-xl shadow-amber-500/40 hover:bg-amber-400 hover:scale-110 active:scale-95 transition-all"
          title="Lancer immédiatement"
        >
          <Play class="w-6 h-6 fill-current translate-x-0.5" />
        </button>

        <!-- Install Button for Uninstalled -->
        <button
          v-else
          type="button"
          @click.stop="gamesStore.installGame(game.id)"
          :class="[
            'flex items-center gap-2 px-4 py-2.5 rounded-full font-bold text-xs shadow-xl hover:scale-105 active:scale-95 transition-all text-white',
            game.platform === 'battlenet'
              ? 'bg-amber-600 hover:bg-amber-500 shadow-amber-600/40'
              : 'bg-[#1a9fff] hover:bg-[#158ce0] shadow-[#1a9fff]/40'
          ]"
          :title="game.platform === 'battlenet' ? 'Installer via Battle.net' : 'Installer via Steam'"
        >
          <Download class="w-4 h-4" />
          <span>{{ game.platform === 'battlenet' ? 'Installer (Battle.net)' : 'Installer' }}</span>
        </button>
      </div>

      <!-- Real Status Indicators in Corner -->
      <div
        v-if="isRunning"
        class="absolute bottom-3 left-3 px-2 py-1 rounded-md bg-emerald-500 text-zinc-950 text-[10px] font-bold font-mono uppercase tracking-wider flex items-center gap-1.5 shadow-md"
      >
        <span class="w-1.5 h-1.5 rounded-full bg-zinc-950" />
        {{ game.platform === 'battlenet' ? 'En jeu (Battle.net)' : 'En jeu (Steam)' }}
      </div>

      <div
        v-else-if="isStarting"
        class="absolute bottom-3 left-3 px-2 py-1 rounded-md bg-amber-500 text-zinc-950 text-[10px] font-bold font-mono uppercase tracking-wider flex items-center gap-1.5 shadow-md"
      >
        <span class="w-2 h-2 rounded-full border-2 border-zinc-950 border-t-transparent animate-spin" />
        Lancement...
      </div>
    </div>

    <!-- Info Bottom Footer -->
    <div class="p-3.5 flex flex-col justify-between flex-1 gap-2">
      <div>
        <h4 class="font-bold text-sm text-zinc-100 group-hover:text-amber-400 transition-colors line-clamp-1">
          {{ game.title }}
        </h4>
        <div class="flex items-center gap-1.5 mt-1 text-[11px] text-zinc-400">
          <span v-if="game.playtimeMinutes > 0">{{ (game.playtimeMinutes / 60).toFixed(1) }} h</span>
          <span v-else class="text-zinc-500">Non joué</span>
          <span>•</span>
          <span v-if="game.installed && game.sizeGb > 0" class="text-zinc-400 font-mono">{{ game.sizeGb }} Go</span>
          <span v-else-if="!game.installed" class="text-amber-400/80 font-mono">À télécharger</span>
          <span v-else class="text-zinc-500 font-mono">{{ game.config.runner }}</span>
        </div>
      </div>

      <div class="pt-2 border-t border-zinc-800/60 flex items-center justify-between text-[11px] text-zinc-500">
        <span class="truncate max-w-[120px]">{{ game.genre[0] }}</span>
        <button
          type="button"
          @click.stop="emit('configure', game)"
          class="p-1 rounded hover:bg-zinc-800 text-zinc-400 hover:text-zinc-200 transition-colors"
          title="Configurer le runner"
        >
          <Settings2 class="w-3.5 h-3.5" />
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.game-card-contain {
  /*
   * Confinement CSS natif : zappe le calcul de mise en page, de style et de peinture
   * pour toutes les cartes de jeux situées hors du viewport actif.
   * contain-intrinsic-size maintient la hauteur approximative de la carte dans le layout
   * afin d'éviter tout saut ou artefact sur la barre de défilement.
   */
  content-visibility: auto;
  contain-intrinsic-size: auto 380px;
}
</style>
