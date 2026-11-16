<script setup lang="ts">
import { computed } from 'vue'
import {
  HardDrive,
  ShieldCheck,
  ShieldAlert,
  RotateCw,
  Thermometer,
  Clock,
  Activity,
  CheckCircle2,
  Info,
  Lock,
  Unlock
} from 'lucide-vue-next'
import type { SmartDiskInfo } from '@/models/monitoring'

interface Props {
  disks: SmartDiskInfo[]
  isLoading: boolean
}

const props = defineProps<Props>()

const emit = defineEmits<{
  (e: 'refresh'): void
  (e: 'scan-elevated'): void
}>()

const hasAnyAdminData = computed(() => {
  return props.disks.some((d) => d.hasAdminData === true)
})

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
          <ShieldCheck v-if="hasAnyAdminData" class="w-7 h-7 text-emerald-400" />
          <ShieldAlert v-else class="w-7 h-7 text-amber-400" />
        </div>
        <div>
          <div class="flex items-center gap-2">
            <h2 class="text-lg font-bold text-zinc-100">Diagnostic S.M.A.R.T. des Disques</h2>
            <span class="px-2 py-0.5 rounded-full text-[10px] font-mono font-bold bg-amber-500/10 text-amber-400 border border-amber-500/20">
              {{ disks.length }} disques physiques
            </span>
            <span
              v-if="hasAnyAdminData"
              class="px-2.5 py-0.5 rounded-full text-[10px] font-mono font-bold bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 flex items-center gap-1"
            >
              <Unlock class="w-3 h-3" />
              Droits Admin Accordés
            </span>
            <span
              v-else
              class="px-2.5 py-0.5 rounded-full text-[10px] font-mono font-bold bg-zinc-800 text-zinc-400 border border-zinc-700/60 flex items-center gap-1"
            >
              <Lock class="w-3 h-3 text-amber-400" />
              Accès Standard
            </span>
          </div>
          <p class="text-xs text-zinc-400 mt-0.5">
            Surveillance de l'état d'usure, intégrité physique et relevés d'erreurs matérielles
          </p>
        </div>
      </div>

      <div class="flex flex-wrap items-center gap-3">
        <!-- Bouton Élévation Admin SMART -->
        <button
          type="button"
          @click="emit('scan-elevated')"
          :disabled="isLoading"
          :class="[
            'flex items-center gap-2 px-4 py-2.5 rounded-xl text-xs font-semibold font-mono transition-all border shadow-sm select-none disabled:opacity-50',
            hasAnyAdminData
              ? 'bg-emerald-500/10 hover:bg-emerald-500/20 border-emerald-500/30 text-emerald-300'
              : 'bg-amber-500 hover:bg-amber-400 border-amber-400 text-zinc-950 font-bold shadow-amber-500/20'
          ]"
          :title="hasAnyAdminData ? 'Relancer l\'analyse avec les droits administrateur (pkexec smartctl)' : 'Demander les droits administrateur pour l\'analyse S.M.A.R.T. complète'"
        >
          <component :is="hasAnyAdminData ? ShieldCheck : ShieldAlert" class="w-4 h-4" />
          <span>{{ hasAnyAdminData ? 'Réanalyser (Admin)' : 'Autorisation Admin (smartctl)' }}</span>
        </button>

        <!-- Bouton Actualiser Standard -->
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

    <!-- Admin Permissions Banner (when not elevated yet) -->
    <div
      v-if="!hasAnyAdminData && disks.length > 0"
      class="p-4 rounded-2xl bg-amber-500/10 border border-amber-500/20 text-amber-300 text-xs flex items-start gap-3"
    >
      <Info class="w-5 h-5 flex-shrink-0 text-amber-400 mt-0.5" />
      <div class="space-y-1">
        <p class="font-bold text-amber-200">
          Mode standard actif (Lecture sysfs Linux)
        </p>
        <p class="text-zinc-300 leading-relaxed">
          Sous Linux, l'accès direct aux registres S.M.A.R.T. complets (<code class="font-mono bg-zinc-900/80 px-1 py-0.5 rounded text-amber-400">/dev/nvme*</code> et <code class="font-mono bg-zinc-900/80 px-1 py-0.5 rounded text-amber-400">/dev/sd*</code>) nécessite les privilèges superutilisateur.
          Cliquez sur <strong>« Autorisation Admin (smartctl) »</strong> pour déclencher la boîte de dialogue d'authentification native du système (Polkit) et obtenir l'historique complet d'usure, secteurs réalloués et journaux d'erreurs.
        </p>
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

            <!-- Reallocated Sectors (Admin / SMART) -->
            <div class="p-3 rounded-xl bg-zinc-950/60 border border-zinc-800/80">
              <span class="text-zinc-500 text-[10px] block mb-1">Secteurs Réalloués</span>
              <p
                class="font-bold"
                :class="disk.reallocatedSectors && disk.reallocatedSectors > 0 ? 'text-rose-400' : (disk.hasAdminData ? 'text-emerald-400' : 'text-zinc-400')"
              >
                {{ disk.reallocatedSectors != null ? disk.reallocatedSectors : (disk.hasAdminData ? '0' : '--') }}
              </p>
            </div>

            <!-- Media Errors (Admin / NVMe) -->
            <div class="p-3 rounded-xl bg-zinc-950/60 border border-zinc-800/80">
              <span class="text-zinc-500 text-[10px] block mb-1">Erreurs Médias</span>
              <p
                class="font-bold"
                :class="disk.mediaErrors && disk.mediaErrors > 0 ? 'text-rose-400' : (disk.hasAdminData ? 'text-emerald-400' : 'text-zinc-400')"
              >
                {{ disk.mediaErrors != null ? disk.mediaErrors : (disk.hasAdminData ? '0' : '--') }}
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
          <span :class="disk.hasAdminData ? 'text-emerald-400/90' : 'text-zinc-500'">
            {{ disk.hasAdminData ? '✓ Registres SMART complets (Root)' : 'Données sysfs standard' }}
          </span>
          <span>Protocole NVMe / ATA</span>
        </div>
      </div>
    </div>
  </div>
</template>
