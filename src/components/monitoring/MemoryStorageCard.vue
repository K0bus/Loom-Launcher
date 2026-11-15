<script setup lang="ts">
import { computed } from 'vue'
import {
  HardDrive,
  Database,
  ArrowDownCircle,
  ArrowUpCircle,
  Layers,
  Thermometer
} from 'lucide-vue-next'
import type { MemorySnapshot, DiskActivitySnapshot } from '@/models/monitoring'
import SparklineChart from './SparklineChart.vue'
import { getMetricColorClass } from '@/stores/useSystemMonitorStore'

interface Props {
  memory: MemorySnapshot
  disks: DiskActivitySnapshot
  ramHistory?: number[]
  diskReadHistory?: number[]
  diskWriteHistory?: number[]
}

const props = defineProps<Props>()

function bytesToGb(bytes: number): string {
  return (bytes / (1024 * 1024 * 1024)).toFixed(1)
}

function bytesToMbPerSec(bytes: number): string {
  const mb = bytes / (1024 * 1024)
  if (mb >= 100) return `${mb.toFixed(0)} Mo/s`
  if (mb >= 1) return `${mb.toFixed(1)} Mo/s`
  const kb = bytes / 1024
  return `${kb.toFixed(0)} Ko/s`
}

const ramUsage = computed(() => Math.round(props.memory.usagePercent * 10) / 10)
const ramColor = computed(() => getMetricColorClass(ramUsage.value))

const usedGb = computed(() => bytesToGb(props.memory.usedBytes))
const totalGb = computed(() => bytesToGb(props.memory.totalBytes))
const availGb = computed(() => bytesToGb(props.memory.availableBytes))
const cachedGb = computed(() => bytesToGb(props.memory.cachedBytes + props.memory.buffersBytes))

const swapUsage = computed(() => Math.round(props.memory.swapUsagePercent * 10) / 10)
const swapUsedGb = computed(() => bytesToGb(props.memory.swapUsedBytes))
const swapTotalGb = computed(() => bytesToGb(props.memory.swapTotalBytes))

// Percentages for stacked RAM breakdown
const usedPct = computed(() => {
  if (props.memory.totalBytes === 0) return 0
  const realUsed = Math.max(0, props.memory.usedBytes - (props.memory.cachedBytes + props.memory.buffersBytes))
  return Math.max(0, Math.round((realUsed / props.memory.totalBytes) * 100))
})

const cachePct = computed(() => {
  if (props.memory.totalBytes === 0) return 0
  return Math.min(100, Math.round(((props.memory.cachedBytes + props.memory.buffersBytes) / props.memory.totalBytes) * 100))
})
</script>

