<script setup lang="ts">
import { computed, ref } from 'vue'
import { useWowAddonsStore } from '@/stores/wowAddonsStore'
import {
  X,
  Download,
  CheckCircle2,
  ArrowUpCircle,
  ExternalLink,
  Calendar,
  Image as ImageIcon,
  Sparkles
} from 'lucide-vue-next'
import UiBadge from '@/components/ui/UiBadge.vue'

const wowStore = useWowAddonsStore()

const modalOpen = computed(() => wowStore.detailsModalOpen)
const addon = computed(() => wowStore.selectedAddonForDetails)
const files = computed(() => wowStore.detailsFiles)
const isLoading = computed(() => wowStore.isLoadingDetails)

const activeSubTab = ref<'description' | 'files' | 'screenshots'>('description')

function close() {
  wowStore.closeDetailsModal()
}

function formatDate(dateStr?: string): string {
  if (!dateStr) return '-'
  try {
    return new Date(dateStr).toLocaleDateString('fr-FR', {
      year: 'numeric',
      month: 'short',
      day: 'numeric'
    })
  } catch {
    return dateStr
  }
}

function formatSize(bytes?: number): string {
  if (!bytes) return '-'
  const mb = bytes / (1024 * 1024)
  if (mb < 0.1) return `${Math.round(bytes / 1024)} Ko`
  return `${mb.toFixed(1)} Mo`
}

async function installSpecificFile(file: any) {
  if (!addon.value) return
  const p1 = Math.floor(file.id / 1000)
  const p2 = file.id % 1000
  const dlUrl = file.downloadUrl || `https://edge.forgecdn.net/files/${p1}/${p2}/${encodeURIComponent(file.fileName)}`

  await wowStore.installAddon({
    modId: addon.value.id,
    fileId: file.id,
    downloadUrl: dlUrl,
    modName: addon.value.name,
    version: file.displayName || file.fileName,
    iconUrl: addon.value.iconUrl
  })
}

async function handleMainInstall() {
  if (!addon.value) return
  await wowStore.installAddon({
    modId: addon.value.id,
    modName: addon.value.name,
    iconUrl: addon.value.iconUrl
  })
}
</script>

