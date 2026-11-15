<script setup lang="ts">
import { ref, onMounted } from 'vue'
import {
  Search,
  Download,
  Check,
  Loader2,
  Sparkles
} from 'lucide-vue-next'
import { useModdingStore } from '@/stores/moddingStore'
import { useMinecraftStore } from '@/stores/minecraftStore'
import type { UnifiedModSearchResult } from '@/models/modding'
import UiBadge from '@/components/ui/UiBadge.vue'
import UiButton from '@/components/ui/UiButton.vue'
import ModDetailsModal from '@/components/minecraft/ModDetailsModal.vue'

const moddingStore = useModdingStore()
const minecraftStore = useMinecraftStore()

const detailsModalOpen = ref(false)
const selectedModProvider = ref('curseforge')
const selectedModId = ref('')

function openModDetails(mod: UnifiedModSearchResult) {
  selectedModProvider.value = mod.provider
  selectedModId.value = mod.id
  detailsModalOpen.value = true
}

function onModInstalled() {
  if (minecraftStore.activeInstance) {
    minecraftStore.refreshInstanceMods(minecraftStore.activeInstance.id)
  }
}

onMounted(() => {
  if (moddingStore.searchResults.length === 0) {
    moddingStore.searchMods()
  }
})

function formatDownloads(count: number): string {
  if (count >= 1000000) {
    return `${(count / 1000000).toFixed(1)}M`
  }
  if (count >= 1000) {
    return `${(count / 1000).toFixed(0)}k`
  }
  return String(count)
}

async function installMod(mod: UnifiedModSearchResult) {
  if (!minecraftStore.activeInstance) {
    const newInst = await minecraftStore.createInstance({
      name: `Instance ${mod.name}`,
      icon: '⛏️',
      gameVersion: mod.game_versions?.[0] || '1.20.1',
      loader: (mod.loaders?.[0] as any) || 'fabric',
      allocatedRamGb: 6
    })
    moddingStore.requestInstallMod(newInst.id, mod)
    return
  }
  moddingStore.requestInstallMod(minecraftStore.activeInstance.id, mod)
}

function isModInstalled(mod: UnifiedModSearchResult): boolean {
  if (!minecraftStore.activeInstance) return false
  const activeMods = minecraftStore.activeInstance.mods
  return activeMods.some(
    (m) =>
      m.name.toLowerCase() === mod.name.toLowerCase() ||
      m.fileName.toLowerCase().includes(mod.slug.toLowerCase()) ||
      m.id === mod.id
  )
}
</script>

