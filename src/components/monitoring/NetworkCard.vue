<script setup lang="ts">
import { computed } from 'vue'
import {
  Wifi,
  Network,
  ArrowDownCircle,
  ArrowUpCircle,
  Globe,
  Cable,
  Activity,
  CheckCircle2,
  XCircle,
  Radio,
  Gauge,
  Server,
  Layers
} from 'lucide-vue-next'
import type { NetworkSnapshot } from '@/models/monitoring'
import SparklineChart from './SparklineChart.vue'

interface Props {
  network: NetworkSnapshot
  downloadHistory?: number[]
  uploadHistory?: number[]
}

const props = defineProps<Props>()

function bytesToThroughput(bytes: number): string {
  const mb = bytes / (1024 * 1024)
  if (mb >= 10) return `${mb.toFixed(1)} Mo/s`
  if (mb >= 1) return `${mb.toFixed(2)} Mo/s`
  const kb = bytes / 1024
  if (kb >= 1) return `${kb.toFixed(1)} Ko/s`
  return `${bytes} B/s`
}

function formatTotalBytes(bytes: number): string {
  const gb = bytes / (1024 * 1024 * 1024)
  if (gb >= 1) return `${gb.toFixed(2)} Go`
  const mb = bytes / (1024 * 1024)
  if (mb >= 1) return `${mb.toFixed(1)} Mo`
  const kb = bytes / 1024
  return `${kb.toFixed(0)} Ko`
}

function formatSpeed(mbps?: number): string {
  if (!mbps) return '--'
  if (mbps >= 1000) {
    const gbps = mbps / 1000
    return `${gbps % 1 === 0 ? gbps.toFixed(0) : gbps.toFixed(1)} Gbps`
  }
  return `${mbps} Mbps`
}

const primary = computed(() => props.network.primaryInterface)

const downloadSpeed = computed(() => {
  const bps = primary.value ? primary.value.rxBytesPerSec : props.network.rxBytesPerSec
  return bytesToThroughput(bps)
})

const uploadSpeed = computed(() => {
  const bps = primary.value ? primary.value.txBytesPerSec : props.network.txBytesPerSec
  return bytesToThroughput(bps)
})

const connectivityBadge = computed(() => {
  const status = primary.value?.connectivity
  if (status === 'full') {
    return {
      text: 'Internet Plein Accès',
      bg: 'bg-emerald-500/15',
      border: 'border-emerald-500/30',
      textCol: 'text-emerald-400'
    }
  }
  if (status === 'limited') {
    return {
      text: 'Réseau Local Uniquement',
      bg: 'bg-amber-500/15',
      border: 'border-amber-500/30',
      textCol: 'text-amber-400'
    }
  }
  if (status === 'none') {
    return {
      text: 'Déconnecté',
      bg: 'bg-rose-500/15',
      border: 'border-rose-500/30',
      textCol: 'text-rose-400'
    }
  }
  return {
    text: primary.value?.isUp ? 'Liaison Active' : 'Déconnecté',
    bg: primary.value?.isUp ? 'bg-emerald-500/15' : 'bg-rose-500/15',
    border: primary.value?.isUp ? 'border-emerald-500/30' : 'border-rose-500/30',
    textCol: primary.value?.isUp ? 'text-emerald-400' : 'text-rose-400'
  }
})

// Other interfaces excluding loopback and primary
const secondaryInterfaces = computed(() => {
  const primaryName = primary.value?.interfaceName
  return props.network.interfaces.filter((i) => i.name !== primaryName && i.name !== 'lo')
})
</script>

