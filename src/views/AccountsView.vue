<script setup lang="ts">
import { ref } from 'vue'
import {
  UserCheck,
  Link,
  Unlink,
  ExternalLink,
  ShieldCheck,
  Copy,
  Check
} from 'lucide-vue-next'
import { useAccountsStore } from '@/stores/accountsStore'
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

const platformIcons: Record<string, string> = {
  steam: '🎮',
  microsoft: '🟩',
  gog: '🟣',
  epic: '⚡'
}
</script>

<template>
  <div class="max-w-7xl mx-auto px-6 sm:px-10 lg:px-14 py-6 space-y-8 pb-16">
    <!-- Header -->
    <div>
      <h1 class="text-3xl font-black text-zinc-100 tracking-tight font-display flex items-center gap-3">
        <UserCheck class="w-8 h-8 text-amber-400" />
        Hub de Comptes & Services
      </h1>
      <p class="text-xs text-zinc-400 mt-1">
        Synchronisez vos bibliothèques Steam, GOG, Epic et authentifiez votre compte Microsoft Minecraft officiel
      </p>
    </div>

    <!-- Security Banner -->
    <div class="p-4 rounded-2xl bg-amber-500/10 border border-amber-500/30 flex items-start gap-3.5 text-xs">
      <ShieldCheck class="w-5 h-5 text-amber-400 flex-shrink-0 mt-0.5" />
      <div class="space-y-1">
        <div class="font-bold text-amber-300">Authentification Sécurisée & Chiffrée Localement</div>
        <p class="text-zinc-300 leading-relaxed">
          Les jetons de session sont stockés dans le trousseau de clés chiffré de votre système Linux (Secret Service API / KWallet). Loom n'a jamais accès à vos mots de passe.
        </p>
      </div>
    </div>

    <!-- Accounts Cards Grid -->
    <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
      <div
        v-for="acc in accountsStore.accounts"
        :key="acc.id"
        class="p-6 rounded-2xl bg-zinc-900/60 border border-zinc-800/80 hover:border-zinc-700 transition-all space-y-5 shadow-lg flex flex-col justify-between"
      >
        <div class="space-y-4">
          <div class="flex items-start justify-between gap-4">
            <div class="flex items-center gap-3.5">
              <img
                :src="acc.avatarUrl"
                :alt="acc.displayName"
                class="w-12 h-12 rounded-xl object-cover border border-zinc-700"
              />
              <div>
                <div class="flex items-center gap-2">
                  <h3 class="font-bold text-sm text-zinc-100">{{ acc.displayName }}</h3>
                  <span class="text-lg">{{ platformIcons[acc.platform] }}</span>
                </div>
                <p class="text-xs text-zinc-400 font-mono mt-0.5">@{{ acc.username }}</p>
              </div>
            </div>

            <UiBadge
              :variant="acc.connected ? 'emerald' : 'zinc'"
              size="sm"
              :dot="acc.connected"
            >
              {{ acc.connected ? 'Connecté' : 'Non lié' }}
            </UiBadge>
          </div>

          <div class="grid grid-cols-2 gap-3 p-3 rounded-xl bg-zinc-950/60 border border-zinc-800/60 text-xs">
            <div>
              <span class="text-zinc-500 block">Jeux détectés:</span>
              <span class="text-zinc-200 font-bold font-mono">{{ acc.gamesCount }} titres</span>
            </div>
            <div>
              <span class="text-zinc-500 block">Protocole:</span>
              <span class="text-zinc-200 font-medium capitalize">{{ acc.authType.replace('_', ' ') }}</span>
            </div>
          </div>
        </div>

        <div class="pt-4 border-t border-zinc-800/80 flex items-center justify-between">
          <span v-if="acc.connected" class="text-[11px] text-zinc-500">
            Lié le {{ acc.connectedAt }}
          </span>
          <span v-else class="text-[11px] text-zinc-500">
            Prêt pour synchronisation
          </span>

          <UiButton
            :variant="acc.connected ? 'outline' : 'primary'"
            size="sm"
            @click="accountsStore.toggleConnection(acc.platform)"
          >
            <template #prefix>
              <Unlink v-if="acc.connected" class="w-3.5 h-3.5" />
              <Link v-else class="w-3.5 h-3.5" />
            </template>
            {{ acc.connected ? 'Déconnecter' : 'Associer le compte' }}
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
