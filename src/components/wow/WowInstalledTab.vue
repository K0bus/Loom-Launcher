<script setup lang="ts">
import { computed, ref } from 'vue'
import { useWowAddonsStore } from '@/stores/wowAddonsStore'
import type { InstalledWowAddon } from '@/models/wowAddon'
import {
  Search,
  RefreshCw,
  ArrowUpCircle,
  Trash2,
  ExternalLink,
  Info,
  Package,
  HardDrive,
  FolderOpen,
  Layers,
  ChevronDown,
  Download
} from 'lucide-vue-next'
import UiBadge from '@/components/ui/UiBadge.vue'

const wowStore = useWowAddonsStore()

const installedList = computed(() => wowStore.filteredInstalledAddons)
const totalCount = computed(() => wowStore.installedCount)
const totalFoldersCount = computed(() => wowStore.totalFoldersCount)
const enabledCount = computed(() => wowStore.enabledCount)
const updatesCount = computed(() => wowStore.updatesAvailableCount)

const expandedSubmodules = ref<Record<string, boolean>>({})

function toggleExpand(folderName: string) {
  expandedSubmodules.value[folderName] = !expandedSubmodules.value[folderName]
}

async function handleToggle(addon: InstalledWowAddon) {
  await wowStore.toggleAddon(addon)
}

async function handleUpdate(addon: InstalledWowAddon) {
  await wowStore.updateAddon(addon)
}

async function handleUninstall(addon: InstalledWowAddon) {
  const msg = addon.secondaryFolders && addon.secondaryFolders.length > 0
    ? `Voulez-vous vraiment désinstaller l'addon "${addon.title}" ainsi que ses ${addon.secondaryFolders.length} sous-dossiers associés ?`
    : `Voulez-vous vraiment désinstaller l'addon "${addon.title}" ?`

  if (confirm(msg)) {
    await wowStore.uninstallAddon(addon)
  }
}

function openDetails(addon: InstalledWowAddon) {
  wowStore.openDetailsModal(addon)
}
</script>

