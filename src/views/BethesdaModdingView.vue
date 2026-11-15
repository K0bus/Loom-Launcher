<script setup lang="ts">
import { onMounted, ref, computed, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import {
  ArrowLeft,
  Layers,
  AlertTriangle,
  FolderOpen,
  CheckCircle2,
  Trash2,
  ArrowUp,
  ArrowDown,
  Rocket,
  Cpu,
  FileCode2,
  Plus,
  KeyRound,
  DownloadCloud,
  ExternalLink,
  Edit3
} from 'lucide-vue-next'
import { useBethesdaStore, BETHESDA_GAMES } from '@/stores/bethesdaStore'
import { ipc } from '@/services/ipc'
import { safeBrowserOpen } from '@/utils/externalLinks'
import UiBadge from '@/components/ui/UiBadge.vue'
import UiButton from '@/components/ui/UiButton.vue'
import UiModal from '@/components/ui/UiModal.vue'

const route = useRoute()
const router = useRouter()
const store = useBethesdaStore()

const gameId = computed(() => (route.params.gameId as string) || 'skyrim_se')

// New profile modal
const isCreateProfileModalOpen = ref(false)
const newProfileName = ref('')
const newProfilePath = ref('')

// Nexus API Key Modal & NXM Events
const isNexusModalOpen = ref(false)
const nexusApiKeyInput = ref('')
const hasNexusApiKey = ref(false)
const pendingNxmUri = ref<string | null>(null)
const downloadStatusMessage = ref<string | null>(null)

onMounted(async () => {
  await store.loadGame(gameId.value)
  await checkNexusApiKey()

  ipc.listen('nxm-auth-required', (event: any) => {
    pendingNxmUri.value = event.payload?.uri || null
    isNexusModalOpen.value = true
  })

  ipc.listen('nxm-download-started', (event: any) => {
    downloadStatusMessage.value = `Téléchargement du mod Nexus #${event.payload?.mod_id || ''} en cours...`
  })

  ipc.listen('nxm-download-completed', (event: any) => {
    downloadStatusMessage.value = `Mod "${event.payload?.mod_name || 'Nexus'}" installé et déployé avec succès !`
    store.refreshActiveProfileData()
    setTimeout(() => {
      downloadStatusMessage.value = null
    }, 6000)
  })
})

async function checkNexusApiKey() {
  try {
    const key = await ipc.getNexusApiKeyStatus()
    hasNexusApiKey.value = !!key
  } catch (err) {
    hasNexusApiKey.value = false
  }
}

async function handleSaveNexusKey() {
  if (!nexusApiKeyInput.value.trim()) return
  await ipc.saveNexusApiKey(nexusApiKeyInput.value.trim())
  hasNexusApiKey.value = true
  isNexusModalOpen.value = false

  // If an NXM download was pending authentication, trigger it now
  if (pendingNxmUri.value) {
    const uri = pendingNxmUri.value
    pendingNxmUri.value = null
    await store.handleNxmUri(uri)
  }
}

function openNexusApiKeyWebpage() {
  safeBrowserOpen('https://www.nexusmods.com/users/myaccount?tab=api')
}

watch(
  () => route.params.gameId,
  async (newId) => {
    if (newId) {
      await store.loadGame(newId as string)
    }
  }
)

function switchGame(targetId: string) {
  router.push(`/modding/bethesda/${targetId}`)
}

async function handleCreateProfile() {
  if (!newProfileName.value.trim()) return
  await store.createProfile(newProfileName.value.trim(), newProfilePath.value.trim() || undefined)
  newProfileName.value = ''
  newProfilePath.value = ''
  isCreateProfileModalOpen.value = false
}

async function handleChangeDeploymentFolder() {
  const current = store.activeProfile?.deployment_path
  const chosen = await ipc.pickDirectory('Sélectionner le dossier du jeu', current)
  if (chosen) {
    await store.updateDeploymentPath(chosen)
    await store.deployActiveProfile()
  }
}

function movePlugin(index: number, direction: 'up' | 'down') {
  const current = [...store.plugins]
  const targetIndex = direction === 'up' ? index - 1 : index + 1
  if (targetIndex < 0 || targetIndex >= current.length) return

  const temp = current[index]
  current[index] = current[targetIndex]
  current[targetIndex] = temp

  const orderedNames = current.map((p) => p.filename)
  store.reorderPlugins(orderedNames)
}
</script>

<template>
  <div class="max-w-7xl mx-auto px-6 sm:px-10 lg:px-14 py-6 space-y-6 pb-28">
    <!-- 1. En-tête de navigation -->
    <div class="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4">
      <button
        type="button"
        @click="router.push('/modding')"
        class="inline-flex items-center gap-2 text-xs font-mono text-zinc-400 hover:text-amber-400 transition"
      >
        <ArrowLeft class="w-4 h-4" />
        <span>Retour au Hub Modding</span>
      </button>

      <!-- Sélecteur rapide de jeu Bethesda -->
      <div class="flex items-center gap-2 bg-zinc-900/90 p-1.5 rounded-2xl border border-zinc-800">
        <button
          v-for="(meta, gId) in BETHESDA_GAMES"
          :key="gId"
          type="button"
          @click="switchGame(gId)"
          class="px-3 py-1.5 rounded-xl text-xs font-mono font-semibold transition-all flex items-center gap-2"
          :class="[
            store.currentGameId === gId
              ? 'bg-amber-500 text-zinc-950 shadow-md font-bold'
              : 'text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800/60'
          ]"
        >
          <span>{{ meta.icon }}</span>
          <span>{{ gId === 'skyrim_se' ? 'Skyrim SE' : gId === 'fallout4' ? 'Fallout 4' : 'Oblivion' }}</span>
        </button>
      </div>
    </div>

    <!-- 2. Bannière Hero du jeu sélectionné -->
    <div
      class="relative overflow-hidden rounded-3xl bg-gradient-to-br border border-amber-500/40 p-6 sm:p-8 shadow-2xl transition-all"
      :class="store.currentGame.bannerGradient"
    >
      <div class="relative z-10 flex flex-col lg:flex-row items-start lg:items-center justify-between gap-6">
        <div class="flex items-start sm:items-center gap-5">
          <div class="w-20 h-20 rounded-2xl bg-zinc-900 border border-zinc-700/80 flex items-center justify-center text-4xl shadow-xl flex-shrink-0">
            {{ store.currentGame.icon }}
          </div>

          <div class="space-y-2">
            <div class="flex flex-wrap items-center gap-3">
              <h1 class="text-2xl sm:text-3xl font-black text-zinc-100 font-display">
                {{ store.currentGame.title }}
              </h1>
              <UiBadge variant="amber" size="xs" class="font-mono">
                MOTEUR GAMEBRYO / CREATION
              </UiBadge>
            </div>
            <p class="text-xs text-zinc-400 max-w-2xl leading-relaxed">
              {{ store.currentGame.subtitle }}
            </p>

            <div class="flex flex-wrap items-center gap-4 text-xs font-mono text-zinc-400 pt-1">
              <span class="flex items-center gap-1.5 bg-zinc-950/80 px-2.5 py-1 rounded-lg border border-zinc-800">
                <Cpu class="w-3.5 h-3.5 text-amber-400" />
                Loader : {{ store.currentGame.defaultExecutable }}
              </span>
              <button
                type="button"
                @click="handleChangeDeploymentFolder"
                class="flex items-center gap-1.5 bg-zinc-950/80 hover:bg-zinc-800 hover:border-amber-400 px-2.5 py-1 rounded-lg border border-zinc-800 transition cursor-pointer text-left group"
                title="Cliquer pour changer le dossier du jeu"
              >
                <FolderOpen class="w-3.5 h-3.5 text-zinc-400 group-hover:text-amber-400" />
                <span class="truncate max-w-xs">Dossier : {{ store.activeProfile?.deployment_path || 'Auto (Wine / Steam)' }}</span>
                <Edit3 class="w-3 h-3 text-zinc-500 group-hover:text-amber-400 ml-1" />
              </button>
            </div>
          </div>
        </div>

        <!-- Boutons d'action : Clé Nexus & Déploiement -->
        <div class="flex flex-wrap items-center gap-3 w-full lg:w-auto">
          <UiButton
            variant="secondary"
            size="lg"
            @click="isNexusModalOpen = true"
            class="font-mono text-xs border-zinc-700 hover:border-amber-400"
          >
            <template #prefix>
              <KeyRound class="w-4 h-4 text-amber-400" />
            </template>
            {{ hasNexusApiKey ? 'Clé Nexus Connectée' : 'Lier Clé Nexus' }}
          </UiButton>

          <UiButton
            variant="primary"
            size="lg"
            :loading="store.isDeploying"
            @click="store.deployActiveProfile"
            class="shadow-xl shadow-amber-500/20 font-mono font-bold"
          >
            <template #prefix>
              <Rocket class="w-4 h-4" />
            </template>
            Déployer le Profil
          </UiButton>
        </div>
      </div>

      <!-- Messages de feedback déploiement & téléchargement -->
      <div v-if="downloadStatusMessage" class="mt-4 p-3 rounded-xl bg-amber-500/20 border border-amber-500/40 text-amber-300 text-xs font-mono flex items-center gap-2 animate-pulse">
        <DownloadCloud class="w-4 h-4 text-amber-400" />
        <span>{{ downloadStatusMessage }}</span>
      </div>
      <div v-if="store.deploySuccessMessage" class="mt-4 p-3 rounded-xl bg-emerald-500/20 border border-emerald-500/40 text-emerald-300 text-xs font-mono flex items-center gap-2">
        <CheckCircle2 class="w-4 h-4 text-emerald-400" />
        <span>{{ store.deploySuccessMessage }}</span>
      </div>
      <div v-if="store.deployErrorMessage" class="mt-4 p-3 rounded-xl bg-rose-500/20 border border-rose-500/40 text-rose-300 text-xs font-mono flex items-center gap-2">
        <AlertTriangle class="w-4 h-4 text-rose-400" />
        <span>{{ store.deployErrorMessage }}</span>
      </div>
    </div>

    <!-- 3. Barre de Profil & Onglets -->
    <div class="flex flex-col md:flex-row items-stretch md:items-center justify-between gap-4 bg-zinc-900/60 p-3 rounded-2xl border border-zinc-800">
      <!-- Sélecteur de Profil Actif -->
      <div class="flex items-center gap-3">
        <span class="text-xs font-mono text-zinc-400 font-semibold uppercase">Profil :</span>
        <select
          v-model="store.activeProfileId"
          @change="store.selectProfile(store.activeProfileId)"
          class="bg-zinc-950 text-zinc-200 text-xs font-mono px-3 py-1.5 rounded-xl border border-zinc-700 focus:outline-none focus:border-amber-400"
        >
          <option v-for="p in store.profiles" :key="p.id" :value="p.id">
            {{ p.name }}
          </option>
        </select>
        <button
          type="button"
          @click="isCreateProfileModalOpen = true"
          class="p-1.5 rounded-xl bg-zinc-800 hover:bg-zinc-700 text-zinc-300 hover:text-amber-400 transition"
          title="Nouveau profil"
        >
          <Plus class="w-4 h-4" />
        </button>
      </div>

      <!-- Onglets du Studio -->
      <div class="flex items-center gap-2 overflow-x-auto">
        <button
          type="button"
          @click="store.activeTab = 'mods'"
          class="px-4 py-2 rounded-xl text-xs font-mono font-semibold transition flex items-center gap-2"
          :class="[
            store.activeTab === 'mods'
              ? 'bg-amber-500/20 text-amber-300 border border-amber-500/40'
              : 'text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800/40'
          ]"
        >
          <Layers class="w-4 h-4" />
          <span>Mods Installés ({{ store.enabledModsCount }})</span>
        </button>

        <button
          type="button"
          @click="store.activeTab = 'conflicts'"
          class="px-4 py-2 rounded-xl text-xs font-mono font-semibold transition flex items-center gap-2"
          :class="[
            store.activeTab === 'conflicts'
              ? 'bg-amber-500/20 text-amber-300 border border-amber-500/40'
              : 'text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800/40'
          ]"
        >
          <AlertTriangle class="w-4 h-4 text-amber-400" />
          <span>Conflits ({{ store.conflictsCount }})</span>
        </button>

        <button
          type="button"
          @click="store.activeTab = 'plugins'"
          class="px-4 py-2 rounded-xl text-xs font-mono font-semibold transition flex items-center gap-2"
          :class="[
            store.activeTab === 'plugins'
              ? 'bg-amber-500/20 text-amber-300 border border-amber-500/40'
              : 'text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800/40'
          ]"
        >
          <FileCode2 class="w-4 h-4" />
          <span>Load Order ({{ store.activePluginsCount }})</span>
        </button>
      </div>
    </div>

    <!-- 4. CONTENU DES ONGLETS -->

    <!-- ONGLET 1 : MODS INSTALLÉS -->
    <div v-if="store.activeTab === 'mods'" class="space-y-4">
      <div v-if="store.mods.length === 0" class="text-center py-16 bg-zinc-900/30 rounded-3xl border border-zinc-800/80 p-8 space-y-4">
        <div class="w-16 h-16 rounded-2xl bg-zinc-800/60 flex items-center justify-center mx-auto text-3xl">
          📦
        </div>
        <div class="space-y-1">
          <h3 class="text-lg font-bold text-zinc-200">Aucun mod installé dans ce profil</h3>
          <p class="text-xs text-zinc-400 max-w-md mx-auto">
            Téléchargez des mods depuis Nexus Mods via le lien <span class="font-mono text-amber-400">nxm://</span> ou glissez une archive dans le dossier de staging du profil.
          </p>
        </div>
      </div>

      <div v-else class="space-y-2">
        <div
          v-for="pmod in store.mods"
          :key="pmod.id"
          class="flex items-center justify-between p-4 rounded-2xl bg-zinc-900/60 border border-zinc-800 hover:border-zinc-700 transition gap-4"
        >
          <div class="flex items-center gap-4 min-w-0">
            <!-- Interrupteur d'activation -->
            <input
              type="checkbox"
              :checked="pmod.is_enabled"
              @change="store.toggleMod(pmod.id, !pmod.is_enabled)"
              class="w-4 h-4 accent-amber-500 rounded cursor-pointer"
            />

            <!-- Priorité d'ordre -->
            <span class="w-8 text-center text-xs font-mono font-bold text-zinc-500 bg-zinc-950 px-2 py-1 rounded-lg">
              #{{ pmod.load_priority }}
            </span>

            <div class="min-w-0">
              <h4 class="text-sm font-bold text-zinc-100 truncate" :class="{ 'line-through text-zinc-500': !pmod.is_enabled }">
                {{ pmod.name }}
              </h4>
              <p class="text-[11px] font-mono text-zinc-400 truncate">
                {{ pmod.filename || pmod.file_path }}
              </p>
            </div>
          </div>

          <div class="flex items-center gap-3 flex-shrink-0">
            <UiBadge variant="zinc" size="xs" class="font-mono uppercase">
              {{ pmod.provider }}
            </UiBadge>

            <!-- Reorder buttons -->
            <div class="flex items-center gap-1">
              <button
                type="button"
                @click="store.reorderMod(pmod.id, pmod.load_priority - 1)"
                class="p-1 rounded-lg bg-zinc-800 hover:bg-zinc-700 text-zinc-300"
                title="Augmenter la priorité"
              >
                <ArrowUp class="w-3.5 h-3.5" />
              </button>
              <button
                type="button"
                @click="store.reorderMod(pmod.id, pmod.load_priority + 1)"
                class="p-1 rounded-lg bg-zinc-800 hover:bg-zinc-700 text-zinc-300"
                title="Diminuer la priorité"
              >
                <ArrowDown class="w-3.5 h-3.5" />
              </button>
            </div>

            <!-- Delete button -->
            <button
              type="button"
              @click="store.deleteMod(pmod.id)"
              class="p-1.5 rounded-lg text-zinc-400 hover:text-rose-400 hover:bg-rose-500/10 transition"
              title="Supprimer ce mod"
            >
              <Trash2 class="w-4 h-4" />
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- ONGLET 2 : CONFLITS DE FICHIERS -->
    <div v-if="store.activeTab === 'conflicts'" class="space-y-4">
      <div v-if="store.conflicts.length === 0" class="text-center py-16 bg-zinc-900/30 rounded-3xl border border-zinc-800/80 p-8 space-y-3">
        <CheckCircle2 class="w-12 h-12 text-emerald-400 mx-auto" />
        <h3 class="text-lg font-bold text-zinc-200">Aucun conflit de fichiers détecté</h3>
        <p class="text-xs text-zinc-400 max-w-md mx-auto">
          Tous les fichiers des mods actifs possèdent des chemins relatifs distincts dans le répertoire <span class="font-mono text-amber-400">Data/</span>.
        </p>
      </div>

      <div v-else class="space-y-3">
        <div class="text-xs font-mono text-zinc-400 flex items-center justify-between pb-2">
          <span>{{ store.conflicts.length }} collision(s) identifiée(s) entre vos mods</span>
          <span class="text-amber-400">Sélectionnez le mod prioritaire pour chaque fichier</span>
        </div>

        <div
          v-for="conflict in store.conflicts"
          :key="conflict.relative_path"
          class="p-4 rounded-2xl bg-zinc-900/70 border border-zinc-800 space-y-3"
        >
          <div class="flex items-center gap-2">
            <AlertTriangle class="w-4 h-4 text-amber-400 flex-shrink-0" />
            <span class="text-xs font-mono font-bold text-zinc-200 break-all">
              {{ conflict.relative_path }}
            </span>
          </div>

          <!-- Liste des mods en collision -->
          <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-2 pt-1">
            <button
              v-for="claimant in conflict.conflicting_mods"
              :key="claimant.mod_id"
              type="button"
              @click="store.setConflictWinner(conflict.relative_path, claimant.mod_id)"
              class="p-2.5 rounded-xl border text-left text-xs font-mono transition flex items-center justify-between gap-2"
              :class="[
                conflict.current_winner_mod_id === claimant.mod_id
                  ? 'bg-amber-500/20 border-amber-500/60 text-amber-300 font-bold'
                  : 'bg-zinc-950/60 border-zinc-800 text-zinc-400 hover:border-zinc-700'
              ]"
            >
              <div class="truncate">
                <div>{{ claimant.mod_name }}</div>
                <div class="text-[10px] text-zinc-500">Priorité #{{ claimant.load_priority }}</div>
              </div>

              <span
                v-if="conflict.current_winner_mod_id === claimant.mod_id"
                class="px-1.5 py-0.5 rounded bg-amber-500 text-zinc-950 text-[10px] font-black uppercase"
              >
                Gagnant
              </span>
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- ONGLET 3 : LOAD ORDER (PLUGINS.TXT) -->
    <div v-if="store.activeTab === 'plugins'" class="space-y-4">
      <div v-if="store.plugins.length === 0" class="text-center py-16 bg-zinc-900/30 rounded-3xl border border-zinc-800/80 p-8 space-y-3">
        <FileCode2 class="w-12 h-12 text-zinc-600 mx-auto" />
        <h3 class="text-lg font-bold text-zinc-200">Aucun plugin (.esm, .esl, .esp) trouvé</h3>
        <p class="text-xs text-zinc-400 max-w-md mx-auto">
          Les plugins seront scannés automatiquement lors de l'ajout de mods contenant des extensions Gamebryo.
        </p>
      </div>

      <div v-else class="space-y-2">
        <div class="text-xs font-mono text-zinc-400 flex items-center justify-between pb-2">
          <span>Ordre de chargement des plugins (synchronisé avec plugins.txt)</span>
          <span class="text-zinc-500">Ordre recommandé : Master (.esm) > Light (.esl) > Standard (.esp)</span>
        </div>

        <div
          v-for="(plugin, idx) in store.plugins"
          :key="plugin.filename"
          class="flex items-center justify-between p-3.5 rounded-2xl bg-zinc-900/60 border border-zinc-800 gap-4"
        >
          <div class="flex items-center gap-3 min-w-0">
            <input
              type="checkbox"
              :checked="plugin.is_active"
              @change="store.togglePlugin(plugin.filename, !plugin.is_active)"
              class="w-4 h-4 accent-amber-500 rounded cursor-pointer"
            />

            <span class="w-8 text-center text-xs font-mono font-bold text-zinc-500 bg-zinc-950 px-2 py-1 rounded-lg">
              #{{ idx + 1 }}
            </span>

            <span class="text-xs font-mono font-bold text-zinc-200 truncate">
              {{ plugin.filename }}
            </span>
          </div>

          <div class="flex items-center gap-2 flex-shrink-0">
            <!-- Badge type -->
            <UiBadge
              :variant="plugin.filename.toLowerCase().endsWith('.esm') ? 'amber' : plugin.filename.toLowerCase().endsWith('.esl') ? 'emerald' : 'zinc'"
              size="xs"
              class="font-mono uppercase"
            >
              {{ plugin.filename.toLowerCase().endsWith('.esm') ? 'Master' : plugin.filename.toLowerCase().endsWith('.esl') ? 'Light' : 'Plugin' }}
            </UiBadge>

            <!-- Move Up / Down -->
            <button
              type="button"
              @click="movePlugin(idx, 'up')"
              :disabled="idx === 0"
              class="p-1 rounded-lg bg-zinc-800 hover:bg-zinc-700 text-zinc-300 disabled:opacity-40"
              title="Monter"
            >
              <ArrowUp class="w-3.5 h-3.5" />
            </button>
            <button
              type="button"
              @click="movePlugin(idx, 'down')"
              :disabled="idx === store.plugins.length - 1"
              class="p-1 rounded-lg bg-zinc-800 hover:bg-zinc-700 text-zinc-300 disabled:opacity-40"
              title="Descendre"
            >
              <ArrowDown class="w-3.5 h-3.5" />
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- Modal Nouveau Profil -->
    <UiModal
      :open="isCreateProfileModalOpen"
      @close="isCreateProfileModalOpen = false"
      title="Créer un Nouveau Profil de Modding"
      maxWidth="md"
    >
      <div class="space-y-4">
        <div>
          <label class="block text-xs font-mono text-zinc-400 mb-1">Nom du Profil</label>
          <input
            v-model="newProfileName"
            type="text"
            placeholder="Ex: Skyrim Modded Ultra 2026"
            class="w-full bg-zinc-950 text-zinc-200 text-xs font-mono px-3 py-2 rounded-xl border border-zinc-800 focus:outline-none focus:border-amber-400"
          />
        </div>

        <div>
          <label class="block text-xs font-mono text-zinc-400 mb-1">Chemin d'installation du Jeu (Optionnel)</label>
          <input
            v-model="newProfilePath"
            type="text"
            placeholder="Ex: /home/user/.steam/steam/steamapps/common/Skyrim Special Edition"
            class="w-full bg-zinc-950 text-zinc-200 text-xs font-mono px-3 py-2 rounded-xl border border-zinc-800 focus:outline-none focus:border-amber-400"
          />
          <p class="text-[11px] text-zinc-500 mt-1">Laissez vide pour utiliser la détection automatique Wine / Proton.</p>
        </div>

        <div class="flex justify-end gap-3 pt-2">
          <UiButton variant="secondary" size="sm" @click="isCreateProfileModalOpen = false">
            Annuler
          </UiButton>
          <UiButton variant="primary" size="sm" @click="handleCreateProfile">
            Créer le Profil
          </UiButton>
        </div>
      </div>
    </UiModal>

    <!-- Modal Clé API Nexus Mods -->
    <UiModal
      :open="isNexusModalOpen"
      @close="isNexusModalOpen = false"
      title="Authentification Nexus Mods (Protocole NXM)"
      maxWidth="md"
    >
      <div class="space-y-4">
        <p class="text-xs text-zinc-300 leading-relaxed">
          Pour permettre à Loom de télécharger automatiquement les mods lorsque vous cliquez sur un lien <span class="font-mono text-amber-400">nxm://</span> sur Nexus Mods, renseignez votre clé API personnelle.
        </p>

        <div class="p-3.5 rounded-xl bg-zinc-950 border border-zinc-800 space-y-2">
          <div class="text-xs font-mono font-semibold text-zinc-300 flex items-center justify-between">
            <span>Comment obtenir votre clé gratuite ?</span>
            <button
              type="button"
              @click="openNexusApiKeyWebpage"
              class="text-amber-400 hover:underline flex items-center gap-1 text-[11px]"
            >
              <span>Ouvrir Nexus Mods</span>
              <ExternalLink class="w-3 h-3" />
            </button>
          </div>
          <p class="text-[11px] text-zinc-400 leading-snug">
            1. Connectez-vous sur Nexus Mods.<br />
            2. Descendez en bas de la page à la section <strong>Personal API Key</strong>.<br />
            3. Cliquez sur <strong>Request an API Key</strong> et collez-la ci-dessous.
          </p>
        </div>

        <div>
          <label class="block text-xs font-mono text-zinc-400 mb-1">Clé API Personnelle Nexus Mods</label>
          <input
            v-model="nexusApiKeyInput"
            type="password"
            placeholder="Collez votre clé API Nexus Mods ici"
            class="w-full bg-zinc-950 text-zinc-200 text-xs font-mono px-3 py-2 rounded-xl border border-zinc-800 focus:outline-none focus:border-amber-400"
          />
        </div>

        <div class="flex justify-end gap-3 pt-2">
          <UiButton variant="secondary" size="sm" @click="isNexusModalOpen = false">
            Annuler
          </UiButton>
          <UiButton variant="primary" size="sm" @click="handleSaveNexusKey">
            Enregistrer la Clé
          </UiButton>
        </div>
      </div>
    </UiModal>
  </div>
</template>
