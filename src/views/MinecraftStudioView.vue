<script setup lang="ts">
import { ref, computed } from 'vue'
import {
  Box,
  Plus,
  Play,
  Square,
  Search,
  PackageOpen,
  Layers
} from 'lucide-vue-next'
import { useMinecraftStore } from '@/stores/minecraftStore'
import InstanceWizardModal from '@/components/minecraft/InstanceWizardModal.vue'
import ModpackCard from '@/components/minecraft/ModpackCard.vue'
import InstanceModsTab from '@/components/minecraft/InstanceModsTab.vue'
import InstanceServersTab from '@/components/minecraft/InstanceServersTab.vue'
import InstanceLogsTab from '@/components/minecraft/InstanceLogsTab.vue'
import InstanceGeneralTab from '@/components/minecraft/InstanceGeneralTab.vue'
import UiButton from '@/components/ui/UiButton.vue'
import UiBadge from '@/components/ui/UiBadge.vue'
import UiTabs from '@/components/ui/UiTabs.vue'

const minecraftStore = useMinecraftStore()

// Sub-navigation: "instances" | "catalog"
const subNav = ref<'instances' | 'catalog'>('instances')

// Wizard state
const wizardOpen = ref(false)

// Active instance tabs: "mods" | "servers" | "general" | "logs"
const instanceTab = ref('mods')

const instanceTabs = computed(() => [
  { id: 'mods', label: 'Gestion des Mods', badge: activeInst.value?.mods?.length || 0 },
  { id: 'servers', label: 'Serveurs Multijoueur', badge: activeInst.value?.servers?.length || 0 },
  { id: 'general', label: 'Mondes & Shaders' },
  { id: 'logs', label: 'Console & Logs en Direct' }
])

// Catalog search & filter
const catalogQuery = ref('')
const selectedCatalogSource = ref<'all' | 'modrinth' | 'curseforge'>('all')

const filteredCatalog = computed(() => {
  return minecraftStore.catalog.filter((pack) => {
    if (selectedCatalogSource.value !== 'all' && pack.source !== selectedCatalogSource.value) {
      return false
    }
    if (catalogQuery.value.trim()) {
      const q = catalogQuery.value.toLowerCase()
      return (
        pack.title.toLowerCase().includes(q) ||
        pack.author.toLowerCase().includes(q) ||
        pack.summary.toLowerCase().includes(q) ||
        pack.categories.some((c) => c.toLowerCase().includes(q))
      )
    }
    return true
  })
})

const activeInst = computed(() => minecraftStore.activeInstance)
const isInstanceRunning = computed(() => activeInst.value?.status === 'running')
</script>

