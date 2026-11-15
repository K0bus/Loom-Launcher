<script setup lang="ts">
import { ref, computed, watch, onMounted } from 'vue'
import {
  Terminal,
  FileText,
  Archive,
  AlertOctagon,
  Copy,
  Check,
  Trash2,
  RefreshCw,
  Search,
  ArrowDownCircle,
  Loader2,
  Pause,
  Play
} from 'lucide-vue-next'
import { useMinecraftStore } from '@/stores/minecraft'
import UiBadge from '@/components/ui/UiBadge.vue'
import UiButton from '@/components/ui/UiButton.vue'

const minecraftStore = useMinecraftStore()
const activeInst = computed(() => minecraftStore.activeInstance)

// Sub-tabs: 'live' | 'latest' | 'history' | 'crashes'
const activeSubTab = ref<'live' | 'latest' | 'history' | 'crashes'>('live')

// 1. Live JVM console state
const terminalRef = ref<HTMLDivElement | null>(null)
const autoScroll = ref(true)
const isPaused = ref(false)
const liveFilterText = ref('')
const liveLevel = ref<'ALL' | 'ERROR' | 'WARN' | 'INFO'>('ALL')
const copied = ref(false)

// 2. Disk logs state
const isLoadingFiles = ref(false)
const diskLogs = ref<any[]>([])
const crashReports = ref<any[]>([])
const selectedLogFile = ref<string | null>(null)
const selectedCrashFile = ref<string | null>(null)
const logContent = ref<string>('')
const isLoadingContent = ref(false)
const diskFilterText = ref('')
const showAllDiskLines = ref(false)

async function loadDiskLogFiles() {
  if (!activeInst.value) return
  isLoadingFiles.value = true
  try {
    const data = await minecraftStore.getInstanceLogFiles(activeInst.value.id)
    diskLogs.value = data.logs || []
    crashReports.value = data.crashReports || []

    if (activeSubTab.value === 'latest') {
      selectLogFile('latest.log', false)
    } else if (activeSubTab.value === 'crashes' && crashReports.value.length > 0 && !selectedCrashFile.value) {
      selectCrashFile(crashReports.value[0].name)
    }
  } catch (err) {
    console.warn('[LogsTab] Error loading log files:', err)
  } finally {
    isLoadingFiles.value = false
  }
}

async function selectLogFile(fileName: string, isCrash = false) {
  if (!activeInst.value) return
  selectedLogFile.value = fileName
  showAllDiskLines.value = false
  isLoadingContent.value = true
  logContent.value = ''
  try {
    const res = await minecraftStore.readInstanceLogContent(activeInst.value.id, fileName, isCrash)
    logContent.value = res.content || 'Fichier vide.'
  } catch (err: any) {
    logContent.value = `Erreur lors de la lecture du fichier: ${err?.message || err}`
  } finally {
    isLoadingContent.value = false
  }
}

async function selectCrashFile(fileName: string) {
  selectedCrashFile.value = fileName
  await selectLogFile(fileName, true)
}

function switchSubTab(tab: 'live' | 'latest' | 'history' | 'crashes') {
  activeSubTab.value = tab
  diskFilterText.value = ''
  showAllDiskLines.value = false
  if (tab === 'latest') {
    selectLogFile('latest.log', false)
  } else if (tab === 'history') {
    if (diskLogs.value.length > 0 && !selectedLogFile.value) {
      selectLogFile(diskLogs.value[0].name, false)
    }
  } else if (tab === 'crashes') {
    if (crashReports.value.length > 0 && !selectedCrashFile.value) {
      selectCrashFile(crashReports.value[0].name)
    }
  }
}

// Live filtered logs - Capped at last 250 items to keep DOM rendering at 60fps
const filteredLiveLogs = computed(() => {
  if (!activeInst.value?.logs) return []
  const logs = activeInst.value.logs
  const q = liveFilterText.value.toLowerCase().trim()
  const lvl = liveLevel.value

  if (lvl === 'ALL' && !q) {
    return logs.length > 250 ? logs.slice(logs.length - 250) : logs
  }

  const matches: any[] = []
  for (let i = logs.length - 1; i >= 0; i--) {
    const log = logs[i]
    if (lvl !== 'ALL' && log.level !== lvl) continue
    if (q && !log.message.toLowerCase().includes(q) && !log.source.toLowerCase().includes(q)) continue
    matches.unshift(log)
    if (matches.length >= 250) break
  }
  return matches
})

