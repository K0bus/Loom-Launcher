<script setup lang="ts">
import { computed } from 'vue'
import {
  Play,
  Square,
  Plus,
  Layers,
  Package,
  Cpu,
  Clock,
  Calendar,
  ShieldCheck
} from 'lucide-vue-next'
import { useMinecraftStore } from '@/stores/minecraft'
import MinecraftBreadcrumb from '@/components/minecraft/MinecraftBreadcrumb.vue'
import MinecraftAvatar from '@/components/minecraft/MinecraftAvatar.vue'
import UiBadge from '@/components/ui/UiBadge.vue'

const emit = defineEmits<{
  (e: 'create-instance'): void
}>()

const minecraftStore = useMinecraftStore()

const instances = computed(() => minecraftStore.instances)
const lastPlayed = computed(() => minecraftStore.lastPlayedInstance)
const account = computed(() => minecraftStore.account)
const globalPlaytime = computed(() => minecraftStore.globalPlaytimeFormatted)
const globalLastPlayed = computed(() => minecraftStore.globalLastPlayedFormatted)

const defaultBanner = computed(() => minecraftStore.defaultBannerUrl)
const defaultCover = computed(() => minecraftStore.defaultCoverUrl)
const igdbSummary = computed(() => minecraftStore.igdbSummary)
const igdbRating = computed(() => minecraftStore.igdbRating)

const heroBanner = computed(() => {
  return lastPlayed.value?.bannerUrl || defaultBanner.value
})

const isAnyRunning = computed(() => {
  return instances.value.some((i) => i.status === 'running')
})

const runningInstance = computed(() => {
  return instances.value.find((i) => i.status === 'running')
})

const totalModsCount = computed(() => {
  return instances.value.reduce((acc, i) => acc + (i.mods?.length || 0), 0)
})

function handlePrimaryPlay() {
  if (isAnyRunning.value && runningInstance.value) {
    minecraftStore.stopInstance(runningInstance.value.id)
    return
  }

  if (lastPlayed.value) {
    minecraftStore.launchInstance(lastPlayed.value.id)
  } else {
    emit('create-instance')
  }
}
</script>

