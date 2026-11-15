<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import type { Game } from '@/models/game'
import type { WowCharacter, WowInstance } from '@/models/wowAddon'
import { ipc } from '@/services/ipc'

const props = defineProps<{
  game: Game
}>()

const characters = ref<WowCharacter[]>([])
const isLoading = ref(false)
const searchFilter = ref('')
const selectedRealm = ref<string>('all')

const WOW_CLASS_COLORS: Record<string, string> = {
  WARRIOR: '#C79C6E',
  PALADIN: '#F58CBA',
  HUNTER: '#ABD473',
  ROGUE: '#FFF569',
  PRIEST: '#F0EBE0',
  DEATHKNIGHT: '#C41E3A',
  SHAMAN: '#0070DE',
  MAGE: '#69CCF0',
  WARLOCK: '#9482C9',
  MONK: '#00FF96',
  DRUID: '#FF7D0A',
  DEMONHUNTER: '#A330C9',
  EVOKER: '#33937F'
}

const WOW_CLASS_NAMES: Record<string, string> = {
  WARRIOR: 'Guerrier',
  PALADIN: 'Paladin',
  HUNTER: 'Chasseur',
  ROGUE: 'Voleur',
  PRIEST: 'Prêtre',
  DEATHKNIGHT: 'Chevalier de la mort',
  SHAMAN: 'Chaman',
  MAGE: 'Mage',
  WARLOCK: 'Démoniste',
  MONK: 'Moine',
  DRUID: 'Druide',
  DEMONHUNTER: 'Chasseur de démons',
  EVOKER: 'Évocateur'
}

const WOW_RACE_NAMES: Record<string, string> = {
  dwarf: 'Nain',
  human: 'Humain',
  nightelf: 'Elfe de la nuit',
  gnome: 'Gnome',
  draenei: 'Draeneï',
  worgen: 'Worgen',
  orc: 'Orc',
  undead: 'Mort-vivant',
  scourge: 'Mort-vivant',
  tauren: 'Tauren',
  troll: 'Troll',
  bloodelf: 'Elfe de sang',
  goblin: 'Gobelin',
  pandaren: 'Pandaren'
}

function getClassColor(charClass?: string): string {
  if (!charClass) return '#9CA3AF'
  return WOW_CLASS_COLORS[charClass.toUpperCase()] || '#9CA3AF'
}

function getClassName(charClass?: string): string {
  if (!charClass) return 'Classe inconnue'
  return WOW_CLASS_NAMES[charClass.toUpperCase()] || charClass
}

function getRaceName(charRace?: string): string {
  if (!charRace) return ''
  const clean = charRace.toLowerCase().replace(/[^a-z]/g, '')
  return WOW_RACE_NAMES[clean] || charRace
}

function formatLastPlayed(timestamp?: number): string {
  if (!timestamp || timestamp <= 0) return 'Jamais joué'
  const now = Math.floor(Date.now() / 1000)
  const diffSec = now - timestamp

  if (diffSec < 120) return "À l'instant"
  if (diffSec < 3600) {
    const mins = Math.floor(diffSec / 60)
    return `Il y a ${mins} min`
  }
  if (diffSec < 86400) {
    const hours = Math.floor(diffSec / 3600)
    return `Il y a ${hours} h`
  }
  if (diffSec < 172800) {
    const d = new Date(timestamp * 1000)
    const timeStr = d.toLocaleTimeString('fr-FR', { hour: '2-digit', minute: '2-digit' })
    return `Hier à ${timeStr}`
  }
  return new Date(timestamp * 1000).toLocaleDateString('fr-FR', {
    day: 'numeric',
    month: 'short',
    year: 'numeric'
  })
}

// Find matching WoW instance path
function resolveInstancePath(instances: WowInstance[]): string | null {
  const gId = props.game.id.toLowerCase()
  const title = (props.game.title || '').toLowerCase()

  let targetFlavor: string = 'forever'
  if (gId.includes('wowf') || title.includes('forever')) {
    targetFlavor = 'forever'
  } else if (gId.includes('classic_era') || title.includes('discovery') || title.includes('era')) {
    targetFlavor = 'classic_era'
  } else if (gId.includes('classic') || title.includes('classic')) {
    targetFlavor = 'classic'
  } else if (gId.includes('retail') || gId === 'bnet-wow' || title === 'world of warcraft') {
    targetFlavor = 'retail'
  }

  // 1. Check exact flavor match from scanned instances
  const match = instances.find((i) => i.flavor === targetFlavor && i.installed)
  if (match) return match.path

  // 2. Check if installPath contains the flavor directory
  if (props.game.installPath) {
    const dirMap: Record<string, string> = {
      forever: '_forever_',
      retail: '_retail_',
      classic: '_classic_',
      classic_era: '_classic_era_'
    }
    const expectedSub = dirMap[targetFlavor]
    if (props.game.installPath.includes(expectedSub)) {
      return props.game.installPath
    }
    // Base directory with subfolder
    return `${props.game.installPath.replace(/\/$/, '')}/${expectedSub}`
  }

  // 3. Fallback to any installed instance
  const anyInst = instances.find((i) => i.installed)
  return anyInst ? anyInst.path : null
}

