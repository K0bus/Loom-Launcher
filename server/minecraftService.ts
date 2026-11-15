import fs from 'fs'
import path from 'path'
import os from 'os'
import crypto from 'crypto'
import https from 'https'
import net from 'net'
import zlib from 'zlib'
import { spawn, execSync, type ChildProcess } from 'child_process'
import type { IncomingMessage, ServerResponse } from 'http'
import { marked } from 'marked'

function httpsGetJson(urlStr: string, headers: Record<string, string> = {}): Promise<any> {
  return new Promise((resolve, reject) => {
    const url = new URL(urlStr)
    const req = https.request(
      {
        hostname: url.hostname,
        port: 443,
        path: url.pathname + url.search,
        method: 'GET',
        family: 4,
        headers: {
          Accept: 'application/json',
          'User-Agent': 'LoomLauncher/0.1.0',
          ...headers
        }
      },
      (res) => {
        let data = ''
        res.on('data', (c) => (data += c))
        res.on('end', () => {
          try {
            resolve({ status: res.statusCode, json: JSON.parse(data) })
          } catch {
            resolve({ status: res.statusCode, json: null, raw: data })
          }
        })
      }
    )
    req.on('error', reject)
    req.end()
  })
}

export const CURSEFORGE_FALLBACK_KEY =
  '$2a$10$bL4bIL5pUWqfcO7KQtnMReakwtfHbNKh6v1uTpKlzhwoueEJQnPnm'

export const CURSEFORGE_API_KEY =
  process.env.CURSEFORGE_API_KEY ||
  CURSEFORGE_FALLBACK_KEY

// --- Directories ---
export function getLoomBaseDir(): string {
  const base = path.join(os.homedir(), '.local', 'share', 'loom')
  if (!fs.existsSync(base)) fs.mkdirSync(base, { recursive: true })
  return base
}

export function getInstancesDir(): string {
  const dir = path.join(getLoomBaseDir(), 'instances')
  if (!fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true })
  return dir
}

export function openInstanceFolder(instanceId: string, subDir?: string): boolean {
  let targetPath = path.join(getInstancesDir(), instanceId)
  if (subDir) {
    if (['mods', 'resourcepacks', 'shaderpacks', 'config', 'logs', 'saves'].includes(subDir)) {
      targetPath = path.join(targetPath, '.minecraft', subDir)
    } else {
      targetPath = path.join(targetPath, subDir)
    }
  }
  if (!fs.existsSync(targetPath)) {
    fs.mkdirSync(targetPath, { recursive: true })
  }
  try {
    if (process.platform === 'win32') {
      spawn('explorer.exe', [targetPath], { detached: true, stdio: 'ignore' })
    } else if (process.platform === 'darwin') {
      spawn('open', [targetPath], { detached: true, stdio: 'ignore' })
    } else {
      spawn('xdg-open', [targetPath], { detached: true, stdio: 'ignore' })
    }
    return true
  } catch (err) {
    console.error('[Loom::Minecraft] Failed to open instance folder:', err)
    return false
  }
}

export function getCacheModsDir(): string {
  const dir = path.join(getLoomBaseDir(), 'cache', 'mods')
  if (!fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true })
  return dir
}

export function getCommonAssetsDir(): string {
  const dir = path.join(getLoomBaseDir(), 'common', 'assets')
  if (!fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true })
  return dir
}

export function getCommonLibrariesDir(): string {
  const dir = path.join(getLoomBaseDir(), 'common', 'libraries')
  if (!fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true })
  return dir
}

export function getCommonVersionsDir(): string {
  const dir = path.join(getLoomBaseDir(), 'common', 'versions')
  if (!fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true })
  return dir
}

export function getRuntimesJavaDir(): string {
  const dir = path.join(getLoomBaseDir(), 'runtimes', 'java')
  if (!fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true })
  return dir
}

// --- SSE Event Broadcaster ---
const sseClients = new Set<ServerResponse>()

export function broadcastEvent(eventName: string, payload: any) {
  const data = `event: ${eventName}\ndata: ${JSON.stringify(payload)}\n\n`
  for (const client of sseClients) {
    try {
      client.write(data)
    } catch {
      sseClients.delete(client)
    }
  }
}

// Track running processes and sessions for playtime
const activeProcesses = new Map<string, ChildProcess>()

export function isInstanceRunning(instId: string): boolean {
  if (activeProcesses.has(instId)) return true
  try {
    const pidFile = path.join(getInstancesDir(), instId, 'instance.pid')
    if (fs.existsSync(pidFile)) {
      const pid = parseInt(fs.readFileSync(pidFile, 'utf8').trim(), 10)
      if (!isNaN(pid) && pid > 0) {
        process.kill(pid, 0)
        return true
      }
    }
  } catch {
    try {
      const pidFile = path.join(getInstancesDir(), instId, 'instance.pid')
      if (fs.existsSync(pidFile)) fs.unlinkSync(pidFile)
    } catch {}
  }
  return false
}

// Kill any remaining instances when Loom server shuts down
const cleanAllActiveProcesses = () => {
  for (const [, child] of activeProcesses.entries()) {
    if (child && child.pid) {
      try {
        if (process.platform !== 'win32') {
          process.kill(-child.pid, 'SIGKILL')
        } else {
          child.kill('SIGKILL')
        }
      } catch {}
    }
  }
}

process.once('exit', cleanAllActiveProcesses)

export function logInstanceMessage(
  instanceId: string,
  source: string,
  level: string,
  message: string
) {
  if (level === 'ERROR') {
    console.error(`[${source}] [ERROR] ${message}`)
  } else if (level === 'WARN') {
    console.warn(`[${source}] [WARN] ${message}`)
  } else {
    console.log(`[${source}] [${level}] ${message}`)
  }

  if (instanceId) {
    broadcastEvent('minecraft-log', {
      instance_id: instanceId,
      timestamp: new Date().toLocaleTimeString(),
      level,
      source,
      message
    })
  }
}

function log(source: string, level: string, message: string, instanceId: string = '') {
  logInstanceMessage(instanceId, source, level, message)
}

export interface MinecraftGlobalStats {
  playtime_seconds: number
  last_played: number | null
  sessions_count: number
}

interface ActiveInstanceSession {
  instanceId: string
  sessionStartTime: number
  lastTickTime: number
  child: ChildProcess
}

const activeSessions = new Map<string, ActiveInstanceSession>()
let playtimeInterval: NodeJS.Timeout | null = null

function getGlobalStatsPath(): string {
  return path.join(getLoomBaseDir(), 'minecraft_global_stats.json')
}

export function getGlobalMinecraftStats(): MinecraftGlobalStats {
  const p = getGlobalStatsPath()
  if (fs.existsSync(p)) {
    try {
      const data = JSON.parse(fs.readFileSync(p, 'utf8'))
      return {
        playtime_seconds: Math.max(0, Number(data.playtime_seconds) || 0),
        last_played: data.last_played ? Number(data.last_played) : null,
        sessions_count: Math.max(0, Number(data.sessions_count) || 0)
      }
    } catch {}
  }
  const initial: MinecraftGlobalStats = {
    playtime_seconds: 0,
    last_played: null,
    sessions_count: 0
  }
  try {
    fs.writeFileSync(p, JSON.stringify(initial, null, 2))
  } catch {}
  return initial
}

export function saveGlobalMinecraftStats(stats: Partial<MinecraftGlobalStats>): MinecraftGlobalStats {
  const current = getGlobalMinecraftStats()
  const updated: MinecraftGlobalStats = {
    playtime_seconds: stats.playtime_seconds !== undefined ? Math.max(0, stats.playtime_seconds) : current.playtime_seconds,
    last_played: stats.last_played !== undefined ? stats.last_played : current.last_played,
    sessions_count: stats.sessions_count !== undefined ? stats.sessions_count : current.sessions_count
  }
  try {
    fs.writeFileSync(getGlobalStatsPath(), JSON.stringify(updated, null, 2))
  } catch (err) {
    console.warn('[MinecraftService] Failed to save global stats:', err)
  }
  return updated
}

export function addGlobalPlaytime(seconds: number): MinecraftGlobalStats {
  if (seconds <= 0) return getGlobalMinecraftStats()
  const current = getGlobalMinecraftStats()
  current.playtime_seconds += seconds
  current.last_played = Math.floor(Date.now() / 1000)
  return saveGlobalMinecraftStats(current)
}

export function addInstancePlaytime(instanceId: string, seconds: number): number {
  if (seconds <= 0) return 0
  const configFile = path.join(getInstancesDir(), instanceId, 'instance.json')
  if (!fs.existsSync(configFile)) return 0
  try {
    const data = JSON.parse(fs.readFileSync(configFile, 'utf8'))
    const current = Math.max(0, Number(data.playtime_seconds) || 0)
    const updated = current + seconds
    data.playtime_seconds = updated
    data.last_played = Math.floor(Date.now() / 1000)
    fs.writeFileSync(configFile, JSON.stringify(data, null, 2))
    return updated
  } catch {
    return 0
  }
}

function ensurePlaytimeHeartbeat() {
  if (playtimeInterval) return
  playtimeInterval = setInterval(() => {
    if (activeSessions.size === 0) {
      if (playtimeInterval) {
        clearInterval(playtimeInterval)
        playtimeInterval = null
      }
      return
    }

    const now = Date.now()
    for (const [instanceId, session] of activeSessions.entries()) {
      const elapsedSec = Math.floor((now - session.lastTickTime) / 1000)
      if (elapsedSec >= 5) {
        session.lastTickTime = now
        const instPlaytime = addInstancePlaytime(instanceId, elapsedSec)
        const globalStats = addGlobalPlaytime(elapsedSec)

        broadcastEvent('minecraft-playtime-tick', {
          instance_id: instanceId,
          elapsed_seconds: elapsedSec,
          instance_playtime_seconds: instPlaytime,
          global_playtime_seconds: globalStats.playtime_seconds,
          instance_last_played: Math.floor(now / 1000),
          global_last_played: globalStats.last_played
        })
      }
    }
  }, 5000)
}

export function finalizeSession(instanceId: string) {
  const session = activeSessions.get(instanceId)
  if (session) {
    const now = Date.now()
    const nowSec = Math.floor(now / 1000)
    const remainingSec = Math.floor((now - session.lastTickTime) / 1000)
    if (remainingSec > 0) {
      const instPlaytime = addInstancePlaytime(instanceId, remainingSec)
      const globalStats = addGlobalPlaytime(remainingSec)
      broadcastEvent('minecraft-playtime-tick', {
        instance_id: instanceId,
        elapsed_seconds: remainingSec,
        instance_playtime_seconds: instPlaytime,
        global_playtime_seconds: globalStats.playtime_seconds,
        instance_last_played: nowSec,
        global_last_played: globalStats.last_played
      })
    } else {
      updateInstanceSettings(instanceId, { last_played: nowSec })
      saveGlobalMinecraftStats({ last_played: nowSec })
    }
    activeSessions.delete(instanceId)
  }
}

// --- Mod Downloads Progress Tracker ---
export interface ModDownloadProgress {
  id: string
  instanceId: string
  instanceName?: string
  remoteModId: string
  modName: string
  iconUrl?: string
  provider: string
  fileName?: string
  bytesDownloaded: number
  bytesTotal: number
  progressPercent: number
  downloadRateBytesPerSec: number
  status: 'pending' | 'downloading' | 'installing' | 'completed' | 'error'
  error?: string
}

export const activeModDownloads = new Map<string, ModDownloadProgress>()

// --- Utilities ---
function sha1(buffer: Buffer): string {
  return crypto.createHash('sha1').update(buffer).digest('hex')
}

async function downloadFile(
  url: string,
  destPath: string,
  expectedSha1?: string,
  onProgress?: (downloaded: number, total: number, rateBytesPerSec: number) => void
): Promise<void> {
  const parent = path.dirname(destPath)
  if (!fs.existsSync(parent)) fs.mkdirSync(parent, { recursive: true })

  if (fs.existsSync(destPath)) {
    if (!expectedSha1) {
      if (onProgress) onProgress(100, 100, 0)
      return
    }
    const existingBuf = fs.readFileSync(destPath)
    if (sha1(existingBuf).toLowerCase() === expectedSha1.toLowerCase()) {
      if (onProgress) onProgress(100, 100, 0)
      return
    }
  }

  const res = await fetch(url, {
    headers: { 'User-Agent': 'LoomLauncher/0.1.0' }
  })
  if (!res.ok) {
    throw new Error(`Échec téléchargement ${url} (HTTP ${res.status})`)
  }

  const total = parseInt(res.headers.get('content-length') || '0', 10)
  let downloaded = 0
  const startTime = Date.now()
  let lastProgressEmit = 0

  if (!res.body) {
    const arrayBuf = await res.arrayBuffer()
    const buf = Buffer.from(arrayBuf)
    if (expectedSha1 && sha1(buf).toLowerCase() !== expectedSha1.toLowerCase()) {
      throw new Error(`Checksum SHA1 invalide pour ${destPath}`)
    }
    fs.writeFileSync(destPath, buf)
    if (onProgress) onProgress(buf.length, buf.length, 0)
    return
  }

  const reader = res.body.getReader()
  const chunks: Buffer[] = []

  while (true) {
    const { done, value } = await reader.read()
    if (done) break
    if (value) {
      chunks.push(Buffer.from(value))
      downloaded += value.length
      const now = Date.now()
      if (onProgress && (now - lastProgressEmit > 50 || downloaded === total)) {
        lastProgressEmit = now
        const elapsedSec = (now - startTime) / 1000
        const rate = elapsedSec > 0 ? Math.round(downloaded / elapsedSec) : 0
        onProgress(downloaded, total || downloaded, rate)
      }
    }
  }

  const buf = Buffer.concat(chunks)
  if (expectedSha1 && sha1(buf).toLowerCase() !== expectedSha1.toLowerCase()) {
    throw new Error(`Checksum SHA1 invalide pour ${destPath}`)
  }

  fs.writeFileSync(destPath, buf)
  if (onProgress) onProgress(buf.length, buf.length, 0)
}

// --- Java Provisioning & Resolution ---

export interface DetectedJavaRuntime {
  path: string
  major: number
  versionStr: string
  vendor: string
  isDefault?: boolean
}

export function inspectJavaBinary(binPath: string): DetectedJavaRuntime | null {
  try {
    if (!fs.existsSync(binPath)) return null
    const real = fs.realpathSync(binPath)
    const out = execSync(`"${real}" -version 2>&1`, { encoding: 'utf8' })
    const match = out.match(/version\s+"(\d+)(?:\.(\d+))?/i) || out.match(/build\s+(\d+)/i)
    let major = 0
    if (match) {
      const v1 = parseInt(match[1], 10)
      major = v1 === 1 && match[2] ? parseInt(match[2], 10) : v1
    }
    const verMatch = out.match(/version\s+"([^"]+)"/i)
    const verStr = verMatch ? verMatch[1] : `Java ${major}`
    let vendor = 'OpenJDK'
    if (out.includes('Temurin')) vendor = 'Eclipse Temurin'
    else if (out.includes('Red_Hat') || out.includes('Red Hat')) vendor = 'Red Hat OpenJDK'
    else if (out.includes('Oracle')) vendor = 'Oracle HotSpot'
    else if (out.includes('GraalVM')) vendor = 'GraalVM'
    else if (out.includes('Zulu')) vendor = 'Azul Zulu'
    else if (out.includes('Corretto')) vendor = 'Amazon Corretto'
    return { path: real, major, versionStr: verStr, vendor }
  } catch {
    return null
  }
}

export function detectSystemJavaInstallations(): DetectedJavaRuntime[] {
  const candidates = new Set<string>()
  const searchDirs = [
    path.join(os.homedir(), '.sdkman/candidates/java'),
    '/usr/lib/jvm',
    '/usr/lib64/jvm',
    '/opt/java',
    '/opt/jdk',
    getRuntimesJavaDir()
  ]

  for (const sDir of searchDirs) {
    if (fs.existsSync(sDir)) {
      try {
        const entries = fs.readdirSync(sDir)
        for (const entry of entries) {
          const directBin = path.join(sDir, entry, 'bin', 'java')
          if (fs.existsSync(directBin)) candidates.add(directBin)
          const jreBin = path.join(sDir, entry, 'jre', 'bin', 'java')
          if (fs.existsSync(jreBin)) candidates.add(jreBin)
        }
      } catch {}
    }
  }

  try {
    const whichJava = execSync('which java', { encoding: 'utf8' }).trim()
    if (whichJava && fs.existsSync(whichJava)) candidates.add(whichJava)
  } catch {}

  const results: DetectedJavaRuntime[] = []
  const seenPaths = new Set<string>()
  for (const bin of candidates) {
    const info = inspectJavaBinary(bin)
    if (info && !seenPaths.has(info.path)) {
      seenPaths.add(info.path)
      results.push(info)
    }
  }
  results.sort((a, b) => b.major - a.major)
  return results
}

export function getRequiredJavaVersion(mcVersion: string): number {
  const clean = (mcVersion || '').trim()
  const parts = clean.split('.').map((p) => parseInt(p, 10))

  if (parts.length > 0 && !isNaN(parts[0])) {
    // 1. Modern Minecraft calendar/year versions (26.1, 26.3, 25.x, or >= 25)
    if (parts[0] >= 25) {
      return 25
    }
    // 2. Standard 1.x versioning
    if (parts[0] === 1) {
      const minor = parts[1] || 0
      const patch = parts[2] || 0
      if (minor < 17) return 8
      if (minor === 17) return 17
      if (minor < 20 || (minor === 20 && patch < 5)) return 17
      if (minor === 20 && patch >= 5) return 21
      if (minor === 21) return 21
      if (minor >= 22) return 25
    }
  }

  // 3. Snapshot format (e.g. 26w..., 25w..., 24w...)
  const snapMatch = clean.match(/^(\d{2})w/i)
  if (snapMatch) {
    const year = parseInt(snapMatch[1], 10)
    if (year >= 25) return 25
    if (year === 24) return 21
  }

  return 21
}

export async function resolveJavaBinary(
  mcVersion: string,
  customJavaPath?: string,
  instanceId?: string
): Promise<string> {
  // 1. Check if user configured a manual custom Java path
  if (customJavaPath && customJavaPath.trim()) {
    const trimmed = customJavaPath.trim()
    if (fs.existsSync(trimmed)) {
      const info = inspectJavaBinary(trimmed)
      if (instanceId) {
        broadcastEvent('minecraft-log', {
          instance_id: instanceId,
          timestamp: new Date().toLocaleTimeString(),
          level: 'INFO',
          source: 'LoomLauncher',
          message: `Java manuel : ${trimmed} (${info ? `${info.vendor} Java ${info.major} (${info.versionStr})` : 'version personnalisée'})`
        })
      }
      return trimmed
    } else {
      if (instanceId) {
        broadcastEvent('minecraft-log', {
          instance_id: instanceId,
          timestamp: new Date().toLocaleTimeString(),
          level: 'WARN',
          source: 'LoomLauncher',
          message: `Le chemin Java manuel n'existe pas (${trimmed}). Recherche automatique d'un runtime compatible...`
        })
      }
    }
  }

  // 2. Default matching according to Minecraft version
  const requiredMajor = getRequiredJavaVersion(mcVersion)

  if (instanceId) {
    broadcastEvent('minecraft-log', {
      instance_id: instanceId,
      timestamp: new Date().toLocaleTimeString(),
      level: 'INFO',
      source: 'LoomLauncher',
      message: `Version Java requise par défaut pour Minecraft ${mcVersion} : Java ${requiredMajor}`
    })
  }

  // 3. Check system Java installations (SDKMAN, /usr/lib/jvm, PATH, etc.)
  const systemJavas = detectSystemJavaInstallations()

  // 3a. Exact match on major version
  const exactMatch = systemJavas.find((j) => j.major === requiredMajor)
  if (exactMatch) {
    if (instanceId) {
      broadcastEvent('minecraft-log', {
        instance_id: instanceId,
        timestamp: new Date().toLocaleTimeString(),
        level: 'INFO',
        source: 'LoomLauncher',
        message: `Runtime Java sélectionné : ${exactMatch.path} (${exactMatch.vendor} ${exactMatch.versionStr})`
      })
    }
    return exactMatch.path
  }

  // 3b. Local Loom runtime: ~/.local/share/loom/runtimes/java/${requiredMajor}/bin/java
  const localJava = path.join(getRuntimesJavaDir(), String(requiredMajor), 'bin', 'java')
  if (fs.existsSync(localJava)) {
    if (instanceId) {
      broadcastEvent('minecraft-log', {
        instance_id: instanceId,
        timestamp: new Date().toLocaleTimeString(),
        level: 'INFO',
        source: 'LoomLauncher',
        message: `Runtime Loom provisionné sélectionné : ${localJava} (Java ${requiredMajor})`
      })
    }
    return localJava
  }

  // 3c. Compatible match: if major >= requiredMajor
  const compatibleMatch = systemJavas.find((j) => j.major >= requiredMajor)
  if (compatibleMatch) {
    if (instanceId) {
      broadcastEvent('minecraft-log', {
        instance_id: instanceId,
        timestamp: new Date().toLocaleTimeString(),
        level: 'INFO',
        source: 'LoomLauncher',
        message: `Runtime Java compatible sélectionné : ${compatibleMatch.path} (${compatibleMatch.vendor} ${compatibleMatch.versionStr})`
      })
    }
    return compatibleMatch.path
  }

  // 4. Fallback: provision Adoptium Temurin
  if (instanceId) {
    broadcastEvent('minecraft-log', {
      instance_id: instanceId,
      timestamp: new Date().toLocaleTimeString(),
      level: 'INFO',
      source: 'LoomLauncher',
      message: `Téléchargement autonome d'Adoptium Temurin Java ${requiredMajor}...`
    })
  }

  const targetDir = path.join(getRuntimesJavaDir(), String(requiredMajor))
  if (!fs.existsSync(targetDir)) fs.mkdirSync(targetDir, { recursive: true })

  const adoptiumUrl = `https://api.adoptium.net/v3/binary/latest/${requiredMajor}/ga/linux/x64/jdk/hotspot/normal/eclipse`
  const archivePath = path.join(getRuntimesJavaDir(), `temurin-${requiredMajor}.tar.gz`)

  try {
    await downloadFile(adoptiumUrl, archivePath)
    execSync(`tar -xzf "${archivePath}" -C "${targetDir}" --strip-components=1`)
    if (fs.existsSync(archivePath)) fs.unlinkSync(archivePath)
    const javaBin = path.join(targetDir, 'bin', 'java')
    if (fs.existsSync(javaBin)) {
      fs.chmodSync(javaBin, 0o755)
      return javaBin
    }
  } catch (err: any) {
    console.warn(`[Loom::Java] Échec provisioning Adoptium Temurin:`, err.message)
  }

  // Last ditch effort: return 'java'
  return 'java'
}

