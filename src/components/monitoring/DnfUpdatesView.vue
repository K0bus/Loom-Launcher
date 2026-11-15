<script setup lang="ts">
import { ref, computed } from 'vue'
import {
  Package,
  RotateCw,
  Search,
  X,
  AlertCircle,
  Copy,
  Terminal
} from 'lucide-vue-next'
import type { DnfUpdateStatus } from '@/models/monitoring'
import { useToastStore } from '@/stores/toastStore'

interface Props {
  status: DnfUpdateStatus | null
  isLoading: boolean
}

const props = defineProps<Props>()

const emit = defineEmits<{
  (e: 'refresh'): void
}>()

const toast = useToastStore()
const searchQuery = ref('')

const filteredPackages = computed(() => {
  if (!props.status?.packages) return []
  const q = searchQuery.value.trim().toLowerCase()
  if (!q) return props.status.packages
  return props.status.packages.filter(
    (p) =>
      p.name.toLowerCase().includes(q) ||
      p.repository.toLowerCase().includes(q) ||
      p.evr.toLowerCase().includes(q) ||
      p.arch.toLowerCase().includes(q)
  )
})

function copyUpgradeCommand() {
  navigator.clipboard.writeText('sudo dnf upgrade --refresh')
  toast.show('Commande copiée dans le presse-papier !', 'success')
}

function formatDate(timestamp: number): string {
  if (!timestamp) return '--'
  return new Date(timestamp).toLocaleTimeString('fr-FR', {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit'
  })
}
</script>

