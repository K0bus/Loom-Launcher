<script setup lang="ts">
import { ref, computed } from 'vue'
import {
  Search,
  FolderOpen,
  ArrowUpCircle,
  UploadCloud,
  Trash2,
  ChevronUp,
  ChevronDown,
  RefreshCw,
  Package,
  Loader2
} from 'lucide-vue-next'
import type { MinecraftInstance, MinecraftMod } from '@/models/minecraft'
import { useMinecraftStore } from '@/stores/minecraftStore'
import UiInput from '@/components/ui/UiInput.vue'
import UiBadge from '@/components/ui/UiBadge.vue'
import UiButton from '@/components/ui/UiButton.vue'
import ModDetailsModal from '@/components/minecraft/ModDetailsModal.vue'

interface Props {
  instance: MinecraftInstance
}

const props = defineProps<Props>()

const minecraftStore = useMinecraftStore()
const searchMod = ref('')
const isDraggingOver = ref(false)
const fileInputRef = ref<HTMLInputElement | null>(null)

// Modal state
const detailsModalOpen = ref(false)
const selectedModProvider = ref('curseforge')
const selectedModId = ref('')
const selectedModVersion = ref('')

const isCheckingUpdates = computed(() => minecraftStore.isCheckingUpdates)
const updatingModIds = computed(() => minecraftStore.updatingModIds)
const updatesAvailableCount = computed(() => props.instance.mods.filter((m) => m.updateAvailable).length)

function openModDetails(mod: MinecraftMod) {
  selectedModProvider.value = mod.provider || 'curseforge'
  selectedModId.value = mod.remoteModId || mod.id
  selectedModVersion.value = mod.version
  detailsModalOpen.value = true
}

async function checkUpdates() {
  await minecraftStore.checkModUpdates(props.instance.id)
}

async function quickUpdate(mod: MinecraftMod) {
  await minecraftStore.updateMod(props.instance.id, mod.id, mod.newFileId, mod.newDownloadUrl)
}

function onModUpdated() {
  minecraftStore.refreshInstanceMods(props.instance.id)
}

const filteredMods = computed(() => {
  if (!searchMod.value.trim()) return props.instance.mods
  const q = searchMod.value.toLowerCase()
  return props.instance.mods.filter((m) =>
    m.name.toLowerCase().includes(q) ||
    m.author.toLowerCase().includes(q) ||
    m.fileName.toLowerCase().includes(q)
  )
})

function handleDragOver(e: DragEvent) {
  e.preventDefault()
  isDraggingOver.value = true
}

function handleDragLeave(e: DragEvent) {
  e.preventDefault()
  isDraggingOver.value = false
}

function handleDrop(e: DragEvent) {
  e.preventDefault()
  isDraggingOver.value = false

  if (e.dataTransfer && e.dataTransfer.files) {
    for (let i = 0; i < e.dataTransfer.files.length; i++) {
      const file = e.dataTransfer.files[i]
      if (
        file.name.endsWith('.jar') ||
        file.name.endsWith('.zip') ||
        file.name.endsWith('.mrpack')
      ) {
        minecraftStore.importLocalFile(props.instance.id, file)
      }
    }
  }
}

function triggerFileInput() {
  fileInputRef.value?.click()
}

function onFileInputChange(e: Event) {
  const target = e.target as HTMLInputElement
  if (target.files) {
    for (let i = 0; i < target.files.length; i++) {
      const file = target.files[i]
      minecraftStore.importLocalFile(props.instance.id, file)
    }
  }
}

function moveUp(index: number) {
  if (index > 0) {
    minecraftStore.reorderMod(props.instance.id, index, index - 1)
  }
}

function moveDown(index: number) {
  if (index < props.instance.mods.length - 1) {
    minecraftStore.reorderMod(props.instance.id, index, index + 1)
  }
}

function confirmDelete(modId: string, modName: string) {
  if (confirm(`Voulez-vous vraiment désinstaller le mod "${modName}" ?`)) {
    minecraftStore.deleteMod(props.instance.id, modId)
  }
}
</script>

