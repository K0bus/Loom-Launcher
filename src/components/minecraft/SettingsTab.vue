<script setup lang="ts">
import { ref, computed, watch, onMounted } from 'vue'
import {
  Cpu,
  Save,
  Check,
  RotateCcw,
  Sliders,
  ShieldCheck,
  Edit3,
  AlertTriangle,
  AlertCircle,
  Trash2,
  PowerOff,
  Coffee,
  RefreshCw,
  Image as ImageIcon
} from 'lucide-vue-next'
import { useMinecraftStore } from '@/stores/minecraft'
import UiButton from '@/components/ui/UiButton.vue'
import UiBadge from '@/components/ui/UiBadge.vue'

const minecraftStore = useMinecraftStore()
const activeInst = computed(() => minecraftStore.activeInstance)

const presets = [
  { name: 'Minecraft IGDB (Défaut)', url: '' },
  { name: 'Nether Realm', url: 'https://images.unsplash.com/photo-1518709268805-4e9042af9f23?q=80&w=1200&auto=format&fit=crop' },
  { name: 'Lush Caves & Shaders', url: 'https://images.unsplash.com/photo-1579373903781-fd5c0c30c4cd?q=80&w=1200&auto=format&fit=crop' },
  { name: 'Cherry Grove', url: 'https://images.unsplash.com/photo-1522383225653-ed111181a951?q=80&w=1200&auto=format&fit=crop' },
  { name: 'Deep Forest', url: 'https://images.unsplash.com/photo-1448375240586-882707db888b?q=80&w=1200&auto=format&fit=crop' }
]

const instanceName = ref('')
const bannerUrl = ref('')
const minRamMb = ref(2048)
const maxRamMb = ref(4096)
const jvmArgsText = ref('')
const customJavaPath = ref('')
const javaSelectionMode = ref<'auto' | 'detected' | 'custom'>('auto')
const isSaved = ref(false)
const confirmDelete = ref(false)

// Java Runtimes & Matching
const detectedRuntimes = computed(() => minecraftStore.detectedJavaRuntimes)
const isDetectingJava = computed(() => minecraftStore.isDetectingJava)
const requiredMajor = computed(() => {
  return activeInst.value ? minecraftStore.getRequiredJavaVersion(activeInst.value.gameVersion) : 21
})

const autoMatchedJava = computed(() => {
  return (
    detectedRuntimes.value.find((j) => j.major === requiredMajor.value) ||
    detectedRuntimes.value.find((j) => j.major >= requiredMajor.value) ||
    null
  )
})

const selectedDetectedRuntime = computed(() => {
  return detectedRuntimes.value.find((j) => j.path === customJavaPath.value) || null
})

const customPathInfo = ref<{ valid: boolean; runtime?: any } | null>(null)
let checkDebounce: any = null

async function checkCustomPath() {
  if (!customJavaPath.value.trim()) {
    customPathInfo.value = null
    return
  }
  const res = await minecraftStore.inspectJavaPath(customJavaPath.value.trim())
  customPathInfo.value = res
}

function onCustomPathInput() {
  clearTimeout(checkDebounce)
  checkDebounce = setTimeout(checkCustomPath, 500)
}

function selectJavaMode(mode: 'auto' | 'detected' | 'custom') {
  javaSelectionMode.value = mode
  if (mode === 'auto') {
    customJavaPath.value = ''
    customPathInfo.value = null
  } else if (mode === 'detected') {
    customJavaPath.value = autoMatchedJava.value?.path || detectedRuntimes.value[0]?.path || ''
    customPathInfo.value = null
  } else if (mode === 'custom') {
    if (!customJavaPath.value) {
      customJavaPath.value = '/usr/bin/java'
    }
    checkCustomPath()
  }
}

