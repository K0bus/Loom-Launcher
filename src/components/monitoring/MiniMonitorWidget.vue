<script setup lang="ts">
import { ref, computed } from 'vue'
import {
  Cpu,
  Tv,
  Database,
  ArrowDownUp,
  ChevronDown,
  ChevronUp,
  Activity
} from 'lucide-vue-next'
import { useSystemMonitorStore, getMetricColorClass } from '@/stores/useSystemMonitorStore'

const monitorStore = useSystemMonitorStore()
const isCollapsed = ref(false)

const cpuUsage = computed(() =>
  monitorStore.metrics ? Math.round(monitorStore.metrics.cpu.globalUsage) : 0
)
const ramUsage = computed(() =>
  monitorStore.metrics ? Math.round(monitorStore.metrics.memory.usagePercent) : 0
)
const gpuUsage = computed(() =>
  monitorStore.metrics?.gpu?.usagePercent != null
    ? Math.round(monitorStore.metrics.gpu.usagePercent)
    : null
)

const networkSpeed = computed(() => {
  if (!monitorStore.metrics) return '0 Mo/s'
  const dlMb = monitorStore.metrics.network.rxBytesPerSec / (1024 * 1024)
  return `${dlMb.toFixed(1)} Mo/s`
})
</script>

<template>
  <div class="fixed bottom-6 right-6 z-40 select-none">
    <div class="bg-zinc-950/95 border border-zinc-800/90 shadow-2xl rounded-2xl backdrop-blur-md overflow-hidden transition-all duration-300">
      <!-- Mini Header Bar -->
      <div
        class="flex items-center justify-between px-3.5 py-2 cursor-pointer bg-zinc-900/60 hover:bg-zinc-900 transition-colors gap-3"
        @click="isCollapsed = !isCollapsed"
      >
        <div class="flex items-center gap-2">
          <Activity class="w-4 h-4 text-amber-400 animate-pulse" />
          <span class="text-xs font-bold font-mono tracking-tight text-zinc-200">LOOM HUD</span>
        </div>

        <div class="flex items-center gap-2">
          <span class="text-[10px] font-mono text-zinc-500">
            {{ monitorStore.isPaused ? 'EN PAUSE' : `${monitorStore.refreshInterval}ms` }}
          </span>
          <component
            :is="isCollapsed ? ChevronUp : ChevronDown"
            class="w-3.5 h-3.5 text-zinc-400"
          />
        </div>
      </div>

      <!-- Collapsible Body -->
      <div v-show="!isCollapsed" class="p-3 space-y-2 text-xs font-mono min-w-[200px]">
        <!-- CPU Line -->
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-1.5 text-zinc-400">
            <Cpu class="w-3.5 h-3.5 text-amber-400" />
            <span>CPU</span>
          </div>
          <span class="font-bold" :class="getMetricColorClass(cpuUsage).text">
            {{ cpuUsage }}%
          </span>
        </div>

        <!-- GPU Line -->
        <div v-if="gpuUsage != null" class="flex items-center justify-between">
          <div class="flex items-center gap-1.5 text-zinc-400">
            <Tv class="w-3.5 h-3.5 text-sky-400" />
            <span>GPU</span>
          </div>
          <span class="font-bold" :class="getMetricColorClass(gpuUsage).text">
            {{ gpuUsage }}%
          </span>
        </div>

        <!-- RAM Line -->
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-1.5 text-zinc-400">
            <Database class="w-3.5 h-3.5 text-emerald-400" />
            <span>RAM</span>
          </div>
          <span class="font-bold" :class="getMetricColorClass(ramUsage).text">
            {{ ramUsage }}%
          </span>
        </div>

        <!-- Network Line -->
        <div class="flex items-center justify-between pt-1 border-t border-zinc-800/80">
          <div class="flex items-center gap-1.5 text-zinc-400">
            <ArrowDownUp class="w-3.5 h-3.5 text-zinc-400" />
            <span>Réseau</span>
          </div>
          <span class="text-zinc-200 font-semibold">
            {{ networkSpeed }}
          </span>
        </div>
      </div>
    </div>
  </div>
</template>