// --- Instance Management ---
export interface InstanceRecord {
  id: string
  name: string
  mc_version: string
  modloader: string
  loader_version?: string | null
  min_ram_mb: number
  max_ram_mb: number
  java_path?: string | null
  jvm_args: string
  banner_url?: string | null
  cover_url?: string | null
  icon?: string | null
  playtime_seconds?: number
  last_played?: number | null
  created_at: number
}

export function listInstances(): InstanceRecord[] {
  const dir = getInstancesDir()
  const files = fs.readdirSync(dir)
  const instances: InstanceRecord[] = []

  for (const id of files) {
    const configFile = path.join(dir, id, 'instance.json')
    if (fs.existsSync(configFile)) {
      try {
        const data = JSON.parse(fs.readFileSync(configFile, 'utf8'))
        const instId = data.id || id
        instances.push({
          id: instId,
          name: data.name || id,
          mc_version: data.mc_version || '1.20.1',
          modloader: data.modloader || 'fabric',
          loader_version: data.loader_version || null,
          min_ram_mb: data.min_ram_mb || 2048,
          max_ram_mb: data.max_ram_mb || 4096,
          java_path: data.java_path || null,
          jvm_args: typeof data.jvm_args === 'string' ? data.jvm_args : JSON.stringify(data.jvm_args || []),
          banner_url: data.banner_url || null,
          cover_url: data.cover_url || null,
          icon: data.icon || '⛏️',
          playtime_seconds: Math.max(0, Number(data.playtime_seconds) || 0),
          last_played: data.last_played || null,
          created_at: data.created_at || Math.floor(Date.now() / 1000),
          status: isInstanceRunning(instId) ? 'running' : 'idle'
        } as any)
      } catch {}
    }
  }

  instances.sort((a, b) => b.created_at - a.created_at)
  return instances
}

export function createInstance(params: {
  name: string
  mc_version: string
  modloader: string
  loader_version?: string | null
  min_ram_mb?: number
  max_ram_mb?: number
  java_path?: string | null
  jvm_args?: string[]
  banner_url?: string | null
  cover_url?: string | null
  icon?: string | null
}): InstanceRecord {
  const instanceId = `inst-${Date.now()}`
  const instDir = path.join(getInstancesDir(), instanceId)
  const mcDir = path.join(instDir, '.minecraft')

  fs.mkdirSync(instDir, { recursive: true })
  fs.mkdirSync(path.join(mcDir, 'mods'), { recursive: true })
  fs.mkdirSync(path.join(mcDir, 'resourcepacks'), { recursive: true })
  fs.mkdirSync(path.join(mcDir, 'shaderpacks'), { recursive: true })
  fs.mkdirSync(path.join(mcDir, 'config'), { recursive: true })

  // Write options.txt
  const optFile = path.join(mcDir, 'options.txt')
  if (!fs.existsSync(optFile)) {
    fs.writeFileSync(optFile, 'lang:fr_fr\nfullscreen:false\n')
  }

  const jargs = params.jvm_args || [
    '-XX:+UseG1GC',
    '-XX:+UnlockExperimentalVMOptions',
    '-XX:G1NewSizePercent=20',
    '-XX:G1ReservePercent=20',
    '-XX:MaxGCPauseMillis=50',
    '-XX:G1HeapRegionSize=32M'
  ]

  const record: InstanceRecord = {
    id: instanceId,
    name: params.name,
    mc_version: params.mc_version,
    modloader: params.modloader || 'fabric',
    loader_version: params.loader_version || null,
    min_ram_mb: params.min_ram_mb || 2048,
    max_ram_mb: params.max_ram_mb || 4096,
    java_path: params.java_path || null,
    jvm_args: JSON.stringify(jargs),
    banner_url: params.banner_url || null,
    cover_url: params.cover_url || null,
    icon: params.icon || '⛏️',
    playtime_seconds: 0,
    last_played: null,
    created_at: Math.floor(Date.now() / 1000)
  }

  fs.writeFileSync(path.join(instDir, 'instance.json'), JSON.stringify(record, null, 2))
  fs.writeFileSync(path.join(instDir, 'mods.json'), JSON.stringify([], null, 2))

  return record
}

export function deleteInstance(instanceId: string) {
  finalizeSession(instanceId)
  stopInstance(instanceId)
  activeProcesses.delete(instanceId)
  activeSessions.delete(instanceId)
  const instDir = path.join(getInstancesDir(), instanceId)
  if (fs.existsSync(instDir)) {
    fs.rmSync(instDir, { recursive: true, force: true })
  }
}

export function updateInstanceSettings(
  instanceId: string,
  settings: {
    name?: string
    min_ram_mb?: number
    max_ram_mb?: number
    jvm_args?: string[]
    java_path?: string | null
    banner_url?: string | null
    cover_url?: string | null
    icon?: string | null
    last_played?: number | null
  }
) {
  const configFile = path.join(getInstancesDir(), instanceId, 'instance.json')
  if (!fs.existsSync(configFile)) return
  const data = JSON.parse(fs.readFileSync(configFile, 'utf8'))

  if (settings.name && settings.name.trim()) data.name = settings.name.trim()
  if (settings.min_ram_mb !== undefined) data.min_ram_mb = settings.min_ram_mb
  if (settings.max_ram_mb !== undefined) data.max_ram_mb = settings.max_ram_mb
  if (settings.jvm_args !== undefined) data.jvm_args = JSON.stringify(settings.jvm_args)
  if (settings.java_path !== undefined) data.java_path = settings.java_path
  if (settings.banner_url !== undefined) data.banner_url = settings.banner_url
  if (settings.cover_url !== undefined) data.cover_url = settings.cover_url
  if (settings.icon !== undefined) data.icon = settings.icon
  if (settings.last_played !== undefined) data.last_played = settings.last_played

  fs.writeFileSync(configFile, JSON.stringify(data, null, 2))
}

// --- Mods Management ---
export interface ModRecord {
  id: string
  profile_id: string
  provider: string
  remote_mod_id: string
  remote_file_id: string
  name: string
  file_path: string
  filename: string
  load_priority: number
  is_enabled: boolean
  version?: string
  author?: string
  description?: string
  icon_url?: string
  file_size_mb?: number
  update_available?: boolean
  new_version?: string
  new_file_id?: string
  new_download_url?: string
}

export function listInstanceMods(instanceId: string): ModRecord[] {
  const modsFile = path.join(getInstancesDir(), instanceId, 'mods.json')
  if (!fs.existsSync(modsFile)) return []
  try {
    const list: ModRecord[] = JSON.parse(fs.readFileSync(modsFile, 'utf8'))
    const instanceModsDir = path.join(getInstancesDir(), instanceId, '.minecraft', 'mods')
    return list.map((m) => {
      let fileSizeMb = m.file_size_mb
      if (!fileSizeMb) {
        try {
          const fPath = path.join(instanceModsDir, m.filename)
          if (fs.existsSync(fPath)) {
            fileSizeMb = Math.round((fs.statSync(fPath).size / (1024 * 1024)) * 100) / 100
          }
        } catch {}
      }
      return {
        ...m,
        file_size_mb: fileSizeMb || 0,
        version: m.version || m.filename.replace(/\.jar$/, ''),
        name: m.name || m.filename.replace(/\.jar$/, ''),
        author: m.author || (m.provider === 'curseforge' ? 'CurseForge' : m.provider === 'modrinth' ? 'Modrinth' : 'Local')
      }
    })
  } catch {
    return []
  }
}

function saveInstanceMods(instanceId: string, mods: ModRecord[]) {
  const modsFile = path.join(getInstancesDir(), instanceId, 'mods.json')
  fs.writeFileSync(modsFile, JSON.stringify(mods, null, 2))
}

export function listInstanceResourcePacks(instanceId: string): ModRecord[] {
  const dir = path.join(getInstancesDir(), instanceId, '.minecraft', 'resourcepacks')
  if (!fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true })

  const jsonFile = path.join(getInstancesDir(), instanceId, 'resourcepacks.json')
  let saved: ModRecord[] = []
  if (fs.existsSync(jsonFile)) {
    try {
      saved = JSON.parse(fs.readFileSync(jsonFile, 'utf8'))
    } catch {}
  }

  const diskFiles = fs.readdirSync(dir).filter((f) => !f.startsWith('.'))
  const savedByFile = new Map<string, ModRecord>()
  for (const s of saved) {
    savedByFile.set(s.filename.replace(/\.disabled$/, ''), s)
  }

  const records: ModRecord[] = []
  for (const file of diskFiles) {
    const cleanBase = file.replace(/\.disabled$/, '')
    const isEnabled = !file.endsWith('.disabled')
    const existing = savedByFile.get(cleanBase)
    const filePath = path.join(dir, file)
    let sizeMb = 0
    try {
      const st = fs.statSync(filePath)
      if (st.isFile()) {
        sizeMb = Math.round((st.size / (1024 * 1024)) * 100) / 100
      }
    } catch {}

    const cleanTitle = cleanBase.replace(/\.zip$/i, '').replace(/[-_]/g, ' ')

    records.push({
      id: existing?.id || `rp-${Buffer.from(cleanBase).toString('hex').slice(0, 12)}`,
      profile_id: instanceId,
      provider: existing?.provider || 'local',
      remote_mod_id: existing?.remote_mod_id || '',
      remote_file_id: existing?.remote_file_id || '',
      name: existing?.name || cleanTitle,
      file_path: file,
      filename: file,
      load_priority: existing?.load_priority || 0,
      is_enabled: isEnabled,
      version: existing?.version || '1.0.0',
      icon_url: existing?.icon_url || '',
      description: existing?.description || `Pack de ressources (${cleanTitle})`,
      author: existing?.author || (existing?.provider === 'curseforge' ? 'CurseForge' : existing?.provider === 'modrinth' ? 'Modrinth' : 'Local'),
      file_size_mb: sizeMb || existing?.file_size_mb || 0,
      update_available: false
    })
  }

  try {
    fs.writeFileSync(jsonFile, JSON.stringify(records, null, 2))
  } catch {}

  return records
}

export function saveInstanceResourcePacks(instanceId: string, packs: ModRecord[]) {
  const jsonFile = path.join(getInstancesDir(), instanceId, 'resourcepacks.json')
  fs.writeFileSync(jsonFile, JSON.stringify(packs, null, 2))
}

export function toggleResourcePackState(instanceId: string, packId: string, enabled: boolean) {
  const packs = listInstanceResourcePacks(instanceId)
  const target = packs.find((p) => p.id === packId || p.filename === packId)
  if (!target) return

  const dir = path.join(getInstancesDir(), instanceId, '.minecraft', 'resourcepacks')
  const cleanBase = target.filename.replace(/\.disabled$/, '')
  const activePath = path.join(dir, cleanBase)
  const disabledPath = path.join(dir, `${cleanBase}.disabled`)

  if (enabled) {
    if (fs.existsSync(disabledPath)) {
      fs.renameSync(disabledPath, activePath)
    }
    target.filename = cleanBase
    target.file_path = cleanBase
    target.is_enabled = true
  } else {
    if (fs.existsSync(activePath)) {
      fs.renameSync(activePath, disabledPath)
    }
    target.filename = `${cleanBase}.disabled`
    target.file_path = `${cleanBase}.disabled`
    target.is_enabled = false
  }

  saveInstanceResourcePacks(instanceId, packs)
}

export function deleteResourcePackFromInstance(instanceId: string, packId: string) {
  const packs = listInstanceResourcePacks(instanceId)
  const target = packs.find((p) => p.id === packId || p.filename === packId)
  if (!target) return

  const dir = path.join(getInstancesDir(), instanceId, '.minecraft', 'resourcepacks')
  const activePath = path.join(dir, target.filename)
  const disabledPath = path.join(dir, `${target.filename}.disabled`)

  try { if (fs.existsSync(activePath)) fs.unlinkSync(activePath) } catch {}
  try { if (fs.existsSync(disabledPath)) fs.unlinkSync(disabledPath) } catch {}

  saveInstanceResourcePacks(instanceId, packs.filter((p) => p.id !== target.id))
}

export function listInstanceShaderPacks(instanceId: string): ModRecord[] {
  const dir = path.join(getInstancesDir(), instanceId, '.minecraft', 'shaderpacks')
  if (!fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true })

  const jsonFile = path.join(getInstancesDir(), instanceId, 'shaderpacks.json')
  let saved: ModRecord[] = []
  if (fs.existsSync(jsonFile)) {
    try {
      saved = JSON.parse(fs.readFileSync(jsonFile, 'utf8'))
    } catch {}
  }

  const diskFiles = fs.readdirSync(dir).filter((f) => !f.startsWith('.'))
  const savedByFile = new Map<string, ModRecord>()
  for (const s of saved) {
    savedByFile.set(s.filename.replace(/\.disabled$/, ''), s)
  }

  const records: ModRecord[] = []
  for (const file of diskFiles) {
    const cleanBase = file.replace(/\.disabled$/, '')
    const isEnabled = !file.endsWith('.disabled')
    const existing = savedByFile.get(cleanBase)
    const filePath = path.join(dir, file)
    let sizeMb = 0
    try {
      const st = fs.statSync(filePath)
      if (st.isFile()) {
        sizeMb = Math.round((st.size / (1024 * 1024)) * 100) / 100
      }
    } catch {}

    const cleanTitle = cleanBase.replace(/\.zip$/i, '').replace(/[-_]/g, ' ')

    records.push({
      id: existing?.id || `sp-${Buffer.from(cleanBase).toString('hex').slice(0, 12)}`,
      profile_id: instanceId,
      provider: existing?.provider || 'local',
      remote_mod_id: existing?.remote_mod_id || '',
      remote_file_id: existing?.remote_file_id || '',
      name: existing?.name || cleanTitle,
      file_path: file,
      filename: file,
      load_priority: existing?.load_priority || 0,
      is_enabled: isEnabled,
      version: existing?.version || '1.0.0',
      icon_url: existing?.icon_url || '',
      description: existing?.description || `Shaderpack Minecraft (${cleanTitle})`,
      author: existing?.author || (existing?.provider === 'curseforge' ? 'CurseForge' : existing?.provider === 'modrinth' ? 'Modrinth' : 'Local'),
      file_size_mb: sizeMb || existing?.file_size_mb || 0,
      update_available: false
    })
  }

  try {
    fs.writeFileSync(jsonFile, JSON.stringify(records, null, 2))
  } catch {}

  return records
}

export function saveInstanceShaderPacks(instanceId: string, packs: ModRecord[]) {
  const jsonFile = path.join(getInstancesDir(), instanceId, 'shaderpacks.json')
  fs.writeFileSync(jsonFile, JSON.stringify(packs, null, 2))
}

export function toggleShaderPackState(instanceId: string, packId: string, enabled: boolean) {
  const packs = listInstanceShaderPacks(instanceId)
  const target = packs.find((p) => p.id === packId || p.filename === packId)
  if (!target) return

  const dir = path.join(getInstancesDir(), instanceId, '.minecraft', 'shaderpacks')
  const cleanBase = target.filename.replace(/\.disabled$/, '')
  const activePath = path.join(dir, cleanBase)
  const disabledPath = path.join(dir, `${cleanBase}.disabled`)

  if (enabled) {
    if (fs.existsSync(disabledPath)) {
      fs.renameSync(disabledPath, activePath)
    }
    target.filename = cleanBase
    target.file_path = cleanBase
    target.is_enabled = true
  } else {
    if (fs.existsSync(activePath)) {
      fs.renameSync(activePath, disabledPath)
    }
    target.filename = `${cleanBase}.disabled`
    target.file_path = `${cleanBase}.disabled`
    target.is_enabled = false
  }

  saveInstanceShaderPacks(instanceId, packs)
}

export function deleteShaderPackFromInstance(instanceId: string, packId: string) {
  const packs = listInstanceShaderPacks(instanceId)
  const target = packs.find((p) => p.id === packId || p.filename === packId)
  if (!target) return

  const dir = path.join(getInstancesDir(), instanceId, '.minecraft', 'shaderpacks')
  const activePath = path.join(dir, target.filename)
  const disabledPath = path.join(dir, `${target.filename}.disabled`)

  try { if (fs.existsSync(activePath)) fs.unlinkSync(activePath) } catch {}
  try { if (fs.existsSync(disabledPath)) fs.unlinkSync(disabledPath) } catch {}

  saveInstanceShaderPacks(instanceId, packs.filter((p) => p.id !== target.id))
}

export async function checkModDependencies(
  instanceId: string,
  provider: string,
  remoteModId: string
): Promise<{
  targetVersion: any
  requiredDependencies: Array<{
    id: string
    slug: string
    name: string
    provider: string
    summary: string
    icon_url?: string
    required: boolean
  }>
}> {
  const inst = listInstances().find((i) => i.id === instanceId)
  const mcVersion = inst?.mc_version || '1.20.1'
  const modloader = (inst?.modloader || 'fabric').toLowerCase()
  const installedMods = listInstanceMods(instanceId)

  const installedKeys = new Set<string>()
  for (const m of installedMods) {
    if (m.remote_mod_id) installedKeys.add(m.remote_mod_id.toLowerCase())
    if (m.name) installedKeys.add(m.name.toLowerCase())
    if (m.filename) installedKeys.add(m.filename.toLowerCase().replace(/\.jar$/, ''))
  }

  const missingDeps: any[] = []
  let targetVersion: any = null

  if (provider === 'modrinth') {
    try {
      // 1. Fetch versions compatible with exact Minecraft version & Modloader
      const verUrl = `https://api.modrinth.com/v2/project/${remoteModId}/version?game_versions=${encodeURIComponent(
        JSON.stringify([mcVersion])
      )}&loaders=${encodeURIComponent(JSON.stringify([modloader]))}`

      const res = await fetch(verUrl, {
        headers: { 'User-Agent': 'LoomLauncher/0.1.0' }
      })
      if (res.ok) {
        const vData: any = await res.json()
        if (Array.isArray(vData) && vData.length > 0) {
          targetVersion = vData[0]
          const requiredDepProjectIds: string[] = []

          for (const d of targetVersion.dependencies || []) {
            if (d.dependency_type === 'required' && d.project_id) {
              requiredDepProjectIds.push(d.project_id)
            }
          }

          if (requiredDepProjectIds.length > 0) {
            const depUrl = `https://api.modrinth.com/v2/projects?ids=${encodeURIComponent(
              JSON.stringify(requiredDepProjectIds)
            )}`
            const depRes = await fetch(depUrl, {
              headers: { 'User-Agent': 'LoomLauncher/0.1.0' }
            })
            if (depRes.ok) {
              const depProjects: any = await depRes.json()
              for (const p of depProjects || []) {
                const isInstalled =
                  installedKeys.has(p.id.toLowerCase()) ||
                  installedKeys.has(p.slug.toLowerCase()) ||
                  installedKeys.has(p.title.toLowerCase())

                if (!isInstalled) {
                  missingDeps.push({
                    id: p.id,
                    slug: p.slug,
                    name: p.title,
                    provider: 'modrinth',
                    summary: p.description || '',
                    icon_url: p.icon_url || undefined,
                    required: true
                  })
                }
              }
            }
          }
        }
      }
    } catch (err) {
      console.warn('[Loom::Deps] Error checking Modrinth dependencies:', err)
    }
  } else if (provider === 'curseforge') {
    try {
      const loaderMap: Record<string, number> = { forge: 1, fabric: 4, quilt: 5, neoforge: 6 }
      const loaderType = loaderMap[modloader] || 4
      const filesUrl = `https://api.curseforge.com/v1/mods/${remoteModId}/files?gameVersion=${mcVersion}&modLoaderType=${loaderType}&pageSize=5`
      const fRes = await fetch(filesUrl, {
        headers: {
          'x-api-key': CURSEFORGE_API_KEY,
          'User-Agent': 'LoomLauncher/0.1.0'
        }
      })
      if (fRes.ok) {
        const fData: any = await fRes.json()
        const targetFile = fData?.data?.[0]
        if (targetFile) {
          targetVersion = targetFile
          const requiredModIds: number[] = []
          for (const d of targetFile.dependencies || []) {
            if (d.relationType === 3 && d.modId) {
              requiredModIds.push(d.modId)
            }
          }

          for (const depModId of requiredModIds) {
            try {
              const mRes = await fetch(`https://api.curseforge.com/v1/mods/${depModId}`, {
                headers: {
                  'x-api-key': CURSEFORGE_API_KEY,
                  'User-Agent': 'LoomLauncher/0.1.0'
                }
              })
              if (mRes.ok) {
                const mData: any = await mRes.json()
                const depMod = mData?.data
                if (depMod) {
                  const isInstalled =
                    installedKeys.has(String(depMod.id)) ||
                    installedKeys.has(depMod.slug?.toLowerCase()) ||
                    installedKeys.has(depMod.name?.toLowerCase())
                  if (!isInstalled) {
                    missingDeps.push({
                      id: String(depMod.id),
                      slug: depMod.slug,
                      name: depMod.name,
                      provider: 'curseforge',
                      summary: depMod.summary || '',
                      icon_url: depMod.logo?.url || undefined,
                      required: true
                    })
                  }
                }
              }
            } catch {}
          }
        }
      }
    } catch (err) {
      console.warn('[Loom::Deps] Error checking CurseForge dependencies:', err)
    }
  }

  return {
    targetVersion,
    requiredDependencies: missingDeps
  }
}

