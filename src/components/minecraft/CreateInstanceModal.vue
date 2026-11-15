<script setup lang="ts">
import { ref, computed, watch, onMounted } from 'vue'
import {
  Box,
  Cpu,
  Sliders,
  ChevronDown,
  Sparkles,
  Check,
  Package,
  UploadCloud,
  FileArchive,
  Search,
  Loader2,
  AlertCircle
} from 'lucide-vue-next'
import { useMinecraftStore } from '@/stores/minecraft'
import type { MinecraftLoader } from '@/models/minecraft'
import ModpackCard from '@/components/minecraft/ModpackCard.vue'
import UiModal from '@/components/ui/UiModal.vue'
import UiButton from '@/components/ui/UiButton.vue'

const props = defineProps<{
  open: boolean
}>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'created', instanceId: string): void
}>()

const minecraftStore = useMinecraftStore()

// Tab Mode
const creationMode = ref<'modpacks' | 'custom' | 'import'>('modpacks')

// --- Mode 1: Modpack Catalog ---
const modpackQuery = ref('')
const selectedProvider = ref<'all' | 'modrinth' | 'curseforge'>('all')
const modpacks = computed(() => minecraftStore.modpackSearchResults)
const isSearchingModpacks = computed(() => minecraftStore.isSearchingModpacks)

function handleModpackSearch() {
  minecraftStore.searchModpacks(modpackQuery.value, selectedProvider.value)
}

function setProvider(p: 'all' | 'modrinth' | 'curseforge') {
  selectedProvider.value = p
  handleModpackSearch()
}

function onModpackInstalled(instId?: string) {
  if (instId) {
    emit('created', instId)
    emit('close')
  }
}

// --- Mode 2: Custom Instance ---
const name = ref('Mon Instance Minecraft')
const mcVersion = ref('1.20.1')
const modloader = ref<MinecraftLoader>('fabric')
const loaderVersion = ref('')
const ramGb = ref(4)
const isSubmitting = ref(false)
const showAdvanced = ref(false)
const jvmArgs = ref(
  '-XX:+UseG1GC -XX:+UnlockExperimentalVMOptions -XX:G1NewSizePercent=20 -XX:G1ReservePercent=20 -XX:MaxGCPauseMillis=50 -XX:G1HeapRegionSize=32M'
)

const availableMcVersions = [
  '26.3',
  '26.1',
  '1.21.4',
  '1.21.1',
  '1.21',
  '1.20.6',
  '1.20.4',
  '1.20.2',
  '1.20.1',
  '1.19.4',
  '1.19.2',
  '1.18.2',
  '1.16.5',
  '1.12.2',
  '1.8.9',
  '1.7.10'
]

const selectedJavaPath = ref('')
const customJavaInput = ref('')
const detectedRuntimes = computed(() => minecraftStore.detectedJavaRuntimes)
const requiredMajor = computed(() => minecraftStore.getRequiredJavaVersion(mcVersion.value))

const loaders: Array<{
  id: MinecraftLoader
  name: string
  desc: string
  badge?: string
}> = [
  { id: 'fabric', name: 'Fabric', desc: 'Moteur moderne, ultra-rapide et modulaire', badge: 'Recommandé' },
  { id: 'neoforge', name: 'NeoForge', desc: 'Le fork moderne et dynamique de Forge', badge: '1.20.2+' },
  { id: 'forge', name: 'Forge', desc: 'Le modloader classique pour les grands modpacks' },
  { id: 'quilt', name: 'Quilt', desc: 'Écosystème communautaire compatible Fabric' },
  { id: 'vanilla', name: 'Vanilla', desc: 'Jeu original pur sans aucun modloader' }
]