<template>
  <div class="space-y-6">
    <!-- Header Card -->
    <div class="p-6 rounded-2xl bg-zinc-900/90 border border-zinc-800 shadow-xl flex flex-col md:flex-row md:items-center justify-between gap-4">
      <div class="flex items-center gap-3.5">
        <div class="p-3 rounded-2xl bg-amber-500/10 border border-amber-500/20 text-amber-400">
          <Package class="w-7 h-7" />
        </div>
        <div>
          <div class="flex items-center gap-2">
            <h2 class="text-lg font-bold text-zinc-100">Mises à Jour Système Linux (DNF)</h2>
            <span
              v-if="status?.isAvailable"
              class="px-2 py-0.5 rounded-full text-[10px] font-mono font-bold uppercase border"
              :class="[
                status.upgradesCount > 0
                  ? 'bg-amber-500/15 text-amber-400 border-amber-500/30'
                  : 'bg-emerald-500/15 text-emerald-400 border-emerald-500/30'
              ]"
            >
              {{ status.upgradesCount > 0 ? `${status.upgradesCount} mises à jour` : 'Système à jour' }}
            </span>
          </div>
          <p class="text-xs text-zinc-400 mt-0.5">
            Surveillance et détection des paquets logiciels en attente de mise à niveau via DNF / DNF5
          </p>
        </div>
      </div>

      <div class="flex items-center gap-3">
        <div v-if="status?.lastCheckTimestamp" class="text-[11px] font-mono text-zinc-500">
          Dernier scan : {{ formatDate(status.lastCheckTimestamp) }}
        </div>

        <button
          type="button"
          @click="emit('refresh')"
          :disabled="isLoading"
          class="flex items-center gap-2 px-3.5 py-2 rounded-xl bg-zinc-900 border border-zinc-800 text-xs font-semibold text-zinc-300 hover:text-zinc-100 hover:bg-zinc-800 transition-colors disabled:opacity-50"
        >
          <RotateCw class="w-3.5 h-3.5" :class="{ 'animate-spin': isLoading }" />
          <span>Vérifier</span>
        </button>
      </div>
    </div>

    <!-- If DNF is NOT available -->
    <div
      v-if="status && !status.isAvailable"
      class="p-6 rounded-2xl bg-zinc-900/90 border border-zinc-800 text-center py-12 space-y-3"
    >
      <div class="p-3.5 rounded-2xl bg-zinc-800/60 border border-zinc-700/60 text-zinc-400 inline-block">
        <AlertCircle class="w-8 h-8 mx-auto" />
      </div>
      <h3 class="text-base font-bold text-zinc-200">Gestionnaire DNF non détecté</h3>
      <p class="text-xs text-zinc-400 max-w-md mx-auto">
        Le sous-système de vérification des paquets DNF est conçu pour les distributions de la famille Fedora, Red Hat Enterprise Linux, Rocky Linux et AlmaLinux.
      </p>
    </div>

    <!-- If DNF is available -->
    <div v-else class="space-y-4">
      <!-- Terminal Command Helper Card -->
      <div class="p-4 rounded-xl bg-zinc-950/80 border border-zinc-800/80 flex flex-col sm:flex-row sm:items-center justify-between gap-3">
        <div class="flex items-center gap-2.5 text-xs font-mono text-zinc-300">
          <Terminal class="w-4 h-4 text-amber-400" />
          <span>Commande pour appliquer les mises à jour :</span>
          <code class="px-2 py-0.5 rounded bg-zinc-900 border border-zinc-700 text-amber-400 font-bold">
            sudo dnf upgrade --refresh
          </code>
        </div>

        <button
          type="button"
          @click="copyUpgradeCommand"
          class="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-zinc-900 hover:bg-zinc-800 border border-zinc-700/80 text-xs text-zinc-200 transition-colors self-start sm:self-auto"
        >
          <Copy class="w-3.5 h-3.5 text-amber-400" />
          <span>Copier</span>
        </button>
      </div>

      <!-- Packages Table -->
      <div class="p-6 rounded-2xl bg-zinc-900/90 border border-zinc-800 shadow-xl space-y-4">
        <div class="flex items-center justify-between gap-4">
          <div class="relative w-full sm:w-80">
            <Search class="w-4 h-4 text-zinc-500 absolute left-3 top-1/2 -translate-y-1/2" />
            <input
              type="text"
              v-model="searchQuery"
              placeholder="Filtrer les paquets ou dépôts..."
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
            {{ filteredPackages.length }} paquets listés
          </span>
        </div>

        <!-- Table -->
        <div class="overflow-x-auto rounded-xl border border-zinc-800/80 bg-zinc-950/40">
          <table class="w-full text-left text-xs">
            <thead class="bg-zinc-950/80 text-zinc-400 font-mono uppercase text-[10px] tracking-wider border-b border-zinc-800 select-none">
              <tr>
                <th class="py-3 px-4">Paquet</th>
                <th class="py-3 px-4">Architecture</th>
                <th class="py-3 px-4">Nouvelle Version</th>
                <th class="py-3 px-4">Dépôt Source</th>
              </tr>
            </thead>

            <tbody class="divide-y divide-zinc-800/60 font-mono">
              <tr
                v-for="pkg in filteredPackages"
                :key="`${pkg.name}-${pkg.arch}`"
                class="hover:bg-zinc-900/60 transition-colors group"
              >
                <!-- Name -->
                <td class="py-2.5 px-4 font-bold text-zinc-200 group-hover:text-amber-400 transition-colors">
                  {{ pkg.name }}
                </td>

                <!-- Arch -->
                <td class="py-2.5 px-4 text-zinc-400 text-[11px]">
                  <span class="px-1.5 py-0.5 rounded bg-zinc-800 text-zinc-400 border border-zinc-700/60">
                    {{ pkg.arch }}
                  </span>
                </td>

                <!-- EVR -->
                <td class="py-2.5 px-4 font-bold text-amber-400">
                  {{ pkg.evr }}
                </td>

                <!-- Repo -->
                <td class="py-2.5 px-4 text-zinc-400 text-[11px]">
                  <span class="px-2 py-0.5 rounded-full bg-zinc-950 border border-zinc-800 text-zinc-300">
                    {{ pkg.repository }}
                  </span>
                </td>
              </tr>

              <tr v-if="filteredPackages.length === 0">
                <td colspan="4" class="py-12 text-center text-zinc-500 text-xs">
                  {{ isLoading ? 'Recherche des mises à jour DNF en cours...' : 'Tous les paquets du système sont à jour !' }}
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </div>
  </div>
</template>
