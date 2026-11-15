<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import {
  Search,
  Download,
  Check,
  Loader2,
  Package,
  Layers,
  Palette,
  Sparkles
} from 'lucide-vue-next'
import { useMinecraftStore } from '@/stores/minecraft'
import { useDownloadsStore } from '@/stores/downloadsStore'
import type { UnifiedModSearchResult } from '@/models/modding'
import ModpackCard from '@/components/minecraft/ModpackCard.vue'
import ModDetailsModal from '@/components/minecraft/ModDetailsModal.vue'
import UiBadge from '@/components/ui/UiBadge.vue'
import UiButton from '@/components/ui/UiButton.vue'

export type CatalogContentType = 'mods' | 'resourcepacks' | 'shaderpacks' | 'modpacks'

interface Props {
  initialCategory?: CatalogContentType
}

const props = withDefaults(defineProps<Props>(), {
  initialCategory: 'mods'
})

const minecraftStore = useMinecraftStore()
const downloadsStore = useDownloadsStore()
const activeInst = computed(() => minecraftStore.activeInstance)

// Toggle between Mods, Resourcepacks, Shaders and Modpacks
const catalogMode = ref<CatalogContentType>(props.initialCategory)

const searchQuery = ref('')
const selectedProvider = ref<'all' | 'modrinth' | 'curseforge'>('all')

// Mods & Packs state
const isSearchingMods = computed(() => minecraftStore.isSearchingMods)
const modResults = computed(() => minecraftStore.modSearchResults)
const installingIds = computed(() => minecraftStore.installingModIds)

// Modpacks state
const isSearchingModpacks = computed(() => minecraftStore.isSearchingModpacks)
const modpackResults = computed(() => minecraftStore.modpackSearchResults)

// Placeholder based on category
const searchPlaceholder = computed(() => {
  switch (catalogMode.value) {
    case 'resourcepacks':
      return 'Rechercher des packs de textures (Faithful, Bare Bones, Stay True...)'
    case 'shaderpacks':
      return 'Rechercher des shaders (BSL, Complementary, AstraLex, Bliss...)'
    case 'modpacks':
      return 'Rechercher des modpacks (All The Mods, Better MC, Fabulously...)'
    default:
      return 'Rechercher des mods (JEI, JourneyMap, Sodium, Iris, Waystones...)'
  }
})

// Check if an item is already installed in the active instance
function isItemInstalled(item: UnifiedModSearchResult): boolean {
  if (!activeInst.value) return false
  const pType = item.project_type || (catalogMode.value === 'resourcepacks' ? 'resourcepack' : catalogMode.value === 'shaderpacks' ? 'shader' : 'mod')

  if (pType === 'resourcepack') {
    return (activeInst.value.resourcePacks || []).some(
      (m) =>
        m.remoteModId === item.id ||
        m.name.toLowerCase() === item.name.toLowerCase()
    )
  }
  if (pType === 'shader') {
    return (activeInst.value.shaderPacks || []).some(
      (m) =>
        m.remoteModId === item.id ||
        m.name.toLowerCase() === item.name.toLowerCase()
    )
  }
  return (activeInst.value.mods || []).some(
    (m) =>
      m.remoteModId === item.id ||
      m.name.toLowerCase() === item.name.toLowerCase()
  )
}

function getItemDownload(item: UnifiedModSearchResult) {
  if (!activeInst.value) return undefined
  return (
    downloadsStore.getModDownload(item.id, activeInst.value.id) ||
    downloadsStore.getModDownload(item.slug, activeInst.value.id) ||
    downloadsStore.getModDownload(item.name, activeInst.value.id)
  )
}

function handleSearch() {
  if (catalogMode.value === 'modpacks') {
    minecraftStore.searchModpacks(searchQuery.value, selectedProvider.value)
  } else if (catalogMode.value === 'resourcepacks') {
    minecraftStore.searchContent(searchQuery.value, 'resourcepack', selectedProvider.value)
  } else if (catalogMode.value === 'shaderpacks') {
    minecraftStore.searchContent(searchQuery.value, 'shader', selectedProvider.value)
  } else {
    minecraftStore.searchContent(searchQuery.value, 'mod', selectedProvider.value)
  }
}