export async function installModToInstance(params: {
  instanceId: string
  provider: string
  remoteModId: string
  remoteFileId?: string
  downloadUrl?: string
  filename?: string
  modName?: string
  iconUrl?: string
  version?: string
  author?: string
  description?: string
  mcVersion?: string
  modloader?: string
  contentType?: 'mod' | 'resourcepack' | 'shader'
}): Promise<ModRecord> {
  const inst = listInstances().find((i) => i.id === params.instanceId)
  const mcVersion = params.mcVersion || inst?.mc_version || '1.20.1'
  const modloader = (params.modloader || inst?.modloader || 'fabric').toLowerCase()
  const contentType = params.contentType || 'mod'

  const targetSubDir = contentType === 'resourcepack' ? 'resourcepacks' : contentType === 'shader' ? 'shaderpacks' : 'mods'
  const defaultExt = contentType === 'mod' ? '.jar' : '.zip'
  const cacheDir = contentType === 'mod' ? getCacheModsDir() : path.join(getLoomBaseDir(), 'cache', targetSubDir)
  if (!fs.existsSync(cacheDir)) fs.mkdirSync(cacheDir, { recursive: true })

  let safeFilename = params.filename || `${params.remoteModId}${defaultExt}`
  let downloadUrl = params.downloadUrl
  const downloadKey = `${params.instanceId}:${params.remoteModId}`

  const emitProgress = (
    status: 'pending' | 'downloading' | 'installing' | 'completed' | 'error',
    progressPercent: number,
    bytesDownloaded: number,
    bytesTotal: number,
    downloadRateBytesPerSec: number = 0,
    error?: string
  ) => {
    const progressItem: ModDownloadProgress = {
      id: downloadKey,
      instanceId: params.instanceId,
      instanceName: inst?.name || 'Instance Minecraft',
      remoteModId: params.remoteModId,
      modName: params.modName || safeFilename,
      iconUrl: params.iconUrl,
      provider: params.provider,
      fileName: safeFilename,
      bytesDownloaded,
      bytesTotal,
      progressPercent,
      downloadRateBytesPerSec,
      status,
      error
    }
    if (status === 'completed' || status === 'error') {
      activeModDownloads.delete(downloadKey)
    } else {
      activeModDownloads.set(downloadKey, progressItem)
    }
    broadcastEvent('minecraft-download-progress', progressItem)
  }

  try {
    emitProgress('pending', 5, 0, 0, 0)

    // 1. Resolve URL
    if (!downloadUrl) {
      if (params.provider === 'modrinth') {
        try {
          let verUrl = `https://api.modrinth.com/v2/project/${params.remoteModId}/version`
          if (contentType === 'mod') {
            verUrl += `?game_versions=${encodeURIComponent(
              JSON.stringify([mcVersion])
            )}&loaders=${encodeURIComponent(JSON.stringify([modloader]))}`
          }

          const res = await fetch(verUrl, { headers: { 'User-Agent': 'LoomLauncher/0.1.0' } })
          if (res.ok) {
            const vData: any = await res.json()
            if (Array.isArray(vData) && vData.length > 0) {
              const targetVersion = vData[0]
              const primary =
                targetVersion.files?.find((f: any) => f.primary) || targetVersion.files?.[0]
              if (primary?.url) {
                downloadUrl = primary.url
                if (primary.filename) safeFilename = primary.filename
              }
            }
          }
        } catch (e) {
          console.warn('[Loom::Install] Modrinth version resolution failed:', e)
        }

        // Fallback if specific version endpoint returned no results
        if (!downloadUrl) {
          const fallbackUrl = params.remoteFileId && params.remoteFileId !== params.remoteModId
            ? `https://api.modrinth.com/v2/version/${params.remoteFileId}`
            : `https://api.modrinth.com/v2/project/${params.remoteModId}/version`
          const res = await fetch(fallbackUrl, { headers: { 'User-Agent': 'LoomLauncher/0.1.0' } })
          if (res.ok) {
            const vData: any = await res.json()
            const primary = Array.isArray(vData) ? vData[0]?.files?.[0] : vData?.files?.[0]
            downloadUrl = primary?.url
            if (primary?.filename) safeFilename = primary.filename
          }
        }
      } else if (params.provider === 'curseforge') {
        try {
          const loaderMap: Record<string, number> = { forge: 1, fabric: 4, quilt: 5, neoforge: 6 }
          const loaderType = loaderMap[modloader] || 4
          let cfUrl = `https://api.curseforge.com/v1/mods/${params.remoteModId}/files?pageSize=10`
          if (contentType === 'mod') {
            cfUrl += `&gameVersion=${mcVersion}&modLoaderType=${loaderType}`
          }
          const cfRes = await fetch(cfUrl, {
            headers: {
              'x-api-key': CURSEFORGE_API_KEY,
              'User-Agent': 'LoomLauncher/0.1.0'
            }
          })
          if (cfRes.ok) {
            const cfData: any = await cfRes.json()
            const targetFile = cfData?.data?.[0]
            if (targetFile) {
              if (targetFile.fileName) safeFilename = targetFile.fileName
              if (targetFile.downloadUrl) {
                downloadUrl = targetFile.downloadUrl
              } else {
                const urlRes = await fetch(
                  `https://api.curseforge.com/v1/mods/${params.remoteModId}/files/${targetFile.id}/download-url`,
                  {
                    headers: {
                      'x-api-key': CURSEFORGE_API_KEY,
                      'User-Agent': 'LoomLauncher/0.1.0'
                    }
                  }
                )
                if (urlRes.ok) {
                  const uData: any = await urlRes.json()
                  downloadUrl = uData?.data
                }
              }
            }
          }
        } catch (e) {
          console.warn('[Loom::Install] CurseForge version resolution failed:', e)
        }
      }
    }

    emitProgress('pending', 12, 0, 0, 0)
    const cachedPath = path.join(cacheDir, safeFilename)

    if (downloadUrl && !fs.existsSync(cachedPath)) {
      emitProgress('downloading', 15, 0, 0, 0)
      await downloadFile(downloadUrl, cachedPath, undefined, (downloaded, total, rate) => {
        const pct = total > 0 ? Math.min(92, Math.round((downloaded / total) * 92)) : 50
        emitProgress('downloading', Math.max(15, pct), downloaded, total, rate)
      })
    } else if (fs.existsSync(cachedPath)) {
      const existingSize = fs.statSync(cachedPath).size
      emitProgress('downloading', 60, existingSize, existingSize, 0)
      await new Promise((r) => setTimeout(r, 60))
    }

    emitProgress('installing', 95, 0, 0, 0)

    // 2. Symlink/Copy into instance directory
    const instanceTargetDir = path.join(getInstancesDir(), params.instanceId, '.minecraft', targetSubDir)
    if (!fs.existsSync(instanceTargetDir)) fs.mkdirSync(instanceTargetDir, { recursive: true })
    const targetSymlink = path.join(instanceTargetDir, safeFilename)

    try {
      fs.unlinkSync(targetSymlink)
    } catch {}

    if (fs.existsSync(cachedPath)) {
      try {
        fs.symlinkSync(cachedPath, targetSymlink)
      } catch {
        fs.copyFileSync(cachedPath, targetSymlink)
      }
    }

    // 3. Save to corresponding json file
    let records: ModRecord[] = []
    if (contentType === 'resourcepack') {
      records = listInstanceResourcePacks(params.instanceId)
    } else if (contentType === 'shader') {
      records = listInstanceShaderPacks(params.instanceId)
    } else {
      records = listInstanceMods(params.instanceId)
    }

    const existingIdx = records.findIndex(
      (m) =>
        m.remote_mod_id === params.remoteModId ||
        m.filename.toLowerCase() === safeFilename.toLowerCase() ||
        m.name.toLowerCase() === (params.modName || safeFilename).toLowerCase()
    )

    const newRecord: ModRecord = {
      id: existingIdx >= 0 ? records[existingIdx].id : `${contentType === 'resourcepack' ? 'rp' : contentType === 'shader' ? 'sp' : 'pmod'}-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`,
      profile_id: params.instanceId,
      provider: params.provider,
      remote_mod_id: params.remoteModId,
      remote_file_id: params.remoteFileId || '',
      name: params.modName || (existingIdx >= 0 ? records[existingIdx].name : safeFilename),
      file_path: safeFilename,
      filename: safeFilename,
      load_priority: existingIdx >= 0 ? records[existingIdx].load_priority : records.length,
      is_enabled: true,
      icon_url: params.iconUrl || (existingIdx >= 0 ? records[existingIdx].icon_url : undefined),
      version: params.version || (existingIdx >= 0 ? records[existingIdx].version : undefined) || safeFilename.replace(/\.(jar|zip)$/i, ''),
      author: params.author || (existingIdx >= 0 ? records[existingIdx].author : undefined),
      description: params.description || (existingIdx >= 0 ? records[existingIdx].description : undefined),
      file_size_mb: fs.existsSync(cachedPath) ? Math.round((fs.statSync(cachedPath).size / (1024 * 1024)) * 100) / 100 : 0
    }

    if (existingIdx >= 0) {
      records[existingIdx] = newRecord
    } else {
      records.push(newRecord)
    }

    if (contentType === 'resourcepack') {
      saveInstanceResourcePacks(params.instanceId, records)
    } else if (contentType === 'shader') {
      saveInstanceShaderPacks(params.instanceId, records)
    } else {
      saveInstanceMods(params.instanceId, records)
    }

    const finalSize = fs.existsSync(cachedPath) ? fs.statSync(cachedPath).size : 0
    emitProgress('completed', 100, finalSize, finalSize, 0)

    return newRecord
  } catch (err: any) {
    emitProgress('error', 0, 0, 0, 0, err.message)
    throw err
  }
}

export function toggleModState(instanceId: string, modId: string, enabled: boolean) {
  const mods = listInstanceMods(instanceId)
  const target = mods.find((m) => m.id === modId)
  if (!target) return

  const instanceModsDir = path.join(getInstancesDir(), instanceId, '.minecraft', 'mods')
  const activePath = path.join(instanceModsDir, target.filename)
  const disabledPath = path.join(instanceModsDir, `${target.filename}.disabled`)

  if (enabled) {
    if (fs.existsSync(disabledPath)) {
      try {
        fs.renameSync(disabledPath, activePath)
      } catch {}
    } else {
      const cachePath = path.join(getCacheModsDir(), target.filename)
      if (fs.existsSync(cachePath)) {
        try {
          fs.unlinkSync(activePath)
        } catch {}
        try {
          fs.symlinkSync(cachePath, activePath)
        } catch {
          fs.copyFileSync(cachePath, activePath)
        }
      }
    }
  } else {
    try {
      fs.renameSync(activePath, disabledPath)
    } catch {}
  }

  target.is_enabled = enabled
  saveInstanceMods(instanceId, mods)
}

export function deleteModFromInstance(instanceId: string, modId: string) {
  const mods = listInstanceMods(instanceId)
  const target = mods.find((m) => m.id === modId)
  if (!target) return

  const instanceModsDir = path.join(getInstancesDir(), instanceId, '.minecraft', 'mods')
  const activePath = path.join(instanceModsDir, target.filename)
  const disabledPath = path.join(instanceModsDir, `${target.filename}.disabled`)

  try {
    fs.unlinkSync(activePath)
  } catch {}
  try {
    fs.unlinkSync(disabledPath)
  } catch {}

  saveInstanceMods(
    instanceId,
    mods.filter((m) => m.id !== modId)
  )
}

export async function getModFullDetails(provider: string, modId: string): Promise<any> {
  if (provider === 'curseforge') {
    // 1. Fetch Mod Info
    const modRes = await fetch(`https://api.curseforge.com/v1/mods/${modId}`, {
      headers: {
        'x-api-key': CURSEFORGE_API_KEY,
        'User-Agent': 'LoomLauncher/0.1.0'
      }
    })
    if (!modRes.ok) {
      throw new Error(`CurseForge API mod error (${modRes.status})`)
    }
    const modJson: any = await modRes.json()
    const data = modJson?.data
    if (!data) throw new Error('Mod CurseForge introuvable')

    // 2. Fetch Description (HTML)
    let descriptionHtml = data.summary || ''
    try {
      const descRes = await fetch(`https://api.curseforge.com/v1/mods/${modId}/description`, {
        headers: {
          'x-api-key': CURSEFORGE_API_KEY,
          'User-Agent': 'LoomLauncher/0.1.0'
        }
      })
      if (descRes.ok) {
        const descJson: any = await descRes.json()
        if (descJson?.data) descriptionHtml = descJson.data
      }
    } catch (e) {
      console.warn('[Loom::Details] Failed to fetch CF description:', e)
    }

    // 3. Fetch Files / Versions
    let versions: any[] = []
    try {
      const filesRes = await fetch(`https://api.curseforge.com/v1/mods/${modId}/files?pageSize=50`, {
        headers: {
          'x-api-key': CURSEFORGE_API_KEY,
          'User-Agent': 'LoomLauncher/0.1.0'
        }
      })
      if (filesRes.ok) {
        const filesJson: any = await filesRes.json()
        const releaseTypeMap: Record<number, 'release' | 'beta' | 'alpha'> = { 1: 'release', 2: 'beta', 3: 'alpha' }
        versions = (filesJson?.data || []).map((f: any) => {
          const dUrl = f.downloadUrl || `https://edge.forgecdn.net/files/${Math.floor(f.id / 1000)}/${f.id % 1000}/${f.fileName}`
          const loaders: string[] = []
          const gvList: string[] = []
          for (const gv of f.gameVersions || []) {
            const lower = gv.toLowerCase()
            if (['forge', 'fabric', 'neoforge', 'quilt'].includes(lower)) {
              if (!loaders.includes(lower)) loaders.push(lower)
            } else if (/^\d+\.\d+/.test(gv)) {
              if (!gvList.includes(gv)) gvList.push(gv)
            }
          }
          return {
            id: String(f.id),
            name: f.displayName || f.fileName,
            version_number: f.fileName,
            release_type: releaseTypeMap[f.releaseType] || 'release',
            game_versions: gvList,
            loaders: loaders.length > 0 ? loaders : ['forge', 'fabric', 'neoforge'],
            date_published: f.fileDate,
            downloads: f.downloadCount || 0,
            filename: f.fileName,
            files: [
              {
                id: String(f.id),
                filename: f.fileName,
                url: dUrl,
                size_bytes: f.fileLength || 0,
                primary: true
              }
            ]
          }
        })
      }
    } catch (e) {
      console.warn('[Loom::Details] Failed to fetch CF files:', e)
    }

    // Extract categories
    const categories = (data.categories || []).map((c: any) => c.name)

    // Extract gallery
    const gallery = (data.screenshots || []).map((s: any) => ({
      url: s.url || s.thumbnailUrl,
      title: s.title || '',
      description: s.description || ''
    }))

    // Extract unique loaders and game_versions from versions
    const allLoaders = new Set<string>()
    const allGameVersions = new Set<string>()
    for (const v of versions) {
      for (const l of v.loaders) allLoaders.add(l)
      for (const gv of v.game_versions) allGameVersions.add(gv)
    }

    return {
      id: String(data.id),
      slug: data.slug,
      name: data.name,
      summary: data.summary || '',
      description: descriptionHtml,
      icon_url: data.logo?.url,
      banner_url: gallery[0]?.url || data.logo?.url,
      author: data.authors?.[0]?.name || 'Inconnu',
      authors: (data.authors || []).map((a: any) => ({ name: a.name, role: 'Author' })),
      provider: 'curseforge',
      downloads: data.downloadCount || 0,
      categories,
      loaders: Array.from(allLoaders),
      game_versions: Array.from(allGameVersions),
      gallery,
      versions,
      links: {
        website: data.links?.websiteUrl,
        wiki: data.links?.wikiUrl,
        issues: data.links?.issuesUrl,
        source: data.links?.sourceUrl
      }
    }
  }

  // Modrinth
  const pRes = await fetch(`https://api.modrinth.com/v2/project/${modId}`, {
    headers: { 'User-Agent': 'LoomLauncher/0.1.0' }
  })
  if (!pRes.ok) {
    throw new Error(`Modrinth API error (${pRes.status})`)
  }
  const p: any = await pRes.json()

  let descriptionHtml = p.description || ''
  try {
    if (p.body) {
      descriptionHtml = await marked.parse(p.body)
    }
  } catch (e) {
    console.warn('[Loom::Details] Failed to parse Modrinth markdown body:', e)
  }

  let versions: any[] = []
  try {
    const vRes = await fetch(`https://api.modrinth.com/v2/project/${modId}/version`, {
      headers: { 'User-Agent': 'LoomLauncher/0.1.0' }
    })
    if (vRes.ok) {
      const vList: any = await vRes.json()
      versions = (Array.isArray(vList) ? vList : []).map((v: any) => {
        let changelogHtml: string | undefined = undefined
        if (v.changelog) {
          try {
            changelogHtml = marked.parse(v.changelog) as string
          } catch {
            changelogHtml = v.changelog
          }
        }
        return {
          id: v.id,
          name: v.name || v.version_number,
          version_number: v.version_number,
          release_type: v.version_type || 'release',
          game_versions: v.game_versions || [],
          loaders: v.loaders || [],
          date_published: v.date_published,
          downloads: v.downloads || 0,
          changelog: changelogHtml,
          filename: v.files?.[0]?.filename,
          files: (v.files || []).map((f: any) => ({
            id: f.hashes?.sha1 || f.filename,
            filename: f.filename,
            url: f.url,
            size_bytes: f.size,
            primary: Boolean(f.primary)
          }))
        }
      })
    }
  } catch (e) {
    console.warn('[Loom::Details] Failed to fetch Modrinth versions:', e)
  }

  let authors: any[] = []
  let primaryAuthor = 'Inconnu'
  try {
    const mRes = await fetch(`https://api.modrinth.com/v2/project/${modId}/members`, {
      headers: { 'User-Agent': 'LoomLauncher/0.1.0' }
    })
    if (mRes.ok) {
      const mList: any = await mRes.json()
      authors = (Array.isArray(mList) ? mList : []).map((m: any) => ({
        name: m.user?.username || m.user?.name || 'Inconnu',
        role: m.role || 'Member'
      }))
      if (authors.length > 0) primaryAuthor = authors[0].name
    }
  } catch (e) {
    console.warn('[Loom::Details] Failed to fetch Modrinth members:', e)
  }

  const gallery = (p.gallery || []).map((g: any) => ({
    url: g.raw_url || g.url,
    title: g.title || '',
    description: g.description || ''
  }))

  return {
    id: p.id,
    slug: p.slug,
    name: p.title,
    summary: p.description || '',
    description: descriptionHtml,
    icon_url: p.icon_url,
    banner_url: gallery.find((g: any) => g.featured)?.url || gallery[0]?.url || p.icon_url,
    author: primaryAuthor,
    authors,
    provider: 'modrinth',
    downloads: p.downloads || 0,
    follows: p.followers || 0,
    categories: p.categories || [],
    loaders: p.loaders || [],
    game_versions: p.game_versions || [],
    gallery,
    versions,
    links: {
      website: p.source_url,
      wiki: p.wiki_url,
      issues: p.issues_url,
      discord: p.discord_url
    }
  }
}

const changelogCache = new Map<string, string>()

