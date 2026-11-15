<script setup lang="ts">
import { ref, computed, watch, nextTick } from 'vue'
import {
  Terminal,
  Trash2,
  ArrowDown
} from 'lucide-vue-next'
import { useMinecraftStore } from '@/stores/minecraft'
import UiBadge from '@/components/ui/UiBadge.vue'

const minecraftStore = useMinecraftStore()
const activeInst = computed(() => minecraftStore.activeInstance)

const logsContainerRef = ref<HTMLDivElement | null>(null)
const autoScroll = ref(true)
const selectedLevel = ref<'ALL' | 'INFO' | 'WARN' | 'ERROR'>('ALL')

const logs = computed(() => {
  if (!activeInst.value?.logs) return []
  if (selectedLevel.value === 'ALL') return activeInst.value.logs
  return activeInst.value.logs.filter((l) => l.level === selectedLevel.value)
})

function scrollToBottom() {
  if (autoScroll.value && logsContainerRef.value) {
    logsContainerRef.value.scrollTop = logsContainerRef.value.scrollHeight
  }
}

watch(
  () => activeInst.value?.logs?.length,
  () => {
    nextTick(scrollToBottom)
  }
)

function clearConsole() {
  if (activeInst.value) {
    minecraftStore.clearLogs(activeInst.value.id)
  }
}

function getLevelClass(level: string) {
  switch (level) {
    case 'ERROR':
      return 'text-rose-400 font-bold bg-rose-500/10 px-1.5 py-0.5 rounded'
    case 'WARN':
      return 'text-amber-400 font-bold bg-amber-500/10 px-1.5 py-0.5 rounded'
    case 'DEBUG':
      return 'text-zinc-500 font-medium'
    default:
      return 'text-emerald-400 font-semibold'
  }
}
</script>

<template>
  <div class="space-y-4">
    <!-- 1. CONSOLE TOOLBAR -->
    <div class="flex flex-col sm:flex-row items-stretch sm:items-center justify-between gap-4 bg-zinc-900/60 p-4 rounded-2xl border border-zinc-800/80">
      <div class="flex items-center gap-3">
        <div class="flex items-center gap-2">
          <Terminal class="w-4 h-4 text-amber-400" />
          <span class="text-xs font-mono font-bold text-zinc-300 uppercase tracking-wider">
            Console JVM
          </span>
        </div>

        <UiBadge
          :variant="activeInst?.status === 'running' ? 'emerald' : 'zinc'"
          size="xs"
          class="font-mono flex items-center gap-1"
        >
          <span
            v-if="activeInst?.status === 'running'"
            class="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse"
          />
          {{ activeInst?.status === 'running' ? 'PROCESSUS ACTIF' : 'IDLE' }}
        </UiBadge>
      </div>

      <!-- Controls: Level filter, Auto-scroll, Clear -->
      <div class="flex items-center gap-2 flex-wrap">
        <!-- Level Filters -->
        <div class="flex items-center bg-zinc-950 p-1 rounded-xl border border-zinc-800 text-[11px] font-mono">
          <button
            type="button"
            @click="selectedLevel = 'ALL'"
            :class="[
              'px-2.5 py-1 rounded-lg transition-colors',
              selectedLevel === 'ALL' ? 'bg-zinc-800 text-zinc-100 font-bold' : 'text-zinc-500 hover:text-zinc-300'
            ]"
          >
            Tous
          </button>
          <button
            type="button"
            @click="selectedLevel = 'INFO'"
            :class="[
              'px-2.5 py-1 rounded-lg transition-colors',
              selectedLevel === 'INFO' ? 'bg-emerald-500/20 text-emerald-400 font-bold' : 'text-zinc-500 hover:text-zinc-300'
            ]"
          >
            Info
          </button>
          <button
            type="button"
            @click="selectedLevel = 'WARN'"
            :class="[
              'px-2.5 py-1 rounded-lg transition-colors',
              selectedLevel === 'WARN' ? 'bg-amber-500/20 text-amber-400 font-bold' : 'text-zinc-500 hover:text-zinc-300'
            ]"
          >
            Warn
          </button>
          <button
            type="button"
            @click="selectedLevel = 'ERROR'"
            :class="[
              'px-2.5 py-1 rounded-lg transition-colors',
              selectedLevel === 'ERROR' ? 'bg-rose-500/20 text-rose-400 font-bold' : 'text-zinc-500 hover:text-zinc-300'
            ]"
          >
            Error
          </button>
        </div>

        <!-- Auto-scroll toggle -->
        <button
          type="button"
          @click="autoScroll = !autoScroll"
          :class="[
            'px-3 py-1.5 rounded-xl border text-xs font-mono flex items-center gap-1.5 transition-colors select-none',
            autoScroll
              ? 'bg-amber-500/10 border-amber-500/30 text-amber-400 font-bold'
              : 'bg-zinc-950 border-zinc-800 text-zinc-500 hover:text-zinc-300'
          ]"
          title="Défilement automatique"
        >
          <ArrowDown class="w-3.5 h-3.5" />
          <span>Auto-scroll</span>
        </button>

        <!-- Clear console -->
        <button
          type="button"
          @click="clearConsole"
          class="p-2 rounded-xl bg-zinc-950 border border-zinc-800 text-zinc-400 hover:text-rose-400 hover:border-rose-500/30 transition-colors"
          title="Effacer la console"
        >
          <Trash2 class="w-3.5 h-3.5" />
        </button>
      </div>
    </div>

    <!-- 2. VIRTUAL TERMINAL WINDOW -->
    <div
      ref="logsContainerRef"
      class="bg-zinc-950/95 border border-zinc-800 rounded-3xl p-5 font-mono text-xs leading-relaxed h-[520px] overflow-y-auto shadow-inner space-y-1.5 select-text"
    >
      <div v-if="logs.length > 0" class="space-y-1">
        <div
          v-for="(log, idx) in logs"
          :key="idx"
          class="flex items-start gap-2.5 hover:bg-zinc-900/40 px-2 py-0.5 rounded transition-colors"
        >
          <!-- Timestamp -->
          <span class="text-zinc-600 shrink-0 select-none">
            {{ log.timestamp }}
          </span>

          <!-- Level tag -->
          <span class="shrink-0 text-[10px] uppercase select-none" :class="getLevelClass(log.level)">
            {{ log.level }}
          </span>

          <!-- Source tag -->
          <span class="text-zinc-500 shrink-0 select-none">
            [{{ log.source }}]:
          </span>

          <!-- Message -->
          <span
            class="break-all whitespace-pre-wrap flex-1"
            :class="[
              log.level === 'ERROR'
                ? 'text-rose-300 font-semibold'
                : log.level === 'WARN'
                  ? 'text-amber-200'
                  : 'text-zinc-300'
            ]"
          >
            {{ log.message }}
          </span>
        </div>
      </div>

      <!-- Empty state when no logs -->
      <div
        v-else
        class="h-full flex flex-col items-center justify-center text-center space-y-2 text-zinc-600 select-none"
      >
        <Terminal class="w-8 h-8 opacity-40 text-amber-400" />
        <p class="text-xs">Console prête. Aucun flux stdout/stderr enregistré pour l'instant.</p>
        <p class="text-[11px] text-zinc-700">Lancez l'instance pour visualiser les logs JVM en temps réel.</p>
      </div>
    </div>
  </div>
</template>
