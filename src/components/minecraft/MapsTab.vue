<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import {
  Compass,
  FolderOpen,
  Copy,
  Trash2,
  Search,
  RefreshCw,
  HardDrive,
  Calendar,
  Loader2,
  Check
} from 'lucide-vue-next'
import { useMinecraftStore } from '@/stores/minecraft'
import UiBadge from '@/components/ui/UiBadge.vue'
import UiButton from '@/components/ui/UiButton.vue'

const minecraftStore = useMinecraftStore()
const activeInst = computed(() => minecraftStore.activeInstance)

const isLoading = ref(false)
const saves = ref<any[]>([])
const searchQuery = ref('')
const backingUpFolder = ref<string | null>(null)
const deletingFolder = ref<string | null>(null)
const successNotice = ref<string | null>(null)

async function loadSaves() {
  if (!activeInst.value) return
  isLoading.value = true
  try {
    saves.value = await minecraftStore.getInstanceSaves(activeInst.value.id)
  } catch (err) {
    console.warn('[MapsTab] Error loading saves:', err)
  } finally {
    isLoading.value = false
  }
}

const filteredSaves = computed(() => {
  const q = searchQuery.value.trim().toLowerCase()
  if (!q) return saves.value
  return saves.value.filter(
    (s) =>
      s.name.toLowerCase().includes(q) ||
      s.folderName.toLowerCase().includes(q) ||
      (s.gameMode && s.gameMode.toLowerCase().includes(q))
  )
})

function formatBytes(bytes: number): string {
  if (!bytes || bytes <= 0) return '0 Mo'
  const mb = bytes / (1024 * 1024)
  if (mb < 0.1) {
    const kb = bytes / 1024
    return `${kb.toFixed(0)} Ko`
  }
  if (mb >= 1000) {
    return `${(mb / 1024).toFixed(2)} Go`
  }
  return `${mb.toFixed(1)} Mo`
}

function formatDate(ts: number): string {
  if (!ts) return 'Inconnue'
  try {
    const d = new Date(ts)
    return d.toLocaleDateString('fr-FR', {
      day: 'numeric',
      month: 'short',
      year: 'numeric',
      hour: '2-digit',
      minute: '2-digit'
    })
  } catch {
    return 'Inconnue'
  }
}

function openSavesFolder() {
  if (activeInst.value) {
    minecraftStore.openInstanceFolder(activeInst.value.id, 'saves')
  }
}

function openSpecificWorld(folderName: string) {
  if (activeInst.value) {
    minecraftStore.openInstanceSaveFolder(activeInst.value.id, folderName)
  }
}

async function handleBackup(folderName: string) {
  if (!activeInst.value) return
  backingUpFolder.value = folderName
  try {
    const res = await minecraftStore.backupInstanceSave(activeInst.value.id, folderName)
    if (res?.success) {
      successNotice.value = `Sauvegarde "${folderName}" créée avec succès dans .minecraft/backups/ !`
      setTimeout(() => (successNotice.value = null), 4000)
    }
  } catch (err: any) {
    alert(`Erreur de sauvegarde: ${err?.message || err}`)
  } finally {
    backingUpFolder.value = null
  }
}

async function handleDelete(folderName: string, worldName: string) {
  if (!activeInst.value) return
  if (confirm(`Supprimer définitivement le monde "${worldName}" (${folderName}) ? Cette action est irréversible.`)) {
    deletingFolder.value = folderName
    try {
      await minecraftStore.deleteInstanceSave(activeInst.value.id, folderName)
      saves.value = saves.value.filter((s) => s.folderName !== folderName)
    } catch (err: any) {
      alert(`Erreur de suppression: ${err?.message || err}`)
    } finally {
      deletingFolder.value = null
    }
  }
}

onMounted(() => {
  loadSaves()
})
</script>