<template>
  <div class="space-y-5">
    <!-- Barre de filtres et d'actions globales -->
    <div class="flex flex-col md:flex-row items-stretch md:items-center justify-between gap-4 bg-zinc-900/40 p-4 rounded-2xl border border-zinc-800/80">
      <!-- Recherche locale -->
      <div class="relative flex-1 max-w-md">
        <Search class="w-4 h-4 text-zinc-400 absolute left-3.5 top-1/2 -translate-y-1/2" />
        <input
          v-model="wowStore.installedFilter"
          type="text"
          placeholder="Filtrer vos addons installés (nom, dossier, auteur)..."
          class="w-full pl-10 pr-4 py-2 bg-zinc-950/80 border border-zinc-800 rounded-xl text-xs text-zinc-200 placeholder-zinc-500 focus:outline-none focus:border-amber-500/80 transition font-mono"
        />
      </div>

      <!-- Résumé & Boutons d'action -->
      <div class="flex flex-wrap items-center gap-3">
        <!-- Compteur d'addons regroupés et dossiers totaux -->
        <div class="text-xs font-mono text-zinc-400 flex items-center gap-2">
          <span class="text-zinc-200 font-bold">{{ totalCount }} addons</span>
          <span class="text-zinc-500 text-[11px]">({{ totalFoldersCount }} dossiers)</span>
          <span class="text-zinc-600">•</span>
          <span class="text-emerald-400 font-semibold">{{ enabledCount }} activés</span>
        </div>

        <!-- Bouton Tout Mettre à Jour -->
        <button
          v-if="updatesCount > 0"
          type="button"
          @click="wowStore.updateAllAddons()"
          :disabled="wowStore.isUpdatingAll"
          class="flex items-center gap-2 px-3.5 py-2 rounded-xl bg-gradient-to-r from-amber-500 to-amber-600 hover:from-amber-400 hover:to-amber-500 text-zinc-950 font-bold text-xs tracking-wide shadow-lg shadow-amber-500/20 active:scale-95 transition disabled:opacity-50"
        >
          <ArrowUpCircle
            class="w-4 h-4"
            :class="{ 'animate-spin': wowStore.isUpdatingAll }"
          />
          <span>Tout mettre à jour ({{ updatesCount }})</span>
        </button>

        <!-- Exporter les mods et la configuration -->
        <div class="relative group">
          <button
            type="button"
            @click="wowStore.exportBackup()"
            :disabled="wowStore.isExporting"
            class="p-2 rounded-xl bg-zinc-800/80 hover:bg-zinc-700/80 border border-zinc-700/60 text-zinc-300 hover:text-white transition disabled:opacity-50"
            title="exporter les mods et config"
          >
            <Download
              class="w-4 h-4 text-amber-400"
              :class="{ 'animate-bounce': wowStore.isExporting }"
            />
          </button>
          <div class="absolute bottom-full left-1/2 -translate-x-1/2 mb-2 hidden group-hover:flex items-center px-2.5 py-1 rounded-lg bg-zinc-900 border border-zinc-700/80 text-[11px] font-mono text-zinc-200 whitespace-nowrap shadow-xl pointer-events-none z-30">
            exporter les mods et config
          </div>
        </div>

        <!-- Rafraîchir les vérifications -->
        <button
          type="button"
          @click="wowStore.loadInstalledAddons(true)"
          :disabled="wowStore.isLoadingInstalled || wowStore.isCheckingUpdates"
          class="p-2 rounded-xl bg-zinc-800/80 hover:bg-zinc-700/80 border border-zinc-700/60 text-zinc-300 hover:text-white transition disabled:opacity-50"
          title="Rechercher des mises à jour CurseForge"
        >
          <RefreshCw
            class="w-4 h-4 text-amber-400"
            :class="{ 'animate-spin': wowStore.isLoadingInstalled || wowStore.isCheckingUpdates }"
          />
        </button>
      </div>
    </div>

    <!-- Bannière de mise à jour si des mises à jour sont en attente -->
    <div
      v-if="updatesCount > 0 && !wowStore.installedFilter"
      class="bg-amber-500/10 border border-amber-500/30 rounded-2xl p-4 flex items-center justify-between gap-4"
    >
      <div class="flex items-center gap-3">
        <div class="w-9 h-9 rounded-xl bg-amber-500/20 border border-amber-500/40 flex items-center justify-center flex-shrink-0">
          <ArrowUpCircle class="w-5 h-5 text-amber-400" />
        </div>
        <div>
          <h4 class="text-xs font-bold text-amber-300 font-mono">
            {{ updatesCount }} mise{{ updatesCount > 1 ? 's' : '' }} à jour disponible{{ updatesCount > 1 ? 's' : '' }}
          </h4>
          <p class="text-[11px] text-zinc-400">
            De nouvelles versions compatibles ont été détectées sur CurseForge pour vos addons.
          </p>
        </div>
      </div>
      <button
        type="button"
        @click="wowStore.updateAllAddons()"
        :disabled="wowStore.isUpdatingAll"
        class="px-3.5 py-1.5 rounded-xl bg-amber-500 hover:bg-amber-400 text-zinc-950 font-bold text-xs font-mono transition flex-shrink-0"
      >
        {{ wowStore.isUpdatingAll ? 'Mise à jour en cours...' : 'Mettre à jour tout' }}
      </button>
    </div>

    <!-- État de chargement initial -->
    <div
      v-if="wowStore.isLoadingInstalled && installedList.length === 0"
      class="py-20 flex flex-col items-center justify-center gap-3 text-zinc-400"
    >
      <RefreshCw class="w-8 h-8 text-amber-400 animate-spin" />
      <span class="text-xs font-mono">Regroupement des modules et analyse des fichiers .toc...</span>
    </div>

    <!-- État vide : aucun addon installé -->
    <div
      v-else-if="installedList.length === 0"
      class="py-16 text-center bg-zinc-950/40 rounded-3xl border border-zinc-800/80 p-8 space-y-4"
    >
      <div class="w-16 h-16 rounded-2xl bg-zinc-900 border border-zinc-800 flex items-center justify-center mx-auto text-amber-400">
        <Package class="w-8 h-8" />
      </div>
      <div class="space-y-1">
        <h3 class="text-base font-bold text-zinc-200">
          {{ wowStore.installedFilter ? 'Aucun addon trouvé' : 'Aucun addon installé pour cette instance' }}
        </h3>
        <p class="text-xs text-zinc-400 max-w-md mx-auto">
          {{
            wowStore.installedFilter
              ? 'Aucun addon installé ne correspond à votre filtre de recherche.'
              : 'Explorez le catalogue d\'addons CurseForge pour installer des mods d\'interface, de quêtes, ou de combat en un clic.'
          }}
        </p>
      </div>

      <div class="pt-2 flex items-center justify-center gap-3">
        <button
          v-if="!wowStore.installedFilter"
          type="button"
          @click="wowStore.activeTab = 'catalog'"
          class="px-4 py-2 rounded-xl bg-amber-500 hover:bg-amber-400 text-zinc-950 font-bold text-xs font-mono transition"
        >
          Parcourir le catalogue
        </button>
        <button
          type="button"
          @click="wowStore.openAddonsFolder()"
          class="px-4 py-2 rounded-xl bg-zinc-800 hover:bg-zinc-700 text-zinc-300 font-medium text-xs font-mono transition flex items-center gap-1.5"
        >
          <FolderOpen class="w-3.5 h-3.5 text-amber-400" />
          Ouvrir le dossier local
        </button>
      </div>
    </div>

    <!-- Liste des addons installés regroupés -->
    <div v-else class="space-y-3">
      <div
        v-for="addon in installedList"
        :key="addon.folderName"
        :class="[
          'relative p-4 sm:p-5 rounded-2xl border transition-all flex flex-col gap-3.5 group',
          addon.enabled
            ? 'bg-zinc-900/50 border-zinc-800/80 hover:border-zinc-700 hover:bg-zinc-900/80'
            : 'bg-zinc-950/40 border-zinc-900 opacity-60 hover:opacity-90'
        ]"
      >
        <!-- Ligne Principale : Infos + Contrôles -->
        <div class="flex flex-col md:flex-row items-start md:items-center justify-between gap-4 w-full">
          <!-- Info gauche : Icône + Nom + Description + Métas -->
          <div class="flex items-start gap-3.5 min-w-0 flex-1">
            <!-- Icône addon -->
            <div
              class="relative w-12 h-12 rounded-xl bg-zinc-950 border border-zinc-800 flex items-center justify-center flex-shrink-0 overflow-hidden cursor-pointer"
              @click="openDetails(addon)"
            >
              <img
                v-if="addon.iconUrl"
                :src="addon.iconUrl"
                :alt="addon.title"
                class="w-full h-full object-cover"
              />
              <div
                v-else
                class="text-xs font-black font-mono text-amber-400/80 flex items-center justify-center"
              >
                {{ addon.title.slice(0, 2).toUpperCase() }}
              </div>

              <!-- Pastille de désactivation -->
              <div
                v-if="!addon.enabled"
                class="absolute inset-0 bg-zinc-950/70 flex items-center justify-center text-[10px] text-zinc-400 font-mono font-bold"
              >
                OFF
              </div>
            </div>

            <!-- Contenu textuel -->
            <div class="min-w-0 space-y-1 flex-1">
              <div class="flex flex-wrap items-center gap-2">
                <h4
                  class="font-bold text-sm text-zinc-100 hover:text-amber-400 transition cursor-pointer truncate"
                  @click="openDetails(addon)"
                >
                  {{ addon.title }}
                </h4>

                <UiBadge variant="zinc" size="xs" class="font-mono text-[10px]">
                  v{{ addon.version }}
                </UiBadge>

                <!-- Badge sous-modules regroupés (type Modrinth) -->
                <button
                  v-if="addon.secondaryFolders && addon.secondaryFolders.length > 0"
                  type="button"
                  @click.stop="toggleExpand(addon.folderName)"
                  class="inline-flex items-center gap-1 px-2 py-0.5 rounded-md bg-zinc-800/90 hover:bg-zinc-700/90 border border-zinc-700/80 text-[10px] font-mono text-amber-300 transition"
                  :title="'Voir les ' + addon.secondaryFolders.length + ' sous-dossiers inclus'"
                >
                  <Layers class="w-2.5 h-2.5 text-amber-400" />
                  <span>+{{ addon.secondaryFolders.length }} sous-dossiers</span>
                  <ChevronDown
                    class="w-2.5 h-2.5 transition-transform"
                    :class="{ 'rotate-180': expandedSubmodules[addon.folderName] }"
                  />
                </button>

                <UiBadge
                  v-if="addon.category"
                  variant="amber"
                  size="xs"
                  class="font-mono text-[10px]"
                >
                  {{ addon.category }}
                </UiBadge>

                <span
                  v-if="addon.updateAvailable"
                  class="flex items-center gap-1 text-[10px] font-mono font-bold text-amber-400 bg-amber-500/10 px-2 py-0.5 rounded-full border border-amber-500/30 animate-pulse"
                >
                  MàJ v{{ addon.latestVersion }} dispo
                </span>
              </div>

              <!-- Notes / description courte -->
              <p v-if="addon.notes" class="text-xs text-zinc-400 line-clamp-1 leading-relaxed">
                {{ addon.notes }}
              </p>

              <!-- Métadonnées : dossier, auteur, taille -->
              <div class="flex flex-wrap items-center gap-3 text-[11px] font-mono text-zinc-500 pt-0.5">
                <span class="text-zinc-400 flex items-center gap-1">
                  <HardDrive class="w-3 h-3 text-zinc-500" />
                  {{ addon.sizeFormatted }}
                </span>

                <span v-if="addon.author">
                  Par <span class="text-zinc-300">{{ addon.author }}</span>
                </span>

                <span class="text-zinc-600">•</span>
                <span class="text-zinc-400">/{{ addon.folderName }}</span>

                <a
                  v-if="addon.website"
                  :href="addon.website"
                  target="_blank"
                  rel="noopener"
                  class="text-amber-400/70 hover:text-amber-400 inline-flex items-center gap-0.5 hover:underline"
                >
                  Site <ExternalLink class="w-2.5 h-2.5" />
                </a>
              </div>
            </div>
          </div>

          <!-- Actions droite : Switch activer + MàJ + Supprimer -->
          <div class="flex items-center gap-3 w-full md:w-auto justify-end pt-2 md:pt-0 border-t md:border-t-0 border-zinc-800/60 flex-shrink-0">
            <!-- Bouton Mise à jour dédié si dispo -->
            <button
              v-if="addon.updateAvailable"
              type="button"
              @click="handleUpdate(addon)"
              :disabled="wowStore.installingMap[addon.curseforgeId || 0]"
              class="flex items-center gap-1.5 px-3 py-1.5 rounded-xl bg-amber-500/20 hover:bg-amber-500/30 border border-amber-500/50 text-amber-300 font-mono font-bold text-xs transition disabled:opacity-50"
              title="Mettre à jour l'addon vers la dernière version"
            >
              <ArrowUpCircle
                class="w-3.5 h-3.5"
                :class="{ 'animate-spin': wowStore.installingMap[addon.curseforgeId || 0] }"
              />
              <span>Mettre à jour</span>
            </button>

            <!-- Toggle Activer / Désactiver -->
            <button
              type="button"
              @click="handleToggle(addon)"
              :disabled="wowStore.togglingMap[addon.folderName]"
              :class="[
                'relative inline-flex h-6 w-11 flex-shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none disabled:opacity-50',
                addon.enabled ? 'bg-amber-500' : 'bg-zinc-800'
              ]"
              :title="addon.enabled ? 'Désactiver l\'addon et ses sous-modules' : 'Activer l\'addon et ses sous-modules'"
            >
              <span
                :class="[
                  'pointer-events-none inline-block h-5 w-5 transform rounded-full bg-zinc-950 shadow ring-0 transition duration-200 ease-in-out',
                  addon.enabled ? 'translate-x-5' : 'translate-x-0'
                ]"
              />
            </button>

            <!-- Bouton Détails -->
            <button
              type="button"
              @click="openDetails(addon)"
              class="p-2 rounded-xl bg-zinc-800/80 hover:bg-zinc-700/80 text-zinc-400 hover:text-white transition"
              title="Afficher les détails et versions"
            >
              <Info class="w-4 h-4" />
            </button>

            <!-- Bouton Supprimer -->
            <button
              type="button"
              @click="handleUninstall(addon)"
              :disabled="wowStore.uninstallingMap[addon.folderName]"
              class="p-2 rounded-xl bg-zinc-800/80 hover:bg-rose-500/20 hover:text-rose-400 text-zinc-400 transition disabled:opacity-50"
              title="Désinstaller cet addon et ses sous-modules"
            >
              <Trash2 class="w-4 h-4" />
            </button>
          </div>
        </div>

        <!-- Section dépliable : sous-dossiers regroupés (Modrinth view) -->
        <div
          v-if="expandedSubmodules[addon.folderName] && addon.secondaryFolders && addon.secondaryFolders.length > 0"
          class="w-full pt-3 border-t border-zinc-800/80 space-y-2 animate-in fade-in duration-150"
        >
          <div class="flex items-center justify-between text-[11px] font-mono text-zinc-400">
            <span class="flex items-center gap-1.5 text-amber-400/90 font-semibold">
              <Layers class="w-3 h-3" />
              Dossiers secondaires gérés par ce paquet ({{ addon.secondaryFolders.length }}) :
            </span>
            <span class="text-zinc-500">Activés et mis à jour conjointement</span>
          </div>

          <div class="flex flex-wrap gap-1.5 pt-0.5">
            <span
              v-for="sub in addon.secondaryFolders"
              :key="sub"
              class="px-2 py-1 rounded-lg bg-zinc-950 border border-zinc-800/80 text-[10px] font-mono text-zinc-300 flex items-center gap-1 hover:border-zinc-700"
            >
              <span class="text-amber-500/70">/</span>{{ sub }}
            </span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