<template>
  <div
    v-if="modalOpen"
    class="fixed inset-0 z-50 bg-black/80 backdrop-blur-md flex items-center justify-center p-3 sm:p-6"
    @click.self="close"
  >
    <div
      class="bg-zinc-950 border border-zinc-800 rounded-3xl w-full max-w-4xl max-h-[90vh] flex flex-col shadow-2xl overflow-hidden animate-in fade-in zoom-in-95 duration-150"
    >
      <!-- En-tête de la modale -->
      <div class="p-5 sm:p-6 border-b border-zinc-800/80 bg-zinc-900/40 relative">
        <button
          type="button"
          @click="close"
          class="absolute top-5 right-5 p-2 rounded-xl bg-zinc-800/80 hover:bg-zinc-700 text-zinc-400 hover:text-white transition"
        >
          <X class="w-5 h-5" />
        </button>

        <div v-if="addon" class="flex flex-col sm:flex-row items-start sm:items-center gap-4 pr-12">
          <!-- Logo -->
          <div class="w-16 h-16 rounded-2xl bg-zinc-900 border border-zinc-800 overflow-hidden flex-shrink-0 flex items-center justify-center shadow-lg">
            <img
              v-if="addon.iconUrl"
              :src="addon.iconUrl"
              :alt="addon.name"
              class="w-full h-full object-cover"
            />
            <span v-else class="text-base font-black font-mono text-amber-400">
              {{ addon.name.slice(0, 2).toUpperCase() }}
            </span>
          </div>

          <!-- Infos Titre & Auteur -->
          <div class="space-y-1.5 flex-1 min-w-0">
            <div class="flex flex-wrap items-center gap-2">
              <h2 class="text-xl font-black text-zinc-100 font-display truncate">
                {{ addon.name }}
              </h2>
              <UiBadge variant="amber" size="xs" class="font-mono">
                CurseForge
              </UiBadge>
              <UiBadge v-if="addon.isInstalled" variant="emerald" size="xs" class="font-mono">
                INSTALLÉ ({{ addon.installedVersion || 'v1.0' }})
              </UiBadge>
            </div>

            <div class="flex flex-wrap items-center gap-3 text-xs font-mono text-zinc-400">
              <span>Par <strong class="text-zinc-200">{{ addon.author }}</strong></span>
              <span class="text-zinc-600">•</span>
              <span class="flex items-center gap-1 text-amber-400">
                <Download class="w-3.5 h-3.5" />
                {{ addon.downloadCount.toLocaleString('fr-FR') }} téléchargements
              </span>
              <span v-if="addon.dateModified" class="text-zinc-600">•</span>
              <span v-if="addon.dateModified" class="flex items-center gap-1 text-zinc-400">
                <Calendar class="w-3.5 h-3.5" />
                {{ formatDate(addon.dateModified) }}
              </span>
              <a
                v-if="addon.websiteUrl"
                :href="addon.websiteUrl"
                target="_blank"
                rel="noopener"
                class="text-amber-400/80 hover:text-amber-300 inline-flex items-center gap-1 hover:underline ml-1"
              >
                Page CurseForge <ExternalLink class="w-3 h-3" />
              </a>
            </div>
          </div>

          <!-- Action principale en haut -->
          <div class="flex items-center gap-3 self-stretch sm:self-auto justify-end pt-2 sm:pt-0">
            <button
              v-if="addon.isInstalled && addon.updateAvailable"
              type="button"
              @click="handleMainInstall"
              :disabled="wowStore.installingMap[addon.id]"
              class="px-5 py-2.5 rounded-xl bg-amber-500 hover:bg-amber-400 text-zinc-950 font-mono font-bold text-xs tracking-wide shadow-lg shadow-amber-500/20 active:scale-95 transition flex items-center gap-2 disabled:opacity-50"
            >
              <ArrowUpCircle class="w-4 h-4" :class="{ 'animate-spin': wowStore.installingMap[addon.id] }" />
              <span>Mettre à jour</span>
            </button>

            <button
              v-else-if="addon.isInstalled"
              type="button"
              class="px-5 py-2.5 rounded-xl bg-zinc-800 border border-zinc-700 text-emerald-400 font-mono font-semibold text-xs flex items-center gap-2 cursor-default"
            >
              <CheckCircle2 class="w-4 h-4" />
              <span>Installé</span>
            </button>

            <button
              v-else
              type="button"
              @click="handleMainInstall"
              :disabled="wowStore.installingMap[addon.id]"
              class="px-6 py-2.5 rounded-xl bg-amber-500 hover:bg-amber-400 text-zinc-950 font-mono font-bold text-xs tracking-wide shadow-lg shadow-amber-500/20 active:scale-95 transition flex items-center gap-2 disabled:opacity-50"
            >
              <Download class="w-4 h-4" :class="{ 'animate-bounce': wowStore.installingMap[addon.id] }" />
              <span>{{ wowStore.installingMap[addon.id] ? 'Installation...' : 'Installer' }}</span>
            </button>
          </div>
        </div>
      </div>

      <!-- Barre d'onglets interne -->
      <div class="flex items-center gap-4 px-6 border-b border-zinc-800/80 bg-zinc-950 text-xs font-mono">
        <button
          type="button"
          @click="activeSubTab = 'description'"
          :class="[
            'py-3 border-b-2 font-semibold transition',
            activeSubTab === 'description'
              ? 'border-amber-400 text-amber-400'
              : 'border-transparent text-zinc-400 hover:text-zinc-200'
          ]"
        >
          Description
        </button>

        <button
          type="button"
          @click="activeSubTab = 'files'"
          :class="[
            'py-3 border-b-2 font-semibold transition flex items-center gap-1.5',
            activeSubTab === 'files'
              ? 'border-amber-400 text-amber-400'
              : 'border-transparent text-zinc-400 hover:text-zinc-200'
          ]"
        >
          <span>Versions & Fichiers</span>
          <span class="px-1.5 py-0.2 rounded-md bg-zinc-800 text-[10px] text-zinc-300">
            {{ files.length }}
          </span>
        </button>

        <button
          v-if="addon?.screenshots && addon.screenshots.length > 0"
          type="button"
          @click="activeSubTab = 'screenshots'"
          :class="[
            'py-3 border-b-2 font-semibold transition flex items-center gap-1.5',
            activeSubTab === 'screenshots'
              ? 'border-amber-400 text-amber-400'
              : 'border-transparent text-zinc-400 hover:text-zinc-200'
          ]"
        >
          <ImageIcon class="w-3.5 h-3.5" />
          <span>Captures ({{ addon.screenshots.length }})</span>
        </button>
      </div>

      <!-- Corps défilable -->
      <div class="p-6 overflow-y-auto flex-1 space-y-4">
        <!-- Chargement -->
        <div v-if="isLoading" class="py-16 flex flex-col items-center justify-center gap-3 text-zinc-400">
          <Sparkles class="w-8 h-8 text-amber-400 animate-spin" />
          <span class="text-xs font-mono">Récupération des données CurseForge...</span>
        </div>

        <!-- Onglet 1 : Description -->
        <div v-else-if="activeSubTab === 'description'" class="space-y-4">
          <p v-if="addon?.summary" class="text-sm text-zinc-300 leading-relaxed bg-zinc-900/40 p-4 rounded-2xl border border-zinc-800/60">
            {{ addon.summary }}
          </p>

          <div
            v-if="addon?.descriptionHtml"
            class="prose prose-invert prose-amber max-w-none text-xs leading-relaxed text-zinc-300 overflow-x-auto"
            v-html="addon.descriptionHtml"
          />
          <div v-else class="text-xs text-zinc-500 py-6 text-center font-mono">
            Aucune description détaillée disponible pour cet addon.
          </div>
        </div>

        <!-- Onglet 2 : Fichiers et versions -->
        <div v-else-if="activeSubTab === 'files'" class="space-y-3">
          <div class="text-xs text-zinc-400 font-mono mb-2">
            Sélectionnez une version spécifique compatible avec votre instance WoW :
          </div>

          <div
            v-for="file in files"
            :key="file.id"
            class="p-4 rounded-xl bg-zinc-900/50 hover:bg-zinc-900/80 border border-zinc-800/80 flex flex-col sm:flex-row items-start sm:items-center justify-between gap-3 transition"
          >
            <div class="space-y-1 min-w-0">
              <div class="flex flex-wrap items-center gap-2">
                <span class="font-bold text-xs text-zinc-200 font-mono">
                  {{ file.displayName || file.fileName }}
                </span>
                <UiBadge
                  :variant="file.releaseType === 1 ? 'emerald' : file.releaseType === 2 ? 'amber' : 'rose'"
                  size="xs"
                  class="font-mono text-[10px]"
                >
                  {{ file.releaseType === 1 ? 'Release' : file.releaseType === 2 ? 'Beta' : 'Alpha' }}
                </UiBadge>
              </div>

              <!-- Versions de jeu supportées -->
              <div class="flex flex-wrap items-center gap-1.5 text-[10px] font-mono text-zinc-400">
                <span class="text-zinc-500">WoW :</span>
                <span
                  v-for="gv in (file.gameVersions || []).slice(0, 5)"
                  :key="gv"
                  class="px-1.5 py-0.5 rounded bg-zinc-950 border border-zinc-800 text-zinc-300"
                >
                  {{ gv }}
                </span>
                <span class="text-zinc-600">•</span>
                <span>{{ formatSize(file.fileLength) }}</span>
                <span class="text-zinc-600">•</span>
                <span>{{ formatDate(file.fileDate) }}</span>
              </div>
            </div>

            <button
              type="button"
              @click="installSpecificFile(file)"
              class="px-3.5 py-1.5 rounded-xl bg-zinc-800 hover:bg-amber-500 hover:text-zinc-950 text-zinc-300 font-mono font-bold text-xs transition flex items-center gap-1.5 flex-shrink-0"
            >
              <Download class="w-3.5 h-3.5" />
              <span>Installer ce fichier</span>
            </button>
          </div>
        </div>

        <!-- Onglet 3 : Captures d'écran -->
        <div
          v-else-if="activeSubTab === 'screenshots'"
          class="grid grid-cols-1 sm:grid-cols-2 gap-4"
        >
          <div
            v-for="shot in addon?.screenshots"
            :key="shot.id"
            class="rounded-xl overflow-hidden border border-zinc-800 bg-zinc-900 group"
          >
            <img
              :src="shot.url"
              :alt="shot.title || 'Capture'"
              class="w-full h-48 object-cover group-hover:scale-105 transition-transform"
            />
            <div v-if="shot.title" class="p-2.5 text-xs text-zinc-400 font-mono bg-zinc-950 truncate">
              {{ shot.title }}
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
