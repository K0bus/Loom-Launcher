<script setup lang="ts">
import { ref, computed, watch, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import {
  Box,
  Plus,
  Play,
  Square,
  Layers,
  Trash2,
  Sliders,
  ArrowLeft,
  ChevronDown,
  Search,
  Sparkles,
  Clock,
  Calendar,
  FolderOpen,
  Compass,
  Globe,
  FileText,
  FileCode
} from 'lucide-vue-next'
import { useMinecraftStore } from '@/stores/minecraft'
import MinecraftHubHero from '@/components/minecraft/MinecraftHubHero.vue'
import InstanceCard from '@/components/minecraft/InstanceCard.vue'
import MinecraftBreadcrumb from '@/components/minecraft/MinecraftBreadcrumb.vue'
import CreateInstanceModal from '@/components/minecraft/CreateInstanceModal.vue'
import ModsTab from '@/components/minecraft/ModsTab.vue'
import CatalogTab from '@/components/minecraft/CatalogTab.vue'
import MapsTab from '@/components/minecraft/MapsTab.vue'
import ServersTab from '@/components/minecraft/ServersTab.vue'
import LogsTab from '@/components/minecraft/LogsTab.vue'
import ConfigTab from '@/components/minecraft/ConfigTab.vue'
import SettingsTab from '@/components/minecraft/SettingsTab.vue'
import MicrosoftAuthModal from '@/components/minecraft/MicrosoftAuthModal.vue'
import DependencyModal from '@/components/minecraft/DependencyModal.vue'
import MinecraftAvatar from '@/components/minecraft/MinecraftAvatar.vue'
import UiButton from '@/components/ui/UiButton.vue'
import UiBadge from '@/components/ui/UiBadge.vue'

const route = useRoute()
const router = useRouter()
const minecraftStore = useMinecraftStore()

const createModalOpen = ref(false)
const activeTab = ref<'mods' | 'catalog' | 'maps' | 'servers' | 'logs' | 'config' | 'settings'>('mods')
const targetCatalogCategory = ref<'mods' | 'resourcepacks' | 'shaderpacks' | 'modpacks'>('mods')
const catalogTabRef = ref<any>(null)
const showDeleteConfirm = ref(false)
const searchQuery = ref('')

const defaultBanner = computed(() => minecraftStore.defaultBannerUrl)

const instances = computed(() => minecraftStore.instances)
const activeInst = computed(() => minecraftStore.activeInstance)
const account = computed(() => minecraftStore.account)
const isRunning = computed(() => activeInst.value?.status === 'running')

const totalContentCount = computed(() => {
  if (!activeInst.value) return 0
  return (
    (activeInst.value.mods?.length || 0) +
    (activeInst.value.resourcePacks?.length || 0) +
    (activeInst.value.shaderPacks?.length || 0)
  )
})

// Determine if we are on the Hub or a specific Instance Detail view
const instanceIdParam = computed(() => (route.params.instanceId as string) || '')
const isDetailView = computed(() => Boolean(instanceIdParam.value))

// Filter instances on the Hub
const filteredInstances = computed(() => {
  const q = searchQuery.value.trim().toLowerCase()
  if (!q) return instances.value
  return instances.value.filter(
    (i) =>
      i.name.toLowerCase().includes(q) ||
      i.gameVersion.toLowerCase().includes(q) ||
      i.loader.toLowerCase().includes(q)
  )
})

// Sync route params with store
watch(
  () => instanceIdParam.value,
  (newId) => {
    if (newId) {
      minecraftStore.selectInstance(newId)
    }
  },
  { immediate: true }
)

function switchInstance(id: string) {
  showDeleteConfirm.value = false
  minecraftStore.selectInstance(id)
  router.push(`/modding/minecraft/${id}`)
}

function handleLaunch() {
  if (!activeInst.value) return
  if (isRunning.value) {
    minecraftStore.stopInstance(activeInst.value.id)
  } else {
    minecraftStore.launchInstance(activeInst.value.id)
  }
}

function openActiveInstanceFolder() {
  if (activeInst.value) {
    minecraftStore.openInstanceFolder(activeInst.value.id)
  }
}

function handleSwitchToCatalog(cat?: 'mods' | 'resourcepacks' | 'shaderpacks') {
  targetCatalogCategory.value = cat || 'mods'
  activeTab.value = 'catalog'
  setTimeout(() => {
    catalogTabRef.value?.setCategory(targetCatalogCategory.value)
  }, 50)
}

async function confirmDeleteCurrentInstance() {
  if (!activeInst.value) return
  const id = activeInst.value.id
  showDeleteConfirm.value = false
  await minecraftStore.deleteInstance(id)
  router.push('/modding/minecraft')
}

function onInstanceCreated(newInstanceId: string) {
  createModalOpen.value = false
  router.push(`/modding/minecraft/${newInstanceId}`)
}

onMounted(() => {
  minecraftStore.init()
})
</script>

<template>
  <div class="min-h-full pb-20 selection:bg-amber-500/30 selection:text-amber-200">
    <!-- ============================================================== -->
    <!-- MODE 1 : MINECRAFT HUB VIEW (Fullscreen Hero + Instances Grid) -->
    <!-- ============================================================== -->
    <div v-if="!isDetailView">
      <!-- 1. Fullscreen Cinematic Hero -->
      <MinecraftHubHero @create-instance="createModalOpen = true" />

      <!-- 2. Instances Grid Section -->
      <div class="max-w-7xl mx-auto px-6 sm:px-10 lg:px-14 py-8 space-y-6">
        <!-- Section Header : Title, Counter, Search & Create Action -->
        <div class="flex flex-col sm:flex-row items-stretch sm:items-center justify-between gap-4">
          <div class="space-y-1">
            <div class="flex items-center gap-3">
              <h2 class="text-xl sm:text-2xl font-black text-white font-display tracking-tight flex items-center gap-2.5">
                <Layers class="w-6 h-6 text-amber-400" />
                Vos Environnements & Instances
              </h2>
              <UiBadge variant="amber" size="sm" class="font-mono">
                {{ instances.length }} INSTANCE{{ instances.length > 1 ? 'S' : '' }}
              </UiBadge>
            </div>
            <p class="text-xs text-zinc-400">
              Chaque instance dispose de son dossier <span class="font-mono text-zinc-300">.minecraft/</span> hermétique et de ses réglages JVM dédiés.
            </p>
          </div>

          <div class="flex items-center gap-3">
            <!-- Search bar -->
            <div class="relative w-full sm:w-64">
              <input
                v-model="searchQuery"
                type="text"
                placeholder="Rechercher une instance..."
                class="w-full bg-zinc-900/80 border border-zinc-800 rounded-2xl pl-9 pr-4 py-2 text-xs text-zinc-100 placeholder-zinc-500 focus:outline-none focus:border-amber-500/80 shadow-inner"
              />
              <Search class="w-3.5 h-3.5 text-zinc-500 absolute left-3 top-1/2 -translate-y-1/2 pointer-events-none" />
            </div>

            <!-- Create CTA Button -->
            <UiButton
              variant="primary"
              size="sm"
              class="shrink-0"
              @click="createModalOpen = true"
            >
              <template #prefix>
                <Plus class="w-4 h-4" />
              </template>
              Nouvelle Instance
            </UiButton>
          </div>
        </div>

        <!-- Instances Grid -->
        <div
          v-if="filteredInstances.length > 0"
          class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6 pt-2"
        >
          <InstanceCard
            v-for="inst in filteredInstances"
            :key="inst.id"
            :instance="inst"
          />
        </div>

        <!-- Empty Search Result -->
        <div
          v-else-if="instances.length > 0"
          class="p-16 text-center rounded-3xl bg-zinc-900/40 border border-zinc-800/80 space-y-3"
        >
          <Search class="w-8 h-8 text-zinc-600 mx-auto" />
          <h4 class="text-sm font-bold text-zinc-200">Aucune instance ne correspond à votre recherche</h4>
          <p class="text-xs text-zinc-500 max-w-sm mx-auto">
            Vérifiez l'orthographe du nom ou réinitialisez le filtre pour voir toutes vos instances.
          </p>
          <UiButton variant="ghost" size="sm" @click="searchQuery = ''">
            Réinitialiser le filtre
          </UiButton>
        </div>

        <!-- Empty State if Zero Instances -->
        <div
          v-else
          class="p-16 text-center rounded-3xl bg-zinc-900/40 border border-zinc-800/80 space-y-4 max-w-lg mx-auto"
        >
          <div class="w-16 h-16 rounded-2xl bg-zinc-800 text-amber-400 flex items-center justify-center text-3xl mx-auto shadow-inner">
            ⛏️
          </div>
          <div class="space-y-1">
            <h3 class="text-base font-bold text-zinc-100">Aucune instance Minecraft</h3>
            <p class="text-xs text-zinc-400">
              Créez votre premier environnement hermétique (Vanilla, Fabric, NeoForge ou Forge) pour commencer.
            </p>
          </div>
          <UiButton
            variant="primary"
            size="md"
            @click="createModalOpen = true"
          >
            <template #prefix>
              <Plus class="w-4 h-4" />
            </template>
            Créer ma première instance
          </UiButton>
        </div>
      </div>
    </div>

    <!-- ============================================================== -->
    <!-- MODE 2 : INSTANCE GLOBAL VIEW (Detail page with dedicated tabs)-->
    <!-- ============================================================== -->
    <div v-else class="space-y-8">
      <!-- 1. CINEMATIC INSTANCE HEADER -->
      <div class="relative w-full min-h-[220px] sm:min-h-[260px] overflow-hidden select-none border-b border-zinc-800/80 bg-zinc-950 flex flex-col justify-between">
        <!-- Background Banner Artwork -->
        <div class="absolute inset-0 z-0">
          <img
            :src="activeInst?.bannerUrl || activeInst?.coverUrl || defaultBanner"
            :alt="activeInst?.name"
            class="w-full h-full object-cover object-[center_35%] scale-105 transition-transform duration-700 ease-out"
          />
          <div class="absolute inset-0 hero-vignette pointer-events-none" />
          <div class="absolute inset-0 bg-gradient-to-t from-zinc-950 via-zinc-950/70 to-zinc-950/40 pointer-events-none" />
        </div>

        <!-- Top Row : Breadcrumb & Quick Actions -->
        <div class="relative z-20 max-w-7xl mx-auto w-full px-6 sm:px-10 lg:px-14 pt-6 flex flex-wrap items-start justify-between gap-4">
          <!-- Breadcrumb : Modding > Minecraft > [Nom Instance] -->
          <MinecraftBreadcrumb
            :items="[
              { label: 'Modding', to: '/modding' },
              { label: 'Minecraft', to: '/modding/minecraft' },
              { label: activeInst?.name || 'Instance', active: true, icon: Box }
            ]"
          />

          <!-- Right: Instance Quick Switcher Dropdown & Return to Hub + Player Profile -->
          <div class="flex flex-col items-end gap-2.5">
            <div class="flex items-center gap-3">
              <button
                type="button"
                @click="router.push('/modding/minecraft')"
                class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-full bg-zinc-900/80 hover:bg-zinc-800 border border-zinc-800 text-xs font-mono text-zinc-400 hover:text-zinc-200 transition backdrop-blur-md"
              >
                <ArrowLeft class="w-3.5 h-3.5 text-amber-400" />
                <span>Toutes les instances</span>
              </button>

              <!-- Quick instance select -->
              <div v-if="instances.length > 1" class="relative">
                <select
                  :value="activeInst?.id"
                  @change="(e) => switchInstance((e.target as HTMLSelectElement).value)"
                  class="bg-zinc-900/80 border border-zinc-800 rounded-full pl-3 pr-8 py-1.5 text-xs font-mono text-zinc-200 appearance-none focus:outline-none focus:border-amber-500/80 cursor-pointer backdrop-blur-md shadow-sm"
                >
                  <option v-for="inst in instances" :key="inst.id" :value="inst.id">
                    {{ inst.name }} ({{ inst.loader }})
                  </option>
                </select>
                <ChevronDown class="w-3.5 h-3.5 text-zinc-500 absolute right-3 top-1/2 -translate-y-1/2 pointer-events-none" />
              </div>
            </div>

            <!-- Microsoft Profile Pill -->
            <div
              v-if="account"
              class="flex items-center gap-2.5 bg-zinc-900/80 border border-zinc-800 px-3 py-1.5 rounded-2xl shadow-inner backdrop-blur-md"
            >
              <MinecraftAvatar
                :uuid="account.uuid"
                :name="account.name"
                :skin-url="account.skin_url"
                :size="24"
              />
              <div class="text-left font-mono pr-1">
                <div class="text-xs font-bold text-zinc-200 leading-none truncate max-w-[120px]">{{ account.name }}</div>
                <div class="text-[9px] text-emerald-400 flex items-center gap-1 mt-0.5">
                  <span class="w-1.5 h-1.5 rounded-full bg-emerald-400" />
                  Xbox Live
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- Bottom Row : Instance Identity & Actions Bar -->
        <div class="relative z-20 max-w-7xl mx-auto w-full px-6 sm:px-10 lg:px-14 pb-6 pt-4 flex flex-col md:flex-row items-start md:items-end justify-between gap-6">
          <div class="flex items-center gap-4">
            <div class="w-14 h-14 rounded-2xl bg-zinc-900/90 border border-zinc-700/80 flex items-center justify-center text-3xl shrink-0 shadow-xl backdrop-blur-md">
              {{ activeInst?.icon || '⛏️' }}
            </div>

            <div class="space-y-1">
              <div class="flex items-center gap-2.5">
                <h1 class="text-2xl sm:text-3xl font-black text-white font-display tracking-tight drop-shadow-md">
                  {{ activeInst?.name }}
                </h1>
                <UiBadge
                  :variant="activeInst?.loader === 'fabric' ? 'emerald' : activeInst?.loader === 'neoforge' ? 'amber' : 'sky'"
                  size="xs"
                  class="font-mono uppercase font-bold"
                >
                  {{ activeInst?.loader }}
                </UiBadge>
                <UiBadge variant="zinc" size="xs" class="font-mono">
                  MC {{ activeInst?.gameVersion }}
                </UiBadge>
              </div>

              <p class="text-xs font-mono text-zinc-400 flex flex-wrap items-center gap-3">
                <span>{{ activeInst?.mods?.length || 0 }} mods</span>
                <span>•</span>
                <span>{{ activeInst?.resourcePacks?.length || 0 }} packs</span>
                <span>•</span>
                <span>{{ activeInst?.shaderPacks?.length || 0 }} shaders</span>
                <span>•</span>
                <span>{{ activeInst?.allocatedRamGb }} Go RAM</span>
                <span>•</span>
                <span class="flex items-center gap-1 text-zinc-300">
                  <Clock class="w-3.5 h-3.5 text-amber-400" />
                  {{ minecraftStore.formatPlaytime(activeInst?.playtimeSeconds) }} de jeu
                </span>
                <span>•</span>
                <span class="flex items-center gap-1 text-zinc-300">
                  <Calendar class="w-3.5 h-3.5 text-sky-400" />
                  {{ minecraftStore.formatLastPlayed(activeInst?.lastPlayed) }}
                </span>
                <span>•</span>
                <span :class="isRunning ? 'text-emerald-400 font-bold' : 'text-zinc-500'">
                  {{ isRunning ? 'En cours d\'exécution' : 'Prête au lancement' }}
                </span>
                <button
                  v-if="isRunning"
                  type="button"
                  @click="minecraftStore.resetInstanceStatus(activeInst!.id)"
                  class="text-[10px] text-amber-400 hover:underline cursor-pointer ml-1"
                  title="Réinitialiser le statut si le jeu a quitté inopinément"
                >
                  (Débloquer)
                </button>
              </p>
            </div>
          </div>

          <!-- Instance CTA Actions -->
          <div class="flex items-center gap-3 shrink-0">
            <!-- Big Golden Launch Button -->
            <button
              v-if="activeInst"
              type="button"
              @click="handleLaunch"
              :class="[
                'flex items-center gap-2.5 px-6 py-2.5 rounded-2xl font-bold text-xs tracking-wide active:scale-95 transition-all duration-200 shadow-xl select-none',
                isRunning
                  ? 'bg-rose-500 hover:bg-rose-600 text-white shadow-rose-500/30 ring-2 ring-rose-400/50 animate-pulse'
                  : 'bg-gradient-to-r from-amber-500 via-amber-400 to-amber-500 text-zinc-950 shadow-amber-500/30 hover:brightness-110'
              ]"
            >
              <Square v-if="isRunning" class="w-4 h-4 fill-current" />
              <Play v-else class="w-4 h-4 fill-current translate-x-0.5" />
              <span class="font-black text-sm">{{ isRunning ? 'Arrêter' : 'Jouer' }}</span>
            </button>

            <!-- Open Folder Button -->
            <button
              v-if="activeInst"
              type="button"
              @click="openActiveInstanceFolder"
              class="flex items-center justify-center p-2.5 rounded-2xl bg-zinc-900/80 hover:bg-zinc-800 border border-zinc-800 hover:border-zinc-700 text-zinc-300 hover:text-amber-400 transition-all shadow-md active:scale-95"
              title="Ouvrir le dossier de l'instance"
            >
              <FolderOpen class="w-4 h-4 text-amber-400" />
            </button>

            <!-- Delete confirmation -->
            <template v-if="!showDeleteConfirm">
              <button
                type="button"
                @click="showDeleteConfirm = true"
                class="flex items-center justify-center text-xs font-mono text-zinc-500 hover:text-rose-400 p-2.5 rounded-2xl bg-zinc-900/80 hover:bg-rose-500/10 border border-zinc-800 hover:border-rose-500/30 transition-colors"
                title="Supprimer cette instance"
              >
                <Trash2 class="w-4 h-4" />
              </button>
            </template>
            <div v-else class="flex items-center gap-2 bg-rose-500/10 border border-rose-500/30 p-1.5 rounded-2xl">
              <span class="text-[11px] font-mono text-rose-300 px-2 font-bold">Confirmer ?</span>
              <button
                type="button"
                @click="confirmDeleteCurrentInstance"
                class="px-2.5 py-1 rounded-xl bg-rose-600 hover:bg-rose-500 text-white text-xs font-bold transition"
              >
                Oui
              </button>
              <button
                type="button"
                @click="showDeleteConfirm = false"
                class="px-2.5 py-1 rounded-xl bg-zinc-800 text-zinc-300 text-xs transition"
              >
                Non
              </button>
            </div>
          </div>
        </div>
      </div>

      <!-- 2. INSTANCE TABS & CONTENT -->
      <div class="max-w-7xl mx-auto px-6 sm:px-10 lg:px-14 space-y-6">
        <!-- Tab Navigation Bar -->
        <div class="flex items-center gap-2 border-b border-zinc-800/80 pb-3">
          <button
            type="button"
            @click="activeTab = 'mods'"
            :class="[
              'px-4 py-2 rounded-2xl text-xs font-mono font-bold transition-all flex items-center gap-2 select-none',
              activeTab === 'mods'
                ? 'bg-amber-500 text-zinc-950 shadow-md shadow-amber-500/10'
                : 'text-zinc-400 hover:text-zinc-200 hover:bg-zinc-900'
            ]"
          >
            <Layers class="w-4 h-4" />
            <span>Mon Modpack</span>
            <span
              v-if="totalContentCount > 0"
              :class="[
                'text-[10px] px-1.5 py-0.5 rounded-full font-bold',
                activeTab === 'mods' ? 'bg-zinc-950 text-amber-400' : 'bg-zinc-800 text-zinc-400'
              ]"
            >
              {{ totalContentCount }}
            </span>
          </button>

          <button
            type="button"
            @click="activeTab = 'catalog'"
            :class="[
              'px-4 py-2 rounded-2xl text-xs font-mono font-bold transition-all flex items-center gap-2 select-none',
              activeTab === 'catalog'
                ? 'bg-amber-500 text-zinc-950 shadow-md shadow-amber-500/10'
                : 'text-zinc-400 hover:text-zinc-200 hover:bg-zinc-900'
            ]"
          >
            <Sparkles class="w-4 h-4" />
            <span>Ajouter du contenu</span>
          </button>

          <button
            type="button"
            @click="activeTab = 'maps'"
            :class="[
              'px-4 py-2 rounded-2xl text-xs font-mono font-bold transition-all flex items-center gap-2 select-none',
              activeTab === 'maps'
                ? 'bg-amber-500 text-zinc-950 shadow-md shadow-amber-500/10'
                : 'text-zinc-400 hover:text-zinc-200 hover:bg-zinc-900'
            ]"
          >
            <Compass class="w-4 h-4" />
            <span>Mondes</span>
          </button>

          <button
            type="button"
            @click="activeTab = 'servers'"
            :class="[
              'px-4 py-2 rounded-2xl text-xs font-mono font-bold transition-all flex items-center gap-2 select-none',
              activeTab === 'servers'
                ? 'bg-amber-500 text-zinc-950 shadow-md shadow-amber-500/10'
                : 'text-zinc-400 hover:text-zinc-200 hover:bg-zinc-900'
            ]"
          >
            <Globe class="w-4 h-4" />
            <span>Serveurs</span>
          </button>

          <button
            type="button"
            @click="activeTab = 'logs'"
            :class="[
              'px-4 py-2 rounded-2xl text-xs font-mono font-bold transition-all flex items-center gap-2 select-none',
              activeTab === 'logs'
                ? 'bg-amber-500 text-zinc-950 shadow-md shadow-amber-500/10'
                : 'text-zinc-400 hover:text-zinc-200 hover:bg-zinc-900'
            ]"
          >
            <FileText class="w-4 h-4" />
            <span>Logs & Crashs</span>
            <span v-if="isRunning" class="w-2 h-2 rounded-full bg-emerald-400 animate-ping" />
          </button>

          <button
            type="button"
            @click="activeTab = 'config'"
            :class="[
              'px-4 py-2 rounded-2xl text-xs font-mono font-bold transition-all flex items-center gap-2 select-none',
              activeTab === 'config'
                ? 'bg-amber-500 text-zinc-950 shadow-md shadow-amber-500/10'
                : 'text-zinc-400 hover:text-zinc-200 hover:bg-zinc-900'
            ]"
          >
            <FileCode class="w-4 h-4" />
            <span>Configuration</span>
          </button>

          <button
            type="button"
            @click="activeTab = 'settings'"
            :class="[
              'px-4 py-2 rounded-2xl text-xs font-mono font-bold transition-all flex items-center gap-2 select-none',
              activeTab === 'settings'
                ? 'bg-amber-500 text-zinc-950 shadow-md shadow-amber-500/10'
                : 'text-zinc-400 hover:text-zinc-200 hover:bg-zinc-900'
            ]"
          >
            <Sliders class="w-4 h-4" />
            <span>Paramètres</span>
          </button>
        </div>

        <!-- Tab Contents -->
        <div v-if="activeInst">
          <ModsTab v-if="activeTab === 'mods'" @switch-to-catalog="handleSwitchToCatalog" />
          <CatalogTab v-else-if="activeTab === 'catalog'" ref="catalogTabRef" :initialCategory="targetCatalogCategory" />
          <MapsTab v-else-if="activeTab === 'maps'" />
          <ServersTab v-else-if="activeTab === 'servers'" />
          <LogsTab v-else-if="activeTab === 'logs'" />
          <ConfigTab v-else-if="activeTab === 'config'" />
          <SettingsTab v-else-if="activeTab === 'settings'" />
        </div>
      </div>
    </div>

    <!-- Modals -->
    <CreateInstanceModal
      :open="createModalOpen"
      @close="createModalOpen = false"
      @created="onInstanceCreated"
    />
    <MicrosoftAuthModal />
    <DependencyModal />
  </div>
</template>
