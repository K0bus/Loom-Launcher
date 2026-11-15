<script setup lang="ts">
import { ref } from 'vue'
import type { GameFilter } from '@/models/game'
import UiModal from '@/components/ui/UiModal.vue'
import UiButton from '@/components/ui/UiButton.vue'

interface Props {
  open: boolean
  filters: GameFilter
}

const props = defineProps<Props>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'apply', filters: GameFilter): void
  (e: 'reset'): void
}>()

const localFilters = ref<GameFilter>({ ...props.filters })

const platforms = [
  { id: 'all', label: 'Toutes les plateformes' },
  { id: 'steam', label: 'Steam' },
  { id: 'battlenet', label: 'Blizzard Battle.net' },
  { id: 'epic', label: 'Epic Games' },
  { id: 'gog', label: 'GOG Galaxy' },
  { id: 'minecraft', label: 'Minecraft (Mojang)' }
]

const runners = [
  { id: 'all', label: 'Tous les runners' },
  { id: 'native', label: 'Linux Natif' },
  { id: 'proton-ge', label: 'Proton GE (Custom)' },
  { id: 'proton-experimental', label: 'Proton Experimental' },
  { id: 'wine-staging', label: 'Wine Staging' }
]

const genres = [
  { id: 'all', label: 'Tous les genres' },
  { id: 'Action RPG', label: 'Action RPG' },
  { id: 'Open World', label: 'Open World' },
  { id: 'Sandbox', label: 'Sandbox' },
  { id: 'Souls-like', label: 'Souls-like' },
  { id: 'CRPG', label: 'CRPG' },
  { id: 'Rogue-like', label: 'Rogue-like' },
  { id: 'Metroidvania', label: 'Metroidvania' }
]

function apply() {
  emit('apply', { ...localFilters.value })
  emit('close')
}

function reset() {
  localFilters.value = {
    search: '',
    platform: 'all',
    genre: 'all',
    runner: 'all',
    sortBy: 'lastPlayed',
    sortOrder: 'desc'
  }
  emit('apply', { ...localFilters.value })
  emit('close')
}
</script>

<template>
  <UiModal
    :open="open"
    title="Filtres Avancés de la Bibliothèque"
    subtitle="Affinez votre catalogue selon la compatibilité Linux et vos préférences"
    max-width="md"
    @close="emit('close')"
  >
    <div class="space-y-4">
      <!-- Platform -->
      <div>
        <label class="block text-xs font-semibold text-zinc-300 uppercase tracking-wider mb-2 font-mono">
          Plateforme Source
        </label>
        <div class="grid grid-cols-2 gap-2">
          <button
            v-for="p in platforms"
            :key="p.id"
            type="button"
            @click="localFilters.platform = p.id"
            :class="[
              'px-3 py-2 rounded-xl text-xs font-medium border text-left transition-all',
              localFilters.platform === p.id
                ? 'bg-amber-500/15 border-amber-500/50 text-amber-300 font-semibold shadow-sm'
                : 'bg-zinc-950/60 border-zinc-800 text-zinc-400 hover:border-zinc-700'
            ]"
          >
            {{ p.label }}
          </button>
        </div>
      </div>

      <!-- Runner / Compatibility Layer -->
      <div>
        <label class="block text-xs font-semibold text-zinc-300 uppercase tracking-wider mb-2 font-mono">
          Moteur / Runner Linux
        </label>
        <div class="grid grid-cols-2 gap-2">
          <button
            v-for="r in runners"
            :key="r.id"
            type="button"
            @click="localFilters.runner = r.id"
            :class="[
              'px-3 py-2 rounded-xl text-xs font-medium border text-left transition-all',
              localFilters.runner === r.id
                ? 'bg-amber-500/15 border-amber-500/50 text-amber-300 font-semibold shadow-sm'
                : 'bg-zinc-950/60 border-zinc-800 text-zinc-400 hover:border-zinc-700'
            ]"
          >
            {{ r.label }}
          </button>
        </div>
      </div>

      <!-- Genre -->
      <div>
        <label class="block text-xs font-semibold text-zinc-300 uppercase tracking-wider mb-2 font-mono">
          Genre
        </label>
        <select
          v-model="localFilters.genre"
          class="w-full bg-zinc-950/80 border border-zinc-800 rounded-xl px-3 py-2 text-xs text-zinc-200 outline-none focus:border-amber-500"
        >
          <option v-for="g in genres" :key="g.id" :value="g.id">
            {{ g.label }}
          </option>
        </select>
      </div>

      <!-- Sorting -->
      <div class="grid grid-cols-2 gap-3 pt-2">
        <div>
          <label class="block text-xs font-semibold text-zinc-300 uppercase tracking-wider mb-2 font-mono">
            Trier Par
          </label>
          <select
            v-model="localFilters.sortBy"
            class="w-full bg-zinc-950/80 border border-zinc-800 rounded-xl px-3 py-2 text-xs text-zinc-200 outline-none focus:border-amber-500"
          >
            <option value="lastPlayed">Dernier joué</option>
            <option value="playtime">Temps de jeu</option>
            <option value="title">Titre alphabétique</option>
            <option value="size">Espace disque</option>
          </select>
        </div>

        <div>
          <label class="block text-xs font-semibold text-zinc-300 uppercase tracking-wider mb-2 font-mono">
            Ordre
          </label>
          <select
            v-model="localFilters.sortOrder"
            class="w-full bg-zinc-950/80 border border-zinc-800 rounded-xl px-3 py-2 text-xs text-zinc-200 outline-none focus:border-amber-500"
          >
            <option value="desc">Décroissant</option>
            <option value="asc">Croissant</option>
          </select>
        </div>
      </div>
    </div>

    <template #footer>
      <UiButton variant="ghost" size="sm" @click="reset">
        Réinitialiser
      </UiButton>
      <UiButton variant="primary" size="sm" @click="apply">
        Appliquer les filtres
      </UiButton>
    </template>
  </UiModal>
</template>
