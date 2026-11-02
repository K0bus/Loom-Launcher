<script setup lang="ts">
import { ref } from 'vue'
import type { MinecraftInstance, MinecraftServer } from '@/models/minecraft'
import { useMinecraftStore } from '@/stores/minecraftStore'
import { useToastStore } from '@/stores/toastStore'
import UiButton from '@/components/ui/UiButton.vue'
import UiBadge from '@/components/ui/UiBadge.vue'
import UiModal from '@/components/ui/UiModal.vue'
import UiInput from '@/components/ui/UiInput.vue'

interface Props {
  instance: MinecraftInstance
}

const props = defineProps<Props>()

const minecraftStore = useMinecraftStore()
const toast = useToastStore()

const copiedServerId = ref<string | null>(null)
const isPinging = ref(false)

// Add Server Modal
const addServerModalOpen = ref(false)
const newServerName = ref('')
const newServerIp = ref('')

function copyIp(server: MinecraftServer) {
  navigator.clipboard.writeText(server.ip)
  copiedServerId.value = server.id
  toast.show(`Adresse IP ${server.ip} copiée !`, 'success')
  setTimeout(() => {
    copiedServerId.value = null
  }, 2000)
}

function refreshPings() {
  isPingsOngoing()
}

function isPingsOngoing() {
  isPinging.value = true
  setTimeout(() => {
    props.instance.servers.forEach((srv) => {
      if (srv.status === 'online') {
        srv.pingMs = Math.max(8, srv.pingMs + Math.floor((Math.random() - 0.5) * 8))
      }
    })
    isPinging.value = false
    toast.show('Pings des serveurs mis à jour (servers.dat)', 'info')
  }, 400)
}

function launchAndJoin(server: MinecraftServer) {
  toast.show(`Lancement de l'instance et connexion directe à ${server.name} (${server.ip})...`, 'success')
  minecraftStore.launchInstance(props.instance.id)
}

function confirmAddServer() {
  if (!newServerName.value.trim() || !newServerIp.value.trim()) return

  const newServer: MinecraftServer = {
    id: `srv-${Date.now()}`,
    name: newServerName.value.trim(),
    ip: newServerIp.value.trim(),
    motd: 'Nouveau serveur ajouté à servers.dat',
    pingMs: Math.floor(15 + Math.random() * 30),
    playersOnline: 0,
    playersMax: 50,
    status: 'online',
    version: props.instance.gameVersion
  }

  props.instance.servers.push(newServer)
  addServerModalOpen.value = false
  newServerName.value = ''
  newServerIp.value = ''
  toast.show(`Serveur ${newServer.name} ajouté avec succès !`, 'success')
}

function deleteServer(id: string) {
  const idx = props.instance.servers.findIndex((s) => s.id === id)
  if (idx !== -1) {
    const deleted = props.instance.servers.splice(idx, 1)[0]
    toast.show(`Serveur ${deleted.name} supprimé de l'instance.`, 'info')
  }
}

function formatMotd(motd?: string): string {
  if (!motd) return ''
  // Strip Minecraft color codes for clean reading
  return motd.replace(/§[0-9a-fk-or]/gi, '')
}
</script>

