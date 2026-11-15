import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type {
  SystemMetricsSnapshot,
  SystemSummary,
  ProcessSnapshot,
  GpuProcessInfo,
  SmartDiskInfo,
  DnfUpdateStatus
} from '@/models/monitoring'
import { ipc } from '@/services/ipc'

export interface MonitorHistory {
  cpuTotal: number[]
  ram: number[]
  gpu: number[]
  downloadMb: number[]
  uploadMb: number[]
  diskReadMb: number[]
  diskWriteMb: number[]
  timestamps: number[]
}

const MAX_HISTORY_POINTS = 60

export function getMetricColorClass(percent: number): {
  text: string
  bg: string
  border: string
  hex: string
} {
  if (percent >= 90) {
    return {
      text: 'text-rose-400',
      bg: 'bg-rose-500/15',
      border: 'border-rose-500/30',
      hex: '#f43f5e'
    }
  }
  if (percent >= 75) {
    return {
      text: 'text-amber-400',
      bg: 'bg-amber-500/15',
      border: 'border-amber-500/30',
      hex: '#f59e0b'
    }
  }
  return {
    text: 'text-emerald-400',
    bg: 'bg-emerald-500/15',
    border: 'border-emerald-500/30',
    hex: '#10b981'
  }
}

export const useSystemMonitorStore = defineStore('systemMonitor', () => {
  const metrics = ref<SystemMetricsSnapshot | null>(null)
  const summary = ref<SystemSummary | null>(null)
  const isLoading = ref(true)
  const isRefreshing = ref(false)
  const isPaused = ref(false)
  const error = ref<string | null>(null)

  // Polling settings
  const refreshInterval = ref<number>(1000) // ms
  let pollTimer: ReturnType<typeof setInterval> | null = null

  // 60-second sliding history buffers
  const history = ref<MonitorHistory>({
    cpuTotal: [],
    ram: [],
    gpu: [],
    downloadMb: [],
    uploadMb: [],
    diskReadMb: [],
    diskWriteMb: [],
    timestamps: []
  })

  const processFilter = ref<string>('')
  const processSortField = ref<'cpu' | 'memory' | 'name' | 'pid'>('cpu')
  const processSortDirection = ref<'asc' | 'desc'>('desc')

  // Sub-navigation tabs
  const currentTab = ref<'overview' | 'gpu-procs' | 'all-procs' | 'smart' | 'dnf'>('overview')

  // Additional monitoring datasets
  const gpuProcesses = ref<GpuProcessInfo[]>([])
  const allProcesses = ref<ProcessSnapshot[]>([])
  const smartDisks = ref<SmartDiskInfo[]>([])
  const dnfStatus = ref<DnfUpdateStatus | null>(null)

  const isGpuLoading = ref(false)
  const isAllProcsLoading = ref(false)
  const isSmartLoading = ref(false)
  const isDnfLoading = ref(false)

  function pushHistoryPoint(snapshot: SystemMetricsSnapshot) {
    const pushFifo = (arr: number[], val: number) => {
      arr.push(val)
      if (arr.length > MAX_HISTORY_POINTS) {
        arr.shift()
      }
    }

    pushFifo(history.value.cpuTotal, Math.round(snapshot.cpu.globalUsage * 10) / 10)
    pushFifo(history.value.ram, Math.round(snapshot.memory.usagePercent * 10) / 10)
    pushFifo(
      history.value.gpu,
      snapshot.gpu?.usagePercent != null ? Math.round(snapshot.gpu.usagePercent * 10) / 10 : 0
    )

    const dlMb = Math.round((snapshot.network.rxBytesPerSec / (1024 * 1024)) * 100) / 100
    const ulMb = Math.round((snapshot.network.txBytesPerSec / (1024 * 1024)) * 100) / 100
    pushFifo(history.value.downloadMb, dlMb)
    pushFifo(history.value.uploadMb, ulMb)

    const rDiskMb = Math.round((snapshot.disks.readBytesPerSec / (1024 * 1024)) * 100) / 100
    const wDiskMb = Math.round((snapshot.disks.writeBytesPerSec / (1024 * 1024)) * 100) / 100
    pushFifo(history.value.diskReadMb, rDiskMb)
    pushFifo(history.value.diskWriteMb, wDiskMb)

    pushFifo(history.value.timestamps, snapshot.timestampMs)
  }

  async function fetchStaticInfo() {
    try {
      summary.value = await ipc.getSystemStaticInfo()
    } catch (err: any) {
      console.warn('[MonitorStore] Failed to fetch static summary:', err)
    }
  }

  async function fetchMetrics() {
    if (isPaused.value || isRefreshing.value) return
    isRefreshing.value = true
    try {
      const snap = await ipc.getSystemMetrics()
      metrics.value = snap
      pushHistoryPoint(snap)
      error.value = null
    } catch (err: any) {
      console.error('[MonitorStore] Error fetching system metrics:', err)
      error.value = err?.message || String(err)
    } finally {
      isRefreshing.value = false
      isLoading.value = false
    }
  }

  function startPolling() {
    stopPolling()
    // Initial fetches
    fetchStaticInfo()
    fetchMetrics()

    pollTimer = setInterval(() => {
      fetchMetrics()
    }, refreshInterval.value)
  }

  function stopPolling() {
    if (pollTimer) {
      clearInterval(pollTimer)
      pollTimer = null
    }
  }

  function setRefreshRate(ms: number) {
    refreshInterval.value = ms
    if (pollTimer) {
      startPolling()
    }
  }

  function togglePause() {
    isPaused.value = !isPaused.value
    if (!isPaused.value) {
      fetchMetrics()
    }
  }

  async function killProcess(pid: number, force: boolean = false): Promise<void> {
    try {
      await ipc.killProcess(pid, force)
      // Remove process locally or refresh immediately
      if (metrics.value) {
        metrics.value.topProcesses = metrics.value.topProcesses.filter((p) => p.pid !== pid)
      }
      setTimeout(() => {
        fetchMetrics()
      }, 300)
    } catch (err: any) {
      throw new Error(err?.message || String(err))
    }
  }

  function setProcessFilter(query: string) {
    processFilter.value = query
  }

  function setProcessSorting(field: 'cpu' | 'memory' | 'name' | 'pid') {
    if (processSortField.value === field) {
      processSortDirection.value = processSortDirection.value === 'asc' ? 'desc' : 'asc'
    } else {
      processSortField.value = field
      processSortDirection.value = 'desc'
    }
  }

  async function fetchGpuProcesses() {
    isGpuLoading.value = true
    try {
      gpuProcesses.value = await ipc.getGpuProcesses()
    } catch (err: any) {
      console.warn('[MonitorStore] Failed to fetch GPU processes:', err)
    } finally {
      isGpuLoading.value = false
    }
  }

  async function fetchAllProcesses() {
    isAllProcsLoading.value = true
    try {
      allProcesses.value = await ipc.getAllSystemProcesses()
    } catch (err: any) {
      console.warn('[MonitorStore] Failed to fetch all processes:', err)
    } finally {
      isAllProcsLoading.value = false
    }
  }

  async function fetchSmartDisks() {
    isSmartLoading.value = true
    try {
      smartDisks.value = await ipc.getSmartDisksInfo()
    } catch (err: any) {
      console.warn('[MonitorStore] Failed to fetch SMART disks:', err)
    } finally {
      isSmartLoading.value = false
    }
  }

  async function fetchDnfUpdates(refresh: boolean = false) {
    isDnfLoading.value = true
    try {
      dnfStatus.value = await ipc.checkDnfUpdates(refresh)
    } catch (err: any) {
      console.warn('[MonitorStore] Failed to check DNF updates:', err)
    } finally {
      isDnfLoading.value = false
    }
  }

  function setTab(tab: 'overview' | 'gpu-procs' | 'all-procs' | 'smart' | 'dnf') {
    currentTab.value = tab
    if (tab === 'gpu-procs') fetchGpuProcesses()
    if (tab === 'all-procs') fetchAllProcesses()
    if (tab === 'smart') fetchSmartDisks()
    if (tab === 'dnf') fetchDnfUpdates()
  }

  // Filtered and sorted processes
  const filteredProcesses = computed<ProcessSnapshot[]>(() => {
    if (!metrics.value?.topProcesses) return []
    let list = [...metrics.value.topProcesses]

    const q = processFilter.value.trim().toLowerCase()
    if (q) {
      list = list.filter(
        (p) =>
          p.name.toLowerCase().includes(q) ||
          p.command.toLowerCase().includes(q) ||
          p.pid.toString().includes(q) ||
          (p.user && p.user.toLowerCase().includes(q))
      )
    }

    const dir = processSortDirection.value === 'asc' ? 1 : -1
    const field = processSortField.value

    list.sort((a, b) => {
      if (field === 'cpu') {
        return (a.cpuUsage - b.cpuUsage) * dir
      }
      if (field === 'memory') {
        return (a.memoryBytes - b.memoryBytes) * dir
      }
      if (field === 'pid') {
        return (a.pid - b.pid) * dir
      }
      return a.name.localeCompare(b.name) * dir
    })

    return list
  })

  // Color indicators for current usage
  const cpuStatus = computed(() => getMetricColorClass(metrics.value?.cpu.globalUsage ?? 0))
  const ramStatus = computed(() => getMetricColorClass(metrics.value?.memory.usagePercent ?? 0))
  const gpuStatus = computed(() => getMetricColorClass(metrics.value?.gpu?.usagePercent ?? 0))

  return {
    metrics,
    summary,
    isLoading,
    isRefreshing,
    isPaused,
    error,
    refreshInterval,
    history,
    processFilter,
    processSortField,
    processSortDirection,
    filteredProcesses,
    cpuStatus,
    ramStatus,
    gpuStatus,
    currentTab,
    gpuProcesses,
    allProcesses,
    smartDisks,
    dnfStatus,
    isGpuLoading,
    isAllProcsLoading,
    isSmartLoading,
    isDnfLoading,
    setTab,
    fetchGpuProcesses,
    fetchAllProcesses,
    fetchSmartDisks,
    fetchDnfUpdates,
    startPolling,
    stopPolling,
    setRefreshRate,
    togglePause,
    fetchStaticInfo,
    fetchMetrics,
    killProcess,
    setProcessFilter,
    setProcessSorting
  }
})
