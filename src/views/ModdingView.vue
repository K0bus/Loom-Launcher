<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import {
  Layers,
  Box,
  ArrowRight,
  ShieldCheck,
  PackageCheck,
  Play,
  Cpu,
  Download,
  FolderGit2,
  CheckCircle2,
  Clock,
  Calendar,
  Sparkles,
  Swords
} from 'lucide-vue-next'
import { useMinecraftStore } from '@/stores/minecraftStore'
import { useWowAddonsStore } from '@/stores/wowAddonsStore'
import MinecraftAvatar from '@/components/minecraft/MinecraftAvatar.vue'
import UiButton from '@/components/ui/UiButton.vue'
import UiBadge from '@/components/ui/UiBadge.vue'

const router = useRouter()
const minecraftStore = useMinecraftStore()
const wowStore = useWowAddonsStore()

const activeInstance = computed(() => minecraftStore.activeInstance)
const totalInstancesCount = computed(() => minecraftStore.instances.length)
const account = computed(() => minecraftStore.account)

const wowActiveInstance = computed(() => wowStore.selectedInstance)
const wowTotalAddons = computed(() => {
  return wowStore.instances.reduce((acc, curr) => acc + (curr.addonsCount || 0), 0)
})

onMounted(() => {
  wowStore.loadInstances()
})
</script>

