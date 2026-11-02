<script setup lang="ts">
import { ref } from 'vue'
import type { MinecraftLoader } from '@/models/minecraft'
import { useMinecraftStore } from '@/stores/minecraftStore'
import UiModal from '@/components/ui/UiModal.vue'
import UiButton from '@/components/ui/UiButton.vue'
import UiInput from '@/components/ui/UiInput.vue'
import UiBadge from '@/components/ui/UiBadge.vue'

interface Props {
  open: boolean
}

defineProps<Props>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'created', instanceId: string): void
}>()

const minecraftStore = useMinecraftStore()

const currentStep = ref<1 | 2 | 3 | 4>(1)

// Form fields
const instanceName = ref('Nouvelle Aventure')
const selectedIcon = ref('⛏️')
const selectedVersion = ref('1.21.4')
const selectedLoader = ref<MinecraftLoader>('fabric')
const allocatedRam = ref(6)

const availableIcons = ['⛏️', '🚀', '⚡', '🐉', '🏰', '🛡️', '⚔️', '🧪', '💎', '🔥']

const minecraftVersions = [
  { version: '1.21.4', type: 'Release', recommended: true },
  { version: '1.21.1', type: 'Release' },
  { version: '1.20.1', type: 'Release (Idéal Mods)', recommended: true },
  { version: '1.19.2', type: 'Release' },
  { version: '1.16.5', type: 'Legacy LTS' },
  { version: '24w44a', type: 'Snapshot' }
]

const loaders: Array<{
  id: MinecraftLoader
  name: string
  desc: string
  badge: string
}> = [
  {
    id: 'fabric',
    name: 'Fabric',
    desc: 'Léger, ultra-rapide et supporte les meilleurs mods d\'optimisation moderne (Sodium/Iris).',
    badge: 'Recommandé'
  },
  {
    id: 'neoforge',
    name: 'NeoForge',
    desc: 'Le successeur moderne et actif du projet Forge pour Minecraft 1.20.2 et plus.',
    badge: 'Modpacks Récents'
  },
  {
    id: 'forge',
    name: 'Minecraft Forge',
    desc: 'Le loader classique avec la plus vaste bibliothèque de mods historiques 1.12-1.20.1.',
    badge: 'Classique'
  },
  {
    id: 'quilt',
    name: 'Quilt',
    desc: 'Fork communautaire de Fabric avec rétro-compatibilité totale.',
    badge: 'Moderne'
  },
  {
    id: 'vanilla',
    name: 'Vanilla Pur',
    desc: 'Aucun mod loader, expérience officielle du jeu sans altération.',
    badge: 'Officiel'
  }
]

function nextStep() {
  if (currentStep.value < 4) {
    currentStep.value = (currentStep.value + 1) as any
  } else {
    finish()
  }
}

function prevStep() {
  if (currentStep.value > 1) {
    currentStep.value = (currentStep.value - 1) as any
  }
}

function finish() {
  const newInst = minecraftStore.createInstance({
    name: instanceName.value,
    icon: selectedIcon.value,
    gameVersion: selectedVersion.value,
    loader: selectedLoader.value,
    allocatedRamGb: allocatedRam.value
  })

  currentStep.value = 1
  emit('created', newInst.id)
  emit('close')
}
</script>