function switchMode(mode: CatalogContentType) {
  catalogMode.value = mode
  searchQuery.value = ''
  handleSearch()
}

function setCategory(cat: CatalogContentType) {
  catalogMode.value = cat
  handleSearch()
}

defineExpose({ setCategory })

function setProvider(prov: 'all' | 'modrinth' | 'curseforge') {
  selectedProvider.value = prov
  handleSearch()
}

const detailsModalOpen = ref(false)
const selectedModProvider = ref('curseforge')
const selectedModId = ref('')
const selectedContentType = ref<'mod' | 'resourcepack' | 'shader'>('mod')

function openModDetails(item: UnifiedModSearchResult) {
  selectedModProvider.value = item.provider || (item as any).source || 'curseforge'
  selectedModId.value = item.id
  selectedContentType.value = item.project_type || (catalogMode.value === 'resourcepacks' ? 'resourcepack' : catalogMode.value === 'shaderpacks' ? 'shader' : 'mod')
  detailsModalOpen.value = true
}

function onItemInstalled() {
  if (activeInst.value) {
    minecraftStore.refreshInstanceMods(activeInst.value.id)
    minecraftStore.refreshInstanceResourcePacks(activeInst.value.id)
    minecraftStore.refreshInstanceShaderPacks(activeInst.value.id)
  }
}

async function installItem(item: UnifiedModSearchResult) {
  if (!activeInst.value) return
  const pType = item.project_type || (catalogMode.value === 'resourcepacks' ? 'resourcepack' : catalogMode.value === 'shaderpacks' ? 'shader' : 'mod')
  await minecraftStore.installModFromCatalog(activeInst.value.id, {
    ...item,
    project_type: pType
  })
}

onMounted(() => {
  if (catalogMode.value === 'modpacks') {
    if (modpackResults.value.length === 0) handleSearch()
  } else {
    if (modResults.value.length === 0) handleSearch()
  }
})
</script>

