<script setup lang="ts">
import { ref, computed, watch, nextTick, onMounted } from 'vue'
import {
  Terminal,
  Trash2,
  Copy,
  Check,
  ArrowDownCircle,
  Search
} from 'lucide-vue-next'
import type { MinecraftInstance } from '@/models/minecraft'
import UiButton from '@/components/ui/UiButton.vue'

interface Props {
  instance: MinecraftInstance
}

const props = defineProps<Props>()

const terminalRef = ref<HTMLDivElement | null>(null)
const autoScroll = ref(true)
const copied = ref(false)
const filterText = ref('')
const selectedLevel = ref<'ALL' | 'ERROR' | 'WARN' | 'INFO'>('ALL')

function scrollToBottom() {
  if (autoScroll.value && terminalRef.value) {
    terminalRef.value.scrollTop = terminalRef.value.scrollHeight
  }
}

watch(
  () => props.instance.logs.length,
  async () => {
    await nextTick()
    scrollToBottom()
  }
)

onMounted(() => {
  scrollToBottom()
})

const filteredLogs = computed(() => {
  return props.instance.logs.filter((log) => {
    if (selectedLevel.value !== 'ALL' && log.level !== selectedLevel.value) {
      return false
    }
    if (filterText.value.trim()) {
      const q = filterText.value.toLowerCase()
      return (
        log.message.toLowerCase().includes(q) ||
        log.source.toLowerCase().includes(q) ||
        log.timestamp.includes(q)
      )
    }
    return true
  })
})

function copyLogs() {
  const fullText = props.instance.logs
    .map((l) => `[${l.timestamp}] [${l.source}/${l.level}]: ${l.message}`)
    .join('\n')
  navigator.clipboard.writeText(fullText)
  copied.value = true
  setTimeout(() => (copied.value = false), 2000)
}

function clearLogs() {
  props.instance.logs.length = 0
}

const levelColors: Record<string, string> = {
  INFO: 'text-sky-400',
  WARN: 'text-amber-400 font-semibold',
  ERROR: 'text-rose-400 font-bold',
  DEBUG: 'text-zinc-500'
}
</script>

<template>
  <div class="space-y-3">
    <!-- Header / Action bar -->
    <div class="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-3 bg-zinc-900/60 p-3 rounded-2xl border border-zinc-800/80">
      <div class="flex items-center gap-3">
        <div class="w-8 h-8 rounded-lg bg-zinc-950 border border-zinc-800 flex items-center justify-center text-amber-400">
          <Terminal class="w-4 h-4" />
        </div>
        <div>
          <div class="text-xs font-bold text-zinc-200 font-mono">Console d'exécution JVM Minecraft</div>
          <div class="text-[11px] text-zinc-500 font-mono flex items-center gap-2">
            <span>{{ instance.logs.length }} lignes capturées</span>
            <span>•</span>
            <span v-if="instance.status === 'running'" class="text-amber-400 animate-pulse">Flux en direct actif</span>
            <span v-else class="text-zinc-500">Instance inactive</span>
          </div>
        </div>
      </div>

      <!-- Filters & Action buttons -->
      <div class="flex flex-wrap items-center gap-2 w-full sm:w-auto">
        <!-- Level pills -->
        <div class="flex items-center gap-1 bg-zinc-950 p-1 rounded-xl border border-zinc-800 text-[11px] font-mono">
          <button
            type="button"
            @click="selectedLevel = 'ALL'"
            :class="[
              'px-2 py-0.5 rounded-lg transition-colors',
              selectedLevel === 'ALL' ? 'bg-zinc-800 text-zinc-100' : 'text-zinc-500 hover:text-zinc-300'
            ]"
          >
            Tous
          </button>
          <button
            type="button"
            @click="selectedLevel = 'ERROR'"
            :class="[
              'px-2 py-0.5 rounded-lg transition-colors',
              selectedLevel === 'ERROR' ? 'bg-rose-500/20 text-rose-300 font-bold' : 'text-zinc-500 hover:text-zinc-300'
            ]"
          >
            Erreurs
          </button>
          <button
            type="button"
            @click="selectedLevel = 'WARN'"
            :class="[
              'px-2 py-0.5 rounded-lg transition-colors',
              selectedLevel === 'WARN' ? 'bg-amber-500/20 text-amber-300' : 'text-zinc-500 hover:text-zinc-300'
            ]"
          >
            Warnings
          </button>
        </div>

        <!-- Autoscroll toggle -->
        <button
          type="button"
          @click="autoScroll = !autoScroll"
          :class="[
            'px-2.5 py-1.5 rounded-xl border text-xs font-mono flex items-center gap-1.5 transition-colors',
            autoScroll
              ? 'bg-amber-500/15 border-amber-500/40 text-amber-300'
              : 'bg-zinc-950 border-zinc-800 text-zinc-500'
          ]"
          title="Activer / désactiver le défilement automatique"
        >
          <ArrowDownCircle class="w-3.5 h-3.5" />
          <span>Auto-scroll</span>
        </button>

        <UiButton variant="ghost" size="sm" @click="copyLogs">
          <template #prefix>
            <Check v-if="copied" class="w-3.5 h-3.5 text-emerald-400" />
            <Copy v-else class="w-3.5 h-3.5" />
          </template>
          {{ copied ? 'Copié' : 'Copier' }}
        </UiButton>

        <UiButton variant="ghost" size="sm" @click="clearLogs">
          <template #prefix>
            <Trash2 class="w-3.5 h-3.5" />
          </template>
          Effacer
        </UiButton>
      </div>
    </div>

    <!-- Filter input -->
    <div class="relative">
      <Search class="absolute left-3 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-zinc-500" />
      <input
        v-model="filterText"
        type="text"
        placeholder="Filtrer dans la sortie console..."
        class="w-full bg-zinc-950/80 text-zinc-100 placeholder-zinc-600 text-xs rounded-xl pl-9 pr-4 py-2 border border-zinc-800 focus:border-amber-500/70 outline-none font-mono"
      />
    </div>

    <!-- Terminal Window -->
    <div
      ref="terminalRef"
      class="bg-zinc-950 border border-zinc-800/80 rounded-2xl p-4 font-mono text-xs overflow-y-auto max-h-[520px] min-h-[340px] space-y-1 shadow-2xl selection:bg-amber-500/30 selection:text-amber-200"
    >
      <div
        v-for="(log, idx) in filteredLogs"
        :key="idx"
        class="flex items-start gap-2 leading-relaxed hover:bg-zinc-900/40 px-1 py-0.5 rounded transition-colors"
      >
        <span class="text-zinc-600 select-none flex-shrink-0">[{{ log.timestamp }}]</span>
        <span class="text-zinc-500 font-medium flex-shrink-0">[{{ log.source }}]:</span>
        <span :class="['break-all whitespace-pre-wrap', levelColors[log.level] || 'text-zinc-300']">
          {{ log.message }}
        </span>
      </div>

      <div v-if="filteredLogs.length === 0" class="text-zinc-600 italic py-16 text-center space-y-2">
        <Terminal class="w-8 h-8 mx-auto text-zinc-700 mb-2" />
        <div>Aucun message console à afficher.</div>
        <p class="text-[11px] text-zinc-600">
          Les flux stdout et stderr de la machine virtuelle Java seront relayés en temps réel dès le démarrage.
        </p>
      </div>
    </div>
  </div>
</template>