<template>
  <UiModal
    :open="open"
    title="Assistant de Création d'Instance"
    subtitle="Configurez votre environnement Minecraft personnalisé en quelques clics"
    max-width="xl"
    @close="emit('close')"
  >
    <!-- Stepper indicator -->
    <div class="flex items-center justify-between pb-2 border-b border-zinc-800">
      <div
        v-for="step in 4"
        :key="step"
        class="flex items-center gap-2"
      >
        <div
          :class="[
            'w-7 h-7 rounded-full flex items-center justify-center text-xs font-bold transition-all',
            currentStep === step
              ? 'bg-amber-500 text-zinc-950 shadow-md shadow-amber-500/20'
              : currentStep > step
                ? 'bg-amber-500/20 text-amber-300'
                : 'bg-zinc-800 text-zinc-500'
          ]"
        >
          {{ step }}
        </div>
        <span
          :class="[
            'text-xs font-medium hidden sm:inline',
            currentStep === step ? 'text-zinc-100 font-semibold' : 'text-zinc-500'
          ]"
        >
          {{
            step === 1 ? 'Identité' :
            step === 2 ? 'Version' :
            step === 3 ? 'Mod Loader' : 'Mémoire'
          }}
        </span>
        <div v-if="step < 4" class="w-6 h-px bg-zinc-800 mx-2 hidden sm:block" />
      </div>
    </div>

    <!-- Step 1: Name and Icon -->
    <div v-if="currentStep === 1" class="space-y-4 py-2">
      <div>
        <label class="block text-xs font-semibold text-zinc-300 uppercase tracking-wider mb-2 font-mono">
          Nom de l'Instance
        </label>
        <UiInput
          v-model="instanceName"
          placeholder="Ex: Survie Fabric 1.21.4"
        />
      </div>

      <div>
        <label class="block text-xs font-semibold text-zinc-300 uppercase tracking-wider mb-2 font-mono">
          Icône de Profil
        </label>
        <div class="grid grid-cols-5 gap-2">
          <button
            v-for="icon in availableIcons"
            :key="icon"
            type="button"
            @click="selectedIcon = icon"
            :class="[
              'p-3 text-2xl rounded-xl border text-center transition-all',
              selectedIcon === icon
                ? 'bg-amber-500/20 border-amber-500 shadow-md'
                : 'bg-zinc-950/60 border-zinc-800 hover:border-zinc-700'
            ]"
          >
            {{ icon }}
          </button>
        </div>
      </div>
    </div>

    <!-- Step 2: Minecraft Version -->
    <div v-else-if="currentStep === 2" class="space-y-3 py-2">
      <label class="block text-xs font-semibold text-zinc-300 uppercase tracking-wider mb-1 font-mono">
        Version du Jeu (Java Edition)
      </label>
      <div class="space-y-2 max-h-64 overflow-y-auto pr-1">
        <button
          v-for="v in minecraftVersions"
          :key="v.version"
          type="button"
          @click="selectedVersion = v.version"
          :class="[
            'w-full p-3 rounded-xl border flex items-center justify-between text-left transition-all',
            selectedVersion === v.version
              ? 'bg-amber-500/15 border-amber-500/60 text-amber-300 font-semibold'
              : 'bg-zinc-950/60 border-zinc-800 text-zinc-300 hover:border-zinc-700'
          ]"
        >
          <div>
            <div class="text-sm font-bold">{{ v.version }}</div>
            <div class="text-xs text-zinc-500">{{ v.type }}</div>
          </div>
          <UiBadge v-if="v.recommended" variant="amber" size="xs">
            Recommandé
          </UiBadge>
        </button>
      </div>
    </div>

    <!-- Step 3: Mod Loader Selection -->
    <div v-else-if="currentStep === 3" class="space-y-3 py-2">
      <label class="block text-xs font-semibold text-zinc-300 uppercase tracking-wider mb-1 font-mono">
        Sélection du Mod Loader
      </label>
      <div class="space-y-2">
        <button
          v-for="loader in loaders"
          :key="loader.id"
          type="button"
          @click="selectedLoader = loader.id"
          :class="[
            'w-full p-3.5 rounded-xl border text-left transition-all flex items-start justify-between gap-4',
            selectedLoader === loader.id
              ? 'bg-amber-500/15 border-amber-500/60'
              : 'bg-zinc-950/60 border-zinc-800 hover:border-zinc-700'
          ]"
        >
          <div>
            <div class="flex items-center gap-2">
              <span
                :class="[
                  'text-sm font-bold',
                  selectedLoader === loader.id ? 'text-amber-300' : 'text-zinc-200'
                ]"
              >
                {{ loader.name }}
              </span>
              <UiBadge variant="zinc" size="xs">{{ loader.badge }}</UiBadge>
            </div>
            <p class="text-xs text-zinc-400 mt-1 leading-relaxed">{{ loader.desc }}</p>
          </div>
          <div
            :class="[
              'w-5 h-5 rounded-full border-2 flex items-center justify-center flex-shrink-0 mt-0.5',
              selectedLoader === loader.id
                ? 'border-amber-400 bg-amber-400'
                : 'border-zinc-700'
            ]"
          >
            <div v-if="selectedLoader === loader.id" class="w-2 h-2 rounded-full bg-zinc-950" />
          </div>
        </button>
      </div>
    </div>

    <!-- Step 4: RAM and JVM settings -->
    <div v-else-if="currentStep === 4" class="space-y-5 py-2">
      <div>
        <div class="flex items-center justify-between mb-2">
          <label class="text-xs font-semibold text-zinc-300 uppercase tracking-wider font-mono">
            Mémoire RAM Allouée
          </label>
          <span class="text-sm font-bold text-amber-400 font-mono">{{ allocatedRam }} Go</span>
        </div>
        <input
          v-model.number="allocatedRam"
          type="range"
          min="2"
          max="24"
          step="1"
          class="w-full accent-amber-500 cursor-pointer"
        />
        <div class="flex justify-between text-[11px] text-zinc-500 mt-1">
          <span>2 Go (Minimum)</span>
          <span>6-8 Go (Recommandé)</span>
          <span>24 Go (Lourd)</span>
        </div>
      </div>

      <!-- Recap Card -->
      <div class="p-4 rounded-xl bg-zinc-950/80 border border-zinc-800/80 space-y-2 text-xs">
        <div class="font-bold text-zinc-200 flex items-center gap-2">
          <span>{{ selectedIcon }}</span>
          <span>{{ instanceName }}</span>
        </div>
        <div class="grid grid-cols-2 gap-2 text-zinc-400 text-[11px]">
          <div>Version: <span class="text-zinc-200 font-semibold">{{ selectedVersion }}</span></div>
          <div>Loader: <span class="text-amber-400 font-semibold uppercase">{{ selectedLoader }}</span></div>
          <div>RAM: <span class="text-zinc-200 font-semibold">{{ allocatedRam }} Go</span></div>
          <div>Environnement: <span class="text-emerald-400 font-semibold">Linux Native Java 21</span></div>
        </div>
      </div>
    </div>

    <template #footer>
      <UiButton
        v-if="currentStep > 1"
        variant="ghost"
        size="sm"
        @click="prevStep"
      >
        Retour
      </UiButton>
      <UiButton
        variant="primary"
        size="sm"
        @click="nextStep"
      >
        {{ currentStep === 4 ? 'Créer l\'instance' : 'Suivant' }}
      </UiButton>
    </template>
  </UiModal>
</template>
