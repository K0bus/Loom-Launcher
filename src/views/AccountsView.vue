<script setup lang="ts">
import { ref } from 'vue'
import {
  UserCheck,
  Link,
  Unlink,
  ExternalLink,
  ShieldCheck,
  Copy,
  Check,
  CheckCircle2
} from 'lucide-vue-next'
import { useAccountsStore } from '@/stores/accountsStore'
import {
  PlatformIcon,
  SteamIcon,
  EpicIcon,
  XboxIcon,
  GogIcon,
  BattleNetIcon
} from '@/components/icons/platforms'
import type { AccountPlatform } from '@/models/account'
import UiButton from '@/components/ui/UiButton.vue'
import UiBadge from '@/components/ui/UiBadge.vue'
import UiModal from '@/components/ui/UiModal.vue'

const accountsStore = useAccountsStore()

const copiedCode = ref(false)

function copyDeviceCode() {
  if (accountsStore.currentDeviceCode?.userCode) {
    navigator.clipboard.writeText(accountsStore.currentDeviceCode.userCode)
    copiedCode.value = true
    setTimeout(() => (copiedCode.value = false), 2000)
  }
}

interface PlatformConfig {
  name: string
  provider: string
  description: string
  colorClass: string
  bgLight: string
  borderColor: string
  features: string[]
}

const platformConfigs: Record<string, PlatformConfig> = {
  steam: {
    name: 'Steam',
    provider: 'Valve Corporation',
    description: 'Détection locale des manifests ACF (~/.steam/steam), synchronisation des succès et temps de jeu.',
    colorClass: 'text-[#1a9fff]',
    bgLight: 'bg-[#1a9fff]/10',
    borderColor: 'border-[#1a9fff]/30',
    features: ['Bibliothèque Locale', 'Cloud Saves', 'Succès Steam']
  },
  epic: {
    name: 'Epic Games',
    provider: 'Epic Games, Inc.',
    description: 'Importation des titres Epic Games Store, gestion des lanceurs Wine/Proton et Unreal Engine.',
    colorClass: 'text-white',
    bgLight: 'bg-zinc-800/80',
    borderColor: 'border-zinc-700',
    features: ['EGS Store', 'Jeux Gratuits', 'Unreal Engine']
  },
  xbox: {
    name: 'Xbox / Microsoft',
    provider: 'Microsoft Gaming',
    description: 'Authentification OAuth2 Device Authorization Grant pour Minecraft Java/Bedrock et profils Xbox.',
    colorClass: 'text-[#107c10]',
    bgLight: 'bg-[#107c10]/10',
    borderColor: 'border-[#107c10]/30',
    features: ['Minecraft Officiel', 'Gamertag Xbox', 'Profils Multi']
  },
  microsoft: {
    name: 'Xbox / Microsoft',
    provider: 'Microsoft Gaming',
    description: 'Authentification OAuth2 Device Authorization Grant pour Minecraft Java/Bedrock et profils Xbox.',
    colorClass: 'text-[#107c10]',
    bgLight: 'bg-[#107c10]/10',
    borderColor: 'border-[#107c10]/30',
    features: ['Minecraft Officiel', 'Gamertag Xbox', 'Profils Multi']
  },
  gog: {
    name: 'GOG Galaxy',
    provider: 'CD PROJEKT',
    description: 'Catalogue de jeux 100% sans DRM, synchronisation des sauvegardes cloud et installeurs hors-ligne.',
    colorClass: 'text-[#b44aff]',
    bgLight: 'bg-[#86328a]/15',
    borderColor: 'border-[#86328a]/35',
    features: ['Sans DRM', 'Installeurs Offline', 'Cloud GOG']
  },
  battlenet: {
    name: 'Battle.net',
    provider: 'Blizzard Entertainment',
    description: 'Intégration du lanceur Battle.net sous Linux (Wine-GE / Proton) et synchronisation des licences.',
    colorClass: 'text-[#00aeff]',
    bgLight: 'bg-[#00aeff]/10',
    borderColor: 'border-[#00aeff]/30',
    features: ['Blizzard Launcher', 'Overwatch / Diablo', 'Wine Staging']
  }
}

function getConfig(platform: string): PlatformConfig {
  return (
    platformConfigs[platform] || {
      name: platform,
      provider: 'Plateforme Tierce',
      description: 'Synchronisation unifiée avec Loom.',
      colorClass: 'text-zinc-300',
      bgLight: 'bg-zinc-800',
      borderColor: 'border-zinc-700',
      features: ['Lancement Natif']
    }
  )
}
</script>

