<script setup lang="ts">
import { ref, computed } from 'vue'
import {
  Tv,
  Search,
  X,
  RotateCw,
  Skull
} from 'lucide-vue-next'
import type { GpuProcessInfo } from '@/models/monitoring'
import UiModal from '@/components/ui/UiModal.vue'
import UiButton from '@/components/ui/UiButton.vue'
import { getMetricColorClass } from '@/stores/useSystemMonitorStore'

interface Props {
  processes: GpuProcessInfo[]
  isLoading: boolean
}

const props = defineProps<Props>()

const emit = defineEmits<{
  (e: 'refresh'): void
  (e: 'kill', payload: { pid: number; force: boolean }): void
}>()

const searchQuery = ref('')
const targetProcess = ref<GpuProcessInfo | null>(null)
const forceKill = ref(false)
const isKilling = ref(false)

const filtered = computed(() => {
  const q = searchQuery.value.trim().toLowerCase()
  if (!q) return props.processes
  return props.processes.filter(
    (p) =>
      p.name.toLowerCase().includes(q) ||
      p.command.toLowerCase().includes(q) ||
      p.pid.toString().includes(q) ||
      p.processType.toLowerCase().includes(q)
  )
})

const totalVramUsed = computed(() => {
  return props.processes.reduce((acc, p) => acc + (p.vramUsedMb || 0), 0)
})

function formatRam(bytes: number): string {
  const mb = bytes / (1024 * 1024)
  if (mb >= 1024) return `${(mb / 1024).toFixed(1)} Go`
  return `${mb.toFixed(0)} Mo`
}

function openKillModal(proc: GpuProcessInfo) {
  targetProcess.value = proc
  forceKill.value = false
}

function closeKillModal() {
  targetProcess.value = null
  forceKill.value = false
}

function confirmKill() {
  if (!targetProcess.value) return
  isKilling.value = true
  emit('kill', { pid: targetProcess.value.pid, force: forceKill.value })
  setTimeout(() => {
    isKilling.value = false
    closeKillModal()
    emit('refresh')
  }, 400)
}
</script>

