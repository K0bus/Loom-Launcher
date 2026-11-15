<script setup lang="ts">
import { computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import {
  LayoutDashboard,
  Gamepad2,
  UserCheck,
  Settings,
  ArrowDownUp,
  Bell,
  Cpu,
  Layers,
  Activity
} from 'lucide-vue-next'
import { useSystemStore } from '@/stores/systemStore'
import UiBadge from '@/components/ui/UiBadge.vue'

const route = useRoute()
const router = useRouter()
const systemStore = useSystemStore()

const navItems = [
  { name: 'Dashboard', path: '/', icon: LayoutDashboard },
  { name: 'Bibliothèque', path: '/library', icon: Gamepad2 },
  { name: 'Modding', path: '/modding', icon: Layers, badge: 'PRO' },
  { name: 'Monitoring', path: '/monitoring', icon: Activity },
  { name: 'Comptes', path: '/accounts', icon: UserCheck },
  { name: 'Paramètres', path: '/settings', icon: Settings },
]

function isActive(path: string) {
  if (path === '/') return route.path === '/'
  return route.path.startsWith(path)
}

const networkFormatted = computed(() => {
  const down = (systemStore.stats.networkDownKbps / 1024).toFixed(1)
  return `${down} Mo/s`
})
</script>

<template>
  <aside class="w-64 h-screen flex flex-col bg-zinc-950 border-r border-zinc-800/80 select-none z-30">
    <!-- Brand Header -->
    <div class="h-20 flex items-center px-6 gap-3.5 border-b border-zinc-800/80">
      <div class="relative flex items-center justify-center w-10 h-10 rounded-xl bg-zinc-900 border border-zinc-700/80 shadow-md">
        <img src="/logo.png" alt="Loom" class="w-7 h-7 object-contain drop-shadow-[0_0_10px_rgba(245,158,11,0.4)]" />
      </div>
      <div>
        <div class="flex items-center gap-1.5">
          <span class="font-extrabold text-lg tracking-tight text-zinc-100 font-display">LOOM</span>
          <span class="text-xs font-bold text-amber-400 bg-amber-500/10 px-1.5 py-0.5 rounded border border-amber-500/20">V2</span>
        </div>
        <p class="text-[11px] text-zinc-500 font-medium">Universal Game Hub</p>
      </div>
    </div>

    <!-- Navigation List -->
    <nav class="flex-1 px-3 py-6 space-y-1.5 overflow-y-auto">
      <div class="px-3 pb-2 text-[10px] font-bold uppercase tracking-wider text-zinc-500 font-mono">
        Menu Principal
      </div>
      
      <button
        v-for="item in navItems"
        :key="item.path"
        type="button"
        @click="router.push(item.path)"
        :class="[
          'w-full flex items-center justify-between px-3.5 py-2.5 rounded-xl text-sm font-medium transition-all group',
          isActive(item.path)
            ? 'bg-amber-500/10 text-amber-400 font-semibold border border-amber-500/30 shadow-sm shadow-amber-500/5'
            : 'text-zinc-400 hover:text-zinc-100 hover:bg-zinc-900/80 border border-transparent'
        ]"
      >
        <div class="flex items-center gap-3">
          <component
            :is="item.icon"
            :class="[
              'w-5 h-5 transition-colors',
              isActive(item.path)
                ? 'text-amber-400'
                : 'text-zinc-500 group-hover:text-zinc-200'
            ]"
          />
          <span>{{ item.name }}</span>
        </div>

        <UiBadge
          v-if="item.badge"
          variant="amber"
          size="xs"
        >
          {{ item.badge }}
        </UiBadge>
      </button>
    </nav>

    <!-- Bottom System Health & Status Footer -->
    <div class="p-4 border-t border-zinc-800/80 bg-zinc-950/80 space-y-3">
      <!-- Network throughput & CPU -->
      <div
        @click="router.push('/monitoring')"
        class="p-2.5 rounded-xl bg-zinc-900/90 hover:bg-zinc-900 border border-zinc-800/80 hover:border-amber-500/30 text-xs space-y-2 cursor-pointer transition-all group"
        title="Ouvrir le Monitoring Système complet"
      >
        <div class="flex items-center justify-between text-zinc-400">
          <span class="flex items-center gap-1.5">
            <ArrowDownUp class="w-3.5 h-3.5 text-amber-400 group-hover:scale-110 transition-transform" />
            <span class="group-hover:text-zinc-200 transition-colors">Débit Réseau</span>
          </span>
          <span class="font-mono text-zinc-200 font-medium">{{ networkFormatted }}</span>
        </div>

        <div class="flex items-center justify-between text-zinc-400">
          <span class="flex items-center gap-1.5">
            <Cpu class="w-3.5 h-3.5 text-zinc-400 group-hover:text-amber-400 transition-colors" />
            <span class="group-hover:text-zinc-200 transition-colors">CPU Charge</span>
          </span>
          <span class="font-mono text-zinc-200 font-medium">{{ systemStore.stats.cpuUsage.toFixed(1) }}%</span>
        </div>
      </div>

      <!-- Footer Info -->
      <div class="flex items-center justify-between px-1 text-[11px] text-zinc-500">
        <div class="flex items-center gap-1.5">
          <span
            :class="[
              'w-2 h-2 rounded-full',
              systemStore.stats.tauriDetected ? 'bg-emerald-400 animate-pulse' : 'bg-amber-400'
            ]"
            :title="systemStore.stats.tauriDetected ? 'Tauri v2 Core Actif' : 'Mode Simulé IPC Web'"
          />
          <span class="font-mono text-[10px] text-zinc-400 truncate max-w-[110px]">
            {{ systemStore.stats.tauriDetected ? 'Tauri Runtime' : 'IPC Web Emul' }}
          </span>
        </div>

        <div class="flex items-center gap-2">
          <button
            type="button"
            class="relative p-1 text-zinc-400 hover:text-zinc-200 transition-colors"
            title="Notifications"
          >
            <Bell class="w-3.5 h-3.5" />
            <span
              v-if="systemStore.stats.notificationsCount > 0"
              class="absolute top-0 right-0 w-1.5 h-1.5 bg-amber-400 rounded-full"
            />
          </button>
          <span class="font-mono text-[10px] text-zinc-500">v0.1.0</span>
        </div>
      </div>
    </div>
  </aside>
</template>
