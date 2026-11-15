<script setup lang="ts">
import { ref } from 'vue'
import { UploadCloud, PackageOpen, Layers } from 'lucide-vue-next'
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

const creationMode = ref<'custom' | 'modpack'>('custom')
const currentStep = ref<1 | 2 | 3 | 4>(1)

// Form fields
const instanceName = ref('Nouvelle Aventure')
const selectedIcon = ref('⛏️')
const selectedVersion = ref('1.20.1')
const selectedLoader = ref<MinecraftLoader>('fabric')
const allocatedRam = ref(6)

// Modpack import fields
const modpackFile = ref<File | null>(null)
const modpackName = ref('')
const isImportingModpack = ref(false)

const availableIcons = ['⛏️', '🚀', '⚡', '🐉', '🏰', '🛡️', '⚔️', '🧪', '💎', '🔥']

const minecraftVersions = [
  { version: '1.21.4', type: 'Release (Latest)', recommended: false },
  { version: '1.21.1', type: 'Release' },
  { version: '1.20.1', type: 'Release (Idéal Modding)', recommended: true },
  { version: '1.19.2', type: 'Release' },
  { version: '1.16.5', type: 'Legacy LTS' }
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
    badge: 'Moderne'
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

async function finish() {
  const newInst = await minecraftStore.createInstance({
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

function onFileSelect(e: Event) {
  const target = e.target as HTMLInputElement
  if (target.files && target.files[0]) {
    modpackFile.value = target.files[0]
    modpackName.value = target.files[0].name.replace(/\.(mrpack|zip)$/i, '')
  }
}

async function handleModpackImport() {
  if (!modpackFile.value) return
  isImportingModpack.value = true

  try {
    const newInst = await minecraftStore.createInstance({
      name: modpackName.value || 'Modpack Importé',
      icon: '📦',
      gameVersion: '1.20.1',
      loader: 'fabric',
      allocatedRamGb: 8
    })

    await minecraftStore.importLocalFile(newInst.id, modpackFile.value)
    emit('created', newInst.id)
    emit('close')
  } finally {
    isImportingModpack.value = false
  }
}
</script>

<template>
  <UiModal
    :open="open"
    title="Assistant de Création d'Instance"
    subtitle="Configurez votre environnement Minecraft personnalisé ou importez une archive"
    max-width="xl"
    @close="emit('close')"
  >
    <!-- Mode selection pills -->
    <div class="flex items-center gap-2 pb-3 border-b border-zinc-800">
      <button
        type="button"
        @click="creationMode = 'custom'"
        :class="[
          'px-3.5 py-1.5 rounded-xl text-xs font-bold transition-all flex items-center gap-2',
          creationMode === 'custom'
            ? 'bg-amber-500 text-zinc-950 shadow-md shadow-amber-500/20'
            : 'bg-zinc-900 text-zinc-400 hover:text-zinc-200'
        ]"
      >
        <Layers class="w-3.5 h-3.5" />
        Configuration Personnalisée
      </button>

      <button
        type="button"
        @click="creationMode = 'modpack'"
        :class="[
          'px-3.5 py-1.5 rounded-xl text-xs font-bold transition-all flex items-center gap-2',
          creationMode === 'modpack'
            ? 'bg-amber-500 text-zinc-950 shadow-md shadow-amber-500/20'
            : 'bg-zinc-900 text-zinc-400 hover:text-zinc-200'
        ]"
      >
        <PackageOpen class="w-3.5 h-3.5" />
        Importer un Modpack (.mrpack / .zip)
      </button>
    </div>

    <!-- MODE A : CUSTOM WIZARD -->
    <div v-if="creationMode === 'custom'" class="space-y-4 pt-3">
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
            placeholder="Ex: Survie Fabric 1.20.1"
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
            <div>Provisioning Java: <span class="text-emerald-400 font-semibold">Adoptium Temurin Automatique</span></div>
          </div>
        </div>
      </div>
    </div>

    <!-- MODE B : MODPACK ARCHIVE IMPORT -->
    <div v-else class="space-y-5 py-4">
      <div class="p-6 rounded-2xl border-2 border-dashed border-zinc-800 hover:border-amber-500/50 bg-zinc-950/60 text-center space-y-3 cursor-pointer relative">
        <input
          type="file"
          accept=".mrpack,.zip"
          @change="onFileSelect"
          class="absolute inset-0 opacity-0 cursor-pointer"
        />
        <div class="w-12 h-12 mx-auto rounded-2xl bg-zinc-900 border border-zinc-800 flex items-center justify-center text-amber-400">
          <UploadCloud class="w-6 h-6" />
        </div>
        <div>
          <div class="font-bold text-sm text-zinc-200">
            {{ modpackFile ? modpackFile.name : 'Sélectionnez une archive .mrpack ou .zip' }}
          </div>
          <p class="text-xs text-zinc-500 mt-1">
            Support des modpacks Modrinth v2 et CurseForge Core avec extraction automatique des overrides
          </p>
        </div>
      </div>

      <div v-if="modpackFile" class="space-y-3">
        <div>
          <label class="block text-xs font-semibold text-zinc-300 uppercase tracking-wider mb-2 font-mono">
            Nom de l'Instance
          </label>
          <UiInput
            v-model="modpackName"
            placeholder="Nom personnalisé..."
          />
        </div>

        <div class="p-3.5 rounded-xl bg-zinc-900/80 border border-zinc-800 text-xs text-zinc-400 flex items-center justify-between">
          <span>Taille de l'archive:</span>
          <span class="font-mono text-zinc-200 font-bold">
            {{ (modpackFile.size / (1024 * 1024)).toFixed(1) }} Mo
          </span>
        </div>
      </div>
    </div>

    <template #footer>
      <div v-if="creationMode === 'custom'" class="flex items-center justify-between w-full">
        <UiButton
          v-if="currentStep > 1"
          variant="ghost"
          size="sm"
          @click="prevStep"
        >
          Retour
        </UiButton>
        <div v-else />

        <UiButton
          variant="primary"
          size="sm"
          @click="nextStep"
        >
          {{ currentStep === 4 ? 'Créer l\'instance' : 'Suivant' }}
        </UiButton>
      </div>

      <div v-else class="flex items-center justify-end w-full gap-2">
        <UiButton
          variant="ghost"
          size="sm"
          @click="emit('close')"
        >
          Annuler
        </UiButton>
        <UiButton
          variant="primary"
          size="sm"
          :disabled="!modpackFile"
          :loading="isImportingModpack"
          @click="handleModpackImport"
        >
          Importer & Déployer
        </UiButton>
      </div>
    </template>
  </UiModal>
</template>