<template>
  <div class="space-y-6">
    <!-- Header Summary Card -->
    <div class="p-6 rounded-2xl bg-zinc-900/90 border border-zinc-800 shadow-xl flex flex-col md:flex-row md:items-center justify-between gap-4">
      <div class="flex items-center gap-3.5">
        <div class="p-3 rounded-2xl bg-amber-500/10 border border-amber-500/20 text-amber-400">
          <Tv class="w-7 h-7" />
        </div>
        <div>
          <div class="flex items-center gap-2">
            <h2 class="text-lg font-bold text-zinc-100">Processus GPU Actifs</h2>
            <span class="px-2 py-0.5 rounded-full text-[10px] font-mono font-bold bg-amber-500/10 text-amber-400 border border-amber-500/20">
              {{ processes.length }} processus
            </span>
          </div>
          <p class="text-xs text-zinc-400 mt-0.5">
            Applications et contextes de calcul / graphismes alloués sur la carte graphique
          </p>
        </div>
      </div>

      <div class="flex items-center gap-3">
        <!-- VRAM Total Badge -->
        <div class="px-3.5 py-1.5 rounded-xl bg-zinc-950/80 border border-zinc-800 text-xs font-mono">
          <span class="text-zinc-500 mr-2">VRAM Totale Utilisée :</span>
          <span class="font-bold text-amber-400">{{ (totalVramUsed / 1024).toFixed(2) }} Go</span>
          <span class="text-zinc-500 ml-1">({{ totalVramUsed }} Mo)</span>
        </div>

        <!-- Refresh Button -->
        <button
          type="button"
          @click="emit('refresh')"
          :disabled="isLoading"
          class="p-2.5 rounded-xl bg-zinc-900 border border-zinc-800 text-zinc-400 hover:text-zinc-100 hover:bg-zinc-800 transition-colors disabled:opacity-50"
          title="Actualiser la télémétrie GPU"
        >
          <RotateCw class="w-4 h-4" :class="{ 'animate-spin': isLoading }" />
        </button>
      </div>
    </div>

    <!-- Process Table Card -->
    <div class="p-6 rounded-2xl bg-zinc-900/90 border border-zinc-800 shadow-xl space-y-4">
      <!-- Search Input -->
      <div class="flex items-center justify-between gap-4">
        <div class="relative w-full sm:w-80">
          <Search class="w-4 h-4 text-zinc-500 absolute left-3 top-1/2 -translate-y-1/2" />
          <input
            type="text"
            v-model="searchQuery"
            placeholder="Filtrer par nom, PID, commande..."
            class="w-full bg-zinc-950/80 border border-zinc-800 rounded-xl pl-9 pr-8 py-2 text-xs font-mono text-zinc-200 placeholder-zinc-500 focus:outline-none focus:border-amber-500/50 transition-colors"
          />
          <button
            v-if="searchQuery"
            @click="searchQuery = ''"
            class="absolute right-2.5 top-1/2 -translate-y-1/2 text-zinc-500 hover:text-zinc-300"
          >
            <X class="w-3.5 h-3.5" />
          </button>
        </div>

        <span class="text-xs font-mono text-zinc-500">
          {{ filtered.length }} / {{ processes.length }} affichés
        </span>
      </div>

      <!-- Table -->
      <div class="overflow-x-auto rounded-xl border border-zinc-800/80 bg-zinc-950/40">
        <table class="w-full text-left text-xs">
          <thead class="bg-zinc-950/80 text-zinc-400 font-mono uppercase text-[10px] tracking-wider border-b border-zinc-800 select-none">
            <tr>
              <th class="py-3 px-4">PID</th>
              <th class="py-3 px-4">Application / Commande</th>
              <th class="py-3 px-4">Type Contexte</th>
              <th class="py-3 px-4 text-right">VRAM Dédiée</th>
              <th class="py-3 px-4 text-right">SM Util (GPU)</th>
              <th class="py-3 px-4 text-right">RAM Hôte</th>
              <th class="py-3 px-4 text-right">% CPU</th>
              <th class="py-3 px-4 text-center">Action</th>
            </tr>
          </thead>

          <tbody class="divide-y divide-zinc-800/60 font-mono">
            <tr
              v-for="proc in filtered"
              :key="proc.pid"
              class="hover:bg-zinc-900/60 transition-colors group"
            >
              <!-- PID -->
              <td class="py-2.5 px-4 font-bold text-amber-400">
                {{ proc.pid }}
              </td>

              <!-- Name & Command -->
              <td class="py-2.5 px-4 max-w-sm">
                <div class="font-sans font-bold text-zinc-200 truncate group-hover:text-amber-400 transition-colors">
                  {{ proc.name }}
                </div>
                <div class="text-[10px] text-zinc-500 truncate" :title="proc.command">
                  {{ proc.command }}
                </div>
              </td>

              <!-- Type Badge -->
              <td class="py-2.5 px-4">
                <span
                  class="text-[10px] px-2 py-0.5 rounded-full font-mono font-medium border"
                  :class="[
                    proc.processType.includes('Calcul')
                      ? 'bg-purple-500/10 text-purple-400 border-purple-500/30'
                      : 'bg-sky-500/10 text-sky-400 border-sky-500/30'
                  ]"
                >
                  {{ proc.processType }}
                </span>
              </td>

              <!-- VRAM Used -->
              <td class="py-2.5 px-4 text-right font-extrabold text-amber-400">
                {{ proc.vramUsedMb != null ? `${proc.vramUsedMb} Mo` : '--' }}
              </td>

              <!-- SM Util % -->
              <td class="py-2.5 px-4 text-right text-zinc-300">
                <span v-if="proc.smUtilPercent != null" :class="getMetricColorClass(proc.smUtilPercent).text" class="font-bold">
                  {{ proc.smUtilPercent }}%
                </span>
                <span v-else class="text-zinc-600">-</span>
              </td>

              <!-- Host RAM -->
              <td class="py-2.5 px-4 text-right text-zinc-300">
                {{ formatRam(proc.ramMemoryBytes) }}
              </td>

              <!-- CPU % -->
              <td class="py-2.5 px-4 text-right font-bold" :class="getMetricColorClass(proc.cpuUsage).text">
                {{ proc.cpuUsage.toFixed(1) }}%
              </td>

              <!-- Kill Action -->
              <td class="py-2.5 px-4 text-center">
                <button
                  type="button"
                  @click="openKillModal(proc)"
                  class="p-1.5 rounded-lg text-zinc-500 hover:text-rose-400 hover:bg-rose-500/10 transition-colors"
                  title="Arrêter le processus"
                >
                  <Skull class="w-4 h-4" />
                </button>
              </td>
            </tr>

            <tr v-if="filtered.length === 0">
              <td colspan="8" class="py-12 text-center text-zinc-500 text-xs">
                {{ isLoading ? 'Recherche des processus GPU...' : 'Aucun processus GPU actif trouvé.' }}
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <!-- Modal Kill Confirmation -->
    <UiModal
      :open="!!targetProcess"
      title="Arrêter le processus GPU"
      subtitle="Confirmation d'interruption"
      max-width="md"
      @close="closeKillModal"
    >
      <div v-if="targetProcess" class="space-y-4">
        <p class="text-xs text-zinc-300">
          Êtes-vous sûr de vouloir interrompre <strong class="text-amber-400">{{ targetProcess.name }}</strong> (PID: {{ targetProcess.pid }}) ?
          La VRAM sera immédiatement libérée par le pilote GPU.
        </p>

        <label class="flex items-center gap-2.5 text-xs text-zinc-300 cursor-pointer select-none">
          <input
            type="checkbox"
            v-model="forceKill"
            class="rounded border-zinc-700 bg-zinc-950 text-amber-500 focus:ring-0"
          />
          <span>Forcer l'arrêt immédiat (SIGKILL / -9)</span>
        </label>

        <div class="flex items-center justify-end gap-2.5 pt-3 border-t border-zinc-800">
          <UiButton variant="ghost" size="sm" @click="closeKillModal">Annuler</UiButton>
          <UiButton variant="danger" size="sm" :loading="isKilling" @click="confirmKill">
            {{ forceKill ? 'Forcer (SIGKILL)' : 'Terminer' }}
          </UiButton>
        </div>
      </div>
    </UiModal>
  </div>
</template>