// Scroll handling with requestAnimationFrame throttling
let scrollRaf: number | null = null
function scheduleScrollToBottom() {
  if (!autoScroll.value || isPaused.value || !terminalRef.value) return
  if (scrollRaf) return
  scrollRaf = requestAnimationFrame(() => {
    scrollRaf = null
    if (terminalRef.value && autoScroll.value && !isPaused.value) {
      terminalRef.value.scrollTop = terminalRef.value.scrollHeight
    }
  })
}

function handleScroll() {
  if (!terminalRef.value) return
  const { scrollTop, scrollHeight, clientHeight } = terminalRef.value
  const isAtBottom = scrollHeight - scrollTop - clientHeight < 40
  autoScroll.value = isAtBottom
}

watch(
  () => activeInst.value?.logs?.length,
  () => {
    if (activeSubTab.value === 'live') {
      scheduleScrollToBottom()
    }
  }
)

function copyText(str: string) {
  navigator.clipboard.writeText(str)
  copied.value = true
  setTimeout(() => (copied.value = false), 2000)
}

function copyLiveLogs() {
  if (!activeInst.value) return
  const full = activeInst.value.logs
    .map((l) => `[${l.timestamp}] [${l.source}/${l.level}]: ${l.message}`)
    .join('\n')
  copyText(full)
}

function clearLiveLogs() {
  if (activeInst.value) {
    minecraftStore.clearLogs(activeInst.value.id)
  }
}

// Format bytes
function formatBytes(bytes: number): string {
  if (!bytes || bytes <= 0) return '0 Ko'
  const kb = bytes / 1024
  if (kb < 1000) return `${kb.toFixed(0)} Ko`
  return `${(kb / 1024).toFixed(1)} Mo`
}

function formatDate(iso: string): string {
  if (!iso) return ''
  try {
    const d = new Date(iso)
    return d.toLocaleDateString('fr-FR', {
      day: 'numeric',
      month: 'short',
      hour: '2-digit',
      minute: '2-digit'
    })
  } catch {
    return iso
  }
}

// Disk lines handling with pagination window
const allDiskLines = computed(() => (logContent.value ? logContent.value.split('\n') : []))
const totalDiskLinesCount = computed(() => allDiskLines.value.length)

const displayedDiskLines = computed(() => {
  const all = allDiskLines.value
  if (all.length === 0) return []
  const q = diskFilterText.value.toLowerCase().trim()

  let filtered = all
  if (q) {
    filtered = all.filter((l) => l.toLowerCase().includes(q))
  }

  if (!showAllDiskLines.value && filtered.length > 500) {
    return filtered.slice(filtered.length - 500)
  }
  return filtered
})

// Crash Report Root Cause Extractor
const crashAnalysis = computed(() => {
  if (!logContent.value || activeSubTab.value !== 'crashes') return null
  const lines = logContent.value.split('\n')
  let description = ''
  let suspectedMod = ''

  for (const line of lines) {
    if (line.startsWith('Description:')) {
      description = line.replace('Description:', '').trim()
    }
    if (!suspectedMod && (line.includes('Suspected Mod:') || line.includes('Suspected Mods:'))) {
      suspectedMod = line.split(':')[1]?.trim() || ''
    }
  }

  if (!suspectedMod) {
    for (const line of lines.slice(0, 50)) {
      const match = line.match(/at\s+([a-zA-Z0-9_]+)\.([a-zA-Z0-9_]+)\./)
      if (match) {
        const pkg = match[1]
        if (!['java', 'jdk', 'sun', 'net', 'com_mojang'].includes(pkg)) {
          suspectedMod = pkg
          break
        }
      }
    }
  }

  return {
    description: description || 'Erreur d\'exécution Java inattendue',
    suspectedMod
  }
})

