<script setup lang="ts">
import { ref, computed } from 'vue'
import {
  Copy,
  Check,
  ExternalLink,
  ShieldCheck,
  Loader2,
  AlertCircle,
  RefreshCw
} from 'lucide-vue-next'
import { useMinecraftStore } from '@/stores/minecraftStore'
import { openExternalLink } from '@/utils/externalLinks'
import UiModal from '@/components/ui/UiModal.vue'
import UiButton from '@/components/ui/UiButton.vue'
import MinecraftAvatar from '@/components/minecraft/MinecraftAvatar.vue'

const minecraftStore = useMinecraftStore()
const copied = ref(false)

const open = computed(() => minecraftStore.authModalOpen)
const codeInfo = computed(() => minecraftStore.deviceCodeInfo)
const authError = computed(() => minecraftStore.authError)
const account = computed(() => minecraftStore.account)
const isPolling = computed(() => minecraftStore.isAuthPolling)

const displayVerificationUrl = computed(() => {
  if (!codeInfo.value?.verification_uri) return 'microsoft.com/link'
  try {
    const u = new URL(codeInfo.value.verification_uri)
    return `${u.hostname}${u.pathname}`
  } catch {
    return 'microsoft.com/link'
  }
})

function copyCode() {
  if (codeInfo.value?.user_code) {
    navigator.clipboard.writeText(codeInfo.value.user_code)
    copied.value = true
    setTimeout(() => (copied.value = false), 2500)
  }
}

function openVerificationLink() {
  // Automatically copy code when user clicks to open the official verification page
  if (codeInfo.value?.user_code) {
    navigator.clipboard.writeText(codeInfo.value.user_code)
    copied.value = true
    setTimeout(() => (copied.value = false), 2500)
  }
  openExternalLink(codeInfo.value?.verification_uri || 'https://microsoft.com/link')
}
</script>

<template>
  <UiModal
    :open="open"
    title="Connexion Compte Microsoft / Xbox Live"
    subtitle="Authentification officielle Device Code Flow (RFC 8628)"
    max-width="md"
    @close="minecraftStore.closeAuthModal"
  >
    <div class="space-y-6 py-2 text-center">
      <!-- SUCCESS STATE -->
      <div v-if="account" class="space-y-4 py-4">
        <div class="relative w-16 h-16 mx-auto rounded-2xl bg-zinc-900 border border-emerald-500/40 p-1 flex items-center justify-center shadow-lg shadow-emerald-500/10">
          <MinecraftAvatar
            :uuid="account.uuid"
            :name="account.name"
            :skin-url="account.skin_url"
            :raw-skin-url="account.raw_skin_url"
            :size="56"
            rounded="rounded-xl"
          />
          <div class="absolute -bottom-1 -right-1 w-5 h-5 rounded-full bg-emerald-500 flex items-center justify-center text-zinc-950 border-2 border-zinc-900 shadow">
            <Check class="w-3 h-3 stroke-[3]" />
          </div>
        </div>
        <div class="space-y-1">
          <h3 class="text-base font-bold text-zinc-100">Connexion réussie !</h3>
          <p class="text-xs text-emerald-400 font-mono">
            Bienvenue dans Loom Launcher, {{ account.name }}
          </p>
        </div>
      </div>

      <!-- ERROR STATE -->
      <div v-else-if="authError" class="space-y-4 py-2">
        <div class="w-16 h-16 mx-auto rounded-3xl bg-rose-500/20 border border-rose-500/40 flex items-center justify-center text-rose-400">
          <AlertCircle class="w-8 h-8" />
        </div>
        <div class="space-y-1">
          <h3 class="text-base font-bold text-rose-400">Échec de l'authentification</h3>
          <p class="text-xs text-zinc-400 leading-relaxed max-w-sm mx-auto font-mono bg-zinc-900/80 p-3 rounded-xl border border-zinc-800">
            {{ authError }}
          </p>
        </div>
        <div class="flex justify-center pt-2">
          <UiButton
            variant="primary"
            size="md"
            @click="minecraftStore.startMicrosoftLogin"
          >
            <template #prefix>
              <RefreshCw class="w-4 h-4" />
            </template>
            Générer un Nouveau Code
          </UiButton>
        </div>
      </div>

      <!-- ACTIVE DEVICE CODE FLOW -->
      <template v-else>
        <!-- Icon graphic -->
        <div class="w-16 h-16 mx-auto rounded-3xl bg-amber-500/10 border border-amber-500/30 flex items-center justify-center text-amber-400">
          <ShieldCheck class="w-8 h-8" />
        </div>

        <div class="space-y-2">
          <h3 class="text-base font-bold text-zinc-100">
            Autorisez Loom Launcher sur Microsoft
          </h3>
          <p class="text-xs text-zinc-400 max-w-sm mx-auto leading-relaxed">
            Pour lier votre compte officiel Minecraft: Java Edition, saisissez le code ci-dessous sur la page Microsoft sécurisée :
          </p>
        </div>

        <!-- User Code Display -->
        <div class="p-4 rounded-2xl bg-zinc-900/90 border border-zinc-800 flex items-center justify-between gap-4 max-w-xs mx-auto shadow-inner relative group">
          <!-- Loading state if code is generating -->
          <div v-if="!codeInfo?.user_code" class="flex items-center justify-center gap-2.5 w-full py-1 text-zinc-400">
            <Loader2 class="w-5 h-5 animate-spin text-amber-400" />
            <span class="font-mono text-sm tracking-wide">Génération du code...</span>
          </div>

          <!-- Code ready state -->
          <template v-else>
            <span class="font-mono text-2xl font-black text-amber-400 tracking-wider select-all">
              {{ codeInfo.user_code }}
            </span>
            <button
              type="button"
              @click="copyCode"
              class="p-2 rounded-xl bg-zinc-800 text-zinc-300 hover:text-white hover:bg-zinc-700 transition-colors"
              title="Copier le code"
            >
              <Check v-if="copied" class="w-4 h-4 text-emerald-400" />
              <Copy v-else class="w-4 h-4" />
            </button>
          </template>
        </div>

        <!-- Copied feedback tooltip -->
        <div v-if="copied" class="text-[11px] font-mono text-emerald-400 -mt-3">
          ✓ Code copié dans votre presse-papier
        </div>

        <!-- Action link -->
        <div class="flex justify-center">
          <UiButton
            variant="primary"
            size="md"
            :disabled="!codeInfo?.user_code"
            @click="openVerificationLink"
          >
            <template #prefix>
              <ExternalLink class="w-4 h-4" />
            </template>
            Ouvrir {{ displayVerificationUrl }}
          </UiButton>
        </div>

        <!-- Polling Status -->
        <div v-if="isPolling" class="flex items-center justify-center gap-2 text-xs font-mono text-zinc-500">
          <Loader2 class="w-3.5 h-3.5 animate-spin text-amber-400" />
          <span>En attente de votre autorisation dans le navigateur...</span>
        </div>
      </template>
    </div>
  </UiModal>
</template>