<template>
  <div class="space-y-5">
    <!-- Universal Drag & Drop Zone -->
    <div
      @dragover="handleDragOver"
      @dragleave="handleDragLeave"
      @drop="handleDrop"
      @click="triggerFileInput"
      :class="[
        'border-2 border-dashed rounded-3xl p-6 text-center cursor-pointer transition-all duration-200 select-none flex flex-col items-center justify-center gap-2 group',
        isDraggingOver
          ? 'border-amber-500 bg-amber-500/10 scale-[1.01]'
          : 'border-zinc-800 bg-zinc-950/40 hover:border-zinc-700 hover:bg-zinc-900/40'
      ]"
    >
      <input
        ref="fileInputRef"
        type="file"
        multiple
        accept=".jar,.zip,.mrpack"
        class="hidden"
        @change="onFileInputChange"
      />

      <div class="w-12 h-12 rounded-2xl bg-zinc-900 border border-zinc-800 flex items-center justify-center text-amber-400 group-hover:scale-110 transition-transform shadow-inner">
        <UploadCloud class="w-6 h-6" />
      </div>

      <div>
        <div class="font-bold text-sm text-zinc-200">
          Glissez-déposez vos fichiers <span class="text-amber-400">.jar</span>, <span class="text-amber-400">.zip</span> ou modpacks <span class="text-amber-400">.mrpack</span>
        </div>
        <p class="text-xs text-zinc-500 mt-1">
          Installation atomique et déploiement par liens symboliques (symlinks) sans duplication de disque
        </p>
      </div>
    </div>

    <!-- Toolbar -->
    <div class="flex flex-col sm:flex-row items-stretch sm:items-center justify-between gap-4">
      <div class="max-w-xs w-full">
        <UiInput
          v-model="searchMod"
          placeholder="Filtrer les mods installés..."
        >
          <template #prefix>
            <Search class="w-4 h-4 text-zinc-500" />
          </template>
        </UiInput>
      </div>

      <div class="flex flex-wrap items-center gap-2">
        <UiBadge variant="amber" size="sm">
          {{ instance.mods.filter((m) => m.enabled).length }} / {{ instance.mods.length }} Actifs
        </UiBadge>

        <UiBadge
          v-if="updatesAvailableCount > 0"
          variant="amber"
          size="sm"
          class="font-mono animate-pulse flex items-center gap-1"
        >
          <ArrowUpCircle class="w-3.5 h-3.5 text-amber-400" />
          {{ updatesAvailableCount }} MAJ
        </UiBadge>

        <UiButton
          variant="outline"
          size="sm"
          @click="checkUpdates"
          :loading="isCheckingUpdates"
          title="Vérifier les mises à jour pour tous les mods"
        >
          <template #prefix>
            <RefreshCw :class="['w-3.5 h-3.5 text-amber-400', isCheckingUpdates ? 'animate-spin' : '']" />
          </template>
          Vérifier les MAJ
        </UiButton>

        <UiButton
          variant="outline"
          size="sm"
          @click="minecraftStore.openInstanceFolder(instance.id, '.minecraft/mods')"
        >
          <template #prefix>
            <FolderOpen class="w-3.5 h-3.5" />
          </template>
          Dossier Mods
        </UiButton>
      </div>
    </div>

    <!-- Mod list with Load Order and controls -->
    <div class="border border-zinc-800/80 rounded-2xl overflow-hidden bg-zinc-950/40 divide-y divide-zinc-800/60 shadow-inner">
      <div
        v-for="(mod, index) in filteredMods"
        :key="mod.id"
        @click="openModDetails(mod)"
        class="p-4 flex items-center justify-between gap-4 hover:bg-zinc-900/50 transition-colors group cursor-pointer"
      >
        <!-- Reorder Handle & Priority -->
        <div class="flex items-center gap-2 flex-shrink-0" @click.stop>
          <div class="flex flex-col items-center">
            <button
              type="button"
              :disabled="index === 0"
              @click.stop="moveUp(index)"
              class="text-zinc-600 hover:text-zinc-300 disabled:opacity-20 p-0.5"
              title="Monter en priorité de chargement"
            >
              <ChevronUp class="w-3.5 h-3.5" />
            </button>
            <button
              type="button"
              :disabled="index === filteredMods.length - 1"
              @click.stop="moveDown(index)"
              class="text-zinc-600 hover:text-zinc-300 disabled:opacity-20 p-0.5"
              title="Descendre en priorité de chargement"
            >
              <ChevronDown class="w-3.5 h-3.5" />
            </button>
          </div>

          <span class="font-mono text-[10px] text-zinc-500 w-5 text-center">
            #{{ index + 1 }}
          </span>
        </div>

        <div class="flex items-center gap-3.5 flex-1 min-w-0">
          <!-- Toggle switch -->
          <button
            type="button"
            @click.stop="minecraftStore.toggleMod(instance.id, mod.id)"
            :class="[
              'relative inline-flex h-5 w-9 flex-shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 focus:outline-none',
              mod.enabled ? 'bg-amber-500' : 'bg-zinc-700'
            ]"
            title="Activer / Désactiver ce mod"
          >
            <span
              :class="[
                'pointer-events-none inline-block h-4 w-4 transform rounded-full bg-zinc-950 shadow transition duration-200',
                mod.enabled ? 'translate-x-4' : 'translate-x-0'
              ]"
            />
          </button>

          <!-- Mod Provider Icon -->
          <div class="w-10 h-10 rounded-xl bg-zinc-950 border border-zinc-800 shrink-0 overflow-hidden flex items-center justify-center p-0.5 shadow-inner">
            <img
              v-if="mod.iconUrl"
              :src="mod.iconUrl"
              :alt="mod.name"
              class="w-full h-full object-cover rounded-lg group-hover:scale-105 transition-transform"
              loading="lazy"
            />
            <Package v-else class="w-5 h-5 text-zinc-600" />
          </div>

          <div class="min-w-0 flex-1">
            <div class="flex items-center gap-2 flex-wrap">
              <span
                :class="[
                  'font-bold text-sm truncate group-hover:text-amber-400 transition-colors',
                  mod.enabled ? 'text-zinc-100' : 'text-zinc-500 line-through'
                ]"
              >
                {{ mod.name }}
              </span>
              <span class="text-xs text-zinc-500 font-mono">v{{ mod.version }}</span>

              <!-- Platform Source Badge -->
              <UiBadge
                v-if="mod.provider === 'modrinth'"
                variant="emerald"
                size="xs"
              >
                Modrinth
              </UiBadge>
              <UiBadge
                v-else-if="mod.provider === 'curseforge'"
                variant="amber"
                size="xs"
              >
                CurseForge
              </UiBadge>
              <UiBadge
                v-else
                variant="zinc"
                size="xs"
              >
                Local
              </UiBadge>

              <UiBadge v-if="mod.updateAvailable" variant="amber" size="xs" class="animate-pulse">
                <ArrowUpCircle class="w-3 h-3 text-amber-400 inline mr-0.5" />
                Mise à jour {{ mod.newVersion }}
              </UiBadge>
            </div>
            <p class="text-xs text-zinc-400 truncate mt-0.5">
              {{ mod.description }}
            </p>
          </div>
        </div>

        <div class="flex items-center gap-3 shrink-0 font-mono text-[11px] text-zinc-500" @click.stop>
          <!-- Quick Update Button -->
          <button
            v-if="mod.updateAvailable"
            type="button"
            @click.stop="quickUpdate(mod)"
            :disabled="updatingModIds[mod.id]"
            class="px-2.5 py-1 rounded-xl bg-amber-500/20 hover:bg-amber-500 border border-amber-500/40 text-amber-300 hover:text-zinc-950 font-mono text-xs font-bold transition-all flex items-center gap-1 shadow-sm disabled:opacity-50"
            title="Mettre à jour directement"
          >
            <Loader2 v-if="updatingModIds[mod.id]" class="w-3.5 h-3.5 animate-spin" />
            <ArrowUpCircle v-else class="w-3.5 h-3.5" />
            <span class="hidden sm:inline">MAJ</span>
          </button>

          <div class="text-right hidden sm:block">
            <div class="text-zinc-400 truncate max-w-[160px]">{{ mod.fileName }}</div>
            <div>{{ mod.fileSizeMb }} Mo • {{ mod.author }}</div>
          </div>

          <!-- Delete action -->
          <button
            type="button"
            @click.stop="confirmDelete(mod.id, mod.name)"
            class="p-2 rounded-xl text-zinc-500 hover:text-rose-400 hover:bg-rose-500/10 transition-colors"
            title="Désinstaller le mod"
          >
            <Trash2 class="w-4 h-4" />
          </button>
        </div>
      </div>

      <div v-if="filteredMods.length === 0" class="p-10 text-center text-zinc-500 text-sm">
        Aucun mod installé correspondant à votre recherche. Glissez des fichiers .jar ci-dessus ou parcourez le catalogue.
      </div>
    </div>

    <!-- Mod Details Modal -->
    <ModDetailsModal
      v-model="detailsModalOpen"
      :provider="selectedModProvider"
      :modId="selectedModId"
      :installedVersion="selectedModVersion"
      :instanceId="instance.id"
      @updated="onModUpdated"
      @installed="onModUpdated"
    />
  </div>
</template>