<template>
  <div class="space-y-6">
    <!-- MODE TOGGLE (Mods | Resourcepacks | Shaderpacks | Modpacks) -->
    <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
      <div class="flex flex-wrap items-center gap-1.5 p-1 bg-zinc-900/80 rounded-2xl border border-zinc-800">
        <!-- 1. Mods -->
        <button
          type="button"
          @click="switchMode('mods')"
          :class="[
            'flex items-center gap-2 px-3.5 py-2 rounded-xl text-xs font-mono font-bold transition-all select-none',
            catalogMode === 'mods'
              ? 'bg-amber-500 text-zinc-950 shadow-md shadow-amber-500/10'
              : 'text-zinc-400 hover:text-zinc-200'
          ]"
        >
          <Layers class="w-4 h-4" />
          <span>Mods</span>
          <span v-if="activeInst" class="text-[10px] text-zinc-500 font-normal hidden md:inline">
            ({{ activeInst.loader }} {{ activeInst.gameVersion }})
          </span>
        </button>

        <!-- 2. Resource Packs -->
        <button
          type="button"
          @click="switchMode('resourcepacks')"
          :class="[
            'flex items-center gap-2 px-3.5 py-2 rounded-xl text-xs font-mono font-bold transition-all select-none',
            catalogMode === 'resourcepacks'
              ? 'bg-amber-500 text-zinc-950 shadow-md shadow-amber-500/10'
              : 'text-zinc-400 hover:text-zinc-200'
          ]"
        >
          <Palette class="w-4 h-4" />
          <span>Packs de ressources</span>
        </button>

        <!-- 3. Shaders -->
        <button
          type="button"
          @click="switchMode('shaderpacks')"
          :class="[
            'flex items-center gap-2 px-3.5 py-2 rounded-xl text-xs font-mono font-bold transition-all select-none',
            catalogMode === 'shaderpacks'
              ? 'bg-amber-500 text-zinc-950 shadow-md shadow-amber-500/10'
              : 'text-zinc-400 hover:text-zinc-200'
          ]"
        >
          <Sparkles class="w-4 h-4" />
          <span>Shaders</span>
        </button>

        <!-- 4. Modpacks -->
        <button
          type="button"
          @click="switchMode('modpacks')"
          :class="[
            'flex items-center gap-2 px-3.5 py-2 rounded-xl text-xs font-mono font-bold transition-all select-none',
            catalogMode === 'modpacks'
              ? 'bg-amber-500 text-zinc-950 shadow-md shadow-amber-500/10'
              : 'text-zinc-400 hover:text-zinc-200'
          ]"
        >
          <Package class="w-4 h-4" />
          <span>Modpacks complets</span>
          <span class="text-[10px] px-1.5 py-0.5 rounded-full bg-amber-500/20 text-amber-400 font-bold ml-1">
            Clé-en-main
          </span>
        </button>
      </div>

      <div class="text-xs text-zinc-500 font-mono hidden sm:block">
        {{ catalogMode === 'modpacks' ? 'Crée une nouvelle instance autonome' : 'Ajout direct à cette instance' }}
      </div>
    </div>

    <!-- 1. SEARCH BAR & PROVIDER FILTERS -->
    <div class="flex flex-col md:flex-row items-stretch md:items-center justify-between gap-4 bg-zinc-900/60 p-5 rounded-3xl border border-zinc-800/80">
      <!-- Search input -->
      <form @submit.prevent="handleSearch" class="relative flex-1">
        <input
          v-model="searchQuery"
          type="text"
          :placeholder="searchPlaceholder"
          class="w-full bg-zinc-950 border border-zinc-800 rounded-2xl pl-11 pr-24 py-3 text-sm text-zinc-100 placeholder-zinc-500 focus:outline-none focus:border-amber-500/80 transition-colors"
        />
        <Search class="w-4 h-4 text-zinc-500 absolute left-4 top-1/2 -translate-y-1/2 pointer-events-none" />

        <UiButton
          type="submit"
          variant="primary"
          size="sm"
          class="absolute right-2 top-1/2 -translate-y-1/2 text-xs"
        >
          Chercher
        </UiButton>
      </form>

      <!-- Source Filters -->
      <div class="flex items-center gap-1.5 bg-zinc-950 p-1.5 rounded-2xl border border-zinc-800 shrink-0">
        <button
          type="button"
          @click="setProvider('all')"
          :class="[
            'px-3.5 py-1.5 rounded-xl text-xs font-mono font-bold transition-colors select-none',
            selectedProvider === 'all'
              ? 'bg-amber-500 text-zinc-950 shadow-sm'
              : 'text-zinc-400 hover:text-zinc-200'
          ]"
        >
          Tous
        </button>

        <button
          type="button"
          @click="setProvider('modrinth')"
          :class="[
            'px-3.5 py-1.5 rounded-xl text-xs font-mono font-bold transition-colors flex items-center gap-1.5 select-none',
            selectedProvider === 'modrinth'
              ? 'bg-emerald-500 text-zinc-950 shadow-sm'
              : 'text-zinc-400 hover:text-emerald-400'
          ]"
        >
          <span class="w-1.5 h-1.5 rounded-full bg-emerald-400" />
          Modrinth
        </button>

        <button
          type="button"
          @click="setProvider('curseforge')"
          :class="[
            'px-3.5 py-1.5 rounded-xl text-xs font-mono font-bold transition-colors flex items-center gap-1.5 select-none',
            selectedProvider === 'curseforge'
              ? 'bg-amber-500 text-zinc-950 shadow-sm'
              : 'text-zinc-400 hover:text-amber-400'
          ]"
        >
          <span class="w-1.5 h-1.5 rounded-full bg-amber-400" />
          CurseForge
        </button>
      </div>
    </div>

    <!-- ============================================== -->
    <!-- VIEW A: MODPACKS GRID                          -->
    <!-- ============================================== -->
    <div v-if="catalogMode === 'modpacks'">
      <!-- Loading State -->
      <div v-if="isSearchingModpacks" class="py-20 text-center">
        <Loader2 class="w-8 h-8 text-amber-500 animate-spin mx-auto mb-3" />
        <p class="text-sm text-zinc-300 font-mono">Recherche des modpacks en cours...</p>
      </div>

      <!-- Modpack Cards Grid -->
      <div
        v-else-if="modpackResults.length > 0"
        class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-5"
      >
        <ModpackCard
          v-for="pack in modpackResults"
          :key="`${pack.source}-${pack.id}`"
          :modpack="pack"
        />
      </div>

      <!-- Empty State -->
      <div
        v-else
        class="p-16 text-center rounded-3xl bg-zinc-900/30 border border-zinc-800/80 space-y-3"
      >
        <Package class="w-8 h-8 text-zinc-600 mx-auto" />
        <h4 class="text-sm font-bold text-zinc-300">Aucun modpack trouvé</h4>
        <p class="text-xs text-zinc-500 max-w-sm mx-auto">
          Modifiez vos mots-clés de recherche ou sélectionnez un autre fournisseur d'API.
        </p>
      </div>
    </div>

    <!-- ============================================== -->
    <!-- VIEW B: MODS / RESOURCE PACKS / SHADERS GRID   -->
    <!-- ============================================== -->
    <div v-else>
      <!-- Loading State -->
      <div v-if="isSearchingMods" class="py-20 text-center">
        <Loader2 class="w-8 h-8 text-amber-500 animate-spin mx-auto mb-3" />
        <p class="text-sm text-zinc-300 font-mono">
          {{
            catalogMode === 'resourcepacks'
              ? 'Recherche des packs de textures sur Modrinth & CurseForge...'
              : catalogMode === 'shaderpacks'
                ? 'Recherche des shaders sur Modrinth & CurseForge...'
                : 'Interrogation des dépôts Modrinth & CurseForge...'
          }}
        </p>
      </div>

      <!-- Results Grid -->
      <div
        v-else-if="modResults.length > 0"
        class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-5"
      >
        <div
          v-for="item in modResults"
          :key="`${item.provider}-${item.id}`"
          @click="openModDetails(item)"
          class="flex flex-col justify-between p-5 rounded-3xl bg-zinc-900/60 border border-zinc-800/80 hover:border-amber-500/40 transition-colors group select-none gap-4 cursor-pointer"
        >
          <div class="space-y-3">
            <!-- Header: Icon, Name & Source -->
            <div class="flex items-start gap-3">
              <div class="w-12 h-12 rounded-2xl bg-zinc-950 border border-zinc-800 shrink-0 overflow-hidden flex items-center justify-center p-1">
                <img
                  v-if="item.icon_url"
                  :src="item.icon_url"
                  :alt="item.name"
                  class="w-full h-full object-cover rounded-xl"
                  loading="lazy"
                />
                <Palette v-else-if="catalogMode === 'resourcepacks'" class="w-6 h-6 text-zinc-600" />
                <Sparkles v-else-if="catalogMode === 'shaderpacks'" class="w-6 h-6 text-zinc-600" />
                <Package v-else class="w-6 h-6 text-zinc-600" />
              </div>

              <div class="flex-1 min-w-0">
                <div class="flex items-center justify-between gap-2">
                  <h4 class="font-bold text-sm text-zinc-100 group-hover:text-amber-400 transition-colors truncate" :title="item.name">
                    {{ item.name }}
                  </h4>
                  <UiBadge
                    :variant="item.provider === 'modrinth' ? 'emerald' : 'amber'"
                    size="xs"
                    class="shrink-0 uppercase"
                  >
                    {{ item.provider }}
                  </UiBadge>
                </div>
                <p class="text-xs text-zinc-500 truncate mt-0.5">Par {{ item.author }}</p>
              </div>
            </div>

            <!-- Description -->
            <p class="text-xs text-zinc-400 line-clamp-2 leading-relaxed" :title="item.summary">
              {{ item.summary || 'Aucune description disponible.' }}
            </p>

            <!-- Tags -->
            <div v-if="item.categories && item.categories.length > 0" class="flex flex-wrap gap-1.5 pt-1">
              <span
                v-for="cat in item.categories.slice(0, 3)"
                :key="cat"
                class="px-2 py-0.5 rounded-md bg-zinc-950 border border-zinc-800 text-[10px] font-mono text-zinc-400"
              >
                {{ cat }}
              </span>
            </div>
          </div>

          <!-- Footer Card: Downloads & Install action -->
          <div class="pt-3 border-t border-zinc-800/80 flex items-center justify-between gap-3">
            <div class="text-[11px] font-mono text-zinc-500">
              {{ (item.downloads >= 1000000) ? (item.downloads / 1000000).toFixed(1) + 'M' : (item.downloads >= 1000) ? Math.round(item.downloads / 1000) + 'k' : item.downloads }} dl
            </div>

            <!-- 1. Installed badge -->
            <div
              v-if="isItemInstalled(item)"
              class="flex items-center gap-1.5 text-xs font-mono font-bold text-emerald-400 py-1.5 px-3 rounded-xl bg-emerald-500/10 border border-emerald-500/30"
            >
              <Check class="w-3.5 h-3.5" />
              <span>Installé</span>
            </div>

            <!-- 2. Currently downloading / installing -->
            <div
              v-else-if="getItemDownload(item) || installingIds[item.id]"
              class="relative overflow-hidden h-8 min-w-[130px] rounded-xl bg-zinc-950 border border-amber-500/50 shadow-inner flex items-center justify-center select-none"
            >
              <!-- Progress Fill Bar -->
              <div
                class="absolute inset-y-0 left-0 bg-gradient-to-r from-amber-600 via-amber-500 to-amber-400 transition-all duration-200 pointer-events-none"
                :style="{ width: `${Math.max(getItemDownload(item)?.progressPercent || 20, 6)}%` }"
              >
                <!-- Animated Shimmer -->
                <div class="absolute inset-0 bg-white/20 animate-pulse" />
              </div>

              <!-- Foreground Progress Label -->
              <div class="relative z-10 flex items-center gap-1.5 px-3 text-[11px] font-mono font-bold text-zinc-950 drop-shadow-[0_1px_1px_rgba(255,255,255,0.4)]">
                <Loader2 class="w-3 h-3 animate-spin text-zinc-950" />
                <span>
                  {{
                    getItemDownload(item)?.status === 'installing' || (getItemDownload(item)?.progressPercent || 0) >= 95
                      ? 'Finalisation...'
                      : getItemDownload(item)?.progressPercent
                        ? `${getItemDownload(item)!.progressPercent}%`
                        : 'Installation...'
                  }}
                </span>
              </div>
            </div>

            <!-- 3. Normal Install Button -->
            <UiButton
              v-else
              variant="primary"
              size="sm"
              @click.stop="installItem(item)"
            >
              <template #prefix>
                <Download class="w-3.5 h-3.5" />
              </template>
              Installer
            </UiButton>
          </div>
        </div>
      </div>

      <!-- Empty Search State -->
      <div
        v-else
        class="p-16 text-center rounded-3xl bg-zinc-900/30 border border-zinc-800/80 space-y-3"
      >
        <Palette v-if="catalogMode === 'resourcepacks'" class="w-8 h-8 text-zinc-600 mx-auto" />
        <Sparkles v-else-if="catalogMode === 'shaderpacks'" class="w-8 h-8 text-zinc-600 mx-auto" />
        <Package v-else class="w-8 h-8 text-zinc-600 mx-auto" />
        <h4 class="text-sm font-bold text-zinc-300">
          {{
            catalogMode === 'resourcepacks'
              ? 'Aucun pack de ressources trouvé'
              : catalogMode === 'shaderpacks'
                ? 'Aucun shaderpack trouvé'
                : 'Aucun mod trouvé'
          }}
        </h4>
        <p class="text-xs text-zinc-500 max-w-sm mx-auto">
          Modifiez vos mots-clés de recherche ou sélectionnez un autre fournisseur d'API.
        </p>
      </div>
    </div>

    <!-- Full Mod Details Modal -->
    <ModDetailsModal
      v-model="detailsModalOpen"
      :provider="selectedModProvider"
      :modId="selectedModId"
      :instanceId="activeInst?.id"
      :contentType="selectedContentType"
      @installed="onItemInstalled"
    />
  </div>
</template>
