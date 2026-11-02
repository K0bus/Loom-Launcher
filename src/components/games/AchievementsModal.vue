<script setup lang="ts">
import { ref, computed } from 'vue'
import type { Achievement, Game } from '@/models/game'
import UiModal from '@/components/ui/UiModal.vue'
import UiButton from '@/components/ui/UiButton.vue'
import { Trophy, CheckCircle2, Lock, Search, Sparkles } from 'lucide-vue-next'

interface Props {
  open: boolean
  game: Game
  achievements: Achievement[]
  loading?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  loading: false
})

const emit = defineEmits<{
  (e: 'close'): void
}>()

const searchQuery = ref('')
const filterTab = ref<'all' | 'unlocked' | 'locked'>('all')

const unlockedList = computed(() => props.achievements.filter((a) => a.unlocked))
const lockedList = computed(() => props.achievements.filter((a) => !a.unlocked))

const totalCount = computed(() => props.game.totalAchievementsCount || props.achievements.length || 0)
const unlockedCount = computed(() => unlockedList.value.length || props.game.unlockedAchievementsCount || 0)
const percentage = computed(() => {
  if (!totalCount.value) return 0
  return Math.round((unlockedCount.value / totalCount.value) * 100)
})

const filteredAchievements = computed(() => {
  let list = props.achievements

  if (filterTab.value === 'unlocked') {
    list = unlockedList.value
  } else if (filterTab.value === 'locked') {
    list = lockedList.value
  }

  if (searchQuery.value.trim()) {
    const q = searchQuery.value.trim().toLowerCase()
    list = list.filter(
      (a) =>
        a.name.toLowerCase().includes(q) ||
        (a.description && a.description.toLowerCase().includes(q))
    )
  }

  // Sort: unlocked first, then by unlock date descending, then rarity
  return [...list].sort((a, b) => {
    if (a.unlocked !== b.unlocked) {
      return a.unlocked ? -1 : 1
    }
    if (a.unlockTimestamp && b.unlockTimestamp) {
      return b.unlockTimestamp - a.unlockTimestamp
    }
    return (b.rarityPercentage || 0) - (a.rarityPercentage || 0)
  })
})

function formatUnlockDate(isoOrTs?: string | number): string {
  if (!isoOrTs) return ''
  try {
    const date = typeof isoOrTs === 'number' ? new Date(isoOrTs * 1000) : new Date(isoOrTs)
    return date.toLocaleDateString('fr-FR', {
      day: 'numeric',
      month: 'short',
      year: 'numeric'
    })
  } catch {
    return ''
  }
}
</script>

