<script setup lang="ts">
import { ref, watch } from 'vue'
import type { Game, RunnerType } from '@/models/game'
import { useGamesStore } from '@/stores/gamesStore'
import UiModal from '@/components/ui/UiModal.vue'
import UiButton from '@/components/ui/UiButton.vue'
import UiSwitch from '@/components/ui/UiSwitch.vue'
import UiInput from '@/components/ui/UiInput.vue'

interface Props {
  open: boolean
  game: Game | null
}

const props = defineProps<Props>()

const emit = defineEmits<{
  (e: 'close'): void
}>()

const gamesStore = useGamesStore()

const localRunner = ref<RunnerType>('proton-ge')
const localGamemode = ref(true)
const localMangohud = ref(true)
const localGamescope = ref(false)
const localWinePrefix = ref('')
const localLaunchArgs = ref('')

watch(() => props.game, (newGame) => {
  if (newGame) {
    localRunner.value = newGame.config.runner
    localGamemode.value = newGame.config.gamemode
    localMangohud.value = newGame.config.mangohud
    localGamescope.value = newGame.config.gamescope
    localWinePrefix.value = newGame.config.winePrefix || ''
    localLaunchArgs.value = newGame.config.launchArguments || ''
  }
}, { immediate: true })

function save() {
  if (!props.game) return
  gamesStore.updateGameConfig(props.game.id, {
    runner: localRunner.value,
    gamemode: localGamemode.value,
    mangohud: localMangohud.value,
    gamescope: localGamescope.value,
    winePrefix: localWinePrefix.value,
    launchArguments: localLaunchArgs.value
  })
  emit('close')
}
</script>

<template>
  <UiModal
    :open="open"
    :title="`Configuration Runner — ${game?.title || ''}`"
    subtitle="Ajustez les paramètres d'exécution Linux, Wine/Proton et couches de compatibilité"
    max-width="lg"
    @close="emit('close')"
  >
    <div v-if="game" class="space-y-5">
      <!-- Runner selection -->
      <div>
        <label class="block text-xs font-semibold text-zinc-300 uppercase tracking-wider mb-2 font-mono">
          Moteur d'exécution
        </label>
        <div class="grid grid-cols-2 gap-2">
          <button
            type="button"
            @click="localRunner = 'native'"
            :class="[
              'p-3 rounded-xl border text-left transition-all',
              localRunner === 'native'
                ? 'bg-amber-500/15 border-amber-500/50 text-amber-300'
                : 'bg-zinc-950/60 border-zinc-800 text-zinc-400 hover:border-zinc-700'
            ]"
          >
            <div class="text-xs font-bold">Linux Natif</div>
            <div class="text-[10px] text-zinc-500 mt-0.5">Binaire ELF Linux standard</div>
          </button>

          <button
            type="button"
            @click="localRunner = 'proton-ge'"
            :class="[
              'p-3 rounded-xl border text-left transition-all',
              localRunner === 'proton-ge'
                ? 'bg-amber-500/15 border-amber-500/50 text-amber-300'
                : 'bg-zinc-950/60 border-zinc-800 text-zinc-400 hover:border-zinc-700'
            ]"
          >
            <div class="text-xs font-bold">Proton-GE 9-20</div>
            <div class="text-[10px] text-zinc-500 mt-0.5">GloriousEggroll optimisé gaming</div>
          </button>

          <button
            type="button"
            @click="localRunner = 'proton-experimental'"
            :class="[
              'p-3 rounded-xl border text-left transition-all',
              localRunner === 'proton-experimental'
                ? 'bg-amber-500/15 border-amber-500/50 text-amber-300'
                : 'bg-zinc-950/60 border-zinc-800 text-zinc-400 hover:border-zinc-700'
            ]"
          >
            <div class="text-xs font-bold">Proton Experimental</div>
            <div class="text-[10px] text-zinc-500 mt-0.5">Dernières corrections Valve</div>
          </button>

          <button
            type="button"
            @click="localRunner = 'wine-staging'"
            :class="[
              'p-3 rounded-xl border text-left transition-all',
              localRunner === 'wine-staging'
                ? 'bg-amber-500/15 border-amber-500/50 text-amber-300'
                : 'bg-zinc-950/60 border-zinc-800 text-zinc-400 hover:border-zinc-700'
            ]"
          >
            <div class="text-xs font-bold">Wine Staging (System)</div>
            <div class="text-[10px] text-zinc-500 mt-0.5">Wine 9.22 avec patches ESYNC/FSYNC</div>
          </button>
        </div>
      </div>

      <!-- Performance Optimizers Switches -->
      <div class="p-4 rounded-xl bg-zinc-950/60 border border-zinc-800/80 space-y-3">
        <UiSwitch
          v-model="localGamemode"
          label="Feral GameMode"
          description="Optimise le gouverneur CPU (performance), les priorités scheduler et les I/O"
        />

        <div class="h-px bg-zinc-800/60" />

        <UiSwitch
          v-model="localMangohud"
          label="MangoHud Overlay"
          description="Affichage tête haute des FPS, frametime, températures et conso VRAM"
        />

        <div class="h-px bg-zinc-800/60" />

        <UiSwitch
          v-model="localGamescope"
          label="Gamescope Micro-compositor"
          description="Isole la résolution du jeu dans un conteneur Wayland haute précision et FSR intégré"
        />
      </div>

      <!-- Wine Prefix & Arguments -->
      <div class="space-y-3">
        <div>
          <label class="block text-xs font-semibold text-zinc-300 uppercase tracking-wider mb-1 font-mono">
            Emplacement du Préfixe Wine (WINEPREFIX)
          </label>
          <UiInput
            v-model="localWinePrefix"
            placeholder="~/.local/share/loom/prefixes/..."
          />
        </div>

        <div>
          <label class="block text-xs font-semibold text-zinc-300 uppercase tracking-wider mb-1 font-mono">
            Arguments de Lancement Additionnels
          </label>
          <UiInput
            v-model="localLaunchArgs"
            placeholder="-novid -high -fullscreen"
          />
        </div>
      </div>
    </div>

    <template #footer>
      <UiButton variant="ghost" size="sm" @click="emit('close')">
        Annuler
      </UiButton>
      <UiButton variant="primary" size="sm" @click="save">
        Sauvegarder les paramètres
      </UiButton>
    </template>
  </UiModal>
</template>
