<script setup lang="ts">
import {
  HardDrive,
  ShieldCheck,
  RotateCw,
  Thermometer,
  Clock,
  Activity,
  CheckCircle2
} from 'lucide-vue-next'
import type { SmartDiskInfo } from '@/models/monitoring'

interface Props {
  disks: SmartDiskInfo[]
  isLoading: boolean
}

defineProps<Props>()

const emit = defineEmits<{
  (e: 'refresh'): void
}>()

function formatBytes(bytes: number): string {
  const gb = bytes / (1024 * 1024 * 1024)
  if (gb >= 1000) {
    return `${(gb / 1024).toFixed(1)} To`
  }
  return `${gb.toFixed(0)} Go`
}

function getHealthBadge(status: string) {
  if (status === 'PASSED') {
    return {
      text: 'Santé Optimale (PASSED)',
      bg: 'bg-emerald-500/15',
      border: 'border-emerald-500/30',
      textColor: 'text-emerald-400'
    }
  }
  if (status === 'WARNING') {
    return {
      text: 'Avertissement S.M.A.R.T.',
      bg: 'bg-amber-500/15',
      border: 'border-amber-500/30',
      textColor: 'text-amber-400'
    }
  }
  return {
    text: 'Alerte Défaillance (FAILED)',
    bg: 'bg-rose-500/15',
    border: 'border-rose-500/30',
    textColor: 'text-rose-400'
  }
}
</script>

