<script setup lang="ts">
import { onMounted, computed } from 'vue'
import { useRouter } from 'vue-router'
import { useWowAddonsStore } from '@/stores/wowAddonsStore'
import {
  Package,
  Compass,
  ArrowLeft,
  FolderOpen,
  Download
} from 'lucide-vue-next'
import WowInstanceSelector from '@/components/wow/WowInstanceSelector.vue'
import WowInstalledTab from '@/components/wow/WowInstalledTab.vue'
import WowCatalogTab from '@/components/wow/WowCatalogTab.vue'
import WowAddonDetailsModal from '@/components/wow/WowAddonDetailsModal.vue'
import UiBadge from '@/components/ui/UiBadge.vue'

const router = useRouter()
const wowStore = useWowAddonsStore()

const activeTab = computed({
  get: () => wowStore.activeTab,
  set: (val) => (wowStore.activeTab = val)
})

const installedCount = computed(() => wowStore.installedCount)
const updatesCount = computed(() => wowStore.updatesAvailableCount)

onMounted(async () => {
  await wowStore.loadInstances()
  await wowStore.loadInstalledAddons(true)
})
</script>

<template>
  <div class="max-w-7xl mx-auto px-6 sm:px-10 lg:px-14 py-6 space-y-6 pb-24">
    <!-- 1. Fil d'ariane & Navigation de retour -->
    <div class="flex items-center justify-between gap-4">
      <button
        type="button"
        @click="router.push('/modding')"
        class="inline-flex items-center gap-2 text-xs font-mono text-zinc-400 hover:text-amber-400 transition"
      >
        <ArrowLeft class="w-4 h-4" />
        <span>Retour au Hub Modding</span>
      </button>

      <div class="flex items-center gap-2">
        <UiBadge variant="amber" size="xs" class="font-mono">
          WoW AddOns Engine
        </UiBadge>
        <UiBadge variant="zinc" size="xs" class="font-mono">
          CurseForge Core
        </UiBadge>
      </div>
    </div>

    <!-- 2. Bannière Hero World of Warcraft -->
    <div class="relative overflow-hidden rounded-3xl bg-gradient-to-br from-zinc-950 via-zinc-900 to-zinc-950 border border-amber-500/40 p-6 sm:p-8 shadow-2xl">
      <div class="absolute -right-16 -top-16 w-80 h-80 bg-amber-500/10 rounded-full blur-3xl pointer-events-none" />

      <div class="relative z-10 flex flex-col md:flex-row items-start md:items-center justify-between gap-6">
        <div class="flex items-start sm:items-center gap-5">
          <!-- Icône emblème WoW -->
          <div class="relative w-16 h-16 sm:w-20 sm:h-20 rounded-2xl bg-zinc-900 border border-zinc-700/80 flex items-center justify-center text-4xl shadow-xl flex-shrink-0">
            ⚔️
            <div class="absolute -bottom-1 -right-1 w-6 h-6 rounded-lg bg-amber-500 text-zinc-950 flex items-center justify-center text-[10px] font-black shadow-md">
              WoW
            </div>
          </div>

          <div class="space-y-1.5">
            <div class="flex flex-wrap items-center gap-3">
              <h1 class="text-2xl sm:text-3xl font-black text-zinc-100 tracking-tight font-display">
                World of Warcraft Addons Studio
              </h1>
              <UiBadge variant="emerald" size="xs" class="font-mono">
                PRÊT
              </UiBadge>
            </div>
            <p class="text-xs text-zinc-400 max-w-2xl leading-relaxed">
              Gestionnaire natif d'addons : explorez, installez et maintenez vos modules d'interface (Questie, Details, DBM, Auctionator) à jour directement sur vos instances de jeu.
            </p>
          </div>
        </div>

        <!-- Raccourcis rapides -->
        <div class="flex items-center gap-2.5 self-stretch md:self-auto justify-end">
          <!-- Bouton export sans texte avec tooltip -->
          <div class="relative group">
            <button
              type="button"
              @click="wowStore.exportBackup()"
              :disabled="wowStore.isExporting"
              class="p-2.5 rounded-xl bg-zinc-800/80 hover:bg-zinc-700 border border-zinc-700/80 text-zinc-300 hover:text-white transition flex items-center justify-center disabled:opacity-50"
              title="exporter les mods et config"
            >
              <Download
                class="w-4 h-4 text-amber-400"
                :class="{ 'animate-bounce': wowStore.isExporting }"
              />
            </button>
            <div class="absolute bottom-full right-0 mb-2 hidden group-hover:flex items-center px-2.5 py-1 rounded-lg bg-zinc-900 border border-zinc-700/80 text-[11px] font-mono text-zinc-200 whitespace-nowrap shadow-xl pointer-events-none z-30">
              exporter les mods et config
            </div>
          </div>

          <button
            type="button"
            @click="wowStore.openAddonsFolder()"
            class="px-4 py-2 rounded-xl bg-zinc-800/80 hover:bg-zinc-700 border border-zinc-700/80 text-xs font-mono text-zinc-300 hover:text-white transition flex items-center gap-1.5"
          >
            <FolderOpen class="w-3.5 h-3.5 text-amber-400" />
            <span>Dossier AddOns</span>
          </button>
        </div>
      </div>
    </div>

    <!-- 3. Sélecteur d'instances WoW (Forever, Retail, Classic, Classic Era) -->
    <WowInstanceSelector />

    <!-- 4. Onglets principaux : Installés vs Catalogue -->
    <div class="space-y-6">
      <div class="flex items-center justify-between border-b border-zinc-800/80 pb-3">
        <div class="flex items-center gap-2 sm:gap-4">
          <!-- Onglet Addons Installés -->
          <button
            type="button"
            @click="activeTab = 'installed'"
            :class="[
              'flex items-center gap-2 px-4 py-2 rounded-xl font-mono text-xs font-bold transition-all',
              activeTab === 'installed'
                ? 'bg-amber-500 text-zinc-950 shadow-md shadow-amber-500/20'
                : 'text-zinc-400 hover:text-zinc-200 hover:bg-zinc-900/60'
            ]"
          >
            <Package class="w-4 h-4" />
            <span>AddOns Installés</span>
            <span
              :class="[
                'px-1.5 py-0.2 rounded-full text-[10px]',
                activeTab === 'installed' ? 'bg-zinc-950 text-amber-400' : 'bg-zinc-800 text-zinc-300'
              ]"
            >
              {{ installedCount }}
            </span>

            <span
              v-if="updatesCount > 0"
              class="w-2 h-2 rounded-full bg-rose-500 animate-pulse ml-0.5"
              title="Mises à jour disponibles"
            />
          </button>

          <!-- Onglet Catalogue CurseForge -->
          <button
            type="button"
            @click="activeTab = 'catalog'"
            :class="[
              'flex items-center gap-2 px-4 py-2 rounded-xl font-mono text-xs font-bold transition-all',
              activeTab === 'catalog'
                ? 'bg-amber-500 text-zinc-950 shadow-md shadow-amber-500/20'
                : 'text-zinc-400 hover:text-zinc-200 hover:bg-zinc-900/60'
            ]"
          >
            <Compass class="w-4 h-4" />
            <span>Explorer le Catalogue</span>
          </button>
        </div>

        <div class="hidden sm:flex items-center gap-2 text-xs font-mono text-zinc-500">
          <span>Instance active :</span>
          <span class="text-amber-400 font-bold">
            {{ wowStore.selectedInstance?.name || 'World of Warcraft' }}
          </span>
        </div>
      </div>

      <!-- Contenu des onglets -->
      <div>
        <WowInstalledTab v-if="activeTab === 'installed'" />
        <WowCatalogTab v-else />
      </div>
    </div>

    <!-- Modale de détails d'addon -->
    <WowAddonDetailsModal />
  </div>
</template>
