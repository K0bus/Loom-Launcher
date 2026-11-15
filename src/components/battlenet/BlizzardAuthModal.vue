<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import {
  KeyRound,
  ExternalLink,
  Copy,
  Check,
  ShieldCheck,
  Eye,
  EyeOff,
  Sparkles,
  Loader2,
  AlertCircle
} from 'lucide-vue-next'
import UiModal from '@/components/ui/UiModal.vue'
import UiButton from '@/components/ui/UiButton.vue'
import { useBattleNetStore } from '@/stores/useBattleNetStore'
import { useToastStore } from '@/stores/toastStore'
import { ipc } from '@/services/ipc'

const bnetStore = useBattleNetStore()
const toast = useToastStore()

const clientId = ref('')
const clientSecret = ref('')
const showSecret = ref(false)
const copiedUris = ref(false)
const errorMessage = ref<string | null>(null)
const isSubmitting = ref(false)

const redirectUrisText = `http://localhost:5173/api/auth/battlenet/callback\nhttp://localhost:8999/callback\nhttp://127.0.0.1:8999/callback`

// Pre-fill existing Client ID if already known
watch(
  () => bnetStore.isAuthModalOpen,
  (open) => {
    if (open) {
      errorMessage.value = null
      if (bnetStore.configuredClientId) {
        clientId.value = bnetStore.configuredClientId
      }
    }
  },
  { immediate: true }
)

const canSubmit = computed(() => {
  return clientId.value.trim().length > 0 && clientSecret.value.trim().length > 0
})

function copyUris() {
  navigator.clipboard.writeText(redirectUrisText)
  copiedUris.value = true
  toast.show('URLs de redirection copiées dans le presse-papier !', 'info')
  setTimeout(() => {
    copiedUris.value = false
  }, 2500)
}

function openDevPortal() {
  ipc.openExternalUrl('https://develop.battle.net/access/clients')
}

async function handleSaveAndConnect() {
  errorMessage.value = null
  isSubmitting.value = true

  try {
    // 1. Save credentials to .env / backend
    await bnetStore.saveCredentials(clientId.value.trim(), clientSecret.value.trim())

    // 2. Launch official Blizzard OAuth 2.0 flow
    await bnetStore.loginBlizzard(true)
  } catch (err: any) {
    errorMessage.value = err?.message || 'Erreur lors de la connexion Blizzard'
  } finally {
    isSubmitting.value = false
  }
}

async function handleDirectConnect() {
  errorMessage.value = null
  isSubmitting.value = true
  try {
    await bnetStore.loginBlizzard(true)
  } catch (err: any) {
    errorMessage.value = err?.message || 'Erreur lors de la connexion Blizzard'
  } finally {
    isSubmitting.value = false
  }
}
</script>

