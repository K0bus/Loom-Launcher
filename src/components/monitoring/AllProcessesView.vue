<script setup lang="ts">
import { ref, computed } from 'vue'
import {
  SlidersHorizontal,
  Search,
  X,
  RotateCw,
  Skull,
  ArrowUpDown,
  ArrowUp,
  ArrowDown,
  ChevronLeft,
  ChevronRight
} from 'lucide-vue-next'
import type { ProcessSnapshot } from '@/models/monitoring'
import UiModal from '@/components/ui/UiModal.vue'
import UiButton from '@/components/ui/UiButton.vue'
import { getMetricColorClass } from '@/stores/useSystemMonitorStore'

interface Props {
  processes: ProcessSnapshot[]
  isLoading: boolean
}

const props = defineProps<Props>()

const emit = defineEmits<{
  (e: 'refresh'): void
  (e: 'kill', payload: { pid: number; force: boolean }): void
}>()

const searchQuery = ref('')
const sortField = ref<'cpu' | 'memory' | 'name' | 'pid' | 'user'>('cpu')
const sortDirection = ref<'asc' | 'desc'>('desc')
const currentPage = ref(1)
const pageSize = ref(50)

const targetProcess = ref<ProcessSnapshot | null>(null)
const forceKill = ref(false)
const isKilling = ref(false)

function handleSort(field: 'cpu' | 'memory' | 'name' | 'pid' | 'user') {
  if (sortField.value === field) {
    sortDirection.value = sortDirection.value === 'asc' ? 'desc' : 'asc'
  } else {
    sortField.value = field
    sortDirection.value = 'desc'
  }
}

const sortedAndFiltered = computed(() => {
  let list = props.processes
  const q = searchQuery.value.trim().toLowerCase()
  if (q) {
    list = list.filter(
      (p) =>
        p.name.toLowerCase().includes(q) ||
        p.command.toLowerCase().includes(q) ||
        p.pid.toString().includes(q) ||
        (p.user && p.user.toLowerCase().includes(q))
    )
  }

  const dir = sortDirection.value === 'asc' ? 1 : -1
  const field = sortField.value

  return [...list].sort((a, b) => {
    if (field === 'cpu') return (a.cpuUsage - b.cpuUsage) * dir
    if (field === 'memory') return (a.memoryBytes - b.memoryBytes) * dir
    if (field === 'pid') return (a.pid - b.pid) * dir
    if (field === 'user') return (a.user || '').localeCompare(b.user || '') * dir
    return a.name.localeCompare(b.name) * dir
  })
})

const totalPages = computed(() => Math.ceil(sortedAndFiltered.value.length / pageSize.value) || 1)

const paginatedProcesses = computed(() => {
  const start = (currentPage.value - 1) * pageSize.value
  return sortedAndFiltered.value.slice(start, start + pageSize.value)
})

