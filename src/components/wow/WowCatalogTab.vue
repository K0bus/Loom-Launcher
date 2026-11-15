<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { useWowAddonsStore, WOW_CATEGORIES } from '@/stores/wowAddonsStore'
import type { WowAddonSearchResult } from '@/models/wowAddon'
import {
  Search,
  Download,
  CheckCircle2,
  ArrowUpCircle,
  Sparkles,
  ArrowUpDown,
  Info
} from 'lucide-vue-next'

const wowStore = useWowAddonsStore()

const categories = WOW_CATEGORIES
const addons = computed(() => wowStore.catalogAddons)
const isLoading = computed(() => wowStore.isLoadingCatalog)

let debounceTimer: any = null
function handleSearchInput() {
  clearTimeout(debounceTimer)
  debounceTimer = setTimeout(() => {
    wowStore.searchCatalog()
  }, 350)
}

function selectCategory(catId: number) {
  wowStore.selectedCategory = catId === 0 ? null : catId
  wowStore.searchCatalog()
}

function handleSortChange(event: Event) {
  const val = parseInt((event.target as HTMLSelectElement).value, 10)
  wowStore.sortField = val
  wowStore.searchCatalog()
}

function formatDownloads(count: number): string {
  if (count >= 1_000_000_000) return `${(count / 1_000_000_000).toFixed(1)}B`
  if (count >= 1_000_000) return `${(count / 1_000_000).toFixed(1)}M`
  if (count >= 1_000) return `${Math.round(count / 1_000)}k`
  return `${count}`
}

async function handleInstall(mod: WowAddonSearchResult) {
  await wowStore.installAddon({
    modId: mod.id,
    modName: mod.name,
    iconUrl: mod.iconUrl
  })
}

function openDetails(mod: WowAddonSearchResult) {
  wowStore.openDetailsModal(mod)
}

onMounted(() => {
  if (wowStore.catalogAddons.length === 0) {
    wowStore.searchCatalog()
  }
})
</script>

