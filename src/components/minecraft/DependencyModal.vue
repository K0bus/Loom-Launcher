<script setup lang="ts">
import { computed } from 'vue'
import { AlertCircle, Download, CheckCircle2, ShieldAlert, Layers } from 'lucide-vue-next'
import { useMinecraftStore } from '@/stores/minecraft'
import UiModal from '@/components/ui/UiModal.vue'
import UiButton from '@/components/ui/UiButton.vue'
import UiBadge from '@/components/ui/UiBadge.vue'

const minecraftStore = useMinecraftStore()

const open = computed(() => minecraftStore.dependencyModalOpen)
const parentMod = computed(() => minecraftStore.pendingModToInstall)
const dependencies = computed(() => minecraftStore.detectedDependencies)
const activeInst = computed(() => minecraftStore.activeInstance)
const isBatchInstalling = computed(() => minecraftStore.isBatchInstalling)

function isDepSelected(dep: any) {
  const key = dep.id || dep.slug
  return minecraftStore.selectedDependencies.includes(key)
}

function toggleDep(dep: any) {
  const key = dep.id || dep.slug
  const idx = minecraftStore.selectedDependencies.indexOf(key)
  if (idx !== -1) {
    minecraftStore.selectedDependencies.splice(idx, 1)
  } else {
    minecraftStore.selectedDependencies.push(key)
  }
}

function selectAll() {
  minecraftStore.selectedDependencies = dependencies.value.map((d: any) => d.id || d.slug)
}
</script>

<template>
  <UiModal
    :open="open"
    title="Résolution Interactive des Dépendances"
    subtitle="Des composants obligatoires sont requis pour garantir la compatibilité et la stabilité du jeu"
    max-width="lg"
    @close="minecraftStore.closeDependencyModal"
  >
    <div class="space-y-5 py-2">
      <!-- Target Instance & Version Info -->
      <div
        v-if="activeInst"
        class="flex items-center justify-between text-xs font-mono text-zinc-400 bg-zinc-900/80 px-4 py-2.5 rounded-xl border border-zinc-800"
      >
        <span class="flex items-center gap-2">
          <Layers class="w-4 h-4 text-amber-400" />
          <span>Instance : <strong class="text-zinc-200">{{ activeInst.name }}</strong></span>
        </span>
        <span class="flex items-center gap-2">
          <span class="text-amber-400 font-bold">MC {{ activeInst.gameVersion }}</span>
          <span>•</span>
          <span class="text-zinc-300 capitalize">{{ activeInst.loader }}</span>
        </span>
      </div>

      <!-- Target Mod Highlight -->
      <div v-if="parentMod" class="p-4 rounded-2xl bg-zinc-900 border border-amber-500/30 flex items-center gap-4">
        <div class="w-12 h-12 rounded-xl bg-zinc-950 border border-zinc-800 flex items-center justify-center flex-shrink-0 overflow-hidden">
          <img
            v-if="parentMod.icon_url"
            :src="parentMod.icon_url"
            :alt="parentMod.name"
            class="w-full h-full object-cover"
          />
          <span v-else class="text-xl">📦</span>
        </div>
        <div class="min-w-0 flex-1">
          <div class="flex items-center gap-2">
            <span class="font-bold text-sm text-zinc-100 truncate">{{ parentMod.name }}</span>
            <UiBadge :variant="parentMod.provider === 'modrinth' ? 'emerald' : 'amber'" size="xs">
              {{ parentMod.provider.toUpperCase() }}
            </UiBadge>
          </div>
          <p class="text-xs text-zinc-400 truncate mt-0.5">{{ parentMod.summary }}</p>
        </div>
      </div>

      <!-- Warning info banner -->
      <div class="p-3.5 rounded-xl bg-amber-500/10 border border-amber-500/20 flex items-start gap-3 text-xs text-amber-300">
        <AlertCircle class="w-4 h-4 text-amber-400 flex-shrink-0 mt-0.5" />
        <div class="leading-relaxed">
          Ce mod nécessite les dépendances suivantes pour fonctionner sans crash sur Minecraft
          <strong>{{ activeInst?.gameVersion }}</strong> avec <strong>{{ activeInst?.loader }}</strong>.
          Elles seront automatiquement téléchargées et liées à votre instance.
        </div>
      </div>

      <!-- Dependencies list -->
      <div class="space-y-2">
        <div class="flex items-center justify-between text-xs font-mono text-zinc-400">
          <span>Dépendances Requises Détectées ({{ dependencies.length }})</span>
          <button
            type="button"
            class="text-amber-400 hover:underline"
            @click="selectAll"
          >
            Tout sélectionner
          </button>
        </div>

        <div class="border border-zinc-800 rounded-xl divide-y divide-zinc-800/60 overflow-hidden bg-zinc-950/60 max-h-60 overflow-y-auto">
          <div
            v-for="dep in dependencies"
            :key="dep.id || dep.slug"
            class="p-3.5 flex items-center justify-between gap-3 hover:bg-zinc-900/50 transition-colors"
          >
            <div class="flex items-center gap-3 min-w-0">
              <input
                type="checkbox"
                :checked="isDepSelected(dep)"
                @change="toggleDep(dep)"
                class="rounded border-zinc-700 bg-zinc-900 text-amber-500 focus:ring-amber-500/20 w-4 h-4 cursor-pointer"
              />
              <div class="w-9 h-9 rounded-lg bg-zinc-900 border border-zinc-800 flex items-center justify-center flex-shrink-0 overflow-hidden">
                <img
                  v-if="dep.icon_url"
                  :src="dep.icon_url"
                  :alt="dep.name"
                  class="w-full h-full object-cover"
                />
                <span v-else class="text-sm">🧩</span>
              </div>
              <div class="min-w-0">
                <div class="font-bold text-xs text-zinc-200 truncate flex items-center gap-2">
                  <span>{{ dep.name || dep.slug || dep.id }}</span>
                  <UiBadge variant="emerald" size="xs">Requis</UiBadge>
                </div>
                <div class="text-[11px] text-zinc-500 truncate max-w-sm mt-0.5">
                  {{ dep.summary || `ID: ${dep.id}` }}
                </div>
              </div>
            </div>

            <CheckCircle2
              v-if="isDepSelected(dep)"
              class="w-4 h-4 text-emerald-400 flex-shrink-0"
            />
            <ShieldAlert
              v-else
              class="w-4 h-4 text-zinc-600 flex-shrink-0"
            />
          </div>
        </div>
      </div>
    </div>

    <template #footer>
      <UiButton
        variant="ghost"
        size="sm"
        @click="minecraftStore.closeDependencyModal"
        :disabled="isBatchInstalling"
      >
        Annuler
      </UiButton>

      <UiButton
        variant="primary"
        size="sm"
        :loading="isBatchInstalling"
        @click="minecraftStore.confirmCascadeInstall"
      >
        <template #prefix>
          <Download class="w-4 h-4" />
        </template>
        Installer tout en un clic (Cascade)
      </UiButton>
    </template>
  </UiModal>
</template>