<template>
  <div class="max-w-7xl mx-auto px-6 sm:px-10 lg:px-14 py-6 space-y-8 pb-16">
    <!-- Header -->
    <div class="flex flex-col md:flex-row items-start md:items-center justify-between gap-4">
      <div>
        <h1 class="text-3xl font-black text-zinc-100 tracking-tight font-display flex items-center gap-3">
          <UserCheck class="w-8 h-8 text-amber-400" />
          Hub de Comptes & Services
        </h1>
        <p class="text-xs text-zinc-400 mt-1">
          Associez vos plateformes de jeux officielles pour synchroniser vos bibliothèques sans fausse information
        </p>
      </div>

      <!-- Quick Platform Badge Pill Strip -->
      <div class="flex items-center gap-2 p-1.5 rounded-2xl bg-zinc-900/80 border border-zinc-800/80 backdrop-blur-md">
        <div class="px-2.5 py-1 text-[11px] font-mono text-zinc-400 font-semibold uppercase tracking-wider">
          Supportés
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

    <!-- Security Banner -->
    <div class="p-4 rounded-2xl bg-amber-500/10 border border-amber-500/30 flex items-start gap-3.5 text-xs">
      <ShieldCheck class="w-5 h-5 text-amber-400 flex-shrink-0 mt-0.5" />
      <div class="space-y-1">
        <div class="font-bold text-amber-300">Authentification Sécurisée & Chiffrée Localement</div>
        <p class="text-zinc-300 leading-relaxed">
          Les jetons de session et clés d'API sont stockés dans le trousseau sécurisé de votre machine Linux (Secret Service API / KWallet). Loom ne transmet aucun mot de passe et n'invente aucune fausse donnée.
        </p>
      </div>
    </div>

    <!-- Accounts Cards Grid (5 Supported Platforms) -->
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
      <div
        v-for="acc in accountsStore.accounts"
        :key="acc.id"
        class="p-6 rounded-2xl bg-zinc-900/60 border border-zinc-800/80 hover:border-zinc-700/80 transition-all space-y-5 shadow-lg flex flex-col justify-between group"
      >
        <div class="space-y-4">
          <!-- Card Header : SVG Logo + Platform Name + Status Badge -->
          <div class="flex items-start justify-between gap-3">
            <div class="flex items-center gap-3.5">
              <!-- Platform Official SVG Avatar or User Avatar with SVG Badge -->
              <div class="relative">
                <div
                  v-if="acc.avatarUrl"
                  class="w-12 h-12 rounded-xl overflow-hidden border border-zinc-700 bg-zinc-950 flex-shrink-0"
                >
                  <img :src="acc.avatarUrl" :alt="acc.displayName" class="w-full h-full object-cover" />
                </div>
                <!-- Default Official Platform SVG Icon Display -->
                <div
                  v-else
                  :class="[
                    'w-12 h-12 rounded-xl flex items-center justify-center border flex-shrink-0 shadow-inner transition-transform group-hover:scale-105',
                    getConfig(acc.platform).bgLight,
                    getConfig(acc.platform).borderColor
                  ]"
                >
                  <PlatformIcon
                    :platform="acc.platform"
                    class-name="w-6 h-6"
                    :colored="true"
                  />
                </div>

                <!-- Mini Status Indicator -->
                <span
                  v-if="acc.connected"
                  class="absolute -bottom-1 -right-1 w-4 h-4 rounded-full bg-emerald-500 border-2 border-zinc-900 flex items-center justify-center text-zinc-950"
                  title="Connecté"
                >
                  <CheckCircle2 class="w-3 h-3 text-white" />
                </span>
              </div>

              <!-- Names & Provider -->
              <div class="min-w-0">
                <div class="flex items-center gap-1.5">
                  <h3 class="font-bold text-sm text-zinc-100 truncate">
                    {{ getConfig(acc.platform).name }}
                  </h3>
                  <!-- Small inline SVG icon -->
                  <PlatformIcon
                    :platform="acc.platform"
                    class-name="w-3.5 h-3.5 flex-shrink-0"
                    :colored="true"
                  />
                </div>
                <p class="text-[11px] text-zinc-400 font-medium truncate">
                  {{ acc.connected && acc.username ? `@${acc.username}` : getConfig(acc.platform).provider }}
                </p>
              </div>
            </div>

            <!-- Connection Status Badge -->
            <UiBadge
              :variant="acc.connected ? 'emerald' : 'zinc'"
              size="sm"
              :dot="acc.connected"
            >
              {{ acc.connected ? 'Connecté' : 'Non lié' }}
            </UiBadge>
          </div>

          <!-- Description text -->
          <p class="text-xs text-zinc-400 leading-relaxed min-h-[36px]">
            {{ getConfig(acc.platform).description }}
          </p>

          <!-- Feature Tag Pills -->
          <div class="flex flex-wrap items-center gap-1.5 pt-1">
            <span
              v-for="(feat, idx) in getConfig(acc.platform).features"
              :key="idx"
              class="px-2 py-0.5 rounded-md bg-zinc-950/70 border border-zinc-800/80 text-[10px] text-zinc-400 font-mono"
            >
              {{ feat }}
            </span>
          </div>

          <!-- Metrics Box -->
          <div class="grid grid-cols-2 gap-3 p-3 rounded-xl bg-zinc-950/60 border border-zinc-800/60 text-xs">
            <div>
              <span class="text-zinc-500 block text-[11px]">Jeux détectés:</span>
              <span class="text-zinc-200 font-bold font-mono">
                {{ acc.connected ? `${acc.gamesCount} titres` : '—' }}
              </span>
            </div>
            <div>
              <span class="text-zinc-500 block text-[11px]">Protocole:</span>
              <span class="text-zinc-200 font-medium capitalize font-mono text-[11px]">
                {{ acc.authType.replace('_', ' ') }}
              </span>
            </div>
          </div>
        </div>

        <!-- Footer Action Button -->
        <div class="pt-4 border-t border-zinc-800/80 flex items-center justify-between gap-3">
          <span v-if="acc.connected && acc.connectedAt" class="text-[11px] text-zinc-500 truncate">
            Lié le {{ acc.connectedAt }}
          </span>
          <span v-else class="text-[11px] text-zinc-500">
            Prêt pour synchronisation
          </span>

          <UiButton
            :variant="acc.connected ? 'outline' : 'primary'"
            size="sm"
            @click="accountsStore.toggleConnection(acc.platform as AccountPlatform)"
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

    <!-- OAUTH2 DEVICE CODE MODAL FOR MICROSOFT / MINECRAFT -->
    <UiModal
      :open="accountsStore.deviceCodeModalOpen"
      title="Connexion Compte Microsoft (Minecraft)"
      subtitle="Protocole OAuth2 Device Authorization Grant"
      max-width="md"
      @close="accountsStore.deviceCodeModalOpen = false"
    >
      <div v-if="accountsStore.currentDeviceCode" class="space-y-5 py-2 text-center">
        <div class="p-3 bg-zinc-950/80 border border-zinc-800 rounded-xl space-y-1">
          <p class="text-xs text-zinc-400">
            Ouvrez la page de validation Microsoft suivante dans votre navigateur :
          </p>
          <a
            :href="accountsStore.currentDeviceCode.verificationUri"
            target="_blank"
            class="text-amber-400 font-bold text-sm inline-flex items-center gap-1.5 hover:underline"
          >
            {{ accountsStore.currentDeviceCode.verificationUri }}
            <ExternalLink class="w-3.5 h-3.5" />
          </a>
        </div>

        <div class="space-y-2">
          <div class="text-xs text-zinc-400 uppercase tracking-wider font-mono">
            Code d'Appairage Unique
          </div>
          <div class="flex items-center justify-center gap-3">
            <span class="text-3xl font-mono font-black text-amber-400 tracking-widest bg-zinc-950 px-6 py-3 rounded-2xl border border-amber-500/40 shadow-inner">
              {{ accountsStore.currentDeviceCode.userCode }}
            </span>
            <button
              type="button"
              @click="copyDeviceCode"
              class="p-3 rounded-xl bg-zinc-800 hover:bg-zinc-700 text-zinc-200 transition-colors"
              title="Copier le code"
            >
              <Check v-if="copiedCode" class="w-5 h-5 text-emerald-400" />
              <Copy v-else class="w-5 h-5" />
            </button>
          </div>
        </div>

        <div class="flex items-center justify-center gap-2 text-xs text-zinc-500 font-mono">
          <span class="w-2 h-2 rounded-full bg-amber-400 animate-ping" />
          <span>En attente de confirmation sur les serveurs Microsoft...</span>
        </div>
      </div>

      <template #footer>
        <UiButton
          variant="ghost"
          size="sm"
          @click="accountsStore.deviceCodeModalOpen = false"
        >
          Annuler
        </UiButton>
        <UiButton
          variant="primary"
          size="sm"
          :loading="accountsStore.isAuthorizing"
          @click="accountsStore.confirmMicrosoftDeviceCode"
        >
          Confirmer l'authentification
        </UiButton>
      </template>
    </UiModal>
  </div>
</template>
