<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import {
  Settings,
  HardDrive,
  Cpu,
  FolderOpen,
  Terminal,
  RotateCcw,
  RotateCw,
  AlertTriangle,
  Check,
  Sparkles,
  Zap,
  Power,
  Layers,
  Sliders,
  Palette,
  ExternalLink,
  Activity,
  Gauge,
  Monitor,
  ShieldCheck
} from 'lucide-vue-next'
import { ipc } from '@/services/ipc'
import { useSystemStore } from '@/stores/systemStore'
import { useIgdbStore } from '@/stores/igdbStore'
import { useSettingsStore, type AutostartInstances } from '@/stores/settingsStore'
import { useToastStore } from '@/stores/toastStore'
import { SteamIcon, BattleNetIcon, EpicIcon, GogIcon } from '@/components/icons/platforms'
import UiSwitch from '@/components/ui/UiSwitch.vue'
import UiInput from '@/components/ui/UiInput.vue'
import UiButton from '@/components/ui/UiButton.vue'
import UiBadge from '@/components/ui/UiBadge.vue'

const systemStore = useSystemStore()
const igdbStore = useIgdbStore()
const settingsStore = useSettingsStore()
const toast = useToastStore()

// Local IGDB states
const igdbClientId = ref(igdbStore.credentials.clientId)
const igdbAccessToken = ref(igdbStore.credentials.accessToken)
const igdbClientSecret = ref(igdbStore.credentials.clientSecret)

const saved = ref(false)

// Platform definitions for the autostart section
const platformInstances = [
  {
    key: 'steam' as keyof AutostartInstances,
    name: 'Steam',
    provider: 'Valve Corporation',
    description: 'Initialise le client Steam natif (~/.local/share/Steam) et synchronise le statut des jeux.',
    colorClass: 'text-[#1a9fff]',
    bgLight: 'bg-[#1a9fff]/10',
    borderColor: 'border-[#1a9fff]/30',
    component: SteamIcon
  },
  {
    key: 'battlenet' as keyof AutostartInstances,
    name: 'Battle.net',
    provider: 'Blizzard Entertainment',
    description: 'Initialise le préfixe Wine dédié et prépare le service de lancement Blizzard.',
    colorClass: 'text-[#00aeff]',
    bgLight: 'bg-[#00aeff]/10',
    borderColor: 'border-[#00aeff]/30',
    component: BattleNetIcon
  },
  {
    key: 'epic' as keyof AutostartInstances,
    name: 'Epic Games',
    provider: 'Epic Games / Heroic',
    description: 'Vérifie les sessions du magasin Epic Games et la disponibilité du lanceur Heroic.',
    colorClass: 'text-white',
    bgLight: 'bg-zinc-800/80',
    borderColor: 'border-zinc-700',
    component: EpicIcon
  },
  {
    key: 'gog' as keyof AutostartInstances,
    name: 'GOG Galaxy',
    provider: 'CD PROJEKT',
    description: 'Initialise la prise en charge des jeux sans DRM et synchronise les dossiers de stockage.',
    colorClass: 'text-[#b44aff]',
    bgLight: 'bg-[#86328a]/15',
    borderColor: 'border-[#86328a]/35',
    component: GogIcon
  }
]

// Theme accent options
const accentOptions = [
  { id: 'amber' as const, name: 'Ambre Loom', class: 'bg-amber-500', border: 'border-amber-500' },
  { id: 'emerald' as const, name: 'Émeraude', class: 'bg-emerald-500', border: 'border-emerald-500' },
  { id: 'violet' as const, name: 'Néon Violet', class: 'bg-violet-500', border: 'border-violet-500' },
  { id: 'sky' as const, name: 'Bleu Cyan', class: 'bg-sky-500', border: 'border-sky-500' },
  { id: 'rose' as const, name: 'Rubis Rose', class: 'bg-rose-500', border: 'border-rose-500' }
]

const selectedInstancesCount = computed(() => {
  return Object.values(settingsStore.settings.autostartInstances).filter(Boolean).length
})

watch(
  () => settingsStore.settings.hardwareAcceleration,
  (val) => {
    settingsStore.saveSettings()
    toast.show(
      val
        ? "Accélération matérielle GPU activée. Redémarrez l'application pour appliquer le rendu 60 FPS."
        : "Rendu logiciel sécurisé activé. Redémarrez l'application pour appliquer le changement.",
      'info'
    )
  }
)

const restarting = ref(false)
const showGpuDetails = ref(false)

async function handleRunGpuBenchmark() {
  toast.show('Mesure des performances graphiques et VSync en cours...', 'info')
  await settingsStore.refreshGpuDiagnostic(true)
  if (settingsStore.gpuDiagnostic?.benchmark) {
    const fps = settingsStore.gpuDiagnostic.benchmark.fps
    toast.show(
      `Test terminé : ${fps} FPS mesurés (${settingsStore.gpuDiagnostic.webgl.renderer})`,
      'success'
    )
  }
}