function loadFromInstance() {
  if (!activeInst.value) return
  instanceName.value = activeInst.value.name || ''
  bannerUrl.value = activeInst.value.bannerUrl || ''
  maxRamMb.value = (activeInst.value.allocatedRamGb || 4) * 1024
  minRamMb.value = 2048
  confirmDelete.value = false

  const curPath = activeInst.value.javaPath?.trim() || ''
  customJavaPath.value = curPath

  if (!curPath) {
    javaSelectionMode.value = 'auto'
    customPathInfo.value = null
  } else if (detectedRuntimes.value.some((j) => j.path === curPath)) {
    javaSelectionMode.value = 'detected'
    customPathInfo.value = null
  } else {
    javaSelectionMode.value = 'custom'
    checkCustomPath()
  }

  if (activeInst.value.jvmArgs) {
    try {
      const parsed = JSON.parse(activeInst.value.jvmArgs)
      jvmArgsText.value = Array.isArray(parsed) ? parsed.join(' ') : activeInst.value.jvmArgs
    } catch {
      jvmArgsText.value = activeInst.value.jvmArgs
    }
  } else {
    jvmArgsText.value =
      '-XX:+UseG1GC -XX:+UnlockExperimentalVMOptions -XX:G1NewSizePercent=20 -XX:G1ReservePercent=20 -XX:MaxGCPauseMillis=50 -XX:G1HeapRegionSize=32M'
  }
}

watch(() => activeInst.value?.id, loadFromInstance, { immediate: true })

async function saveSettings() {
  if (!activeInst.value) return

  const argsArray = jvmArgsText.value
    .split(' ')
    .map((s) => s.trim())
    .filter((s) => s.length > 0)

  const finalJava = javaSelectionMode.value === 'auto' ? undefined : customJavaPath.value.trim() || undefined

  await minecraftStore.updateInstanceSettings(activeInst.value.id, {
    name: instanceName.value.trim() || activeInst.value.name,
    bannerUrl: bannerUrl.value.trim() || null,
    minRamMb: minRamMb.value,
    maxRamMb: maxRamMb.value,
    jvmArgs: argsArray,
    javaPath: finalJava
  })

  isSaved.value = true
  setTimeout(() => (isSaved.value = false), 2500)
}

function resetDefaultJvmArgs() {
  jvmArgsText.value =
    '-XX:+UseG1GC -XX:+UnlockExperimentalVMOptions -XX:G1NewSizePercent=20 -XX:G1ReservePercent=20 -XX:MaxGCPauseMillis=50 -XX:G1HeapRegionSize=32M'
}

async function handleResetStatus() {
  if (!activeInst.value) return
  await minecraftStore.resetInstanceStatus(activeInst.value.id)
}

async function handleDeleteInstance() {
  if (!activeInst.value) return
  await minecraftStore.deleteInstance(activeInst.value.id)
}

onMounted(() => {
  minecraftStore.fetchDetectedJavaRuntimes().then(() => {
    loadFromInstance()
  })
})
</script>