<template>
  <div class="max-w-7xl mx-auto px-6 sm:px-10 lg:px-14 py-6 space-y-8 pb-20">
    <!-- 1. En-tête de section -->
    <div class="flex flex-col md:flex-row items-start md:items-center justify-between gap-4">
      <div>
        <h1 class="text-3xl font-black text-zinc-100 tracking-tight font-display flex items-center gap-3">
          <Layers class="w-8 h-8 text-amber-400" />
          Sous-Système de Modding
        </h1>
        <p class="text-xs text-zinc-400 mt-1">
          Sélectionnez un titre supporté pour accéder à son studio de modding dédié, ses instances et ses catalogues unifiés
        </p>
      </div>

      <div class="flex items-center gap-3">
        <UiBadge variant="amber" size="sm" class="font-mono">
          GameRegistry V2 • 2 Adaptateurs Actifs
        </UiBadge>
      </div>
    </div>

    <!-- 2. SÉLECTION DES JEUX -->
    <div class="space-y-6">
      <div class="flex items-center justify-between">
        <div class="flex items-center gap-2 text-xs font-mono font-semibold text-zinc-400 uppercase tracking-wider">
          <Box class="w-4 h-4 text-amber-400" />
          <span>Sélection du Jeu & Studios Dédiés</span>
        </div>
        <span class="text-xs font-mono text-emerald-400 flex items-center gap-1.5">
          <span class="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse" />
          2 adaptateurs opérationnels
        </span>
      </div>

      <!-- CARTE 1 : MINECRAFT: JAVA EDITION -->
      <div class="relative overflow-hidden rounded-3xl bg-gradient-to-br from-zinc-950 via-zinc-900 to-zinc-950 border border-amber-500/40 p-8 shadow-2xl transition-all hover:border-amber-500/70 group">
        <div class="absolute -right-12 -top-12 w-96 h-96 bg-amber-500/10 rounded-full blur-3xl pointer-events-none transition-opacity group-hover:opacity-100 opacity-60" />

        <div class="relative z-10 flex flex-col lg:flex-row items-start lg:items-center justify-between gap-8">
          <div class="flex flex-col sm:flex-row items-start sm:items-center gap-6">
            <div class="relative w-24 h-24 rounded-2xl bg-zinc-900 border border-zinc-700/80 flex items-center justify-center text-5xl shadow-xl flex-shrink-0 group-hover:scale-105 transition-transform">
              ⛏️
              <div class="absolute -bottom-2 -right-2 w-7 h-7 rounded-xl bg-amber-500 text-zinc-950 flex items-center justify-center text-xs font-black shadow-md">
                MC
              </div>
            </div>

            <div class="space-y-3">
              <div class="flex flex-wrap items-center gap-3">
                <h2 class="text-2xl font-black text-zinc-100 font-display">
                  Minecraft: Java Edition
                </h2>
                <UiBadge variant="emerald" size="xs" class="font-mono">
                  STUDIO OPÉRATIONNEL
                </UiBadge>
                <span class="text-xs text-zinc-500 font-mono">Mojang Studios</span>
              </div>

              <p class="text-xs text-zinc-400 max-w-2xl leading-relaxed">
                Gestionnaire modulaire avancé avec provisioning autonome Java (Adoptium 8/17/21), recherche unifiée Modrinth & CurseForge, isolation d'instances et déploiement de mods par liens symboliques atomiques.
              </p>

              <!-- Statut Compte Microsoft / Xbox Live connecté -->
              <div class="flex flex-wrap items-center gap-4 text-xs font-mono pt-1">
                <div v-if="account" class="flex items-center gap-2 bg-zinc-950/80 px-3 py-1.5 rounded-xl border border-zinc-800">
                  <MinecraftAvatar
                    :uuid="account.uuid"
                    :name="account.name"
                    :skin-url="account.skin_url"
                    :raw-skin-url="account.raw_skin_url"
                    :size="20"
                    rounded="rounded-md"
                  />
                  <span class="text-zinc-200 font-bold">{{ account.name }}</span>
                  <span class="text-[10px] text-emerald-400 flex items-center gap-1">
                    <span class="w-1.5 h-1.5 rounded-full bg-emerald-400" />
                    Xbox Live
                  </span>
                </div>

                <div v-else class="text-zinc-500 flex items-center gap-1.5">
                  <span class="w-1.5 h-1.5 rounded-full bg-zinc-600" />
                  <span>Aucun compte lié (mode hors-ligne disponible)</span>
                </div>

                <span class="text-zinc-600">•</span>

                <div class="flex items-center gap-1.5 text-zinc-300">
                  <PackageCheck class="w-4 h-4 text-amber-400" />
                  <span>{{ totalInstancesCount }} {{ totalInstancesCount > 1 ? 'instances configurées' : 'instance configurée' }}</span>
                </div>

                <span class="text-zinc-600">•</span>

                <div class="flex items-center gap-1.5 text-zinc-300">
                  <Clock class="w-4 h-4 text-amber-400" />
                  <span>{{ minecraftStore.globalPlaytimeFormatted }} de jeu</span>
                </div>

                <template v-if="minecraftStore.globalLastPlayedFormatted !== 'Jamais jouée'">
                  <span class="text-zinc-600">•</span>

                  <div class="flex items-center gap-1.5 text-zinc-300">
                    <Calendar class="w-4 h-4 text-sky-400" />
                    <span>Joué {{ minecraftStore.globalLastPlayedFormatted }}</span>
                  </div>
                </template>
              </div>
            </div>
          </div>

          <!-- Actions du Studio Minecraft -->
          <div class="flex flex-col sm:flex-row items-stretch sm:items-center gap-3 w-full lg:w-auto flex-shrink-0">
            <UiButton
              v-if="activeInstance"
              variant="outline"
              size="lg"
              class="border-zinc-700 hover:border-zinc-600"
              @click="minecraftStore.launchInstance(activeInstance.id)"
            >
              <template #prefix>
                <Play class="w-4 h-4 text-amber-400 fill-current" />
              </template>
              Lancer {{ activeInstance.name }}
            </UiButton>

            <UiButton
              variant="primary"
              size="lg"
              class="shadow-lg shadow-amber-500/20"
              @click="router.push('/modding/minecraft')"
            >
              <template #prefix>
                <Box class="w-5 h-5" />
              </template>
              Ouvrir le Studio Minecraft
              <template #suffix>
                <ArrowRight class="w-4 h-4 ml-1" />
              </template>
            </UiButton>
          </div>
        </div>

        <!-- Détails techniques de l'adaptateur Minecraft -->
        <div class="mt-8 pt-6 border-t border-zinc-800/80 grid grid-cols-1 sm:grid-cols-3 gap-4">
          <div class="space-y-1.5 bg-zinc-950/40 p-3.5 rounded-2xl border border-zinc-800/50">
            <div class="text-[11px] font-mono text-zinc-400 flex items-center gap-1.5 font-semibold">
              <Cpu class="w-3.5 h-3.5 text-amber-400" />
              <span>Modloaders Reconnus</span>
            </div>
            <div class="flex flex-wrap gap-1.5 pt-0.5">
              <span
                v-for="l in ['Fabric', 'NeoForge', 'Quilt', 'Forge', 'Vanilla']"
                :key="l"
                class="px-2 py-0.5 rounded-md bg-zinc-900 border border-zinc-800 text-[10px] font-mono text-zinc-300"
              >
                {{ l }}
              </span>
            </div>
          </div>

          <div class="space-y-1.5 bg-zinc-950/40 p-3.5 rounded-2xl border border-zinc-800/50">
            <div class="text-[11px] font-mono text-zinc-400 flex items-center gap-1.5 font-semibold">
              <Download class="w-3.5 h-3.5 text-amber-400" />
              <span>APIs de Téléchargement</span>
            </div>
            <div class="flex flex-wrap gap-1.5 pt-0.5">
              <span
                v-for="p in ['Modrinth v2', 'CurseForge Core', 'Fichiers locaux (.jar, .zip)']"
                :key="p"
                class="px-2 py-0.5 rounded-md bg-zinc-900 border border-zinc-800 text-[10px] font-mono text-amber-400/90"
              >
                {{ p }}
              </span>
            </div>
          </div>

          <div class="space-y-1.5 bg-zinc-950/40 p-3.5 rounded-2xl border border-zinc-800/50">
            <div class="text-[11px] font-mono text-zinc-400 flex items-center gap-1.5 font-semibold">
              <ShieldCheck class="w-3.5 h-3.5 text-emerald-400" />
              <span>Isolation & Déploiement</span>
            </div>
            <p class="text-[11px] font-mono text-zinc-400 pt-0.5 leading-snug">
              Liens symboliques atomiques vers cache global sans duplication disque.
            </p>
          </div>
        </div>
      </div>

      <!-- CARTE 2 : WORLD OF WARCRAFT -->
      <div class="relative overflow-hidden rounded-3xl bg-gradient-to-br from-zinc-950 via-zinc-900 to-zinc-950 border border-amber-500/40 p-8 shadow-2xl transition-all hover:border-amber-500/70 group">
        <div class="absolute -right-12 -top-12 w-96 h-96 bg-amber-500/10 rounded-full blur-3xl pointer-events-none transition-opacity group-hover:opacity-100 opacity-60" />

        <div class="relative z-10 flex flex-col lg:flex-row items-start lg:items-center justify-between gap-8">
          <div class="flex flex-col sm:flex-row items-start sm:items-center gap-6">
            <div class="relative w-24 h-24 rounded-2xl bg-zinc-900 border border-zinc-700/80 flex items-center justify-center text-5xl shadow-xl flex-shrink-0 group-hover:scale-105 transition-transform">
              ⚔️
              <div class="absolute -bottom-2 -right-2 w-7 h-7 rounded-xl bg-amber-500 text-zinc-950 flex items-center justify-center text-xs font-black shadow-md">
                WoW
              </div>
            </div>

            <div class="space-y-3">
              <div class="flex flex-wrap items-center gap-3">
                <h2 class="text-2xl font-black text-zinc-100 font-display">
                  World of Warcraft
                </h2>
                <UiBadge variant="emerald" size="xs" class="font-mono">
                  STUDIO OPÉRATIONNEL
                </UiBadge>
                <span class="text-xs text-zinc-500 font-mono">Blizzard Entertainment</span>
              </div>

              <p class="text-xs text-zinc-400 max-w-2xl leading-relaxed">
                Gestionnaire natif d'addons sans création d'instances : détection directe des versions installées (Forever, Retail, Classic), recherche et installation d'addons CurseForge avec mise à jour et bascule d'activation en un clic.
              </p>

              <!-- Métadonnées WoW -->
              <div class="flex flex-wrap items-center gap-4 text-xs font-mono pt-1">
                <div class="flex items-center gap-2 bg-zinc-950/80 px-3 py-1.5 rounded-xl border border-zinc-800">
                  <span class="w-1.5 h-1.5 rounded-full bg-emerald-400" />
                  <span class="text-zinc-200 font-bold">
                    {{ wowActiveInstance?.name || 'World of Warcraft' }}
                  </span>
                  <span class="text-[10px] text-amber-400 font-mono">
                    {{ wowActiveInstance?.flavor ? `_${wowActiveInstance.flavor}_` : 'Natif' }}
                  </span>
                </div>

                <span class="text-zinc-600">•</span>

                <div class="flex items-center gap-1.5 text-zinc-300">
                  <PackageCheck class="w-4 h-4 text-amber-400" />
                  <span>{{ wowTotalAddons }} addons détectés</span>
                </div>

                <span class="text-zinc-600">•</span>

                <div class="flex items-center gap-1.5 text-zinc-300">
                  <Sparkles class="w-4 h-4 text-amber-400" />
                  <span>CurseForge WoW API active</span>
                </div>
              </div>
            </div>
          </div>

          <!-- Actions du Studio WoW -->
          <div class="flex flex-col sm:flex-row items-stretch sm:items-center gap-3 w-full lg:w-auto flex-shrink-0">
            <UiButton
              variant="primary"
              size="lg"
              class="shadow-lg shadow-amber-500/20"
              @click="router.push('/modding/wow')"
            >
              <template #prefix>
                <Swords class="w-5 h-5" />
              </template>
              Gérer les AddOns WoW
              <template #suffix>
                <ArrowRight class="w-4 h-4 ml-1" />
              </template>
            </UiButton>
          </div>
        </div>

        <!-- Détails techniques de l'adaptateur WoW -->
        <div class="mt-8 pt-6 border-t border-zinc-800/80 grid grid-cols-1 sm:grid-cols-3 gap-4">
          <div class="space-y-1.5 bg-zinc-950/40 p-3.5 rounded-2xl border border-zinc-800/50">
            <div class="text-[11px] font-mono text-zinc-400 flex items-center gap-1.5 font-semibold">
              <Cpu class="w-3.5 h-3.5 text-amber-400" />
              <span>Format & Moteur</span>
            </div>
            <div class="flex flex-wrap gap-1.5 pt-0.5">
              <span
                v-for="l in ['AddOns (.toc)', 'Moteur Lua', 'Interface XML']"
                :key="l"
                class="px-2 py-0.5 rounded-md bg-zinc-900 border border-zinc-800 text-[10px] font-mono text-zinc-300"
              >
                {{ l }}
              </span>
            </div>
          </div>

          <div class="space-y-1.5 bg-zinc-950/40 p-3.5 rounded-2xl border border-zinc-800/50">
            <div class="text-[11px] font-mono text-zinc-400 flex items-center gap-1.5 font-semibold">
              <Download class="w-3.5 h-3.5 text-amber-400" />
              <span>Fournisseurs Addons</span>
            </div>
            <div class="flex flex-wrap gap-1.5 pt-0.5">
              <span
                v-for="p in ['CurseForge API v1 (Game #1)', 'Fichiers locaux (.zip, .toc)']"
                :key="p"
                class="px-2 py-0.5 rounded-md bg-zinc-900 border border-zinc-800 text-[10px] font-mono text-amber-400/90"
              >
                {{ p }}
              </span>
            </div>
          </div>

          <div class="space-y-1.5 bg-zinc-950/40 p-3.5 rounded-2xl border border-zinc-800/50">
            <div class="text-[11px] font-mono text-zinc-400 flex items-center gap-1.5 font-semibold">
              <ShieldCheck class="w-3.5 h-3.5 text-emerald-400" />
              <span>Intégration Directe</span>
            </div>
            <p class="text-[11px] font-mono text-zinc-400 pt-0.5 leading-snug">
              Gestion in-place dans Interface/AddOns sans créer de profils isolés redondants.
            </p>
          </div>
        </div>
      </div>
    </div>

    <!-- 3. Socle Technique & Architecture Modulaire (GameAdapter) -->
    <div class="space-y-4 pt-4">
      <div class="flex items-center gap-2 text-xs font-mono font-semibold text-zinc-400 uppercase tracking-wider">
        <FolderGit2 class="w-4 h-4 text-amber-400" />
        <span>Architecture Modulaire Multi-Jeux (Backend Rust)</span>
      </div>

      <div class="grid grid-cols-1 md:grid-cols-3 gap-5">
        <div class="p-5 rounded-2xl bg-zinc-900/40 border border-zinc-800/80 space-y-2">
          <div class="flex items-center gap-2 text-amber-400 font-bold text-sm">
            <CheckCircle2 class="w-4 h-4" />
            <span>GameAdapter & GameRegistry</span>
          </div>
          <p class="text-xs text-zinc-400 leading-relaxed">
            Le cœur Rust de Loom Launcher enregistre les adaptateurs via un registre centralisé, isolant la logique de chaque jeu de manière hermétique.
          </p>
        </div>

        <div class="p-5 rounded-2xl bg-zinc-900/40 border border-zinc-800/80 space-y-2">
          <div class="flex items-center gap-2 text-amber-400 font-bold text-sm">
            <CheckCircle2 class="w-4 h-4" />
            <span>CurseForge & Modrinth Hub</span>
          </div>
          <p class="text-xs text-zinc-400 leading-relaxed">
            Interrogation temps réel des APIs de téléchargement de mods et addons avec validation des signatures et versioning multi-saveurs.
          </p>
        </div>

        <div class="p-5 rounded-2xl bg-zinc-900/40 border border-zinc-800/80 space-y-2">
          <div class="flex items-center gap-2 text-amber-400 font-bold text-sm">
            <CheckCircle2 class="w-4 h-4" />
            <span>Zéro Duplication d'Instances</span>
          </div>
          <p class="text-xs text-zinc-400 leading-relaxed">
            Pour les jeux à installation unifiée comme World of Warcraft, Loom opère directement sur le répertoire du jeu sans créer d'instances parasites.
          </p>
        </div>
      </div>
    </div>
  </div>
</template>