async function loadCharacters() {
  isLoading.value = true
  try {
    const instances = await ipc.scanWowInstances()
    const path = resolveInstancePath(instances)
    if (path) {
      const chars = await ipc.getWowCharacters(path)
      // Sort by lastPlayed descending
      chars.sort((a, b) => (b.lastPlayed || 0) - (a.lastPlayed || 0))
      characters.value = chars
    } else {
      characters.value = []
    }
  } catch (err) {
    console.warn('[WowCharactersCard] Erreur chargement personnages:', err)
    characters.value = []
  } finally {
    isLoading.value = false
  }
}

// Available unique realms
const realms = computed(() => {
  const rSet = new Set<string>()
  characters.value.forEach((c) => {
    if (c.realm) rSet.add(c.realm)
  })
  return Array.from(rSet)
})

// Filtered characters
const filteredCharacters = computed(() => {
  return characters.value.filter((c) => {
    if (selectedRealm.value !== 'all' && c.realm !== selectedRealm.value) {
      return false
    }
    if (searchFilter.value.trim()) {
      const q = searchFilter.value.toLowerCase().trim()
      const matchName = c.name.toLowerCase().includes(q)
      const matchRealm = c.realm.toLowerCase().includes(q)
      const matchClass = c.class ? c.class.toLowerCase().includes(q) : false
      if (!matchName && !matchRealm && !matchClass) return false
    }
    return true
  })
})

watch(() => props.game.id, () => {
  loadCharacters()
})

onMounted(() => {
  loadCharacters()
})
</script>

