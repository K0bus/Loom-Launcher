<script setup lang="ts">
import { ref } from 'vue'
import {
  Search,
  X,
  Skull,
  AlertTriangle,
  ArrowUpDown,
  ArrowUp,
  ArrowDown,
  Activity
} from 'lucide-vue-next'
import type { ProcessSnapshot } from '@/models/monitoring'
import UiModal from '@/components/ui/UiModal.vue'
import UiButton from '@/components/ui/UiButton.vue'
import { getMetricColorClass } from '@/stores/useSystemMonitorStore'

interface Props {
  processes: ProcessSnapshot[]
  searchQuery: string
  sortField: 'cpu' | 'memory' | 'name' | 'pid'
  sortDirection: 'asc' | 'desc'
}

defineProps<Props>()

const emit = defineEmits<{
  (e: 'update:searchQuery', val: string): void
  (e: 'sort', field: 'cpu' | 'memory' | 'name' | 'pid'): void
  (e: 'kill', payload: { pid: number; force: boolean }): void
}>()

// Modal State
const targetProcess = ref<ProcessSnapshot | null>(null)
const forceKill = ref(false)
const isKilling = ref(false)
const modalError = ref<string | null>(null)

function openKillModal(proc: ProcessSnapshot) {
  targetProcess.value = proc
  forceKill.value = false
  modalError.value = null
}

function closeKillModal() {
  targetProcess.value = null
  forceKill.value = false
  modalError.value = null
}

async function confirmKill() {
  if (!targetProcess.value) return
  isKilling.value = true
  modalError.value = null
  try {
    emit('kill', { pid: targetProcess.value.pid, force: forceKill.value })
    closeKillModal()
  } catch (err: any) {
    modalError.value = err?.message || String(err)
  } finally {
    isKilling.value = false
  }
}

function formatMemory(bytes: number): string {
  const mb = bytes / (1024 * 1024)
  if (mb >= 1024) {
    return `${(mb / 1024).toFixed(1)} Go`
  }
  return `${mb.toFixed(0)} Mo`
}

function formatDiskIo(readBytes: number, writeBytes: number): string {
  const total = readBytes + writeBytes
  if (total === 0) return '-'
  const mb = total / (1024 * 1024)
  if (mb >= 1) return `${mb.toFixed(1)} Mo/s`
  const kb = total / 1024
  return `${kb.toFixed(0)} Ko/s`
}

function isCriticalProcess(proc: ProcessSnapshot): boolean {
  return proc.pid <= 1 || proc.name.toLowerCase().includes('loom')
}
</script>

