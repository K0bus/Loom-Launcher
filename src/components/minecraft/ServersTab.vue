<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import {
  Server,
  Users,
  Copy,
  Check,
  Plus,
  Trash2,
  RefreshCw,
  Search,
  Loader2,
  Globe
} from 'lucide-vue-next'
import { useMinecraftStore } from '@/stores/minecraft'
import UiBadge from '@/components/ui/UiBadge.vue'
import UiButton from '@/components/ui/UiButton.vue'
import UiModal from '@/components/ui/UiModal.vue'

const minecraftStore = useMinecraftStore()
const activeInst = computed(() => minecraftStore.activeInstance)

const servers = ref<any[]>([])
const pings = ref<Record<string, any>>({})
const pingingIds = ref<Record<string, boolean>>({})
const isPingingAll = ref(false)
const copiedId = ref<string | null>(null)
const searchQuery = ref('')

// Add Server Modal
const addModalOpen = ref(false)
const newServerName = ref('')
const newServerIp = ref('')
const isTestingNewServer = ref(false)
const newServerTestResult = ref<any | null>(null)

async function loadServers() {
  if (!activeInst.value) return
  try {
    servers.value = await minecraftStore.getInstanceServers(activeInst.value.id)
    pingAllServers()
  } catch (err) {
    console.warn('[ServersTab] Error loading servers:', err)
  }
}

async function pingServer(server: any) {
  const id = server.id || server.ip
  pingingIds.value[id] = true
  try {
    const res = await minecraftStore.pingMinecraftServer(server.ip)
    pings.value[id] = res
  } catch (err) {
    console.warn('[ServersTab] Ping error for', server.ip, err)
    pings.value[id] = {
      online: false,
      pingMs: 999,
      motd: { clean: 'Impossible de joindre le serveur', html: '<span style="color:#FF5555">Impossible de joindre le serveur</span>' },
      players: { online: 0, max: 0, sample: [] }
    }
  } finally {
    pingingIds.value[id] = false
  }
}

async function pingAllServers() {
  isPingingAll.value = true
  try {
    await Promise.all(servers.value.map((s) => pingServer(s)))
  } finally {
    isPingingAll.value = false
  }
}

function copyIp(server: any) {
  navigator.clipboard.writeText(server.ip)
  copiedId.value = server.id || server.ip
  setTimeout(() => {
    copiedId.value = null
  }, 2000)
}

async function deleteServer(server: any) {
  if (!activeInst.value) return
  const id = server.id || server.ip
  if (confirm(`Supprimer le serveur "${server.name}" (${server.ip}) de cette instance ?`)) {
    try {
      await minecraftStore.deleteInstanceServer(activeInst.value.id, id)
      servers.value = servers.value.filter((s) => (s.id || s.ip) !== id)
    } catch (err: any) {
      alert(`Erreur de suppression: ${err?.message || err}`)
    }
  }
}

// Live ping preview in Add Modal
async function testNewServer() {
  if (!newServerIp.value.trim()) return
  isTestingNewServer.value = true
  newServerTestResult.value = null
  try {
    const res = await minecraftStore.pingMinecraftServer(newServerIp.value.trim())
    newServerTestResult.value = res
    if (res.online && !newServerName.value.trim() && res.version?.name) {
      newServerName.value = 'Serveur Minecraft'
    }
  } finally {
    isTestingNewServer.value = false
  }
}

async function confirmAddServer() {
  if (!activeInst.value || !newServerIp.value.trim()) return
  const name = newServerName.value.trim() || 'Serveur Minecraft'
  const ip = newServerIp.value.trim()

  const newEntry = {
    id: `srv-${Date.now()}`,
    name,
    ip,
    icon: newServerTestResult.value?.icon ? newServerTestResult.value.icon.replace(/^data:image\/png;base64,/, '') : undefined,
    acceptTextures: 1
  }

  const updated = [...servers.value, newEntry]
  servers.value = updated
  await minecraftStore.saveInstanceServers(activeInst.value.id, updated)

  // Ping immediately
  pingServer(newEntry)

  // Reset modal
  addModalOpen.value = false
  newServerName.value = ''
  newServerIp.value = ''
  newServerTestResult.value = null
}

