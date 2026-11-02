<script setup lang="ts">
import { computed } from 'vue'
import { useRouter } from 'vue-router'
import {
  Play,
  Square,
  Trophy,
  Clock,
  HardDrive,
  Flame,
  ChevronRight,
  Sparkles,
  Newspaper,
  Calendar,
  Gamepad2
} from 'lucide-vue-next'
import { useGamesStore } from '@/stores/gamesStore'
import { useSystemStore } from '@/stores/systemStore'
import type { Game } from '@/models/game'
import UiButton from '@/components/ui/UiButton.vue'
import UiBadge from '@/components/ui/UiBadge.vue'
import UiProgressBar from '@/components/ui/UiProgressBar.vue'

const router = useRouter()
const gamesStore = useGamesStore()
const systemStore = useSystemStore()

const hero = computed(() => gamesStore.heroGame)

function openGame(game: Game) {
  router.push(`/games/${game.slug || game.id}`)
}
</script>

<template>
  <div class="space-y-8 pb-12">
    <!-- 1. HERO CINÉMATIQUE 21:9 -->
    <section v-if="hero" class="relative rounded-3xl overflow-hidden border border-zinc-800/80 shadow-2xl bg-zinc-950 group">
      <!-- Background Banner 21:9 format -->
      <div class="relative w-full aspect-[21/9] min-h-[360px] max-h-[460px] overflow-hidden">
        <img
          :src="hero.bannerUrl"
          :alt="hero.title"
          class="w-full h-full object-cover object-center transform scale-100 group-hover:scale-105 transition-transform duration-700 ease-out"
        />

        <!-- Multi-layered dark gradients for readability -->
        <div class="absolute inset-0 bg-gradient-to-t from-zinc-950 via-zinc-950/70 to-zinc-950/20" />
        <div class="absolute inset-0 bg-gradient-to-r from-zinc-950 via-zinc-950/80 to-transparent" />
      </div>

      <!-- Hero Content Overlay -->
      <div class="absolute inset-0 p-8 sm:p-10 flex flex-col justify-end">
        <div class="max-w-2xl space-y-4">
          <!-- Badges -->
          <div class="flex items-center gap-2">
            <UiBadge variant="amber" size="sm" :dot="true">
              Dernier Jeu Lancé
            </UiBadge>
            <UiBadge variant="zinc" size="sm">
              Proton-GE 9-20 • GameMode
            </UiBadge>
          </div>

          <!-- Title -->
          <h1 class="text-3xl sm:text-5xl font-black text-zinc-100 tracking-tight font-display drop-shadow-md">
            {{ hero.title }}
          </h1>

          <!-- Quick Metrics & Info -->
          <div class="flex items-center gap-4 text-xs font-medium text-zinc-300">
            <span class="flex items-center gap-1.5 bg-zinc-900/80 backdrop-blur-md px-3 py-1 rounded-full border border-zinc-700/60">
              <Clock class="w-3.5 h-3.5 text-amber-400" />
              {{ (hero.playtimeMinutes / 60).toFixed(1) }} heures de jeu
            </span>
            <span class="flex items-center gap-1.5 bg-zinc-900/80 backdrop-blur-md px-3 py-1 rounded-full border border-zinc-700/60">
              <Trophy class="w-3.5 h-3.5 text-amber-400" />
              {{ hero.achievements.filter(a => a.unlocked).length }}/{{ hero.achievements.length }} Succès débloqués
            </span>
            <span class="text-zinc-400 hidden md:inline">
              Dernière session : {{ hero.lastPlayed }}
            </span>
          </div>

          <!-- Call to action buttons & Recent Achievements preview -->
          <div class="pt-2 flex flex-wrap items-center gap-4">
            <UiButton
              v-if="gamesStore.runningGameId !== hero.id"
              variant="primary"
              size="lg"
              :loading="gamesStore.isLaunching"
              @click="gamesStore.launchGame(hero.id)"
              class="shadow-amber-500/25"
            >
              <template #prefix>
                <Play class="w-5 h-5 fill-current" />
              </template>
              JOUER MAINTENANT
            </UiButton>

            <UiButton
              v-else
              variant="danger"
              size="lg"
              @click="gamesStore.stopGame(hero.id)"
            >
              <template #prefix>
                <Square class="w-5 h-5 fill-current" />
              </template>
              ARRÊTER LA SESSION
            </UiButton>

            <UiButton
              variant="secondary"
              size="lg"
              @click="openGame(hero)"
            >
              Fiche Détaillée & Runner
            </UiButton>

            <!-- Mini Achievements Preview in Hero -->
            <div class="hidden lg:flex items-center gap-2 pl-4 border-l border-zinc-700/60">
              <span class="text-xs text-zinc-400">Succès Récents:</span>
              <div
                v-for="ach in hero.achievements.slice(0, 3)"
                :key="ach.id"
                class="w-8 h-8 rounded-lg bg-zinc-800/90 border border-zinc-700 flex items-center justify-center text-sm shadow cursor-help"
                :title="`${ach.name} : ${ach.description}`"
              >
                {{ ach.icon }}
              </div>
            </div>
          </div>
        </div>
      </div>
    </section>

    <!-- Empty hero fallback if no games in library -->
    <section v-else class="p-10 rounded-3xl border border-zinc-800/80 bg-zinc-950/80 text-center space-y-4">
      <div class="w-14 h-14 rounded-2xl bg-amber-500/10 border border-amber-500/30 flex items-center justify-center text-amber-400 mx-auto text-2xl">
        <Gamepad2 class="w-7 h-7" />
      </div>
      <div class="space-y-1">
        <h2 class="text-xl font-bold text-white">Aucun jeu dans la bibliothèque</h2>
        <p class="text-xs text-zinc-400">Connectez votre compte Steam ou explorez IGDB pour démarrer votre collection.</p>
      </div>
      <div class="flex items-center justify-center gap-3 pt-2">
        <UiButton variant="primary" size="sm" @click="router.push('/accounts')">
          Lier un compte Steam
        </UiButton>
        <UiButton variant="secondary" size="sm" @click="router.push('/library')">
          Aller à la bibliothèque
        </UiButton>
      </div>
    </section>

    <!-- 2. CARROUSEL HORIZONTAL DES DERNIERS JEUX JOUÉS -->
    <section v-if="gamesStore.recentGames.length > 0" class="space-y-4">
      <div class="flex items-center justify-between">
        <div class="flex items-center gap-2.5">
          <Flame class="w-5 h-5 text-amber-400" />
          <h2 class="text-lg font-bold text-zinc-100 tracking-tight">
            Reprendre une Partie
          </h2>
        </div>
        <button
          type="button"
          @click="router.push('/library')"
          class="text-xs font-semibold text-amber-400 hover:text-amber-300 flex items-center gap-1 group"
        >
          <span>Voir toute la bibliothèque ({{ gamesStore.games.length }})</span>
          <ChevronRight class="w-4 h-4 transform group-hover:translate-x-0.5 transition-transform" />
        </button>
      </div>

      <!-- Horizontal scroll list -->
      <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 gap-4">
        <div
          v-for="game in gamesStore.recentGames"
          :key="game.id"
          @click="openGame(game)"
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

            <!-- Quick play hover -->
            <button
              type="button"
              @click.stop="gamesStore.launchGame(game.id)"
              class="absolute bottom-3 right-3 p-2.5 rounded-full bg-amber-500 text-zinc-950 shadow-lg opacity-0 group-hover:opacity-100 transform translate-y-2 group-hover:translate-y-0 transition-all"
              title="Lancer"
            >
              <Play class="w-4 h-4 fill-current translate-x-0.5" />
            </button>
          </div>

          <div class="p-3">
            <h4 class="font-bold text-xs text-zinc-200 group-hover:text-amber-400 transition-colors truncate">
              {{ game.title }}
            </h4>
            <div class="flex items-center justify-between text-[11px] text-zinc-500 mt-1">
              <span>{{ (game.playtimeMinutes / 60).toFixed(0) }} h</span>
              <span>{{ game.config.runner }}</span>
            </div>
          </div>
        </div>
      </div>
    </section>

    <!-- 3. STATS SYSTÈME & TIMELINE DES ACTUALITÉS (GRID 2 COLS) -->
    <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
      <!-- Widget Stats & Activité (1 col) -->
      <section class="lg:col-span-1 rounded-2xl bg-zinc-900/60 border border-zinc-800/80 p-6 space-y-6">
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-2">
            <Sparkles class="w-4 h-4 text-amber-400" />
            <h3 class="font-bold text-sm text-zinc-100">Activité & Stockage</h3>
          </div>
          <UiBadge variant="amber" size="xs">Linux Gaming</UiBadge>
        </div>

        <!-- Weekly playtime -->
        <div class="p-4 rounded-xl bg-zinc-950/70 border border-zinc-800/80 space-y-1.5">
          <div class="flex items-center justify-between text-xs text-zinc-400">
            <span>Temps de jeu cette semaine</span>
            <span class="text-amber-400 font-bold font-mono">+4.2h vs sem. dernière</span>
          </div>
          <div class="text-2xl font-black text-zinc-100 font-display">
            {{ systemStore.stats.weeklyPlaytimeHours }} Heures
          </div>
          <div class="text-[11px] text-zinc-500">
            Jeu le plus actif : <span class="text-zinc-300 font-medium">Cyberpunk 2077</span>
          </div>
        </div>

        <!-- Disks Partition Gauges -->
        <div class="space-y-4">
          <div class="text-xs font-semibold text-zinc-400 uppercase tracking-wider font-mono flex items-center gap-1.5">
            <HardDrive class="w-3.5 h-3.5 text-zinc-500" />
            Partitions de Jeux
          </div>

          <div
            v-for="disk in systemStore.stats.disks"
            :key="disk.mountPoint"
            class="space-y-2 p-3 rounded-xl bg-zinc-950/40 border border-zinc-800/60"
          >
            <div class="flex items-center justify-between text-xs">
              <span class="font-medium text-zinc-200 truncate max-w-[170px]">{{ disk.label }}</span>
              <span class="font-mono text-zinc-400">{{ disk.freeGb }} Go libres</span>
            </div>
            <UiProgressBar
              :value="disk.usedPercentage"
              :variant="disk.isGamePartition ? 'amber' : 'emerald'"
              height="sm"
            />
            <div class="flex justify-between text-[10px] text-zinc-500 font-mono">
              <span>{{ disk.mountPoint }}</span>
              <span>{{ disk.usedPercentage }}% utilisé</span>
            </div>
          </div>
        </div>
      </section>

      <!-- 4. Timeline des actualités (2 cols) -->
      <section class="lg:col-span-2 rounded-2xl bg-zinc-900/60 border border-zinc-800/80 p-6 space-y-5">
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-2">
            <Newspaper class="w-4 h-4 text-amber-400" />
            <h3 class="font-bold text-sm text-zinc-100">Actualités & Patch Notes</h3>
          </div>
          <span class="text-xs text-zinc-500 font-mono">Jeux Installés</span>
        </div>

        <!-- Vertical Timeline Items -->
        <div class="space-y-4">
          <div
            v-for="item in gamesStore.allNews"
            :key="item.news.id"
            class="p-4 rounded-xl bg-zinc-950/60 border border-zinc-800/80 hover:border-zinc-700 transition-all space-y-2"
          >
            <div class="flex items-center justify-between gap-2">
              <div class="flex items-center gap-2">
                <UiBadge variant="amber" size="xs">
                  {{ item.game.title }}
                </UiBadge>
                <span class="text-xs font-semibold text-zinc-400 flex items-center gap-1 font-mono">
                  <Calendar class="w-3 h-3 text-zinc-500" />
                  {{ item.news.date }}
                </span>
              </div>
              <span class="text-[11px] text-zinc-500">Par {{ item.news.author }}</span>
            </div>

            <h4 class="font-bold text-sm text-zinc-100 hover:text-amber-400 transition-colors cursor-pointer" @click="openGame(item.game)">
              {{ item.news.title }}
            </h4>

            <p class="text-xs text-zinc-400 leading-relaxed">
              {{ item.news.summary }}
            </p>
          </div>
        </div>
      </section>
    </div>
  </div>
</template>