<template>
  <div class="p-6 rounded-2xl bg-zinc-900/90 border border-zinc-800 shadow-xl space-y-4">
    <!-- Table Header & Search Filter Bar -->
    <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-3 pb-3 border-b border-zinc-800/80">
      <div class="flex items-center gap-3">
        <div class="p-2 rounded-xl bg-amber-500/10 border border-amber-500/20 text-amber-400">
          <Activity class="w-5 h-5" />
        </div>
        <div>
          <h3 class="text-base font-bold text-zinc-100">Gestionnaire de Processus</h3>
          <p class="text-xs text-zinc-400 font-medium">
            {{ processes.length }} processus actifs les plus consommateurs
          </p>
        </div>
      </div>

      <!-- Search Input -->
      <div class="relative w-full sm:w-72">
        <Search class="w-4 h-4 text-zinc-500 absolute left-3 top-1/2 -translate-y-1/2" />
        <input
          type="text"
          :value="searchQuery"
          @input="emit('update:searchQuery', ($event.target as HTMLInputElement).value)"
          placeholder="Rechercher par PID, nom ou utilisateur..."
          class="w-full bg-zinc-950/80 border border-zinc-800 rounded-xl pl-9 pr-8 py-2 text-xs font-mono text-zinc-200 placeholder-zinc-500 focus:outline-none focus:border-amber-500/50 transition-colors"
        />
        <button
          v-if="searchQuery"
          @click="emit('update:searchQuery', '')"
          class="absolute right-2.5 top-1/2 -translate-y-1/2 text-zinc-500 hover:text-zinc-300"
        >
          <X class="w-3.5 h-3.5" />
        </button>
      </div>
    </div>

    <!-- Process Table -->
    <div class="overflow-x-auto rounded-xl border border-zinc-800/80 bg-zinc-950/40">
      <table class="w-full text-left text-xs">
        <thead class="bg-zinc-950/80 text-zinc-400 font-mono uppercase text-[10px] tracking-wider border-b border-zinc-800 select-none">
          <tr>
            <th
              class="py-3 px-4 cursor-pointer hover:text-zinc-200 transition-colors"
              @click="emit('sort', 'pid')"
            >
              <div class="flex items-center gap-1.5">
                <span>PID</span>
                <component
                  :is="sortField === 'pid' ? (sortDirection === 'asc' ? ArrowUp : ArrowDown) : ArrowUpDown"
                  class="w-3 h-3 text-zinc-500"
                />
              </div>
            </th>

            <th
              class="py-3 px-4 cursor-pointer hover:text-zinc-200 transition-colors"
              @click="emit('sort', 'name')"
            >
              <div class="flex items-center gap-1.5">
                <span>Processus / Commande</span>
                <component
                  :is="sortField === 'name' ? (sortDirection === 'asc' ? ArrowUp : ArrowDown) : ArrowUpDown"
                  class="w-3 h-3 text-zinc-500"
                />
              </div>
            </th>

            <th class="py-3 px-4">Utilisateur</th>

            <th
              class="py-3 px-4 cursor-pointer hover:text-zinc-200 transition-colors text-right"
              @click="emit('sort', 'cpu')"
            >
              <div class="flex items-center justify-end gap-1.5">
                <span>% CPU</span>
                <component
                  :is="sortField === 'cpu' ? (sortDirection === 'asc' ? ArrowUp : ArrowDown) : ArrowUpDown"
                  class="w-3 h-3 text-zinc-500"
                />
              </div>
            </th>

            <th
              class="py-3 px-4 cursor-pointer hover:text-zinc-200 transition-colors text-right"
              @click="emit('sort', 'memory')"
            >
              <div class="flex items-center justify-end gap-1.5">
                <span>Mémoire</span>
                <component
                  :is="sortField === 'memory' ? (sortDirection === 'asc' ? ArrowUp : ArrowDown) : ArrowUpDown"
                  class="w-3 h-3 text-zinc-500"
                />
              </div>
            </th>

            <th class="py-3 px-4 text-right">I/O Disque</th>

            <th class="py-3 px-4 text-center">Action</th>
          </tr>
        </thead>

        <tbody class="divide-y divide-zinc-800/60 font-mono">
          <tr
            v-for="proc in processes"
            :key="proc.pid"
            class="hover:bg-zinc-900/60 transition-colors group"
          >
            <!-- PID -->
            <td class="py-2.5 px-4 font-bold text-zinc-400">
              {{ proc.pid }}
            </td>

            <!-- Name & Command -->
            <td class="py-2.5 px-4 max-w-xs sm:max-w-md">
              <div class="font-sans font-bold text-zinc-200 truncate group-hover:text-amber-400 transition-colors">
                {{ proc.name }}
              </div>
              <div class="text-[10px] text-zinc-500 truncate font-mono" :title="proc.command">
                {{ proc.command }}
              </div>
            </td>

            <!-- User -->
            <td class="py-2.5 px-4 text-zinc-400 text-[11px]">
              {{ proc.user || 'root' }}
            </td>

            <!-- CPU -->
            <td class="py-2.5 px-4 text-right font-bold" :class="getMetricColorClass(proc.cpuUsage).text">
              {{ proc.cpuUsage.toFixed(1) }}%
            </td>

            <!-- Memory -->
            <td class="py-2.5 px-4 text-right text-zinc-300 font-semibold">
              {{ formatMemory(proc.memoryBytes) }}
            </td>

            <!-- Disk I/O -->
            <td class="py-2.5 px-4 text-right text-zinc-400 text-[11px]">
              {{ formatDiskIo(proc.diskReadBytesPerSec, proc.diskWriteBytesPerSec) }}
            </td>

            <!-- Action -->
            <td class="py-2.5 px-4 text-center">
              <button
                v-if="!isCriticalProcess(proc)"
                type="button"
                @click="openKillModal(proc)"
                class="p-1.5 rounded-lg text-zinc-500 hover:text-rose-400 hover:bg-rose-500/10 transition-colors"
                title="Terminer le processus"
              >
                <Skull class="w-4 h-4" />
              </button>
              <span
                v-else
                class="text-[10px] text-zinc-600 cursor-not-allowed"
                title="Processus critique protégé"
              >
                Protégé
              </span>
            </td>
          </tr>

          <tr v-if="processes.length === 0">
            <td colspan="7" class="py-8 text-center text-zinc-500 text-xs">
              Aucun processus ne correspond aux critères de recherche.
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- Kill Process Confirmation Modal -->
    <UiModal
      :open="!!targetProcess"
      title="Arrêter le processus"
      subtitle="Confirmation d'interruption système"
      max-width="md"
      @close="closeKillModal"
    >
      <div v-if="targetProcess" class="space-y-4">
        <div class="p-3.5 rounded-xl bg-rose-500/10 border border-rose-500/20 text-rose-400 flex items-start gap-3">
          <AlertTriangle class="w-5 h-5 flex-shrink-0 mt-0.5" />
          <div class="text-xs">
            <p class="font-bold">Attention lors de l'arrêt forcé</p>
            <p class="text-rose-300/80 mt-0.5">
              L'interruption brutale d'un processus peut entraîner des pertes de données non sauvegardées pour ce programme.
            </p>
          </div>
        </div>

        <!-- Target details -->
        <div class="p-3 rounded-xl bg-zinc-900 border border-zinc-800 text-xs font-mono space-y-1.5">
          <div class="flex justify-between text-zinc-400">
            <span>Processus :</span>
            <span class="font-bold text-zinc-200">{{ targetProcess.name }}</span>
          </div>
          <div class="flex justify-between text-zinc-400">
            <span>PID :</span>
            <span class="text-amber-400">{{ targetProcess.pid }}</span>
          </div>
          <div class="flex justify-between text-zinc-400">
            <span>Mémoire :</span>
            <span class="text-zinc-200">{{ formatMemory(targetProcess.memoryBytes) }}</span>
          </div>
          <div class="flex justify-between text-zinc-400">
            <span>Charge CPU :</span>
            <span class="text-zinc-200">{{ targetProcess.cpuUsage.toFixed(1) }}%</span>
          </div>
        </div>

        <!-- Force Toggle checkbox -->
        <label class="flex items-center gap-2.5 text-xs text-zinc-300 cursor-pointer select-none">
          <input
            type="checkbox"
            v-model="forceKill"
            class="rounded border-zinc-700 bg-zinc-950 text-amber-500 focus:ring-0 focus:ring-offset-0"
          />
          <span>Forcer l'arrêt immédiat (Signal SIGKILL / -9)</span>
        </label>

        <!-- Error display if any -->
        <div v-if="modalError" class="text-xs text-rose-400 font-mono">
          {{ modalError }}
        </div>

        <!-- Action buttons -->
        <div class="flex items-center justify-end gap-2.5 pt-3 border-t border-zinc-800">
          <UiButton variant="ghost" size="sm" @click="closeKillModal">
            Annuler
          </UiButton>
          <UiButton
            variant="danger"
            size="sm"
            :loading="isKilling"
            @click="confirmKill"
          >
            {{ forceKill ? 'Forcer l\'arrêt (SIGKILL)' : 'Terminer (SIGTERM)' }}
          </UiButton>
        </div>
      </div>
    </UiModal>
  </div>
</template>