<template>
  <UiModal
    :open="bnetStore.isAuthModalOpen"
    title="Connexion Officielle Blizzard Battle.net"
    subtitle="Authentification OAuth 2.0 sécurisée via oauth.battle.net"
    max-width="xl"
    @close="bnetStore.closeAuthModal"
  >
    <!-- Waiting/Authenticating in Browser Overlay -->
    <div
      v-if="bnetStore.isAuthenticating"
      class="py-8 flex flex-col items-center justify-center text-center space-y-4"
    >
      <div class="relative">
        <div class="w-16 h-16 rounded-full bg-amber-500/10 border border-amber-500/40 flex items-center justify-center animate-pulse">
          <Loader2 class="w-8 h-8 text-amber-400 animate-spin" />
        </div>
        <div class="absolute -bottom-1 -right-1 w-6 h-6 rounded-full bg-zinc-900 border border-amber-500 flex items-center justify-center">
          <Sparkles class="w-3.5 h-3.5 text-amber-400" />
        </div>
      </div>

      <div class="space-y-2 max-w-md">
        <h4 class="text-base font-semibold text-zinc-100">
          Authentification en cours dans votre navigateur
        </h4>
        <p class="text-xs text-zinc-400 leading-relaxed">
          Une fenêtre de votre navigateur s'est ouverte sur le portail officiel
          <span class="text-amber-400 font-mono">oauth.battle.net</span>. Identifiez-vous et autorisez Loom pour synchroniser votre compte.
        </p>
      </div>

      <div class="p-3 bg-zinc-950/80 border border-zinc-800 rounded-xl text-xs text-zinc-500 font-mono">
        En attente du retour de Blizzard... (Délai : 120s)
      </div>
    </div>

    <!-- Main Configuration Form -->
    <div v-else class="space-y-5">
      <!-- Intro Card with Amber accents -->
      <div class="p-4 rounded-xl bg-gradient-to-r from-amber-500/10 via-zinc-900 to-zinc-950 border border-amber-500/30 space-y-2">
        <div class="flex items-center gap-2 text-amber-400 font-semibold text-xs tracking-wide uppercase">
          <ShieldCheck class="w-4 h-4" />
          <span>Vrai OAuth 2.0 Blizzard (Direct & Sans Intermédiaire)</span>
        </div>
        <p class="text-xs text-zinc-300 leading-relaxed">
          Pour relier votre véritable compte Battle.net (World of Warcraft, Diablo IV, Overwatch 2...), vous devez renseigner vos identifiants d'API Blizzard Client. La création est
          <span class="text-amber-400 font-medium">100% gratuite</span> et prend 30 secondes.
        </p>
      </div>

      <!-- Quick Steps Guide -->
      <div class="p-3.5 rounded-xl bg-zinc-950 border border-zinc-800 space-y-3 text-xs">
        <div class="flex items-center justify-between">
          <span class="font-medium text-zinc-200 flex items-center gap-1.5">
            <KeyRound class="w-3.5 h-3.5 text-amber-400" />
            Comment obtenir vos clés d'API Blizzard :
          </span>
          <button
            type="button"
            class="text-amber-400 hover:text-amber-300 font-medium flex items-center gap-1 transition-colors text-[11px]"
            @click="openDevPortal"
          >
            <span>Ouvrir develop.battle.net</span>
            <ExternalLink class="w-3 h-3" />
          </button>
        </div>

        <ol class="space-y-1.5 text-zinc-400 list-decimal list-inside pl-1 text-[11.5px] leading-relaxed">
          <li>Connectez-vous sur <span class="text-zinc-200">develop.battle.net</span> et cliquez sur <strong class="text-zinc-200">Create Client</strong>.</li>
          <li>Nommez-le <strong class="text-amber-400">Loom Launcher</strong> et cochez les autorisations standard.</li>
          <li>
            Dans le champ <strong class="text-zinc-200">Redirect URLs</strong>, ajoutez les URLs locales suivantes :
            <div class="mt-1 flex items-center justify-between gap-2 p-2 bg-zinc-900 rounded-lg border border-zinc-800 font-mono text-[10.5px] text-zinc-300">
              <span class="truncate">http://localhost:5173/... et http://localhost:8999/...</span>
              <button
                type="button"
                class="px-2 py-0.5 rounded bg-zinc-800 hover:bg-zinc-700 text-amber-400 flex items-center gap-1 flex-shrink-0 transition-colors"
                @click="copyUris"
              >
                <Check v-if="copiedUris" class="w-3 h-3 text-emerald-400" />
                <Copy v-else class="w-3 h-3" />
                <span>{{ copiedUris ? 'Copié !' : 'Copier les 3 URLs' }}</span>
              </button>
            </div>
          </li>
          <li>Copiez votre <strong class="text-zinc-200">Client ID</strong> et votre <strong class="text-zinc-200">Client Secret</strong> ci-dessous.</li>
        </ol>
      </div>

      <!-- Form Inputs -->
      <div class="space-y-3.5">
        <div class="space-y-1.5">
          <label class="block text-xs font-medium text-zinc-300">
            Blizzard Client ID <span class="text-amber-400">*</span>
          </label>
          <input
            v-model="clientId"
            type="text"
            placeholder="Ex: d8a9f234bc56..."
            class="w-full px-3.5 py-2.5 bg-zinc-950 border border-zinc-800 rounded-xl text-zinc-100 font-mono text-xs focus:outline-none focus:border-amber-500 transition-colors"
          />
        </div>

        <div class="space-y-1.5">
          <label class="block text-xs font-medium text-zinc-300">
            Blizzard Client Secret <span class="text-amber-400">*</span>
          </label>
          <div class="relative">
            <input
              v-model="clientSecret"
              :type="showSecret ? 'text' : 'password'"
              placeholder="••••••••••••••••••••••••••••••••"
              class="w-full px-3.5 py-2.5 bg-zinc-950 border border-zinc-800 rounded-xl text-zinc-100 font-mono text-xs focus:outline-none focus:border-amber-500 transition-colors pr-10"
            />
            <button
              type="button"
              class="absolute right-3 top-1/2 -translate-y-1/2 text-zinc-500 hover:text-zinc-300 transition-colors"
              @click="showSecret = !showSecret"
            >
              <EyeOff v-if="showSecret" class="w-4 h-4" />
              <Eye v-else class="w-4 h-4" />
            </button>
          </div>
        </div>
      </div>

      <!-- Error Alert -->
      <div
        v-if="errorMessage"
        class="p-3 rounded-xl bg-red-500/10 border border-red-500/30 text-red-400 text-xs flex items-start gap-2"
      >
        <AlertCircle class="w-4 h-4 flex-shrink-0 mt-0.5" />
        <div class="space-y-1">
          <span class="font-medium">Erreur d'authentification :</span>
          <p class="text-red-300">{{ errorMessage }}</p>
        </div>
      </div>
    </div>

    <!-- Modal Footer Actions -->
    <template #footer>
      <div class="flex items-center justify-between w-full">
        <button
          type="button"
          class="text-xs text-zinc-500 hover:text-zinc-300 transition-colors"
          @click="bnetStore.closeAuthModal"
        >
          Annuler
        </button>

        <div class="flex items-center gap-2">
          <!-- If credentials already configured, offer quick reconnect without retyping -->
          <UiButton
            v-if="bnetStore.credentialsConfigured && !canSubmit"
            variant="outline"
            size="sm"
            :loading="isSubmitting"
            @click="handleDirectConnect"
          >
            Lancer avec clés actuelles
          </UiButton>

          <UiButton
            variant="primary"
            size="sm"
            :disabled="!canSubmit"
            :loading="isSubmitting"
            @click="handleSaveAndConnect"
          >
            <template #prefix>
              <KeyRound class="w-3.5 h-3.5 text-zinc-950" />
            </template>
            Enregistrer & Se Connecter avec Blizzard
          </UiButton>
        </div>
      </div>
    </template>
  </UiModal>
</template>
