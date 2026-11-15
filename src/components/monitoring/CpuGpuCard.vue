<script setup lang="ts">
import { computed } from 'vue'
import {
  Cpu,
  Tv,
  Thermometer,
  Zap,
  Fan,
  BatteryCharging,
  BatteryMedium,
  SlidersHorizontal
} from 'lucide-vue-next'
import type { CpuSnapshot, GpuSnapshot, BatterySnapshot } from '@/models/monitoring'
import SparklineChart from './SparklineChart.vue'
import { getMetricColorClass } from '@/stores/useSystemMonitorStore'

interface Props {
  cpu: CpuSnapshot
  gpu: GpuSnapshot | null
  battery: BatterySnapshot | null
  cpuHistory?: number[]
  gpuHistory?: number[]
}

const props = defineProps<Props>()

const cpuUsage = computed(() => Math.round(props.cpu.globalUsage * 10) / 10)
const cpuColor = computed(() => getMetricColorClass(cpuUsage.value))

const gpuUsage = computed(() =>
  props.gpu?.usagePercent != null ? Math.round(props.gpu.usagePercent * 10) / 10 : null
)
const gpuColor = computed(() => getMetricColorClass(gpuUsage.value ?? 0))

function formatFreq(mhz: number): string {
  if (mhz >= 1000) {
    return `${(mhz / 1000).toFixed(2)} GHz`
  }
  return `${mhz} MHz`
}

function getTempColorClass(temp?: number): string {
  if (temp == null) return 'text-zinc-500'
  if (temp >= 85) return 'text-rose-400 font-bold'
  if (temp >= 72) return 'text-amber-400'
  return 'text-emerald-400'
}

const vramInfo = computed(() => {
  if (!props.gpu?.memoryTotalMb || !props.gpu?.memoryUsedMb) return null
  const usedGb = (props.gpu.memoryUsedMb / 1024).toFixed(1)
  const totalGb = (props.gpu.memoryTotalMb / 1024).toFixed(1)
  const pct = Math.round((props.gpu.memoryUsedMb / props.gpu.memoryTotalMb) * 100)
  return { usedGb, totalGb, pct }
})
</script>