<template>
  <div class="max-w-7xl mx-auto px-6 sm:px-10 lg:px-14 py-6 space-y-8 pb-16">
    <!-- 1. HEADER & SUB-NAVIGATION -->
    <div class="flex flex-col md:flex-row items-start md:items-center justify-between gap-4">
      <div>
        <h1 class="text-3xl font-black text-zinc-100 tracking-tight font-display flex items-center gap-3">
          <Box class="w-8 h-8 text-amber-400" />
          Minecraft Studio
        </h1>
        <p class="text-xs text-zinc-400 mt-1">
          Gestionnaire modulaire d'environnements, Fabric / NeoForge / Quilt et intégration Modrinth & CurseForge
        </p>
      </div>

      <!-- Right action: Create instance wizard -->
      <UiButton
        variant="primary"
        size="md"
        @click="wizardOpen = true"
      >
        <template #prefix>
          <Plus class="w-4 h-4" />
        </template>
        Créer une Instance
      </UiButton>
    </div>

    <!-- Sub-navigation Pills -->
    <div class="flex items-center gap-2 border-b border-zinc-800/80 pb-4">
      <button
        type="button"
        @click="subNav = 'instances'"
        :class="[
          'px-5 py-2 rounded-xl text-xs font-bold transition-all flex items-center gap-2 select-none',
          subNav === 'instances'
            ? 'bg-amber-500 text-zinc-950 shadow-md shadow-amber-500/20'
            : 'bg-zinc-900/80 text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800'
        ]"
      >
        <Layers class="w-4 h-4" />
        Mes Instances ({{ minecraftStore.instances.length }})
      </button>

      <button
        type="button"
        @click="subNav = 'catalog'"
        :class="[
          'px-5 py-2 rounded-xl text-xs font-bold transition-all flex items-center gap-2 select-none',
          subNav === 'catalog'
            ? 'bg-amber-500 text-zinc-950 shadow-md shadow-amber-500/20'
            : 'bg-zinc-900/80 text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800'
        ]"
      >
        <PackageOpen class="w-4 h-4" />
        Catalogue Modpacks (Modrinth & CurseForge)
      </button>
    </div>

    <!-- 2. VUE A : MES INSTANCES -->
    <div v-if="subNav === 'instances'" class="grid grid-cols-1 xl:grid-cols-4 gap-8">
      <!-- Left Column: Instance Selector List -->
      <div class="xl:col-span-1 space-y-3">
        <div class="flex items-center justify-between text-xs font-semibold text-zinc-400 font-mono">
          <span>Profils Actifs</span>
          <span class="text-zinc-500">{{ minecraftStore.instances.length }} créés</span>
        </div>

        <div class="space-y-2">
          <button
            v-for="inst in minecraftStore.instances"
            :key="inst.id"
            type="button"
            @click="minecraftStore.selectInstance(inst.id)"
            :class="[
              'w-full p-3.5 rounded-2xl border text-left transition-all flex items-center gap-3.5 select-none group',
              minecraftStore.selectedInstanceId === inst.id
                ? 'bg-zinc-900 border-amber-500/60 shadow-lg shadow-amber-500/5'
                : 'bg-zinc-950/60 border-zinc-800/80 hover:border-zinc-700 text-zinc-400'
            ]"
          >
            <div class="w-11 h-11 rounded-xl bg-zinc-900 border border-zinc-800 flex items-center justify-center text-2xl flex-shrink-0 group-hover:scale-105 transition-transform shadow">
              {{ inst.icon }}
            </div>
            <div class="min-w-0 flex-1">
              <div class="flex items-center justify-between gap-1">
                <span
                  :class="[
                    'font-bold text-xs truncate',
                    minecraftStore.selectedInstanceId === inst.id ? 'text-zinc-100' : 'text-zinc-300'
                  ]"
                >
                  {{ inst.name }}
                </span>
                <span
                  v-if="inst.status === 'running'"
                  class="w-2 h-2 rounded-full bg-amber-400 animate-ping flex-shrink-0"
                />
              </div>
              <div class="text-[11px] text-zinc-500 font-mono mt-0.5 flex items-center gap-1.5">
                <span class="uppercase text-amber-400/90 font-semibold">{{ inst.loader }}</span>
                <span>•</span>
                <span>{{ inst.gameVersion }}</span>
              </div>
            </div>
          </button>
        </div>

        <UiButton
          variant="outline"
          size="sm"
          block
          @click="wizardOpen = true"
          class="mt-4"
        >
          <template #prefix>
            <Plus class="w-3.5 h-3.5" />
          </template>
          Nouvelle Instance
        </UiButton>
      </div>

      <!-- Right Column: Detail of Selected Instance -->
      <div v-if="activeInst" class="xl:col-span-3 space-y-6">
        <!-- Instance Header Banner -->
        <div class="p-6 rounded-3xl bg-zinc-900/60 border border-zinc-800/80 shadow-xl flex flex-col md:flex-row items-start md:items-center justify-between gap-6">
          <div class="flex items-center gap-5">
            <div class="w-16 h-16 rounded-2xl bg-zinc-950 border border-zinc-800 flex items-center justify-center text-4xl shadow-inner">
              {{ activeInst.icon }}
            </div>
            <div class="space-y-1">
              <div class="flex items-center gap-2.5">
                <h2 class="text-xl font-black text-zinc-100 font-display">
                  {{ activeInst.name }}
                </h2>
                <UiBadge
                  :variant="isInstanceRunning ? 'amber' : 'zinc'"
                  size="xs"
                  :dot="isInstanceRunning"
                >
                  {{ isInstanceRunning ? 'En cours de jeu' : 'Prête' }}
                </UiBadge>
              </div>
              <div class="flex items-center gap-3 text-xs text-zinc-400 font-mono">
                <span>Minecraft {{ activeInst.gameVersion }}</span>
                <span>•</span>
                <span class="text-amber-400 font-medium">{{ activeInst.loaderVersion }}</span>
                <span>•</span>
                <span>{{ activeInst.allocatedRamGb }} Go RAM</span>
              </div>
            </div>
          </div>

          <!-- Launch / Stop Controls -->
          <div class="flex items-center gap-3 w-full md:w-auto">
            <UiButton
              v-if="!isInstanceRunning"
              variant="primary"
              size="lg"
              class="w-full md:w-auto shadow-amber-500/25"
              @click="minecraftStore.launchInstance(activeInst.id)"
            >
              <template #prefix>
                <Play class="w-5 h-5 fill-current" />
              </template>
              JOUER CETTE INSTANCE
            </UiButton>

            <UiButton
              v-else
              variant="danger"
              size="lg"
              class="w-full md:w-auto"
              @click="minecraftStore.stopInstance(activeInst.id)"
            >
              <template #prefix>
                <Square class="w-5 h-5 fill-current" />
              </template>
              ARRÊTER L'INSTANCE
            </UiButton>
          </div>
        </div>

        <!-- Instance Tabs -->
        <UiTabs
          v-model="instanceTab"
          :tabs="instanceTabs"
          variant="underline"
        />

        <!-- Tab contents -->
        <div class="pt-2">
          <InstanceModsTab
            v-if="instanceTab === 'mods'"
            :instance="activeInst"
          />

          <InstanceServersTab
            v-else-if="instanceTab === 'servers'"
            :instance="activeInst"
          />

          <InstanceGeneralTab
            v-else-if="instanceTab === 'general'"
            :instance="activeInst"
          />

          <InstanceLogsTab
            v-else-if="instanceTab === 'logs'"
            :instance="activeInst"
          />
        </div>
      </div>
    </div>

    <!-- 3. VUE B : CATALOGUE MODPACKS (MODRINTH & CURSEFORGE) -->
    <div v-else class="space-y-6">
      <!-- Search & Filter bar -->
      <div class="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4">
        <div class="relative w-full sm:w-96">
          <Search class="absolute left-3.5 top-1/2 -translate-y-1/2 w-4 h-4 text-zinc-500" />
          <input
            v-model="catalogQuery"
            type="text"
            placeholder="Rechercher parmi des millions de modpacks..."
            class="w-full bg-zinc-900/80 text-zinc-100 placeholder-zinc-500 text-xs rounded-xl pl-10 pr-4 py-2.5 border border-zinc-800 focus:border-amber-500/70 focus:ring-2 focus:ring-amber-500/20 outline-none transition-all"
          />
        </div>

        <div class="flex items-center gap-2">
          <button
            type="button"
            @click="selectedCatalogSource = 'all'"
            :class="[
              'px-3 py-1.5 rounded-lg text-xs font-medium border transition-colors',
              selectedCatalogSource === 'all'
                ? 'bg-amber-500/15 border-amber-500 text-amber-300'
                : 'bg-zinc-900/60 border-zinc-800 text-zinc-400'
            ]"
          >
            Tous
          </button>
          <button
            type="button"
            @click="selectedCatalogSource = 'modrinth'"
            :class="[
              'px-3 py-1.5 rounded-lg text-xs font-medium border transition-colors',
              selectedCatalogSource === 'modrinth'
                ? 'bg-emerald-500/15 border-emerald-500 text-emerald-300'
                : 'bg-zinc-900/60 border-zinc-800 text-zinc-400'
            ]"
          >
            Modrinth API
          </button>
          <button
            type="button"
            @click="selectedCatalogSource = 'curseforge'"
            :class="[
              'px-3 py-1.5 rounded-lg text-xs font-medium border transition-colors',
              selectedCatalogSource === 'curseforge'
                ? 'bg-rose-500/15 border-rose-500 text-rose-300'
                : 'bg-zinc-900/60 border-zinc-800 text-zinc-400'
            ]"
          >
            CurseForge API
          </button>
        </div>
      </div>

      <!-- Modpacks Grid -->
      <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-6">
        <ModpackCard
          v-for="pack in filteredCatalog"
          :key="pack.id"
          :modpack="pack"
        />
      </div>
    </div>

    <!-- Wizard Modal -->
    <InstanceWizardModal
      :open="wizardOpen"
      @close="wizardOpen = false"
      @created="(id) => { subNav = 'instances'; minecraftStore.selectInstance(id); }"
    />
  </div>
</template>