<template>
  <div class="bg-zinc-900/60 backdrop-blur-xl rounded-2xl border border-zinc-800/80 p-5 shadow-xl space-y-4">
    <!-- Header -->
    <div class="flex items-center justify-between">
      <div class="flex items-center gap-2.5">
        <div class="w-8 h-8 rounded-lg bg-amber-500/10 border border-amber-500/20 flex items-center justify-center text-amber-400">
          <i class="ph-bold ph-shield-chevron text-lg"></i>
        </div>
        <div>
          <h3 class="text-sm font-semibold text-zinc-200">
            Personnages
          </h3>
          <p class="text-[11px] text-zinc-400">
            Roster local de cette version
          </p>
        </div>
      </div>

      <div class="flex items-center gap-2">
        <span
          v-if="characters.length > 0"
          class="text-xs px-2 py-0.5 rounded-full bg-zinc-800/80 border border-zinc-700/50 text-zinc-300 font-mono"
        >
          {{ characters.length }}
        </span>
        <button
          type="button"
          @click="loadCharacters"
          class="p-1.5 rounded-lg text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800/60 transition"
          :title="'Actualiser les personnages'"
        >
          <i class="ph ph-arrows-clockwise text-sm" :class="{ 'animate-spin': isLoading }"></i>
        </button>
      </div>
    </div>

    <!-- Realm Filter (if multiple realms exist) -->
    <div v-if="realms.length > 1" class="flex items-center gap-1.5 overflow-x-auto pb-1 text-xs scrollbar-none">
      <button
        type="button"
        @click="selectedRealm = 'all'"
        class="px-2.5 py-1 rounded-lg transition whitespace-nowrap text-[11px]"
        :class="selectedRealm === 'all'
          ? 'bg-amber-500/20 text-amber-300 border border-amber-500/40 font-medium'
          : 'bg-zinc-800/40 text-zinc-400 hover:text-zinc-200 border border-zinc-800'"
      >
        Tous les royaumes
      </button>
      <button
        v-for="r in realms"
        :key="r"
        type="button"
        @click="selectedRealm = r"
        class="px-2.5 py-1 rounded-lg transition whitespace-nowrap text-[11px]"
        :class="selectedRealm === r
          ? 'bg-amber-500/20 text-amber-300 border border-amber-500/40 font-medium'
          : 'bg-zinc-800/40 text-zinc-400 hover:text-zinc-200 border border-zinc-800'"
      >
        {{ r }}
      </button>
    </div>

    <!-- Quick Search (if > 3 characters) -->
    <div v-if="characters.length > 3" class="relative">
      <i class="ph ph-magnifying-glass absolute left-3 top-1/2 -translate-y-1/2 text-zinc-400 text-xs"></i>
      <input
        v-model="searchFilter"
        type="text"
        placeholder="Filtrer par nom, classe, royaume..."
        class="w-full pl-8 pr-3 py-1.5 rounded-lg bg-zinc-950/60 border border-zinc-800/80 text-xs text-zinc-200 placeholder-zinc-500 focus:outline-none focus:border-amber-500/50"
      />
    </div>

    <!-- Loading State -->
    <div v-if="isLoading" class="py-8 flex flex-col items-center justify-center gap-2 text-zinc-400">
      <span class="w-5 h-5 rounded-full border-2 border-amber-500 border-t-transparent animate-spin"></span>
      <span class="text-xs">Recherche des profils dans le répertoire WTF...</span>
    </div>

    <!-- Empty State -->
    <div v-else-if="filteredCharacters.length === 0" class="py-6 text-center space-y-2">
      <div class="w-10 h-10 rounded-full bg-zinc-800/60 mx-auto flex items-center justify-center text-zinc-500">
        <i class="ph ph-user-circle text-xl"></i>
      </div>
      <p class="text-xs text-zinc-300 font-medium">
        {{ characters.length === 0 ? 'Aucun personnage détecté' : 'Aucun résultat correspondant' }}
      </p>
      <p class="text-[11px] text-zinc-500 max-w-xs mx-auto">
        {{ characters.length === 0
          ? 'Connectez-vous au jeu une première fois pour initialiser le profil du personnage dans le dossier WTF.'
          : 'Modifiez vos filtres de recherche pour afficher les personnages.'
        }}
      </p>
    </div>

    <!-- Character List -->
    <div v-else class="space-y-2.5 max-h-[340px] overflow-y-auto pr-1">
      <div
        v-for="char in filteredCharacters"
        :key="`${char.account}-${char.realm}-${char.fullName}`"
        class="relative flex items-center justify-between p-3 rounded-xl bg-zinc-950/60 hover:bg-zinc-800/40 border border-zinc-800/70 transition group overflow-hidden"
      >
        <!-- Class Color Accent Bar -->
        <div
          class="absolute left-0 top-0 bottom-0 w-1 rounded-l-xl transition-all group-hover:w-1.5"
          :style="{ backgroundColor: getClassColor(char.class) }"
        />

        <!-- Character Details -->
        <div class="flex items-center gap-3 pl-2 min-w-0">
          <!-- Level Badge -->
          <div
            class="w-8 h-8 rounded-lg flex items-center justify-center font-bold text-xs flex-shrink-0 border"
            :style="{
              backgroundColor: `${getClassColor(char.class)}15`,
              borderColor: `${getClassColor(char.class)}40`,
              color: getClassColor(char.class)
            }"
            :title="char.level ? `Niveau ${char.level}` : 'Niveau inconnu'"
          >
            {{ char.level || '?' }}
          </div>

          <div class="min-w-0 space-y-0.5">
            <!-- Name & Class Color -->
            <div class="flex items-center gap-2">
              <span
                class="font-semibold text-sm truncate tracking-wide"
                :style="{ color: getClassColor(char.class) }"
              >
                {{ char.name }}
              </span>

              <!-- Faction Badge -->
              <span
                v-if="char.faction"
                class="text-[9px] px-1.5 py-0.5 rounded uppercase font-bold tracking-wider"
                :class="char.faction.toLowerCase() === 'alliance'
                  ? 'bg-blue-500/20 text-blue-300 border border-blue-500/30'
                  : 'bg-red-500/20 text-red-300 border border-red-500/30'"
              >
                {{ char.faction }}
              </span>
            </div>

            <!-- Race, Class & Realm -->
            <p class="text-[11px] text-zinc-400 truncate flex items-center gap-1.5">
              <span>{{ getRaceName(char.race) }} {{ getClassName(char.class) }}</span>
              <span class="text-zinc-600">•</span>
              <span class="text-zinc-500">{{ char.realm }}</span>
            </p>
          </div>
        </div>

        <!-- Right Side: Last Played -->
        <div class="text-right flex-shrink-0 pl-2">
          <p class="text-[10px] text-zinc-500 font-medium">
            {{ formatLastPlayed(char.lastPlayed) }}
          </p>
          <span
            v-if="char.account"
            class="text-[9px] text-zinc-600 font-mono block truncate max-w-[90px]"
            :title="char.account"
          >
            {{ char.account }}
          </span>
        </div>
      </div>
    </div>
  </div>
</template>
