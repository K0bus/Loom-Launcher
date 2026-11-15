<script setup lang="ts">
import { computed } from 'vue'
import { useRouter } from 'vue-router'
import {
  Play,
  Square,
  Package,
  Cpu,
  Clock,
  ChevronRight
} from 'lucide-vue-next'
import type { MinecraftInstance } from '@/models/minecraft'
import { useMinecraftStore } from '@/stores/minecraft'
import { useDownloadsStore } from '@/stores/downloadsStore'
import UiBadge from '@/components/ui/UiBadge.vue'

const props = defineProps<{
  instance: MinecraftInstance
}>()

const router = useRouter()
const minecraftStore = useMinecraftStore()
const downloadsStore = useDownloadsStore()

const bannerImage = computed(() => {
  return props.instance.bannerUrl || props.instance.coverUrl || minecraftStore.defaultBannerUrl
})

const isRunning = computed(() => props.instance.status === 'running')

const activeDownloadCount = computed(() => {
  return downloadsStore.activeModDownloads.filter((d) => d.instanceId === props.instance.id).length
})

const isDownloading = computed(() => activeDownloadCount.value > 0)
const formatLastPlayed = minecraftStore.formatLastPlayed

function handleCardClick() {
  minecraftStore.selectInstance(props.instance.id)
  router.push(`/modding/minecraft/${props.instance.id}`)
}

function handleQuickLaunch(e: Event) {
  e.stopPropagation()
  if (isRunning.value) {
    minecraftStore.stopInstance(props.instance.id)
  } else {
    minecraftStore.launchInstance(props.instance.id)
  }
}
</script>

