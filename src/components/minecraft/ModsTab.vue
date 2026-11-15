<script setup lang="ts">
import { ref, computed } from 'vue'
import {
  UploadCloud,
  Search,
  Trash2,
  Loader2,
  RefreshCw,
  ArrowUpCircle,
  FolderOpen,
  Package,
  Palette,
  Sparkles,
  Plus
} from 'lucide-vue-next'
import { useMinecraftStore } from '@/stores/minecraft'
import { useDownloadsStore } from '@/stores/downloadsStore'
import type { MinecraftMod } from '@/models/minecraft'
import UiBadge from '@/components/ui/UiBadge.vue'
import UiButton from '@/components/ui/UiButton.vue'
import ModDetailsModal from '@/components/minecraft/ModDetailsModal.vue'

const emit = defineEmits<{
  (e: 'switch-to-catalog', category?: 'mods' | 'resourcepacks' | 'shaderpacks'): void
}>()

const minecraftStore = useMinecraftStore()
const downloadsStore = useDownloadsStore()
const activeInst = computed(() => minecraftStore.activeInstance)

// Active Section inside "Mon Modpack"
const activeSection = ref<'mods' | 'resourcepacks' | 'shaderpacks'>('mods')

const isDragging = ref(false)
const searchQuery = ref('')
const fileInputRef = ref<HTMLInputElement | null>(null)

// Modal Details State
const detailsModalOpen = ref(false)
const selectedModProvider = ref('curseforge')
const selectedModId = ref('')
const selectedModVersion = ref('')
const selectedContentType = ref<'mod' | 'resourcepack' | 'shader'>('mod')

const isCheckingUpdates = computed(() => minecraftStore.isCheckingUpdates)
const updatingModIds = computed(() => minecraftStore.updatingModIds)
const updatesAvailableCount = computed(() => activeInst.value?.mods?.filter((m) => m.updateAvailable).length || 0)

const activeModDownloads = computed(() => {
  if (!activeInst.value) return []
  return downloadsStore.activeModDownloads.filter((d) => d.instanceId === activeInst.value?.id)
})

function formatBytes(bytes: number): string {
  if (!bytes || bytes <= 0) return '0 Mo'
  const mb = bytes / (1024 * 1024)
  if (mb < 0.1) {
    const kb = bytes / 1024
    return `${kb.toFixed(0)} Ko`
  }
  return `${mb.toFixed(1)} Mo`
}

function formatModRate(bytesPerSec: number): string {
  if (!bytesPerSec || bytesPerSec <= 0) return ''
  const mb = bytesPerSec / (1024 * 1024)
  if (mb >= 1) return `${mb.toFixed(1)} Mo/s`
  const kb = bytesPerSec / 1024
  return `${kb.toFixed(0)} Ko/s`
}

// Current items according to selected section
const currentItems = computed<MinecraftMod[]>(() => {
  if (!activeInst.value) return []
  if (activeSection.value === 'resourcepacks') {
    return activeInst.value.resourcePacks || []
  }
  if (activeSection.value === 'shaderpacks') {
    return activeInst.value.shaderPacks || []
  }
  return activeInst.value.mods || []
})

const filteredItems = computed<MinecraftMod[]>(() => {
  const list = currentItems.value
  const q = searchQuery.value.trim().toLowerCase()
  if (!q) return list
  return list.filter(
    (m) =>
      m.name.toLowerCase().includes(q) ||
      m.fileName.toLowerCase().includes(q) ||
      (m.author && m.author.toLowerCase().includes(q)) ||
      (m.provider && m.provider.toLowerCase().includes(q))
  )
})

const activeCount = computed(() => {
  return currentItems.value.filter((m) => m.enabled).length
})

function onDragOver(e: DragEvent) {
  e.preventDefault()
  isDragging.value = true
}

function onDragLeave(e: DragEvent) {
  e.preventDefault()
  isDragging.value = false
}

async function handleImportFiles(files: File[]) {
  if (!activeInst.value) return
  for (const file of files) {
    if (activeSection.value === 'resourcepacks') {
      if (file.name.endsWith('.zip')) {
        await minecraftStore.importLocalResourcePack(activeInst.value.id, file)
      }
    } else if (activeSection.value === 'shaderpacks') {
      if (file.name.endsWith('.zip')) {
        await minecraftStore.importLocalShaderPack(activeInst.value.id, file)
      }
    } else {
      if (file.name.endsWith('.jar') || file.name.endsWith('.zip')) {
        await minecraftStore.importLocalJar(activeInst.value.id, file)
      }
    }
  }
}