async function handleCreateCustom() {
  if (!name.value.trim()) return

  isSubmitting.value = true
  try {
    const minRamMb = 2048
    const maxRamMb = ramGb.value * 1024
    const argsArray = jvmArgs.value
      .split(' ')
      .map((s) => s.trim())
      .filter((s) => s.length > 0)

    const finalJava = selectedJavaPath.value === 'custom'
      ? customJavaInput.value.trim() || undefined
      : selectedJavaPath.value.trim() || undefined

    const newInst = await minecraftStore.createInstance({
      name: name.value.trim(),
      mcVersion: mcVersion.value,
      modloader: modloader.value,
      loaderVersion: loaderVersion.value.trim() || undefined,
      minRamMb,
      maxRamMb,
      jvmArgs: argsArray,
      javaPath: finalJava
    })

    emit('created', newInst.id)
    emit('close')
  } catch (err) {
    console.error('Failed to create instance:', err)
  } finally {
    isSubmitting.value = false
  }
}

// --- Mode 3: Local Archive Import ---
const localFile = ref<File | null>(null)
const isDragging = ref(false)
const isImporting = ref(false)
const importError = ref<string | null>(null)

function onFileSelect(e: Event) {
  const target = e.target as HTMLInputElement
  if (target.files && target.files.length > 0) {
    localFile.value = target.files[0]
    importError.value = null
  }
}

function onDrop(e: DragEvent) {
  isDragging.value = false
  if (e.dataTransfer && e.dataTransfer.files.length > 0) {
    const f = e.dataTransfer.files[0]
    if (f.name.endsWith('.mrpack') || f.name.endsWith('.zip')) {
      localFile.value = f
      importError.value = null
    } else {
      importError.value = 'Veuillez déposer un fichier d\'archive .mrpack ou .zip valide.'
    }
  }
}

async function handleImportLocal() {
  if (!localFile.value) return
  isImporting.value = true
  importError.value = null
  try {
    const inst = await minecraftStore.importLocalModpack(localFile.value)
    emit('created', inst.id)
    emit('close')
  } catch (err: any) {
    importError.value = err?.message || 'Erreur lors de l\'importation de l\'archive.'
  } finally {
    isImporting.value = false
  }
}

watch(
  () => props.open,
  (isOpen) => {
    if (isOpen && modpacks.value.length === 0) {
      handleModpackSearch()
    }
  }
)

onMounted(() => {
  minecraftStore.fetchDetectedJavaRuntimes()
  if (props.open && modpacks.value.length === 0) {
    handleModpackSearch()
  }
})
</script>

