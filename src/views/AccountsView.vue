<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import {
  UserCheck,
  Link,
  Unlink,
  CheckCircle2,
  FolderSync,
  HardDrive,
  RefreshCw,
  FolderPlus,
  KeyRound,
  Gamepad2
} from 'lucide-vue-next'
import { useAccountsStore } from '@/stores/accountsStore'
import { useBattleNetStore } from '@/stores/useBattleNetStore'
import { useToastStore } from '@/stores/toastStore'
import { useGamesStore } from '@/stores/gamesStore'
import { useSystemStore } from '@/stores/systemStore'
import {
  PlatformIcon,
  SteamIcon,
  EpicIcon,
  XboxIcon,
  GogIcon,
  BattleNetIcon
} from '@/components/icons/platforms'
import { steamService } from '@/services/steamService'
import UiButton from '@/components/ui/UiButton.vue'
import UiBadge from '@/components/ui/UiBadge.vue'
import UiModal from '@/components/ui/UiModal.vue'
import BlizzardAuthModal from '@/components/battlenet/BlizzardAuthModal.vue'

const accountsStore = useAccountsStore()
const bnetStore = useBattleNetStore()
const gamesStore = useGamesStore()
const systemStore = useSystemStore()
const toast = useToastStore()

const isSyncingSteam = ref(false)

const realSteamAccount = computed(() => steamService.getDetectedAccount())
const showSteamGamesList = ref(false)
const steamGamesTab = ref<'installed' | 'uninstalled'>('installed')

const showExternalModal = ref(false)
const externalPathInput = ref('')
const isAddingExternal = ref(false)
const showBnetGamesList = ref(false)
const bnetGamesTab = ref<'installed' | 'owned'>('installed')
const bnetAvatarError = ref(false)

watch(() => bnetStore.avatarUrl, () => {
  bnetAvatarError.value = false
})

async function handleAddExternalGame() {
  if (!externalPathInput.value.trim()) return
  isAddingExternal.value = true
  try {
    await bnetStore.addExternalGame(externalPathInput.value.trim())
    await accountsStore.syncBattleNetAccount()
    externalPathInput.value = ''
    showExternalModal.value = false
  } catch {} finally {
    isAddingExternal.value = false
  }
}

async function handleScanBnet() {
  await bnetStore.scanLocalGames()
  await accountsStore.syncBattleNetAccount()
}

async function handleBattleNetAction(isConnected: boolean) {
  if (isConnected) {
    await bnetStore.logoutBlizzard()
    await accountsStore.syncBattleNetAccount()
  } else {
    if (!bnetStore.credentialsConfigured) {
      bnetStore.openAuthModal()
    } else {
      try {
        await bnetStore.loginBlizzard()
        await accountsStore.syncBattleNetAccount()
      } catch {}
    }
  }
}

interface PlatformDetails {
  name: string
  provider: string
  clientPath?: string
  description: string
  colorClass: string
  bgLight: string
  borderColor: string
}

const platformDetails: Record<string, PlatformDetails> = {
  steam: {
    name: 'Steam',
    provider: 'Valve Corporation',
    clientPath: '~/.local/share/Steam',
    description: 'Client Steam natif détecté avec manifeste ACF des jeux installés.',
    colorClass: 'text-[#1a9fff]',
    bgLight: 'bg-[#1a9fff]/10',
    borderColor: 'border-[#1a9fff]/30'
  },
  epic: {
    name: 'Epic Games',
    provider: 'Epic Games, Inc.',
    description: 'Aucune installation ou session active détectée.',
    colorClass: 'text-white',
    bgLight: 'bg-zinc-800/80',
    borderColor: 'border-zinc-700'
  },
  xbox: {
    name: 'Xbox / Microsoft',
    provider: 'Microsoft Gaming',
    description: 'Aucune session Minecraft ou profil Xbox Live connecté.',
    colorClass: 'text-[#107c10]',
    bgLight: 'bg-[#107c10]/10',
    borderColor: 'border-[#107c10]/30'
  },
  microsoft: {
    name: 'Xbox / Microsoft',
    provider: 'Microsoft Gaming',
    description: 'Aucune session Minecraft ou profil Xbox Live connecté.',
    colorClass: 'text-[#107c10]',
    bgLight: 'bg-[#107c10]/10',
    borderColor: 'border-[#107c10]/30'
  },
  gog: {
    name: 'GOG Galaxy',
    provider: 'CD PROJEKT',
    description: 'Aucun compte GOG.com lié actuellement.',
    colorClass: 'text-[#b44aff]',
    bgLight: 'bg-[#86328a]/15',
    borderColor: 'border-[#86328a]/35'
  },
  battlenet: {
    name: 'Battle.net',
    provider: 'Blizzard Entertainment',
    description: 'Gestion du préfixe Wine dédié (~/.local/share/loom/prefixes/battlenet/) et authentification OAuth2 Blizzard.',
    colorClass: 'text-[#00aeff]',
    bgLight: 'bg-[#00aeff]/10',
    borderColor: 'border-[#00aeff]/30'
  }
}