async function onDrop(e: DragEvent) {
  e.preventDefault()
  isDragging.value = false
  if (!e.dataTransfer?.files || e.dataTransfer.files.length === 0) return
  await handleImportFiles(Array.from(e.dataTransfer.files))
}

function triggerBrowse() {
  fileInputRef.value?.click()
}

async function onFileInputChange(e: Event) {
  const target = e.target as HTMLInputElement
  if (!target.files || !activeInst.value) return
  await handleImportFiles(Array.from(target.files))
  target.value = ''
}

function openModDetails(mod: MinecraftMod) {
  selectedModProvider.value = mod.provider || 'curseforge'
  selectedModId.value = mod.remoteModId || mod.id
  selectedModVersion.value = mod.version
  selectedContentType.value = activeSection.value === 'resourcepacks' ? 'resourcepack' : activeSection.value === 'shaderpacks' ? 'shader' : 'mod'
  detailsModalOpen.value = true
}

async function checkUpdates() {
  if (!activeInst.value) return
  await minecraftStore.checkModUpdates(activeInst.value.id)
}

function openCurrentFolder() {
  if (!activeInst.value) return
  const sub = activeSection.value === 'resourcepacks' ? 'resourcepacks' : activeSection.value === 'shaderpacks' ? 'shaderpacks' : 'mods'
  minecraftStore.openInstanceFolder(activeInst.value.id, sub)
}

async function quickUpdateMod(mod: MinecraftMod) {
  if (!activeInst.value) return
  await minecraftStore.updateMod(activeInst.value.id, mod.id, mod.newFileId, mod.newDownloadUrl)
}

function onContentUpdated() {
  if (!activeInst.value) return
  if (activeSection.value === 'resourcepacks') {
    minecraftStore.refreshInstanceResourcePacks(activeInst.value.id)
  } else if (activeSection.value === 'shaderpacks') {
    minecraftStore.refreshInstanceShaderPacks(activeInst.value.id)
  } else {
    minecraftStore.refreshInstanceMods(activeInst.value.id)
  }
}

function toggleItem(item: MinecraftMod) {
  if (!activeInst.value) return
  if (activeSection.value === 'resourcepacks') {
    minecraftStore.toggleResourcePack(activeInst.value.id, item.id, !item.enabled)
  } else if (activeSection.value === 'shaderpacks') {
    minecraftStore.toggleShaderPack(activeInst.value.id, item.id, !item.enabled)
  } else {
    minecraftStore.toggleMod(activeInst.value.id, item.id)
  }
}

function deleteItem(item: MinecraftMod) {
  if (!activeInst.value) return
  const label = activeSection.value === 'resourcepacks' ? 'le pack de textures' : activeSection.value === 'shaderpacks' ? 'le shader' : 'le mod'
  if (confirm(`Supprimer définitivement ${label} "${item.name}" de cette instance ?`)) {
    if (activeSection.value === 'resourcepacks') {
      minecraftStore.deleteResourcePack(activeInst.value.id, item.id)
    } else if (activeSection.value === 'shaderpacks') {
      minecraftStore.deleteShaderPack(activeInst.value.id, item.id)
    } else {
      minecraftStore.deleteMod(activeInst.value.id, item.id)
    }
  }
}
</script>