<template>
  <div class="space-y-6">
    <!-- ========================================== -->
    <!-- CATÉGORIE 1: INTERFACE RÉSEAU PRINCIPALE   -->
    <!-- ========================================== -->
    <div class="p-6 rounded-2xl bg-zinc-900/90 border border-amber-500/30 shadow-xl shadow-amber-500/5 relative overflow-hidden">
      <!-- Glow ambient background effect -->
      <div class="absolute -top-12 -right-12 w-64 h-64 bg-amber-500/5 rounded-full blur-3xl pointer-events-none" />

      <!-- Section Title & Status Header -->
      <div class="flex flex-col lg:flex-row lg:items-center justify-between gap-4 pb-5 border-b border-zinc-800/80 relative">
        <div class="flex items-start sm:items-center gap-3.5">
          <div class="p-3 rounded-2xl bg-amber-500/15 border border-amber-500/30 text-amber-400 shrink-0">
            <component :is="primary?.isWireless ? Wifi : (primary?.interfaceType === 'ethernet' ? Cable : Globe)" class="w-7 h-7" />
          </div>

          <div>
            <div class="flex flex-wrap items-center gap-2">
              <span class="px-2 py-0.5 rounded-full text-[10px] font-mono font-extrabold uppercase bg-amber-500 text-zinc-950 shadow-sm">
                Réseau Principal
              </span>
              <h3 class="text-lg font-bold text-zinc-100 font-mono">
                {{ primary?.interfaceName || 'Interface par défaut' }}
              </h3>
              <span
                v-if="primary?.connectionName"
                class="text-xs font-medium text-zinc-400 bg-zinc-800/90 px-2 py-0.5 rounded-md border border-zinc-700/60"
              >
                {{ primary.connectionName }}
              </span>
            </div>

            <!-- Hardware / Controller model subtitle -->
            <p class="text-xs text-zinc-400 mt-1 flex flex-wrap items-center gap-x-2">
              <span v-if="primary?.product" class="text-zinc-200 font-medium">
                {{ primary.vendor ? `${primary.vendor} ` : '' }}{{ primary.product }}
              </span>
              <span v-if="primary?.driver" class="text-zinc-500 font-mono text-[11px]">
                (Pilote : {{ primary.driver }})
              </span>
              <span v-if="!primary?.product && !primary?.driver" class="text-zinc-400">
                Liaison réseau active
              </span>
            </p>
          </div>
        </div>

        <!-- Right Side: Connectivity & Latency Badges -->
        <div class="flex flex-wrap items-center gap-3">
          <!-- Latency Badge -->
          <div
            v-if="primary?.latencyMs != null"
            class="flex items-center gap-2 px-3 py-1.5 rounded-xl bg-zinc-950/80 border border-zinc-800 font-mono text-xs"
            title="Latence Ping vers la passerelle ou DNS"
          >
            <Activity class="w-3.5 h-3.5 text-amber-400" />
            <span class="text-zinc-400">Ping :</span>
            <span
              class="font-bold"
              :class="primary.latencyMs < 10 ? 'text-emerald-400' : (primary.latencyMs < 50 ? 'text-amber-400' : 'text-rose-400')"
            >
              {{ primary.latencyMs.toFixed(1) }} ms
            </span>
          </div>

          <!-- Internet / Connectivity Badge -->
          <div
            class="flex items-center gap-2 px-3.5 py-1.5 rounded-xl border text-xs font-mono font-bold"
            :class="[connectivityBadge.bg, connectivityBadge.border, connectivityBadge.textCol]"
          >
            <component :is="primary?.isUp ? CheckCircle2 : XCircle" class="w-4 h-4" />
            <span>{{ connectivityBadge.text }}</span>
          </div>
        </div>
      </div>

      <!-- Live Bandwidth & Cumulative Transfer Badges -->
      <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-3.5 pt-5 pb-3">
        <!-- Download Throughput Instant -->
        <div class="p-4 rounded-xl bg-zinc-950/70 border border-emerald-500/20 shadow-sm flex items-center justify-between">
          <div>
            <span class="text-[10px] uppercase tracking-wider text-zinc-500 font-mono font-bold block mb-1">
              Débit Réception (DL)
            </span>
            <span class="font-mono text-lg font-extrabold text-emerald-400">
              {{ downloadSpeed }}
            </span>
          </div>
          <div class="p-2.5 rounded-xl bg-emerald-500/10 text-emerald-400 border border-emerald-500/20">
            <ArrowDownCircle class="w-6 h-6" />
          </div>
        </div>

        <!-- Upload Throughput Instant -->
        <div class="p-4 rounded-xl bg-zinc-950/70 border border-sky-500/20 shadow-sm flex items-center justify-between">
          <div>
            <span class="text-[10px] uppercase tracking-wider text-zinc-500 font-mono font-bold block mb-1">
              Débit Envoi (UL)
            </span>
            <span class="font-mono text-lg font-extrabold text-sky-400">
              {{ uploadSpeed }}
            </span>
          </div>
          <div class="p-2.5 rounded-xl bg-sky-500/10 text-sky-400 border border-sky-500/20">
            <ArrowUpCircle class="w-6 h-6" />
          </div>
        </div>

        <!-- Total Volume Downloaded -->
        <div class="p-4 rounded-xl bg-zinc-950/60 border border-zinc-800 shadow-sm">
          <span class="text-[10px] uppercase tracking-wider text-zinc-500 font-mono font-bold block mb-1">
            Total Téléchargé (Session)
          </span>
          <p class="font-mono text-base font-bold text-zinc-200">
            {{ formatTotalBytes(primary?.totalRxBytes || 0) }}
          </p>
        </div>

        <!-- Total Volume Uploaded -->
        <div class="p-4 rounded-xl bg-zinc-950/60 border border-zinc-800 shadow-sm">
          <span class="text-[10px] uppercase tracking-wider text-zinc-500 font-mono font-bold block mb-1">
            Total Envoyé (Session)
          </span>
          <p class="font-mono text-base font-bold text-zinc-200">
            {{ formatTotalBytes(primary?.totalTxBytes || 0) }}
          </p>
        </div>
      </div>

      <!-- Real-Time Traffic History Charts (Sparklines) -->
      <div class="grid grid-cols-1 md:grid-cols-2 gap-4 py-2">
        <!-- Download Sparkline -->
        <div class="p-3.5 rounded-xl bg-zinc-950/60 border border-zinc-800/80">
          <div class="flex items-center justify-between text-xs font-mono mb-1.5">
            <span class="text-zinc-400 flex items-center gap-1.5 font-semibold">
              <ArrowDownCircle class="w-3.5 h-3.5 text-emerald-400" />
              <span>Historique Réception (60s)</span>
            </span>
            <span class="font-bold text-emerald-400">{{ downloadSpeed }}</span>
          </div>
          <SparklineChart
            :data="downloadHistory || [0]"
            color="emerald"
            :height="55"
            unit="Mo/s"
          />
        </div>

        <!-- Upload Sparkline -->
        <div class="p-3.5 rounded-xl bg-zinc-950/60 border border-zinc-800/80">
          <div class="flex items-center justify-between text-xs font-mono mb-1.5">
            <span class="text-zinc-400 flex items-center gap-1.5 font-semibold">
              <ArrowUpCircle class="w-3.5 h-3.5 text-sky-400" />
              <span>Historique Envoi (60s)</span>
            </span>
            <span class="font-bold text-sky-400">{{ uploadSpeed }}</span>
          </div>
          <SparklineChart
            :data="uploadHistory || [0]"
            color="sky"
            :height="55"
            unit="Mo/s"
          />
        </div>
      </div>

      <!-- Detailed Technical & Connectivity Properties Grid -->
      <div class="pt-4 border-t border-zinc-800/80 space-y-4">
        <h4 class="text-xs font-bold uppercase tracking-wider font-mono text-amber-400/90 flex items-center gap-1.5">
          <Layers class="w-3.5 h-3.5" />
          <span>Propriétés de Connexion & Liaison Matérielle</span>
        </h4>

        <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-3 text-xs font-mono">
          <!-- IPv4 Address -->
          <div class="p-3 rounded-xl bg-zinc-950/60 border border-zinc-800/80">
            <span class="text-[10px] text-zinc-500 uppercase block mb-1">Adresse IPv4</span>
            <p class="font-bold text-zinc-100 truncate">
              {{ primary?.ipAddress || network.localIp || 'Non assignée' }}
            </p>
          </div>

          <!-- Gateway -->
          <div class="p-3 rounded-xl bg-zinc-950/60 border border-zinc-800/80">
            <span class="text-[10px] text-zinc-500 uppercase block mb-1">Passerelle (Gateway)</span>
            <p class="font-bold text-amber-400/90 truncate">
              {{ primary?.gateway || network.defaultGateway || 'Non configurée' }}
            </p>
          </div>

          <!-- DNS Servers -->
          <div class="p-3 rounded-xl bg-zinc-950/60 border border-zinc-800/80">
            <span class="text-[10px] text-zinc-500 uppercase block mb-1 flex items-center justify-between">
              <span>Serveurs DNS</span>
              <Server class="w-3 h-3 text-zinc-500" />
            </span>
            <p class="font-bold text-zinc-200 truncate" :title="primary?.dnsServers?.join(', ')">
              {{ primary?.dnsServers && primary.dnsServers.length > 0 ? primary.dnsServers.join(', ') : 'Par défaut (DHCP)' }}
            </p>
          </div>

          <!-- MAC Address -->
          <div class="p-3 rounded-xl bg-zinc-950/60 border border-zinc-800/80">
            <span class="text-[10px] text-zinc-500 uppercase block mb-1">Adresse MAC</span>
            <p class="font-bold text-zinc-300 truncate">
              {{ primary?.macAddress || 'Non communiquée' }}
            </p>
          </div>

          <!-- Link Speed (negotiated) -->
          <div class="p-3 rounded-xl bg-zinc-950/60 border border-zinc-800/80">
            <span class="text-[10px] text-zinc-500 uppercase block mb-1 flex items-center justify-between">
              <span>Vitesse de Liaison</span>
              <Gauge class="w-3 h-3 text-amber-400" />
            </span>
            <p class="font-bold text-zinc-100">
              {{ formatSpeed(primary?.speedMbps) }}
              <span v-if="primary?.duplex" class="text-[10px] text-zinc-400 font-normal uppercase ml-1">
                ({{ primary.duplex }})
              </span>
            </p>
          </div>

          <!-- MTU -->
          <div class="p-3 rounded-xl bg-zinc-950/60 border border-zinc-800/80">
            <span class="text-[10px] text-zinc-500 uppercase block mb-1">Taille MTU</span>
            <p class="font-bold text-zinc-200">
              {{ primary?.mtu ? `${primary.mtu} octets` : '1500 octets' }}
            </p>
          </div>

          <!-- IPv6 Address -->
          <div class="p-3 rounded-xl bg-zinc-950/60 border border-zinc-800/80">
            <span class="text-[10px] text-zinc-500 uppercase block mb-1">Adresse IPv6</span>
            <p class="font-bold text-zinc-300 truncate" :title="primary?.ipv6Address">
              {{ primary?.ipv6Address || 'Désactivée / Non routée' }}
            </p>
          </div>

          <!-- Packet Health / Loss -->
          <div class="p-3 rounded-xl bg-zinc-950/60 border border-zinc-800/80">
            <span class="text-[10px] text-zinc-500 uppercase block mb-1">Pertes / Erreurs Paquets</span>
            <p
              class="font-bold"
              :class="(primary?.rxErrors || 0) + (primary?.txErrors || 0) > 0 ? 'text-rose-400' : 'text-emerald-400'"
            >
              {{ (primary?.rxErrors || 0) + (primary?.txErrors || 0) }} err
              <span class="text-[10px] text-zinc-500 font-normal ml-1">
                ({{ (primary?.rxDropped || 0) + (primary?.txDropped || 0) }} rejetés)
              </span>
            </p>
          </div>
        </div>

        <!-- Optional Wi-Fi Details Bar (if wireless) -->
        <div
          v-if="primary?.isWireless || network.wifiSsid"
          class="p-4 rounded-xl bg-amber-500/10 border border-amber-500/20 text-xs font-mono flex flex-wrap items-center justify-between gap-4"
        >
          <div class="flex items-center gap-3">
            <Radio class="w-5 h-5 text-amber-400" />
            <div>
              <span class="text-zinc-400 text-[11px] block">Réseau Wi-Fi Connecté</span>
              <p class="font-bold text-amber-300 text-sm">
                {{ primary?.wifiSsid || network.wifiSsid || 'Réseau sans fil' }}
              </p>
            </div>
          </div>

          <div class="flex items-center gap-6">
            <div v-if="primary?.wifiFrequencyMhz">
              <span class="text-zinc-400 text-[11px] block">Fréquence</span>
              <p class="font-bold text-zinc-200">
                {{ (primary.wifiFrequencyMhz / 1000).toFixed(1) }} GHz
              </p>
            </div>

            <div v-if="primary?.wifiSignalDbm != null || network.wifiSignalDbm != null">
              <span class="text-zinc-400 text-[11px] block">Puissance Signal</span>
              <p class="font-bold text-zinc-200">
                {{ primary?.wifiSignalDbm ?? network.wifiSignalDbm }} dBm
              </p>
            </div>

            <div v-if="primary?.wifiSignalPercent != null || network.wifiSignalPercent != null">
              <span class="text-zinc-400 text-[11px] block">Qualité Réception</span>
              <div class="flex items-center gap-2">
                <div class="w-20 bg-zinc-800 rounded-full h-2 overflow-hidden">
                  <div
                    class="bg-amber-400 h-2 rounded-full transition-all"
                    :style="{ width: `${primary?.wifiSignalPercent ?? network.wifiSignalPercent}%` }"
                  />
                </div>
                <span class="font-bold text-amber-400">
                  {{ primary?.wifiSignalPercent ?? network.wifiSignalPercent }}%
                </span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- ======================================================= -->
    <!-- CATÉGORIE 2: AUTRES INTERFACES RÉSEAU (SECONDAIRES)    -->
    <!-- ======================================================= -->
    <div class="p-6 rounded-2xl bg-zinc-900/90 border border-zinc-800 shadow-xl space-y-4">
      <div class="flex items-center justify-between pb-3 border-b border-zinc-800/80">
        <div class="flex items-center gap-2.5">
          <div class="p-2 rounded-xl bg-zinc-800 text-zinc-400">
            <Network class="w-5 h-5" />
          </div>
          <div>
            <h4 class="text-sm font-bold text-zinc-200">Autres Interfaces Réseau</h4>
            <p class="text-xs text-zinc-500">Périphériques secondaires, conteneurs et interfaces virtuelles</p>
          </div>
        </div>
        <span class="text-xs font-mono text-zinc-500 bg-zinc-950 px-2.5 py-1 rounded-lg border border-zinc-800">
          {{ secondaryInterfaces.length }} interface{{ secondaryInterfaces.length > 1 ? 's' : '' }}
        </span>
      </div>

      <div v-if="secondaryInterfaces.length === 0" class="text-center py-6 text-xs font-mono text-zinc-500">
        Aucune autre interface réseau active détectée sur ce système.
      </div>

      <div v-else class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
        <div
          v-for="iface in secondaryInterfaces"
          :key="iface.name"
          class="p-3.5 rounded-xl bg-zinc-950/60 border border-zinc-800/80 hover:border-zinc-700/80 transition-colors flex items-center justify-between"
        >
          <div class="flex items-center gap-2.5">
            <div
              class="w-2.5 h-2.5 rounded-full shrink-0"
              :class="iface.isUp ? 'bg-emerald-400' : 'bg-zinc-600'"
            />
            <div class="min-w-0">
              <div class="flex items-center gap-1.5">
                <span class="font-mono font-bold text-xs text-zinc-200 truncate">{{ iface.name }}</span>
                <span
                  v-if="iface.isWireless"
                  class="text-[9px] uppercase font-mono px-1 py-0.5 rounded bg-amber-500/10 text-amber-400 border border-amber-500/20"
                >
                  Wi-Fi
                </span>
                <span
                  v-else-if="iface.name.startsWith('docker') || iface.name.startsWith('br-') || iface.name.startsWith('veth')"
                  class="text-[9px] uppercase font-mono px-1 py-0.5 rounded bg-blue-500/10 text-blue-400 border border-blue-500/20"
                >
                  Virtuel
                </span>
              </div>
              <p class="text-[10px] font-mono text-zinc-500 truncate max-w-[150px]">
                {{ iface.ipAddress || iface.macAddress || (iface.isUp ? 'Actif (Sans IP)' : 'Inactif') }}
              </p>
            </div>
          </div>

          <div class="text-right font-mono text-[10px] text-zinc-400 space-y-0.5 shrink-0 ml-2">
            <div class="text-emerald-400 font-semibold">↓ {{ bytesToThroughput(iface.rxBytesPerSec) }}</div>
            <div class="text-sky-400 font-semibold">↑ {{ bytesToThroughput(iface.txBytesPerSec) }}</div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