<template>
  <UiModal
    :open="open"
    title="Nouvelle Instance Minecraft"
    subtitle="Installez un modpack prêt à l'emploi ou configurez un profil sur-mesure"
    max-width="3xl"
    @close="emit('close')"
  >
    <!-- Tab Switcher -->
    <div class="flex items-center gap-2 p-1 bg-zinc-950 rounded-2xl border border-zinc-800/80 mb-6">
      <button
        type="button"
        @click="creationMode = 'modpacks'"
        :class="[
          'flex-1 flex items-center justify-center gap-2 py-2.5 px-3 rounded-xl text-xs font-mono font-bold transition-all select-none',
          creationMode === 'modpacks'
            ? 'bg-amber-500 text-zinc-950 shadow-md shadow-amber-500/10'
            : 'text-zinc-400 hover:text-zinc-200 hover:bg-zinc-900'
        ]"
      >
        <Package class="w-4 h-4" />
        <span>Catalogue Modpacks</span>
        <span class="text-[10px] px-1.5 py-0.2 bg-zinc-950/40 rounded-full font-bold ml-1">
          1-Clic
        </span>
      </button>

      <button
        type="button"
        @click="creationMode = 'custom'"
        :class="[
          'flex-1 flex items-center justify-center gap-2 py-2.5 px-3 rounded-xl text-xs font-mono font-bold transition-all select-none',
          creationMode === 'custom'
            ? 'bg-amber-500 text-zinc-950 shadow-md shadow-amber-500/10'
            : 'text-zinc-400 hover:text-zinc-200 hover:bg-zinc-900'
        ]"
      >
        <Box class="w-4 h-4" />
        <span>Instance Vierge</span>
      </button>

      <button
        type="button"
        @click="creationMode = 'import'"
        :class="[
          'flex-1 flex items-center justify-center gap-2 py-2.5 px-3 rounded-xl text-xs font-mono font-bold transition-all select-none',
          creationMode === 'import'
            ? 'bg-amber-500 text-zinc-950 shadow-md shadow-amber-500/10'
            : 'text-zinc-400 hover:text-zinc-200 hover:bg-zinc-900'
        ]"
      >
        <FileArchive class="w-4 h-4" />
        <span>Importer Archive</span>
        <span class="text-[10px] text-zinc-400 font-normal">.mrpack / .zip</span>
      </button>
    </div>

    <!-- ============================================== -->
    <!-- TAB 1: MODPACK CATALOG                         -->
    <!-- ============================================== -->
    <div v-if="creationMode === 'modpacks'" class="space-y-5">
      <!-- Search & Filter Bar -->
      <div class="flex flex-col sm:flex-row items-stretch sm:items-center justify-between gap-3 bg-zinc-900/60 p-3 rounded-2xl border border-zinc-800">
        <form @submit.prevent="handleModpackSearch" class="relative flex-1">
          <input
            v-model="modpackQuery"
            type="text"
            placeholder="Rechercher (Fabulously, Better MC, All The Mods, DawnCraft...)"
            class="w-full bg-zinc-950 border border-zinc-800 rounded-xl pl-10 pr-20 py-2.5 text-xs text-zinc-100 placeholder-zinc-500 focus:outline-none focus:border-amber-500/80 transition-colors"
          />
          <Search class="w-4 h-4 text-zinc-500 absolute left-3.5 top-1/2 -translate-y-1/2 pointer-events-none" />

          <UiButton
            type="submit"
            variant="primary"
            size="sm"
            class="absolute right-1.5 top-1/2 -translate-y-1/2 text-xs py-1 px-3"
          >
            Chercher
          </UiButton>
        </form>

        <div class="flex items-center gap-1 bg-zinc-950 p-1 rounded-xl border border-zinc-800 shrink-0">
          <button
            type="button"
            @click="setProvider('all')"
            :class="[
              'px-2.5 py-1 rounded-lg text-xs font-mono font-bold transition-colors select-none',
              selectedProvider === 'all'
                ? 'bg-amber-500 text-zinc-950 shadow-sm'
                : 'text-zinc-400 hover:text-zinc-200'
            ]"
          >
            Tous
          </button>
          <button
            type="button"
            @click="setProvider('modrinth')"
            :class="[
              'px-2.5 py-1 rounded-lg text-xs font-mono font-bold transition-colors flex items-center gap-1 select-none',
              selectedProvider === 'modrinth'
                ? 'bg-emerald-500 text-zinc-950 shadow-sm'
                : 'text-zinc-400 hover:text-emerald-400'
            ]"
          >
            <span class="w-1.5 h-1.5 rounded-full bg-emerald-400" />
            Modrinth
          </button>
          <button
            type="button"
            @click="setProvider('curseforge')"
            :class="[
              'px-2.5 py-1 rounded-lg text-xs font-mono font-bold transition-colors flex items-center gap-1 select-none',
              selectedProvider === 'curseforge'
                ? 'bg-amber-500 text-zinc-950 shadow-sm'
                : 'text-zinc-400 hover:text-amber-400'
            ]"
          >
            <span class="w-1.5 h-1.5 rounded-full bg-amber-400" />
            CurseForge
          </button>
        </div>
      </div>

      <!-- Loading State -->
      <div v-if="isSearchingModpacks" class="py-16 text-center">
        <Loader2 class="w-8 h-8 text-amber-500 animate-spin mx-auto mb-3" />
        <p class="text-sm text-zinc-300 font-mono">Exploration des catalogues Modrinth & CurseForge...</p>
      </div>

      <!-- Modpack Grid -->
      <div
        v-else-if="modpacks.length > 0"
        class="grid grid-cols-1 md:grid-cols-2 gap-4 max-h-[55vh] overflow-y-auto pr-1"
      >
        <ModpackCard
          v-for="pack in modpacks"
          :key="`${pack.source}-${pack.id}`"
          :modpack="pack"
          @installed="onModpackInstalled"
        />
      </div>

      <!-- Empty State -->
      <div v-else class="py-16 text-center bg-zinc-900/30 rounded-3xl border border-zinc-800/80">
        <Package class="w-10 h-10 text-zinc-600 mx-auto mb-3" />
        <h3 class="text-sm font-bold text-zinc-200">Aucun modpack trouvé</h3>
        <p class="text-xs text-zinc-500 max-w-sm mx-auto mt-1">
          Essayez d'ajuster votre recherche ou filtrez par fournisseur spécifique.
        </p>
      </div>
    </div>

    <!-- ============================================== -->
    <!-- TAB 2: CUSTOM INSTANCE (ORIGINAL FORM)         -->
    <!-- ============================================== -->
    <form v-else-if="creationMode === 'custom'" @submit.prevent="handleCreateCustom" class="space-y-6 py-2">
      <!-- 1. Nom de l'instance -->
      <div class="space-y-2">
        <label class="block text-xs font-mono font-bold text-zinc-300 uppercase tracking-wider">
          Nom de l'environnement
        </label>
        <div class="relative">
          <input
            v-model="name"
            type="text"
            required
            placeholder="Ex: Fabric Aventure 1.20.1"
            class="w-full bg-zinc-900 border border-zinc-800 rounded-2xl px-4 py-3 text-sm text-zinc-100 placeholder-zinc-500 focus:outline-none focus:border-amber-500/80 transition-colors"
          />
          <Box class="w-4 h-4 text-zinc-500 absolute right-4 top-1/2 -translate-y-1/2 pointer-events-none" />
        </div>
      </div>

      <!-- 2. Choix de la version Minecraft -->
      <div class="space-y-2">
        <label class="block text-xs font-mono font-bold text-zinc-300 uppercase tracking-wider">
          Version de Minecraft: Java Edition
        </label>
        <div class="relative">
          <select
            v-model="mcVersion"
            class="w-full bg-zinc-900 border border-zinc-800 rounded-2xl px-4 py-3 text-sm text-zinc-100 appearance-none focus:outline-none focus:border-amber-500/80 transition-colors cursor-pointer"
          >
            <option v-for="ver in availableMcVersions" :key="ver" :value="ver">
              Minecraft {{ ver }} {{ ver === '1.20.1' ? '(Recommandée pour les mods)' : '' }}
            </option>
          </select>
          <ChevronDown class="w-4 h-4 text-zinc-500 absolute right-4 top-1/2 -translate-y-1/2 pointer-events-none" />
        </div>
      </div>

      <!-- 3. Choix du Modloader -->
      <div class="space-y-2">
        <label class="block text-xs font-mono font-bold text-zinc-300 uppercase tracking-wider flex items-center justify-between">
          <span>Modloader</span>
          <span class="text-[11px] text-amber-400 font-normal">Compatible Modrinth & CurseForge</span>
        </label>
        <div class="grid grid-cols-1 sm:grid-cols-2 gap-2.5">
          <button
            v-for="l in loaders"
            :key="l.id"
            type="button"
            @click="modloader = l.id"
            :class="[
              'p-3.5 rounded-2xl border text-left transition-all flex flex-col justify-between gap-1.5 relative select-none',
              modloader === l.id
                ? 'bg-amber-500/10 border-amber-500/80 shadow-md shadow-amber-500/10'
                : 'bg-zinc-900/60 border-zinc-800/80 hover:border-zinc-700 hover:bg-zinc-900'
            ]"
          >
            <div class="flex items-center justify-between">
              <span class="font-bold text-sm text-zinc-100 flex items-center gap-1.5">
                {{ l.name }}
              </span>
              <span
                v-if="l.badge"
                class="px-1.5 py-0.5 rounded text-[10px] font-mono font-bold bg-amber-500/20 text-amber-400 border border-amber-500/30"
              >
                {{ l.badge }}
              </span>
              <Check v-else-if="modloader === l.id" class="w-4 h-4 text-amber-400" />
            </div>
            <p class="text-[11px] text-zinc-400 leading-tight">
              {{ l.desc }}
            </p>
          </button>
        </div>
      </div>

      <!-- 4. Allocation Mémoire RAM -->
      <div class="space-y-3 bg-zinc-900/50 p-4 rounded-2xl border border-zinc-800/80">
        <div class="flex items-center justify-between">
          <label class="text-xs font-mono font-bold text-zinc-300 uppercase tracking-wider flex items-center gap-2">
            <Cpu class="w-4 h-4 text-amber-400" />
            <span>Allocation RAM Maximale</span>
          </label>
          <span class="font-mono text-sm font-bold text-amber-400">
            {{ ramGb }} Go
          </span>
        </div>

        <input
          v-model.number="ramGb"
          type="range"
          min="2"
          max="16"
          step="1"
          class="w-full accent-amber-500 cursor-pointer h-2 bg-zinc-800 rounded-lg appearance-none"
        />

        <div class="flex justify-between text-[10px] font-mono text-zinc-500">
          <span>2 Go (Min)</span>
          <span>4 Go (Idéal)</span>
          <span>8 Go (Gros Packs)</span>
          <span>16 Go (Max)</span>
        </div>
      </div>

      <!-- 5. Paramètres Avancés (JVM Args) -->
      <div class="border-t border-zinc-800/80 pt-3">
        <button
          type="button"
          @click="showAdvanced = !showAdvanced"
          class="text-xs font-mono text-zinc-400 hover:text-zinc-200 flex items-center gap-1.5 transition-colors select-none"
        >
          <Sliders class="w-3.5 h-3.5 text-amber-400" />
          <span>{{ showAdvanced ? 'Masquer les options JVM avancées' : 'Afficher les options JVM avancées' }}</span>
        </button>

        <div v-if="showAdvanced" class="mt-3 space-y-3 pl-1">
          <div class="space-y-1">
            <label class="block text-[11px] font-mono text-zinc-400">Arguments JVM (-XX:...)</label>
            <textarea
              v-model="jvmArgs"
              rows="3"
              class="w-full bg-zinc-900 border border-zinc-800 rounded-xl p-3 text-xs font-mono text-zinc-200 focus:outline-none focus:border-amber-500/80"
              placeholder="Arguments JVM séparés par des espaces"
            ></textarea>
          </div>

          <!-- Java Runtime selection -->
          <div class="space-y-1.5 pt-2">
            <div class="flex items-center justify-between">
              <label class="block text-[11px] font-mono text-zinc-400">Runtime Java (JRE / JDK)</label>
              <span class="text-[10px] font-mono text-amber-400">Requis : Java {{ requiredMajor }}</span>
            </div>
            <select
              v-model="selectedJavaPath"
              class="w-full bg-zinc-900 border border-zinc-800 rounded-xl px-3 py-2.5 text-xs font-mono text-zinc-200 focus:outline-none focus:border-amber-500/80 cursor-pointer"
            >
              <option value="">
                Automatique (Recommandé : Java {{ requiredMajor }})
              </option>
              <option
                v-for="j in detectedRuntimes"
                :key="j.path"
                :value="j.path"
              >
                Java {{ j.major }} ({{ j.versionStr }}) — {{ j.vendor }} {{ j.major === requiredMajor ? '★ Recommandé' : '' }}
              </option>
              <option value="custom">
                Spécifier un chemin manuel...
              </option>
            </select>

            <input
              v-if="selectedJavaPath === 'custom'"
              v-model="customJavaInput"
              type="text"
              placeholder="/chemin/absolu/vers/java"
              class="w-full mt-2 bg-zinc-950 border border-zinc-800 rounded-xl px-3 py-2 text-xs font-mono text-zinc-200 focus:outline-none focus:border-amber-500/80"
            />
          </div>
        </div>
      </div>

      <!-- Actions Footer -->
      <div class="flex items-center justify-end gap-3 pt-3">
        <UiButton
          type="button"
          variant="outline"
          size="md"
          @click="emit('close')"
        >
          Annuler
        </UiButton>

        <UiButton
          type="submit"
          variant="primary"
          size="md"
          :disabled="isSubmitting"
        >
          <template #prefix>
            <Sparkles class="w-4 h-4" />
          </template>
          {{ isSubmitting ? 'Création de l\'instance...' : 'Créer l\'instance' }}
        </UiButton>
      </div>
    </form>

    <!-- ============================================== -->
    <!-- TAB 3: IMPORT LOCAL ARCHIVE (.mrpack / .zip)  -->
    <!-- ============================================== -->
    <div v-else-if="creationMode === 'import'" class="space-y-6 py-2">
      <!-- Dropzone -->
      <div
        @dragover.prevent="isDragging = true"
        @dragleave.prevent="isDragging = false"
        @drop.prevent="onDrop"
        :class="[
          'relative border-2 border-dashed rounded-3xl p-8 text-center transition-all flex flex-col items-center justify-center cursor-pointer select-none',
          isDragging
            ? 'border-amber-500 bg-amber-500/10'
            : localFile
            ? 'border-emerald-500/60 bg-emerald-500/5'
            : 'border-zinc-800 hover:border-zinc-700 bg-zinc-900/40'
        ]"
      >
        <input
          type="file"
          accept=".mrpack,.zip"
          class="absolute inset-0 opacity-0 cursor-pointer w-full h-full"
          @change="onFileSelect"
        />

        <div v-if="localFile" class="flex flex-col items-center gap-2">
          <div class="w-14 h-14 rounded-2xl bg-emerald-500/20 text-emerald-400 flex items-center justify-center border border-emerald-500/30">
            <FileArchive class="w-7 h-7" />
          </div>
          <span class="font-bold text-sm text-zinc-100 mt-2">{{ localFile.name }}</span>
          <span class="text-xs font-mono text-zinc-400">{{ (localFile.size / (1024 * 1024)).toFixed(1) }} Mo</span>
          <span class="text-[11px] text-emerald-400 font-medium">Archive prête à l'installation</span>
        </div>

        <div v-else class="flex flex-col items-center gap-2">
          <div class="w-14 h-14 rounded-2xl bg-zinc-900 text-zinc-400 flex items-center justify-center border border-zinc-800">
            <UploadCloud class="w-7 h-7" />
          </div>
          <span class="font-bold text-sm text-zinc-200 mt-2">Glissez-déposez votre archive ici</span>
          <p class="text-xs text-zinc-500 max-w-sm">
            Formats supportés : modpacks Modrinth <span class="text-amber-400 font-mono">.mrpack</span> et CurseForge <span class="text-amber-400 font-mono">.zip</span> avec extraction complète des configs et datapacks.
          </p>
          <span class="mt-2 px-3 py-1 bg-zinc-800 text-zinc-300 text-xs rounded-xl font-mono">
            Parcourir les fichiers
          </span>
        </div>
      </div>

      <!-- Error message -->
      <div v-if="importError" class="p-3 bg-rose-500/10 border border-rose-500/30 rounded-2xl flex items-center gap-2 text-xs text-rose-300">
        <AlertCircle class="w-4 h-4 text-rose-400 shrink-0" />
        <span>{{ importError }}</span>
      </div>

      <!-- Actions -->
      <div class="flex items-center justify-end gap-3 pt-3 border-t border-zinc-800/80">
        <UiButton
          type="button"
          variant="outline"
          size="md"
          @click="emit('close')"
        >
          Annuler
        </UiButton>

        <UiButton
          type="button"
          variant="primary"
          size="md"
          :disabled="!localFile || isImporting"
          :loading="isImporting"
          @click="handleImportLocal"
        >
          <template #prefix>
            <Sparkles class="w-4 h-4" />
          </template>
          {{ isImporting ? 'Installation du modpack...' : 'Installer le Modpack' }}
        </UiButton>
      </div>
    </div>
  </UiModal>
</template>
