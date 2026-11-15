<script setup lang="ts">
import { computed } from 'vue'
import { useWowAddonsStore } from '@/stores/wowAddonsStore'
import { FolderOpen, RefreshCw, CheckCircle2, AlertCircle } from 'lucide-vue-next'
import UiBadge from '@/components/ui/UiBadge.vue'

const wowStore = useWowAddonsStore()

const instances = computed(() => wowStore.instances)
const selectedId = computed(() => wowStore.selectedInstanceId)
</script>

<template>
  <div class="bg-zinc-900/60 backdrop-blur-md border border-zinc-800/80 rounded-2xl p-4 sm:p-5 shadow-xl space-y-4">
    <div class="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-3">
      <div>
        <div class="flex items-center gap-2">
          <span class="text-xs font-mono font-semibold uppercase tracking-wider text-amber-400">
            Instances Détectées
          </span>
          <UiBadge variant="amber" size="xs" class="font-mono">
            Sans profil externe
          </UiBadge>
        </div>
        <p class="text-xs text-zinc-400 mt-0.5">
          Sélectionnez la version du jeu pour synchroniser et gérer directement ses addons
        </p>
      </div>

      <!-- Actions rapides dossier & refresh -->
      <div class="flex items-center gap-2 self-end sm:self-auto">
        <button
          type="button"
          @click="wowStore.loadInstalledAddons(true)"
          :disabled="wowStore.isLoadingInstalled || wowStore.isCheckingUpdates"
          class="flex items-center gap-1.5 px-3 py-1.5 rounded-xl bg-zinc-800/80 hover:bg-zinc-700/80 border border-zinc-700/60 text-xs font-mono text-zinc-300 hover:text-white transition disabled:opacity-50"
          title="Vérifier les mises à jour et recharger"
        >
          <RefreshCw
            class="w-3.5 h-3.5 text-amber-400"
            :class="{ 'animate-spin': wowStore.isLoadingInstalled || wowStore.isCheckingUpdates }"
          />
          <span>Vérifier MàJ</span>
        </button>

        <button
          type="button"
          @click="wowStore.openAddonsFolder()"
          class="flex items-center gap-1.5 px-3 py-1.5 rounded-xl bg-zinc-800/80 hover:bg-zinc-700/80 border border-zinc-700/60 text-xs font-mono text-zinc-300 hover:text-white transition"
          title="Ouvrir le dossier Interface/AddOns dans l'explorateur"
        >
          <FolderOpen class="w-3.5 h-3.5 text-amber-400" />
          <span>Ouvrir le dossier AddOns</span>
        </button>
      </div>
    </div>

    <!-- Sélecteur d'instances sous forme de cartes d'onglets -->
    <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-3">
      <button
        v-for="inst in instances"
        :key="inst.id"
        type="button"
        @click="wowStore.selectInstance(inst.id)"
        :class="[
          'relative text-left p-3.5 rounded-xl border transition-all flex flex-col justify-between gap-2.5 group',
          selectedId === inst.id
            ? 'bg-amber-500/10 border-amber-500/70 shadow-lg shadow-amber-500/10'
            : 'bg-zinc-950/60 border-zinc-800/80 hover:border-zinc-700 hover:bg-zinc-900/40'
        ]"
      >
        <!-- En-tête de la carte d'instance -->
        <div class="flex items-center justify-between gap-2">
          <div class="flex items-center gap-2 min-w-0">
            <img
              v-if="inst.icon"
              :src="inst.icon"
              :alt="inst.name"
              class="w-7 h-7 rounded-lg object-cover flex-shrink-0 border border-zinc-700/80"
            />
            <div
              v-else
              class="w-7 h-7 rounded-lg bg-zinc-800 border border-zinc-700 flex items-center justify-center text-xs font-black text-amber-400 flex-shrink-0"
            >
              WoW
            </div>
            <div class="truncate">
              <div class="font-bold text-xs text-zinc-200 truncate group-hover:text-amber-300 transition">
                {{ inst.name }}
              </div>
              <div class="text-[10px] font-mono text-zinc-500">
                v{{ inst.version }}
              </div>
            </div>
          </div>

          <!-- Indicateur d'installation -->
          <span
            v-if="inst.installed"
            class="flex items-center gap-1 text-[10px] font-mono font-medium text-emerald-400 bg-emerald-500/10 px-2 py-0.5 rounded-full border border-emerald-500/20 flex-shrink-0"
          >
            <CheckCircle2 class="w-2.5 h-2.5" />
            Installé
          </span>
          <span
            v-else
            class="flex items-center gap-1 text-[10px] font-mono text-zinc-500 bg-zinc-800/40 px-2 py-0.5 rounded-full flex-shrink-0"
          >
            <AlertCircle class="w-2.5 h-2.5" />
            Absent
          </span>
        </div>

        <!-- Pied de carte : nombre d'addons installés -->
        <div class="flex items-center justify-between pt-1 border-t border-zinc-800/60 text-[11px] font-mono">
          <span class="text-zinc-400">
            AddOns actifs :
          </span>
          <span :class="inst.addonsCount > 0 ? 'text-amber-400 font-bold' : 'text-zinc-500'">
            {{ inst.addonsCount }} {{ inst.addonsCount > 1 ? 'addons' : 'addon' }}
          </span>
        </div>

        <!-- Halo indicateur de sélection active -->
        <div
          v-if="selectedId === inst.id"
          class="absolute inset-x-0 -bottom-px h-[2px] bg-gradient-to-r from-transparent via-amber-400 to-transparent"
        />
      </button>
    </div>
  </div>
</template>