function getDetails(platform: string): PlatformDetails {
  return (
    platformDetails[platform] || {
      name: platform,
      provider: 'Plateforme',
      description: 'Non configuré.',
      colorClass: 'text-zinc-300',
      bgLight: 'bg-zinc-800',
      borderColor: 'border-zinc-700'
    }
  )
}

async function handleSyncSteam() {
  isSyncingSteam.value = true
  try {
    await gamesStore.fetchLibrary()
    await systemStore.refreshStats()
    await gamesStore.syncAchievementSummaries()
    toast.show(`${realSteamAccount.value.installedGames.length} jeux Steam réels synchronisés avec succès !`, 'success')
  } catch (err: any) {
    toast.show(`Erreur lors de la synchronisation Steam : ${err?.message || err}`, 'error')
  } finally {
    isSyncingSteam.value = false
  }
}
</script>

<template>
  <div class="max-w-7xl mx-auto px-6 sm:px-10 lg:px-14 py-6 space-y-8 pb-16">
    <!-- Header -->
    <div class="flex flex-col md:flex-row items-start md:items-center justify-between gap-4">
      <div>
        <h1 class="text-3xl font-black text-zinc-100 tracking-tight font-display flex items-center gap-3">
          <UserCheck class="w-8 h-8 text-amber-400" />
          Hub de Plateformes & Comptes
        </h1>
        <p class="text-xs text-zinc-400 mt-1">
          Détection réelle des clients et liaisons officielles sans données fictives
        </p>
      </div>

      <!-- Quick Platform Badge Pill Strip -->
      <div class="flex items-center gap-2 p-1.5 rounded-2xl bg-zinc-900/80 border border-zinc-800/80 backdrop-blur-md">
        <div class="px-2.5 py-1 text-[11px] font-mono text-zinc-400 font-semibold uppercase tracking-wider">
          Plateformes
        </div>
        <div class="flex items-center gap-1.5 pr-2">
          <span class="p-1.5 rounded-lg bg-zinc-950/80 border border-zinc-800" title="Steam">
            <SteamIcon className="w-4 h-4" :colored="true" />
          </span>
          <span class="p-1.5 rounded-lg bg-zinc-950/80 border border-zinc-800" title="Epic Games">
            <EpicIcon className="w-4 h-4" :colored="true" />
          </span>
          <span class="p-1.5 rounded-lg bg-zinc-950/80 border border-zinc-800" title="Xbox">
            <XboxIcon className="w-4 h-4" :colored="true" />
          </span>
          <span class="p-1.5 rounded-lg bg-zinc-950/80 border border-zinc-800" title="GOG Galaxy">
            <GogIcon className="w-4 h-4" :colored="true" />
          </span>
          <span class="p-1.5 rounded-lg bg-zinc-950/80 border border-zinc-800" title="Battle.net">
            <BattleNetIcon className="w-4 h-4" :colored="true" />
          </span>
        </div>
      </div>
    </div>

    <!-- Accounts Cards Grid (5 Supported Platforms) -->
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
      <div
        v-for="acc in accountsStore.accounts"
        :key="acc.id"
        class="relative overflow-hidden p-6 rounded-2xl bg-zinc-900/60 border border-zinc-800/80 hover:border-zinc-700/80 transition-all space-y-5 shadow-lg flex flex-col justify-between group"
      >
        <!-- Background Platform SVG (quart haut gauche, en plus clair / watermark, un peu de biais) -->
        <div
          class="absolute -top-6 -left-6 w-36 h-36 md:w-40 md:h-40 pointer-events-none select-none text-white/[0.08] group-hover:text-white/[0.16] -rotate-12 group-hover:-rotate-6 group-hover:scale-105 transition-all duration-500 ease-out z-0"
          aria-hidden="true"
        >
          <PlatformIcon
            :platform="acc.platform"
            class-name="w-full h-full"
            :colored="false"
          />
        </div>

        <div class="relative z-10 space-y-4">
          <!-- Card Header : SVG Logo + Platform Name + Status Badge -->
          <div class="flex items-start justify-between gap-3">
            <div class="flex items-center gap-3.5">
              <div class="relative">
                <div
                  v-if="acc.platform === 'battlenet' && bnetStore.avatarUrl && !bnetAvatarError"
                  class="w-12 h-12 rounded-xl overflow-hidden border border-amber-500/40 shadow-inner flex-shrink-0 bg-zinc-950"
                >
                  <img
                    :src="bnetStore.avatarUrl"
                    :alt="bnetStore.battletag"
                    class="w-full h-full object-cover"
                    @error="bnetAvatarError = true"
                  />
                </div>
                <div
                  v-else
                  :class="[
                    'w-12 h-12 rounded-xl flex items-center justify-center border flex-shrink-0 shadow-inner transition-transform group-hover:scale-105',
                    getDetails(acc.platform).bgLight,
                    getDetails(acc.platform).borderColor
                  ]"
                >
                  <PlatformIcon
                    :platform="acc.platform"
                    class-name="w-6 h-6"
                    :colored="true"
                  />
                </div>

                <!-- Mini Status Indicator if connected -->
                <span
                  v-if="acc.connected"
                  class="absolute -bottom-1 -right-1 w-4 h-4 rounded-full bg-emerald-500 border-2 border-zinc-900 flex items-center justify-center text-zinc-950"
                  title="Détecté"
                >
                  <CheckCircle2 class="w-3 h-3 text-white" />
                </span>
              </div>

              <!-- Names & Provider -->
              <div class="min-w-0">
                <div class="flex items-center gap-1.5">
                  <h3 class="font-bold text-sm text-zinc-100 truncate">
                    {{ acc.connected && acc.displayName ? acc.displayName : getDetails(acc.platform).name }}
                  </h3>
                  <PlatformIcon
                    :platform="acc.platform"
                    class-name="w-3.5 h-3.5 flex-shrink-0"
                    :colored="true"
                  />
                </div>
                <p class="text-[11px] text-zinc-400 font-mono truncate">
                  {{ acc.connected && acc.username ? `@${acc.username}` : getDetails(acc.platform).provider }}
                </p>
              </div>
            </div>

            <!-- Connection Status Badge -->
            <UiBadge
              :variant="acc.connected ? 'emerald' : 'zinc'"
              size="sm"
              :dot="acc.connected"
            >
              {{ acc.connected ? 'Détecté' : 'Non lié' }}
            </UiBadge>
          </div>

          <!-- Description text -->
          <p class="text-xs text-zinc-400 leading-relaxed">
            {{ getDetails(acc.platform).description }}
          </p>

          <!-- Real Local Steam Details -->
          <div v-if="acc.platform === 'steam' && acc.connected" class="space-y-3">
            <div class="p-3 rounded-xl bg-zinc-950/70 border border-zinc-800/70 space-y-2 text-xs">
              <div class="flex items-center justify-between">
                <span class="text-zinc-500 flex items-center gap-1.5 font-medium">
                  <HardDrive class="w-3.5 h-3.5 text-amber-400" />
                  Répertoire local :
                </span>
                <span class="text-zinc-300 font-mono text-[11px] truncate max-w-[170px]">
                  ~/.local/share/Steam
                </span>
              </div>

              <div class="flex items-center justify-between">
                <span class="text-zinc-500 font-medium">Jeux installés :</span>
                <span class="text-emerald-400 font-bold font-mono">
                  {{ realSteamAccount.installedCount }} jeux locaux
                </span>
              </div>

              <div class="flex items-center justify-between">
                <span class="text-zinc-500 font-medium">Bibliothèque totale :</span>
                <span class="text-amber-400 font-bold font-mono">
                  {{ realSteamAccount.totalGamesCount }} jeux ({{ realSteamAccount.uninstalledCount }} non installés)
                </span>
              </div>

              <div class="flex items-center justify-between">
                <span class="text-zinc-500 font-medium">SteamID64 :</span>
                <span class="text-zinc-400 font-mono text-[11px]">
                  {{ realSteamAccount.steamId64 }}
                </span>
              </div>
            </div>

            <!-- Toggle View Real Steam Games (Installed vs Uninstalled) -->
            <button
              type="button"
              @click="showSteamGamesList = !showSteamGamesList"
              class="w-full text-left text-[11px] text-amber-400 hover:text-amber-300 font-medium flex items-center justify-between px-1"
            >
              <span>{{ showSteamGamesList ? 'Masquer la liste' : `Parcourir les ${realSteamAccount.totalGamesCount} jeux détectés` }}</span>
              <i :class="showSteamGamesList ? 'ph ph-caret-up' : 'ph ph-caret-down'"></i>
            </button>

            <!-- Real games list with Installed / Uninstalled tabs -->
            <div
              v-if="showSteamGamesList"
              class="space-y-2 pt-1"
            >
              <div class="flex items-center gap-1.5 p-1 rounded-lg bg-zinc-950 border border-zinc-800 text-[11px]">
                <button
                  type="button"
                  @click="steamGamesTab = 'installed'"
                  :class="[
                    'flex-1 py-1 rounded font-medium transition-colors text-center',
                    steamGamesTab === 'installed'
                      ? 'bg-emerald-500/20 text-emerald-300 border border-emerald-500/40'
                      : 'text-zinc-400 hover:text-zinc-200'
                  ]"
                >
                  Installés ({{ realSteamAccount.installedCount }})
                </button>
                <button
                  type="button"
                  @click="steamGamesTab = 'uninstalled'"
                  :class="[
                    'flex-1 py-1 rounded font-medium transition-colors text-center',
                    steamGamesTab === 'uninstalled'
                      ? 'bg-amber-500/20 text-amber-300 border border-amber-500/40'
                      : 'text-zinc-400 hover:text-zinc-200'
                  ]"
                >
                  Non installés ({{ realSteamAccount.uninstalledCount }})
                </button>
              </div>

              <div class="max-h-48 overflow-y-auto space-y-1.5 pr-1 no-scrollbar">
                <div
                  v-for="game in (steamGamesTab === 'installed' ? realSteamAccount.installedGames : realSteamAccount.uninstalledGames)"
                  :key="game.appid"
                  class="flex items-center justify-between p-2 rounded-lg bg-zinc-950/50 border border-zinc-800/50 text-[11px]"
                >
                  <span class="text-zinc-200 truncate font-medium">{{ game.name }}</span>
                  <span v-if="game.installed && game.sizeGb > 0" class="text-emerald-400 font-mono text-[10px] flex-shrink-0 ml-2">
                    {{ game.sizeGb }} Go
                  </span>
                  <span v-else-if="game.playtimeMinutes > 0" class="text-zinc-500 font-mono text-[10px] flex-shrink-0 ml-2">
                    {{ (game.playtimeMinutes / 60).toFixed(1) }} h
                  </span>
                  <span v-else class="text-zinc-600 font-mono text-[10px] flex-shrink-0 ml-2">
                    Non installé
                  </span>
                </div>
              </div>
            </div>
          </div>

          <!-- Real Battle.net Details (Wine prefix, games scanner, external folder) -->
          <div v-if="acc.platform === 'battlenet'" class="space-y-3">
            <div class="p-3 rounded-xl bg-zinc-950/70 border border-zinc-800/70 space-y-2 text-xs">
              <div class="flex items-center justify-between">
                <span class="text-zinc-500 flex items-center gap-1.5 font-medium">
                  <HardDrive class="w-3.5 h-3.5 text-amber-400" />
                  Préfixe Wine géré :
                </span>
                <span class="text-zinc-300 font-mono text-[11px] truncate max-w-[170px]" title="~/.local/share/loom/prefixes/battlenet/">
                  {{ bnetStore.config?.wine_prefix_path || '~/.local/share/loom/prefixes/battlenet/' }}
                </span>
              </div>

              <div class="flex items-center justify-between">
                <span class="text-zinc-500 font-medium">Exécutable Runner :</span>
                <span class="text-amber-400 font-mono text-[11px]">
                  {{ bnetStore.config?.runner_executable || 'wine' }} (64-bit)
                </span>
              </div>

              <div class="flex items-center justify-between">
                <span class="text-zinc-500 font-medium">Jeux détectés en local :</span>
                <span class="text-emerald-400 font-bold font-mono">
                  {{ bnetStore.installedCount }} jeux
                </span>
              </div>

              <div class="flex items-center justify-between">
                <span class="text-zinc-500 font-medium">Licences Blizzard (Cloud) :</span>
                <span class="text-amber-400 font-bold font-mono">
                  {{ bnetStore.ownedGames.length }} jeux possédés
                </span>
              </div>

              <div class="flex items-center justify-between pt-1 border-t border-zinc-800/60 text-[11px]">
                <span class="text-zinc-500">Optimisations :</span>
                <span class="text-zinc-300 font-mono">
                  {{ bnetStore.config?.enable_gamemode ? 'GameMode' : '' }}
                  {{ bnetStore.config?.enable_gamemode && bnetStore.config?.enable_mangohud ? ' • ' : '' }}
                  {{ bnetStore.config?.enable_mangohud ? 'MangoHud' : '' }}
                  {{ !bnetStore.config?.enable_gamemode && !bnetStore.config?.enable_mangohud ? 'Par défaut' : '' }}
                </span>
              </div>
            </div>

            <!-- Actions Row: Scanner les jeux locaux & Ajouter dossier externe -->
            <div class="grid grid-cols-2 gap-2">
              <button
                type="button"
                @click="handleScanBnet"
                :disabled="bnetStore.isScanning"
                class="px-3 py-2 rounded-xl bg-zinc-950/80 hover:bg-zinc-800 border border-zinc-800 hover:border-amber-500/40 text-zinc-300 hover:text-amber-300 text-[11px] font-medium flex items-center justify-center gap-1.5 transition-colors"
                title="Scanner product.db et les manifests .build.info"
              >
                <RefreshCw :class="['w-3.5 h-3.5 text-amber-400', bnetStore.isScanning ? 'animate-spin' : '']" />
                <span>{{ bnetStore.isScanning ? 'Scan...' : 'Scanner local' }}</span>
              </button>

              <button
                type="button"
                @click="showExternalModal = true"
                class="px-3 py-2 rounded-xl bg-zinc-950/80 hover:bg-zinc-800 border border-zinc-800 hover:border-amber-500/40 text-zinc-300 hover:text-amber-300 text-[11px] font-medium flex items-center justify-center gap-1.5 transition-colors"
                title="Déclarer une partition ou un dossier externe"
              >
                <FolderPlus class="w-3.5 h-3.5 text-amber-400" />
                <span>Dossier externe</span>
              </button>
            </div>

            <!-- Quick Link to Library for Battle.net Games -->
            <router-link
              v-if="bnetStore.totalGamesCount > 0"
              to="/library?tag=battlenet"
              class="w-full px-3 py-2 rounded-xl bg-amber-500/10 hover:bg-amber-500/20 border border-amber-500/30 text-amber-300 text-xs font-semibold flex items-center justify-center gap-2 transition-colors shadow-sm"
            >
              <Gamepad2 class="w-4 h-4 text-amber-400" />
              <span>Voir les {{ bnetStore.totalGamesCount }} jeux dans ma Bibliothèque</span>
            </router-link>

            <!-- Toggle View Battle.net Games -->
            <button
              v-if="bnetStore.installedGames.length > 0 || bnetStore.ownedGames.length > 0"
              type="button"
              @click="showBnetGamesList = !showBnetGamesList"
              class="w-full text-left text-[11px] text-amber-400 hover:text-amber-300 font-medium flex items-center justify-between px-1"
            >
              <span>{{ showBnetGamesList ? 'Masquer les jeux' : `Parcourir les ${bnetStore.totalGamesCount} jeux Battle.net` }}</span>
              <i :class="showBnetGamesList ? 'ph ph-caret-up' : 'ph ph-caret-down'"></i>
            </button>

            <!-- Games list with tabs -->
            <div
              v-if="showBnetGamesList"
              class="space-y-2 pt-1"
            >
              <div class="flex items-center gap-1.5 p-1 rounded-lg bg-zinc-950 border border-zinc-800 text-[11px]">
                <button
                  type="button"
                  @click="bnetGamesTab = 'installed'"
                  :class="[
                    'flex-1 py-1 rounded font-medium transition-colors text-center',
                    bnetGamesTab === 'installed'
                      ? 'bg-emerald-500/20 text-emerald-300 border border-emerald-500/40'
                      : 'text-zinc-400 hover:text-zinc-200'
                  ]"
                >
                  Installés ({{ bnetStore.installedCount }})
                </button>
                <button
                  type="button"
                  @click="bnetGamesTab = 'owned'"
                  :class="[
                    'flex-1 py-1 rounded font-medium transition-colors text-center',
                    bnetGamesTab === 'owned'
                      ? 'bg-amber-500/20 text-amber-300 border border-amber-500/40'
                      : 'text-zinc-400 hover:text-zinc-200'
                  ]"
                >
                  Licences Cloud ({{ bnetStore.ownedGames.length }})
                </button>
              </div>

              <div class="max-h-48 overflow-y-auto space-y-1.5 pr-1 no-scrollbar">
                <template v-if="bnetGamesTab === 'installed'">
                  <div
                    v-for="game in bnetStore.installedGames"
                    :key="game.product_uid"
                    class="flex items-center justify-between p-2 rounded-lg bg-zinc-950/50 border border-zinc-800/50 text-[11px]"
                  >
                    <div class="min-w-0 pr-2">
                      <p class="text-zinc-200 truncate font-medium">{{ game.name }}</p>
                      <p class="text-[9px] text-zinc-500 font-mono truncate">
                        UID: {{ game.product_uid }} <span v-if="game.version_build">• {{ game.version_build }}</span>
                      </p>
                    </div>
                    <span v-if="game.is_external" class="px-1.5 py-0.5 rounded text-[9px] font-mono bg-purple-500/20 text-purple-300 border border-purple-500/30 flex-shrink-0">
                      Externe
                    </span>
                    <span v-else class="text-emerald-400 font-mono text-[10px] flex-shrink-0">
                      Local
                    </span>
                  </div>
                </template>
                <template v-else>
                  <div
                    v-for="game in bnetStore.ownedGames"
                    :key="game.product_uid"
                    class="flex items-center justify-between p-2 rounded-lg bg-zinc-950/50 border border-zinc-800/50 text-[11px]"
                  >
                    <span class="text-zinc-200 truncate font-medium">{{ game.name }}</span>
                    <span class="text-amber-400/80 font-mono text-[10px] flex-shrink-0 ml-2">
                      Possédé
                    </span>
                  </div>
                </template>
              </div>
            </div>
          </div>
        </div>

        <!-- Footer Action Button -->
        <div class="relative z-10 pt-4 border-t border-zinc-800/80 flex items-center justify-between gap-3">
          <span v-if="acc.connected" class="text-[11px] text-zinc-500 truncate font-mono">
            {{ acc.connectedAt }}
          </span>
          <span v-else class="text-[11px] text-zinc-500">
            Aucun compte lié
          </span>

          <UiButton
            v-if="acc.platform === 'steam' && acc.connected"
            variant="secondary"
            size="sm"
            :disabled="isSyncingSteam"
            @click="handleSyncSteam"
          >
            <template #prefix>
              <FolderSync :class="['w-3.5 h-3.5 text-amber-400', isSyncingSteam ? 'animate-spin' : '']" />
            </template>
            {{ isSyncingSteam ? 'Synchronisation...' : 'Synchroniser' }}
          </UiButton>

          <div v-else-if="acc.platform === 'battlenet'" class="flex items-center gap-2">
            <UiButton
              variant="outline"
              size="sm"
              title="Configurer les clés d'API Blizzard Developer"
              @click="bnetStore.openAuthModal"
            >
              <template #prefix>
                <KeyRound class="w-3.5 h-3.5 text-amber-400" />
              </template>
              API
            </UiButton>

            <UiButton
              :variant="acc.connected ? 'outline' : 'primary'"
              size="sm"
              :loading="bnetStore.isLoading"
              @click="handleBattleNetAction(acc.connected)"
            >
              <template #prefix>
                <Unlink v-if="acc.connected" class="w-3.5 h-3.5" />
                <Link v-else class="w-3.5 h-3.5" />
              </template>
              {{ acc.connected ? 'Déconnecter' : 'Lier Blizzard' }}
            </UiButton>
          </div>

          <UiButton
            v-else
            :variant="acc.connected ? 'outline' : 'primary'"
            size="sm"
            @click="accountsStore.toggleConnection(acc.platform)"
          >
            <template #prefix>
              <Unlink v-if="acc.connected" class="w-3.5 h-3.5" />
              <Link v-else class="w-3.5 h-3.5" />
            </template>
            {{ acc.connected ? 'Déconnecter' : 'Associer' }}
          </UiButton>
        </div>
      </div>
    </div>

    <!-- Modal d'ajout de dossier externe Battle.net -->
    <UiModal
      :open="showExternalModal"
      title="Ajouter un Répertoire Externe Battle.net"
      subtitle="Spécifiez le chemin absolu du dossier d'installation (ex: /mnt/games/Blizzard/World of Warcraft)"
      max-width="md"
      @close="showExternalModal = false"
    >
      <div class="space-y-4">
        <div>
          <label class="block text-xs font-semibold text-zinc-300 uppercase tracking-wider mb-2 font-mono">
            Chemin du répertoire
          </label>
          <input
            v-model="externalPathInput"
            type="text"
            placeholder="/mnt/games/Blizzard/World of Warcraft"
            class="w-full px-3.5 py-2.5 rounded-xl bg-zinc-950 border border-zinc-800 focus:border-amber-500 text-xs text-zinc-100 placeholder-zinc-600 focus:outline-none font-mono"
            @keydown.enter="handleAddExternalGame"
          />
          <p class="text-[11px] text-zinc-500 mt-1.5">
            Le répertoire doit contenir un fichier <code class="text-amber-400 font-mono">.build.info</code> et les exécutables officiels de Blizzard.
          </p>
        </div>

        <div class="flex items-center justify-end gap-2 pt-3 border-t border-zinc-800">
          <UiButton variant="ghost" size="sm" @click="showExternalModal = false">
            Annuler
          </UiButton>
          <UiButton
            variant="primary"
            size="sm"
            :loading="isAddingExternal"
            :disabled="!externalPathInput.trim()"
            @click="handleAddExternalGame"
          >
            Valider et Scanner
          </UiButton>
        </div>
      </div>
    </UiModal>
    <!-- Blizzard Developer OAuth Modal -->
    <BlizzardAuthModal />
  </div>
</template>
