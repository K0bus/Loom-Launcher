<script setup lang="ts">
import { onMounted, onUnmounted, computed } from 'vue'
import {
  Activity,
  Play,
  Pause,
  RotateCw,
  Clock,
  AlertCircle,
  LayoutDashboard,
  Tv,
  SlidersHorizontal,
  ShieldCheck,
  Package
} from 'lucide-vue-next'
import { useSystemMonitorStore } from '@/stores/useSystemMonitorStore'
import CpuGpuCard from '@/components/monitoring/CpuGpuCard.vue'
import MemoryStorageCard from '@/components/monitoring/MemoryStorageCard.vue'
import NetworkCard from '@/components/monitoring/NetworkCard.vue'
import ProcessTable from '@/components/monitoring/ProcessTable.vue'
import GpuProcessesView from '@/components/monitoring/GpuProcessesView.vue'
import AllProcessesView from '@/components/monitoring/AllProcessesView.vue'
import SmartDisksView from '@/components/monitoring/SmartDisksView.vue'
import DnfUpdatesView from '@/components/monitoring/DnfUpdatesView.vue'

const monitorStore = useSystemMonitorStore()

const refreshOptions = [
  { label: '500 ms', value: 500 },
  { label: '1 s', value: 1000 },
  { label: '2 s', value: 2000 },
  { label: '5 s', value: 5000 }
]

const navTabs = computed(() => [
  {
    id: 'overview' as const,
    label: "Vue d'ensemble",
    icon: LayoutDashboard,
    badge: null
  },
  {
    id: 'gpu-procs' as const,
    label: 'Processus GPU',
    icon: Tv,
    badge: monitorStore.gpuProcesses.length > 0 ? monitorStore.gpuProcesses.length : null
  },
  {
    id: 'all-procs' as const,
    label: 'Tous les Processus',
    icon: SlidersHorizontal,
    badge: monitorStore.allProcesses.length > 0 ? monitorStore.allProcesses.length : null
  },
  {
    id: 'smart' as const,
    label: 'Santé S.M.A.R.T.',
    icon: ShieldCheck,
    badge: monitorStore.smartDisks.length > 0 ? monitorStore.smartDisks.length : null
  },
  {
    id: 'dnf' as const,
    label: 'Mises à jour (DNF)',
    icon: Package,
    badge: monitorStore.dnfStatus?.upgradesCount ? monitorStore.dnfStatus.upgradesCount : null
  }
])

onMounted(() => {
  monitorStore.startPolling()
  // Pre-fetch tabs data
  monitorStore.fetchGpuProcesses()
  monitorStore.fetchSmartDisks()
  monitorStore.fetchDnfUpdates()
})

onUnmounted(() => {
  monitorStore.stopPolling()
})

function handleKillProcess(payload: { pid: number; force: boolean }) {
  monitorStore.killProcess(payload.pid, payload.force).catch((err) => {
    alert(`Erreur: ${err.message}`)
  })
}
</script>