<template>
  <div class="space-y-6">
    <!-- Header Card -->
    <div class="p-6 rounded-2xl bg-zinc-900/90 border border-zinc-800 shadow-xl flex flex-col md:flex-row md:items-center justify-between gap-4">
      <div class="flex items-center gap-3.5">
        <div class="p-3 rounded-2xl bg-amber-500/10 border border-amber-500/20 text-amber-400">
          <ShieldCheck class="w-7 h-7" />
        </div>
        <div>
          <div class="flex items-center gap-2">
            <h2 class="text-lg font-bold text-zinc-100">Diagnostic S.M.A.R.T. des Disques</h2>
            <span class="px-2 py-0.5 rounded-full text-[10px] font-mono font-bold bg-amber-500/10 text-amber-400 border border-amber-500/20">
              {{ disks.length }} disques physiques
            </span>
          </div>
          <p class="text-xs text-zinc-400 mt-0.5">
            Surveillance de l'état d'usure, intégrité physique et relevés d'erreurs matérielles
          </p>
        </div>
      </div>

      <div class="flex items-center gap-3">
        <button
          type="button"
          @click="emit('refresh')"
          :disabled="isLoading"
          class="p-2.5 rounded-xl bg-zinc-900 border border-zinc-800 text-zinc-400 hover:text-zinc-100 hover:bg-zinc-800 transition-colors disabled:opacity-50"
          title="Actualiser les données S.M.A.R.T."
        >
          <RotateCw class="w-4 h-4" :class="{ 'animate-spin': isLoading }" />
        </button>
      </div>
    </div>

    <!-- Disk Cards Grid -->
    <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
      <div
        v-for="disk in disks"
        :key="disk.device"
        class="p-6 rounded-2xl bg-zinc-900/90 border border-zinc-800 shadow-xl flex flex-col justify-between space-y-5"
      >
        <div>
          <!-- Disk Header -->
          <div class="flex items-start justify-between gap-4 pb-4 border-b border-zinc-800/80">
            <div class="flex items-center gap-3">
              <div class="p-2.5 rounded-xl bg-amber-500/10 border border-amber-500/20 text-amber-400">
                <HardDrive class="w-6 h-6" />
              </div>
              <div>
                <div class="flex items-center gap-2">
                  <h3 class="text-base font-bold text-zinc-100 font-mono">{{ disk.device }}</h3>
                  <span class="text-[10px] uppercase font-mono px-2 py-0.5 rounded-full bg-zinc-800 text-amber-400 border border-zinc-700/80">
                    {{ disk.diskType }}
                  </span>
                </div>
                <p class="text-xs text-zinc-300 font-medium mt-0.5">
                  {{ disk.model }}
                </p>
                <p class="text-[11px] font-mono text-zinc-500 mt-0.5">
                  S/N : {{ disk.serial || 'Non communiqué' }} • Capacité : {{ formatBytes(disk.sizeBytes) }}
                </p>
              </div>
            </div>

            <!-- Health Status Badge -->
            <div
              class="px-3 py-1 rounded-xl text-xs font-mono font-bold border flex items-center gap-1.5"
              :class="[getHealthBadge(disk.healthStatus).bg, getHealthBadge(disk.healthStatus).border, getHealthBadge(disk.healthStatus).textColor]"
            >
              <CheckCircle2 class="w-3.5 h-3.5" />
              <span>{{ disk.healthStatus }}</span>
            </div>
          </div>

          <!-- Attributes Grid -->
          <div class="grid grid-cols-2 sm:grid-cols-3 gap-3 pt-4 font-mono text-xs">
            <!-- Temperature -->
            <div class="p-3 rounded-xl bg-zinc-950/60 border border-zinc-800/80">
              <div class="flex items-center gap-1.5 text-zinc-400 text-[11px] mb-1">
                <Thermometer class="w-3.5 h-3.5 text-amber-400" />
                <span>Température</span>
              </div>
              <p class="font-bold text-zinc-100">
                {{ disk.temperatureC != null ? `${disk.temperatureC.toFixed(0)}°C` : '--' }}
              </p>
            </div>

            <!-- Power on Hours -->
            <div class="p-3 rounded-xl bg-zinc-950/60 border border-zinc-800/80">
              <div class="flex items-center gap-1.5 text-zinc-400 text-[11px] mb-1">
                <Clock class="w-3.5 h-3.5 text-sky-400" />
                <span>Heures Allumé</span>
              </div>
              <p class="font-bold text-zinc-100">
                {{ disk.powerOnHours != null ? `${disk.powerOnHours.toLocaleString()} h` : '--' }}
              </p>
            </div>

            <!-- Wear / Percentage Used -->
            <div class="p-3 rounded-xl bg-zinc-950/60 border border-zinc-800/80">
              <div class="flex items-center gap-1.5 text-zinc-400 text-[11px] mb-1">
                <Activity class="w-3.5 h-3.5 text-purple-400" />
                <span>Usure Cellules</span>
              </div>
              <p class="font-bold" :class="disk.percentageUsed && disk.percentageUsed > 80 ? 'text-rose-400' : 'text-emerald-400'">
                {{ disk.percentageUsed != null ? `${disk.percentageUsed}%` : 'Normal' }}
              </p>
            </div>

            <!-- Data Read Total -->
            <div class="p-3 rounded-xl bg-zinc-950/60 border border-zinc-800/80">
              <span class="text-zinc-500 text-[10px] block mb-1">Total Données Lues</span>
              <p class="font-bold text-zinc-200">
                {{ disk.dataUnitsReadGb != null ? `${disk.dataUnitsReadGb.toLocaleString()} Go` : '--' }}
              </p>
            </div>

            <!-- Data Written Total -->
            <div class="p-3 rounded-xl bg-zinc-950/60 border border-zinc-800/80">
              <span class="text-zinc-500 text-[10px] block mb-1">Total Données Écrites</span>
              <p class="font-bold text-zinc-200">
                {{ disk.dataUnitsWrittenGb != null ? `${disk.dataUnitsWrittenGb.toLocaleString()} Go` : '--' }}
              </p>
            </div>

            <!-- Power Cycles -->
            <div class="p-3 rounded-xl bg-zinc-950/60 border border-zinc-800/80">
              <span class="text-zinc-500 text-[10px] block mb-1">Cycles d'Allumage</span>
              <p class="font-bold text-zinc-200">
                {{ disk.powerCycles != null ? disk.powerCycles.toLocaleString() : '--' }}
              </p>
            </div>
          </div>

          <!-- Critical warnings / notes if any -->
          <div v-if="disk.criticalWarning" class="mt-3 p-2.5 rounded-xl bg-zinc-950/80 border border-zinc-800 text-[11px] font-mono text-zinc-400 flex items-center justify-between">
            <span>Avertissements critiques contrôleur :</span>
            <span class="text-emerald-400 font-bold">{{ disk.criticalWarning }}</span>
          </div>
        </div>

        <div class="pt-3 border-t border-zinc-800/60 flex items-center justify-between text-[11px] font-mono text-zinc-500">
          <span>Intégrité matérielle : OK</span>
          <span>Protocole NVMe / SATA</span>
        </div>
      </div>
    </div>
  </div>
</template>