<template>
  <div
    @click="handleCardClick"
    class="relative rounded-3xl overflow-hidden border border-zinc-800/80 hover:border-amber-500/70 bg-zinc-900/60 hover:bg-zinc-900/90 transition-all duration-300 hover:scale-[1.015] hover:shadow-[0_16px_40px_rgba(0,0,0,0.6)] cursor-pointer group flex flex-col justify-between select-none"
  >
    <!-- 1. BANNER & OVERLAY HEADER -->
    <div class="relative h-44 w-full overflow-hidden bg-zinc-950">
      <img
        :src="bannerImage"
        :alt="instance.name"
        class="w-full h-full object-cover group-hover:scale-105 transition-transform duration-700 ease-out"
        loading="lazy"
      />
      <!-- Vignette and dark gradients -->
      <div class="absolute inset-0 bg-gradient-to-t from-zinc-950 via-zinc-950/40 to-transparent" />
      <div class="absolute inset-0 bg-gradient-to-r from-zinc-950/60 via-transparent to-zinc-950/60" />

      <!-- Top Badges & Status -->
      <div class="absolute top-3.5 inset-x-3.5 flex items-center justify-between gap-2 z-10">
        <!-- Modloader badge -->
        <UiBadge
          :variant="
            instance.loader === 'fabric'
              ? 'emerald'
              : instance.loader === 'neoforge'
                ? 'amber'
                : instance.loader === 'forge'
                  ? 'sky'
                  : 'zinc'
          "
          size="xs"
          class="font-mono font-bold uppercase tracking-wider shadow-md backdrop-blur-md"
        >
          {{ instance.loader }}
        </UiBadge>

        <!-- Running or Downloading pulse badge -->
        <div
          v-if="isRunning"
          class="flex items-center gap-1.5 px-2.5 py-1 rounded-full bg-emerald-500/20 border border-emerald-500/50 backdrop-blur-md text-[10px] font-mono font-bold text-emerald-400 shadow-md animate-pulse"
        >
          <span class="w-1.5 h-1.5 rounded-full bg-emerald-400" />
          <span>EN JEU</span>
        </div>

        <div
          v-else-if="isDownloading"
          class="flex items-center gap-1.5 px-2.5 py-1 rounded-full bg-amber-500/20 border border-amber-500/50 backdrop-blur-md text-[10px] font-mono font-bold text-amber-300 shadow-md"
        >
          <span class="w-1.5 h-1.5 rounded-full bg-amber-400 animate-ping" />
          <span>INSTALLATION</span>
        </div>

        <div
          v-else
          class="flex items-center gap-1.5 px-2.5 py-1 rounded-full bg-zinc-900/80 border border-zinc-700/60 backdrop-blur-md text-[10px] font-mono text-zinc-400"
        >
          <span>PRÊT</span>
        </div>
      </div>

      <!-- Quick Play Button Hover Overlay -->
      <div class="absolute inset-0 flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity duration-200 z-10 pointer-events-none">
        <button
          type="button"
          @click="handleQuickLaunch"
          :class="[
            'pointer-events-auto w-14 h-14 rounded-2xl flex items-center justify-center shadow-2xl transition-all duration-200 transform hover:scale-110 active:scale-95',
            isRunning
              ? 'bg-rose-500 hover:bg-rose-600 text-white shadow-rose-500/30'
              : 'bg-amber-500 hover:bg-amber-400 text-zinc-950 font-bold shadow-amber-500/40'
          ]"
          :title="isRunning ? 'Arrêter le jeu' : 'Lancer le jeu'"
        >
          <Square v-if="isRunning" class="w-6 h-6 fill-current" />
          <Play v-else class="w-6 h-6 fill-current translate-x-0.5" />
        </button>
      </div>

      <!-- Bottom overlay on banner: Title & Version -->
      <div class="absolute bottom-3 left-4 right-4 z-10">
        <div class="flex items-baseline gap-2">
          <h3 class="font-bold text-lg text-white group-hover:text-amber-400 transition-colors truncate drop-shadow-md">
            {{ instance.name }}
          </h3>
          <span class="text-xs font-mono font-bold text-amber-400/90 shrink-0">
            {{ instance.gameVersion }}
          </span>
        </div>
      </div>
    </div>

    <!-- 2. CARD BODY : Specs & Metrics -->
    <div class="p-5 space-y-4">
      <div class="grid grid-cols-4 gap-2 py-2 border-y border-zinc-800/60 text-xs font-mono">
        <!-- Mods count -->
        <div class="flex flex-col">
          <span class="text-[10px] text-zinc-500 uppercase tracking-wider">Mods</span>
          <span class="font-bold text-zinc-200 flex items-center gap-1 mt-0.5">
            <Package class="w-3.5 h-3.5 text-amber-400" />
            {{ instance.mods.length }}
          </span>
        </div>

        <!-- RAM allocated -->
        <div class="flex flex-col">
          <span class="text-[10px] text-zinc-500 uppercase tracking-wider">RAM</span>
          <span class="font-bold text-zinc-200 flex items-center gap-1 mt-0.5">
            <Cpu class="w-3.5 h-3.5 text-zinc-400" />
            {{ instance.allocatedRamGb }} Go
          </span>
        </div>

        <!-- Playtime -->
        <div class="flex flex-col">
          <span class="text-[10px] text-zinc-500 uppercase tracking-wider">Temps</span>
          <span class="font-bold text-amber-300 truncate mt-0.5 flex items-center gap-1">
            <Clock class="w-3.5 h-3.5 text-amber-400" />
            {{ minecraftStore.formatPlaytimeCompact(instance.playtimeSeconds) }}
          </span>
        </div>

        <!-- Last played -->
        <div class="flex flex-col">
          <span class="text-[10px] text-zinc-500 uppercase tracking-wider">Session</span>
          <span class="font-bold text-zinc-300 truncate mt-0.5 flex items-center gap-1">
            {{ formatLastPlayed(instance.lastPlayed) }}
          </span>
        </div>
      </div>

      <!-- 3. ACTIONS FOOTER -->
      <div class="flex items-center justify-between gap-3 pt-1">
        <!-- Manage link -->
        <div class="text-xs font-mono font-semibold text-zinc-400 group-hover:text-amber-400 transition-colors flex items-center gap-1">
          <span>Gérer l'instance</span>
          <ChevronRight class="w-3.5 h-3.5 transition-transform group-hover:translate-x-1" />
        </div>

        <!-- Quick Launch Mini Button -->
        <button
          type="button"
          @click="handleQuickLaunch"
          :class="[
            'px-3.5 py-1.5 rounded-xl font-mono text-xs font-bold transition-all flex items-center gap-1.5 shadow-sm active:scale-95',
            isRunning
              ? 'bg-rose-500/20 hover:bg-rose-500 text-rose-300 hover:text-white border border-rose-500/40'
              : 'bg-amber-500/15 hover:bg-amber-500 text-amber-300 hover:text-zinc-950 border border-amber-500/40'
          ]"
        >
          <Square v-if="isRunning" class="w-3 h-3 fill-current" />
          <Play v-else class="w-3 h-3 fill-current" />
          <span>{{ isRunning ? 'Arrêter' : 'Jouer' }}</span>
        </button>
      </div>
    </div>
  </div>
</template>