<template>
  <div v-if="activeInst" class="space-y-8 max-w-4xl pb-12">
    <!-- 1. Instance Name & Cover Banner -->
    <div class="p-6 rounded-3xl bg-zinc-900/60 border border-zinc-800/80 space-y-5">
      <div class="flex items-center justify-between">
        <h3 class="text-sm font-bold text-zinc-100 flex items-center gap-2">
          <Edit3 class="w-4 h-4 text-amber-400" />
          Identité de l'Instance
        </h3>
        <span class="text-xs font-mono text-zinc-500">ID: {{ activeInst.id }}</span>
      </div>

      <!-- Instance Name Field -->
      <div class="space-y-1.5">
        <label class="block text-xs font-mono text-zinc-400">Nom de l'environnement :</label>
        <input
          v-model="instanceName"
          type="text"
          placeholder="Ex: Mon Instance Fabric"
          class="w-full bg-zinc-950 border border-zinc-800 rounded-2xl px-4 py-2.5 text-sm text-zinc-100 focus:outline-none focus:border-amber-500/80 shadow-inner"
        />
      </div>

      <!-- Banner URL Field -->
      <div class="space-y-1.5 pt-2">
        <label class="block text-xs font-mono text-zinc-400 flex items-center gap-1.5">
          <ImageIcon class="w-3.5 h-3.5 text-zinc-400" />
          <span>Bannière personnalisée (URL) :</span>
        </label>
        <div class="flex items-center gap-2">
          <input
            v-model="bannerUrl"
            type="text"
            placeholder="https://... (ou choisissez un preset ci-dessous)"
            class="flex-1 bg-zinc-950 border border-zinc-800 rounded-2xl px-4 py-2.5 text-xs font-mono text-zinc-100 focus:outline-none focus:border-amber-500/80 shadow-inner"
          />
          <button
            v-if="bannerUrl"
            type="button"
            @click="bannerUrl = ''"
            class="px-3 py-2.5 rounded-2xl bg-zinc-800 hover:bg-zinc-700 text-xs font-mono text-zinc-300 transition"
          >
            Réinitialiser
          </button>
        </div>
      </div>

      <!-- Presets -->
      <div class="space-y-1.5 pt-1">
        <span class="text-[11px] font-mono text-zinc-500 uppercase tracking-wider">Thèmes prédéfinis :</span>
        <div class="flex flex-wrap gap-2">
          <button
            v-for="preset in presets"
            :key="preset.name"
            type="button"
            @click="bannerUrl = preset.url"
            :class="[
              'px-3 py-1.5 rounded-xl text-xs font-mono transition-colors border',
              bannerUrl === preset.url
                ? 'bg-amber-500/20 border-amber-500/50 text-amber-300 font-bold'
                : 'bg-zinc-950/60 border-zinc-800 text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800/60'
            ]"
          >
            {{ preset.name }}
          </button>
        </div>
      </div>
    </div>

    <!-- 2. RAM Allocation Card -->
    <div class="p-6 rounded-3xl bg-zinc-900/60 border border-zinc-800/80 space-y-5">
      <div class="flex items-center justify-between">
        <div class="space-y-1">
          <h3 class="text-sm font-bold text-zinc-100 flex items-center gap-2">
            <Cpu class="w-4 h-4 text-amber-400" />
            Allocation Mémoire JVM
          </h3>
          <p class="text-xs text-zinc-400">
            Définit la quantité de mémoire allouée au tas Java via les arguments <span class="font-mono text-zinc-300">-Xms</span> et <span class="font-mono text-zinc-300">-Xmx</span>.
          </p>
        </div>

        <UiBadge variant="amber" size="md" class="font-mono text-xs font-bold">
          {{ Math.round(maxRamMb / 1024) }} Go RAM
        </UiBadge>
      </div>

      <!-- Slider -->
      <div class="space-y-2 pt-2">
        <input
          v-model.number="maxRamMb"
          type="range"
          min="2048"
          max="16384"
          step="1024"
          class="w-full accent-amber-500 cursor-pointer h-2 bg-zinc-800 rounded-lg appearance-none"
        />
        <div class="flex justify-between text-[11px] font-mono text-zinc-500">
          <span>2 Go</span>
          <span>4 Go (Standard)</span>
          <span>8 Go (Modpacks denses)</span>
          <span>16 Go (Ultra)</span>
        </div>
      </div>
    </div>

    <!-- 3. Java Runtime Environment (JRE / JDK) Card -->
    <div class="p-6 rounded-3xl bg-zinc-900/60 border border-zinc-800/80 space-y-5">
      <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
        <div class="space-y-1">
          <h3 class="text-sm font-bold text-zinc-100 flex items-center gap-2">
            <Coffee class="w-4 h-4 text-amber-400" />
            Environnement d'exécution Java (JRE / JDK)
          </h3>
          <p class="text-xs text-zinc-400">
            Gestion du binaire Java pour exécuter Minecraft {{ activeInst.gameVersion }} et ses modloaders.
          </p>
        </div>

        <!-- Default Matching Badge -->
        <div class="flex items-center gap-2 shrink-0">
          <span class="text-xs font-mono text-zinc-400">Requis :</span>
          <UiBadge variant="amber" size="md" class="font-mono text-xs font-bold">
            Java {{ requiredMajor }} (MC {{ activeInst.gameVersion }})
          </UiBadge>
        </div>
      </div>

      <!-- Selection Mode Tabs -->
      <div class="grid grid-cols-1 sm:grid-cols-3 gap-2.5 pt-1">
        <button
          type="button"
          @click="selectJavaMode('auto')"
          :class="[
            'p-3.5 rounded-2xl border text-left transition-all flex flex-col justify-between gap-1 select-none',
            javaSelectionMode === 'auto'
              ? 'bg-amber-500/10 border-amber-500/80 shadow-md shadow-amber-500/10'
              : 'bg-zinc-950/60 border-zinc-800/80 hover:border-zinc-700'
          ]"
        >
          <div class="flex items-center justify-between">
            <span class="font-bold text-xs text-zinc-100 flex items-center gap-1.5">
              Automatique
            </span>
            <Check v-if="javaSelectionMode === 'auto'" class="w-3.5 h-3.5 text-amber-400" />
          </div>
          <p class="text-[11px] text-zinc-400 leading-tight">
            Matching par défaut : Java {{ requiredMajor }}
          </p>
        </button>

        <button
          type="button"
          @click="selectJavaMode('detected')"
          :class="[
            'p-3.5 rounded-2xl border text-left transition-all flex flex-col justify-between gap-1 select-none',
            javaSelectionMode === 'detected'
              ? 'bg-amber-500/10 border-amber-500/80 shadow-md shadow-amber-500/10'
              : 'bg-zinc-950/60 border-zinc-800/80 hover:border-zinc-700'
          ]"
        >
          <div class="flex items-center justify-between">
            <span class="font-bold text-xs text-zinc-100 flex items-center gap-1.5">
              JVM Détectées
            </span>
            <Check v-if="javaSelectionMode === 'detected'" class="w-3.5 h-3.5 text-amber-400" />
          </div>
          <p class="text-[11px] text-zinc-400 leading-tight">
            {{ detectedRuntimes.length }} runtime{{ detectedRuntimes.length > 1 ? 's' : '' }} système
          </p>
        </button>

        <button
          type="button"
          @click="selectJavaMode('custom')"
          :class="[
            'p-3.5 rounded-2xl border text-left transition-all flex flex-col justify-between gap-1 select-none',
            javaSelectionMode === 'custom'
              ? 'bg-amber-500/10 border-amber-500/80 shadow-md shadow-amber-500/10'
              : 'bg-zinc-950/60 border-zinc-800/80 hover:border-zinc-700'
          ]"
        >
          <div class="flex items-center justify-between">
            <span class="font-bold text-xs text-zinc-100 flex items-center gap-1.5">
              Manuel
            </span>
            <Check v-if="javaSelectionMode === 'custom'" class="w-3.5 h-3.5 text-amber-400" />
          </div>
          <p class="text-[11px] text-zinc-400 leading-tight">
            Chemin personnalisé /usr/bin/java
          </p>
        </button>
      </div>

      <!-- Mode 1: Auto matching preview -->
      <div v-if="javaSelectionMode === 'auto'" class="p-4 bg-zinc-950 rounded-2xl border border-zinc-800 text-xs font-mono space-y-2">
        <div class="flex items-center justify-between text-zinc-400">
          <span>Stratégie de résolution automatique :</span>
          <span class="text-amber-400 font-bold">Java {{ requiredMajor }}</span>
        </div>
        <div v-if="autoMatchedJava" class="flex items-center justify-between pt-2 border-t border-zinc-900 text-[11px]">
          <span class="text-zinc-500">Exécutable sélectionné :</span>
          <span class="text-zinc-300 font-bold truncate max-w-md" :title="autoMatchedJava.path">
            {{ autoMatchedJava.vendor }} Java {{ autoMatchedJava.major }} ({{ autoMatchedJava.versionStr }})
          </span>
        </div>
        <div v-else class="text-[11px] text-zinc-400 pt-1">
          Aucun runtime Java {{ requiredMajor }} détecté localement. Loom téléchargera et provisionnera automatiquement la dernière version d'Adoptium Temurin Java {{ requiredMajor }}.
        </div>
      </div>

      <!-- Mode 2: Detected system JVMs dropdown -->
      <div v-else-if="javaSelectionMode === 'detected'" class="space-y-3">
        <div class="flex items-center justify-between">
          <label class="block text-xs font-mono font-bold text-zinc-300">
            Sélectionnez la machine virtuelle Java :
          </label>
          <button
            type="button"
            @click="minecraftStore.fetchDetectedJavaRuntimes()"
            class="text-[11px] font-mono text-zinc-400 hover:text-amber-400 flex items-center gap-1 transition-colors"
          >
            <RefreshCw :class="['w-3 h-3', isDetectingJava ? 'animate-spin' : '']" />
            <span>Actualiser</span>
          </button>
        </div>

        <div class="relative">
          <select
            v-model="customJavaPath"
            class="w-full bg-zinc-950 border border-zinc-800 rounded-2xl px-4 py-3 text-xs font-mono text-zinc-100 focus:outline-none focus:border-amber-500/80 transition-colors cursor-pointer"
          >
            <option
              v-for="j in detectedRuntimes"
              :key="j.path"
              :value="j.path"
            >
              Java {{ j.major }} ({{ j.versionStr }}) — {{ j.vendor }} [{{ j.path }}] {{ j.major === requiredMajor ? '★ Recommandé' : '' }}
            </option>
          </select>
        </div>

        <div v-if="selectedDetectedRuntime && selectedDetectedRuntime.major < requiredMajor" class="p-3 bg-amber-500/10 border border-amber-500/30 rounded-2xl flex items-center gap-2 text-xs text-amber-300">
          <AlertTriangle class="w-4 h-4 text-amber-400 shrink-0" />
          <span>Attention : Minecraft {{ activeInst.gameVersion }} requiert Java {{ requiredMajor }}, or ce runtime est en Java {{ selectedDetectedRuntime.major }}. Le lancement risque d'échouer.</span>
        </div>
      </div>

      <!-- Mode 3: Custom path input with live inspection -->
      <div v-else-if="javaSelectionMode === 'custom'" class="space-y-2">
        <label class="block text-xs font-mono font-bold text-zinc-300">
          Chemin absolu vers l'exécutable java :
        </label>
        <div class="flex items-center gap-2">
          <input
            v-model="customJavaPath"
            type="text"
            placeholder="/usr/bin/java ou /home/.../bin/java"
            class="flex-1 bg-zinc-950 border border-zinc-800 rounded-2xl px-4 py-2.5 text-xs font-mono text-zinc-100 focus:outline-none focus:border-amber-500/80 shadow-inner"
            @input="onCustomPathInput"
          />
          <button
            type="button"
            @click="checkCustomPath"
            class="px-3.5 py-2.5 rounded-2xl bg-zinc-800 hover:bg-zinc-700 text-xs font-mono text-zinc-300 transition"
          >
            Tester
          </button>
        </div>

        <!-- Custom Path Inspection Result -->
        <div v-if="customPathInfo">
          <div v-if="customPathInfo.valid" class="p-3 bg-emerald-500/10 border border-emerald-500/30 rounded-2xl flex items-center gap-2 text-xs text-emerald-300 font-mono">
            <Check class="w-4 h-4 text-emerald-400 shrink-0" />
            <span>Exécutable valide : {{ customPathInfo.runtime?.vendor }} Java {{ customPathInfo.runtime?.major }} ({{ customPathInfo.runtime?.versionStr }})</span>
          </div>
          <div v-else class="p-3 bg-rose-500/10 border border-rose-500/30 rounded-2xl flex items-center gap-2 text-xs text-rose-300 font-mono">
            <AlertCircle class="w-4 h-4 text-rose-400 shrink-0" />
            <span>Fichier exécutable Java introuvable ou inaccessible à ce chemin.</span>
          </div>
        </div>
      </div>
    </div>

    <!-- 4. Custom JVM Arguments -->
    <div class="p-6 rounded-3xl bg-zinc-900/60 border border-zinc-800/80 space-y-4">
      <div class="flex items-center justify-between">
        <div class="space-y-1">
          <h3 class="text-sm font-bold text-zinc-100 flex items-center gap-2">
            <Sliders class="w-4 h-4 text-amber-400" />
            Arguments JVM & Garbage Collector (G1GC)
          </h3>
          <p class="text-xs text-zinc-400">
            Paramètres de performance injectés directement dans la commande d'exécution du runtime Adoptium.
          </p>
        </div>

        <button
          type="button"
          @click="resetDefaultJvmArgs"
          class="text-xs font-mono text-zinc-400 hover:text-amber-400 transition-colors flex items-center gap-1.5"
        >
          <RotateCcw class="w-3.5 h-3.5" />
          <span>Réinitialiser par défaut</span>
        </button>
      </div>

      <textarea
        v-model="jvmArgsText"
        rows="3"
        class="w-full bg-zinc-950 border border-zinc-800 rounded-2xl p-4 text-xs font-mono text-zinc-200 focus:outline-none focus:border-amber-500/80 leading-relaxed"
      ></textarea>
    </div>

    <!-- 5. Isolation & Paths -->
    <div class="p-6 rounded-3xl bg-zinc-900/60 border border-zinc-800/80 space-y-4">
      <h3 class="text-sm font-bold text-zinc-100 flex items-center gap-2">
        <ShieldCheck class="w-4 h-4 text-emerald-400" />
        Isolation de l'Instance
      </h3>

      <div class="space-y-2 text-xs font-mono">
        <div class="flex items-center justify-between p-3 rounded-xl bg-zinc-950 border border-zinc-800">
          <span class="text-zinc-500">Dossier de jeu (.minecraft) :</span>
          <span class="text-zinc-300">~/.local/share/loom/instances/{{ activeInst.id }}/.minecraft</span>
        </div>
        <div class="flex items-center justify-between p-3 rounded-xl bg-zinc-950 border border-zinc-800">
          <span class="text-zinc-500">Cache partagé des mods :</span>
          <span class="text-zinc-300">~/.local/share/loom/cache/mods</span>
        </div>
      </div>
    </div>

    <!-- Save Settings Button -->
    <div class="flex justify-end pt-1">
      <UiButton
        variant="primary"
        size="md"
        @click="saveSettings"
      >
        <template #prefix>
          <Check v-if="isSaved" class="w-4 h-4 text-emerald-400" />
          <Save v-else class="w-4 h-4" />
        </template>
        {{ isSaved ? 'Paramètres enregistrés !' : 'Enregistrer les modifications' }}
      </UiButton>
    </div>

    <!-- 6. Zone de Danger -->
    <div class="p-6 rounded-3xl bg-rose-950/20 border border-rose-900/40 space-y-5">
      <div class="space-y-1">
        <h3 class="text-sm font-bold text-rose-300 flex items-center gap-2">
          <AlertTriangle class="w-4 h-4 text-rose-400" />
          Gestion & Zone de Danger
        </h3>
        <p class="text-xs text-zinc-400">
          Actions d'urgence pour débloquer l'instance ou supprimer définitivement ses fichiers du disque.
        </p>
      </div>

      <div class="flex flex-wrap items-center justify-between gap-4 pt-2">
        <!-- Unlock / Reset button -->
        <button
          type="button"
          @click="handleResetStatus"
          class="px-4 py-2 rounded-xl text-xs font-mono bg-zinc-900 hover:bg-zinc-800 text-zinc-300 hover:text-zinc-100 border border-zinc-700/80 transition-colors flex items-center gap-2"
        >
          <PowerOff class="w-3.5 h-3.5 text-amber-400" />
          <span>Réinitialiser le statut (débloquer)</span>
        </button>

        <!-- Delete button with inline confirmation -->
        <div class="flex items-center gap-2">
          <template v-if="!confirmDelete">
            <button
              type="button"
              @click="confirmDelete = true"
              class="px-4 py-2 rounded-xl text-xs font-mono bg-rose-500/10 hover:bg-rose-500/20 text-rose-300 hover:text-rose-200 border border-rose-500/30 transition-colors flex items-center gap-2"
            >
              <Trash2 class="w-3.5 h-3.5" />
              <span>Supprimer cette instance</span>
            </button>
          </template>

          <template v-else>
            <span class="text-xs font-mono text-rose-300">Êtes-vous sûr ?</span>
            <button
              type="button"
              @click="handleDeleteInstance"
              class="px-3.5 py-1.5 rounded-xl text-xs font-mono font-bold bg-rose-600 hover:bg-rose-500 text-white shadow-lg shadow-rose-600/30 transition-colors"
            >
              Oui, supprimer définitivement
            </button>
            <button
              type="button"
              @click="confirmDelete = false"
              class="px-3.5 py-1.5 rounded-xl text-xs font-mono bg-zinc-800 hover:bg-zinc-700 text-zinc-300 transition-colors"
            >
              Annuler
            </button>
          </template>
        </div>
      </div>
    </div>
  </div>
</template>