function formatMemory(bytes: number): string {
  const mb = bytes / (1024 * 1024)
  if (mb >= 1024) return `${(mb / 1024).toFixed(1)} Go`
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

function openKillModal(proc: ProcessSnapshot) {
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
    <!-- Header -->
    <div class="p-6 rounded-2xl bg-zinc-900/90 border border-zinc-800 shadow-xl flex flex-col md:flex-row md:items-center justify-between gap-4">
      <div class="flex items-center gap-3.5">
        <div class="p-3 rounded-2xl bg-amber-500/10 border border-amber-500/20 text-amber-400">
          <SlidersHorizontal class="w-7 h-7" />
        </div>
        <div>
          <div class="flex items-center gap-2">
            <h2 class="text-lg font-bold text-zinc-100">Liste Complète des Processus</h2>
            <span class="px-2 py-0.5 rounded-full text-[10px] font-mono font-bold bg-amber-500/10 text-amber-400 border border-amber-500/20">
              {{ processes.length }} processus système
            </span>
          </div>
          <p class="text-xs text-zinc-400 mt-0.5">
            Supervision exhaustive de tous les threads et processus de la session Linux
          </p>
        </div>
      </div>

      <div class="flex items-center gap-3">
        <button
          type="button"
          @click="emit('refresh')"
          :disabled="isLoading"
          class="p-2.5 rounded-xl bg-zinc-900 border border-zinc-800 text-zinc-400 hover:text-zinc-100 hover:bg-zinc-800 transition-colors disabled:opacity-50"
          title="Actualiser la liste"
        >
          <RotateCw class="w-4 h-4" :class="{ 'animate-spin': isLoading }" />
        </button>
      </div>
    </div>

    <!-- Table Card -->
    <div class="p-6 rounded-2xl bg-zinc-900/90 border border-zinc-800 shadow-xl space-y-4">
      <!-- Search & Pagination Controls -->
      <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div class="relative w-full sm:w-80">
          <Search class="w-4 h-4 text-zinc-500 absolute left-3 top-1/2 -translate-y-1/2" />
          <input
            type="text"
            v-model="searchQuery"
            placeholder="Rechercher par PID, nom, utilisateur..."
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

        <!-- Pagination -->
        <div class="flex items-center gap-3 text-xs font-mono text-zinc-400">
          <span>Page {{ currentPage }} sur {{ totalPages }}</span>
          <div class="flex items-center gap-1">
            <button
              type="button"
              :disabled="currentPage <= 1"
              @click="currentPage--"
              class="p-1.5 rounded-lg border border-zinc-800 bg-zinc-950 hover:bg-zinc-800 disabled:opacity-40 transition-colors"
            >
              <ChevronLeft class="w-3.5 h-3.5" />
            </button>
            <button
              type="button"
              :disabled="currentPage >= totalPages"
              @click="currentPage++"
              class="p-1.5 rounded-lg border border-zinc-800 bg-zinc-950 hover:bg-zinc-800 disabled:opacity-40 transition-colors"
            >
              <ChevronRight class="w-3.5 h-3.5" />
            </button>
          </div>
        </div>
      </div>

      <!-- Full Table -->
      <div class="overflow-x-auto rounded-xl border border-zinc-800/80 bg-zinc-950/40">
        <table class="w-full text-left text-xs">
          <thead class="bg-zinc-950/80 text-zinc-400 font-mono uppercase text-[10px] tracking-wider border-b border-zinc-800 select-none">
            <tr>
              <th class="py-3 px-4 cursor-pointer hover:text-zinc-200" @click="handleSort('pid')">
                <div class="flex items-center gap-1.5">
                  <span>PID</span>
                  <component
                    :is="sortField === 'pid' ? (sortDirection === 'asc' ? ArrowUp : ArrowDown) : ArrowUpDown"
                    class="w-3 h-3 text-zinc-500"
                  />
                </div>
              </th>

              <th class="py-3 px-4 cursor-pointer hover:text-zinc-200" @click="handleSort('name')">
                <div class="flex items-center gap-1.5">
                  <span>Processus / Exécutable</span>
                  <component
                    :is="sortField === 'name' ? (sortDirection === 'asc' ? ArrowUp : ArrowDown) : ArrowUpDown"
                    class="w-3 h-3 text-zinc-500"
                  />
                </div>
              </th>

              <th class="py-3 px-4 cursor-pointer hover:text-zinc-200" @click="handleSort('user')">
                <div class="flex items-center gap-1.5">
                  <span>Utilisateur</span>
                  <component
                    :is="sortField === 'user' ? (sortDirection === 'asc' ? ArrowUp : ArrowDown) : ArrowUpDown"
                    class="w-3 h-3 text-zinc-500"
                  />
                </div>
              </th>

              <th class="py-3 px-4 cursor-pointer hover:text-zinc-200 text-right" @click="handleSort('cpu')">
                <div class="flex items-center justify-end gap-1.5">
                  <span>% CPU</span>
                  <component
                    :is="sortField === 'cpu' ? (sortDirection === 'asc' ? ArrowUp : ArrowDown) : ArrowUpDown"
                    class="w-3 h-3 text-zinc-500"
                  />
                </div>
              </th>

              <th class="py-3 px-4 cursor-pointer hover:text-zinc-200 text-right" @click="handleSort('memory')">
                <div class="flex items-center justify-end gap-1.5">
                  <span>Mémoire</span>
                  <component
                    :is="sortField === 'memory' ? (sortDirection === 'asc' ? ArrowUp : ArrowDown) : ArrowUpDown"
                    class="w-3 h-3 text-zinc-500"
                  />
                </div>
              </th>

              <th class="py-3 px-4 text-right">I/O Disque</th>
              <th class="py-3 px-4 text-center">Statut</th>
              <th class="py-3 px-4 text-center">Action</th>
            </tr>
          </thead>

          <tbody class="divide-y divide-zinc-800/60 font-mono">
            <tr
              v-for="proc in paginatedProcesses"
              :key="proc.pid"
              class="hover:bg-zinc-900/60 transition-colors group"
            >
              <td class="py-2.5 px-4 font-bold text-zinc-400">{{ proc.pid }}</td>

              <td class="py-2.5 px-4 max-w-xs sm:max-w-md">
                <div class="font-sans font-bold text-zinc-200 truncate group-hover:text-amber-400 transition-colors">
                  {{ proc.name }}
                </div>
                <div class="text-[10px] text-zinc-500 truncate" :title="proc.command">
                  {{ proc.command }}
                </div>
              </td>

              <td class="py-2.5 px-4 text-zinc-400 text-[11px]">{{ proc.user || 'root' }}</td>

              <td class="py-2.5 px-4 text-right font-bold" :class="getMetricColorClass(proc.cpuUsage).text">
                {{ proc.cpuUsage.toFixed(1) }}%
              </td>

              <td class="py-2.5 px-4 text-right text-zinc-300 font-semibold">
                {{ formatMemory(proc.memoryBytes) }}
              </td>

              <td class="py-2.5 px-4 text-right text-zinc-400 text-[11px]">
                {{ formatDiskIo(proc.diskReadBytesPerSec, proc.diskWriteBytesPerSec) }}
              </td>

              <td class="py-2.5 px-4 text-center">
                <span class="text-[10px] px-1.5 py-0.5 rounded bg-zinc-800 text-zinc-400 border border-zinc-700/60">
                  {{ proc.status }}
                </span>
              </td>

              <td class="py-2.5 px-4 text-center">
                <button
                  v-if="proc.pid > 1 && !proc.name.toLowerCase().includes('loom')"
                  type="button"
                  @click="openKillModal(proc)"
                  class="p-1.5 rounded-lg text-zinc-500 hover:text-rose-400 hover:bg-rose-500/10 transition-colors"
                  title="Terminer le processus"
                >
                  <Skull class="w-4 h-4" />
                </button>
                <span v-else class="text-[10px] text-zinc-600">Protégé</span>
              </td>
            </tr>

            <tr v-if="paginatedProcesses.length === 0">
              <td colspan="8" class="py-12 text-center text-zinc-500 text-xs">
                {{ isLoading ? 'Collecte des processus système...' : 'Aucun processus ne correspond aux critères.' }}
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <!-- Kill Modal -->
    <UiModal
      :open="!!targetProcess"
      title="Arrêter le processus"
      subtitle="Confirmation d'interruption système"
      max-width="md"
      @close="closeKillModal"
    >
      <div v-if="targetProcess" class="space-y-4">
        <p class="text-xs text-zinc-300">
          Voulez-vous terminer <strong class="text-amber-400">{{ targetProcess.name }}</strong> (PID: {{ targetProcess.pid }}) ?
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