// Highlight log line
function getLogLineClass(line: string): string {
  if (line.includes('ERROR') || line.includes('Exception') || line.includes('FATAL') || line.includes('Crash')) {
    return 'text-rose-400 font-semibold'
  }
  if (line.includes('WARN')) {
    return 'text-amber-400'
  }
  if (line.includes('DEBUG')) {
    return 'text-zinc-500'
  }
  return 'text-zinc-300'
}

onMounted(() => {
  loadDiskLogFiles()
  if (activeSubTab.value === 'live') {
    scheduleScrollToBottom()
  }
})
</script>

<template>
  <div class="space-y-6">
    <!-- SUB-TABS NAVIGATION BAR -->
    <div class="flex flex-wrap items-center justify-between gap-4 p-2 bg-zinc-950/80 rounded-3xl border border-zinc-800/80 backdrop-blur-md">
      <div class="flex flex-wrap items-center gap-2">
        <!-- 1. Live JVM Console -->
        <button
          type="button"
          @click="switchSubTab('live')"
          :class="[
            'px-4 py-2 rounded-2xl text-xs font-mono font-bold transition-all flex items-center gap-2 select-none shadow-sm',
            activeSubTab === 'live'
              ? 'bg-amber-500 text-zinc-950 shadow-amber-500/10'
              : 'text-zinc-400 hover:text-zinc-200 hover:bg-zinc-900/80'
          ]"
        >
          <Terminal class="w-4 h-4" />
          <span>Console JVM</span>
          <span
            v-if="activeInst?.status === 'running'"
            class="w-2 h-2 rounded-full bg-emerald-400 animate-pulse ml-0.5"
            title="Session active"
          />
        </button>

        <!-- 2. latest.log -->
        <button
          type="button"
          @click="switchSubTab('latest')"
          :class="[
            'px-4 py-2 rounded-2xl text-xs font-mono font-bold transition-all flex items-center gap-2 select-none shadow-sm',
            activeSubTab === 'latest'
              ? 'bg-amber-500 text-zinc-950 shadow-amber-500/10'
              : 'text-zinc-400 hover:text-zinc-200 hover:bg-zinc-900/80'
          ]"
        >
          <FileText class="w-4 h-4" />
          <span>latest.log</span>
        </button>

        <!-- 3. Logs History -->
        <button
          type="button"
          @click="switchSubTab('history')"
          :class="[
            'px-4 py-2 rounded-2xl text-xs font-mono font-bold transition-all flex items-center gap-2 select-none shadow-sm',
            activeSubTab === 'history'
              ? 'bg-amber-500 text-zinc-950 shadow-amber-500/10'
              : 'text-zinc-400 hover:text-zinc-200 hover:bg-zinc-900/80'
          ]"
        >
          <Archive class="w-4 h-4" />
          <span>Archives de logs</span>
          <span class="text-[10px] px-1.5 py-0.2 rounded-full bg-zinc-800 text-zinc-400 font-bold">
            {{ diskLogs.length }}
          </span>
        </button>

        <!-- 4. Crash Reports -->
        <button
          type="button"
          @click="switchSubTab('crashes')"
          :class="[
            'px-4 py-2 rounded-2xl text-xs font-mono font-bold transition-all flex items-center gap-2 select-none shadow-sm',
            activeSubTab === 'crashes'
              ? 'bg-rose-500 text-white shadow-rose-500/20'
              : 'text-zinc-400 hover:text-zinc-200 hover:bg-zinc-900/80'
          ]"
        >
          <AlertOctagon class="w-4 h-4" />
          <span>Crash Reports</span>
          <span
            v-if="crashReports.length > 0"
            class="text-[10px] px-1.5 py-0.2 rounded-full bg-rose-500/20 text-rose-300 font-bold ml-1"
          >
            {{ crashReports.length }}
          </span>
        </button>
      </div>

      <!-- Action buttons -->
      <div class="flex items-center gap-2">
        <UiButton
          variant="ghost"
          size="sm"
          @click="loadDiskLogFiles"
          :disabled="isLoadingFiles"
          title="Actualiser les fichiers"
        >
          <RefreshCw :class="['w-3.5 h-3.5', isLoadingFiles ? 'animate-spin text-amber-400' : '']" />
        </UiButton>
      </div>
    </div>

    <!-- ============================================================== -->
    <!-- VIEW 1: LIVE JVM CONSOLE (HIGH PERFORMANCE)                    -->
    <!-- ============================================================== -->
    <div v-if="activeSubTab === 'live'" class="space-y-4">
      <!-- Toolbar -->
      <div class="flex flex-col sm:flex-row items-stretch sm:items-center justify-between gap-3 bg-zinc-900/60 p-4 rounded-3xl border border-zinc-800/80">
        <div class="flex flex-wrap items-center gap-2.5">
          <div class="relative flex-1 sm:w-56">
            <input
              v-model="liveFilterText"
              type="text"
              placeholder="Filtrer la sortie console..."
              class="w-full bg-zinc-950 border border-zinc-800 rounded-xl pl-9 pr-3 py-1.5 text-xs text-zinc-200 placeholder-zinc-500 focus:outline-none focus:border-amber-500"
            />
            <Search class="w-3.5 h-3.5 text-zinc-500 absolute left-3 top-1/2 -translate-y-1/2 pointer-events-none" />
          </div>

          <!-- Severity Filter Buttons -->
          <div class="flex items-center gap-1 bg-zinc-950 p-1 rounded-xl border border-zinc-800 text-[11px] font-mono">
            <button
              v-for="lvl in (['ALL', 'INFO', 'WARN', 'ERROR'] as const)"
              :key="lvl"
              type="button"
              @click="liveLevel = lvl"
              :class="[
                'px-2 py-1 rounded-lg font-bold transition',
                liveLevel === lvl ? 'bg-amber-500 text-zinc-950' : 'text-zinc-400 hover:text-zinc-200'
              ]"
            >
              {{ lvl }}
            </button>
          </div>
        </div>

        <div class="flex flex-wrap items-center gap-2">
          <!-- Pause/Resume button to freeze stream -->
          <button
            type="button"
            @click="isPaused = !isPaused"
            :class="[
              'px-3 py-1.5 rounded-xl border text-xs font-mono font-bold transition flex items-center gap-1.5 select-none',
              isPaused
                ? 'bg-rose-500/20 border-rose-500/50 text-rose-300'
                : 'bg-zinc-950 border-zinc-800 text-zinc-400 hover:text-zinc-200'
            ]"
            :title="isPaused ? 'Reprendre le flux en direct' : 'Mettre en pause l\'affichage'"
          >
            <Play v-if="isPaused" class="w-3.5 h-3.5 fill-current" />
            <Pause v-else class="w-3.5 h-3.5" />
            <span>{{ isPaused ? 'Reprendre' : 'Pause' }}</span>
          </button>

          <!-- Auto-scroll toggle -->
          <button
            type="button"
            @click="autoScroll = !autoScroll"
            :class="[
              'px-3 py-1.5 rounded-xl border text-xs font-mono font-bold transition flex items-center gap-1.5 select-none',
              autoScroll && !isPaused
                ? 'bg-amber-500/10 border-amber-500/40 text-amber-400'
                : 'bg-zinc-950 border-zinc-800 text-zinc-500'
            ]"
          >
            <ArrowDownCircle class="w-3.5 h-3.5" />
            <span>Défilement auto</span>
          </button>

          <!-- Copy -->
          <UiButton variant="secondary" size="sm" @click="copyLiveLogs">
            <Check v-if="copied" class="w-3.5 h-3.5 text-emerald-400" />
            <Copy v-else class="w-3.5 h-3.5" />
            <span>{{ copied ? 'Copié' : 'Copier' }}</span>
          </UiButton>

          <!-- Clear -->
          <UiButton variant="ghost" size="sm" @click="clearLiveLogs" title="Vider l'affichage de la console">
            <Trash2 class="w-3.5 h-3.5 text-zinc-500 hover:text-rose-400" />
          </UiButton>
        </div>
      </div>

      <!-- Console Window (Hardware Accelerated & Virtualized) -->
      <div
        ref="terminalRef"
        @scroll.passive="handleScroll"
        class="h-[520px] rounded-3xl bg-zinc-950 border border-zinc-800/80 p-5 font-mono text-xs overflow-y-auto space-y-1 shadow-2xl selection:bg-amber-500/30 selection:text-amber-200 will-change-scroll"
      >
        <div v-if="filteredLiveLogs.length === 0" class="h-full flex items-center justify-center text-zinc-600 text-xs">
          {{ activeInst?.status === 'running' ? 'En attente de messages du jeu...' : 'Instance arrêtée. Lancez le jeu pour voir les logs en direct.' }}
        </div>

        <div
          v-for="(log, idx) in filteredLiveLogs"
          :key="idx"
          class="flex items-start gap-3 hover:bg-white/[0.02] py-0.5 px-1.5 rounded leading-relaxed"
        >
          <span class="text-zinc-600 shrink-0 text-[11px] select-none">{{ log.timestamp }}</span>
          <span
            :class="[
              'px-1.5 py-0.2 rounded text-[10px] font-bold uppercase shrink-0',
              log.level === 'ERROR' ? 'bg-rose-500/20 text-rose-400' :
              log.level === 'WARN' ? 'bg-amber-500/20 text-amber-400' :
              'bg-zinc-800 text-sky-400'
            ]"
          >
            {{ log.level }}
          </span>
          <span class="text-zinc-400 text-[11px] shrink-0 font-semibold select-none">[{{ log.source }}]</span>
          <span :class="['flex-1 break-all whitespace-pre-wrap', log.level === 'ERROR' ? 'text-rose-300' : log.level === 'WARN' ? 'text-amber-200' : 'text-zinc-300']">
            {{ log.message }}
          </span>
        </div>
      </div>
    </div>

    <!-- ============================================================== -->
    <!-- VIEW 2: LATEST.LOG                                             -->
    <!-- ============================================================== -->
    <div v-else-if="activeSubTab === 'latest'" class="space-y-4">
      <!-- Toolbar -->
      <div class="flex flex-col sm:flex-row items-stretch sm:items-center justify-between gap-3 bg-zinc-900/60 p-4 rounded-3xl border border-zinc-800/80">
        <div class="flex items-center gap-3">
          <FileText class="w-5 h-5 text-amber-400" />
          <div>
            <div class="font-bold text-sm text-zinc-200 font-mono">.minecraft/logs/latest.log</div>
            <div class="text-[11px] text-zinc-500 font-mono">{{ totalDiskLinesCount }} lignes au total</div>
          </div>
        </div>

        <div class="flex items-center gap-2">
          <div class="relative w-48 sm:w-64">
            <input
              v-model="diskFilterText"
              type="text"
              placeholder="Filtrer les lignes..."
              class="w-full bg-zinc-950 border border-zinc-800 rounded-xl pl-8 pr-3 py-1.5 text-xs text-zinc-200 placeholder-zinc-500 focus:outline-none focus:border-amber-500"
            />
            <Search class="w-3.5 h-3.5 text-zinc-500 absolute left-2.5 top-1/2 -translate-y-1/2 pointer-events-none" />
          </div>

          <UiButton variant="secondary" size="sm" @click="copyText(logContent)">
            <Check v-if="copied" class="w-3.5 h-3.5 text-emerald-400" />
            <Copy v-else class="w-3.5 h-3.5" />
            <span>{{ copied ? 'Copié' : 'Copier' }}</span>
          </UiButton>

          <UiButton variant="ghost" size="sm" @click="selectLogFile('latest.log', false)">
            <RefreshCw class="w-3.5 h-3.5" />
          </UiButton>
        </div>
      </div>

      <!-- Windowed lines notification -->
      <div
        v-if="!showAllDiskLines && totalDiskLinesCount > 500"
        class="p-2.5 px-4 text-xs font-mono text-amber-300 bg-amber-500/10 border border-amber-500/20 rounded-2xl flex items-center justify-between"
      >
        <span>Affichage optimisé des 500 dernières lignes (sur {{ totalDiskLinesCount }}).</span>
        <button
          type="button"
          @click="showAllDiskLines = true"
          class="underline font-bold hover:text-white transition"
        >
          Afficher tout
        </button>
      </div>

      <!-- File content viewer -->
      <div
        v-if="isLoadingContent"
        class="h-[520px] rounded-3xl bg-zinc-950 border border-zinc-800 p-8 flex items-center justify-center"
      >
        <Loader2 class="w-8 h-8 text-amber-500 animate-spin" />
      </div>

      <div
        v-else
        class="h-[520px] rounded-3xl bg-zinc-950 border border-zinc-800/80 p-5 font-mono text-xs overflow-y-auto space-y-0.5 shadow-2xl selection:bg-amber-500/30 selection:text-amber-200"
      >
        <div
          v-for="(line, idx) in displayedDiskLines"
          :key="idx"
          :class="['leading-relaxed hover:bg-white/[0.02] px-1 rounded whitespace-pre-wrap break-all', getLogLineClass(line)]"
        >
          {{ line }}
        </div>
      </div>
    </div>

    <!-- ============================================================== -->
    <!-- VIEW 3: LOGS HISTORY                                           -->
    <!-- ============================================================== -->
    <div v-else-if="activeSubTab === 'history'" class="grid grid-cols-1 md:grid-cols-3 gap-6">
      <!-- File Selector Sidebar -->
      <div class="md:col-span-1 space-y-2.5 max-h-[580px] overflow-y-auto pr-1">
        <div class="text-xs font-mono font-bold text-zinc-400 uppercase tracking-wider px-1">
          Fichiers d'archive ({{ diskLogs.length }})
        </div>

        <button
          v-for="file in diskLogs"
          :key="file.name"
          type="button"
          @click="selectLogFile(file.name, false)"
          :class="[
            'w-full text-left p-3.5 rounded-2xl border transition-all select-none space-y-1 block',
            selectedLogFile === file.name
              ? 'bg-amber-500/10 border-amber-500/40 text-amber-300'
              : 'bg-zinc-900/60 border-zinc-800/80 text-zinc-300 hover:border-zinc-700'
          ]"
        >
          <div class="flex items-center justify-between font-mono text-xs font-bold truncate">
            <span class="truncate">{{ file.name }}</span>
            <UiBadge v-if="file.isCompressed" variant="zinc" size="xs">.GZ</UiBadge>
          </div>
          <div class="flex items-center justify-between text-[11px] font-mono text-zinc-500">
            <span>{{ formatBytes(file.sizeBytes) }}</span>
            <span>{{ formatDate(file.mtime) }}</span>
          </div>
        </button>
      </div>

      <!-- File Content Viewer -->
      <div class="md:col-span-2 space-y-3">
        <div class="flex items-center justify-between bg-zinc-900/60 p-3 rounded-2xl border border-zinc-800">
          <span class="text-xs font-mono font-bold text-zinc-200 truncate">
            {{ selectedLogFile || 'Sélectionnez un log' }}
          </span>
          <UiButton
            v-if="logContent"
            variant="secondary"
            size="sm"
            @click="copyText(logContent)"
          >
            <Check v-if="copied" class="w-3.5 h-3.5 text-emerald-400" />
            <Copy v-else class="w-3.5 h-3.5" />
            <span>{{ copied ? 'Copié' : 'Copier' }}</span>
          </UiButton>
        </div>

        <div
          class="h-[520px] rounded-3xl bg-zinc-950 border border-zinc-800/80 p-5 font-mono text-xs overflow-y-auto space-y-0.5 shadow-2xl selection:bg-amber-500/30"
        >
          <div v-if="isLoadingContent" class="h-full flex items-center justify-center">
            <Loader2 class="w-7 h-7 text-amber-500 animate-spin" />
          </div>
          <div
            v-else
            v-for="(line, idx) in displayedDiskLines"
            :key="idx"
            :class="['leading-relaxed hover:bg-white/[0.02] px-1 rounded whitespace-pre-wrap break-all', getLogLineClass(line)]"
          >
            {{ line }}
          </div>
        </div>
      </div>
    </div>

    <!-- ============================================================== -->
    <!-- VIEW 4: CRASH REPORTS                                          -->
    <!-- ============================================================== -->
    <div v-else-if="activeSubTab === 'crashes'" class="space-y-4">
      <!-- Empty State if no crashes -->
      <div
        v-if="crashReports.length === 0"
        class="p-16 text-center rounded-3xl bg-zinc-900/30 border border-zinc-800/80 space-y-3"
      >
        <div class="w-14 h-14 mx-auto rounded-2xl bg-emerald-500/10 border border-emerald-500/30 flex items-center justify-center text-emerald-400">
          <Check class="w-7 h-7" />
        </div>
        <h4 class="text-sm font-bold text-zinc-200">Aucun rapport de crash détecté</h4>
        <p class="text-xs text-zinc-500 max-w-sm mx-auto">
          Votre instance n'a généré aucun rapport de crash critique dans <span class="font-mono text-zinc-400">.minecraft/crash-reports/</span>.
        </p>
      </div>

      <div v-else class="grid grid-cols-1 md:grid-cols-3 gap-6">
        <!-- Crash reports list sidebar -->
        <div class="md:col-span-1 space-y-2.5 max-h-[580px] overflow-y-auto pr-1">
          <div class="text-xs font-mono font-bold text-rose-400 uppercase tracking-wider px-1">
            Rapports de crash ({{ crashReports.length }})
          </div>

          <button
            v-for="file in crashReports"
            :key="file.name"
            type="button"
            @click="selectCrashFile(file.name)"
            :class="[
              'w-full text-left p-3.5 rounded-2xl border transition-all select-none space-y-1 block',
              selectedCrashFile === file.name
                ? 'bg-rose-500/10 border-rose-500/40 text-rose-300'
                : 'bg-zinc-900/60 border-zinc-800/80 text-zinc-300 hover:border-zinc-700'
            ]"
          >
            <div class="font-mono text-xs font-bold truncate">
              {{ file.name }}
            </div>
            <div class="flex items-center justify-between text-[11px] font-mono text-zinc-500">
              <span>{{ formatBytes(file.sizeBytes) }}</span>
              <span>{{ formatDate(file.mtime) }}</span>
            </div>
          </button>
        </div>

        <!-- Crash Report Analysis & Viewer -->
        <div class="md:col-span-2 space-y-4">
          <!-- Summary Diagnostic Box -->
          <div
            v-if="crashAnalysis"
            class="p-4 rounded-2xl bg-rose-500/10 border border-rose-500/30 space-y-2"
          >
            <div class="flex items-center justify-between">
              <span class="text-xs font-mono font-bold text-rose-400 uppercase flex items-center gap-1.5">
                <AlertOctagon class="w-4 h-4" />
                Diagnostic automatique
              </span>
              <span v-if="crashAnalysis.suspectedMod" class="text-xs font-mono px-2 py-0.5 rounded-md bg-rose-500/20 text-rose-300 font-bold">
                Mod suspect: {{ crashAnalysis.suspectedMod }}
              </span>
            </div>
            <p class="text-xs font-mono text-zinc-300 leading-relaxed font-semibold">
              {{ crashAnalysis.description }}
            </p>
          </div>

          <!-- Stack Trace Viewer -->
          <div class="space-y-2">
            <div class="flex items-center justify-between bg-zinc-900/60 p-3 rounded-2xl border border-zinc-800">
              <span class="text-xs font-mono text-zinc-400 truncate">
                {{ selectedCrashFile }}
              </span>
              <UiButton variant="primary" size="sm" @click="copyText(logContent)">
                <Check v-if="copied" class="w-3.5 h-3.5" />
                <Copy v-else class="w-3.5 h-3.5" />
                <span>{{ copied ? 'Rapport copié !' : 'Copier pour support' }}</span>
              </UiButton>
            </div>

            <div
              class="h-[460px] rounded-3xl bg-zinc-950 border border-zinc-800/80 p-5 font-mono text-xs overflow-y-auto space-y-0.5 shadow-2xl selection:bg-rose-500/30"
            >
              <div
                v-for="(line, idx) in displayedDiskLines"
                :key="idx"
                :class="['leading-relaxed hover:bg-white/[0.02] px-1 rounded whitespace-pre-wrap break-all', getLogLineClass(line)]"
              >
                {{ line }}
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