async function handleRestart() {
  restarting.value = true
  toast.show('Redémarrage de Loom Launcher en cours...', 'info')
  try {
    await ipc.restartApp()
  } catch (err) {
    console.error('Erreur redémarrage:', err)
    restarting.value = false
    toast.show('Impossible de redémarrer automatiquement. Veuillez relancer manuellement.', 'error')
  }
}

function saveAllSettings() {
  settingsStore.saveSettings()
  igdbStore.saveCredentials(igdbClientId.value, igdbAccessToken.value, igdbClientSecret.value)
  saved.value = true
  if (settingsStore.restartRequired) {
    toast.show('Paramètres enregistrés ! Cliquez sur "Redémarrer pour appliquer".', 'info')
  } else {
    toast.show('Paramètres de Loom enregistrés avec succès !', 'success')
  }
  setTimeout(() => (saved.value = false), 2500)
}

function handleResetDefaults() {
  settingsStore.resetToDefaults()
  toast.show('Paramètres réinitialisés aux valeurs recommandées', 'info')
}

async function handleGenerateToken() {
  if (!igdbClientSecret.value) return
  await igdbStore.generateTwitchToken(igdbClientSecret.value)
  igdbAccessToken.value = igdbStore.credentials.accessToken
  toast.show('Token IGDB généré automatiquement !', 'success')
}

async function handlePickFolder(key: 'gamesDirectory' | 'minecraftDirectory' | 'winePrefixesDirectory') {
  try {
    const titles = {
      gamesDirectory: 'Sélectionner le dossier Bibliothèque Jeux',
      minecraftDirectory: 'Sélectionner le dossier des instances Minecraft',
      winePrefixesDirectory: 'Sélectionner le dossier des préfixes Wine'
    }
    const picked = await ipc.pickDirectory(titles[key], settingsStore.settings[key])
    if (picked) {
      settingsStore.settings[key] = picked
      settingsStore.saveSettings()
      toast.show('Répertoire mis à jour avec succès !', 'success')
    }
  } catch (err) {
    console.error('Erreur sélection dossier:', err)
  }
}

async function handleOpenFolder(path: string) {
  try {
    await ipc.openDirectoryFolder(path)
    toast.show('Dossier ouvert dans le gestionnaire de fichiers', 'info')
  } catch (err) {
    console.error('Erreur ouverture dossier:', err)
  }
}

async function handleTestNotification() {
  try {
    await ipc.sendDesktopNotification(
      'Loom Launcher',
      'Test réussi : Les notifications de bureau sont pleinement opérationnelles !',
      'loom-launcher'
    )
    toast.show('Notification de bureau envoyée !', 'success')
  } catch (err) {
    toast.show('Erreur envoi notification de bureau', 'error')
  }
}
</script>