<template>
  <div class="space-y-6">
    <!-- 0. SUB-SECTION PILLS : Mods | Resourcepacks | Shaderpacks -->
    <div class="flex flex-wrap items-center justify-between gap-4 p-2 bg-zinc-950/80 rounded-3xl border border-zinc-800/80 backdrop-blur-md">
      <div class="flex items-center gap-2">
        <!-- Mods Pill -->
        <button
          type="button"
          @click="activeSection = 'mods'"
          :class="[
            'px-4 py-2 rounded-2xl text-xs font-mono font-bold transition-all flex items-center gap-2 select-none shadow-sm',
            activeSection === 'mods'
              ? 'bg-amber-500 text-zinc-950 shadow-amber-500/10'
              : 'text-zinc-400 hover:text-zinc-200 hover:bg-zinc-900/80'
          ]"
        >
          <Package class="w-4 h-4" />
          <span>Mods</span>
          <span
            :class="[
              'text-[10px] px-1.5 py-0.2 rounded-full font-bold',
              activeSection === 'mods' ? 'bg-zinc-950 text-amber-400' : 'bg-zinc-800 text-zinc-400'
            ]"
          >
            {{ activeInst?.mods?.length || 0 }}
          </span>
        </button>

        <!-- Resourcepacks Pill -->
        <button
          type="button"
          @click="activeSection = 'resourcepacks'"
          :class="[
            'px-4 py-2 rounded-2xl text-xs font-mono font-bold transition-all flex items-center gap-2 select-none shadow-sm',
            activeSection === 'resourcepacks'
              ? 'bg-amber-500 text-zinc-950 shadow-amber-500/10'
              : 'text-zinc-400 hover:text-zinc-200 hover:bg-zinc-900/80'
          ]"
        >
          <Palette class="w-4 h-4" />
          <span>Packs de ressources</span>
          <span
            :class="[
              'text-[10px] px-1.5 py-0.2 rounded-full font-bold',
              activeSection === 'resourcepacks' ? 'bg-zinc-950 text-amber-400' : 'bg-zinc-800 text-zinc-400'
            ]"
          >
            {{ activeInst?.resourcePacks?.length || 0 }}
          </span>
        </button>

        <!-- Shaders Pill -->
        <button
          type="button"
          @click="activeSection = 'shaderpacks'"
          :class="[
            'px-4 py-2 rounded-2xl text-xs font-mono font-bold transition-all flex items-center gap-2 select-none shadow-sm',
            activeSection === 'shaderpacks'
              ? 'bg-amber-500 text-zinc-950 shadow-amber-500/10'
              : 'text-zinc-400 hover:text-zinc-200 hover:bg-zinc-900/80'
          ]"
        >
          <Sparkles class="w-4 h-4" />
          <span>Shaders</span>
          <span
            :class="[
              'text-[10px] px-1.5 py-0.2 rounded-full font-bold',
              activeSection === 'shaderpacks' ? 'bg-zinc-950 text-amber-400' : 'bg-zinc-800 text-zinc-400'
            ]"
          >
            {{ activeInst?.shaderPacks?.length || 0 }}
          </span>
        </button>
      </div>

      <!-- Quick Add Content CTA to jump to catalog -->
      <UiButton
        variant="primary"
        size="sm"
        @click="emit('switch-to-catalog', activeSection)"
        class="shadow-md"
      >
        <template #prefix>
          <Plus class="w-3.5 h-3.5" />
        </template>
        <span>
          Ajouter {{ activeSection === 'resourcepacks' ? 'des packs' : activeSection === 'shaderpacks' ? 'des shaders' : 'des mods' }}
        </span>
      </UiButton>
    </div>

    <!-- 1. DRAG & DROP UNIVERSAL ZONE -->
    <div
      @dragover="onDragOver"
      @dragleave="onDragLeave"
      @drop="onDrop"
      @click="triggerBrowse"
      :class="[
        'border-2 border-dashed rounded-3xl p-7 text-center transition-all cursor-pointer relative overflow-hidden select-none',
        isDragging
          ? 'border-amber-400 bg-amber-500/10 scale-[1.01]'
          : 'border-zinc-800 bg-zinc-900/40 hover:border-zinc-700 hover:bg-zinc-900/60'
      ]"
    >
      <input
        ref="fileInputRef"
        type="file"
        multiple
        :accept="activeSection === 'mods' ? '.jar,.zip' : '.zip'"
        class="hidden"
        @change="onFileInputChange"
      />

      <div class="flex flex-col items-center justify-center gap-2.5">
        <div
          class="w-12 h-12 rounded-2xl flex items-center justify-center transition-transform"
          :class="isDragging ? 'bg-amber-500 text-zinc-950 scale-110' : 'bg-zinc-800 text-amber-400'"
        >
          <UploadCloud class="w-6 h-6" />
        </div>

        <div class="space-y-1">
          <p class="text-sm font-bold text-zinc-100">
            <template v-if="activeSection === 'resourcepacks'">
              Glissez vos packs de ressources <span class="font-mono text-amber-400">.zip</span> ou cliquez pour parcourir
            </template>
            <template v-else-if="activeSection === 'shaderpacks'">
              Glissez vos shaderpacks <span class="font-mono text-amber-400">.zip</span> ou cliquez pour parcourir
            </template>
            <template v-else>
              Glissez vos mods <span class="font-mono text-amber-400">.jar</span> ou cliquez pour parcourir
            </template>
          </p>
          <p class="text-xs text-zinc-400">
            Déploiement direct dans <span class="font-mono text-zinc-300">.minecraft/{{ activeSection === 'resourcepacks' ? 'resourcepacks' : activeSection === 'shaderpacks' ? 'shaderpacks' : 'mods' }}/</span>
          </p>
        </div>
      </div>
    </div>

    <!-- 2. TOOLBAR : Search & Counters -->
    <div class="flex flex-col sm:flex-row items-stretch sm:items-center justify-between gap-4 bg-zinc-900/60 p-4 rounded-2xl border border-zinc-800/80">
      <div class="flex flex-wrap items-center gap-3">
        <span class="text-xs font-mono font-bold text-zinc-300 uppercase tracking-wider">
          {{ activeSection === 'resourcepacks' ? 'Packs de ressources' : activeSection === 'shaderpacks' ? 'Shaders' : 'Mods installés' }}
        </span>
        <UiBadge variant="amber" size="xs" class="font-mono">
          {{ activeCount }} / {{ currentItems.length }} ACTIFS
        </UiBadge>
        <UiBadge
          v-if="activeSection === 'mods' && updatesAvailableCount > 0"
          variant="amber"
          size="xs"
          class="font-mono animate-pulse flex items-center gap-1"
        >
          <ArrowUpCircle class="w-3 h-3 text-amber-400" />
          {{ updatesAvailableCount }} MAJ
        </UiBadge>
      </div>

      <div class="flex items-center gap-2.5">
        <!-- Check Updates Button (only for mods) -->
        <UiButton
          v-if="activeSection === 'mods'"
          variant="outline"
          size="sm"
          @click="checkUpdates"
          :loading="isCheckingUpdates"
          title="Vérifier la disponibilité de mises à jour pour tous les mods installés"
        >
          <template #prefix>
            <RefreshCw :class="['w-3.5 h-3.5 text-amber-400', isCheckingUpdates ? 'animate-spin' : '']" />
          </template>
          Vérifier les MAJ
        </UiButton>

        <!-- Open Folder Button -->
        <UiButton
          variant="outline"
          size="sm"
          @click="openCurrentFolder"
          :title="`Ouvrir le dossier ${activeSection} de cette instance`"
        >
          <template #prefix>
            <FolderOpen class="w-3.5 h-3.5 text-amber-400" />
          </template>
          Dossier {{ activeSection === 'resourcepacks' ? 'Packs' : activeSection === 'shaderpacks' ? 'Shaders' : 'Mods' }}
        </UiButton>

        <!-- Quick search filter -->
        <div class="relative w-full sm:w-64">
          <input
            v-model="searchQuery"
            type="text"
            :placeholder="`Filtrer les ${activeSection === 'resourcepacks' ? 'packs' : activeSection === 'shaderpacks' ? 'shaders' : 'mods'}...`"
            class="w-full bg-zinc-950 border border-zinc-800 rounded-xl pl-9 pr-4 py-1.5 text-xs text-zinc-100 placeholder-zinc-500 focus:outline-none focus:border-amber-500/80 transition-colors font-mono"
          />
          <Search class="w-3.5 h-3.5 text-zinc-500 absolute left-3 top-1/2 -translate-y-1/2 pointer-events-none" />
        </div>
      </div>
    </div>

    <!-- 3. ITEMS LIST -->
    <div v-if="filteredItems.length > 0 || (activeSection === 'mods' && activeModDownloads.length > 0)" class="space-y-2.5">
      <!-- Active Downloads in Progress (only displayed on mods or all) -->
      <div
        v-if="activeSection === 'mods'"
        v-for="dl in activeModDownloads"
        :key="dl.id"
        class="relative overflow-hidden p-4 rounded-2xl border border-amber-500/50 bg-zinc-950/90 shadow-lg shadow-amber-500/5 select-none transition-all group"
      >
        <!-- The Global Card IS the Progress Bar -->
        <div
          class="absolute inset-y-0 left-0 bg-gradient-to-r from-amber-500/25 via-amber-400/20 to-amber-500/35 border-r-2 border-amber-400 transition-all duration-300 pointer-events-none"
          :style="{ width: `${Math.max(dl.progressPercent, 4)}%` }"
        >
          <div class="absolute right-0 top-0 bottom-0 w-3 bg-amber-400 shadow-[0_0_16px_rgba(251,191,36,0.9)] animate-pulse" />
        </div>

        <div class="relative z-10 flex items-center justify-between gap-4">
          <div class="flex items-center gap-3.5 min-w-0">
            <div class="w-10 h-10 rounded-xl bg-zinc-900 border border-amber-500/40 overflow-hidden flex items-center justify-center shrink-0 shadow-inner">
              <img
                v-if="dl.iconUrl"
                :src="dl.iconUrl"
                :alt="dl.name"
                class="w-full h-full object-cover"
              />
              <Loader2 v-else class="w-5 h-5 text-amber-400 animate-spin" />
            </div>

            <div class="min-w-0">
              <div class="flex items-center gap-2">
                <span class="text-sm font-bold text-zinc-100 truncate">
                  {{ dl.name }}
                </span>
                <UiBadge variant="amber" size="xs" class="font-mono text-[9px] uppercase animate-pulse">
                  {{ dl.status === 'installing' ? 'FINALISATION' : 'TÉLÉCHARGEMENT' }}
                </UiBadge>
                <UiBadge
                  :variant="dl.provider === 'modrinth' ? 'emerald' : 'amber'"
                  size="xs"
                  class="font-mono text-[9px] uppercase"
                >
                  {{ dl.provider }}
                </UiBadge>
              </div>
              <p class="text-[11px] font-mono text-zinc-400 truncate mt-0.5">
                {{ dl.fileName || 'Préparation du fichier...' }}
              </p>
            </div>
          </div>

          <div class="flex items-center gap-4 shrink-0 font-mono text-xs text-right">
            <div class="hidden sm:block text-zinc-400 text-[11px]">
              <div>{{ formatBytes(dl.bytesDownloaded) }} / {{ formatBytes(dl.bytesTotal) }}</div>
              <div v-if="dl.downloadRateBytesPerSec > 0" class="text-amber-400/90 text-[10px]">
                ⚡ {{ formatModRate(dl.downloadRateBytesPerSec) }}
              </div>
            </div>
            <div class="text-sm font-black text-amber-400 flex items-center gap-1.5 min-w-[55px] justify-end">
              <Loader2 class="w-3.5 h-3.5 animate-spin" />
              <span>{{ dl.progressPercent }}%</span>
            </div>
          </div>
        </div>
      </div>

      <!-- Installed Items Cards -->
      <div
        v-for="item in filteredItems"
        :key="item.id"
        @click="openModDetails(item)"
        :class="[
          'flex items-center justify-between p-3.5 sm:p-4 rounded-2xl border transition-all gap-4 cursor-pointer group',
          item.enabled
            ? 'bg-zinc-900/60 border-zinc-800 hover:border-zinc-700 hover:bg-zinc-900/90'
            : 'bg-zinc-950/40 border-zinc-900 opacity-60 hover:opacity-80'
        ]"
      >
        <div class="flex items-center gap-3.5 min-w-0 flex-1">
          <!-- Item Provider Icon / Fallback -->
          <div class="w-11 h-11 rounded-xl bg-zinc-950 border border-zinc-800 flex items-center justify-center shrink-0 overflow-hidden shadow-inner p-0.5">
            <img
              v-if="item.iconUrl"
              :src="item.iconUrl"
              :alt="item.name"
              class="w-full h-full object-cover rounded-lg group-hover:scale-105 transition-transform"
              loading="lazy"
            />
            <Palette v-else-if="activeSection === 'resourcepacks'" class="w-5 h-5 text-amber-400" />
            <Sparkles v-else-if="activeSection === 'shaderpacks'" class="w-5 h-5 text-sky-400" />
            <Package v-else class="w-5 h-5 text-zinc-600" />
          </div>

          <div class="min-w-0 flex-1">
            <div class="flex flex-wrap items-center gap-2">
              <span
                :class="[
                  'text-sm font-bold truncate group-hover:text-amber-400 transition-colors',
                  item.enabled ? 'text-zinc-100' : 'text-zinc-500 line-through'
                ]"
                :title="item.name"
              >
                {{ item.name }}
              </span>

              <span v-if="item.version" class="text-xs text-zinc-400 font-mono">v{{ item.version }}</span>

              <UiBadge
                :variant="item.provider === 'modrinth' ? 'emerald' : item.provider === 'curseforge' ? 'amber' : 'zinc'"
                size="xs"
                class="font-mono text-[9px] uppercase"
              >
                {{ item.provider || 'local' }}
              </UiBadge>

              <UiBadge
                v-if="item.updateAvailable"
                variant="amber"
                size="xs"
                class="font-mono text-[9px] animate-pulse flex items-center gap-1"
              >
                <ArrowUpCircle class="w-3 h-3 text-amber-400" />
                MAJ {{ item.newVersion }}
              </UiBadge>
            </div>

            <div class="flex flex-wrap items-center gap-2 text-[11px] font-mono text-zinc-500 truncate mt-0.5">
              <span v-if="item.author" class="text-zinc-400">par {{ item.author }}</span>
              <span v-if="item.author">•</span>
              <span class="truncate max-w-[200px] sm:max-w-xs">{{ item.fileName }}</span>
              <span v-if="item.fileSizeMb">•</span>
              <span v-if="item.fileSizeMb">{{ item.fileSizeMb }} Mo</span>
            </div>
          </div>
        </div>

        <!-- Controls: Quick Update, Toggle & Delete -->
        <div class="flex items-center gap-2.5 shrink-0" @click.stop>
          <!-- Quick Update Button -->
          <button
            v-if="item.updateAvailable"
            type="button"
            @click.stop="quickUpdateMod(item)"
            :disabled="updatingModIds[item.id]"
            class="px-2.5 py-1 rounded-xl bg-amber-500/20 hover:bg-amber-500 border border-amber-500/40 text-amber-300 hover:text-zinc-950 font-mono text-xs font-bold transition-all flex items-center gap-1 shadow-sm disabled:opacity-50"
            title="Installer directement la mise à jour disponible"
          >
            <Loader2 v-if="updatingModIds[item.id]" class="w-3.5 h-3.5 animate-spin" />
            <ArrowUpCircle v-else class="w-3.5 h-3.5" />
            <span class="hidden sm:inline">MAJ</span>
          </button>

          <!-- Toggle switch -->
          <button
            type="button"
            @click.stop="toggleItem(item)"
            :class="[
              'relative inline-flex h-6 w-11 items-center rounded-full transition-colors focus:outline-none select-none',
              item.enabled ? 'bg-amber-500' : 'bg-zinc-800'
            ]"
            :title="item.enabled ? 'Désactiver' : 'Activer'"
          >
            <span
              :class="[
                'inline-block h-4 w-4 transform rounded-full bg-zinc-950 transition-transform',
                item.enabled ? 'translate-x-6' : 'translate-x-1'
              ]"
            />
          </button>

          <!-- Delete -->
          <button
            type="button"
            @click.stop="deleteItem(item)"
            class="p-2 rounded-xl text-zinc-500 hover:text-rose-400 hover:bg-rose-500/10 transition-colors"
            title="Supprimer définitivement"
          >
            <Trash2 class="w-4 h-4" />
          </button>
        </div>
      </div>
    </div>

    <!-- Empty State -->
    <div
      v-else-if="filteredItems.length === 0 && (activeSection !== 'mods' || activeModDownloads.length === 0)"
      class="p-12 text-center rounded-3xl bg-zinc-900/30 border border-zinc-800/80 space-y-4"
    >
      <div class="w-14 h-14 mx-auto rounded-2xl bg-zinc-800/80 flex items-center justify-center text-amber-400">
        <Palette v-if="activeSection === 'resourcepacks'" class="w-7 h-7" />
        <Sparkles v-else-if="activeSection === 'shaderpacks'" class="w-7 h-7" />
        <Package v-else class="w-7 h-7" />
      </div>
      <div class="space-y-1">
        <h4 class="text-sm font-bold text-zinc-200">
          Aucun {{ activeSection === 'resourcepacks' ? 'pack de ressources' : activeSection === 'shaderpacks' ? 'shaderpack' : 'mod' }} dans cette instance
        </h4>
        <p class="text-xs text-zinc-500 max-w-sm mx-auto">
          Glissez-déposez un fichier <span class="font-mono text-amber-400">{{ activeSection === 'mods' ? '.jar' : '.zip' }}</span> ci-dessus ou ajoutez du contenu directement depuis le catalogue.
        </p>
      </div>
      <UiButton
        variant="primary"
        size="sm"
        @click="emit('switch-to-catalog', activeSection)"
      >
        <template #prefix>
          <Plus class="w-3.5 h-3.5" />
        </template>
        Parcourir le catalogue de {{ activeSection === 'resourcepacks' ? 'packs' : activeSection === 'shaderpacks' ? 'shaders' : 'mods' }}
      </UiButton>
    </div>

    <!-- Full Mod / Pack Details Modal -->
    <ModDetailsModal
      v-model="detailsModalOpen"
      :provider="selectedModProvider"
      :modId="selectedModId"
      :installedVersion="selectedModVersion"
      :instanceId="activeInst?.id"
      :contentType="selectedContentType"
      @updated="onContentUpdated"
      @installed="onContentUpdated"
    />
  </div>
</template>