<template>
  <div class="space-y-6">
    <!-- Notice Banner -->
    <div
      v-if="successNotice"
      class="p-4 rounded-2xl bg-emerald-500/10 border border-emerald-500/30 flex items-center justify-between gap-3 text-xs font-mono text-emerald-300 animate-fadeIn"
    >
      <div class="flex items-center gap-2">
        <Check class="w-4 h-4 text-emerald-400" />
        <span>{{ successNotice }}</span>
      </div>
      <button type="button" @click="successNotice = null" class="text-zinc-400 hover:text-white">✕</button>
    </div>

    <!-- Header Actions Toolbar -->
    <div class="flex flex-col sm:flex-row items-stretch sm:items-center justify-between gap-4 bg-zinc-900/60 p-5 rounded-3xl border border-zinc-800/80">
      <div class="flex items-center gap-3">
        <div class="w-10 h-10 rounded-2xl bg-zinc-950 border border-zinc-800 flex items-center justify-center text-amber-400 shadow-inner">
          <Compass class="w-5 h-5" />
        </div>
        <div>
          <div class="text-sm font-bold text-zinc-100 flex items-center gap-2">
            <span>Mondes & Sauvegardes</span>
            <UiBadge variant="amber" size="xs">{{ saves.length }}</UiBadge>
          </div>
          <p class="text-xs text-zinc-500 font-mono">Dossier <span class="text-zinc-400">.minecraft/saves/</span></p>
        </div>
      </div>

      <div class="flex flex-wrap items-center gap-2.5">
        <!-- Search input -->
        <div class="relative flex-1 sm:w-56">
          <input
            v-model="searchQuery"
            type="text"
            placeholder="Filtrer les mondes..."
            class="w-full bg-zinc-950 border border-zinc-800 rounded-2xl pl-9 pr-3 py-2 text-xs text-zinc-200 placeholder-zinc-500 focus:outline-none focus:border-amber-500/80 transition-colors"
          />
          <Search class="w-3.5 h-3.5 text-zinc-500 absolute left-3 top-1/2 -translate-y-1/2 pointer-events-none" />
        </div>

        <!-- Refresh button -->
        <UiButton variant="ghost" size="sm" @click="loadSaves" :disabled="isLoading" title="Actualiser la liste">
          <RefreshCw :class="['w-3.5 h-3.5', isLoading ? 'animate-spin text-amber-400' : '']" />
        </UiButton>

        <!-- Open saves folder -->
        <UiButton variant="secondary" size="sm" @click="openSavesFolder" title="Ouvrir le dossier saves dans l'explorateur">
          <template #prefix>
            <FolderOpen class="w-3.5 h-3.5 text-amber-400" />
          </template>
          Dossier saves
        </UiButton>
      </div>
    </div>

    <!-- Loading State -->
    <div v-if="isLoading && saves.length === 0" class="py-20 text-center">
      <Loader2 class="w-8 h-8 text-amber-500 animate-spin mx-auto mb-3" />
      <p class="text-sm text-zinc-400 font-mono">Scan des mondes de l'instance...</p>
    </div>

    <!-- Worlds Cards Grid -->
    <div
      v-else-if="filteredSaves.length > 0"
      class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-5"
    >
      <div
        v-for="world in filteredSaves"
        :key="world.folderName"
        class="flex flex-col justify-between p-5 rounded-3xl bg-zinc-900/60 border border-zinc-800/80 hover:border-amber-500/40 transition-colors group select-none gap-4"
      >
        <div class="space-y-3">
          <!-- Top row: World Thumbnail, Name & Badges -->
          <div class="flex items-start gap-3.5">
            <div class="w-14 h-14 rounded-2xl bg-zinc-950 border border-zinc-800 shrink-0 overflow-hidden flex items-center justify-center p-0.5 shadow-md">
              <img
                v-if="world.hasIcon && world.iconDataUrl"
                :src="world.iconDataUrl"
                :alt="world.name"
                class="w-full h-full object-cover rounded-xl"
              />
              <div
                v-else
                class="w-full h-full rounded-xl bg-gradient-to-br from-emerald-950 via-zinc-900 to-amber-950 flex items-center justify-center text-emerald-400"
              >
                <Compass class="w-6 h-6 opacity-70" />
              </div>
            </div>

            <div class="flex-1 min-w-0">
              <h4 class="font-bold text-sm text-zinc-100 group-hover:text-amber-400 transition-colors truncate" :title="world.name">
                {{ world.name }}
              </h4>
              <p class="text-[11px] font-mono text-zinc-500 truncate mt-0.5" :title="world.folderName">
                {{ world.folderName }}
              </p>

              <div class="flex flex-wrap items-center gap-1.5 mt-2">
                <!-- Mode Badge -->
                <UiBadge
                  :variant="world.isHardcore ? 'rose' : world.gameMode === 'Créatif' ? 'amber' : world.gameMode === 'Aventure' ? 'sky' : 'emerald'"
                  size="xs"
                >
                  {{ world.gameMode }}
                </UiBadge>

                <!-- Version Badge -->
                <UiBadge v-if="world.versionName" variant="zinc" size="xs" class="font-mono">
                  {{ world.versionName }}
                </UiBadge>
              </div>
            </div>
          </div>

          <!-- Metadata row -->
          <div class="grid grid-cols-2 gap-2 p-2.5 rounded-2xl bg-zinc-950/60 border border-zinc-800/60 text-[11px] font-mono text-zinc-400">
            <div class="flex items-center gap-1.5 truncate">
              <HardDrive class="w-3.5 h-3.5 text-zinc-500 shrink-0" />
              <span>{{ formatBytes(world.sizeBytes) }}</span>
            </div>
            <div class="flex items-center gap-1.5 truncate" :title="`Joué le ${formatDate(world.lastPlayed)}`">
              <Calendar class="w-3.5 h-3.5 text-zinc-500 shrink-0" />
              <span class="truncate">{{ formatDate(world.lastPlayed) }}</span>
            </div>
          </div>
        </div>

        <!-- Footer Actions Bar -->
        <div class="pt-3 border-t border-zinc-800/80 flex items-center justify-between gap-2">
          <!-- Open World Folder -->
          <button
            type="button"
            @click="openSpecificWorld(world.folderName)"
            class="px-2.5 py-1.5 rounded-xl bg-zinc-950 hover:bg-zinc-800 border border-zinc-800 hover:border-zinc-700 text-zinc-300 hover:text-amber-400 font-mono text-xs font-bold transition flex items-center gap-1.5"
            title="Ouvrir le dossier de cette sauvegarde"
          >
            <FolderOpen class="w-3.5 h-3.5 text-amber-400" />
            <span>Dossier</span>
          </button>

          <div class="flex items-center gap-1.5">
            <!-- Backup / Duplicate -->
            <button
              type="button"
              @click="handleBackup(world.folderName)"
              :disabled="backingUpFolder === world.folderName"
              class="px-2.5 py-1.5 rounded-xl bg-zinc-950 hover:bg-zinc-800 border border-zinc-800 hover:border-zinc-700 text-zinc-300 hover:text-emerald-400 font-mono text-xs font-bold transition flex items-center gap-1.5 disabled:opacity-50"
              title="Créer une sauvegarde sécurisée dans backups/"
            >
              <Loader2 v-if="backingUpFolder === world.folderName" class="w-3.5 h-3.5 animate-spin text-emerald-400" />
              <Copy v-else class="w-3.5 h-3.5 text-emerald-400" />
              <span>Backup</span>
            </button>

            <!-- Delete -->
            <button
              type="button"
              @click="handleDelete(world.folderName, world.name)"
              :disabled="deletingFolder === world.folderName"
              class="p-1.5 rounded-xl text-zinc-500 hover:text-rose-400 hover:bg-rose-500/10 transition disabled:opacity-50"
              title="Supprimer définitivement ce monde"
            >
              <Loader2 v-if="deletingFolder === world.folderName" class="w-3.5 h-3.5 animate-spin text-rose-400" />
              <Trash2 v-else class="w-3.5 h-3.5" />
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- Empty State -->
    <div
      v-else
      class="p-16 text-center rounded-3xl bg-zinc-900/30 border border-zinc-800/80 space-y-4"
    >
      <div class="w-14 h-14 mx-auto rounded-2xl bg-zinc-800/80 flex items-center justify-center text-amber-400">
        <Compass class="w-7 h-7" />
      </div>
      <div class="space-y-1">
        <h4 class="text-sm font-bold text-zinc-200">
          {{ searchQuery ? 'Aucun monde ne correspond à votre recherche' : 'Aucun monde dans cette instance' }}
        </h4>
        <p class="text-xs text-zinc-500 max-w-sm mx-auto">
          Lancez le jeu pour créer votre premier monde ou déposez vos sauvegardes existantes dans le dossier
          <span class="font-mono text-amber-400">.minecraft/saves/</span>.
        </p>
      </div>
      <UiButton variant="primary" size="sm" @click="openSavesFolder">
        <template #prefix>
          <FolderOpen class="w-3.5 h-3.5" />
        </template>
        Ouvrir le dossier saves
      </UiButton>
    </div>
  </div>
</template>
