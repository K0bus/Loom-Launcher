<script setup lang="ts">
import { Play, Star, Settings2 } from 'lucide-vue-next'
import type { Game } from '@/models/game'
import { useGamesStore } from '@/stores/gamesStore'
import { PlatformIcon } from '@/components/icons/platforms'
import UiBadge from '@/components/ui/UiBadge.vue'

interface Props {
  game: Game
}

const props = defineProps<Props>()

const emit = defineEmits<{
  (e: 'select', game: Game): void
  (e: 'configure', game: Game): void
}>()

const gamesStore = useGamesStore()

const isRunning = props.game.id === gamesStore.runningGameId

const platformBadgeColor = {
  steam: 'sky',
  epic: 'purple',
  gog: 'rose',
  minecraft: 'emerald',
  manual: 'zinc'
} as const
</script>

<template>
  <div
    class="group relative rounded-2xl overflow-hidden bg-zinc-900/60 border border-zinc-800/80 hover:border-amber-500/50 transition-all duration-300 flex flex-col cursor-pointer shadow-lg hover:shadow-2xl hover:shadow-amber-500/10 hover:-translate-y-1 select-none"
    @click="emit('select', game)"
  >
    <!-- 3:4 Poster Image Container -->
    <div class="relative w-full aspect-[3/4] overflow-hidden bg-zinc-950">
      <img
        :src="game.coverUrl"
        :alt="game.title"
        class="w-full h-full object-cover transition-transform duration-500 group-hover:scale-105"
        loading="lazy"
      />

      <!-- Dark Gradient Overlay -->
      <div class="absolute inset-0 bg-gradient-to-t from-zinc-950 via-zinc-950/20 to-transparent opacity-80 group-hover:opacity-90 transition-opacity" />

      <!-- Top Badges -->
      <div class="absolute top-3 left-3 right-3 flex items-center justify-between pointer-events-none">
        <UiBadge
          :variant="platformBadgeColor[game.platform]"
          size="xs"
          class="flex items-center gap-1.5"
        >
          <PlatformIcon :platform="game.platform" class-name="w-3 h-3" />
          <span class="capitalize">{{ game.platform }}</span>
        </UiBadge>

        <button
          type="button"
          @click.stop="gamesStore.toggleFavorite(game.id)"
          class="pointer-events-auto p-1.5 rounded-full bg-zinc-900/80 hover:bg-zinc-800 text-zinc-400 hover:text-amber-400 backdrop-blur-md transition-colors"
          title="Favori"
        >
          <Star
            class="w-3.5 h-3.5"
            :class="{ 'fill-amber-400 text-amber-400': game.isFavorite }"
          />
        </button>
      </div>

      <!-- Center Hover Quick Play Button -->
      <div class="absolute inset-0 flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity duration-200">
        <button
          type="button"
          @click.stop="gamesStore.launchGame(game.id)"
          class="p-4 rounded-full bg-amber-500 text-zinc-950 shadow-xl shadow-amber-500/40 hover:bg-amber-400 hover:scale-110 active:scale-95 transition-all"
          title="Lancer immédiatement"
        >
          <Play class="w-6 h-6 fill-current translate-x-0.5" />
        </button>
      </div>

      <!-- Running Status Indicator -->
      <div
        v-if="isRunning"
        class="absolute bottom-3 left-3 px-2 py-1 rounded-md bg-amber-500/90 text-zinc-950 text-[10px] font-bold font-mono uppercase tracking-wider flex items-center gap-1.5 shadow-md"
      >
        <span class="w-1.5 h-1.5 rounded-full bg-zinc-950 animate-ping" />
        En cours
      </div>
    </div>

    <!-- Info Bottom Footer -->
    <div class="p-3.5 flex flex-col justify-between flex-1 gap-2">
      <div>
        <h4 class="font-bold text-sm text-zinc-100 group-hover:text-amber-400 transition-colors line-clamp-1">
          {{ game.title }}
        </h4>
        <div class="flex items-center gap-1.5 mt-1 text-[11px] text-zinc-400">
          <span>{{ (game.playtimeMinutes / 60).toFixed(1) }} h jouées</span>
          <span>•</span>
          <span class="text-zinc-500 font-mono">{{ game.config.runner }}</span>
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