<template>
  <div class="space-y-6">
    <!-- Search & Filter Controls -->
    <div class="bg-zinc-900/60 p-5 rounded-3xl border border-zinc-800/80 shadow-xl space-y-4">
      <div class="flex flex-col lg:flex-row items-stretch lg:items-center justify-between gap-4">
        <!-- Search bar -->
        <div class="relative flex-1">
          <Search class="absolute left-4 top-1/2 -translate-y-1/2 w-4 h-4 text-zinc-500" />
          <input
            v-model="moddingStore.searchQuery"
            type="text"
            placeholder="Rechercher des mods (ex: Sodium, Create, JEI, Iris, Shaders)..."
            @keyup.enter="moddingStore.searchMods"
            class="w-full bg-zinc-950 text-zinc-100 placeholder-zinc-500 text-xs rounded-2xl pl-11 pr-4 py-3 border border-zinc-800 focus:border-amber-500/70 focus:ring-2 focus:ring-amber-500/20 outline-none transition-all font-mono"
          />
          <button
            type="button"
            @click="moddingStore.searchMods"
            class="absolute right-2.5 top-1/2 -translate-y-1/2 px-3 py-1.5 rounded-xl bg-amber-500 text-zinc-950 font-bold text-xs hover:bg-amber-400 transition-colors"
          >
            Rechercher
          </button>
        </div>

        <!-- Target Version & Loader Selectors -->
        <div class="flex flex-wrap items-center gap-2.5">
          <div class="flex items-center gap-1.5 text-xs font-mono text-zinc-400">
            <span>Version cible :</span>
            <select
              v-model="moddingStore.selectedVersion"
              @change="moddingStore.searchMods"
              class="bg-zinc-950 text-zinc-200 border border-zinc-800 rounded-xl px-2.5 py-1.5 outline-none focus:border-amber-500 text-xs font-mono"
            >
              <option value="">Toutes</option>
              <option value="1.21.4">1.21.4 (Latest)</option>
              <option value="1.21.1">1.21.1</option>
              <option value="1.20.1">1.20.1 (LTS Mods)</option>
              <option value="1.19.2">1.19.2</option>
              <option value="1.16.5">1.16.5</option>
            </select>
          </div>

          <div class="flex items-center gap-1.5 text-xs font-mono text-zinc-400">
            <span>Loader :</span>
            <select
              v-model="moddingStore.selectedLoader"
              @change="moddingStore.searchMods"
              class="bg-zinc-950 text-zinc-200 border border-zinc-800 rounded-xl px-2.5 py-1.5 outline-none focus:border-amber-500 text-xs font-mono"
            >
              <option value="">Tous</option>
              <option value="fabric">Fabric</option>
              <option value="neoforge">NeoForge</option>
              <option value="forge">Forge</option>
              <option value="quilt">Quilt</option>
            </select>
          </div>
        </div>
      </div>

      <!-- Platform Provider Badges / Toggles -->
      <div class="flex items-center justify-between border-t border-zinc-800/60 pt-3">
        <div class="flex items-center gap-2">
          <span class="text-xs font-mono text-zinc-500">Plateformes :</span>
          <button
            type="button"
            @click="moddingStore.setProviderFilter('all')"
            :class="[
              'px-3 py-1 rounded-xl text-xs font-semibold border transition-all',
              moddingStore.selectedProviders.includes('all')
                ? 'bg-amber-500/15 border-amber-500/60 text-amber-300'
                : 'bg-zinc-950 border-zinc-800 text-zinc-400 hover:border-zinc-700'
            ]"
          >
            Toutes les sources
          </button>
          <button
            type="button"
            @click="moddingStore.setProviderFilter('modrinth')"
            :class="[
              'px-3 py-1 rounded-xl text-xs font-semibold border transition-all flex items-center gap-1.5',
              moddingStore.selectedProviders.includes('modrinth')
                ? 'bg-emerald-500/15 border-emerald-500/60 text-emerald-300 shadow-sm shadow-emerald-500/10'
                : 'bg-zinc-950 border-zinc-800 text-zinc-400 hover:border-zinc-700'
            ]"
          >
            <span class="w-2 h-2 rounded-full bg-emerald-400" />
            Modrinth v2
          </button>
          <button
            type="button"
            @click="moddingStore.setProviderFilter('curseforge')"
            :class="[
              'px-3 py-1 rounded-xl text-xs font-semibold border transition-all flex items-center gap-1.5',
              moddingStore.selectedProviders.includes('curseforge')
                ? 'bg-rose-500/15 border-rose-500/60 text-rose-300 shadow-sm shadow-rose-500/10'
                : 'bg-zinc-950 border-zinc-800 text-zinc-400 hover:border-zinc-700'
            ]"
          >
            <span class="w-2 h-2 rounded-full bg-rose-400" />
            CurseForge Core
          </button>
        </div>

        <div class="text-xs font-mono text-zinc-500 hidden sm:block">
          {{ moddingStore.searchResults.length }} mods disponibles
        </div>
      </div>
    </div>

    <!-- Loading spinner -->
    <div v-if="moddingStore.isSearching" class="py-20 text-center space-y-3">
      <Loader2 class="w-8 h-8 animate-spin mx-auto text-amber-400" />
      <div class="text-xs font-mono text-zinc-400">Interrogation simultanée des APIs Modrinth & CurseForge...</div>
    </div>

    <!-- Mod Results Grid -->
    <div v-else-if="moddingStore.searchResults.length > 0" class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-5">
      <div
        v-for="mod in moddingStore.searchResults"
        :key="`${mod.provider}-${mod.id}`"
        @click="openModDetails(mod)"
        class="p-5 rounded-3xl bg-zinc-900/60 border border-zinc-800/80 hover:border-zinc-700 transition-all flex flex-col justify-between gap-4 group hover:shadow-xl shadow-zinc-950 cursor-pointer"
      >
        <div class="space-y-3">
          <!-- Top Row: Icon, Title, and Provider Tag -->
          <div class="flex items-start gap-3.5">
            <div class="w-13 h-13 rounded-2xl bg-zinc-950 border border-zinc-800 flex items-center justify-center flex-shrink-0 overflow-hidden shadow">
              <img
                v-if="mod.icon_url"
                :src="mod.icon_url"
                :alt="mod.name"
                class="w-full h-full object-cover group-hover:scale-105 transition-transform"
                loading="lazy"
              />
              <span v-else class="text-2xl">📦</span>
            </div>

            <div class="min-w-0 flex-1">
              <div class="flex items-center justify-between gap-2">
                <h3 class="font-bold text-sm text-zinc-100 truncate group-hover:text-amber-400 transition-colors">
                  {{ mod.name }}
                </h3>
                <UiBadge
                  :variant="mod.provider === 'modrinth' ? 'emerald' : 'amber'"
                  size="xs"
                  class="font-mono uppercase flex-shrink-0"
                >
                  {{ mod.provider }}
                </UiBadge>
              </div>

              <div class="text-[11px] text-zinc-500 font-mono mt-0.5 truncate">
                par <span class="text-zinc-400">{{ mod.author }}</span>
              </div>
            </div>
          </div>

          <!-- Summary description -->
          <p class="text-xs text-zinc-400 line-clamp-2 leading-relaxed">
            {{ mod.summary }}
          </p>

          <!-- Badges: categories and loaders -->
          <div class="flex flex-wrap items-center gap-1.5 pt-1">
            <span
              v-for="cat in mod.categories.slice(0, 3)"
              :key="cat"
              class="px-2 py-0.5 rounded-lg bg-zinc-950 border border-zinc-800 text-[10px] font-mono text-zinc-400"
            >
              {{ cat }}
            </span>
            <span
              v-for="l in mod.loaders.slice(0, 2)"
              :key="l"
              class="px-2 py-0.5 rounded-lg bg-amber-500/10 border border-amber-500/30 text-[10px] font-mono text-amber-300 uppercase"
            >
              {{ l }}
            </span>
          </div>
        </div>

        <!-- Bottom row: downloads and action -->
        <div class="flex items-center justify-between pt-3 border-t border-zinc-800/60 font-mono text-xs">
          <div class="flex items-center gap-2 text-zinc-500 text-[11px]">
            <span>{{ formatDownloads(mod.downloads) }} DL</span>
            <span>•</span>
            <span v-if="mod.latest_version">v{{ mod.latest_version }}</span>
          </div>

          <UiButton
            v-if="!isModInstalled(mod)"
            variant="primary"
            size="sm"
            @click.stop="installMod(mod)"
            :loading="moddingStore.isResolvingDeps && moddingStore.pendingModToInstall?.id === mod.id"
          >
            <template #prefix>
              <Download class="w-3.5 h-3.5" />
            </template>
            Installer
          </UiButton>

          <UiBadge
            v-else
            variant="emerald"
            size="sm"
            class="flex items-center gap-1"
          >
            <Check class="w-3.5 h-3.5" />
            Installé
          </UiBadge>
        </div>
      </div>
    </div>

    <!-- Empty State -->
    <div v-else class="py-20 text-center space-y-3">
      <Sparkles class="w-10 h-10 mx-auto text-zinc-700" />
      <div class="text-sm font-bold text-zinc-400">Aucun mod trouvé</div>
      <p class="text-xs text-zinc-500 max-w-sm mx-auto">
        Modifiez vos termes de recherche ou ajustez les filtres de version et de plateforme.
      </p>
    </div>

    <!-- Full Mod Details Modal -->
    <ModDetailsModal
      v-model="detailsModalOpen"
      :provider="selectedModProvider"
      :modId="selectedModId"
      :instanceId="minecraftStore.activeInstance?.id"
      @installed="onModInstalled"
    />
  </div>
</template>