async function resolveExternalChangelog(html: string): Promise<string> {
  if (!html) return ''

  // Look for any external URL in href or text
  const match =
    html.match(/(?:href=["'])(?:(?:\/linkout\?remoteUrl=)?)(https%253a%252f%252f[^"'\s<>]+|https%3A%2F%2F[^"'\s<>]+|https?:\/\/[^"'\s<>]+)["']/i) ||
    html.match(/(https?:\/\/[^\s<>'"]+)/i)

  if (match) {
    let rawUrl = match[1]
    try {
      while (rawUrl.includes('%')) {
        const decoded = decodeURIComponent(rawUrl)
        if (decoded === rawUrl) break
        rawUrl = decoded
      }
    } catch {}

    // 1. GitHub blob: https://github.com/:owner/:repo/blob/:branch/...
    const blobMatch = rawUrl.match(/github\.com\/([^/]+)\/([^/]+)\/blob\/([^/]+)\/(.+)/i)
    if (blobMatch) {
      const [, owner, repo, branch, filePath] = blobMatch
      const rawUrlGithub = `https://raw.githubusercontent.com/${owner}/${repo}/${branch}/${filePath}`
      try {
        const res = await fetch(rawUrlGithub, { headers: { 'User-Agent': 'LoomLauncher/0.1.0' } })
        if (res.ok) {
          const text = await res.text()
          if (text.trim()) {
            return marked.parse(text) as string
          }
        }
      } catch {}
    }

    // 2. GitHub raw: https://raw.githubusercontent.com/:owner/:repo/:branch/...
    const rawMatch = rawUrl.match(/raw\.githubusercontent\.com\/([^/]+)\/([^/]+)\/(?:refs\/heads\/)?([^/]+)\/(.+)/i)
    if (rawMatch) {
      const [, owner, repo, branch, filePath] = rawMatch
      const cleanRawUrl = `https://raw.githubusercontent.com/${owner}/${repo}/${branch}/${filePath}`
      try {
        const res = await fetch(cleanRawUrl, { headers: { 'User-Agent': 'LoomLauncher/0.1.0' } })
        if (res.ok) {
          const text = await res.text()
          if (text.trim()) {
            return marked.parse(text) as string
          }
        }
      } catch {}
    }

    // 3. GitHub release tag: https://github.com/:owner/:repo/releases/tag/:tag
    const releaseMatch = rawUrl.match(/github\.com\/([^/]+)\/([^/]+)\/releases\/tag\/([^/?#]+)/i)
    if (releaseMatch) {
      const [, owner, repo, tag] = releaseMatch
      const apiUrl = `https://api.github.com/repos/${owner}/${repo}/releases/tags/${encodeURIComponent(tag)}`
      try {
        const res = await fetch(apiUrl, { headers: { 'User-Agent': 'LoomLauncher/0.1.0' } })
        if (res.ok) {
          const json: any = await res.json()
          if (json?.body && json.body.trim()) {
            return marked.parse(json.body) as string
          }
        }
      } catch {}
    }
  }

  // Ensure any <a> tags open externally and linkout URLs are clean
  return html.replace(/<a\s+(?:[^>]*?\s+)?href="([^"]*)"[^>]*>/gi, (_full, href) => {
    let cleanHref = href
    if (cleanHref.includes('remoteUrl=')) {
      try {
        const u = new URL(cleanHref, 'http://localhost')
        const r = u.searchParams.get('remoteUrl')
        if (r) cleanHref = decodeURIComponent(decodeURIComponent(r))
      } catch {}
    }
    return `<a href="${cleanHref}" target="_blank" rel="noopener noreferrer">`
  })
}

export async function getModVersionChangelog(provider: string, modId: string, fileId: string): Promise<string> {
  const normProvider = (provider || 'curseforge').toLowerCase()
  const cacheKey = `${normProvider}:${modId}:${fileId}`
  if (changelogCache.has(cacheKey)) {
    return changelogCache.get(cacheKey)!
  }

  let changelogHtml = ''
  try {
    if (normProvider === 'curseforge') {
      let numericModId = modId
      // If modId is not numeric, try to resolve it from CurseForge search
      if (!/^\d+$/.test(numericModId)) {
        try {
          const sRes = await fetch(`https://api.curseforge.com/v1/mods/search?gameId=432&searchFilter=${encodeURIComponent(modId)}&pageSize=1`, {
            headers: { 'x-api-key': CURSEFORGE_API_KEY, 'User-Agent': 'LoomLauncher/0.1.0' }
          })
          if (sRes.ok) {
            const sJson: any = await sRes.json()
            if (sJson?.data?.[0]?.id) numericModId = String(sJson.data[0].id)
          }
        } catch {}
      }

      let res = await fetch(`https://api.curseforge.com/v1/mods/${numericModId}/files/${fileId}/changelog`, {
        headers: {
          'x-api-key': CURSEFORGE_API_KEY,
          'User-Agent': 'LoomLauncher/0.1.0'
        }
      })
      if (res.status === 403 && CURSEFORGE_API_KEY !== CURSEFORGE_FALLBACK_KEY) {
        res = await fetch(`https://api.curseforge.com/v1/mods/${numericModId}/files/${fileId}/changelog`, {
          headers: {
            'x-api-key': CURSEFORGE_FALLBACK_KEY,
            'User-Agent': 'LoomLauncher/0.1.0'
          }
        })
      }
      if (res.ok) {
        const json: any = await res.json()
        changelogHtml = json?.data || ''
      }
    } else if (normProvider === 'modrinth') {
      const res = await fetch(`https://api.modrinth.com/v2/version/${fileId}`, {
        headers: { 'User-Agent': 'LoomLauncher/0.1.0' }
      })
      if (res.ok) {
        const json: any = await res.json()
        if (json?.changelog) {
          changelogHtml = (await marked.parse(json.changelog)) as string
        }
      }
    }

    // Resolve any external links (GitHub changelogs, raw markdown, GitHub releases, linkouts)
    if (changelogHtml) {
      changelogHtml = await resolveExternalChangelog(changelogHtml)
    }
  } catch (err) {
    console.warn(`[Loom::Changelog] Failed to fetch changelog for ${normProvider} ${modId} file ${fileId}:`, err)
  }

  changelogCache.set(cacheKey, changelogHtml)
  return changelogHtml
}

export async function checkInstanceModUpdates(instanceId: string): Promise<ModRecord[]> {
  const inst = listInstances().find((i) => i.id === instanceId)
  if (!inst) return []
  const mods = listInstanceMods(instanceId)
  const mcVersion = inst.mc_version
  const modloader = (inst.modloader || 'fabric').toLowerCase()

  const cfLoaderMap: Record<string, number> = { forge: 1, fabric: 4, quilt: 5, neoforge: 6 }
  const targetCfLoader = cfLoaderMap[modloader] || 4

  // Group CurseForge mods
  const cfMods = mods.filter((m) => m.provider === 'curseforge' && m.remote_mod_id)
  if (cfMods.length > 0) {
    const cfModIds = Array.from(new Set(cfMods.map((m) => parseInt(m.remote_mod_id)).filter(Boolean)))
    for (let i = 0; i < cfModIds.length; i += 50) {
      const chunk = cfModIds.slice(i, i + 50)
      try {
        const res = await fetch('https://api.curseforge.com/v1/mods', {
          method: 'POST',
          headers: {
            'x-api-key': CURSEFORGE_API_KEY,
            'Content-Type': 'application/json',
            'User-Agent': 'LoomLauncher/0.1.0'
          },
          body: JSON.stringify({ modIds: chunk })
        })
        if (res.ok) {
          const data: any = await res.json()
          for (const cfData of data?.data || []) {
            const currentMod = cfMods.find((m) => m.remote_mod_id === String(cfData.id))
            if (!currentMod) continue

            if (!currentMod.icon_url && cfData.logo?.url) currentMod.icon_url = cfData.logo.url
            if (!currentMod.author && cfData.authors?.[0]?.name) currentMod.author = cfData.authors[0].name
            if (currentMod.name === currentMod.filename && cfData.name) currentMod.name = cfData.name

            const matchingIndexes = (cfData.latestFilesIndexes || []).filter((idx: any) => {
              const verMatch = idx.gameVersion === mcVersion || idx.gameVersion?.startsWith(mcVersion)
              const loaderMatch = !idx.modLoader || idx.modLoader === targetCfLoader
              return verMatch && loaderMatch
            })

            if (matchingIndexes.length > 0) {
              matchingIndexes.sort((a: any, b: any) => b.fileId - a.fileId)
              const latest = matchingIndexes[0]
              const currentFileId = parseInt(currentMod.remote_file_id || '0')
              if (latest.fileId > currentFileId) {
                currentMod.update_available = true
                currentMod.new_version = latest.filename
                currentMod.new_file_id = String(latest.fileId)
                currentMod.new_download_url = `https://edge.forgecdn.net/files/${Math.floor(latest.fileId / 1000)}/${latest.fileId % 1000}/${latest.filename}`
              } else {
                currentMod.update_available = false
                currentMod.new_version = undefined
                currentMod.new_file_id = undefined
                currentMod.new_download_url = undefined
              }
            }
          }
        }
      } catch (err) {
        console.warn('[Loom::UpdateCheck] CF batch error:', err)
      }
    }
  }

  // Modrinth mods
  const mrMods = mods.filter((m) => m.provider === 'modrinth' && m.remote_mod_id)
  for (const m of mrMods) {
    try {
      const verUrl = `https://api.modrinth.com/v2/project/${m.remote_mod_id}/version?game_versions=${encodeURIComponent(
        JSON.stringify([mcVersion])
      )}&loaders=${encodeURIComponent(JSON.stringify([modloader]))}`
      const res = await fetch(verUrl, { headers: { 'User-Agent': 'LoomLauncher/0.1.0' } })
      if (res.ok) {
        const vList: any = await res.json()
        if (Array.isArray(vList) && vList.length > 0) {
          const latest = vList[0]
          if (latest.id !== m.remote_file_id && (!m.version || m.version !== latest.version_number)) {
            const primaryFile = latest.files?.find((f: any) => f.primary) || latest.files?.[0]
            m.update_available = true
            m.new_version = latest.version_number
            m.new_file_id = latest.id
            if (primaryFile?.url) m.new_download_url = primaryFile.url
          } else {
            m.update_available = false
            m.new_version = undefined
            m.new_file_id = undefined
            m.new_download_url = undefined
          }
        }
      }
    } catch (err) {
      console.warn('[Loom::UpdateCheck] MR check error for', m.name, err)
    }
  }

  saveInstanceMods(instanceId, mods)
  return mods
}

export async function updateModInInstance(
  instanceId: string,
  modId: string,
  fileId?: string,
  downloadUrl?: string
): Promise<ModRecord> {
  const mods = listInstanceMods(instanceId)
  const target = mods.find((m) => m.id === modId)
  if (!target) throw new Error(`Mod ${modId} introuvable`)

  const oldFilename = target.filename
  const targetFileId = fileId || target.new_file_id || target.remote_file_id
  const targetDownloadUrl = downloadUrl || target.new_download_url

  const updatedMod = await installModToInstance({
    instanceId,
    provider: target.provider,
    remoteModId: target.remote_mod_id,
    remoteFileId: targetFileId,
    downloadUrl: targetDownloadUrl,
    modName: target.name,
    iconUrl: target.icon_url,
    version: target.new_version || target.version,
    author: target.author,
    description: target.description
  })

  // If filename changed, remove old symlink
  if (oldFilename && oldFilename !== updatedMod.filename) {
    const oldPath = path.join(getInstancesDir(), instanceId, '.minecraft', 'mods', oldFilename)
    try {
      if (fs.existsSync(oldPath)) fs.unlinkSync(oldPath)
    } catch {}
  }

  // Clear update flag
  const currentMods = listInstanceMods(instanceId)
  const mIdx = currentMods.findIndex((m) => m.id === updatedMod.id)
  if (mIdx >= 0) {
    currentMods[mIdx].update_available = false
    currentMods[mIdx].new_version = undefined
    currentMods[mIdx].new_file_id = undefined
    currentMods[mIdx].new_download_url = undefined
    saveInstanceMods(instanceId, currentMods)
    return currentMods[mIdx]
  }

  return updatedMod
}

// --- Modpacks Management ---

export interface UnifiedModpackItem {
  id: string
  slug: string
  title: string
  author: string
  downloads: number
  likes: number
  iconUrl: string
  bannerUrl: string
  summary: string
  loader: string
  mcVersion: string
  source: 'modrinth' | 'curseforge'
  categories: string[]
}

export async function searchModpacks(params: {
  query: string
  provider?: 'all' | 'modrinth' | 'curseforge'
  limit?: number
}): Promise<UnifiedModpackItem[]> {
  const query = (params.query || '').trim()
  const provider = params.provider || 'all'
  const limit = params.limit || 24
  const results: UnifiedModpackItem[] = []

  // 1. Modrinth Modpacks
  if (provider === 'all' || provider === 'modrinth') {
    try {
      const facets = encodeURIComponent('[["project_type:modpack"]]')
      const sortParam = query ? 'relevance' : 'downloads'
      const url = `https://api.modrinth.com/v2/search?query=${encodeURIComponent(query)}&facets=${facets}&limit=${limit}&index=${sortParam}`
      const res = await fetch(url, { headers: { 'User-Agent': 'LoomLauncher/0.1.0' } })
      if (res.ok) {
        const data: any = await res.json()
        for (const hit of data.hits || []) {
          const categories = hit.categories || []
          const detectedLoader =
            categories.find((c: string) =>
              ['fabric', 'neoforge', 'forge', 'quilt'].includes(c.toLowerCase())
            ) || 'fabric'
          const mcVer =
            hit.versions && hit.versions.length > 0
              ? hit.versions[hit.versions.length - 1]
              : '1.20.1'

          results.push({
            id: hit.project_id,
            slug: hit.slug,
            title: hit.title,
            author: hit.author,
            downloads: hit.downloads || 0,
            likes: hit.follows || 0,
            iconUrl: hit.icon_url || '',
            bannerUrl: (hit.gallery && hit.gallery[0]) || hit.icon_url || '',
            summary: hit.description || '',
            loader: detectedLoader,
            mcVersion: mcVer,
            source: 'modrinth',
            categories
          })
        }
      }
    } catch (err) {
      console.warn('[Loom::Modpacks] Modrinth search failed:', err)
    }
  }

  // 2. CurseForge Modpacks
  if (provider === 'all' || provider === 'curseforge') {
    try {
      const cfParams = new URLSearchParams({
        gameId: '432',
        classId: '4471',
        pageSize: String(limit),
        sortField: '2', // Popularity (Trending & Best Known First)
        sortOrder: 'desc'
      })
      if (query.trim()) {
        cfParams.set('searchFilter', query.trim())
      }
      const cfUrl = `https://api.curseforge.com/v1/mods/search?${cfParams.toString()}`
      let res = await fetch(cfUrl, {
        headers: {
          'x-api-key': CURSEFORGE_API_KEY,
          'User-Agent': 'LoomLauncher/0.1.0'
        }
      })
      if (res.status === 403 && CURSEFORGE_API_KEY !== CURSEFORGE_FALLBACK_KEY) {
        res = await fetch(cfUrl, {
          headers: {
            'x-api-key': CURSEFORGE_FALLBACK_KEY,
            'User-Agent': 'LoomLauncher/0.1.0'
          }
        })
      }
      if (res.ok) {
        const cfData: any = await res.json()
        const cfLoaderMap: Record<number, string> = { 1: 'forge', 4: 'fabric', 5: 'quilt', 6: 'neoforge' }
        for (const item of cfData.data || []) {
          const lIndex = item.latestFilesIndexes?.[0]
          const loader = (lIndex?.modLoader && cfLoaderMap[lIndex.modLoader]) || 'fabric'
          const mcVer = lIndex?.gameVersion || '1.20.1'
          const banner =
            (item.screenshots && item.screenshots[0]?.url) ||
            item.logo?.url ||
            ''
          results.push({
            id: String(item.id),
            slug: item.slug,
            title: item.name,
            author: item.authors?.[0]?.name || 'Inconnu',
            downloads: item.downloadCount || 0,
            likes: item.thumbsUpCount || 0,
            iconUrl: item.logo?.url || '',
            bannerUrl: banner,
            summary: item.summary || '',
            loader,
            mcVersion: mcVer,
            source: 'curseforge',
            categories: (item.categories || []).map((c: any) => c.name)
          })
        }
      }
    } catch (err) {
      console.warn('[Loom::Modpacks] CurseForge search failed:', err)
    }
  }

  if (provider === 'all') {
    results.sort((a, b) => b.downloads - a.downloads)
  }

  return results
}

async function runWithConcurrency<T>(
  items: T[],
  concurrency: number,
  worker: (item: T, index: number) => Promise<void>
): Promise<void> {
  let index = 0
  const promises: Promise<void>[] = []
  for (let i = 0; i < concurrency && i < items.length; i++) {
    promises.push(
      (async () => {
        while (index < items.length) {
          const currentIdx = index++
          await worker(items[currentIdx], currentIdx)
        }
      })()
    )
  }
  await Promise.all(promises)
}

export interface InstallModpackParams {
  provider: 'modrinth' | 'curseforge' | 'local'
  modpackId?: string
  versionId?: string
  fileId?: number | string
  downloadUrl?: string
  instanceName?: string
  bannerUrl?: string
  iconUrl?: string
  localFilePath?: string
  mcVersion?: string
  modloader?: string
}

export function cleanupStaleModDownloads(): void {
  const allInsts = listInstances()
  for (const [k, v] of activeModDownloads.entries()) {
    const matchingInst = allInsts.find(i => i.name === v.instanceName || i.id === v.instanceId)
    if (matchingInst) {
      const mDir = path.join(getInstancesDir(), matchingInst.id, '.minecraft', 'mods')
      if (fs.existsSync(mDir)) {
        const jarCount = fs.readdirSync(mDir).filter(f => f.endsWith('.jar')).length
        if (jarCount > 0) {
          activeModDownloads.delete(k)
        }
      }
    }
  }
}

export async function installModpackService(params: InstallModpackParams): Promise<InstanceRecord> {
  const workDir = path.join(os.tmpdir(), `loom-modpack-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`)
  fs.mkdirSync(workDir, { recursive: true })
  const packArchive = path.join(workDir, 'pack.zip')
  const extractedDir = path.join(workDir, 'extracted')

  const trackingId = `pack-${params.provider}-${params.modpackId || Date.now()}`
  let packTitle = params.instanceName || 'Nouveau Modpack'
  let createdInstanceId: string | null = null

  // Clean up any stale active download records for this pack
  for (const [k, v] of activeModDownloads.entries()) {
    if (k === trackingId || v.instanceName === packTitle || (params.modpackId && v.remoteModId === String(params.modpackId))) {
      activeModDownloads.delete(k)
    }
  }

  const broadcastModpackProgress = (
    current: number,
    total: number,
    fileName: string,
    status: 'pending' | 'downloading' | 'installing' | 'completed' | 'error' = 'downloading',
    errorMsg?: string
  ) => {
    const percent = total > 0 ? Math.min(100, Math.round((current / total) * 100)) : 0
    const progressItem: ModDownloadProgress = {
      id: trackingId,
      instanceId: createdInstanceId || trackingId,
      instanceName: packTitle,
      remoteModId: params.modpackId ? String(params.modpackId) : trackingId,
      modName: total > 1 ? `Installation ${packTitle} (${current}/${total})` : `Modpack: ${packTitle}`,
      iconUrl: params.iconUrl,
      provider: params.provider,
      fileName,
      bytesDownloaded: current,
      bytesTotal: total,
      progressPercent: percent,
      downloadRateBytesPerSec: 0,
      status,
      error: errorMsg
    }
    if (status === 'completed' || status === 'error') {
      activeModDownloads.delete(trackingId)
      if (createdInstanceId) activeModDownloads.delete(createdInstanceId)
      for (const [k, v] of activeModDownloads.entries()) {
        if (v.instanceName === packTitle) activeModDownloads.delete(k)
      }
    } else {
      activeModDownloads.set(trackingId, progressItem)
    }
    broadcastEvent('minecraft-download-progress', progressItem)
  }

  try {
    broadcastModpackProgress(1, 100, "Téléchargement de l'archive modpack...", 'downloading')

    // 1. Download or retrieve archive
    if (params.provider === 'local') {
      if (!params.localFilePath || !fs.existsSync(params.localFilePath)) {
        throw new Error('Fichier archive local introuvable.')
      }
      fs.copyFileSync(params.localFilePath, packArchive)
    } else if (params.provider === 'modrinth') {
      let downloadUrl = params.downloadUrl
      if (!downloadUrl) {
        const verUrl = `https://api.modrinth.com/v2/project/${params.modpackId}/version`
        const res = await fetch(verUrl, { headers: { 'User-Agent': 'LoomLauncher/0.1.0' } })
        if (!res.ok) throw new Error(`Impossible de récupérer les versions du modpack Modrinth (${res.status})`)
        const versions: any = await res.json()
        const targetVersion = params.versionId
          ? versions.find((v: any) => v.id === params.versionId)
          : versions[0]
        if (!targetVersion) throw new Error('Aucune version disponible pour ce modpack.')
        const packFile = targetVersion.files?.find((f: any) => f.filename?.endsWith('.mrpack')) || targetVersion.files?.[0]
        if (!packFile?.url) throw new Error('Fichier .mrpack introuvable pour cette version.')
        downloadUrl = packFile.url
      }
      const archiveRes = await fetch(downloadUrl, { headers: { 'User-Agent': 'LoomLauncher/0.1.0' } })
      if (!archiveRes.ok) throw new Error(`Échec du téléchargement du modpack (${archiveRes.status})`)
      const buf = Buffer.from(await archiveRes.arrayBuffer())
      fs.writeFileSync(packArchive, buf)
    } else if (params.provider === 'curseforge') {
      let downloadUrl = params.downloadUrl
      let fileName = 'modpack.zip'
      if (!downloadUrl) {
        if (params.fileId) {
          const fRes = await fetch(`https://api.curseforge.com/v1/mods/${params.modpackId}/files/${params.fileId}`, {
            headers: { 'x-api-key': CURSEFORGE_API_KEY, 'User-Agent': 'LoomLauncher/0.1.0' }
          })
          if (fRes.ok) {
            const fData: any = await fRes.json()
            fileName = fData.data?.fileName || fileName
            downloadUrl = fData.data?.downloadUrl
          }
        } else {
          const mRes = await fetch(`https://api.curseforge.com/v1/mods/${params.modpackId}`, {
            headers: { 'x-api-key': CURSEFORGE_API_KEY, 'User-Agent': 'LoomLauncher/0.1.0' }
          })
          if (mRes.ok) {
            const mData: any = await mRes.json()
            const lFile = mData.data?.latestFiles?.[0]
            if (lFile) {
              fileName = lFile.fileName || fileName
              downloadUrl = lFile.downloadUrl
            }
          }
        }
        if (!downloadUrl && params.fileId) {
          const fid = Number(params.fileId)
          downloadUrl = `https://edge.forgecdn.net/files/${Math.floor(fid / 1000)}/${fid % 1000}/${encodeURIComponent(fileName)}`
        }
      }
      if (!downloadUrl) throw new Error('URL de téléchargement du modpack CurseForge introuvable.')
      const archiveRes = await fetch(downloadUrl, { headers: { 'User-Agent': 'LoomLauncher/0.1.0' } })
      if (!archiveRes.ok) throw new Error(`Échec du téléchargement du modpack CurseForge (${archiveRes.status})`)
      const buf = Buffer.from(await archiveRes.arrayBuffer())
      fs.writeFileSync(packArchive, buf)
    }

    // 2. Unpack archive
    broadcastModpackProgress(5, 100, "Extraction de l'archive et analyse des composants...", 'downloading')
    fs.mkdirSync(extractedDir, { recursive: true })
    execSync(`unzip -q -o "${packArchive}" -d "${extractedDir}"`)

    // 3. Inspect format & create instance
    let finalInstance: InstanceRecord | null = null

    const modrinthIndexPath = path.join(extractedDir, 'modrinth.index.json')
    const curseforgeManifestPath = path.join(extractedDir, 'manifest.json')

    if (fs.existsSync(modrinthIndexPath)) {
      // --- MODRINTH FORMAT ---
      const indexData = JSON.parse(fs.readFileSync(modrinthIndexPath, 'utf8'))
      const deps = indexData.dependencies || {}
      const mcVer = deps.minecraft || params.mcVersion || '1.20.1'

      let loader = 'fabric'
      let loaderVer: string | null = null
      if (deps['fabric-loader']) {
        loader = 'fabric'
        loaderVer = deps['fabric-loader']
      } else if (deps['neoforge']) {
        loader = 'neoforge'
        loaderVer = deps['neoforge']
      } else if (deps['forge']) {
        loader = 'forge'
        loaderVer = deps['forge']
      } else if (deps['quilt-loader']) {
        loader = 'quilt'
        loaderVer = deps['quilt-loader']
      }

      packTitle = params.instanceName || indexData.name || 'Modpack Modrinth'

      finalInstance = createInstance({
        name: packTitle,
        mc_version: mcVer,
        modloader: loader,
        loader_version: loaderVer,
        banner_url: params.bannerUrl || null,
        cover_url: params.iconUrl || null,
        icon: '📦'
      })
      createdInstanceId = finalInstance.id

      const mcDir = path.join(getInstancesDir(), finalInstance.id, '.minecraft')

      // Copy overrides (configs, datapacks, resourcepacks, options.txt, etc.)
      const overridesDir = path.join(extractedDir, 'overrides')
      if (fs.existsSync(overridesDir)) {
        fs.cpSync(overridesDir, mcDir, { recursive: true, force: true })
      }
      const clientOverridesDir = path.join(extractedDir, 'client-overrides')
      if (fs.existsSync(clientOverridesDir)) {
        fs.cpSync(clientOverridesDir, mcDir, { recursive: true, force: true })
      }

      // Download all mods
      const files = (indexData.files || []).filter((f: any) => f.env?.client !== 'unsupported')
      const totalMods = files.length
      let completedMods = 0

      broadcastModpackProgress(0, Math.max(1, totalMods), `Téléchargement des ${totalMods} mods...`, 'downloading')

      await runWithConcurrency(files, 5, async (f: any) => {
        const dest = path.join(mcDir, f.path)
        fs.mkdirSync(path.dirname(dest), { recursive: true })
        const dlUrl = f.downloads?.[0]
        if (dlUrl) {
          try {
            const modRes = await fetch(dlUrl, { headers: { 'User-Agent': 'LoomLauncher/0.1.0' } })
            if (modRes.ok) {
              const buf = Buffer.from(await modRes.arrayBuffer())
              fs.writeFileSync(dest, buf)
            }
          } catch (err) {
            console.warn(`[Loom::Modpack] Échec téléchargement mod ${f.path}:`, err)
          }
        }
        completedMods++
        const modFileName = path.basename(f.path)
        broadcastModpackProgress(completedMods, totalMods, modFileName, 'downloading')
      })

    } else if (fs.existsSync(curseforgeManifestPath)) {
      // --- CURSEFORGE FORMAT ---
      const manifestData = JSON.parse(fs.readFileSync(curseforgeManifestPath, 'utf8'))
      const mcVer = manifestData.minecraft?.version || params.mcVersion || '1.20.1'

      const rawLoader = manifestData.minecraft?.modLoaders?.[0]?.id || ''
      let loader = 'fabric'
      let loaderVer: string | null = null
      if (rawLoader.startsWith('fabric-')) {
        loader = 'fabric'
        loaderVer = rawLoader.replace('fabric-', '')
      } else if (rawLoader.startsWith('forge-')) {
        loader = 'forge'
        loaderVer = rawLoader.replace('forge-', '')
      } else if (rawLoader.startsWith('neoforge-')) {
        loader = 'neoforge'
        loaderVer = rawLoader.replace('neoforge-', '')
      } else if (rawLoader.startsWith('quilt-')) {
        loader = 'quilt'
        loaderVer = rawLoader.replace('quilt-', '')
      }

      packTitle = params.instanceName || manifestData.name || 'Modpack CurseForge'

      const recRam = typeof manifestData.minecraft?.recommendedRam === 'number'
        ? manifestData.minecraft.recommendedRam
        : null

      finalInstance = createInstance({
        name: packTitle,
        mc_version: mcVer,
        modloader: loader,
        loader_version: loaderVer,
        min_ram_mb: recRam ? Math.min(4096, recRam) : 2048,
        max_ram_mb: recRam || 4096,
        banner_url: params.bannerUrl || null,
        cover_url: params.iconUrl || null,
        icon: '📦'
      })
      createdInstanceId = finalInstance.id

      const mcDir = path.join(getInstancesDir(), finalInstance.id, '.minecraft')

      // Copy overrides (configs, datapacks, resourcepacks, etc.)
      const ovName = manifestData.overrides || 'overrides'
      const overridesDir = path.join(extractedDir, ovName)
      if (fs.existsSync(overridesDir)) {
        fs.cpSync(overridesDir, mcDir, { recursive: true, force: true })
      }

      // Resolve files using CurseForge Bulk API
      const fileList = manifestData.files || []
      const fileIds = fileList.map((f: any) => f.fileID).filter(Boolean)
      const resolvedFiles: Array<{ id: number; fileName: string; downloadUrl: string }> = []

      broadcastModpackProgress(5, 100, `Résolution des ${fileIds.length} composants CurseForge...`, 'downloading')

      for (let i = 0; i < fileIds.length; i += 80) {
        const chunk = fileIds.slice(i, i + 80)
        const progressCount = Math.min(fileIds.length, i + 80)
        broadcastModpackProgress(
          5 + Math.round((progressCount / Math.max(1, fileIds.length)) * 5),
          100,
          `Résolution des composants (${progressCount}/${fileIds.length})...`,
          'downloading'
        )
        try {
          const bulkRes = await fetch('https://api.curseforge.com/v1/mods/files', {
            method: 'POST',
            headers: {
              'Content-Type': 'application/json',
              'x-api-key': CURSEFORGE_API_KEY,
              'User-Agent': 'LoomLauncher/0.1.0'
            },
            body: JSON.stringify({ fileIds: chunk })
          })
          if (bulkRes.ok) {
            const bulkData: any = await bulkRes.json()
            for (const f of bulkData.data || []) {
              let url = f.downloadUrl
              if (!url) {
                url = `https://edge.forgecdn.net/files/${Math.floor(f.id / 1000)}/${f.id % 1000}/${encodeURIComponent(f.fileName)}`
              }
              resolvedFiles.push({ id: f.id, fileName: f.fileName, downloadUrl: url })
            }
          }
        } catch (e) {
          console.warn('[Loom::Modpack] Erreur résolution fichiers en masse CF:', e)
        }
      }

      const totalMods = resolvedFiles.length
      let completedMods = 0
      const modsDir = path.join(mcDir, 'mods')
      const shaderpacksDir = path.join(mcDir, 'shaderpacks')
      const resourcepacksDir = path.join(mcDir, 'resourcepacks')
      fs.mkdirSync(modsDir, { recursive: true })
      fs.mkdirSync(shaderpacksDir, { recursive: true })
      fs.mkdirSync(resourcepacksDir, { recursive: true })

      broadcastModpackProgress(0, Math.max(1, totalMods), `Téléchargement des ${totalMods} composants...`, 'downloading')

      await runWithConcurrency(resolvedFiles, 5, async (f) => {
        const dest = path.join(modsDir, f.fileName)
        try {
          const modRes = await fetch(f.downloadUrl, { headers: { 'User-Agent': 'LoomLauncher/0.1.0' } })
          if (modRes.ok) {
            const buf = Buffer.from(await modRes.arrayBuffer())
            fs.writeFileSync(dest, buf)

            // Re-route shaderpacks and resourcepacks if packed as zip
            if (f.fileName.toLowerCase().endsWith('.zip')) {
              try {
                const list = execSync(`zipinfo -1 "${dest}"`, { encoding: 'utf8' })
                if (list.includes('shaders/') || list.includes('shaderFile')) {
                  fs.renameSync(dest, path.join(shaderpacksDir, f.fileName))
                } else if (list.includes('pack.mcmeta') || list.includes('assets/')) {
                  fs.renameSync(dest, path.join(resourcepacksDir, f.fileName))
                }
              } catch {}
            }
          }
        } catch (err) {
          console.warn(`[Loom::Modpack] Échec téléchargement composant ${f.fileName}:`, err)
        }
        completedMods++
        broadcastModpackProgress(completedMods, totalMods, f.fileName, 'downloading')
      })

    } else {
      // --- GENERIC ARCHIVE (loose mods / config) ---
      packTitle = params.instanceName || 'Archive Modpack'
      finalInstance = createInstance({
        name: packTitle,
        mc_version: params.mcVersion || '1.20.1',
        modloader: params.modloader || 'fabric',
        loader_version: null,
        banner_url: params.bannerUrl || null,
        icon: '📦'
      })
      createdInstanceId = finalInstance.id

      const mcDir = path.join(getInstancesDir(), finalInstance.id, '.minecraft')
      fs.cpSync(extractedDir, mcDir, { recursive: true, force: true })
    }

    // 4. Populate mods.json from all .jar files in .minecraft/mods
    const mcModsDir = path.join(getInstancesDir(), finalInstance.id, '.minecraft', 'mods')
    if (fs.existsSync(mcModsDir)) {
      const jarFiles = fs.readdirSync(mcModsDir).filter((fn) => fn.endsWith('.jar') || fn.endsWith('.jar.disabled'))
      const records: ModRecord[] = jarFiles.map((fn, idx) => {
        const isEnabled = !fn.endsWith('.disabled')
        const cleanName = fn.replace(/\.disabled$/, '').replace(/\.jar$/, '')
        return {
          id: `mod-${Date.now()}-${idx}`,
          profile_id: finalInstance!.id,
          provider: params.provider,
          remote_mod_id: cleanName,
          remote_file_id: cleanName,
          name: cleanName,
          file_path: fn,
          filename: fn,
          load_priority: idx,
          is_enabled: isEnabled
        }
      })
      saveInstanceMods(finalInstance.id, records)
    }

    broadcastModpackProgress(100, 100, 'Installation terminée avec succès !', 'completed')
    return finalInstance

  } catch (err: any) {
    broadcastModpackProgress(0, 100, 'Erreur lors de l\'installation', 'error', err?.message || String(err))
    throw err
  } finally {
    try {
      fs.rmSync(workDir, { recursive: true, force: true })
    } catch {}
    if (params.localFilePath && params.localFilePath.startsWith(os.tmpdir())) {
      try {
        fs.unlinkSync(params.localFilePath)
      } catch {}
    }
  }
}

async function ensureNeoForgeInstalled(
  mcVersion: string,
  loaderVersion: string,
  log: (source: string, level: string, msg: string) => void
): Promise<any> {
  const commonDir = path.join(getLoomBaseDir(), 'common')
  const nfVerName = `neoforge-${loaderVersion}`
  const nfJsonPath = path.join(getCommonVersionsDir(), nfVerName, `${nfVerName}.json`)

  if (fs.existsSync(nfJsonPath)) {
    try {
      return JSON.parse(fs.readFileSync(nfJsonPath, 'utf8'))
    } catch {}
  }

  log('NeoForge', 'INFO', `Installation autonome de NeoForge ${loaderVersion} pour Minecraft ${mcVersion}...`)

  // Ensure launcher_profiles.json in commonDir
  const lpPath = path.join(commonDir, 'launcher_profiles.json')
  if (!fs.existsSync(lpPath)) {
    fs.writeFileSync(lpPath, JSON.stringify({ profiles: {} }, null, 2))
  }

  // Ensure vanilla version.json and client jar exist in common/versions/<mcVersion>
  const vDir = path.join(getCommonVersionsDir(), mcVersion)
  fs.mkdirSync(vDir, { recursive: true })
  const vJsonPath = path.join(vDir, `${mcVersion}.json`)
  if (!fs.existsSync(vJsonPath)) {
    const mRes = await fetch('https://piston-meta.mojang.com/mc/game/version_manifest_v2.json')
    const mData: any = await mRes.json()
    const entry = mData.versions?.find((v: any) => v.id === mcVersion)
    if (entry) {
      await downloadFile(entry.url, vJsonPath)
    }
  }

  const vJarPath = path.join(vDir, `${mcVersion}.jar`)
  if (!fs.existsSync(vJarPath)) {
    const vJson = JSON.parse(fs.readFileSync(vJsonPath, 'utf8'))
    const clientUrl = vJson.downloads?.client?.url
    if (clientUrl) await downloadFile(clientUrl, vJarPath)
  }

  // Download NeoForge installer
  const installerPath = path.join(getCommonVersionsDir(), `${nfVerName}-installer.jar`)
  if (!fs.existsSync(installerPath)) {
    const installerUrl = `https://maven.neoforged.net/releases/net/neoforged/neoforge/${loaderVersion}/neoforge-${loaderVersion}-installer.jar`
    log('NeoForge', 'INFO', `Téléchargement de l'installateur NeoForge (${loaderVersion})...`)
    await downloadFile(installerUrl, installerPath)
  }

  // Run installer in background
  log('NeoForge', 'INFO', 'Exécution des processeurs et patchs client NeoForge...')
  execSync(`java -jar "${installerPath}" --install-client "${commonDir}"`, {
    stdio: 'ignore'
  })

  if (!fs.existsSync(nfJsonPath)) {
    throw new Error(`Échec de l'installation de NeoForge ${loaderVersion}`)
  }

  log('NeoForge', 'INFO', `NeoForge ${loaderVersion} installé et configuré avec succès.`)
  return JSON.parse(fs.readFileSync(nfJsonPath, 'utf8'))
}

// --- Minecraft Execution Engine ---
export async function launchInstance(instanceId: string, allocatedRamGb?: number): Promise<void> {
  if (isInstanceRunning(instanceId)) {
    throw new Error('Cette instance est déjà en cours d\'exécution.')
  }

  const instDir = path.join(getInstancesDir(), instanceId)
  const configFile = path.join(instDir, 'instance.json')
  if (!fs.existsSync(configFile)) {
    fs.mkdirSync(instDir, { recursive: true })
    const mcDir = path.join(instDir, '.minecraft')
    fs.mkdirSync(path.join(mcDir, 'mods'), { recursive: true })
    fs.mkdirSync(path.join(mcDir, 'resourcepacks'), { recursive: true })
    fs.mkdirSync(path.join(mcDir, 'config'), { recursive: true })
    const optFile = path.join(mcDir, 'options.txt')
    if (!fs.existsSync(optFile)) fs.writeFileSync(optFile, 'lang:fr_fr\nfullscreen:false\n')

    const fallbackConfig: InstanceRecord = {
      id: instanceId,
      name: 'Mon Instance Minecraft',
      mc_version: '1.21.1',
      modloader: 'fabric',
      loader_version: null,
      min_ram_mb: 2048,
      max_ram_mb: (allocatedRamGb ? allocatedRamGb * 1024 : 4096),
      java_path: null,
      jvm_args: JSON.stringify(['-XX:+UseG1GC', '-XX:+UnlockExperimentalVMOptions']),
      created_at: Math.floor(Date.now() / 1000)
    }
    fs.writeFileSync(configFile, JSON.stringify(fallbackConfig, null, 2))
    fs.writeFileSync(path.join(instDir, 'mods.json'), JSON.stringify([], null, 2))
  }

  const inst: InstanceRecord = JSON.parse(fs.readFileSync(configFile, 'utf8'))
  const mcDir = path.join(instDir, '.minecraft')
  fs.mkdirSync(mcDir, { recursive: true })

  const log = (source: string, level: string, message: string) => {
    broadcastEvent('minecraft-log', {
      instance_id: instanceId,
      timestamp: new Date().toLocaleTimeString(),
      level,
      source,
      message
    })
  }

  log('LoomLauncher', 'INFO', `Démarrage de "${inst.name}" (${inst.mc_version} / ${inst.modloader})...`)

  // 1. Resolve Java Binary
  const javaBin = await resolveJavaBinary(inst.mc_version, inst.java_path || undefined, instanceId)
  log('LoomLauncher', 'INFO', `Environnement Java prêt : ${javaBin}`)

  // 2. Fetch Mojang Version Manifest & Version JSON
  log('MojangMeta', 'INFO', `Résolution des métadonnées officielles pour Minecraft ${inst.mc_version}...`)
  const manifestRes = await fetch('https://piston-meta.mojang.com/mc/game/version_manifest_v2.json')
  if (!manifestRes.ok) throw new Error('Impossible de contacter Mojang version manifest.')
  const manifestData: any = await manifestRes.json()

  const verEntry = manifestData.versions?.find((v: any) => v.id === inst.mc_version)
  if (!verEntry) throw new Error(`Version '${inst.mc_version}' inconnue chez Mojang.`)

  const vJsonRes = await fetch(verEntry.url)
  if (!vJsonRes.ok) throw new Error(`Impossible de récupérer version.json depuis ${verEntry.url}`)
  const vJson: any = await vJsonRes.json()

  // 3. Download Client Jar
  const clientUrl = vJson.downloads?.client?.url
  const clientSha1 = vJson.downloads?.client?.sha1
  if (!clientUrl) throw new Error('URL du client Minecraft introuvable dans version.json')

  const clientJarPath = path.join(getCommonVersionsDir(), inst.mc_version, `${inst.mc_version}.jar`)
  if (!fs.existsSync(clientJarPath)) {
    log('MojangMeta', 'INFO', `Téléchargement du client Minecraft ${inst.mc_version}.jar...`)
    await downloadFile(clientUrl, clientJarPath, clientSha1)
  }

  // 4. Download Mojang Libraries
  log('MojangMeta', 'INFO', 'Vérification et téléchargement des bibliothèques Mojang...')
  const classpathEntries: string[] = []
  const libPromises: Promise<any>[] = []

  for (const lib of vJson.libraries || []) {
    // OS filtering: Linux
    if (lib.rules) {
      let allowed = false
      for (const rule of lib.rules) {
        if (rule.action === 'allow') {
          if (!rule.os || rule.os.name === 'linux') allowed = true
        } else if (rule.action === 'disallow') {
          if (rule.os?.name === 'linux') allowed = false
        }
      }
      if (!allowed) continue
    }

    const artifact = lib.downloads?.artifact
    if (artifact?.url && artifact?.path) {
      const destLib = path.join(getCommonLibrariesDir(), artifact.path)
      classpathEntries.push(destLib)
      if (!fs.existsSync(destLib)) {
        libPromises.push(downloadFile(artifact.url, destLib, artifact.sha1))
      }
    }
  }

  // Process downloads in concurrent batches of 10
  for (let i = 0; i < libPromises.length; i += 10) {
    await Promise.all(libPromises.slice(i, i + 10))
  }

  // 5. Download Asset Index and Assets Objects
  const assetIndexUrl = vJson.assetIndex?.url
  const assetIndexId = vJson.assetIndex?.id || inst.mc_version
  const assetIndexSha1 = vJson.assetIndex?.sha1
  const assetIndexPath = path.join(getCommonAssetsDir(), 'indexes', `${assetIndexId}.json`)

  if (assetIndexUrl) {
    if (!fs.existsSync(assetIndexPath)) {
      log('MojangMeta', 'INFO', `Téléchargement de l'index des assets (${assetIndexId})...`)
      await downloadFile(assetIndexUrl, assetIndexPath, assetIndexSha1)
    }

    try {
      if (fs.existsSync(assetIndexPath)) {
        const indexJson = JSON.parse(fs.readFileSync(assetIndexPath, 'utf8'))
        const objects = indexJson.objects || {}
        const entries = Object.entries(objects) as [string, { hash: string; size: number }][]

        const objectsBase = path.join(getCommonAssetsDir(), 'objects')
        if (!fs.existsSync(objectsBase)) fs.mkdirSync(objectsBase, { recursive: true })

        const missing: { name: string; hash: string; prefix: string; path: string }[] = []
        for (const [name, obj] of entries) {
          if (!obj?.hash) continue
          const prefix = obj.hash.slice(0, 2)
          const targetFile = path.join(objectsBase, prefix, obj.hash)
          if (!fs.existsSync(targetFile) || fs.statSync(targetFile).size === 0) {
            missing.push({ name, hash: obj.hash, prefix, path: targetFile })
          }
        }

        if (missing.length > 0) {
          // Prioritize icons, fonts, and GUI textures before sound files
          missing.sort((a, b) => {
            const isPriorityA =
              a.name.includes('texture') || a.name.includes('font') || a.name.includes('icon') ? 1 : 0
            const isPriorityB =
              b.name.includes('texture') || b.name.includes('font') || b.name.includes('icon') ? 1 : 0
            return isPriorityB - isPriorityA
          })

          log('MojangMeta', 'INFO', `Synchronisation des assets Mojang (${missing.length} fichiers à vérifier/télécharger)...`)
          const total = missing.length
          let done = 0
          const concurrency = 30

          const worker = async () => {
            while (missing.length > 0) {
              const item = missing.pop()
              if (!item) break
              const url = `https://resources.download.minecraft.net/${item.prefix}/${item.hash}`
              try {
                await downloadFile(url, item.path, item.hash)
              } catch (err: any) {
                // Ignore transient sound errors
              }
              done++
              if (done % 300 === 0 || done === total) {
                log(
                  'MojangMeta',
                  'INFO',
                  `Téléchargement des assets : ${done}/${total} (${Math.round((done / total) * 100)}%)...`
                )
              }
            }
          }

          const pool = Array.from({ length: concurrency }, () => worker())
          await Promise.all(pool)
          log('MojangMeta', 'INFO', 'Tous les assets Mojang ont été téléchargés et validés.')
        } else {
          log('MojangMeta', 'INFO', 'Tous les assets Mojang sont déjà présents en cache.')
        }
      }
    } catch (err: any) {
      log('MojangMeta', 'WARN', `Vérification des assets partielle : ${err.message}`)
    }
  }

  // 6. Loader Specific Setup (Fabric / NeoForge / Vanilla / Quilt / Forge)
  let mainClass = vJson.mainClass || 'net.minecraft.client.main.Main'
  const extraJvmArgs: string[] = []
  const extraGameArgs: string[] = []

  if (inst.modloader === 'fabric') {
    log('FabricMeta', 'INFO', 'Résolution du loader Fabric et des dépendances Knot...')
    let loaderVer = inst.loader_version
    if (!loaderVer) {
      const fRes = await fetch(`https://meta.fabricmc.net/v2/versions/loader/${inst.mc_version}`)
      if (fRes.ok) {
        const fData: any = await fRes.json()
        loaderVer = fData[0]?.loader?.version || '0.16.9'
      } else {
        loaderVer = '0.16.9'
      }
    }

    const profileUrl = `https://meta.fabricmc.net/v2/versions/loader/${inst.mc_version}/${loaderVer}/profile/json`
    const pRes = await fetch(profileUrl)
    if (pRes.ok) {
      const profile: any = await pRes.json()
      if (profile.mainClass) mainClass = profile.mainClass

      for (const fLib of profile.libraries || []) {
        const [groupId, artifactId, version] = (fLib.name || '').split(':')
        if (groupId && artifactId && version) {
          const relPath = `${groupId.replace(/\./g, '/')}/${artifactId}/${version}/${artifactId}-${version}.jar`
          const libUrl = fLib.url ? `${fLib.url}${relPath}` : `https://maven.fabricmc.net/${relPath}`
          const destPath = path.join(getCommonLibrariesDir(), relPath)
          classpathEntries.push(destPath)
          if (!fs.existsSync(destPath)) {
            await downloadFile(libUrl, destPath).catch(() => {})
          }
        }
      }
    }
    classpathEntries.push(clientJarPath)

  } else if (inst.modloader === 'neoforge') {
    const loaderVer = inst.loader_version || '21.1.249'
    log('NeoForge', 'INFO', `Configuration de l'environnement NeoForge ${loaderVer}...`)
    const nfJson = await ensureNeoForgeInstalled(inst.mc_version, loaderVer, log)

    if (nfJson.mainClass) mainClass = nfJson.mainClass

    const libDir = getCommonLibrariesDir()

    // JVM arguments from NeoForge version JSON
    for (const rawArg of nfJson.arguments?.jvm || []) {
      if (typeof rawArg === 'string') {
        let val = rawArg
          .replace(/\$\{library_directory\}/g, libDir)
          .replace(/\$\{classpath_separator\}/g, ':')
          .replace(/\$\{version_name\}/g, `neoforge-${loaderVer}`)
        if (val.startsWith('-DignoreList=')) {
          // Append client jar names so ModLauncher does not transform vanilla client jar into an automatic module
          val += `,${inst.mc_version}.jar,minecraft-${inst.mc_version}-client.jar`
        }
        extraJvmArgs.push(val)
      }
    }

    // Game arguments from NeoForge version JSON
    for (const rawArg of nfJson.arguments?.game || []) {
      if (typeof rawArg === 'string') {
        extraGameArgs.push(rawArg)
      }
    }

    // NeoForge libraries
    for (const nLib of nfJson.libraries || []) {
      const art = nLib.downloads?.artifact
      if (art?.path) {
        const p = path.join(libDir, art.path)
        classpathEntries.push(p)
      } else if (nLib.name) {
        const parts = nLib.name.split(':')
        if (parts.length >= 3) {
          const [grp, artName, ver, classifier] = parts
          const cSuffix = classifier ? `-${classifier}` : ''
          const relPath = path.join(grp.replace(/\./g, '/'), artName, ver, `${artName}-${ver}${cSuffix}.jar`)
          classpathEntries.push(path.join(libDir, relPath))
        }
      }
    }

    // NeoForge: Include the vanilla client jar on the classpath.
    // Combined with -DignoreList above, ModLauncher will not treat it as a conflicting module.
    classpathEntries.push(clientJarPath)

  } else {
    // Vanilla
    classpathEntries.push(clientJarPath)
  }

  // Deduplicate and filter existing classpath files
  const uniqueClasspath = Array.from(new Set(classpathEntries.filter((p) => fs.existsSync(p))))
  const fullClasspath = uniqueClasspath.join(':')

  // 7. Resolve Microsoft Account (with automatic refresh check)
  let username = 'LoomPlayer'
  let uuid = '00000000-0000-0000-0000-000000000000'
  let accessToken = '00000000000000000000000000000000'
  let xuid = ''
  let clientId = 'c36a9fb6-4f2a-41ff-90bd-ae7cc92031eb'
  let rawSkinUrl = ''

  // Trigger token refresh if near expiration via local auth endpoint
  try {
    const authCheck = await fetch('http://localhost:5173/api/auth/microsoft/account')
    if (authCheck.ok) {
      const aData: any = await authCheck.json()
      if (aData?.account) {
        log('LoomLauncher', 'INFO', `Vérification compte Microsoft : session active pour ${aData.account.name}.`)
      }
    }
  } catch {}

  const authFile = path.join(getLoomBaseDir(), 'minecraft_account.json')
  let userPropertiesStr = '{}'

  if (fs.existsSync(authFile)) {
    try {
      const acc = JSON.parse(fs.readFileSync(authFile, 'utf8'))
      if (acc.name) username = acc.name
      if (acc.uuid) {
        uuid = String(acc.uuid)
        if (!uuid.includes('-') && uuid.length === 32) {
          uuid = uuid.replace(/^(\w{8})(\w{4})(\w{4})(\w{4})(\w{12})$/, '$1-$2-$3-$4-$5')
        }
      }
      if (acc.access_token) accessToken = acc.access_token
      if (acc.raw_skin_url) rawSkinUrl = acc.raw_skin_url
      if (acc.xuid) xuid = acc.xuid
      if (acc.client_id) clientId = acc.client_id

      if ((!xuid || !clientId) && accessToken && accessToken.includes('.')) {
        try {
          const payload = JSON.parse(Buffer.from(accessToken.split('.')[1], 'base64').toString('utf8'))
          if (!xuid && (payload.xuid || payload.xid)) xuid = String(payload.xuid || payload.xid)
          if (!clientId && payload.aid) clientId = String(payload.aid)
        } catch {}
      }

      // Check or fetch official signed textures for offline and singleplayer skin loading
      if (acc.user_properties && Array.isArray(acc.user_properties) && acc.user_properties.length > 0) {
        userPropertiesStr = JSON.stringify(acc.user_properties)
      } else if (uuid && uuid !== '00000000-0000-0000-0000-000000000000') {
        try {
          const stripped = uuid.replace(/-/g, '')
          const pData: any = await httpsGetJson(
            `https://sessionserver.mojang.com/session/minecraft/profile/${stripped}?unsigned=false`
          )
          if (pData?.json?.properties && pData.json.properties.length > 0) {
            acc.user_properties = pData.json.properties
            userPropertiesStr = JSON.stringify(acc.user_properties)
            fs.writeFileSync(authFile, JSON.stringify(acc, null, 2), 'utf8')
          }
        } catch {}
      }

      log(
        'LoomLauncher',
        'INFO',
        `Compte Microsoft lié : ${username} (UUID: ${uuid}, XUID: ${xuid || 'défini'}, Textures: ${acc.user_properties ? 'signées Mojang' : 'par défaut'})`
      )
    } catch {}
  } else {
    log('LoomLauncher', 'WARN', 'Aucun compte Microsoft lié détecté. Lancement en mode joueur local.')
  }

  // Pre-seed usercache.json for instant offline/online profile matching
  try {
    const usercacheFile = path.join(mcDir, 'usercache.json')
    const usercacheData = [
      {
        name: username,
        uuid: uuid,
        expiresOn: '2030-01-01 00:00:00 +0000'
      }
    ]
    fs.writeFileSync(usercacheFile, JSON.stringify(usercacheData, null, 2), 'utf8')
  } catch {}

  // 8. Assemble JVM and Game Arguments
  const minRam = inst.min_ram_mb || 2048
  const maxRam = allocatedRamGb ? allocatedRamGb * 1024 : inst.max_ram_mb || 4096

  let customArgs: string[] = []
  try {
    customArgs = JSON.parse(inst.jvm_args || '[]')
  } catch {
    customArgs = ['-XX:+UseG1GC']
  }

  const nativesDir = path.join(instDir, 'natives')
  if (!fs.existsSync(nativesDir)) fs.mkdirSync(nativesDir, { recursive: true })

  const jvmArgs = [
    `-Xms${minRam}M`,
    `-Xmx${maxRam}M`,
    ...customArgs,
    ...extraJvmArgs,
    // Network flags: enforce IPv4 to avoid Cloudflare/Mojang SSL timeout on Linux
    '-Djava.net.preferIPv4Stack=true',
    '-Djava.net.preferIPv4Addresses=true',
    '-Dsun.net.client.defaultConnectTimeout=15000',
    '-Dsun.net.client.defaultReadTimeout=15000',
    // Natives and extraction paths
    `-Djava.library.path=${nativesDir}`,
    `-Djna.tmpdir=${nativesDir}`,
    `-Dorg.lwjgl.system.SharedLibraryExtractPath=${nativesDir}`,
    `-Dio.netty.native.workdir=${nativesDir}`,
    '-Dminecraft.launcher.brand=Loom',
    '-Dminecraft.launcher.version=0.1.0',
    '-cp',
    fullClasspath,
    mainClass,
    ...extraGameArgs,
    '--username',
    username,
    '--version',
    inst.modloader === 'neoforge' && inst.loader_version ? `neoforge-${inst.loader_version}` : inst.mc_version,
    '--gameDir',
    mcDir,
    '--assetsDir',
    getCommonAssetsDir(),
    '--assetIndex',
    assetIndexId,
    '--uuid',
    uuid,
    '--accessToken',
    accessToken,
    '--clientId',
    clientId,
    '--xuid',
    xuid,
    '--userType',
    'msa',
    '--versionType',
    'release',
    '--userProperties',
    userPropertiesStr,
    '--profileProperties',
    userPropertiesStr
  ]

  log('LoomLauncher', 'INFO', `Exécution de la JVM (RAM: ${minRam}M - ${maxRam}M)...`)

  // 9. Spawn JVM Process
  const child = spawn(javaBin, jvmArgs, {
    cwd: mcDir,
    detached: process.platform !== 'win32',
    env: { ...process.env, MESA_GL_VERSION_OVERRIDE: '4.6' }
  })

  if (child.pid) {
    try {
      fs.writeFileSync(path.join(instDir, 'instance.pid'), String(child.pid))
    } catch {}
  }

  activeProcesses.set(instanceId, child)
  const now = Date.now()
  activeSessions.set(instanceId, {
    instanceId,
    sessionStartTime: now,
    lastTickTime: now,
    child
  })
  ensurePlaytimeHeartbeat()

  function parseLogLine(raw: string): { level: string; source: string; message: string } {
    let level = 'INFO'
    let source = 'Minecraft'
    let message = raw

    const match = raw.match(/^\[([^\]]+)\]\s+\[([^/\]]+)\/([^\]]+)\]:\s+(.*)$/)
    if (match) {
      source = match[2]
      level = match[3].toUpperCase()
      message = match[4]
    } else if (raw.toUpperCase().includes('ERROR') || raw.toUpperCase().includes('EXCEPTION')) {
      level = 'ERROR'
    } else if (raw.toUpperCase().includes('WARN')) {
      level = 'WARN'
    }
    return { level, source, message }
  }

  let logBatch: Array<{ level: string; source: string; message: string; timestamp: string }> = []
  let logBatchTimer: NodeJS.Timeout | null = null

  const flushLogBatch = () => {
    if (logBatchTimer) {
      clearTimeout(logBatchTimer)
      logBatchTimer = null
    }
    if (logBatch.length === 0) return
    const batch = logBatch
    logBatch = []
    broadcastEvent('minecraft-log-batch', {
      instance_id: instanceId,
      entries: batch
    })
  }

  const queueLogLine = (source: string, level: string, message: string) => {
    logBatch.push({
      source,
      level,
      message,
      timestamp: new Date().toLocaleTimeString()
    })
    if (logBatch.length >= 25) {
      flushLogBatch()
    } else if (!logBatchTimer) {
      logBatchTimer = setTimeout(flushLogBatch, 60)
    }
  }

  child.stdout?.on('data', (data: Buffer) => {
    const lines = data.toString('utf8').split('\n')
    for (const line of lines) {
      if (!line.trim()) continue
      const { level, source, message } = parseLogLine(line)
      queueLogLine(source, level, message)
    }
  })

  child.stderr?.on('data', (data: Buffer) => {
    const lines = data.toString('utf8').split('\n')
    for (const line of lines) {
      if (!line.trim()) continue
      queueLogLine('JVM/stderr', 'ERROR', line)
    }
  })

  const cleanupProcess = () => {
    activeProcesses.delete(instanceId)
    try {
      const pidFile = path.join(instDir, 'instance.pid')
      if (fs.existsSync(pidFile)) fs.unlinkSync(pidFile)
    } catch {}
  }

  child.on('exit', (code) => {
    flushLogBatch()
    finalizeSession(instanceId)
    cleanupProcess()
    log('LoomLauncher', code === 0 ? 'INFO' : 'WARN', `Processus Minecraft terminé (code de sortie: ${code}).`)
    broadcastEvent('minecraft-exit', {
      instance_id: instanceId,
      exit_code: code
    })
  })

  child.on('error', (err) => {
    flushLogBatch()
    finalizeSession(instanceId)
    cleanupProcess()
    log('LoomLauncher', 'ERROR', `Erreur critique lors du lancement de la JVM : ${err.message}`)
    broadcastEvent('minecraft-exit', {
      instance_id: instanceId,
      exit_code: 1
    })
  })
}

export function stopInstance(instanceId: string) {
  const log = (source: string, level: string, message: string) => {
    logInstanceMessage(instanceId, source, level, message)
  }

  const child = activeProcesses.get(instanceId)
  finalizeSession(instanceId)

  const instDir = path.join(getInstancesDir(), instanceId)
  let pid: number | null = child?.pid || null

  // If child process object is not found (e.g. server reloaded), read instance.pid
  if (!pid) {
    const pidFile = path.join(instDir, 'instance.pid')
    if (fs.existsSync(pidFile)) {
      try {
        const fileContent = fs.readFileSync(pidFile, 'utf8').trim()
        const parsed = parseInt(fileContent, 10)
        if (!isNaN(parsed) && parsed > 0) {
          pid = parsed
        }
      } catch {}
    }
  }

  log('LoomLauncher', 'INFO', `Arrêt demandé pour l'instance "${instanceId}"${pid ? ` (PID: ${pid})` : ''}...`)

  if (pid) {
    const targetPid = pid

    const sendSignal = (sig: NodeJS.Signals) => {
      try {
        if (process.platform !== 'win32') {
          process.kill(-targetPid, sig)
        } else {
          process.kill(targetPid, sig)
        }
      } catch {
        try {
          process.kill(targetPid, sig)
        } catch {}
      }
      if (child) {
        try { child.kill(sig) } catch {}
      }
    }

    // 1. Send graceful SIGTERM first
    sendSignal('SIGTERM')

    // 2. If still alive after 2s, force SIGKILL on process group + child processes
    const killTimeout = setTimeout(() => {
      try {
        process.kill(targetPid, 0)
        log('LoomLauncher', 'WARN', `Le processus Minecraft (PID: ${targetPid}) n'a pas répondu au SIGTERM, envoi forcé de SIGKILL...`)
        sendSignal('SIGKILL')

        if (process.platform !== 'win32') {
          try {
            execSync(`pkill -9 -P ${targetPid}`, { stdio: 'ignore' })
          } catch {}
        }
      } catch {
        // Process is already terminated
      }

      activeProcesses.delete(instanceId)
      try {
        const pidFile = path.join(instDir, 'instance.pid')
        if (fs.existsSync(pidFile)) fs.unlinkSync(pidFile)
      } catch {}

      broadcastEvent('minecraft-exit', {
        instance_id: instanceId,
        exit_code: 137
      })
    }, 2000)

    if (child) {
      child.once('exit', () => {
        clearTimeout(killTimeout)
        activeProcesses.delete(instanceId)
        try {
          const pidFile = path.join(instDir, 'instance.pid')
          if (fs.existsSync(pidFile)) fs.unlinkSync(pidFile)
        } catch {}
      })
    }
  } else {
    // If no PID found, search for lingering Java process on Linux with this instance directory
    if (process.platform !== 'win32') {
      try {
        const out = execSync(`pgrep -f "${instDir}"`, { encoding: 'utf8' }).trim()
        if (out) {
          const pids = out.split('\n').map((p) => p.trim()).filter(Boolean)
          for (const p of pids) {
            const num = parseInt(p, 10)
            if (!isNaN(num) && num !== process.pid) {
              log('LoomLauncher', 'WARN', `Arrêt du processus Java orphelin (PID: ${num})...`)
              try { process.kill(num, 'SIGKILL') } catch {}
            }
          }
        }
      } catch {}
    }
    activeProcesses.delete(instanceId)
    broadcastEvent('minecraft-exit', {
      instance_id: instanceId,
      exit_code: 0
    })
  }
}

// --- SAVES / WORLDS MANAGEMENT ---
export interface InstanceSaveInfo {
  folderName: string
  name: string
  lastPlayed: number
  gameMode: string
  versionName?: string
  sizeBytes: number
  hasIcon: boolean
  iconDataUrl?: string
  isHardcore: boolean
  difficulty?: string
}

function getDirSizeBytes(dirPath: string): number {
  if (!fs.existsSync(dirPath)) return 0
  let total = 0
  try {
    const entries = fs.readdirSync(dirPath, { withFileTypes: true })
    for (const e of entries) {
      const full = path.join(dirPath, e.name)
      if (e.isDirectory()) {
        total += getDirSizeBytes(full)
      } else if (e.isFile()) {
        try {
          total += fs.statSync(full).size
        } catch {}
      }
    }
  } catch {}
  return total
}

function parseLevelDat(levelDatPath: string): Partial<InstanceSaveInfo> {
  if (!fs.existsSync(levelDatPath)) return {}
  try {
    const raw = fs.readFileSync(levelDatPath)
    let buffer: Buffer
    try {
      buffer = zlib.gunzipSync(raw)
    } catch {
      buffer = raw
    }

    const result: Partial<InstanceSaveInfo> = {}

    const levelNameIdx = buffer.indexOf('LevelName')
    if (levelNameIdx !== -1) {
      const after = levelNameIdx + 'LevelName'.length
      if (after + 2 <= buffer.length) {
        const len = buffer.readUInt16BE(after)
        if (len > 0 && len < 200 && after + 2 + len <= buffer.length) {
          result.name = buffer.slice(after + 2, after + 2 + len).toString('utf8')
        }
      }
    }

    const gameTypeIdx = buffer.indexOf('GameType')
    if (gameTypeIdx !== -1) {
      const after = gameTypeIdx + 'GameType'.length
      if (after + 4 <= buffer.length) {
        const typeInt = buffer.readInt32BE(after)
        if (typeInt === 1) result.gameMode = 'Créatif'
        else if (typeInt === 2) result.gameMode = 'Aventure'
        else if (typeInt === 3) result.gameMode = 'Spectateur'
        else result.gameMode = 'Survie'
      }
    }

    const hardcoreIdx = buffer.indexOf('hardcore')
    if (hardcoreIdx !== -1) {
      const after = hardcoreIdx + 'hardcore'.length
      if (after < buffer.length) {
        const val = buffer[after]
        result.isHardcore = val === 1
        if (result.isHardcore) result.gameMode = 'Hardcore'
      }
    }

    const nameVerIdx = buffer.indexOf('Version')
    if (nameVerIdx !== -1) {
      const sub = buffer.slice(nameVerIdx, Math.min(buffer.length, nameVerIdx + 300))
      const nIdx = sub.indexOf('Name')
      if (nIdx !== -1) {
        const afterN = nIdx + 4
        if (afterN + 2 <= sub.length) {
          const vLen = sub.readUInt16BE(afterN)
          if (vLen > 0 && vLen < 50 && afterN + 2 + vLen <= sub.length) {
            result.versionName = sub.slice(afterN + 2, afterN + 2 + vLen).toString('utf8')
          }
        }
      }
    }

    const lastPlayedIdx = buffer.indexOf('LastPlayed')
    if (lastPlayedIdx !== -1) {
      const after = lastPlayedIdx + 'LastPlayed'.length
      if (after + 8 <= buffer.length) {
        const high = buffer.readInt32BE(after)
        const low = buffer.readUInt32BE(after + 4)
        const ts = high * 4294967296 + low
        if (ts > 1000000000000 && ts < 3000000000000) {
          result.lastPlayed = ts
        }
      }
    }

    return result
  } catch (e) {
    console.warn('[Loom::Saves] Warning parsing level.dat:', e)
    return {}
  }
}

export function listInstanceSaves(instanceId: string): InstanceSaveInfo[] {
  const savesDir = path.join(getInstancesDir(), instanceId, '.minecraft', 'saves')
  if (!fs.existsSync(savesDir)) return []

  const results: InstanceSaveInfo[] = []
  try {
    const folders = fs.readdirSync(savesDir, { withFileTypes: true }).filter((d) => d.isDirectory())
    for (const f of folders) {
      const worldPath = path.join(savesDir, f.name)
      const levelDatPath = path.join(worldPath, 'level.dat')
      const iconPath = path.join(worldPath, 'icon.png')

      let parsed: Partial<InstanceSaveInfo> = {}
      if (fs.existsSync(levelDatPath)) {
        parsed = parseLevelDat(levelDatPath)
      }

      let iconDataUrl: string | undefined = undefined
      let hasIcon = false
      if (fs.existsSync(iconPath)) {
        try {
          const iconBuf = fs.readFileSync(iconPath)
          iconDataUrl = `data:image/png;base64,${iconBuf.toString('base64')}`
          hasIcon = true
        } catch {}
      }

      let mtime = Date.now()
      try {
        const st = fs.statSync(levelDatPath)
        mtime = st.mtimeMs
      } catch {
        try {
          const st = fs.statSync(worldPath)
          mtime = st.mtimeMs
        } catch {}
      }

      const sizeBytes = getDirSizeBytes(worldPath)

      results.push({
        folderName: f.name,
        name: parsed.name || f.name,
        lastPlayed: parsed.lastPlayed || mtime,
        gameMode: parsed.gameMode || 'Survie',
        versionName: parsed.versionName,
        sizeBytes,
        hasIcon,
        iconDataUrl,
        isHardcore: Boolean(parsed.isHardcore),
        difficulty: parsed.difficulty
      })
    }
  } catch (e) {
    console.error('[Loom::Saves] Error listing saves:', e)
  }

  return results.sort((a, b) => b.lastPlayed - a.lastPlayed)
}

export function backupInstanceSave(instanceId: string, folderName: string): { success: boolean; backupName?: string } {
  const savesDir = path.join(getInstancesDir(), instanceId, '.minecraft', 'saves')
  const src = path.join(savesDir, folderName)
  if (!fs.existsSync(src)) throw new Error('Monde introuvable')

  const backupsDir = path.join(getInstancesDir(), instanceId, '.minecraft', 'backups')
  if (!fs.existsSync(backupsDir)) fs.mkdirSync(backupsDir, { recursive: true })

  const dateStr = new Date().toISOString().replace(/[:.]/g, '-').slice(0, 19)
  const backupFolder = path.join(backupsDir, `${folderName}_backup_${dateStr}`)

  fs.cpSync(src, backupFolder, { recursive: true })
  return { success: true, backupName: path.basename(backupFolder) }
}

export function deleteInstanceSave(instanceId: string, folderName: string): boolean {
  const savesDir = path.join(getInstancesDir(), instanceId, '.minecraft', 'saves')
  const target = path.join(savesDir, folderName)
  if (fs.existsSync(target)) {
    fs.rmSync(target, { recursive: true, force: true })
    return true
  }
  return false
}

// --- SERVERS & SERVER PING ---
export interface InstanceServerEntry {
  id: string
  name: string
  ip: string
  icon?: string
  acceptTextures?: number
}

export interface ServerPingResult {
  online: boolean
  ip: string
  port: number
  pingMs: number
  version?: { name: string; protocol: number }
  motd: { raw: string; clean: string; html: string }
  players: {
    online: number
    max: number
    sample: Array<{ name: string; id?: string }>
  }
  icon?: string
}

export function mcFormatToHtml(text: string): string {
  if (!text) return ''
  const colorMap: Record<string, string> = {
    '0': '#000000', '1': '#0000AA', '2': '#00AA00', '3': '#00AAAA',
    '4': '#AA0000', '5': '#AA00AA', '6': '#FFAA00', '7': '#AAAAAA',
    '8': '#555555', '9': '#5555FF', 'a': '#55FF55', 'b': '#55FFFF',
    'c': '#FF5555', 'd': '#FF55FF', 'e': '#FFFF55', 'f': '#FFFFFF'
  }

  let html = ''
  let currentColor = ''
  let isBold = false
  let isItalic = false
  let isUnderline = false
  let isStrikethrough = false

  const parts = text.split(/(§[0-9a-fk-or])/gi)
  for (const part of parts) {
    if (part.startsWith('§')) {
      const code = part[1].toLowerCase()
      if (colorMap[code]) {
        currentColor = colorMap[code]
      } else if (code === 'l') {
        isBold = true
      } else if (code === 'o') {
        isItalic = true
      } else if (code === 'n') {
        isUnderline = true
      } else if (code === 'm') {
        isStrikethrough = true
      } else if (code === 'r') {
        currentColor = ''
        isBold = false
        isItalic = false
        isUnderline = false
        isStrikethrough = false
      }
    } else if (part.length > 0) {
      let styles = ''
      if (currentColor) styles += `color: ${currentColor};`
      if (isBold) styles += 'font-weight: bold;'
      if (isItalic) styles += 'font-style: italic;'
      if (isUnderline) styles += 'text-decoration: underline;'
      if (isStrikethrough) styles += 'text-decoration: line-through;'

      const escaped = part
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;')
        .replace(/\n/g, '<br/>')

      if (styles) {
        html += `<span style="${styles}">${escaped}</span>`
      } else {
        html += escaped
      }
    }
  }
  return html
}

function parseServersDatFile(buffer: Buffer): InstanceServerEntry[] {
  const servers: InstanceServerEntry[] = []
  try {
    let pos = buffer.indexOf('servers')
    if (pos === -1) return servers
    pos += 7
    const itemType = buffer.readUInt8(pos)
    pos += 1
    const count = buffer.readInt32BE(pos)
    pos += 4

    for (let i = 0; i < count; i++) {
      const srv: InstanceServerEntry = { id: `srv-${i}`, name: '', ip: '' }
      while (pos < buffer.length) {
        const tagType = buffer.readUInt8(pos)
        pos += 1
        if (tagType === 0) break
        const nameLen = buffer.readUInt16BE(pos)
        pos += 2
        const tagName = buffer.slice(pos, pos + nameLen).toString('utf8')
        pos += nameLen
        if (tagType === 8) {
          const strLen = buffer.readUInt16BE(pos)
          pos += 2
          const strVal = buffer.slice(pos, pos + strLen).toString('utf8')
          pos += strLen
          if (tagName === 'name') srv.name = strVal
          else if (tagName === 'ip') srv.ip = strVal
          else if (tagName === 'icon') srv.icon = strVal
        } else if (tagType === 1) {
          const bVal = buffer.readUInt8(pos)
          pos += 1
          if (tagName === 'acceptTextures') srv.acceptTextures = bVal
        } else if (tagType === 3) {
          pos += 4
        } else {
          break
        }
      }
      if (srv.name || srv.ip) {
        servers.push(srv)
      }
    }
  } catch (e) {
    console.warn('[Loom::Servers] Error parsing servers.dat:', e)
  }
  return servers
}

function serializeServersDatBuffer(servers: InstanceServerEntry[]): Buffer {
  const parts: Buffer[] = []
  parts.push(Buffer.from([0x0a, 0x00, 0x00]))
  const serversTag = Buffer.from('servers', 'utf8')
  const listHeader = Buffer.alloc(1 + 2 + serversTag.length + 1 + 4)
  let p = 0
  listHeader.writeUInt8(0x09, p++)
  listHeader.writeUInt16BE(serversTag.length, p)
  p += 2
  serversTag.copy(listHeader, p)
  p += serversTag.length
  listHeader.writeUInt8(0x0a, p++)
  listHeader.writeInt32BE(servers.length, p)
  p += 4
  parts.push(listHeader)

  for (const s of servers) {
    const sName = Buffer.from(s.name || '', 'utf8')
    const sIp = Buffer.from(s.ip || '', 'utf8')
    const sIcon = s.icon ? Buffer.from(s.icon, 'utf8') : null

    const nh = Buffer.alloc(1 + 2 + 4 + 2)
    let hp = 0
    nh.writeUInt8(0x08, hp++)
    nh.writeUInt16BE(4, hp)
    hp += 2
    Buffer.from('name').copy(nh, hp)
    hp += 4
    nh.writeUInt16BE(sName.length, hp)
    parts.push(nh, sName)

    const iph = Buffer.alloc(1 + 2 + 2 + 2)
    hp = 0
    iph.writeUInt8(0x08, hp++)
    iph.writeUInt16BE(2, hp)
    hp += 2
    Buffer.from('ip').copy(iph, hp)
    hp += 2
    iph.writeUInt16BE(sIp.length, hp)
    parts.push(iph, sIp)

    if (sIcon) {
      const ich = Buffer.alloc(1 + 2 + 4 + 2)
      hp = 0
      ich.writeUInt8(0x08, hp++)
      ich.writeUInt16BE(4, hp)
      hp += 2
      Buffer.from('icon').copy(ich, hp)
      hp += 4
      ich.writeUInt16BE(sIcon.length, hp)
      parts.push(ich, sIcon)
    }

    const ath = Buffer.alloc(1 + 2 + 14 + 1)
    hp = 0
    ath.writeUInt8(0x01, hp++)
    ath.writeUInt16BE(14, hp)
    hp += 2
    Buffer.from('acceptTextures').copy(ath, hp)
    hp += 14
    ath.writeUInt8(s.acceptTextures !== undefined ? s.acceptTextures : 1, hp)
    parts.push(ath)

    parts.push(Buffer.from([0x00]))
  }

  parts.push(Buffer.from([0x00]))
  return Buffer.concat(parts)
}

export function listInstanceServers(instanceId: string): InstanceServerEntry[] {
  const dotMc = path.join(getInstancesDir(), instanceId, '.minecraft')
  const jsonPath = path.join(dotMc, 'servers.json')
  const datPath = path.join(dotMc, 'servers.dat')

  if (fs.existsSync(jsonPath)) {
    try {
      return JSON.parse(fs.readFileSync(jsonPath, 'utf8'))
    } catch {}
  }

  if (fs.existsSync(datPath)) {
    try {
      const buf = fs.readFileSync(datPath)
      const list = parseServersDatFile(buf)
      fs.writeFileSync(jsonPath, JSON.stringify(list, null, 2), 'utf8')
      return list
    } catch {}
  }

  return []
}

export function saveInstanceServers(instanceId: string, servers: InstanceServerEntry[]): void {
  const dotMc = path.join(getInstancesDir(), instanceId, '.minecraft')
  if (!fs.existsSync(dotMc)) fs.mkdirSync(dotMc, { recursive: true })
  const jsonPath = path.join(dotMc, 'servers.json')
  const datPath = path.join(dotMc, 'servers.dat')

  fs.writeFileSync(jsonPath, JSON.stringify(servers, null, 2), 'utf8')
  try {
    const datBuf = serializeServersDatBuffer(servers)
    fs.writeFileSync(datPath, datBuf)
  } catch (e) {
    console.warn('[Loom::Servers] Error writing servers.dat:', e)
  }
}

function writeVarInt(value: number): Buffer {
  const bytes: number[] = []
  while (true) {
    if ((value & ~0x7f) === 0) {
      bytes.push(value)
      break
    }
    bytes.push((value & 0x7f) | 0x80)
    value >>>= 7
  }
  return Buffer.from(bytes)
}

function readVarInt(buf: Buffer, offset = 0): { value: number; length: number } | null {
  let numRead = 0
  let result = 0
  let read: number
  do {
    if (offset + numRead >= buf.length) return null
    read = buf.readUInt8(offset + numRead)
    const value = read & 0b01111111
    result |= value << (7 * numRead)
    numRead++
    if (numRead > 5) return null
  } while ((read & 0b10000000) !== 0)
  return { value: result, length: numRead }
}

function pingDirectTcp(host: string, port = 25565, timeout = 3000): Promise<{ pingMs: number; json: any } | null> {
  return new Promise((resolve) => {
    const start = Date.now()
    const client = new net.Socket()
    let received = Buffer.alloc(0)

    client.setTimeout(timeout)

    client.connect(port, host, () => {
      const hostBuf = Buffer.from(host, 'utf8')
      const portBuf = Buffer.alloc(2)
      portBuf.writeUInt16BE(port, 0)

      const packetData = Buffer.concat([
        Buffer.from([0x00]),
        writeVarInt(765),
        writeVarInt(hostBuf.length),
        hostBuf,
        portBuf,
        writeVarInt(1)
      ])

      const handshakePacket = Buffer.concat([writeVarInt(packetData.length), packetData])
      const requestPacket = Buffer.from([0x01, 0x00])

      client.write(Buffer.concat([handshakePacket, requestPacket]))
    })

    client.on('data', (chunk) => {
      received = Buffer.concat([received, chunk])
      try {
        const packetLen = readVarInt(received, 0)
        if (!packetLen) return
        let pos = packetLen.length
        const packetId = readVarInt(received, pos)
        if (!packetId || packetId.value !== 0) return
        pos += packetId.length
        const strLen = readVarInt(received, pos)
        if (!strLen) return
        pos += strLen.length
        if (received.length >= pos + strLen.value) {
          const jsonStr = received.slice(pos, pos + strLen.value).toString('utf8')
          const pingMs = Date.now() - start
          client.destroy()
          try {
            resolve({ pingMs, json: JSON.parse(jsonStr) })
          } catch {
            resolve(null)
          }
        }
      } catch {
        resolve(null)
      }
    })

    client.on('error', () => resolve(null))
    client.on('timeout', () => {
      client.destroy()
      resolve(null)
    })
  })
}

export async function pingMinecraftServer(hostStr: string, defaultPort = 25565): Promise<ServerPingResult> {
  let host = hostStr.trim()
  let port = defaultPort
  if (host.includes(':')) {
    const parts = host.split(':')
    host = parts[0]
    port = parseInt(parts[1], 10) || defaultPort
  }

  try {
    const direct = await pingDirectTcp(host, port, 2500)
    if (direct && direct.json) {
      const data = direct.json
      let motdRaw = ''
      if (typeof data.description === 'string') {
        motdRaw = data.description
      } else if (data.description) {
        motdRaw = data.description.text || ''
        if (Array.isArray(data.description.extra)) {
          for (const ex of data.description.extra) {
            motdRaw += typeof ex === 'string' ? ex : ex.text || ''
          }
        }
      }

      const motdClean = motdRaw.replace(/§[0-9a-fk-or]/gi, '')
      const motdHtml = mcFormatToHtml(motdRaw)

      const sampleList: Array<{ name: string; id?: string }> = []
      if (data.players?.sample && Array.isArray(data.players.sample)) {
        for (const p of data.players.sample) {
          if (p.name) sampleList.push({ name: p.name, id: p.id })
        }
      }

      return {
        online: true,
        ip: host,
        port,
        pingMs: direct.pingMs,
        version: data.version || { name: 'Minecraft', protocol: 0 },
        motd: { raw: motdRaw, clean: motdClean, html: motdHtml },
        players: {
          online: Number(data.players?.online || 0),
          max: Number(data.players?.max || 0),
          sample: sampleList
        },
        icon: data.favicon || undefined
      }
    }
  } catch {}

  try {
    const start = Date.now()
    const apiRes = await httpsGetJson(`https://api.mcsrvstat.us/3/${encodeURIComponent(host)}:${port}`)
    const pingMs = Date.now() - start
    const json = apiRes?.json
    if (json && json.online) {
      const rawLines = json.motd?.raw || []
      const cleanLines = json.motd?.clean || []
      const htmlLines = json.motd?.html || []

      const sampleList: Array<{ name: string; id?: string }> = []
      if (json.players?.list && Array.isArray(json.players.list)) {
        for (const p of json.players.list) {
          sampleList.push({ name: p.name, id: p.uuid })
        }
      }

      return {
        online: true,
        ip: host,
        port: json.port || port,
        pingMs,
        version: { name: json.version || 'Minecraft', protocol: json.protocol || 0 },
        motd: {
          raw: rawLines.join('\n'),
          clean: cleanLines.join('\n'),
          html: htmlLines.join('<br/>') || mcFormatToHtml(rawLines.join('\n'))
        },
        players: {
          online: Number(json.players?.online || 0),
          max: Number(json.players?.max || 0),
          sample: sampleList
        },
        icon: json.icon || undefined
      }
    }
  } catch {}

  return {
    online: false,
    ip: host,
    port,
    pingMs: 999,
    motd: { raw: '', clean: 'Hors-ligne ou introuvable', html: '<span style="color:#FF5555">Hors-ligne ou introuvable</span>' },
    players: { online: 0, max: 0, sample: [] }
  }
}

// --- LOGS & CRASH REPORTS ---
export interface LogFileInfo {
  name: string
  path: string
  sizeBytes: number
  mtime: string
  isCompressed: boolean
}

export function listInstanceLogs(instanceId: string): { logs: LogFileInfo[]; crashReports: LogFileInfo[] } {
  const dotMc = path.join(getInstancesDir(), instanceId, '.minecraft')
  const logsDir = path.join(dotMc, 'logs')
  const crashDir = path.join(dotMc, 'crash-reports')

  const logs: LogFileInfo[] = []
  if (fs.existsSync(logsDir)) {
    try {
      const files = fs.readdirSync(logsDir, { withFileTypes: true })
      for (const f of files) {
        if (!f.isFile()) continue
        if (!f.name.endsWith('.log') && !f.name.endsWith('.log.gz') && !f.name.endsWith('.txt')) continue
        const full = path.join(logsDir, f.name)
        const st = fs.statSync(full)
        logs.push({
          name: f.name,
          path: f.name,
          sizeBytes: st.size,
          mtime: st.mtime.toISOString(),
          isCompressed: f.name.endsWith('.gz')
        })
      }
    } catch (e) {
      console.warn('[Loom::Logs] Error reading logs dir:', e)
    }
  }

  logs.sort((a, b) => {
    if (a.name === 'latest.log') return -1
    if (b.name === 'latest.log') return 1
    return new Date(b.mtime).getTime() - new Date(a.mtime).getTime()
  })

  const crashReports: LogFileInfo[] = []
  if (fs.existsSync(crashDir)) {
    try {
      const files = fs.readdirSync(crashDir, { withFileTypes: true })
      for (const f of files) {
        if (!f.isFile() || !f.name.endsWith('.txt')) continue
        const full = path.join(crashDir, f.name)
        const st = fs.statSync(full)
        crashReports.push({
          name: f.name,
          path: f.name,
          sizeBytes: st.size,
          mtime: st.mtime.toISOString(),
          isCompressed: false
        })
      }
    } catch (e) {
      console.warn('[Loom::Logs] Error reading crash reports dir:', e)
    }
  }

  crashReports.sort((a, b) => new Date(b.mtime).getTime() - new Date(a.mtime).getTime())

  return { logs, crashReports }
}

export function readInstanceLogContent(instanceId: string, fileName: string, isCrash = false): string {
  const dotMc = path.join(getInstancesDir(), instanceId, '.minecraft')
  const baseDir = isCrash ? path.join(dotMc, 'crash-reports') : path.join(dotMc, 'logs')
  const safeName = path.basename(fileName)
  const fullPath = path.join(baseDir, safeName)

  if (!fs.existsSync(fullPath)) {
    throw new Error(`Fichier introuvable: ${safeName}`)
  }

  if (safeName.endsWith('.gz')) {
    const raw = fs.readFileSync(fullPath)
    return zlib.gunzipSync(raw).toString('utf8')
  }

  const st = fs.statSync(fullPath)
  if (st.size > 2 * 1024 * 1024) {
    const fd = fs.openSync(fullPath, 'r')
    const startPos = st.size - 2 * 1024 * 1024
    const buf = Buffer.alloc(2 * 1024 * 1024)
    fs.readSync(fd, buf, 0, 2 * 1024 * 1024, startPos)
    fs.closeSync(fd)
    return `--- [Affichage tronqué : derniers 2 Mo du fichier (${(st.size / 1024 / 1024).toFixed(1)} Mo au total)] ---\n\n` + buf.toString('utf8')
  }

  return fs.readFileSync(fullPath, 'utf8')
}

export function clearInstanceLogs(instanceId: string): void {
  const dotMc = path.join(getInstancesDir(), instanceId, '.minecraft')
  const logsDir = path.join(dotMc, 'logs')
  if (fs.existsSync(logsDir)) {
    try {
      const files = fs.readdirSync(logsDir)
      for (const f of files) {
        if (f.endsWith('.gz') || f.endsWith('.log.old')) {
          fs.rmSync(path.join(logsDir, f), { force: true })
        }
      }
    } catch {}
  }
}

// --- CONFIGURATION FILES ---
export interface ConfigFileInfo {
  name: string
  relPath: string
  extension: string
  sizeBytes: number
  mtime: string
  subfolder: string
}

export function listInstanceConfigs(instanceId: string): ConfigFileInfo[] {
  const configDir = path.join(getInstancesDir(), instanceId, '.minecraft', 'config')
  if (!fs.existsSync(configDir)) return []

  const results: ConfigFileInfo[] = []
  const allowedExts = new Set(['toml', 'json', 'json5', 'yaml', 'yml', 'cfg', 'properties', 'txt', 'snbt', 'ini', 'conf'])

  function walk(currentDir: string, currentRel: string, depth = 0) {
    if (depth > 4) return
    try {
      const entries = fs.readdirSync(currentDir, { withFileTypes: true })
      for (const e of entries) {
        if (e.name.startsWith('.')) continue
        const subRel = currentRel ? `${currentRel}/${e.name}` : e.name
        const full = path.join(currentDir, e.name)

        if (e.isDirectory()) {
          walk(full, subRel, depth + 1)
        } else if (e.isFile()) {
          const ext = path.extname(e.name).slice(1).toLowerCase()
          if (allowedExts.has(ext)) {
            try {
              const st = fs.statSync(full)
              results.push({
                name: e.name,
                relPath: subRel,
                extension: ext,
                sizeBytes: st.size,
                mtime: st.mtime.toISOString(),
                subfolder: currentRel || '/'
              })
            } catch {}
          }
        }
      }
    } catch (err) {
      console.warn('[Loom::Config] Error scanning config dir:', err)
    }
  }

  walk(configDir, '')
  return results.sort((a, b) => a.relPath.localeCompare(b.relPath))
}

export function readInstanceConfigContent(instanceId: string, relPath: string): string {
  const configDir = path.join(getInstancesDir(), instanceId, '.minecraft', 'config')
  const cleanRel = path.normalize(relPath).replace(/^(\.\.[\/\\])+/, '')
  const fullPath = path.join(configDir, cleanRel)

  if (!fullPath.startsWith(configDir)) {
    throw new Error('Chemin de configuration interdit (hors dossier config)')
  }
  if (!fs.existsSync(fullPath)) {
    throw new Error(`Fichier introuvable: ${relPath}`)
  }

  return fs.readFileSync(fullPath, 'utf8')
}

export function saveInstanceConfigContent(instanceId: string, relPath: string, content: string): void {
  const configDir = path.join(getInstancesDir(), instanceId, '.minecraft', 'config')
  const cleanRel = path.normalize(relPath).replace(/^(\.\.[\/\\])+/, '')
  const fullPath = path.join(configDir, cleanRel)

  if (!fullPath.startsWith(configDir)) {
    throw new Error('Chemin de configuration interdit (hors dossier config)')
  }

  const parent = path.dirname(fullPath)
  if (!fs.existsSync(parent)) fs.mkdirSync(parent, { recursive: true })

  fs.writeFileSync(fullPath, content, 'utf8')
}

export function deleteInstanceConfigFile(instanceId: string, relPath: string): boolean {
  const configDir = path.join(getInstancesDir(), instanceId, '.minecraft', 'config')
  const cleanRel = path.normalize(relPath).replace(/^(\.\.[\/\\])+/, '')
  const fullPath = path.join(configDir, cleanRel)

  if (!fullPath.startsWith(configDir)) {
    throw new Error('Chemin interdit')
  }

  if (fs.existsSync(fullPath)) {
    fs.rmSync(fullPath, { force: true })
    return true
  }
  return false
}

// --- HTTP Request Router for Vite Middleware ---
export async function handleMinecraftApi(req: IncomingMessage, res: ServerResponse): Promise<boolean> {
  const url = req.url || ''
  if (!url.startsWith('/api/minecraft')) {
    return false
  }

  cleanupStaleModDownloads()

  // SSE event stream
  if (url === '/api/minecraft/events') {
    res.writeHead(200, {
      'Content-Type': 'text/event-stream',
      'Cache-Control': 'no-cache',
      Connection: 'keep-alive',
      'Access-Control-Allow-Origin': '*'
    })
    res.write('event: connected\ndata: {}\n\n')
    for (const dl of activeModDownloads.values()) {
      res.write(`event: minecraft-download-progress\ndata: ${JSON.stringify(dl)}\n\n`)
    }
    sseClients.add(res)
    req.on('close', () => sseClients.delete(res))
    return true
  }

  res.setHeader('Content-Type', 'application/json')

  // GET /api/minecraft/downloads
  if (req.method === 'GET' && url === '/api/minecraft/downloads') {
    res.end(JSON.stringify(Array.from(activeModDownloads.values())))
    return true
  }

  // POST /api/minecraft/downloads/clear
  if (req.method === 'POST' && url === '/api/minecraft/downloads/clear') {
    activeModDownloads.clear()
    res.end(JSON.stringify({ success: true }))
    return true
  }

  // Helper to read JSON body
  const readBody = async (): Promise<any> => {
    return new Promise((resolve) => {
      let body = ''
      req.on('data', (chunk) => (body += chunk))
      req.on('end', () => {
        try {
          resolve(body ? JSON.parse(body) : {})
        } catch {
          resolve({})
        }
      })
    })
  }

  try {
    // 0. POST /api/minecraft/servers/ping
    if (req.method === 'POST' && url === '/api/minecraft/servers/ping') {
      const body = await readBody().catch(() => ({}))
      const result = await pingMinecraftServer(body.ip || '', body.port || 25565)
      res.end(JSON.stringify(result))
      return true
    }

    // 0. GET /api/minecraft/stats
    if (req.method === 'GET' && url === '/api/minecraft/stats') {
      const stats = getGlobalMinecraftStats()
      res.end(
        JSON.stringify({
          ...stats,
          playtime_minutes: Math.floor(stats.playtime_seconds / 60)
        })
      )
      return true
    }

    // 1. GET /api/minecraft/instances
    if (req.method === 'GET' && url === '/api/minecraft/instances') {
      const instances = listInstances()
      res.end(JSON.stringify(instances))
      return true
    }

    // 2. POST /api/minecraft/instances
    if (req.method === 'POST' && url === '/api/minecraft/instances') {
      const body = await readBody()
      const created = createInstance(body)
      res.end(JSON.stringify(created))
      return true
    }

    // 3. Modpacks API
    if (req.method === 'GET' && url.startsWith('/api/minecraft/modpacks/search')) {
      const parsedUrl = new URL(url, 'http://localhost')
      const q = parsedUrl.searchParams.get('query') || ''
      const prov = (parsedUrl.searchParams.get('provider') as any) || 'all'
      const limit = Number(parsedUrl.searchParams.get('limit')) || 24
      const packs = await searchModpacks({ query: q, provider: prov, limit })
      res.end(JSON.stringify(packs))
      return true
    }

    if (req.method === 'POST' && url === '/api/minecraft/modpacks/install') {
      const body = await readBody()
      const inst = await installModpackService(body)
      res.end(JSON.stringify({ success: true, instance: inst }))
      return true
    }

    if (req.method === 'POST' && url.startsWith('/api/minecraft/modpacks/import-local')) {
      const parsedUrl = new URL(url, 'http://localhost')
      const nameParam = parsedUrl.searchParams.get('name')
      const rawHeaderName = (req.headers['x-filename'] as string) || ''
      const filename = nameParam || (rawHeaderName ? decodeURIComponent(rawHeaderName) : 'modpack.zip')

      const tmpPath = path.join(os.tmpdir(), `loom-upload-${Date.now()}-${filename}`)
      const fileStream = fs.createWriteStream(tmpPath)

      await new Promise<void>((resolve, reject) => {
        req.pipe(fileStream)
        req.on('end', () => resolve())
        req.on('error', (err) => reject(err))
      })

      const inst = await installModpackService({
        provider: 'local',
        localFilePath: tmpPath,
        instanceName: filename.replace(/\.(mrpack|zip)$/i, '')
      })
      res.end(JSON.stringify({ success: true, instance: inst }))
      return true
    }

    // 4. Java Detection & Matching API
    if (req.method === 'GET' && url.startsWith('/api/minecraft/java/detect')) {
      const runtimes = detectSystemJavaInstallations()
      res.end(JSON.stringify(runtimes))
      return true
    }

    if (req.method === 'GET' && url.startsWith('/api/minecraft/java/required')) {
      const parsedUrl = new URL(url, 'http://localhost')
      const mcVer = parsedUrl.searchParams.get('mcVersion') || '1.20.1'
      const reqMajor = getRequiredJavaVersion(mcVer)
      res.end(JSON.stringify({ mcVersion: mcVer, requiredMajor: reqMajor }))
      return true
    }

    if (req.method === 'POST' && url === '/api/minecraft/java/inspect') {
      const body = await readBody()
      const p = body?.path || ''
      const inspected = inspectJavaBinary(p)
      res.end(JSON.stringify({ valid: Boolean(inspected), runtime: inspected }))
      return true
    }

    // Mod details endpoint
    if (req.method === 'GET' && url.startsWith('/api/minecraft/mods/details')) {
      const parsedUrl = new URL(url, 'http://localhost')
      const provider = parsedUrl.searchParams.get('provider') || 'modrinth'
      const modId = parsedUrl.searchParams.get('modId') || ''
      const details = await getModFullDetails(provider, modId)
      res.end(JSON.stringify(details))
      return true
    }

    // Mod version changelog endpoint
    if (req.method === 'GET' && url.startsWith('/api/minecraft/mods/version-changelog')) {
      const parsedUrl = new URL(url, 'http://localhost')
      const provider = parsedUrl.searchParams.get('provider') || 'curseforge'
      const modId = parsedUrl.searchParams.get('modId') || ''
      const fileId = parsedUrl.searchParams.get('fileId') || ''
      const changelog = await getModVersionChangelog(provider, modId, fileId)
      res.end(JSON.stringify({ changelog }))
      return true
    }

    // Instance sub-routes
    const instMatch = url.match(/^\/api\/minecraft\/instances\/([^/?]+)(.*)$/)
    if (instMatch) {
      const instanceId = instMatch[1]
      const subPath = instMatch[2]

      // DELETE /api/minecraft/instances/:id
      if (req.method === 'DELETE' && !subPath) {
        deleteInstance(instanceId)
        res.end(JSON.stringify({ success: true }))
        return true
      }

      // PATCH /api/minecraft/instances/:id
      if (req.method === 'PATCH' && !subPath) {
        const body = await readBody()
        updateInstanceSettings(instanceId, body)
        res.end(JSON.stringify({ success: true }))
        return true
      }

      // POST /api/minecraft/instances/:id/launch
      if (req.method === 'POST' && subPath === '/launch') {
        const body = await readBody()
        const nowSec = Math.floor(Date.now() / 1000)
        updateInstanceSettings(instanceId, { last_played: nowSec })
        const gStats = getGlobalMinecraftStats()
        gStats.sessions_count += 1
        gStats.last_played = nowSec
        saveGlobalMinecraftStats(gStats)

        launchInstance(instanceId, body.allocatedRamGb).catch((err) => {
          console.error('[Loom::Minecraft] Launch error:', err)
          broadcastEvent('minecraft-log', {
            instance_id: instanceId,
            timestamp: new Date().toLocaleTimeString(),
            level: 'ERROR',
            source: 'LoomLauncher',
            message: `Erreur de lancement: ${err.message}`
          })
          broadcastEvent('minecraft-exit', { instance_id: instanceId, exit_code: 1 })
        })
        res.end(JSON.stringify({ success: true }))
        return true
      }

      // POST /api/minecraft/instances/:id/stop
      if (req.method === 'POST' && subPath === '/stop') {
        stopInstance(instanceId)
        res.end(JSON.stringify({ success: true }))
        return true
      }

      // POST /api/minecraft/instances/:id/reset
      if (req.method === 'POST' && subPath === '/reset') {
        finalizeSession(instanceId)
        stopInstance(instanceId)
        activeProcesses.delete(instanceId)
        broadcastEvent('minecraft-exit', { instance_id: instanceId, exit_code: 0 })
        res.end(JSON.stringify({ success: true }))
        return true
      }

      // POST /api/minecraft/instances/:id/open-folder
      if (req.method === 'POST' && subPath === '/open-folder') {
        const body = await readBody().catch(() => ({}))
        const ok = openInstanceFolder(instanceId, body?.subDir)
        res.end(JSON.stringify({ success: ok }))
        return true
      }

      // GET /api/minecraft/instances/:id/mods
      if (req.method === 'GET' && subPath === '/mods') {
        const mods = listInstanceMods(instanceId)
        res.end(JSON.stringify(mods))
        return true
      }

      // POST /api/minecraft/instances/:id/mods/check-updates
      if (req.method === 'POST' && subPath === '/mods/check-updates') {
        const updatedMods = await checkInstanceModUpdates(instanceId)
        res.end(JSON.stringify(updatedMods))
        return true
      }

      // POST /api/minecraft/instances/:id/mods/:modId/update
      const updateModMatch = subPath?.match(/^\/mods\/([^/]+)\/update$/)
      if (req.method === 'POST' && updateModMatch) {
        const modId = updateModMatch[1]
        const body = await readBody().catch(() => ({}))
        const updatedMod = await updateModInInstance(instanceId, modId, body?.fileId, body?.downloadUrl)
        res.end(JSON.stringify(updatedMod))
        return true
      }

      // POST /api/minecraft/instances/:id/mods/check-dependencies
      if (req.method === 'POST' && subPath === '/mods/check-dependencies') {
        const body = await readBody()
        const result = await checkModDependencies(instanceId, body.provider, body.remoteModId)
        res.end(JSON.stringify(result))
        return true
      }

      // POST /api/minecraft/instances/:id/mods/install
      if (req.method === 'POST' && subPath === '/mods/install') {
        const body = await readBody()
        const mod = await installModToInstance({
          instanceId,
          provider: body.provider,
          remoteModId: body.remoteModId,
          remoteFileId: body.remoteFileId,
          downloadUrl: body.downloadUrl,
          filename: body.filename,
          modName: body.modName,
          iconUrl: body.iconUrl || body.icon_url,
          version: body.version,
          author: body.author,
          description: body.description,
          mcVersion: body.mcVersion,
          modloader: body.modloader,
          contentType: body.contentType || 'mod'
        })
        res.end(JSON.stringify(mod))
        return true
      }

      // POST /api/minecraft/instances/:id/mods/:modId/toggle
      const toggleMatch = subPath.match(/^\/mods\/([^/]+)\/toggle$/)
      if (req.method === 'POST' && toggleMatch) {
        const modId = toggleMatch[1]
        const body = await readBody()
        toggleModState(instanceId, modId, Boolean(body.enabled))
        res.end(JSON.stringify({ success: true }))
        return true
      }

      // DELETE /api/minecraft/instances/:id/mods/:modId
      const delModMatch = subPath.match(/^\/mods\/([^/]+)$/)
      if (req.method === 'DELETE' && delModMatch) {
        const modId = delModMatch[1]
        deleteModFromInstance(instanceId, modId)
        res.end(JSON.stringify({ success: true }))
        return true
      }

      // --- RESOURCE PACKS SUB-ROUTES ---
      // GET /api/minecraft/instances/:id/resourcepacks
      if (req.method === 'GET' && subPath === '/resourcepacks') {
        const packs = listInstanceResourcePacks(instanceId)
        res.end(JSON.stringify(packs))
        return true
      }

      // POST /api/minecraft/instances/:id/resourcepacks/install
      if (req.method === 'POST' && subPath === '/resourcepacks/install') {
        const body = await readBody()
        const pack = await installModToInstance({
          instanceId,
          provider: body.provider,
          remoteModId: body.remoteModId,
          remoteFileId: body.remoteFileId,
          downloadUrl: body.downloadUrl,
          filename: body.filename,
          modName: body.modName,
          iconUrl: body.iconUrl || body.icon_url,
          version: body.version,
          author: body.author,
          description: body.description,
          mcVersion: body.mcVersion,
          contentType: 'resourcepack'
        })
        res.end(JSON.stringify(pack))
        return true
      }

      // POST /api/minecraft/instances/:id/resourcepacks/:rpId/toggle
      const toggleRpMatch = subPath.match(/^\/resourcepacks\/([^/]+)\/toggle$/)
      if (req.method === 'POST' && toggleRpMatch) {
        const rpId = toggleRpMatch[1]
        const body = await readBody()
        toggleResourcePackState(instanceId, rpId, Boolean(body.enabled))
        res.end(JSON.stringify({ success: true }))
        return true
      }

      // DELETE /api/minecraft/instances/:id/resourcepacks/:rpId
      const delRpMatch = subPath.match(/^\/resourcepacks\/([^/]+)$/)
      if (req.method === 'DELETE' && delRpMatch) {
        const rpId = delRpMatch[1]
        deleteResourcePackFromInstance(instanceId, rpId)
        res.end(JSON.stringify({ success: true }))
        return true
      }

      // --- SHADER PACKS SUB-ROUTES ---
      // GET /api/minecraft/instances/:id/shaderpacks
      if (req.method === 'GET' && subPath === '/shaderpacks') {
        const shaders = listInstanceShaderPacks(instanceId)
        res.end(JSON.stringify(shaders))
        return true
      }

      // POST /api/minecraft/instances/:id/shaderpacks/install
      if (req.method === 'POST' && subPath === '/shaderpacks/install') {
        const body = await readBody()
        const shader = await installModToInstance({
          instanceId,
          provider: body.provider,
          remoteModId: body.remoteModId,
          remoteFileId: body.remoteFileId,
          downloadUrl: body.downloadUrl,
          filename: body.filename,
          modName: body.modName,
          iconUrl: body.iconUrl || body.icon_url,
          version: body.version,
          author: body.author,
          description: body.description,
          mcVersion: body.mcVersion,
          contentType: 'shader'
        })
        res.end(JSON.stringify(shader))
        return true
      }

      // POST /api/minecraft/instances/:id/shaderpacks/:spId/toggle
      const toggleSpMatch = subPath.match(/^\/shaderpacks\/([^/]+)\/toggle$/)
      if (req.method === 'POST' && toggleSpMatch) {
        const spId = toggleSpMatch[1]
        const body = await readBody()
        toggleShaderPackState(instanceId, spId, Boolean(body.enabled))
        res.end(JSON.stringify({ success: true }))
        return true
      }

      // DELETE /api/minecraft/instances/:id/shaderpacks/:spId
      const delSpMatch = subPath.match(/^\/shaderpacks\/([^/]+)$/)
      if (req.method === 'DELETE' && delSpMatch) {
        const spId = delSpMatch[1]
        deleteShaderPackFromInstance(instanceId, spId)
        res.end(JSON.stringify({ success: true }))
        return true
      }

      // --- SAVES / MAPS SUB-ROUTES ---
      // GET /api/minecraft/instances/:id/saves
      if (req.method === 'GET' && subPath === '/saves') {
        const saves = listInstanceSaves(instanceId)
        res.end(JSON.stringify(saves))
        return true
      }

      // POST /api/minecraft/instances/:id/saves/:folder/backup
      const backupSaveMatch = subPath.match(/^\/saves\/([^/]+)\/backup$/)
      if (req.method === 'POST' && backupSaveMatch) {
        const folder = decodeURIComponent(backupSaveMatch[1])
        const resObj = backupInstanceSave(instanceId, folder)
        res.end(JSON.stringify(resObj))
        return true
      }

      // DELETE /api/minecraft/instances/:id/saves/:folder
      const delSaveMatch = subPath.match(/^\/saves\/([^/]+)$/)
      if (req.method === 'DELETE' && delSaveMatch) {
        const folder = decodeURIComponent(delSaveMatch[1])
        const ok = deleteInstanceSave(instanceId, folder)
        res.end(JSON.stringify({ success: ok }))
        return true
      }

      // POST /api/minecraft/instances/:id/saves/:folder/open
      const openSaveMatch = subPath.match(/^\/saves\/([^/]+)\/open$/)
      if (req.method === 'POST' && openSaveMatch) {
        const folder = decodeURIComponent(openSaveMatch[1])
        const ok = openInstanceFolder(instanceId, `saves/${folder}`)
        res.end(JSON.stringify({ success: ok }))
        return true
      }

      // --- SERVERS SUB-ROUTES ---
      // GET /api/minecraft/instances/:id/servers
      if (req.method === 'GET' && subPath === '/servers') {
        const servers = listInstanceServers(instanceId)
        res.end(JSON.stringify(servers))
        return true
      }

      // POST /api/minecraft/instances/:id/servers
      if (req.method === 'POST' && subPath === '/servers') {
        const body = await readBody().catch(() => ({}))
        saveInstanceServers(instanceId, body.servers || [])
        res.end(JSON.stringify({ success: true }))
        return true
      }

      // DELETE /api/minecraft/instances/:id/servers/:srvId
      const delSrvMatch = subPath.match(/^\/servers\/([^/]+)$/)
      if (req.method === 'DELETE' && delSrvMatch) {
        const srvId = decodeURIComponent(delSrvMatch[1])
        const current = listInstanceServers(instanceId)
        const updated = current.filter((s) => s.id !== srvId && s.ip !== srvId)
        saveInstanceServers(instanceId, updated)
        res.end(JSON.stringify({ success: true }))
        return true
      }

      // --- LOGS & CRASH REPORTS SUB-ROUTES ---
      // GET /api/minecraft/instances/:id/logs/files
      if (req.method === 'GET' && subPath === '/logs/files') {
        const logData = listInstanceLogs(instanceId)
        res.end(JSON.stringify(logData))
        return true
      }

      // GET /api/minecraft/instances/:id/logs/content
      if (req.method === 'GET' && subPath.startsWith('/logs/content')) {
        const pUrl = new URL(url, 'http://localhost')
        const file = pUrl.searchParams.get('file') || 'latest.log'
        const isCrash = pUrl.searchParams.get('isCrash') === 'true'
        try {
          const text = readInstanceLogContent(instanceId, file, isCrash)
          res.end(JSON.stringify({ content: text }))
        } catch (err: any) {
          res.statusCode = 404
          res.end(JSON.stringify({ error: err.message, content: '' }))
        }
        return true
      }

      // DELETE /api/minecraft/instances/:id/logs
      if (req.method === 'DELETE' && subPath === '/logs') {
        clearInstanceLogs(instanceId)
        res.end(JSON.stringify({ success: true }))
        return true
      }

      // --- CONFIGS SUB-ROUTES ---
      // GET /api/minecraft/instances/:id/configs
      if (req.method === 'GET' && subPath === '/configs') {
        const configs = listInstanceConfigs(instanceId)
        res.end(JSON.stringify(configs))
        return true
      }

      // GET /api/minecraft/instances/:id/configs/read
      if (req.method === 'GET' && subPath.startsWith('/configs/read')) {
        const pUrl = new URL(url, 'http://localhost')
        const relPath = pUrl.searchParams.get('path') || ''
        try {
          const text = readInstanceConfigContent(instanceId, relPath)
          res.end(JSON.stringify({ content: text }))
        } catch (err: any) {
          res.statusCode = 404
          res.end(JSON.stringify({ error: err.message, content: '' }))
        }
        return true
      }

      // POST /api/minecraft/instances/:id/configs/save
      if (req.method === 'POST' && subPath === '/configs/save') {
        const body = await readBody().catch(() => ({}))
        try {
          saveInstanceConfigContent(instanceId, body.relPath, body.content || '')
          res.end(JSON.stringify({ success: true }))
        } catch (err: any) {
          res.statusCode = 400
          res.end(JSON.stringify({ error: err.message }))
        }
        return true
      }

      // DELETE /api/minecraft/instances/:id/configs
      if (req.method === 'DELETE' && subPath.startsWith('/configs')) {
        const pUrl = new URL(url, 'http://localhost')
        const relPath = pUrl.searchParams.get('path') || ''
        const ok = deleteInstanceConfigFile(instanceId, relPath)
        res.end(JSON.stringify({ success: ok }))
        return true
      }
    }

    res.statusCode = 404
    res.end(JSON.stringify({ error: 'Endpoint Minecraft introuvable' }))
    return true
  } catch (err: any) {
    console.error('[Loom::MinecraftApi] Error:', err)
    res.statusCode = 500
    res.end(JSON.stringify({ error: err.message }))
    return true
  }
}