<template>
  <div class="h-full flex flex-col bg-zinc-950 text-zinc-100 overflow-y-auto">
    <!-- Top System Banner & Monitoring Bar -->
    <header class="border-b border-zinc-800/80 bg-zinc-950/90 backdrop-blur-md sticky top-0 z-20">
      <div class="p-6 pb-4 space-y-4">
        <div class="flex flex-col lg:flex-row lg:items-center justify-between gap-4">
          <!-- Title & Host identity -->
          <div class="flex items-center gap-3.5">
            <div class="p-3 rounded-2xl bg-amber-500/10 border border-amber-500/20 text-amber-400 shadow-lg shadow-amber-500/5">
              <Activity class="w-7 h-7" />
            </div>
            <div>
              <div class="flex items-center gap-2.5">
                <h1 class="text-xl font-extrabold tracking-tight font-display text-zinc-100">
                  Monitoring Système & Matériel
                </h1>
                <span class="px-2 py-0.5 rounded-full text-[10px] font-mono font-bold bg-amber-500/10 text-amber-400 border border-amber-500/20 uppercase">
                  Temps Réel
                </span>
              </div>
              <div class="flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-zinc-400 font-medium mt-0.5">
                <span v-if="monitorStore.metrics?.os">
                  {{ monitorStore.metrics.os.osName }} {{ monitorStore.metrics.os.osVersion }}
                </span>
                <span v-if="monitorStore.metrics?.os" class="text-zinc-600">•</span>
                <span v-if="monitorStore.metrics?.os" class="font-mono text-zinc-300">
                  Kernel {{ monitorStore.metrics.os.kernelVersion }}
                </span>
                <span v-if="monitorStore.metrics?.os" class="text-zinc-600">•</span>
                <span v-if="monitorStore.metrics?.os" class="font-mono text-amber-400">
                  Session {{ monitorStore.metrics.os.displayServer }}
                </span>
              </div>
            </div>
          </div>

          <!-- Controls: Refresh Interval, Pause, Manual Refresh -->
          <div class="flex flex-wrap items-center gap-2.5">
            <!-- Refresh frequency dropdown/pills -->
            <div class="flex items-center bg-zinc-900 border border-zinc-800 p-1 rounded-xl text-xs font-mono">
              <button
                v-for="opt in refreshOptions"
                :key="opt.value"
                type="button"
                @click="monitorStore.setRefreshRate(opt.value)"
                :class="[
                  'px-2.5 py-1 rounded-lg transition-all font-semibold',
                  monitorStore.refreshInterval === opt.value
                    ? 'bg-amber-500 text-zinc-950 shadow-sm'
                    : 'text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800/60'
                ]"
              >
                {{ opt.label }}
              </button>
            </div>

            <!-- Pause / Resume Button -->
            <button
              type="button"
              @click="monitorStore.togglePause"
              :class="[
                'flex items-center gap-2 px-3.5 py-1.5 rounded-xl border text-xs font-semibold transition-all select-none',
                monitorStore.isPaused
                  ? 'bg-rose-500/10 border-rose-500/30 text-rose-400 hover:bg-rose-500/20'
                  : 'bg-zinc-900 border-zinc-800 text-zinc-300 hover:bg-zinc-800/80 hover:text-zinc-100'
              ]"
            >
              <component :is="monitorStore.isPaused ? Play : Pause" class="w-3.5 h-3.5" />
              <span>{{ monitorStore.isPaused ? 'Reprendre' : 'Figer' }}</span>
            </button>

            <!-- Refresh Now -->
            <button
              type="button"
              @click="monitorStore.fetchMetrics"
              :disabled="monitorStore.isRefreshing"
              class="p-2 rounded-xl bg-zinc-900 border border-zinc-800 text-zinc-400 hover:text-zinc-100 hover:bg-zinc-800 transition-colors disabled:opacity-50"
              title="Rafraîchir maintenant"
            >
              <RotateCw class="w-4 h-4" :class="{ 'animate-spin': monitorStore.isRefreshing }" />
            </button>
          </div>
        </div>

        <!-- Quick Metrics Ribbon: Uptime, Load Avg, Hostname -->
        <div v-if="monitorStore.metrics?.os" class="flex flex-wrap items-center justify-between gap-4 pt-2 text-xs font-mono text-zinc-400 border-t border-zinc-800/60">
          <div class="flex items-center gap-4">
            <div class="flex items-center gap-1.5">
              <Clock class="w-3.5 h-3.5 text-amber-400" />
              <span class="text-zinc-500">Uptime :</span>
              <span class="text-zinc-200 font-semibold">{{ monitorStore.metrics.os.uptimeFormatted }}</span>
            </div>

            <div class="hidden sm:flex items-center gap-1.5">
              <span class="text-zinc-500">Hôte :</span>
              <span class="text-zinc-200">{{ monitorStore.metrics.os.hostname }}</span>
            </div>
          </div>

          <div class="flex items-center gap-3">
            <span class="text-zinc-500">Charge Système :</span>
            <span class="text-zinc-300 font-bold">
              1m: <span class="text-amber-400">{{ monitorStore.metrics.os.loadAverage1m.toFixed(2) }}</span>
            </span>
            <span class="text-zinc-400">
              5m: {{ monitorStore.metrics.os.loadAverage5m.toFixed(2) }}
            </span>
            <span class="text-zinc-500">
              15m: {{ monitorStore.metrics.os.loadAverage15m.toFixed(2) }}
            </span>
          </div>
        </div>
      </div>

      <!-- Sub-navigation Centered Menu Bar with generous padding -->
      <nav class="border-t border-zinc-800/80 bg-zinc-950/95 py-4 px-6 flex items-center justify-center">
        <div class="inline-flex items-center justify-center p-1.5 rounded-2xl bg-zinc-900/90 border border-zinc-800/90 shadow-xl shadow-black/40 gap-1.5 overflow-x-auto max-w-full">
          <button
            v-for="tab in navTabs"
            :key="tab.id"
            type="button"
            @click="monitorStore.setTab(tab.id)"
            :class="[
              'flex items-center gap-2.5 px-5 py-2.5 rounded-xl text-xs font-semibold transition-all duration-200 whitespace-nowrap select-none group',
              monitorStore.currentTab === tab.id
                ? 'bg-amber-500 text-zinc-950 shadow-md shadow-amber-500/20 font-bold scale-[1.02]'
                : 'text-zinc-400 hover:text-zinc-100 hover:bg-zinc-800/60 border border-transparent'
            ]"
          >
            <component
              :is="tab.icon"
              :class="[
                'w-4 h-4 transition-colors',
                monitorStore.currentTab === tab.id ? 'text-zinc-950 stroke-[2.5]' : 'text-zinc-400 group-hover:text-zinc-200'
              ]"
            />
            <span>{{ tab.label }}</span>
            <span
              v-if="tab.badge != null"
              :class="[
                'text-[10px] font-mono px-2 py-0.5 rounded-full font-bold transition-colors',
                monitorStore.currentTab === tab.id
                  ? 'bg-zinc-950 text-amber-400 shadow-inner'
                  : 'bg-zinc-800 text-zinc-300'
              ]"
            >
              {{ tab.badge }}
            </span>
          </button>
        </div>
      </nav>
    </header>

    <!-- Main Content Body -->
    <main class="flex-1 p-8 space-y-8 max-w-7xl w-full mx-auto">
      <!-- Loading Skeleton State -->
      <div v-if="monitorStore.isLoading" class="py-24 flex flex-col items-center justify-center text-center space-y-4">
        <div class="p-4 rounded-2xl bg-amber-500/10 border border-amber-500/20 text-amber-400 animate-pulse">
          <Activity class="w-8 h-8" />
        </div>
        <p class="font-mono text-sm text-zinc-400">
          Interrogation des capteurs matériels et du noyau Linux...
        </p>
      </div>

      <!-- Error banner if any -->
      <div
        v-if="monitorStore.error"
        class="p-4 rounded-2xl bg-rose-500/10 border border-rose-500/20 text-rose-400 flex items-center gap-3"
      >
        <AlertCircle class="w-5 h-5 flex-shrink-0" />
        <div class="text-xs font-mono">
          <p class="font-bold">Avertissement de collecte :</p>
          <p>{{ monitorStore.error }}</p>
        </div>
      </div>

      <!-- TAB 1: VUE D'ENSEMBLE (Dashboard complet existant) -->
      <div v-if="!monitorStore.isLoading && monitorStore.currentTab === 'overview'" class="space-y-6">
        <section v-if="monitorStore.metrics">
          <CpuGpuCard
            :cpu="monitorStore.metrics.cpu"
            :gpu="monitorStore.metrics.gpu || null"
            :battery="monitorStore.metrics.battery || null"
            :cpu-history="monitorStore.history.cpuTotal"
            :gpu-history="monitorStore.history.gpu"
          />
        </section>

        <section v-if="monitorStore.metrics">
          <MemoryStorageCard
            :memory="monitorStore.metrics.memory"
            :disks="monitorStore.metrics.disks"
            :ram-history="monitorStore.history.ram"
            :disk-read-history="monitorStore.history.diskReadMb"
            :disk-write-history="monitorStore.history.diskWriteMb"
          />
        </section>

        <section v-if="monitorStore.metrics">
          <NetworkCard
            :network="monitorStore.metrics.network"
            :download-history="monitorStore.history.downloadMb"
            :upload-history="monitorStore.history.uploadMb"
          />
        </section>

        <section v-if="monitorStore.metrics">
          <ProcessTable
            :processes="monitorStore.filteredProcesses"
            :search-query="monitorStore.processFilter"
            :sort-field="monitorStore.processSortField"
            :sort-direction="monitorStore.processSortDirection"
            @update:search-query="monitorStore.setProcessFilter"
            @sort="monitorStore.setProcessSorting"
            @kill="handleKillProcess"
          />
        </section>
      </div>

      <!-- TAB 2: PROCESSUS GPU -->
      <div v-else-if="monitorStore.currentTab === 'gpu-procs'">
        <GpuProcessesView
          :processes="monitorStore.gpuProcesses"
          :is-loading="monitorStore.isGpuLoading"
          @refresh="monitorStore.fetchGpuProcesses"
          @kill="handleKillProcess"
        />
      </div>

      <!-- TAB 3: TOUS LES PROCESSUS -->
      <div v-else-if="monitorStore.currentTab === 'all-procs'">
        <AllProcessesView
          :processes="monitorStore.allProcesses"
          :is-loading="monitorStore.isAllProcsLoading"
          @refresh="monitorStore.fetchAllProcesses"
          @kill="handleKillProcess"
        />
      </div>

      <!-- TAB 4: SANTÉ S.M.A.R.T. -->
      <div v-else-if="monitorStore.currentTab === 'smart'">
        <SmartDisksView
          :disks="monitorStore.smartDisks"
          :is-loading="monitorStore.isSmartLoading"
          @refresh="monitorStore.fetchSmartDisks(false)"
          @scan-elevated="monitorStore.fetchSmartDisks(true)"
        />
      </div>

      <!-- TAB 5: MISES À JOUR SYSTÈME DNF -->
      <div v-else-if="monitorStore.currentTab === 'dnf'">
        <DnfUpdatesView
          :status="monitorStore.dnfStatus"
          :is-loading="monitorStore.isDnfLoading"
          @refresh="monitorStore.fetchDnfUpdates(true)"
        />
      </div>
    </main>
  </div>
</template>
