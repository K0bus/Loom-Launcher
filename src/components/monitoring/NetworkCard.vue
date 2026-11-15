<script setup lang="ts">
import { computed } from 'vue'
import {
  Wifi,
  Network,
  ArrowDownCircle,
  ArrowUpCircle,
  Globe
} from 'lucide-vue-next'
import type { NetworkSnapshot } from '@/models/monitoring'
import SparklineChart from './SparklineChart.vue'

interface Props {
  network: NetworkSnapshot
  downloadHistory?: number[]
  uploadHistory?: number[]
}

const props = defineProps<Props>()

function bytesToThroughput(bytes: number): string {
  const mb = bytes / (1024 * 1024)
  if (mb >= 10) return `${mb.toFixed(1)} Mo/s`
  if (mb >= 1) return `${mb.toFixed(2)} Mo/s`
  const kb = bytes / 1024
  if (kb >= 1) return `${kb.toFixed(1)} Ko/s`
  return `${bytes} B/s`
}

const downloadSpeed = computed(() => bytesToThroughput(props.network.rxBytesPerSec))
const uploadSpeed = computed(() => bytesToThroughput(props.network.txBytesPerSec))

// Filter non-loopback active interfaces
const physicalInterfaces = computed(() => {
  return props.network.interfaces.filter((i) => i.name !== 'lo')
})
</script>

<template>
  <div class="p-6 rounded-2xl bg-zinc-900/90 border border-zinc-800 shadow-xl">
    <!-- Header -->
    <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4 pb-4 border-b border-zinc-800/80">
      <div class="flex items-center gap-3">
        <div class="p-2.5 rounded-xl bg-amber-500/10 border border-amber-500/20 text-amber-400">
          <Globe class="w-6 h-6" />
        </div>
        <div>
          <div class="flex items-center gap-2">
            <h3 class="text-base font-bold text-zinc-100">Réseau & Connectivité</h3>
            <span
              v-if="network.wifiSsid"
              class="flex items-center gap-1 text-[11px] font-mono px-2 py-0.5 rounded-full bg-zinc-800 text-amber-400 border border-zinc-700/80"
            >
              <Wifi class="w-3 h-3" />
              <span>{{ network.wifiSsid }}</span>
              <span v-if="network.wifiSignalPercent != null">({{ network.wifiSignalPercent }}%)</span>
            </span>
          </div>
          <p class="text-xs text-zinc-400 font-medium">
            IP Locale : {{ network.localIp || 'Non assignée' }}
            <span v-if="network.defaultGateway" class="text-zinc-500"> • Passerelle : {{ network.defaultGateway }}</span>
          </p>
        </div>
      </div>

      <!-- Real-time badges for Throughput -->
      <div class="flex items-center gap-3">
        <div class="flex items-center gap-2 px-3.5 py-1.5 rounded-xl bg-zinc-950/70 border border-zinc-800">
          <ArrowDownCircle class="w-4 h-4 text-emerald-400" />
          <div class="flex flex-col">
            <span class="text-[9px] uppercase tracking-wider text-zinc-500 font-mono font-bold">Réception (DL)</span>
            <span class="font-mono text-sm font-extrabold text-emerald-400">
              {{ downloadSpeed }}
            </span>
          </div>
        </div>

        <div class="flex items-center gap-2 px-3.5 py-1.5 rounded-xl bg-zinc-950/70 border border-zinc-800">
          <ArrowUpCircle class="w-4 h-4 text-sky-400" />
          <div class="flex flex-col">
            <span class="text-[9px] uppercase tracking-wider text-zinc-500 font-mono font-bold">Envoi (UL)</span>
            <span class="font-mono text-sm font-extrabold text-sky-400">
              {{ uploadSpeed }}
            </span>
          </div>
        </div>
      </div>
    </div>

    <!-- Live Traffic Sparklines (2 columns) -->
    <div class="grid grid-cols-1 md:grid-cols-2 gap-4 pt-4 pb-2">
      <!-- Download Sparkline -->
      <div class="p-3.5 rounded-xl bg-zinc-950/60 border border-zinc-800/80">
        <div class="flex items-center justify-between text-xs font-mono mb-1">
          <span class="text-zinc-400 flex items-center gap-1.5 font-semibold">
            <ArrowDownCircle class="w-3.5 h-3.5 text-emerald-400" />
            <span>Historique Téléchargement (60s)</span>
          </span>
          <span class="font-bold text-emerald-400">{{ downloadSpeed }}</span>
        </div>
        <SparklineChart
          :data="downloadHistory || [0]"
          color="emerald"
          :height="50"
          unit="Mo/s"
        />
      </div>

      <!-- Upload Sparkline -->
      <div class="p-3.5 rounded-xl bg-zinc-950/60 border border-zinc-800/80">
        <div class="flex items-center justify-between text-xs font-mono mb-1">
          <span class="text-zinc-400 flex items-center gap-1.5 font-semibold">
            <ArrowUpCircle class="w-3.5 h-3.5 text-sky-400" />
            <span>Historique Envoi (60s)</span>
          </span>
          <span class="font-bold text-sky-400">{{ uploadSpeed }}</span>
        </div>
        <SparklineChart
          :data="uploadHistory || [0]"
          color="sky"
          :height="50"
          unit="Mo/s"
        />
      </div>
    </div>

    <!-- Active Interfaces Grid -->
    <div class="pt-3">
      <div class="text-xs font-semibold text-zinc-300 mb-2.5 flex items-center justify-between">
        <span class="flex items-center gap-1.5">
          <Network class="w-3.5 h-3.5 text-amber-400" />
          <span>Interfaces Réseau Actives</span>
        </span>
        <span class="text-[11px] font-mono text-zinc-500">{{ physicalInterfaces.length }} interfaces</span>
      </div>

      <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-2.5">
        <div
          v-for="iface in physicalInterfaces"
          :key="iface.name"
          class="p-3 rounded-xl bg-zinc-950/60 border border-zinc-800/80 hover:border-zinc-700/80 transition-colors flex items-center justify-between"
        >
          <div class="flex items-center gap-2.5">
            <div
              class="w-2.5 h-2.5 rounded-full"
              :class="iface.isUp ? 'bg-emerald-400 animate-pulse' : 'bg-zinc-600'"
            />
            <div>
              <div class="flex items-center gap-1.5">
                <span class="font-mono font-bold text-xs text-zinc-200">{{ iface.name }}</span>
                <span
                  v-if="iface.isWireless"
                  class="text-[9px] uppercase font-mono px-1 py-0.5 rounded bg-amber-500/10 text-amber-400 border border-amber-500/20"
                >
                  Wi-Fi
                </span>
              </div>
              <p class="text-[10px] font-mono text-zinc-500 truncate max-w-[140px]">
                {{ iface.ipAddress || iface.macAddress || 'Non configurée' }}
              </p>
            </div>
          </div>

          <div class="text-right font-mono text-[10px] text-zinc-400 space-y-0.5">
            <div class="text-emerald-400 font-semibold">↓ {{ bytesToThroughput(iface.rxBytesPerSec) }}</div>
            <div class="text-sky-400 font-semibold">↑ {{ bytesToThroughput(iface.txBytesPerSec) }}</div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