<template>
  <div class="max-w-7xl mx-auto px-6 sm:px-10 lg:px-14 py-6 space-y-8 pb-16">
    <!-- Header principal -->
    <div class="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4">
      <div>
        <h1 class="text-3xl font-black text-zinc-100 tracking-tight font-display flex items-center gap-3">
          <Settings class="w-8 h-8 text-amber-400" />
          Paramètres du Lanceur
        </h1>
        <p class="text-xs text-zinc-400 mt-1">
          Personnalisation complète de Loom Launcher, de l'environnement Linux et des intégrations de plateformes
        </p>
      </div>

      <div class="flex items-center gap-2.5">
        <UiButton
          v-if="settingsStore.restartRequired"
          variant="primary"
          size="sm"
          :loading="restarting"
          @click="handleRestart"
          class="!bg-emerald-500 hover:!bg-emerald-400 !text-zinc-950 font-bold !border-emerald-400/40 shadow-lg shadow-emerald-500/25 animate-pulse"
          title="Redémarrer l'application pour appliquer les modifications du moteur graphique"
        >
          <template #prefix>
            <RotateCw class="w-3.5 h-3.5 text-zinc-950" />
          </template>
          Redémarrer pour appliquer
        </UiButton>

        <UiButton
          variant="secondary"
          size="sm"
          @click="handleResetDefaults"
          title="Réinitialiser les options par défaut"
        >
          <template #prefix>
            <RotateCcw class="w-3.5 h-3.5 text-zinc-400" />
          </template>
          Par défaut
        </UiButton>

        <UiButton
          variant="primary"
          size="sm"
          @click="saveAllSettings"
        >
          <template #prefix>
            <Check v-if="saved" class="w-4 h-4 text-zinc-950" />
          </template>
          {{ saved ? 'Enregistré !' : 'Enregistrer' }}
        </UiButton>
      </div>
    </div>

    <!-- ======================================================== -->
    <!-- PREMIÈRE CATÉGORIE : PARAMÈTRES PROPRES À LOOM LAUNCHER -->
    <!-- ======================================================== -->
    <section class="p-6 sm:p-7 rounded-2xl bg-zinc-900/80 border border-amber-500/30 space-y-6 shadow-xl relative overflow-hidden backdrop-blur-sm">
      <!-- Glow d'ambiance haut droite -->
      <div class="absolute -top-24 -right-24 w-60 h-60 bg-amber-500/10 rounded-full blur-3xl pointer-events-none" />

      <!-- Titre de la catégorie -->
      <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-2 border-b border-zinc-800/80 pb-4">
        <div>
          <h2 class="text-base font-bold text-zinc-100 uppercase tracking-wider font-mono flex items-center gap-2.5">
            <Sparkles class="w-5 h-5 text-amber-400" />
            Paramètres Propres à Loom Launcher
          </h2>
          <p class="text-xs text-zinc-400 mt-1">
            Performances du moteur, automatisation au démarrage et comportement de l'application
          </p>
        </div>
        <UiBadge variant="amber" size="sm">Loom Core v2</UiBadge>
      </div>

      <div class="space-y-6">
        <!-- 1. Accélération matérielle & Moteur -->
        <div class="p-4 rounded-xl bg-zinc-950/60 border border-zinc-800/70 space-y-4">
          <div class="flex items-center justify-between">
            <span class="text-xs font-bold uppercase tracking-wider font-mono text-zinc-300 flex items-center gap-2">
              <Zap class="w-4 h-4 text-amber-400" />
              Moteur & Rendu Visuel
            </span>
            <span
              :class="[
                'text-[11px] font-mono px-2.5 py-0.5 rounded-full border flex items-center gap-1.5 transition-colors',
                settingsStore.gpuDiagnostic?.isEffectiveGpuAccelerated
                  ? 'bg-emerald-500/15 border-emerald-500/40 text-emerald-400'
                  : 'bg-amber-500/15 border-amber-500/40 text-amber-300'
              ]"
            >
              <span
                class="w-1.5 h-1.5 rounded-full"
                :class="settingsStore.gpuDiagnostic?.isEffectiveGpuAccelerated ? 'bg-emerald-400 animate-pulse' : 'bg-amber-400'"
              />
              {{
                settingsStore.gpuDiagnostic?.isEffectiveGpuAccelerated
                  ? `GPU Actif : ${settingsStore.gpuDiagnostic.effectiveGpuName.split('/')[0]}`
                  : 'Rendu Logiciel Sécurisé (llvmpipe)'
              }}
            </span>
          </div>

          <UiSwitch
            v-model="settingsStore.settings.hardwareAcceleration"
            label="Accélération Matérielle GPU (EGL / DMA-BUF)"
            description="Délègue le compositing, le scroll et les animations au GPU via DMA-BUF et Skia. Réduit la charge CPU et synchronise à la fréquence de votre écran (60+ FPS)."
          />

          <!-- Alerte & Bouton redémarrage contextuel -->
          <div
            v-if="settingsStore.restartRequired"
            class="flex flex-col sm:flex-row sm:items-center justify-between gap-3 p-3 rounded-lg bg-amber-500/10 border border-amber-500/30 text-amber-200 text-xs mt-2"
          >
            <div class="flex items-center gap-2">
              <AlertTriangle class="w-4 h-4 text-amber-400 shrink-0" />
              <span>Redémarrage requis pour appliquer le profil de rendu au moteur WebKitGTK.</span>
            </div>
            <UiButton
              variant="primary"
              size="sm"
              :loading="restarting"
              @click="handleRestart"
              class="!bg-emerald-500 hover:!bg-emerald-400 !text-zinc-950 font-bold !border-emerald-400/40 shrink-0"
            >
              <template #prefix>
                <RotateCw class="w-3.5 h-3.5 text-zinc-950" />
              </template>
              Redémarrer pour appliquer
            </UiButton>
          </div>

          <!-- Section Diagnostic & Benchmark Graphique -->
          <div class="pt-2 border-t border-zinc-800/60 space-y-3">
            <div class="flex items-center justify-between">
              <div class="text-xs font-medium text-zinc-300 flex items-center gap-2">
                <Monitor class="w-3.5 h-3.5 text-zinc-400" />
                <span>Sonde Graphique & Performances Réelles</span>
              </div>
              <div class="flex items-center gap-2">
                <UiButton
                  variant="ghost"
                  size="sm"
                  @click="showGpuDetails = !showGpuDetails"
                  class="text-[11px] !py-1 text-zinc-400 hover:text-zinc-200"
                >
                  {{ showGpuDetails ? 'Masquer détails' : 'Détails techniques' }}
                </UiButton>
                <UiButton
                  variant="secondary"
                  size="sm"
                  :loading="settingsStore.isRunningBenchmark"
                  @click="handleRunGpuBenchmark"
                  class="text-[11px] !py-1 font-mono gap-1.5"
                >
                  <template #prefix>
                    <Activity class="w-3.5 h-3.5 text-amber-400" />
                  </template>
                  Tester Framerate (FPS)
                </UiButton>
              </div>
            </div>

            <!-- Grille d'état rapide -->
            <div class="grid grid-cols-1 sm:grid-cols-3 gap-2.5 text-xs font-mono">
              <div class="p-2.5 rounded-lg bg-zinc-900/60 border border-zinc-800/60">
                <div class="text-[10px] text-zinc-400 uppercase tracking-wider">GPU / Renderer</div>
                <div
                  class="font-bold text-zinc-200 truncate mt-0.5"
                  :title="settingsStore.gpuDiagnostic?.effectiveGpuName || settingsStore.gpuDiagnostic?.webgl?.renderer"
                >
                  {{ settingsStore.gpuDiagnostic?.effectiveGpuName || settingsStore.gpuDiagnostic?.webgl?.renderer || 'Détection...' }}
                </div>
              </div>

              <div class="p-2.5 rounded-lg bg-zinc-900/60 border border-zinc-800/60">
                <div class="text-[10px] text-zinc-400 uppercase tracking-wider">Serveur / Protocole</div>
                <div class="font-bold text-zinc-200 mt-0.5 flex items-center gap-1.5">
                  <ShieldCheck class="w-3.5 h-3.5 text-amber-400" />
                  {{ settingsStore.gpuDiagnostic?.backend?.displayServer?.toUpperCase() || 'WAYLAND' }}
                </div>
              </div>

              <div class="p-2.5 rounded-lg bg-zinc-900/60 border border-zinc-800/60">
                <div class="text-[10px] text-zinc-400 uppercase tracking-wider">Taux de rafraîchissement</div>
                <div class="font-bold mt-0.5 flex items-center gap-1.5" :class="settingsStore.gpuDiagnostic?.benchmark ? 'text-emerald-400' : 'text-zinc-400'">
                  <Gauge class="w-3.5 h-3.5" />
                  {{
                    settingsStore.gpuDiagnostic?.benchmark
                      ? `${settingsStore.gpuDiagnostic.benchmark.fps} FPS (${settingsStore.gpuDiagnostic.benchmark.avgFrameTimeMs}ms)`
                      : 'Cliquer sur Tester'
                  }}
                </div>
              </div>
            </div>

            <!-- Détails techniques complets dépliables -->
            <div
              v-if="showGpuDetails"
              class="p-3 rounded-lg bg-zinc-900/40 border border-zinc-800/80 text-[11px] font-mono space-y-1.5 text-zinc-300 animate-in fade-in duration-150"
            >
              <div class="flex justify-between" v-if="settingsStore.gpuDiagnostic?.backend?.hardwareGpuName">
                <span class="text-zinc-400">GPU Matériel (Sonde Linux) :</span>
                <span class="text-emerald-400 font-bold">{{ settingsStore.gpuDiagnostic.backend.hardwareGpuName }}</span>
              </div>
              <div class="flex justify-between" v-if="settingsStore.gpuDiagnostic?.isAppleGpuMasked">
                <span class="text-zinc-400">Masque WebKit Anti-Track :</span>
                <span class="text-amber-300 font-bold">Actif ('Apple GPU' masqué en sandbox WebGL)</span>
              </div>
              <div class="flex justify-between">
                <span class="text-zinc-400">Vendor WebGL :</span>
                <span class="text-zinc-200">{{ settingsStore.gpuDiagnostic?.webgl?.vendor || 'N/A' }}</span>
              </div>
              <div class="flex justify-between">
                <span class="text-zinc-400">Version WebGL :</span>
                <span class="text-zinc-200">{{ settingsStore.gpuDiagnostic?.webgl?.webglVersion || 'N/A' }}</span>
              </div>
              <div class="flex justify-between">
                <span class="text-zinc-400">Taille Max Texture :</span>
                <span class="text-zinc-200">{{ settingsStore.gpuDiagnostic?.webgl?.maxTextureSize || 0 }} px</span>
              </div>
              <div class="flex justify-between">
                <span class="text-zinc-400">Mode WebKit Compositor :</span>
                <span class="text-zinc-200">
                  {{
                    settingsStore.gpuDiagnostic?.backend?.webkitDmabufDisabled
                      ? 'DMA-BUF Désactivé (SHM / CPU)'
                      : 'DMA-BUF Zero-Copy Actif'
                  }}
                </span>
              </div>
              <div class="flex justify-between">
                <span class="text-zinc-400">Mesa Software Override :</span>
                <span class="text-zinc-200">
                  {{ settingsStore.gpuDiagnostic?.backend?.libglAlwaysSoftware ? 'Oui (LIBGL_ALWAYS_SOFTWARE=1)' : 'Non (GPU Natif)' }}
                </span>
              </div>
              <div class="pt-1 text-[10px] text-zinc-400 border-t border-zinc-800/60">
                Diagnostic : {{ settingsStore.gpuDiagnostic?.diagnosticMessage }}
              </div>
            </div>
          </div>
        </div>

        <!-- 2. Démarrage avec le PC & Session système -->
        <div class="p-4 rounded-xl bg-zinc-950/60 border border-zinc-800/70 space-y-4">
          <div class="flex items-center justify-between">
            <span class="text-xs font-bold uppercase tracking-wider font-mono text-zinc-300 flex items-center gap-2">
              <Power class="w-4 h-4 text-amber-400" />
              Démarrage & Session Système
            </span>
            <span
              v-if="settingsStore.settings.autostartWithSystem"
              class="text-[11px] font-mono px-2 py-0.5 rounded-full bg-amber-500/15 border border-amber-500/40 text-amber-300"
            >
              Autostart Activé
            </span>
          </div>

          <div class="divide-y divide-zinc-800/80 space-y-3">
            <UiSwitch
              v-model="settingsStore.settings.autostartWithSystem"
              label="Démarrage avec le PC"
              description="Démarre automatiquement Loom Launcher dès l'ouverture de votre session utilisateur (Linux / Windows)."
            />

            <div class="pt-3">
              <UiSwitch
                v-model="settingsStore.settings.startMinimized"
                :disabled="!settingsStore.settings.autostartWithSystem"
                label="Démarrer Loom minimisé (Arrière-plan discret)"
                description="Lance Loom discrètement dans la zone de notification sans afficher la fenêtre principale au premier plan."
              />
            </div>

            <div class="pt-3">
              <UiSwitch
                v-model="settingsStore.settings.closeToTray"
                label="Réduire dans la barre d'état au clic sur fermer (Close to Tray)"
                description="Le bouton de fermeture masque la fenêtre au lieu d'interrompre l'application, maintenant actifs les téléchargements et les notifications."
              />
            </div>
          </div>
        </div>

        <!-- 3. Démarrer les différentes instances de plateformes (Sélectionnable) -->
        <div class="p-4 rounded-xl bg-zinc-950/60 border border-zinc-800/70 space-y-4">
          <div class="flex items-center justify-between">
            <div>
              <span class="text-xs font-bold uppercase tracking-wider font-mono text-zinc-300 flex items-center gap-2">
                <Layers class="w-4 h-4 text-amber-400" />
                Instances & Clients de Jeux au Démarrage
              </span>
              <p class="text-xs text-zinc-400 mt-1">
                Choisissez les plateformes de jeux que Loom doit initialiser ou exécuter simultanément lors de son ouverture :
              </p>
            </div>
            <UiBadge variant="zinc" size="xs">
              {{ selectedInstancesCount }} / {{ platformInstances.length }} sélectionnée(s)
            </UiBadge>
          </div>

          <!-- Grille des plateformes sélectionnables -->
          <div class="grid grid-cols-1 md:grid-cols-2 gap-3 pt-1">
            <div
              v-for="platform in platformInstances"
              :key="platform.key"
              @click="settingsStore.toggleInstance(platform.key)"
              :class="[
                'p-3.5 rounded-xl border transition-all cursor-pointer flex items-start gap-3 select-none group',
                settingsStore.settings.autostartInstances[platform.key]
                  ? 'bg-zinc-900 border-amber-500/60 shadow-md ring-1 ring-amber-500/20'
                  : 'bg-zinc-950/70 border-zinc-800/80 hover:border-zinc-700 hover:bg-zinc-900/40 opacity-80'
              ]"
            >
              <!-- Icône officielle de la plateforme -->
              <div
                class="w-10 h-10 rounded-lg flex items-center justify-center flex-shrink-0 border transition-transform group-hover:scale-105"
                :class="[platform.bgLight, platform.borderColor]"
              >
                <component :is="platform.component" className="w-5 h-5" :colored="true" />
              </div>

              <!-- Infos -->
              <div class="flex-1 min-w-0">
                <div class="flex items-center justify-between gap-2">
                  <span class="text-sm font-semibold text-zinc-200 group-hover:text-white transition">
                    {{ platform.name }}
                  </span>
                  <span
                    :class="[
                      'w-4 h-4 rounded-full flex items-center justify-center text-[10px] border transition-colors',
                      settingsStore.settings.autostartInstances[platform.key]
                        ? 'bg-amber-500 border-amber-400 text-zinc-950 font-bold'
                        : 'border-zinc-700 bg-zinc-900 text-transparent'
                    ]"
                  >
                    ✓
                  </span>
                </div>
                <div class="text-[11px] font-mono text-zinc-500 mt-0.5">
                  {{ platform.provider }}
                </div>
                <p class="text-xs text-zinc-400 mt-1 line-clamp-2 leading-relaxed">
                  {{ platform.description }}
                </p>
              </div>
            </div>
          </div>

          <!-- Option complémentaire pour les instances -->
          <div class="pt-2 border-t border-zinc-800/80">
            <UiSwitch
              v-model="settingsStore.settings.startInstancesMinimized"
              label="Lancer les instances en mode silencieux / réduit"
              description="Démarre les plateformes de jeux sélectionnées en arrière-plan sans ouvrir leurs fenêtres d'accueil au premier plan."
            />
          </div>
        </div>

        <!-- 4. Ergonomie, Comportement en jeu & Personnalisation visuelle -->
        <div class="p-4 rounded-xl bg-zinc-950/60 border border-zinc-800/70 space-y-4">
          <span class="text-xs font-bold uppercase tracking-wider font-mono text-zinc-300 flex items-center gap-2">
            <Sliders class="w-4 h-4 text-amber-400" />
            Comportement en Jeu & Personnalisation
          </span>

          <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
            <!-- Action après lancement d'un jeu -->
            <div>
              <label class="block text-xs font-semibold text-zinc-300 uppercase tracking-wider mb-1.5 font-mono">
                Action après le lancement d'un jeu
              </label>
              <select
                v-model="settingsStore.settings.afterGameLaunch"
                class="w-full bg-zinc-900 border border-zinc-800 rounded-xl px-3.5 py-2.5 text-xs text-zinc-200 outline-none focus:border-amber-500"
              >
                <option value="minimize">Réduire Loom en arrière-plan (Recommandé)</option>
                <option value="hide">Masquer Loom dans la barre d'état (Économie de ressources)</option>
                <option value="keep">Laisser la fenêtre de Loom active</option>
              </select>
              <p class="text-[11px] text-zinc-500 mt-1">
                Libère de la mémoire et des cycles GPU pendant vos sessions de jeu.
              </p>
            </div>

            <!-- Accentuation visuelle / Thème Loom -->
            <div>
              <label class="block text-xs font-semibold text-zinc-300 uppercase tracking-wider mb-1.5 font-mono flex items-center gap-1.5">
                <Palette class="w-3.5 h-3.5 text-amber-400" />
                Accentuation de couleur de l'interface
              </label>
              <div class="flex items-center gap-2.5 pt-1">
                <button
                  v-for="accent in accentOptions"
                  :key="accent.id"
                  type="button"
                  @click="settingsStore.settings.themeAccent = accent.id"
                  :class="[
                    'w-7 h-7 rounded-full transition-transform flex items-center justify-center relative focus:outline-none',
                    accent.class,
                    settingsStore.settings.themeAccent === accent.id
                      ? 'scale-110 ring-2 ring-white ring-offset-2 ring-offset-zinc-950 shadow-md'
                      : 'opacity-70 hover:opacity-100 hover:scale-105'
                  ]"
                  :title="accent.name"
                >
                  <span
                    v-if="settingsStore.settings.themeAccent === accent.id"
                    class="text-zinc-950 font-bold text-xs"
                  >
                    ✓
                  </span>
                </button>
                <span class="text-xs font-mono text-zinc-400 ml-2">
                  {{ accentOptions.find(a => a.id === settingsStore.settings.themeAccent)?.name }}
                </span>
              </div>
            </div>
          </div>

          <!-- Toggles ergonomiques -->
          <div class="pt-3 border-t border-zinc-800/80 space-y-3">
            <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
              <div class="flex-1">
                <UiSwitch
                  v-model="settingsStore.settings.desktopNotifications"
                  label="Notifications système du lanceur"
                  description="Alerte de bureau lors de la fin d'un téléchargement, déblocage d'un succès Steam/Xbox ou mise à jour de mod."
                />
              </div>
              <UiButton
                v-if="settingsStore.settings.desktopNotifications"
                variant="secondary"
                size="sm"
                @click="handleTestNotification"
                class="shrink-0 self-start sm:self-center"
              >
                Tester l'alerte OS
              </UiButton>
            </div>

            <UiSwitch
              v-model="settingsStore.settings.autoSyncCloudSaves"
              label="Synchronisation automatique des sauvegardes locales & Cloud"
              description="Surveille l'intégrité des dossiers de sauvegardes et prépare les synchronisations après chaque session."
            />
          </div>
        </div>
      </div>
    </section>

    <!-- ======================================================== -->
    <!-- SECTION 2 : COUCHES DE COMPATIBILITÉ LINUX & RUNNERS -->
    <!-- ======================================================== -->
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
            v-model="settingsStore.settings.defaultRunner"
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
            v-model="settingsStore.settings.globalGamemode"
            label="Activer Feral GameMode par défaut"
            description="Applique 'gamemoderun' automatiquement à tous les exécutables de jeux"
          />

          <div class="pt-3">
            <UiSwitch
              v-model="settingsStore.settings.globalMangohud"
              label="Activer MangoHud par défaut"
              description="Affiche les statistiques de performances en temps réel sur GPU AMD / NVIDIA"
            />
          </div>
        </div>
      </div>
    </section>

    <!-- ======================================================== -->
    <!-- SECTION 3 : RÉPERTOIRES DE JEUX & INSTANCES -->
    <!-- ======================================================== -->
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
          <UiInput v-model="settingsStore.settings.gamesDirectory">
            <template #suffix>
              <div class="flex items-center gap-1.5">
                <button
                  type="button"
                  @click="handlePickFolder('gamesDirectory')"
                  class="p-1 rounded text-zinc-400 hover:text-amber-400 hover:bg-zinc-800 transition"
                  title="Parcourir le dossier"
                >
                  <FolderOpen class="w-4 h-4" />
                </button>
                <button
                  type="button"
                  @click="handleOpenFolder(settingsStore.settings.gamesDirectory)"
                  class="p-1 rounded text-zinc-400 hover:text-amber-400 hover:bg-zinc-800 transition"
                  title="Ouvrir dans le gestionnaire de fichiers"
                >
                  <ExternalLink class="w-4 h-4" />
                </button>
              </div>
            </template>
          </UiInput>
        </div>

        <div>
          <label class="block text-xs font-semibold text-zinc-300 uppercase tracking-wider mb-1 font-mono">
            Dossier Instances Minecraft Studio
          </label>
          <UiInput v-model="settingsStore.settings.minecraftDirectory">
            <template #suffix>
              <div class="flex items-center gap-1.5">
                <button
                  type="button"
                  @click="handlePickFolder('minecraftDirectory')"
                  class="p-1 rounded text-zinc-400 hover:text-amber-400 hover:bg-zinc-800 transition"
                  title="Parcourir le dossier"
                >
                  <FolderOpen class="w-4 h-4" />
                </button>
                <button
                  type="button"
                  @click="handleOpenFolder(settingsStore.settings.minecraftDirectory)"
                  class="p-1 rounded text-zinc-400 hover:text-amber-400 hover:bg-zinc-800 transition"
                  title="Ouvrir dans le gestionnaire de fichiers"
                >
                  <ExternalLink class="w-4 h-4" />
                </button>
              </div>
            </template>
          </UiInput>
        </div>

        <div>
          <label class="block text-xs font-semibold text-zinc-300 uppercase tracking-wider mb-1 font-mono">
            Dossier Préfixes Wine/Proton Isolés
          </label>
          <UiInput v-model="settingsStore.settings.winePrefixesDirectory">
            <template #suffix>
              <div class="flex items-center gap-1.5">
                <button
                  type="button"
                  @click="handlePickFolder('winePrefixesDirectory')"
                  class="p-1 rounded text-zinc-400 hover:text-amber-400 hover:bg-zinc-800 transition"
                  title="Parcourir le dossier"
                >
                  <FolderOpen class="w-4 h-4" />
                </button>
                <button
                  type="button"
                  @click="handleOpenFolder(settingsStore.settings.winePrefixesDirectory)"
                  class="p-1 rounded text-zinc-400 hover:text-amber-400 hover:bg-zinc-800 transition"
                  title="Ouvrir dans le gestionnaire de fichiers"
                >
                  <ExternalLink class="w-4 h-4" />
                </button>
              </div>
            </template>
          </UiInput>
        </div>
      </div>
    </section>

    <!-- ======================================================== -->
    <!-- SECTION 4 : DIAGNOSTIC SYSTÈME & TAURI V2 -->
    <!-- ======================================================== -->
    <section class="p-6 rounded-2xl bg-zinc-900/60 border border-zinc-800/80 space-y-4 shadow-lg">
      <div class="flex items-center justify-between">
        <h3 class="text-sm font-bold text-zinc-100 uppercase tracking-wider font-mono flex items-center gap-2">
          <Terminal class="w-4 h-4 text-amber-400" />
          Diagnostics du Moteur & Système
        </h3>
        <UiBadge :variant="systemStore.stats.tauriDetected ? 'emerald' : 'amber'" size="xs">
          {{ systemStore.stats.tauriDetected ? 'Tauri Runtime Détecté' : 'Mode Navigateur IPC Émulé' }}
        </UiBadge>
      </div>

      <div class="p-4 rounded-xl bg-zinc-950 font-mono text-xs text-zinc-400 space-y-2 border border-zinc-800/80">
        <div class="grid grid-cols-1 sm:grid-cols-2 gap-2.5">
          <div>Architecture Cible : <span class="text-zinc-200">x86_64-unknown-linux-gnu</span></div>
          <div>Version App : <span class="text-zinc-200">{{ systemStore.stats.appVersion }}</span></div>
          <div>IPC Bridge : <span class="text-amber-400">@tauri-apps/api/core (v2.2.0)</span></div>
          <div>Protocole Graphique : <span :class="systemStore.stats.displayServer === 'Wayland' ? 'text-emerald-400 font-bold' : 'text-sky-400'">{{ systemStore.stats.displayServer || 'Wayland' }} (Session active)</span></div>
          <div>Noyau Système : <span class="text-zinc-200">{{ systemStore.stats.kernelVersion || 'Linux x86_64' }}</span></div>
          <div>Mémoire Vive (RAM) : <span class="text-zinc-200">{{ systemStore.stats.ramUsedGb }} GB / {{ systemStore.stats.ramTotalGb }} GB (Charge: {{ systemStore.stats.cpuUsage }}%)</span></div>
        </div>
      </div>

      <div class="flex items-center justify-between pt-2">
        <UiSwitch
          v-model="settingsStore.settings.checkUpdatesAuto"
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

    <!-- ======================================================== -->
    <!-- SECTION 5 : INTÉGRATION API IGDB & MÉTADONNÉES -->
    <!-- ======================================================== -->
    <section class="p-6 rounded-2xl bg-zinc-900/60 border border-zinc-800/80 space-y-4 shadow-lg">
      <div class="flex items-center justify-between">
        <h3 class="text-sm font-bold text-zinc-100 uppercase tracking-wider font-mono flex items-center gap-2">
          <i class="ph-fill ph-database text-amber-400 text-lg"></i>
          API IGDB v4 & Twitch Developer
        </h3>
        <UiBadge :variant="igdbStore.isLive ? 'emerald' : 'amber'" size="xs">
          {{ igdbStore.isLive ? 'API v4 En Direct' : 'Mode Hybride (Token Public)' }}
        </UiBadge>
      </div>

      <p class="text-xs text-zinc-400 leading-relaxed">
        L'API IGDB (Internet Game Database) alimente les affiches 3:4, les artworks 1080p, les résumés, les captures d'écran, les développeurs et les notes globales de vos jeux.
      </p>

      <div class="space-y-4 pt-1">
        <!-- Client ID Public -->
        <div>
          <div class="flex items-center justify-between mb-1">
            <label class="block text-xs font-semibold text-zinc-300 uppercase tracking-wider font-mono">
              Client ID Public de l'Application
            </label>
            <span class="text-[11px] text-amber-400 font-mono">Token Public Requis</span>
          </div>
          <UiInput v-model="igdbClientId" placeholder="yxup2w2zyqkugl2erkq2ni9ih59rq7" />
        </div>

        <!-- Access Token (Bearer) -->
        <div>
          <div class="flex items-center justify-between mb-1">
            <label class="block text-xs font-semibold text-zinc-300 uppercase tracking-wider font-mono">
              Bearer Access Token (Twitch OAuth)
            </label>
            <span class="text-[11px] text-zinc-500">Optionnel si Secret renseigné</span>
          </div>
          <UiInput
            v-model="igdbAccessToken"
            type="password"
            placeholder="Token d'accès généré (ex: gq4h...)"
          />
        </div>

        <!-- Client Secret (Optional for auto-generation) -->
        <div>
          <div class="flex items-center justify-between mb-1">
            <label class="block text-xs font-semibold text-zinc-300 uppercase tracking-wider font-mono">
              Twitch Client Secret (Génération automatique)
            </label>
            <span class="text-[11px] text-zinc-500">Stocké localement uniquement</span>
          </div>
          <UiInput
            v-model="igdbClientSecret"
            type="password"
            placeholder="Client secret de votre console Twitch Developer"
          />
        </div>

        <!-- Buttons test & generate -->
        <div class="flex flex-wrap items-center justify-between gap-3 pt-2">
          <div class="flex items-center gap-2">
            <UiButton
              variant="secondary"
              size="sm"
              :disabled="!igdbClientSecret"
              @click="handleGenerateToken"
            >
              <template #prefix>
                <i class="ph ph-key text-base"></i>
              </template>
              Générer Token Automatiquement
            </UiButton>

            <UiButton
              variant="secondary"
              size="sm"
              :loading="igdbStore.isTesting"
              @click="igdbStore.testConnection"
            >
              <template #prefix>
                <i class="ph ph-plugs-connected text-base"></i>
              </template>
              Tester la Connexion IGDB
            </UiButton>
          </div>

          <a
            href="https://dev.twitch.tv/console/apps"
            target="_blank"
            rel="noopener noreferrer"
            class="text-xs text-amber-400 hover:text-amber-300 flex items-center gap-1 font-mono"
          >
            Console Twitch Developer
            <ExternalLink class="w-3.5 h-3.5" />
          </a>
        </div>
      </div>
    </section>

    <!-- Save & Restart buttons flottants bas de page -->
    <div class="flex items-center justify-end gap-3 pt-2">
      <UiButton
        v-if="settingsStore.restartRequired"
        variant="primary"
        size="md"
        :loading="restarting"
        @click="handleRestart"
        class="!bg-emerald-500 hover:!bg-emerald-400 !text-zinc-950 font-bold !border-emerald-400/40 shadow-lg shadow-emerald-500/25 animate-pulse"
        title="Redémarrer l'application pour appliquer les modifications"
      >
        <template #prefix>
          <RotateCw class="w-4 h-4 text-zinc-950" />
        </template>
        Redémarrer pour appliquer
      </UiButton>

      <UiButton
        variant="primary"
        size="md"
        @click="saveAllSettings"
      >
        <template #prefix>
          <Check v-if="saved" class="w-4 h-4 text-zinc-950" />
        </template>
        {{ saved ? 'Modifications Enregistrées !' : 'Enregistrer tous les paramètres' }}
      </UiButton>
    </div>
  </div>
</template>