<template>
  <div class="space-y-6">
    <!-- Header / Actions Toolbar -->
    <div class="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4 p-5 rounded-2xl bg-zinc-950/60 border border-zinc-800/80 shadow-md">
      <div>
        <div class="flex items-center gap-2">
          <h3 class="font-bold text-sm text-zinc-100 flex items-center gap-2">
            <i class="ph-fill ph-hard-drives text-amber-400 text-lg"></i>
            Serveurs Récupérés (servers.dat)
          </h3>
          <UiBadge variant="amber" size="xs">
            {{ instance.servers.filter(s => s.status === 'online').length }}/{{ instance.servers.length }} En ligne
          </UiBadge>
        </div>
        <p class="text-xs text-zinc-400 mt-1">
          Liste des serveurs multijoueur synchronisés avec l'environnement isolé de cette instance.
        </p>
      </div>

      <div class="flex items-center gap-2.5">
        <UiButton
          variant="secondary"
          size="sm"
          :loading="isPinging"
          @click="refreshPings"
        >
          <template #prefix>
            <i class="ph ph-arrows-clockwise text-base"></i>
          </template>
          Rafraîchir Pings
        </UiButton>

        <UiButton
          variant="primary"
          size="sm"
          @click="addServerModalOpen = true"
        >
          <template #prefix>
            <i class="ph ph-plus text-base"></i>
          </template>
          Ajouter un Serveur
        </UiButton>
      </div>
    </div>

    <!-- Server Cards List -->
    <div class="space-y-3">
      <div
        v-for="server in instance.servers"
        :key="server.id"
        class="p-4 rounded-2xl bg-zinc-900/60 border border-zinc-800/80 hover:border-zinc-700/80 transition-all flex flex-col md:flex-row items-start md:items-center justify-between gap-4 group"
      >
        <!-- Server Identity & Favicon -->
        <div class="flex items-center gap-4 flex-1 min-w-0">
          <!-- Server Icon -->
          <div class="relative w-14 h-14 rounded-xl bg-zinc-950 border border-zinc-800 flex items-center justify-center flex-shrink-0 overflow-hidden shadow-inner">
            <img
              v-if="server.iconUrl"
              :src="server.iconUrl"
              :alt="server.name"
              class="w-full h-full object-cover"
            />
            <i v-else class="ph-fill ph-cube text-2xl text-amber-400"></i>

            <!-- Online / Offline Ping Status Dot -->
            <span
              :class="[
                'absolute bottom-1 right-1 w-2.5 h-2.5 rounded-full border-2 border-zinc-950',
                server.status === 'online' ? 'bg-emerald-500 animate-pulse' : 'bg-rose-500'
              ]"
            />
          </div>

          <!-- Server Info -->
          <div class="min-w-0 flex-1 space-y-1">
            <div class="flex items-center gap-2.5 flex-wrap">
              <h4 class="font-bold text-sm text-zinc-100 group-hover:text-amber-400 transition-colors truncate">
                {{ server.name }}
              </h4>
              <span
                v-if="server.version"
                class="text-[10px] font-mono px-2 py-0.5 rounded-full bg-zinc-800 text-zinc-300 border border-zinc-700/60"
              >
                {{ server.version }}
              </span>
            </div>

            <!-- MOTD -->
            <p class="text-xs text-zinc-400 line-clamp-1">
              {{ formatMotd(server.motd) }}
            </p>

            <!-- IP with Copy Button -->
            <div class="flex items-center gap-2 text-xs font-mono text-zinc-500 pt-0.5">
              <span class="text-zinc-300 font-semibold">{{ server.ip }}</span>
              <button
                type="button"
                @click="copyIp(server)"
                class="p-1 rounded hover:bg-zinc-800 text-zinc-400 hover:text-amber-400 transition-colors"
                title="Copier l'adresse IP"
              >
                <i :class="copiedServerId === server.id ? 'ph ph-check text-emerald-400' : 'ph ph-copy'"></i>
              </button>
            </div>
          </div>
        </div>

        <!-- Telemetry & Action Buttons -->
        <div class="flex items-center justify-between md:justify-end gap-5 w-full md:w-auto pt-2 md:pt-0 border-t md:border-t-0 border-zinc-800/60">
          <!-- Ping & Players Stats -->
          <div class="flex items-center gap-5 text-right font-mono text-xs">
            <!-- Ping -->
            <div class="space-y-0.5">
              <span class="text-[10px] text-zinc-500 uppercase block tracking-wider font-sans">Latence</span>
              <div class="flex items-center gap-1.5 font-bold">
                <i
                  :class="[
                    'ph text-sm',
                    server.status === 'online' ? 'ph-wifi-high text-emerald-400' : 'ph-wifi-slash text-rose-400'
                  ]"
                />
                <span :class="server.status === 'online' ? 'text-zinc-200' : 'text-zinc-500'">
                  {{ server.status === 'online' ? `${server.pingMs} ms` : 'Hors ligne' }}
                </span>
              </div>
            </div>

            <!-- Players -->
            <div class="space-y-0.5">
              <span class="text-[10px] text-zinc-500 uppercase block tracking-wider font-sans">Joueurs</span>
              <p class="font-bold text-zinc-200">
                <span class="text-amber-400">{{ server.playersOnline }}</span>
                <span class="text-zinc-500 font-normal"> / {{ server.playersMax }}</span>
              </p>
            </div>
          </div>

          <!-- Join Button -->
          <div class="flex items-center gap-2">
            <button
              type="button"
              @click="launchAndJoin(server)"
              :disabled="server.status !== 'online'"
              :class="[
                'px-4 py-2 rounded-xl font-bold text-xs flex items-center gap-2 transition-all shadow-sm',
                server.status === 'online'
                  ? 'btn-gold'
                  : 'bg-zinc-800 text-zinc-500 cursor-not-allowed'
              ]"
              title="Lancer l'instance et rejoindre ce serveur"
            >
              <i class="ph-fill ph-play text-sm"></i>
              Rejoindre
            </button>

            <!-- Delete server button -->
            <button
              type="button"
              @click="deleteServer(server.id)"
              class="p-2 rounded-xl text-zinc-500 hover:text-rose-400 hover:bg-zinc-800/80 transition-colors"
              title="Retirer le serveur de l'instance"
            >
              <i class="ph ph-trash text-base"></i>
            </button>
          </div>
        </div>
      </div>

      <!-- Empty State -->
      <div
        v-if="instance.servers.length === 0"
        class="p-12 text-center rounded-2xl bg-zinc-950/40 border border-zinc-800/60 space-y-3"
      >
        <div class="w-12 h-12 rounded-full bg-zinc-900 border border-zinc-800 flex items-center justify-center text-zinc-400 mx-auto">
          <i class="ph-fill ph-hard-drives text-xl"></i>
        </div>
        <p class="text-sm font-semibold text-zinc-300">
          Aucun serveur enregistré dans ce profil.
        </p>
        <p class="text-xs text-zinc-500 max-w-sm mx-auto">
          Ajoutez vos serveurs préférés ou lancez Minecraft pour que le fichier servers.dat se remplisse automatiquement.
        </p>
        <UiButton
          variant="primary"
          size="sm"
          @click="addServerModalOpen = true"
        >
          <template #prefix>
            <i class="ph ph-plus text-base"></i>
          </template>
          Ajouter un premier serveur
        </UiButton>
      </div>
    </div>

    <!-- MODALE : AJOUTER UN SERVEUR -->
    <UiModal
      :open="addServerModalOpen"
      title="Ajouter un Serveur à l'Instance"
      subtitle="Ce serveur sera inscrit dans le fichier servers.dat de l'instance"
      max-width="md"
      @close="addServerModalOpen = false"
    >
      <div class="space-y-4 py-2">
        <div>
          <label class="block text-xs font-semibold text-zinc-300 uppercase tracking-wider mb-1 font-mono">
            Nom du Serveur
          </label>
          <UiInput
            v-model="newServerName"
            placeholder="Ex: Serveur Survie Fabric"
          />
        </div>

        <div>
          <label class="block text-xs font-semibold text-zinc-300 uppercase tracking-wider mb-1 font-mono">
            Adresse IP / Hôte
          </label>
          <UiInput
            v-model="newServerIp"
            placeholder="Ex: play.monserveur.fr ou 192.168.1.50:25565"
          />
        </div>
      </div>

      <template #footer>
        <UiButton
          variant="ghost"
          size="sm"
          @click="addServerModalOpen = false"
        >
          Annuler
        </UiButton>
        <UiButton
          variant="primary"
          size="sm"
          @click="confirmAddServer"
        >
          Enregistrer le serveur
        </UiButton>
      </template>
    </UiModal>
  </div>
</template>