const filteredServers = computed(() => {
  const q = searchQuery.value.trim().toLowerCase()
  if (!q) return servers.value
  return servers.value.filter(
    (s) =>
      s.name.toLowerCase().includes(q) ||
      s.ip.toLowerCase().includes(q)
  )
})

onMounted(() => {
  loadServers()
})
</script>

<template>
  <div class="space-y-6">
    <!-- Header Toolbar -->
    <div class="flex flex-col sm:flex-row items-stretch sm:items-center justify-between gap-4 bg-zinc-900/60 p-5 rounded-3xl border border-zinc-800/80">
      <div class="flex items-center gap-3">
        <div class="w-10 h-10 rounded-2xl bg-zinc-950 border border-zinc-800 flex items-center justify-center text-amber-400 shadow-inner">
          <Globe class="w-5 h-5" />
        </div>
        <div>
          <div class="text-sm font-bold text-zinc-100 flex items-center gap-2">
            <span>Serveurs Multijoueur</span>
            <UiBadge variant="amber" size="xs">{{ servers.length }}</UiBadge>
          </div>
          <p class="text-xs text-zinc-500 font-mono">Synchronisé avec <span class="text-zinc-400">.minecraft/servers.dat</span></p>
        </div>
      </div>

      <div class="flex flex-wrap items-center gap-2.5">
        <!-- Search bar -->
        <div class="relative flex-1 sm:w-56">
          <input
            v-model="searchQuery"
            type="text"
            placeholder="Filtrer les serveurs..."
            class="w-full bg-zinc-950 border border-zinc-800 rounded-2xl pl-9 pr-3 py-2 text-xs text-zinc-200 placeholder-zinc-500 focus:outline-none focus:border-amber-500/80 transition-colors"
          />
          <Search class="w-3.5 h-3.5 text-zinc-500 absolute left-3 top-1/2 -translate-y-1/2 pointer-events-none" />
        </div>

        <!-- Ping All Refresh button -->
        <UiButton
          variant="ghost"
          size="sm"
          @click="pingAllServers"
          :disabled="isPingingAll"
          title="Rafraîchir l'état de tous les serveurs"
        >
          <RefreshCw :class="['w-3.5 h-3.5', isPingingAll ? 'animate-spin text-amber-400' : '']" />
        </UiButton>

        <!-- Add Server CTA -->
        <UiButton variant="primary" size="sm" @click="addModalOpen = true">
          <template #prefix>
            <Plus class="w-3.5 h-3.5" />
          </template>
          Ajouter un serveur
        </UiButton>
      </div>
    </div>

    <!-- Servers List -->
    <div v-if="filteredServers.length > 0" class="space-y-4">
      <div
        v-for="server in filteredServers"
        :key="server.id || server.ip"
        class="p-5 rounded-3xl bg-zinc-900/60 border border-zinc-800/80 hover:border-zinc-700 transition flex flex-col md:flex-row items-stretch md:items-center justify-between gap-5 group select-none shadow-sm"
      >
        <!-- Server Identity & Icon -->
        <div class="flex items-start gap-4 flex-1 min-w-0">
          <!-- Favicon -->
          <div class="w-14 h-14 rounded-2xl bg-zinc-950 border border-zinc-800 shrink-0 overflow-hidden flex items-center justify-center p-1 shadow-md">
            <img
              v-if="pings[server.id || server.ip]?.icon"
              :src="pings[server.id || server.ip].icon"
              :alt="server.name"
              class="w-full h-full object-contain rounded-xl"
            />
            <img
              v-else-if="server.icon"
              :src="server.icon.startsWith('data:') ? server.icon : `data:image/png;base64,${server.icon}`"
              :alt="server.name"
              class="w-full h-full object-contain rounded-xl"
            />
            <div
              v-else
              class="w-full h-full rounded-xl bg-zinc-900 flex items-center justify-center text-zinc-500"
            >
              <Server class="w-6 h-6" />
            </div>
          </div>

          <div class="flex-1 min-w-0 space-y-1.5">
            <!-- Name & Online Indicator -->
            <div class="flex flex-wrap items-center gap-2.5">
              <h4 class="font-bold text-sm text-zinc-100 truncate group-hover:text-amber-400 transition-colors">
                {{ server.name }}
              </h4>

              <!-- Online / Offline Ping Status -->
              <span
                v-if="pings[server.id || server.ip]?.online"
                class="inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full bg-emerald-500/10 border border-emerald-500/30 text-[10px] font-mono font-bold text-emerald-400"
              >
                <span class="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse" />
                <span>{{ pings[server.id || server.ip].pingMs }} ms</span>
              </span>

              <span
                v-else-if="pingingIds[server.id || server.ip]"
                class="inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full bg-amber-500/10 border border-amber-500/30 text-[10px] font-mono text-amber-300"
              >
                <Loader2 class="w-2.5 h-2.5 animate-spin text-amber-400" />
                <span>Ping...</span>
              </span>

              <span
                v-else
                class="inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full bg-rose-500/10 border border-rose-500/30 text-[10px] font-mono text-rose-400"
              >
                <span class="w-1.5 h-1.5 rounded-full bg-rose-500" />
                <span>Hors-ligne</span>
              </span>

              <!-- Server Version Badge -->
              <UiBadge
                v-if="pings[server.id || server.ip]?.version?.name"
                variant="zinc"
                size="xs"
                class="font-mono text-[10px]"
              >
                {{ pings[server.id || server.ip].version.name }}
              </UiBadge>
            </div>

            <!-- Server IP Pill -->
            <div class="flex items-center gap-2">
              <span class="text-xs font-mono text-zinc-400 selection:bg-amber-500/30">
                {{ server.ip }}
              </span>
              <button
                type="button"
                @click="copyIp(server)"
                class="text-[11px] font-mono text-zinc-500 hover:text-amber-400 flex items-center gap-1 transition"
                title="Copier l'adresse"
              >
                <Check v-if="copiedId === (server.id || server.ip)" class="w-3 h-3 text-emerald-400" />
                <Copy v-else class="w-3 h-3" />
                <span>{{ copiedId === (server.id || server.ip) ? 'Copié !' : 'Copier' }}</span>
              </button>
            </div>

            <!-- MOTD with Minecraft styling -->
            <div
              v-if="pings[server.id || server.ip]?.motd?.html"
              class="text-xs font-minecraft leading-relaxed bg-zinc-950/70 p-2.5 rounded-xl border border-zinc-800/60 max-w-2xl overflow-hidden"
              v-html="pings[server.id || server.ip].motd.html"
            />
            <div
              v-else
              class="text-xs text-zinc-500 italic"
            >
              {{ pings[server.id || server.ip]?.online === false ? 'Aucun signal reçu du serveur.' : 'En attente des données du serveur...' }}
            </div>
          </div>
        </div>

        <!-- Right Side: Players Count (with HOVER SAMPLE) & Actions -->
        <div class="flex items-center justify-between md:justify-end gap-4 shrink-0 pt-3 md:pt-0 border-t md:border-t-0 border-zinc-800/60">
          <!-- PLAYERS COUNT WITH HOVER TOOLTIP -->
          <div class="relative group/players">
            <div
              class="flex items-center gap-2 px-3 py-2 rounded-2xl bg-zinc-950 border border-zinc-800 group-hover/players:border-amber-500/50 transition cursor-help select-none"
            >
              <Users class="w-4 h-4 text-amber-400" />
              <div class="text-right">
                <div class="text-xs font-mono font-bold text-zinc-200">
                  {{ pings[server.id || server.ip]?.players?.online || 0 }} / {{ pings[server.id || server.ip]?.players?.max || 0 }}
                </div>
                <div class="text-[9px] font-mono text-zinc-500 uppercase">
                  Joueurs
                </div>
              </div>
            </div>

            <!-- HOVER POPOVER : Connected Players List -->
            <div
              class="absolute right-0 bottom-full mb-2 w-64 p-3 rounded-2xl bg-zinc-900 border border-zinc-700 shadow-2xl opacity-0 group-hover/players:opacity-100 pointer-events-none transition-all duration-200 z-50 space-y-2 translate-y-1 group-hover/players:translate-y-0"
            >
              <div class="flex items-center justify-between pb-1.5 border-b border-zinc-800 text-[11px] font-mono font-bold text-zinc-300">
                <span class="flex items-center gap-1.5">
                  <Users class="w-3.5 h-3.5 text-amber-400" />
                  Joueurs en ligne
                </span>
                <span class="text-amber-400">
                  {{ pings[server.id || server.ip]?.players?.online || 0 }}
                </span>
              </div>

              <!-- List of player samples -->
              <div
                v-if="pings[server.id || server.ip]?.players?.sample && pings[server.id || server.ip].players.sample.length > 0"
                class="max-h-48 overflow-y-auto space-y-1.5 pr-1"
              >
                <div
                  v-for="p in pings[server.id || server.ip].players.sample"
                  :key="p.name"
                  class="flex items-center gap-2 p-1 rounded-lg bg-zinc-950/60 border border-zinc-800/60 text-xs font-mono text-zinc-200"
                >
                  <img
                    :src="`https://mc-heads.net/avatar/${encodeURIComponent(p.name)}/20`"
                    :alt="p.name"
                    class="w-5 h-5 rounded shrink-0 bg-zinc-900"
                    loading="lazy"
                  />
                  <span class="truncate">{{ p.name }}</span>
                </div>
              </div>

              <!-- Empty or no sample diffusé -->
              <div
                v-else-if="pings[server.id || server.ip]?.online"
                class="text-[11px] text-zinc-500 italic py-1 text-center"
              >
                {{ (pings[server.id || server.ip]?.players?.online || 0) === 0 ? 'Aucun joueur actuellement connecté.' : 'Le serveur ne diffuse pas la liste nominative des joueurs.' }}
              </div>
              <div v-else class="text-[11px] text-zinc-500 italic py-1 text-center">
                Serveur hors-ligne.
              </div>
            </div>
          </div>

          <!-- Actions: Re-ping & Delete -->
          <div class="flex items-center gap-1.5">
            <button
              type="button"
              @click="pingServer(server)"
              :disabled="pingingIds[server.id || server.ip]"
              class="p-2.5 rounded-xl bg-zinc-950 hover:bg-zinc-800 border border-zinc-800 text-zinc-400 hover:text-amber-400 transition disabled:opacity-50"
              title="Tester à nouveau la connexion"
            >
              <RefreshCw :class="['w-4 h-4', pingingIds[server.id || server.ip] ? 'animate-spin text-amber-400' : '']" />
            </button>

            <button
              type="button"
              @click="deleteServer(server)"
              class="p-2.5 rounded-xl bg-zinc-950 hover:bg-rose-500/10 border border-zinc-800 hover:border-rose-500/30 text-zinc-500 hover:text-rose-400 transition"
              title="Supprimer ce serveur"
            >
              <Trash2 class="w-4 h-4" />
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- Empty State -->
    <div
      v-else
      class="p-16 text-center rounded-3xl bg-zinc-900/30 border border-zinc-800/80 space-y-4"
    >
      <div class="w-14 h-14 mx-auto rounded-2xl bg-zinc-800/80 flex items-center justify-center text-amber-400">
        <Server class="w-7 h-7" />
      </div>
      <div class="space-y-1">
        <h4 class="text-sm font-bold text-zinc-200">
          {{ searchQuery ? 'Aucun serveur ne correspond à votre recherche' : 'Aucun serveur enregistré' }}
        </h4>
        <p class="text-xs text-zinc-500 max-w-sm mx-auto">
          Ajoutez vos serveurs multijoueurs favoris. Ils seront automatiquement sauvegardés dans
          <span class="font-mono text-amber-400">servers.dat</span> pour être accessibles directement dans Minecraft.
        </p>
      </div>
      <UiButton variant="primary" size="sm" @click="addModalOpen = true">
        <template #prefix>
          <Plus class="w-3.5 h-3.5" />
        </template>
        Ajouter un premier serveur
      </UiButton>
    </div>

    <!-- Add Server Modal with Live Ping Test -->
    <UiModal :open="addModalOpen" @close="addModalOpen = false" title="Ajouter un serveur Minecraft" maxWidth="md">
      <div class="space-y-5 p-2">
        <div class="space-y-4">
          <!-- Server IP / Host -->
          <div class="space-y-1.5">
            <label class="block text-xs font-mono font-bold text-zinc-300">Adresse du serveur (IP ou domaine) *</label>
            <div class="flex items-center gap-2">
              <input
                v-model="newServerIp"
                type="text"
                placeholder="mc.hypixel.net ou 127.0.0.1:25565"
                @keyup.enter="testNewServer"
                class="flex-1 bg-zinc-950 border border-zinc-800 rounded-2xl px-4 py-2.5 text-xs font-mono text-zinc-100 placeholder-zinc-500 focus:outline-none focus:border-amber-500"
              />
              <UiButton
                type="button"
                variant="secondary"
                size="sm"
                @click="testNewServer"
                :disabled="!newServerIp.trim() || isTestingNewServer"
              >
                <Loader2 v-if="isTestingNewServer" class="w-3.5 h-3.5 animate-spin" />
                <span v-else>Tester</span>
              </UiButton>
            </div>
          </div>

          <!-- Server Name -->
          <div class="space-y-1.5">
            <label class="block text-xs font-mono font-bold text-zinc-300">Nom affiché</label>
            <input
              v-model="newServerName"
              type="text"
              placeholder="Serveur Minecraft"
              class="w-full bg-zinc-950 border border-zinc-800 rounded-2xl px-4 py-2.5 text-xs text-zinc-100 placeholder-zinc-500 focus:outline-none focus:border-amber-500"
            />
          </div>

          <!-- Live Ping Test Preview -->
          <div
            v-if="newServerTestResult"
            :class="[
              'p-4 rounded-2xl border space-y-2 transition-all',
              newServerTestResult.online ? 'bg-emerald-500/10 border-emerald-500/30' : 'bg-rose-500/10 border-rose-500/30'
            ]"
          >
            <div class="flex items-center justify-between text-xs font-mono font-bold">
              <span :class="newServerTestResult.online ? 'text-emerald-400' : 'text-rose-400'">
                {{ newServerTestResult.online ? '✅ Serveur en ligne' : '❌ Impossible de joindre le serveur' }}
              </span>
              <span v-if="newServerTestResult.online" class="text-zinc-400">
                {{ newServerTestResult.pingMs }} ms • {{ newServerTestResult.players?.online || 0 }} joueurs
              </span>
            </div>

            <div
              v-if="newServerTestResult.motd?.html"
              class="text-xs font-minecraft leading-relaxed bg-zinc-950/80 p-2 rounded-xl border border-zinc-800/80"
              v-html="newServerTestResult.motd.html"
            />
          </div>
        </div>

        <!-- Modal Actions -->
        <div class="flex items-center justify-end gap-2.5 pt-4 border-t border-zinc-800">
          <UiButton variant="ghost" size="sm" @click="addModalOpen = false">
            Annuler
          </UiButton>
          <UiButton
            variant="primary"
            size="sm"
            @click="confirmAddServer"
            :disabled="!newServerIp.trim()"
          >
            Enregistrer dans l'instance
          </UiButton>
        </div>
      </div>
    </UiModal>
  </div>
</template>