<template>
  <div class="grid grid-cols-1 xl:grid-cols-2 gap-6">
    <!-- CPU CARD -->
    <div class="p-6 rounded-2xl bg-zinc-900/90 border border-zinc-800 shadow-xl flex flex-col justify-between">
      <div>
        <!-- Card Header -->
        <div class="flex items-start justify-between gap-4 pb-4 border-b border-zinc-800/80">
          <div class="flex items-center gap-3">
            <div class="p-2.5 rounded-xl bg-amber-500/10 border border-amber-500/20 text-amber-400">
              <Cpu class="w-6 h-6" />
            </div>
            <div>
              <div class="flex items-center gap-2">
                <h3 class="text-base font-bold text-zinc-100">Processeur (CPU)</h3>
                <span
                  v-if="cpu.powerGovernor"
                  class="text-[10px] uppercase font-mono px-2 py-0.5 rounded-full bg-zinc-800 text-amber-400 border border-zinc-700/80"
                >
                  {{ cpu.powerGovernor }}
                </span>
              </div>
              <p class="text-xs text-zinc-400 font-medium truncate max-w-[280px] sm:max-w-[340px]">
                {{ cpu.model }}
              </p>
            </div>
          </div>

          <!-- Total CPU Usage Badge -->
          <div class="flex flex-col items-end">
            <div class="flex items-baseline gap-1">
              <span class="text-3xl font-extrabold font-mono" :class="cpuColor.text">
                {{ cpuUsage }}
              </span>
              <span class="text-sm font-semibold text-zinc-500">%</span>
            </div>
            <div class="text-[11px] font-mono text-zinc-400 flex items-center gap-2">
              <span>{{ cpu.physicalCores }}C / {{ cpu.logicalCores }}T</span>
              <span v-if="cpu.packageTemperatureC != null" class="flex items-center gap-1 font-semibold" :class="getTempColorClass(cpu.packageTemperatureC)">
                <Thermometer class="w-3 h-3" />
                {{ cpu.packageTemperatureC.toFixed(0) }}°C
              </span>
            </div>
          </div>
        </div>

        <!-- 60s Sparkline Chart -->
        <div class="pt-4 pb-2">
          <div class="flex items-center justify-between text-[11px] text-zinc-500 font-mono mb-1">
            <span>Historique Charge CPU (60s)</span>
            <span :class="cpuColor.text" class="font-bold">{{ cpuUsage }}%</span>
          </div>
          <SparklineChart
            :data="cpuHistory || [cpuUsage]"
            :color="cpuUsage >= 90 ? 'rose' : cpuUsage >= 75 ? 'amber' : 'emerald'"
            :min="0"
            :max="100"
            :height="52"
          />
        </div>

        <!-- Cores Breakdown Grid -->
        <div class="pt-3">
          <div class="flex items-center justify-between text-xs font-semibold text-zinc-300 mb-2.5">
            <span class="flex items-center gap-1.5">
              <SlidersHorizontal class="w-3.5 h-3.5 text-amber-400" />
              <span>Charge détaillée par Cœur / Thread</span>
            </span>
            <span class="text-[11px] font-mono text-zinc-500">{{ cpu.cores.length }} Threads</span>
          </div>

          <div class="grid grid-cols-2 sm:grid-cols-4 gap-2.5 max-h-48 overflow-y-auto pr-1">
            <div
              v-for="core in cpu.cores"
              :key="core.id"
              class="p-2 rounded-xl bg-zinc-950/60 border border-zinc-800/80 hover:border-zinc-700/80 transition-colors"
            >
              <div class="flex items-center justify-between text-[11px] font-mono mb-1">
                <span class="text-zinc-400 font-medium">#{{ core.id }}</span>
                <span class="font-bold" :class="getMetricColorClass(core.usage).text">
                  {{ Math.round(core.usage) }}%
                </span>
              </div>

              <!-- Mini Progress Bar -->
              <div class="w-full h-1.5 bg-zinc-800 rounded-full overflow-hidden mb-1.5">
                <div
                  class="h-full rounded-full transition-all duration-300"
                  :style="{
                    width: `${Math.min(100, Math.max(0, core.usage))}%`,
                    backgroundColor: getMetricColorClass(core.usage).hex
                  }"
                />
              </div>

              <div class="flex items-center justify-between text-[10px] font-mono text-zinc-500">
                <span>{{ formatFreq(core.frequencyMhz) }}</span>
                <span v-if="core.temperatureC" :class="getTempColorClass(core.temperatureC)">
                  {{ core.temperatureC.toFixed(0) }}°
                </span>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Battery / Laptop strip if active -->
      <div
        v-if="battery && battery.present"
        class="mt-4 pt-3 border-t border-zinc-800/80 flex items-center justify-between text-xs font-mono"
      >
        <div class="flex items-center gap-2">
          <component
            :is="battery.state === 'Charging' ? BatteryCharging : BatteryMedium"
            class="w-4 h-4"
            :class="battery.percentage <= 20 ? 'text-rose-400' : 'text-amber-400'"
          />
          <span class="text-zinc-300">Batterie ({{ battery.state }})</span>
        </div>
        <div class="flex items-center gap-3">
          <span v-if="battery.powerWatts != null" class="text-zinc-400">
            {{ battery.powerWatts.toFixed(1) }}W
          </span>
          <span class="font-bold text-zinc-100">{{ Math.round(battery.percentage) }}%</span>
        </div>
      </div>
    </div>

    <!-- GPU CARD -->
    <div class="p-6 rounded-2xl bg-zinc-900/90 border border-zinc-800 shadow-xl flex flex-col justify-between">
      <div v-if="gpu">
        <!-- GPU Card Header -->
        <div class="flex items-start justify-between gap-4 pb-4 border-b border-zinc-800/80">
          <div class="flex items-center gap-3">
            <div class="p-2.5 rounded-xl bg-amber-500/10 border border-amber-500/20 text-amber-400">
              <Tv class="w-6 h-6" />
            </div>
            <div>
              <div class="flex items-center gap-2">
                <h3 class="text-base font-bold text-zinc-100">Carte Graphique (GPU)</h3>
                <span class="text-[10px] uppercase font-mono px-2 py-0.5 rounded-full bg-zinc-800 text-amber-400 border border-zinc-700/80">
                  {{ gpu.vendor }}
                </span>
              </div>
              <p class="text-xs text-zinc-400 font-medium truncate max-w-[280px] sm:max-w-[340px]">
                {{ gpu.name }}
              </p>
            </div>
          </div>

          <!-- Total GPU Usage Badge -->
          <div class="flex flex-col items-end">
            <div class="flex items-baseline gap-1">
              <span class="text-3xl font-extrabold font-mono" :class="gpuColor.text">
                {{ gpuUsage != null ? gpuUsage : '--' }}
              </span>
              <span class="text-sm font-semibold text-zinc-500">%</span>
            </div>
            <div class="text-[11px] font-mono text-zinc-400 flex items-center gap-2">
              <span v-if="gpu.driverVersion">Pilote {{ gpu.driverVersion }}</span>
              <span v-if="gpu.coreTempC != null" class="flex items-center gap-1 font-semibold" :class="getTempColorClass(gpu.coreTempC)">
                <Thermometer class="w-3 h-3" />
                {{ gpu.coreTempC.toFixed(0) }}°C
              </span>
            </div>
          </div>
        </div>

        <!-- 60s Sparkline Chart -->
        <div class="pt-4 pb-2">
          <div class="flex items-center justify-between text-[11px] text-zinc-500 font-mono mb-1">
            <span>Historique Charge GPU (60s)</span>
            <span :class="gpuColor.text" class="font-bold">{{ gpuUsage != null ? `${gpuUsage}%` : '--' }}</span>
          </div>
          <SparklineChart
            :data="gpuHistory || [gpuUsage || 0]"
            :color="gpuUsage && gpuUsage >= 90 ? 'rose' : gpuUsage && gpuUsage >= 75 ? 'amber' : 'emerald'"
            :min="0"
            :max="100"
            :height="52"
          />
        </div>

        <!-- GPU Details & Telemetry (VRAM, Power, Temps, Fans) -->
        <div class="pt-3 space-y-3">
          <!-- VRAM Section -->
          <div v-if="vramInfo" class="p-3 rounded-xl bg-zinc-950/60 border border-zinc-800/80">
            <div class="flex items-center justify-between text-xs font-mono mb-1.5">
              <span class="text-zinc-400 font-semibold">Mémoire Vidéo (VRAM)</span>
              <span class="text-zinc-200">
                <span class="font-bold text-amber-400">{{ vramInfo.usedGb }} Go</span>
                <span class="text-zinc-500"> / {{ vramInfo.totalGb }} Go</span>
                <span class="text-zinc-400 ml-1.5">({{ vramInfo.pct }}%)</span>
              </span>
            </div>
            <div class="w-full h-2 bg-zinc-800 rounded-full overflow-hidden">
              <div
                class="h-full rounded-full transition-all duration-300"
                :style="{
                  width: `${vramInfo.pct}%`,
                  backgroundColor: getMetricColorClass(vramInfo.pct).hex
                }"
              />
            </div>
          </div>

          <!-- Telemetry Badges Grid -->
          <div class="grid grid-cols-2 sm:grid-cols-4 gap-2.5">
            <!-- Power in Watts -->
            <div class="p-2.5 rounded-xl bg-zinc-950/60 border border-zinc-800/80">
              <div class="flex items-center gap-1.5 text-zinc-400 text-[11px] mb-1">
                <Zap class="w-3.5 h-3.5 text-amber-400" />
                <span>Puissance</span>
              </div>
              <p class="font-mono font-bold text-sm text-zinc-100">
                {{ gpu.powerWatts != null ? `${gpu.powerWatts.toFixed(1)} W` : '--' }}
              </p>
            </div>

            <!-- Fan Speed -->
            <div class="p-2.5 rounded-xl bg-zinc-950/60 border border-zinc-800/80">
              <div class="flex items-center gap-1.5 text-zinc-400 text-[11px] mb-1">
                <Fan class="w-3.5 h-3.5 text-sky-400" />
                <span>Ventilateur</span>
              </div>
              <p class="font-mono font-bold text-sm text-zinc-100">
                {{ gpu.fanSpeedPercent != null ? `${gpu.fanSpeedPercent}%` : gpu.fanSpeedRpm != null ? `${gpu.fanSpeedRpm} RPM` : 'Auto' }}
              </p>
            </div>

            <!-- Hotspot Temp -->
            <div class="p-2.5 rounded-xl bg-zinc-950/60 border border-zinc-800/80">
              <div class="flex items-center gap-1.5 text-zinc-400 text-[11px] mb-1">
                <Thermometer class="w-3.5 h-3.5 text-rose-400" />
                <span>Hotspot</span>
              </div>
              <p class="font-mono font-bold text-sm" :class="getTempColorClass(gpu.hotspotTempC)">
                {{ gpu.hotspotTempC != null ? `${gpu.hotspotTempC.toFixed(0)}°C` : '--' }}
              </p>
            </div>

            <!-- VRAM Temp -->
            <div class="p-2.5 rounded-xl bg-zinc-950/60 border border-zinc-800/80">
              <div class="flex items-center gap-1.5 text-zinc-400 text-[11px] mb-1">
                <Thermometer class="w-3.5 h-3.5 text-amber-400" />
                <span>VRAM Temp</span>
              </div>
              <p class="font-mono font-bold text-sm" :class="getTempColorClass(gpu.vramTempC)">
                {{ gpu.vramTempC != null ? `${gpu.vramTempC.toFixed(0)}°C` : '--' }}
              </p>
            </div>
          </div>
        </div>
      </div>

      <!-- GPU Fallback when no discrete GPU metrics detected -->
      <div v-else class="h-full flex flex-col items-center justify-center py-12 text-center">
        <div class="p-4 rounded-2xl bg-zinc-800/50 border border-zinc-700/60 text-zinc-500 mb-3">
          <Tv class="w-8 h-8" />
        </div>
        <h4 class="text-sm font-bold text-zinc-300">GPU Dédié Non Détecté</h4>
        <p class="text-xs text-zinc-500 max-w-sm mt-1">
          Aucun capteur matériel Nvidia/AMD/Intel actif n'a pu être relevé via les pilotes Linux.
        </p>
      </div>
    </div>
  </div>
</template>
