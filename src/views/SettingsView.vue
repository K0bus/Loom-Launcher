<script setup lang="ts">
import { ref } from 'vue'
import {
  Settings,
  HardDrive,
  Cpu,
  FolderOpen,
  Terminal,
  RotateCcw,
  Check
} from 'lucide-vue-next'
import { useSystemStore } from '@/stores/systemStore'
import UiSwitch from '@/components/ui/UiSwitch.vue'
import UiInput from '@/components/ui/UiInput.vue'
import UiButton from '@/components/ui/UiButton.vue'
import UiBadge from '@/components/ui/UiBadge.vue'

const systemStore = useSystemStore()

// Settings states
const defaultRunner = ref('proton-ge')
const globalGamemode = ref(true)
const globalMangohud = ref(true)
const gamesDirectory = ref('/mnt/games-nvme/SteamLibrary')
const minecraftDirectory = ref('~/.local/share/loom/instances')
const winePrefixesDirectory = ref('~/.local/share/loom/prefixes')
const checkUpdatesAuto = ref(true)

const saved = ref(false)

function saveSettings() {
  saved.value = true
  setTimeout(() => (saved.value = false), 2000)
}
</script>

<template>
  <div class="max-w-7xl mx-auto px-6 sm:px-10 lg:px-14 py-6 space-y-8 pb-16">
    <div>
      <h1 class="text-3xl font-black text-zinc-100 tracking-tight font-display flex items-center gap-3">
        <Settings class="w-8 h-8 text-amber-400" />
        Paramètres du Lanceur
      </h1>
      <p class="text-xs text-zinc-400 mt-1">
        Personnalisation de l'environnement Linux, des répertoires de stockage et du moteur Tauri v2
      </p>
    </div>

    <!-- Section 1 : Couches de Compatibilité Linux -->
    <section class="p-6 rounded-2xl bg-zinc-900/60 border border-zinc-800/80 space-y-4 shadow-lg">
      <div class="flex items-center justify-between">
        <h3 class="text-sm font-bold text-zinc-100 uppercase tracking-wider font-mono flex items-center gap-2">
          <Cpu class="w-4 h-4 text-amber-400" />
          Environnement Linux & Compatibilité
        </h3>
        <UiBadge variant="amber" size="xs">Proton & Wine</UiBadge>
      </div>

      <div class="space-y-4">
        <div>
          <label class="block text-xs font-semibold text-zinc-300 uppercase tracking-wider mb-1.5 font-mono">
            Runner par Défaut pour les Nouveaux Jeux
          </label>
          <select
            v-model="defaultRunner"
            class="w-full bg-zinc-950/80 border border-zinc-800 rounded-xl px-3.5 py-2.5 text-xs text-zinc-200 outline-none focus:border-amber-500"
          >
            <option value="proton-ge">Proton-GE 9-20 (Recommandé)</option>
            <option value="proton-experimental">Proton Experimental (Valve)</option>
            <option value="native">Linux Natif prioritaire</option>
            <option value="wine-staging">Wine Staging (Système)</option>
          </select>
        </div>

        <div class="pt-2 divide-y divide-zinc-800/80 space-y-3">
          <UiSwitch
            v-model="globalGamemode"
            label="Activer Feral GameMode par défaut"
            description="Applique 'gamemoderun' automatiquement à tous les exécutables de jeux"
          />

          <div class="pt-3">
            <UiSwitch
              v-model="globalMangohud"
              label="Activer MangoHud par défaut"
              description="Affiche les statistiques de performances en temps réel sur GPU AMD / NVIDIA"
            />
          </div>
        </div>
      </div>
    </section>

    <!-- Section 2 : Répertoires de Jeux & Instances -->
    <section class="p-6 rounded-2xl bg-zinc-900/60 border border-zinc-800/80 space-y-4 shadow-lg">
      <h3 class="text-sm font-bold text-zinc-100 uppercase tracking-wider font-mono flex items-center gap-2">
        <HardDrive class="w-4 h-4 text-amber-400" />
        Emplacements des Fichiers & Préfixes
      </h3>

      <div class="space-y-4">
        <div>
          <label class="block text-xs font-semibold text-zinc-300 uppercase tracking-wider mb-1 font-mono">
            Dossier Bibliothèque Jeux Principale
          </label>
          <UiInput v-model="gamesDirectory">
            <template #suffix>
              <button type="button" class="text-zinc-400 hover:text-amber-400">
                <FolderOpen class="w-4 h-4" />
              </button>
            </template>
          </UiInput>
        </div>

        <div>
          <label class="block text-xs font-semibold text-zinc-300 uppercase tracking-wider mb-1 font-mono">
            Dossier Instances Minecraft Studio
          </label>
          <UiInput v-model="minecraftDirectory">
            <template #suffix>
              <button type="button" class="text-zinc-400 hover:text-amber-400">
                <FolderOpen class="w-4 h-4" />
              </button>
            </template>
          </UiInput>
        </div>

        <div>
          <label class="block text-xs font-semibold text-zinc-300 uppercase tracking-wider mb-1 font-mono">
            Dossier Préfixes Wine/Proton Isolés
          </label>
          <UiInput v-model="winePrefixesDirectory">
            <template #suffix>
              <button type="button" class="text-zinc-400 hover:text-amber-400">
                <FolderOpen class="w-4 h-4" />
              </button>
            </template>
          </UiInput>
        </div>
      </div>
    </section>

    <!-- Section 3 : Diagnostic Système & Tauri v2 -->
    <section class="p-6 rounded-2xl bg-zinc-900/60 border border-zinc-800/80 space-y-4 shadow-lg">
      <div class="flex items-center justify-between">
        <h3 class="text-sm font-bold text-zinc-100 uppercase tracking-wider font-mono flex items-center gap-2">
          <Terminal class="w-4 h-4 text-amber-400" />
          Diagnostics du Moteur
        </h3>
        <UiBadge :variant="systemStore.stats.tauriDetected ? 'emerald' : 'amber'" size="xs">
          {{ systemStore.stats.tauriDetected ? 'Tauri Runtime Détecté' : 'Mode Navigateur IPC Émulé' }}
        </UiBadge>
      </div>

      <div class="p-4 rounded-xl bg-zinc-950 font-mono text-xs text-zinc-400 space-y-1.5 border border-zinc-800/80">
        <div>Architecture Cible : <span class="text-zinc-200">x86_64-unknown-linux-gnu</span></div>
        <div>Version App : <span class="text-zinc-200">{{ systemStore.stats.appVersion }}</span></div>
        <div>IPC Bridge : <span class="text-amber-400">@tauri-apps/api/core (v2.2.0)</span></div>
        <div>Protocole Graphique : <span class="text-emerald-400">Wayland natif (EGL / Vulkan 1.3)</span></div>
      </div>

      <div class="flex items-center justify-between pt-2">
        <UiSwitch
          v-model="checkUpdatesAuto"
          label="Vérifier les mises à jour automatiquement au démarrage"
        />

        <UiButton variant="secondary" size="sm" @click="systemStore.refreshStats">
          <template #prefix>
            <RotateCcw class="w-3.5 h-3.5" />
          </template>
          Rafraîchir les sondes
        </UiButton>
      </div>
    </section>

    <!-- Save button -->
    <div class="flex justify-end pt-2">
      <UiButton
        variant="primary"
        size="md"
        @click="saveSettings"
      >
        <template #prefix>
          <Check v-if="saved" class="w-4 h-4 text-zinc-950" />
        </template>
        {{ saved ? 'Modifications Enregistrées !' : 'Enregistrer les paramètres' }}
      </UiButton>
    </div>
  </div>
</template>