<template>
  <UiModal
    :open="open"
    :title="`Succès — ${game.title}`"
    :subtitle="`${unlockedCount} sur ${totalCount} succès débloqués (${percentage}%)`"
    max-width="3xl"
    @close="emit('close')"
  >
    <div class="space-y-6">
      <!-- Progress Bar Card -->
      <div class="p-4 rounded-xl bg-zinc-950/80 border border-zinc-800/80 space-y-3">
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-2">
            <Trophy class="w-4 h-4 text-amber-400" />
            <span class="text-xs font-bold text-zinc-200">Progression globale</span>
          </div>
          <span class="font-mono text-xs font-bold text-amber-300">{{ percentage }}%</span>
        </div>

        <div class="w-full h-2 rounded-full bg-zinc-800 overflow-hidden">
          <div
            class="h-full bg-gradient-to-r from-amber-500 to-amber-300 rounded-full transition-all duration-500 shadow-[0_0_12px_rgba(245,158,11,0.5)]"
            :style="{ width: `${percentage}%` }"
          />
        </div>
      </div>

      <!-- Filters & Search Toolbar -->
      <div class="flex flex-col sm:flex-row items-stretch sm:items-center justify-between gap-3">
        <!-- Filter Tabs -->
        <div class="flex items-center gap-1.5 p-1 rounded-xl bg-zinc-950/70 border border-zinc-800">
          <button
            type="button"
            @click="filterTab = 'all'"
            :class="[
              'px-3 py-1.5 rounded-lg text-xs font-semibold transition-all',
              filterTab === 'all'
                ? 'bg-zinc-800 text-white shadow-sm'
                : 'text-zinc-400 hover:text-zinc-200'
            ]"
          >
            Tous ({{ achievements.length }})
          </button>
          <button
            type="button"
            @click="filterTab = 'unlocked'"
            :class="[
              'px-3 py-1.5 rounded-lg text-xs font-semibold transition-all flex items-center gap-1.5',
              filterTab === 'unlocked'
                ? 'bg-amber-500/20 text-amber-300 border border-amber-500/30'
                : 'text-zinc-400 hover:text-zinc-200'
            ]"
          >
            <CheckCircle2 class="w-3 h-3 text-amber-400" />
            Débloqués ({{ unlockedList.length }})
          </button>
          <button
            type="button"
            @click="filterTab = 'locked'"
            :class="[
              'px-3 py-1.5 rounded-lg text-xs font-semibold transition-all flex items-center gap-1.5',
              filterTab === 'locked'
                ? 'bg-zinc-800 text-white shadow-sm'
                : 'text-zinc-400 hover:text-zinc-200'
            ]"
          >
            <Lock class="w-3 h-3 text-zinc-500" />
            Verrouillés ({{ lockedList.length }})
          </button>
        </div>

        <!-- Search Bar -->
        <div class="relative flex-1 sm:max-w-xs">
          <Search class="absolute left-3 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-zinc-500" />
          <input
            v-model="searchQuery"
            type="text"
            placeholder="Rechercher un succès..."
            class="w-full pl-9 pr-3 py-1.5 bg-zinc-950/80 border border-zinc-800 rounded-xl text-xs text-zinc-200 placeholder-zinc-500 outline-none focus:border-amber-500/60 transition-colors"
          />
        </div>
      </div>

      <!-- Loading State -->
      <div v-if="loading" class="py-16 text-center space-y-3">
        <div class="w-8 h-8 rounded-full border-2 border-amber-500 border-t-transparent animate-spin mx-auto" />
        <p class="text-xs text-zinc-400">Chargement des succès officiels Steam en direct...</p>
      </div>

      <!-- Empty State -->
      <div v-else-if="filteredAchievements.length === 0" class="py-12 text-center space-y-2">
        <Trophy class="w-8 h-8 text-zinc-600 mx-auto" />
        <p class="text-xs text-zinc-400">Aucun succès correspondant trouvé</p>
      </div>

      <!-- Achievements Grid List -->
      <div
        v-else
        class="max-h-[55vh] overflow-y-auto space-y-2.5 pr-1.5 custom-scrollbar"
      >
        <div
          v-for="ach in filteredAchievements"
          :key="ach.id"
          :class="[
            'p-3 rounded-xl border flex items-center gap-3.5 transition-all duration-200',
            ach.unlocked
              ? 'bg-zinc-900/70 border-zinc-800 hover:border-amber-500/40 hover:bg-zinc-900/90'
              : 'bg-zinc-950/40 border-zinc-900 opacity-60 hover:opacity-80'
          ]"
        >
          <!-- Icon -->
          <div
            class="relative w-12 h-12 rounded-lg overflow-hidden flex-shrink-0 bg-zinc-950 border border-zinc-800"
          >
            <img
              v-if="ach.icon && (ach.icon.startsWith('http') || ach.icon.startsWith('/'))"
              :src="ach.unlocked ? ach.icon : (ach.iconLocked || ach.icon)"
              :alt="ach.name"
              class="w-full h-full object-cover"
              :class="{ 'grayscale contrast-125 opacity-50': !ach.unlocked }"
              loading="lazy"
            />
            <div
              v-else
              class="w-full h-full flex items-center justify-center text-lg"
              :class="ach.unlocked ? 'bg-amber-500/10 text-amber-300' : 'bg-zinc-900 text-zinc-600'"
            >
              <Trophy class="w-5 h-5" />
            </div>

            <!-- Checkmark badge if unlocked -->
            <div
              v-if="ach.unlocked"
              class="absolute bottom-0 right-0 p-0.5 rounded-tl-md bg-emerald-500 text-zinc-950"
            >
              <CheckCircle2 class="w-2.5 h-2.5" />
            </div>
          </div>

          <!-- Title, Description & Unlock Date -->
          <div class="flex-1 min-w-0 space-y-1">
            <div class="flex items-center justify-between gap-2">
              <h4
                class="text-xs font-bold truncate"
                :class="ach.unlocked ? 'text-zinc-100' : 'text-zinc-400'"
              >
                {{ ach.name }}
              </h4>

              <!-- Unlock Date if available -->
              <span
                v-if="ach.unlocked && (ach.unlockTimestamp || ach.unlockedAt)"
                class="text-[10px] font-mono text-zinc-500 flex-shrink-0"
              >
                {{ formatUnlockDate(ach.unlockTimestamp || ach.unlockedAt) }}
              </span>
            </div>

            <p class="text-[11px] text-zinc-400 leading-snug line-clamp-2">
              {{ ach.description || 'Succès masqué ou sans description' }}
            </p>
          </div>

          <!-- Rarity Tag -->
          <div
            v-if="ach.rarityPercentage > 0"
            class="hidden sm:flex flex-col items-end flex-shrink-0 pl-2"
          >
            <span class="text-[9px] font-mono text-zinc-500 uppercase">Rareté</span>
            <span
              class="text-[11px] font-mono font-bold flex items-center gap-1"
              :class="ach.rarityPercentage <= 10 ? 'text-amber-400' : 'text-zinc-300'"
            >
              <Sparkles v-if="ach.rarityPercentage <= 10" class="w-2.5 h-2.5" />
              {{ ach.rarityPercentage.toFixed(1) }}%
            </span>
          </div>
        </div>
      </div>
    </div>

    <template #footer>
      <div class="flex items-center justify-between w-full">
        <span class="text-[11px] text-zinc-500 font-mono">
          Synchronisé depuis {{ game.platform.toUpperCase() }}
        </span>
        <UiButton variant="secondary" size="sm" @click="emit('close')">
          Fermer
        </UiButton>
      </div>
    </template>
  </UiModal>
</template>