<template>
  <div class="space-y-6">
    <!-- Barre de recherche, filtres et tri -->
    <div class="bg-zinc-900/40 p-4 sm:p-5 rounded-2xl border border-zinc-800/80 space-y-4">
      <div class="flex flex-col sm:flex-row items-stretch sm:items-center justify-between gap-3">
        <!-- Recherche principale -->
        <div class="relative flex-1">
          <Search class="w-4 h-4 text-zinc-400 absolute left-3.5 top-1/2 -translate-y-1/2" />
          <input
            v-model="wowStore.searchQuery"
            @input="handleSearchInput"
            @keydown.enter="wowStore.searchCatalog()"
            type="text"
            placeholder="Rechercher parmi des milliers d'addons WoW (Questie, Details, DBM, Bartender)..."
            class="w-full pl-10 pr-4 py-2.5 bg-zinc-950/80 border border-zinc-800 rounded-xl text-xs text-zinc-200 placeholder-zinc-500 focus:outline-none focus:border-amber-500/80 transition font-mono"
          />
        </div>

        <!-- Sélecteur de tri -->
        <div class="flex items-center gap-2 self-end sm:self-auto flex-shrink-0">
          <ArrowUpDown class="w-3.5 h-3.5 text-zinc-400" />
          <select
            :value="wowStore.sortField"
            @change="handleSortChange"
            class="bg-zinc-950/80 border border-zinc-800 rounded-xl px-3 py-2 text-xs font-mono text-zinc-300 focus:outline-none focus:border-amber-500/80 transition"
          >
            <option :value="2">Popularité (Téléchargements)</option>
            <option :value="3">Dernière mise à jour</option>
            <option :value="1">Mis en avant (Featured)</option>
            <option :value="4">Ordre alphabétique</option>
          </select>
        </div>
      </div>

      <!-- Filtres par catégories (pilules horizontales) -->
      <div class="flex items-center gap-2 overflow-x-auto pb-1 scrollbar-none">
        <button
          v-for="cat in categories"
          :key="cat.id"
          type="button"
          @click="selectCategory(cat.id)"
          :class="[
            'px-3 py-1.5 rounded-xl text-xs font-mono whitespace-nowrap transition-all border flex-shrink-0',
            (cat.id === 0 && !wowStore.selectedCategory) || wowStore.selectedCategory === cat.id
              ? 'bg-amber-500 text-zinc-950 font-bold border-amber-400 shadow-md shadow-amber-500/20'
              : 'bg-zinc-950/60 text-zinc-400 hover:text-zinc-200 border-zinc-800/80 hover:border-zinc-700'
          ]"
        >
          {{ cat.name }}
        </button>
      </div>
    </div>

    <!-- Chargement du catalogue -->
    <div
      v-if="isLoading"
      class="py-24 flex flex-col items-center justify-center gap-3 text-zinc-400"
    >
      <div class="w-10 h-10 rounded-2xl bg-zinc-900 border border-zinc-800 flex items-center justify-center text-amber-400 shadow-lg animate-spin">
        <Sparkles class="w-5 h-5" />
      </div>
      <span class="text-xs font-mono text-zinc-400">Interrogation de l'API CurseForge...</span>
    </div>

    <!-- Aucun résultat -->
    <div
      v-else-if="addons.length === 0"
      class="py-20 text-center bg-zinc-950/40 rounded-3xl border border-zinc-800/80 p-8 space-y-3"
    >
      <Search class="w-10 h-10 text-zinc-600 mx-auto" />
      <h3 class="text-sm font-bold text-zinc-300">Aucun addon trouvé</h3>
      <p class="text-xs text-zinc-500 max-w-sm mx-auto">
        Aucun résultat pour cette recherche ou catégorie pour la version sélectionnée.
      </p>
    </div>

    <!-- Grille de cartes Modrinth-style -->
    <div v-else class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
      <div
        v-for="mod in addons"
        :key="mod.id"
        class="bg-zinc-900/60 hover:bg-zinc-900/90 border border-zinc-800/80 hover:border-zinc-700/80 rounded-2xl p-5 flex flex-col justify-between gap-4 transition-all hover:shadow-xl hover:shadow-black/30 group"
      >
        <!-- En-tête : Logo + Titre + Auteur + Téléchargements -->
        <div class="space-y-3">
          <div class="flex items-start gap-3.5">
            <!-- Logo / Icône -->
            <div
              class="w-12 h-12 rounded-xl bg-zinc-950 border border-zinc-800/80 overflow-hidden flex-shrink-0 flex items-center justify-center cursor-pointer group-hover:scale-105 transition-transform"
              @click="openDetails(mod)"
            >
              <img
                v-if="mod.iconUrl"
                :src="mod.iconUrl"
                :alt="mod.name"
                class="w-full h-full object-cover"
                loading="lazy"
              />
              <span v-else class="text-xs font-black font-mono text-amber-400">
                {{ mod.name.slice(0, 2).toUpperCase() }}
              </span>
            </div>

            <!-- Titre + Auteur -->
            <div class="min-w-0 flex-1">
              <h4
                class="font-bold text-sm text-zinc-100 group-hover:text-amber-400 transition cursor-pointer line-clamp-1"
                @click="openDetails(mod)"
                :title="mod.name"
              >
                {{ mod.name }}
              </h4>
              <p class="text-[11px] font-mono text-zinc-500 truncate mt-0.5">
                Par <span class="text-zinc-400">{{ mod.author }}</span>
              </p>

              <!-- Métadonnées Téléchargements -->
              <div class="flex items-center gap-2.5 text-[11px] font-mono text-zinc-400 pt-1">
                <span class="flex items-center gap-1 text-amber-400/90 font-medium">
                  <Download class="w-3 h-3 text-amber-400" />
                  {{ formatDownloads(mod.downloadCount) }}
                </span>
                <span class="text-zinc-600">•</span>
                <span class="text-zinc-500">CurseForge #{{ mod.id }}</span>
              </div>
            </div>
          </div>

          <!-- Résumé / Summary -->
          <p class="text-xs text-zinc-400 line-clamp-2 leading-relaxed min-h-[2.5rem]">
            {{ mod.summary }}
          </p>

          <!-- Badges de catégories -->
          <div class="flex flex-wrap gap-1.5 pt-1">
            <span
              v-for="cat in mod.categories.slice(0, 3)"
              :key="cat.id"
              class="px-2 py-0.5 rounded-md bg-zinc-950/80 border border-zinc-800 text-[10px] font-mono text-zinc-400"
            >
              {{ cat.name }}
            </span>
          </div>
        </div>

        <!-- Pied de carte : Bouton d'action Installer / Installé -->
        <div class="pt-3.5 border-t border-zinc-800/80 flex items-center justify-between gap-3 mt-auto">
          <button
            type="button"
            @click="openDetails(mod)"
            class="text-xs font-mono text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800/40 px-2 py-1.5 rounded-lg transition flex items-center gap-1.5 -ml-1"
          >
            <Info class="w-3.5 h-3.5 text-zinc-400" />
            <span>Détails & Fichiers</span>
          </button>

          <!-- Bouton Installer / Installé -->
          <template v-if="mod.isInstalled">
            <!-- Si MàJ dispo -->
            <button
              v-if="mod.updateAvailable"
              type="button"
              @click="handleInstall(mod)"
              :disabled="wowStore.installingMap[mod.id]"
              class="flex items-center gap-1.5 px-3.5 py-1.5 rounded-xl bg-amber-500 hover:bg-amber-400 text-zinc-950 font-mono font-bold text-xs tracking-wide shadow-md shadow-amber-500/20 active:scale-95 transition disabled:opacity-50"
            >
              <ArrowUpCircle
                class="w-3.5 h-3.5"
                :class="{ 'animate-spin': wowStore.installingMap[mod.id] }"
              />
              <span>Mettre à jour</span>
            </button>

            <!-- Déjà installé et à jour -->
            <div
              v-else
              class="flex items-center gap-1.5 px-3 py-1.5 rounded-xl bg-zinc-800/80 border border-zinc-700/60 text-emerald-400 font-mono font-semibold text-xs"
            >
              <CheckCircle2 class="w-3.5 h-3.5" />
              <span>Installé</span>
            </div>
          </template>

          <!-- Pas encore installé -->
          <button
            v-else
            type="button"
            @click="handleInstall(mod)"
            :disabled="wowStore.installingMap[mod.id]"
            class="flex items-center gap-1.5 px-4 py-1.5 rounded-xl bg-amber-500 hover:bg-amber-400 text-zinc-950 font-mono font-bold text-xs tracking-wide shadow-md shadow-amber-500/20 active:scale-95 transition disabled:opacity-50"
          >
            <Download
              class="w-3.5 h-3.5"
              :class="{ 'animate-bounce': wowStore.installingMap[mod.id] }"
            />
            <span>{{ wowStore.installingMap[mod.id] ? 'Installation...' : 'Installer' }}</span>
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