<template>
  <div class="relative w-full min-h-[500px] lg:min-h-[560px] flex flex-col justify-between overflow-hidden select-none border-b border-zinc-800/80">
    <!-- 1. Background Cinematic Artwork -->
    <div class="absolute inset-0 z-0">
      <img
        :src="heroBanner"
        alt="Minecraft IGDB Hero"
        class="w-full h-full object-cover object-[center_35%] scale-105 transition-transform duration-1000 ease-out"
      />
      <!-- Hero glowing overlays (matching GameDetailView) -->
      <div class="absolute inset-0 hero-vignette pointer-events-none" />
      <div class="absolute inset-0 bg-gradient-to-t from-zinc-950 via-zinc-950/60 to-transparent pointer-events-none" />
    </div>

    <!-- 2. TOP ROW : Breadcrumb & Microsoft Auth Pill -->
    <div class="relative z-20 px-6 sm:px-10 lg:px-14 pt-6 flex flex-wrap items-center justify-between gap-4">
      <!-- Breadcrumb -->
      <MinecraftBreadcrumb
        :items="[
          { label: 'Modding', to: '/modding' },
          { label: 'Minecraft', active: true }
        ]"
      />

      <!-- Microsoft Session Status Pill -->
      <div class="flex items-center gap-3">
        <div
          v-if="account"
          class="flex items-center gap-3 px-3.5 py-1.5 rounded-full bg-zinc-900/80 border border-zinc-800/80 backdrop-blur-md text-xs shadow-md"
        >
          <MinecraftAvatar
            :uuid="account.uuid"
            :name="account.name"
            :skin-url="account.skin_url"
            :size="28"
          />
          <div class="flex flex-col min-w-0 pr-1">
            <span class="font-bold text-zinc-100 text-xs truncate">{{ account.name }}</span>
            <span class="text-[10px] text-emerald-400 font-mono flex items-center gap-1">
              <span class="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse" />
              Session Xbox Live active
            </span>
          </div>
        </div>

        <button
          v-else
          type="button"
          @click="minecraftStore.startMicrosoftLogin"
          class="flex items-center gap-2 px-3.5 py-1.5 rounded-full bg-zinc-900/80 hover:bg-zinc-800 border border-amber-500/40 text-xs font-mono text-amber-300 hover:text-amber-200 backdrop-blur-md transition shadow-md"
        >
          <ShieldCheck class="w-3.5 h-3.5 text-amber-400" />
          <span>Lier compte Microsoft</span>
        </button>
      </div>
    </div>

    <!-- 3. HERO CONTENT : Title, Cover & Action Buttons -->
    <div class="relative z-20 px-6 sm:px-10 lg:px-14 pb-10 pt-8 max-w-7xl w-full">
      <div class="flex flex-col sm:flex-row items-start sm:items-end gap-6 mb-6">
        <!-- 3:4 Box Art Cover Poster -->
        <div class="w-24 sm:w-28 md:w-32 aspect-[3/4] rounded-2xl overflow-hidden border-2 border-zinc-700/80 shadow-[0_16px_40px_rgba(0,0,0,0.85)] flex-shrink-0 bg-zinc-950 hidden sm:block">
          <img
            :src="defaultCover"
            alt="Minecraft IGDB Cover"
            class="w-full h-full object-cover"
          />
        </div>

        <!-- Typographic Stylized Logo -->
        <div class="space-y-1.5 max-w-2xl">
          <div class="flex items-center gap-2 text-zinc-400 text-xs tracking-widest font-mono uppercase">
            <span>MOJANG STUDIOS</span>
            <span>•</span>
            <span class="text-amber-400/90 font-medium font-sans">JAVA EDITION</span>
            <span>•</span>
            <a
              href="https://www.igdb.com/games/minecraft--1"
              target="_blank"
              rel="noopener noreferrer"
              class="text-zinc-400 hover:text-amber-400 font-sans transition-colors underline-offset-2 hover:underline"
              title="Voir la fiche officielle sur IGDB"
            >
              IGDB {{ igdbRating }}%
            </a>
          </div>

          <h1 class="font-cinzel text-4xl sm:text-6xl font-black tracking-wider text-white drop-shadow-[0_4px_24px_rgba(0,0,0,0.9)]">
            MINECRAFT
          </h1>

          <div class="flex items-center gap-2 pt-1 text-xs sm:text-sm text-zinc-300 font-medium">
            <span>Sandbox</span>
            <span>•</span>
            <span>Survie</span>
            <span>•</span>
            <span>Aventure</span>
            <span>•</span>
            <span class="text-amber-400 font-mono font-bold">Modding Hermétique</span>
          </div>

          <!-- Official IGDB Summary Description -->
          <p class="text-xs sm:text-sm text-zinc-300/80 line-clamp-2 pt-1.5 leading-relaxed font-sans hidden md:block">
            {{ igdbSummary }}
          </p>
        </div>
      </div>

      <!-- Play CTA + Secondary Button + Metrics Pill Bar -->
      <div class="flex flex-wrap items-center gap-4">
        <!-- Primary Action CTA Button -->
        <button
          type="button"
          @click="handlePrimaryPlay"
          :class="[
            'flex items-center gap-3 px-8 py-3.5 rounded-2xl font-bold text-sm tracking-wide active:scale-95 transition-all duration-200 shadow-xl select-none',
            isAnyRunning
              ? 'bg-rose-500 hover:bg-rose-600 text-white shadow-rose-500/30 ring-2 ring-rose-400/50 animate-pulse'
              : 'bg-gradient-to-r from-amber-500 via-amber-400 to-amber-500 text-zinc-950 shadow-amber-500/30 hover:brightness-110'
          ]"
        >
          <Square v-if="isAnyRunning" class="w-5 h-5 fill-current" />
          <Play v-else class="w-5 h-5 fill-current translate-x-0.5" />

          <div class="flex flex-col text-left">
            <span class="text-xs uppercase font-mono tracking-wider opacity-80 leading-none">
              {{ isAnyRunning ? 'Jeu actif' : globalLastPlayed !== 'Jamais jouée' ? `Session : ${globalLastPlayed}` : 'Prêt au lancement' }}
            </span>
            <span class="text-sm font-black leading-tight">
              {{ isAnyRunning ? `Arrêter ${runningInstance?.name}` : lastPlayed ? `Jouer : ${lastPlayed.name}` : 'Créer une instance' }}
            </span>
          </div>

          <UiBadge
            v-if="!isAnyRunning && lastPlayed"
            variant="zinc"
            size="xs"
            class="font-mono text-[10px] ml-1 bg-zinc-950/20 text-zinc-950 border-zinc-950/20"
          >
            {{ lastPlayed.gameVersion }} • {{ lastPlayed.loader }}
          </UiBadge>
        </button>

        <!-- Secondary : Create New Instance -->
        <button
          type="button"
          @click="emit('create-instance')"
          class="flex items-center gap-2 px-5 py-3.5 rounded-2xl font-bold text-xs bg-zinc-900/90 hover:bg-zinc-800 border border-zinc-700/80 text-zinc-200 hover:text-amber-400 transition-all shadow-md active:scale-95"
        >
          <Plus class="w-4 h-4 text-amber-400" />
          <span>Nouvelle Instance</span>
        </button>

        <!-- Metrics Pills Bar -->
        <div class="hidden xl:flex items-center gap-4 px-5 py-2.5 rounded-2xl bg-zinc-950/70 border border-zinc-800/80 backdrop-blur-md text-xs font-mono">
          <div class="flex items-center gap-2 text-zinc-300">
            <Layers class="w-4 h-4 text-amber-400" />
            <span><strong>{{ instances.length }}</strong> instance{{ instances.length > 1 ? 's' : '' }}</span>
          </div>

          <span class="text-zinc-700">•</span>

          <div class="flex items-center gap-2 text-zinc-300">
            <Package class="w-4 h-4 text-emerald-400" />
            <span><strong>{{ totalModsCount }}</strong> mods</span>
          </div>

          <span class="text-zinc-700">•</span>

          <div class="flex items-center gap-2 text-zinc-300">
            <Clock class="w-4 h-4 text-amber-400" />
            <span><strong>{{ globalPlaytime }}</strong> de jeu</span>
          </div>

          <span v-if="globalLastPlayed !== 'Jamais jouée'" class="text-zinc-700">•</span>

          <div v-if="globalLastPlayed !== 'Jamais jouée'" class="flex items-center gap-2 text-zinc-300">
            <Calendar class="w-4 h-4 text-sky-400" />
            <span>Joué <strong>{{ globalLastPlayed }}</strong></span>
          </div>

          <span class="text-zinc-700">•</span>

          <div class="flex items-center gap-2 text-zinc-300">
            <Cpu class="w-4 h-4 text-sky-400" />
            <span>Adoptium Java 21</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