<template>
  <div class="grid grid-cols-1 xl:grid-cols-2 gap-6">
    <!-- MEMORY CARD (RAM & SWAP) -->
    <div class="p-6 rounded-2xl bg-zinc-900/90 border border-zinc-800 shadow-xl flex flex-col justify-between">
      <div>
        <!-- Card Header -->
        <div class="flex items-start justify-between gap-4 pb-4 border-b border-zinc-800/80">
          <div class="flex items-center gap-3">
            <div class="p-2.5 rounded-xl bg-amber-500/10 border border-amber-500/20 text-amber-400">
              <Database class="w-6 h-6" />
            </div>
            <div>
              <h3 class="text-base font-bold text-zinc-100">Mémoire Vive (RAM & Swap)</h3>
              <p class="text-xs text-zinc-400 font-medium">
                {{ usedGb }} Go utilisés sur {{ totalGb }} Go installés
              </p>
            </div>
          </div>

          <!-- Total RAM Usage Badge -->
          <div class="flex flex-col items-end">
            <div class="flex items-baseline gap-1">
              <span class="text-3xl font-extrabold font-mono" :class="ramColor.text">
                {{ ramUsage }}
              </span>
              <span class="text-sm font-semibold text-zinc-500">%</span>
            </div>
            <div class="text-[11px] font-mono text-zinc-400">
              Dispo : {{ availGb }} Go
            </div>
          </div>
        </div>

        <!-- 60s Sparkline Chart -->
        <div class="pt-4 pb-2">
          <div class="flex items-center justify-between text-[11px] text-zinc-500 font-mono mb-1">
            <span>Historique Mémoire (60s)</span>
            <span :class="ramColor.text" class="font-bold">{{ ramUsage }}%</span>
          </div>
          <SparklineChart
            :data="ramHistory || [ramUsage]"
            :color="ramUsage >= 90 ? 'rose' : ramUsage >= 75 ? 'amber' : 'emerald'"
            :min="0"
            :max="100"
            :height="52"
          />
        </div>

        <!-- Stacked RAM Allocation Bar -->
        <div class="pt-3">
          <div class="flex items-center justify-between text-xs font-semibold text-zinc-300 mb-2">
            <span>Répartition Mémoire</span>
            <span class="text-[11px] font-mono text-zinc-400">Cache/Buffers : {{ cachedGb }} Go</span>
          </div>

          <div class="w-full h-3 bg-zinc-950 rounded-full overflow-hidden flex border border-zinc-800/80 p-0.5">
            <!-- Active used -->
            <div
              class="h-full rounded-l-full bg-amber-500 transition-all duration-300"
              :style="{ width: `${usedPct}%` }"
              title="Mémoire Active Utilisée"
            />
            <!-- Cached / Buffers -->
            <div
              class="h-full bg-sky-500/80 transition-all duration-300"
              :style="{ width: `${cachePct}%` }"
              title="Cache & Buffers Noyau"
            />
            <!-- Free -->
            <div class="flex-1 bg-zinc-800/40 rounded-r-full" title="Mémoire Libre" />
          </div>

          <!-- Legend -->
          <div class="flex items-center justify-between text-[10px] font-mono text-zinc-400 mt-2 px-1">
            <span class="flex items-center gap-1.5">
              <span class="w-2 h-2 rounded-full bg-amber-500 inline-block" />
              <span>Active ({{ usedGb }} Go)</span>
            </span>
            <span class="flex items-center gap-1.5">
              <span class="w-2 h-2 rounded-full bg-sky-500/80 inline-block" />
              <span>Cache/Buffers ({{ cachedGb }} Go)</span>
            </span>
            <span class="flex items-center gap-1.5">
              <span class="w-2 h-2 rounded-full bg-zinc-700 inline-block" />
              <span>Libre ({{ availGb }} Go)</span>
            </span>
          </div>
        </div>

        <!-- SWAP SECTION -->
        <div class="mt-4 pt-3 border-t border-zinc-800/80">
          <div class="flex items-center justify-between text-xs font-semibold text-zinc-300 mb-2">
            <div class="flex items-center gap-1.5">
              <Layers class="w-3.5 h-3.5 text-zinc-400" />
              <span>Fichier d'Échange (Swap)</span>
            </div>
            <span class="font-mono text-xs text-zinc-400">
              <span :class="getMetricColorClass(swapUsage).text" class="font-bold">{{ swapUsedGb }} Go</span>
              <span class="text-zinc-500"> / {{ swapTotalGb }} Go</span>
              <span class="ml-1 text-[11px] text-zinc-400">({{ swapUsage }}%)</span>
            </span>
          </div>

          <div class="w-full h-2 bg-zinc-950 rounded-full overflow-hidden border border-zinc-800/80">
            <div
              class="h-full rounded-full transition-all duration-300"
              :style="{
                width: `${swapUsage}%`,
                backgroundColor: getMetricColorClass(swapUsage).hex
              }"
            />
          </div>
        </div>
      </div>
    </div>

    <!-- STORAGE & DISKS CARD -->
    <div class="p-6 rounded-2xl bg-zinc-900/90 border border-zinc-800 shadow-xl flex flex-col justify-between">
      <div>
        <!-- Card Header -->
        <div class="flex items-start justify-between gap-4 pb-4 border-b border-zinc-800/80">
          <div class="flex items-center gap-3">
            <div class="p-2.5 rounded-xl bg-amber-500/10 border border-amber-500/20 text-amber-400">
              <HardDrive class="w-6 h-6" />
            </div>
            <div>
              <h3 class="text-base font-bold text-zinc-100">Stockage & Partitions</h3>
              <p class="text-xs text-zinc-400 font-medium">
                {{ disks.partitions.length }} volumes et points de montage détectés
              </p>
            </div>
          </div>

          <!-- Real-time Disk I/O throughput badges -->
          <div class="flex items-center gap-3">
            <div class="flex items-center gap-1.5 px-2.5 py-1 rounded-xl bg-zinc-950/70 border border-zinc-800">
              <ArrowDownCircle class="w-3.5 h-3.5 text-emerald-400" />
              <div class="flex flex-col text-right">
                <span class="text-[9px] uppercase tracking-wider text-zinc-500 font-mono font-bold">Lecture</span>
                <span class="font-mono text-xs font-bold text-zinc-200">
                  {{ bytesToMbPerSec(disks.readBytesPerSec) }}
                </span>
              </div>
            </div>

            <div class="flex items-center gap-1.5 px-2.5 py-1 rounded-xl bg-zinc-950/70 border border-zinc-800">
              <ArrowUpCircle class="w-3.5 h-3.5 text-sky-400" />
              <div class="flex flex-col text-right">
                <span class="text-[9px] uppercase tracking-wider text-zinc-500 font-mono font-bold">Écriture</span>
                <span class="font-mono text-xs font-bold text-zinc-200">
                  {{ bytesToMbPerSec(disks.writeBytesPerSec) }}
                </span>
              </div>
            </div>
          </div>
        </div>

        <!-- Partitions List -->
        <div class="pt-4 space-y-3 max-h-[280px] overflow-y-auto pr-1">
          <div
            v-for="part in disks.partitions"
            :key="part.mountPoint"
            class="p-3 rounded-xl bg-zinc-950/60 border border-zinc-800/80 hover:border-zinc-700/80 transition-colors"
          >
            <div class="flex items-start justify-between gap-2 mb-1.5">
              <div>
                <div class="flex items-center gap-2">
                  <span class="font-mono font-bold text-xs text-zinc-200">{{ part.mountPoint }}</span>
                  <span class="text-[10px] uppercase font-mono px-1.5 py-0.5 rounded bg-zinc-800 text-zinc-400 border border-zinc-700/60">
                    {{ part.fileSystem }}
                  </span>
                  <span
                    v-if="part.isRemovable"
                    class="text-[10px] uppercase font-mono px-1.5 py-0.5 rounded bg-amber-500/10 text-amber-400 border border-amber-500/20"
                  >
                    Amovible
                  </span>
                </div>
                <p class="text-[11px] text-zinc-500 font-mono mt-0.5">
                  Device: {{ part.name }}
                </p>
              </div>

              <!-- Occupancy info -->
              <div class="text-right">
                <div class="font-mono text-xs font-bold" :class="getMetricColorClass(part.usagePercent).text">
                  {{ Math.round(part.usagePercent) }}%
                </div>
                <div class="text-[11px] font-mono text-zinc-400 flex items-center justify-end gap-1.5">
                  <span>{{ bytesToGb(part.usedBytes) }} / {{ bytesToGb(part.totalBytes) }} Go</span>
                  <span v-if="part.temperatureC != null" class="text-emerald-400 flex items-center font-bold">
                    <Thermometer class="w-3 h-3 inline" />
                    {{ part.temperatureC.toFixed(0) }}°
                  </span>
                </div>
              </div>
            </div>

            <!-- Capacity Progress Bar -->
            <div class="w-full h-1.5 bg-zinc-800 rounded-full overflow-hidden">
              <div
                class="h-full rounded-full transition-all duration-300"
                :style="{
                  width: `${part.usagePercent}%`,
                  backgroundColor: getMetricColorClass(part.usagePercent).hex
                }"
              />
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
