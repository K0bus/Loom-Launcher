import { defineConfig, type Plugin } from 'vite'
import vue from '@vitejs/plugin-vue'
import path from 'path'
import fs from 'fs'
import os from 'os'
import https from 'https'
import { exec, execSync, spawn } from 'child_process'
import { handleMinecraftApi } from './server/minecraftService'

function httpsGet(url: string): Promise<string> {
  return new Promise((resolve, reject) => {
    https.get(url, (res) => {
      let data = ''
      res.on('data', (chunk) => (data += chunk))
      res.on('end', () => resolve(data))
    }).on('error', reject)
  })
}

function parseSteamAchievementsXml(xml: string) {
  const achievements: any[] = []
  const achRegex = /<achievement closed="(\d+)">([\s\S]*?)<\/achievement>/g
  let match: RegExpExecArray | null
  while ((match = achRegex.exec(xml)) !== null) {
    const closed = match[1] === '1'
    const inner = match[2]
    const nameMatch = inner.match(/<name><!\[CDATA\[(.*?)\]\]><\/name>/)
    const apiMatch = inner.match(/<apiname><!\[CDATA\[(.*?)\]\]><\/apiname>/)
    const descMatch = inner.match(/<description><!\[CDATA\[(.*?)\]\]><\/description>/)
    const iconClosedMatch = inner.match(/<iconClosed><!\[CDATA\[(.*?)\]\]><\/iconClosed>/)
    const iconOpenMatch = inner.match(/<iconOpen><!\[CDATA\[(.*?)\]\]><\/iconOpen>/)
    const unlockMatch = inner.match(/<unlockTimestamp>(\d+)<\/unlockTimestamp>/)

    const id = apiMatch ? apiMatch[1] : (nameMatch ? nameMatch[1] : '')
    const iconClosed = iconClosedMatch ? iconClosedMatch[1] : ''
    const iconOpen = iconOpenMatch ? iconOpenMatch[1] : iconClosed

    achievements.push({
      id,
      name: nameMatch ? nameMatch[1] : id,
      description: descMatch ? descMatch[1] : '',
      unlocked: closed,
      unlockedAt: unlockMatch && closed ? new Date(parseInt(unlockMatch[1], 10) * 1000).toISOString() : undefined,
      unlockTimestamp: unlockMatch && closed ? parseInt(unlockMatch[1], 10) : undefined,
      icon: closed ? iconClosed : iconOpen,
      iconLocked: iconOpen,
      rarityPercentage: 0
    })
  }
  return achievements
}

function getRunningSteamState(): { runningAppId: string | null; pids: number[] } {
  // 1. Direct real-time scan of /proc/*/cmdline and /proc/*/environ
  try {
    const procEntries = fs.readdirSync('/proc')
    for (const entry of procEntries) {
      if (!/^\d+$/.test(entry)) continue
      const pid = parseInt(entry, 10)
      try {
        const cmd = fs.readFileSync(`/proc/${entry}/cmdline`, 'utf8')
        if (cmd) {
          const matchAppId = cmd.match(/AppId=(\d+)/i) || cmd.match(/rungameid\/(\d+)/i)
          if (matchAppId && matchAppId[1] !== '0') {
            return { runningAppId: matchAppId[1], pids: [pid] }
          }
        }
        const env = fs.readFileSync(`/proc/${entry}/environ`)
        const idx = env.indexOf('SteamAppId=')
        if (idx !== -1) {
          const end = env.indexOf(0, idx)
          const val = env.subarray(idx + 11, end !== -1 ? end : undefined).toString('utf8').trim()
          if (/^\d+$/.test(val) && val !== '0') {
            return { runningAppId: val, pids: [pid] }
          }
        }
      } catch {}
    }
  } catch {}

  // 2. Fallback to gameprocess_log.txt with 5000 lines window
  const logPath = path.join(os.homedir(), '.local/share/Steam/logs/gameprocess_log.txt')
  if (!fs.existsSync(logPath)) return { runningAppId: null, pids: [] }

  try {
    const content = fs.readFileSync(logPath, 'utf8')
    const lines = content.split('\n').slice(-5000)
    const appPids = new Map<string, Set<number>>()

    for (const line of lines) {
      const remMatch = line.match(/Remove (\d+) from running list/)
      if (remMatch) {
        appPids.delete(remMatch[1])
        continue
      }

      const addMatch = line.match(/AppID (\d+) adding PID (\d+)/)
      if (addMatch) {
        const aid = addMatch[1]
        const pid = parseInt(addMatch[2], 10)
        if (!appPids.has(aid)) appPids.set(aid, new Set())
        appPids.get(aid)!.add(pid)
        continue
      }

      const delMatch = line.match(/AppID (\d+) no longer tracking PID (\d+)/)
      if (delMatch) {
        const aid = delMatch[1]
        const pid = parseInt(delMatch[2], 10)
        if (appPids.has(aid)) {
          appPids.get(aid)!.delete(pid)
          if (appPids.get(aid)!.size === 0) appPids.delete(aid)
        }
      }
    }

    for (const [aid, pids] of appPids.entries()) {
      const alivePids: number[] = []
      for (const pid of pids) {
        if (fs.existsSync(`/proc/${pid}`)) {
          alivePids.push(pid)
        }
      }
      if (alivePids.length > 0) {
        return { runningAppId: aid, pids: alivePids }
      }
    }
  } catch (err) {
    console.warn('[Vite::SteamState] Error reading steam process logs:', err)
  }

  return { runningAppId: null, pids: [] }
}

function scanLocalSteamLibrary() {
  const home = os.homedir()
  const steamBase = path.join(home, '.local/share/Steam')
  const detected = fs.existsSync(steamBase)
  const libraries = [steamBase]
  const vdfPath = path.join(steamBase, 'config/libraryfolders.vdf')

  if (fs.existsSync(vdfPath)) {
    try {
      const content = fs.readFileSync(vdfPath, 'utf8')
      const matches = content.matchAll(/"path"\s+"([^"]+)"/g)
      for (const m of matches) {
        if (m[1] && fs.existsSync(m[1]) && !libraries.includes(m[1])) {
          libraries.push(m[1])
        }
      }
    } catch {}
  }

  const ignoredAppids = new Set([
    '228980', '1070560', '1391110', '1628350', '2180100', '1493710',
    '1826330', '3658110', '1161040', '4183110', '2805730', '4628710'
  ])

  // 1. Scan userdata for real LastPlayed and Playtime
  const playtimeMap = new Map<string, { lastPlayed: number; playtime: number }>()
  const userdataDir = path.join(steamBase, 'userdata')
  if (fs.existsSync(userdataDir)) {
    try {
      const uDirs = fs.readdirSync(userdataDir)
      for (const u of uDirs) {
        const localconfig = path.join(userdataDir, u, 'config/localconfig.vdf')
        if (fs.existsSync(localconfig)) {
          const txt = fs.readFileSync(localconfig, 'utf8')
          let pendingAid: string | null = null
          let activeAid: string | null = null
          let activeDepth = 0
          let depth = 0
          const lines = txt.split('\n')
          for (const line of lines) {
            const trimmed = line.trim()
            if (!trimmed) continue

            if (trimmed === '{' && pendingAid) {
              activeAid = pendingAid
              activeDepth = depth + 1
              pendingAid = null
            }

            const parts = trimmed.split('"')
            if (parts.length >= 2 && /^\d+$/.test(parts[1])) {
              if (trimmed === `"${parts[1]}"`) {
                pendingAid = parts[1]
              }
            }

            if (activeAid) {
              if (trimmed.startsWith('"LastPlayed"')) {
                const val = parseInt(trimmed.match(/"LastPlayed"\s+"(\d+)"/)?.[1] || '0', 10)
                const entry = playtimeMap.get(activeAid) || { lastPlayed: 0, playtime: 0 }
                if (val > entry.lastPlayed) entry.lastPlayed = val
                playtimeMap.set(activeAid, entry)
              } else if (trimmed.startsWith('"Playtime"')) {
                const val = parseInt(trimmed.match(/"Playtime"\s+"(\d+)"/)?.[1] || '0', 10)
                const entry = playtimeMap.get(activeAid) || { lastPlayed: 0, playtime: 0 }
                if (val > entry.playtime) entry.playtime = val
                playtimeMap.set(activeAid, entry)
              }
            }

            depth += (trimmed.match(/\{/g) || []).length - (trimmed.match(/\}/g) || []).length
            if (activeAid && depth < activeDepth) {
              activeAid = null
            }
          }
        }
      }
    } catch {}
  }

  // 2. Scan installed manifests in steamapps
  const installedGames: any[] = []
  const installedAppids = new Set<string>()

  for (const lib of libraries) {
    const steamapps = path.join(lib, 'steamapps')
    if (!fs.existsSync(steamapps)) continue

    try {
      const files = fs.readdirSync(steamapps)
      for (const file of files) {
        if (file.startsWith('appmanifest_') && file.endsWith('.acf')) {
          try {
            const raw = fs.readFileSync(path.join(steamapps, file), 'utf8')
            const appid = raw.match(/"appid"\s+"(\d+)"/)?.[1]
            const name = raw.match(/"name"\s+"([^"]+)"/)?.[1]
            const installDir = raw.match(/"installdir"\s+"([^"]+)"/)?.[1]
            const stateFlags = raw.match(/"StateFlags"\s+"(\d+)"/)?.[1]
            const sizeBytes = parseInt(raw.match(/"SizeOnDisk"\s+"(\d+)"/)?.[1] || '0', 10)

            if (appid && name && !ignoredAppids.has(appid)) {
              if (name.startsWith('Steam Linux Runtime') || name.startsWith('Proton') || name.includes('Redistributable')) {
                continue
              }

              const installed = stateFlags === '4'
              const pt = playtimeMap.get(appid) || { lastPlayed: 0, playtime: 0 }
              const sizeGb = Math.round((sizeBytes / 1024 / 1024 / 1024) * 10) / 10
              const slug = name.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '')

              const coverUrl = ['4126040', '4704690', '480', '2315050'].includes(appid)
                ? `/steam-assets/${appid}/library_capsule.jpg`
                : `https://shared.steamstatic.com/store_item_assets/steam/apps/${appid}/library_600x900.jpg`
              const bannerUrl = ['4126040', '4704690', '480', '2315050'].includes(appid)
                ? `/steam-assets/${appid}/library_hero.jpg`
                : `https://shared.steamstatic.com/store_item_assets/steam/apps/${appid}/library_hero.jpg`

              const g = {
                appid,
                name,
                slug,
                installed,
                playtimeMinutes: pt.playtime,
                lastPlayedTimestamp: pt.lastPlayed,
                sizeGb,
                installDir: installDir || null,
                coverUrl,
                bannerUrl
              }

              if (installed) {
                installedAppids.add(appid)
                installedGames.push(g)
              }
            }
          } catch {}
        }
      }
    } catch {}
  }

  // 3. Uninstalled games with playtime in userdata
  const uninstalledGames: any[] = []
  for (const [appid, pt] of playtimeMap.entries()) {
    if (!installedAppids.has(appid) && !ignoredAppids.has(appid)) {
      const coverUrl = ['4126040', '4704690', '480', '2315050'].includes(appid)
        ? `/steam-assets/${appid}/library_capsule.jpg`
        : `https://shared.steamstatic.com/store_item_assets/steam/apps/${appid}/library_600x900.jpg`
      const bannerUrl = ['4126040', '4704690', '480', '2315050'].includes(appid)
        ? `/steam-assets/${appid}/library_hero.jpg`
        : `https://shared.steamstatic.com/store_item_assets/steam/apps/${appid}/library_hero.jpg`

      uninstalledGames.push({
        appid,
        name: `Steam App ${appid}`,
        slug: `steam-app-${appid}`,
        installed: false,
        playtimeMinutes: pt.playtime,
        lastPlayedTimestamp: pt.lastPlayed,
        sizeGb: 0,
        installDir: null,
        coverUrl,
        bannerUrl
      })
    }
  }

  installedGames.sort((a, b) => (b.lastPlayedTimestamp || 0) - (a.lastPlayedTimestamp || 0))
  uninstalledGames.sort((a, b) => (b.lastPlayedTimestamp || 0) - (a.lastPlayedTimestamp || 0))

  const allGames = [...installedGames, ...uninstalledGames]
  allGames.sort((a, b) => (b.lastPlayedTimestamp || 0) - (a.lastPlayedTimestamp || 0))

  return {
    steamId64: '76561198085607978',
    accountName: 'ik0bus',
    personaName: 'K0bus',
    installPath: steamBase,
    detected,
    installedCount: installedGames.length,
    uninstalledCount: uninstalledGames.length,
    totalGamesCount: allGames.length,
    installedGames,
    uninstalledGames,
    allGames
  }
}

function getSteamLibraries() {
  const vdfPath = path.join(os.homedir(), '.local/share/Steam/config/libraryfolders.vdf')
  if (!fs.existsSync(vdfPath)) {
    return []
  }

  try {
    const content = fs.readFileSync(vdfPath, 'utf8')
    const lines = content.split('\n')
    const folders: Array<{
      index: number
      path: string
      label: string
      appsCount: number
      available: boolean
      freeGb: number
      totalGb: number
      usedPercent: number
      isDefault: boolean
    }> = []

    let currentFolder: any = null
    let inApps = false

    for (const rawLine of lines) {
      const line = rawLine.trim()
      if (!line) continue

      const sectionMatch = line.match(/^"(\d+)"$/)
      if (sectionMatch && !inApps) {
        if (currentFolder) folders.push(currentFolder)
        currentFolder = {
          index: parseInt(sectionMatch[1], 10),
          path: '',
          label: '',
          appsCount: 0,
          isDefault: sectionMatch[1] === '0'
        }
        continue
      }

      if (line === '{' || line === '}') {
        if (line === '}' && inApps) inApps = false
        continue
      }

      if (line === '"apps"') {
        inApps = true
        continue
      }

      if (inApps && currentFolder) {
        const appMatch = line.match(/^"(\d+)"\s+"(\d+)"$/)
        if (appMatch) currentFolder.appsCount++
        continue
      }

      const kvMatch = line.match(/^"([^"]+)"\s+"([^"]*)"$/)
      if (kvMatch && currentFolder) {
        const key = kvMatch[1]
        const val = kvMatch[2]
        if (key === 'path') currentFolder.path = val
        if (key === 'label') currentFolder.label = val
      }
    }
    if (currentFolder) folders.push(currentFolder)

    for (const f of folders) {
      if (f.path && fs.existsSync(f.path)) {
        try {
          const stat = fs.statfsSync(f.path)
          f.available = true
          f.freeGb = parseFloat(((stat.bavail * stat.bsize) / (1024 ** 3)).toFixed(1))
          f.totalGb = parseFloat(((stat.blocks * stat.bsize) / (1024 ** 3)).toFixed(1))
          f.usedPercent = Math.round(((f.totalGb - f.freeGb) / f.totalGb) * 100)
        } catch {
          f.available = false
          f.freeGb = 0
          f.totalGb = 0
          f.usedPercent = 0
        }
      } else {
        f.available = false
        f.freeGb = 0
        f.totalGb = 0
        f.usedPercent = 0
      }

      if (!f.label) {
        if (f.path.includes('.local/share/Steam')) {
          f.label = 'Disque Principal (Steam Défaut)'
        } else if (f.path.includes('/mnt/ssd')) {
          f.label = 'SSD Secondaire'
        } else {
          f.label = path.basename(f.path) || `Bibliothèque ${f.index}`
        }
      }
    }

    return folders
  } catch (e) {
    console.warn('[Vite::SteamLibraries] Erreur de lecture libraryfolders:', e)
    return []
  }
}

// Ignore Steam internal runtimes & shared redistributable packages that aren't games
const IGNORED_STEAM_APPIDS = new Set(['228980', '1070560', '1391110', '1628350', '2180100', '1493710'])

// 1. Resilient directory size calculation (doesn't fail if files are written/moved during download)
const lastKnownBytesDownloaded = new Map<string, number>()

function getDirectorySizeBytes(dirPath: string): number {
  if (!fs.existsSync(dirPath)) return 0
  try {
    const out = execSync(`du -sb "${dirPath}" 2>/dev/null || true`, {
      encoding: 'utf8',
      shell: '/bin/bash',
      timeout: 2000
    })
    const match = out.trim().match(/^(\d+)/)
    if (match) {
      const val = parseInt(match[1], 10)
      if (val > 0) return val
    }
  } catch (err: any) {
    if (err?.stdout) {
      const match = String(err.stdout).trim().match(/^(\d+)/)
      if (match) {
        const val = parseInt(match[1], 10)
        if (val > 0) return val
      }
    }
  }
  return 0
}

// 2. Real-time disk write / staging speed from Steam process I/O (/proc/<steam_pid>/io)
let steamPidCache: { pid: number | null; checkedAt: number } = { pid: null, checkedAt: 0 }
let steamIoTracker: { pid: number; writeBytes: number; time: number; speedMbps: number } | null = null
const appSpeedTracker = new Map<string, { bytes: number; time: number; speedMbps: number }>()

function getSteamProcessPid(): number | null {
  const now = Date.now()
  if (steamPidCache.pid !== null && now - steamPidCache.checkedAt < 4000) {
    if (fs.existsSync(`/proc/${steamPidCache.pid}`)) {
      return steamPidCache.pid
    }
  }
  try {
    const out = execSync('pgrep -x steam 2>/dev/null || true', { encoding: 'utf8', timeout: 500 })
    const pid = parseInt(out.trim().split('\n')[0], 10)
    const validPid = !isNaN(pid) && pid > 0 ? pid : null
    steamPidCache = { pid: validPid, checkedAt: now }
    return validPid
  } catch {
    steamPidCache = { pid: null, checkedAt: now }
    return null
  }
}

function getSteamLiveIoRateMbps(): number {
  const pid = getSteamProcessPid()
  if (!pid) return 0
  try {
    const ioContent = fs.readFileSync(`/proc/${pid}/io`, 'utf8')
    const match = ioContent.match(/write_bytes:\s+(\d+)/) || ioContent.match(/wchar:\s+(\d+)/)
    if (!match) return 0
    const writeBytes = parseInt(match[1], 10)
    const now = Date.now()

    if (!steamIoTracker || steamIoTracker.pid !== pid) {
      steamIoTracker = { pid, writeBytes, time: now, speedMbps: 0 }
      return 0
    }

    const dtSec = (now - steamIoTracker.time) / 1000
    if (dtSec >= 0.5) {
      const deltaBytes = Math.max(0, writeBytes - steamIoTracker.writeBytes)
      const rawMbps = ((deltaBytes * 8) / (1024 * 1024)) / dtSec
      const smoothMbps = steamIoTracker.speedMbps > 0
        ? (steamIoTracker.speedMbps * 0.3) + (rawMbps * 0.7)
        : rawMbps
      const finalMbps = parseFloat(smoothMbps.toFixed(1))
      steamIoTracker = { pid, writeBytes, time: now, speedMbps: finalMbps }
      return finalMbps
    }

    return steamIoTracker.speedMbps
  } catch {
    return 0
  }
}

let cachedLibrariesData: any = null
let cachedLibrariesTimestamp = 0

function getSteamLibrariesCached() {
  const now = Date.now()
  if (cachedLibrariesData && now - cachedLibrariesTimestamp < 10000) {
    return cachedLibrariesData
  }
  cachedLibrariesData = getSteamLibraries()
  cachedLibrariesTimestamp = now
  return cachedLibrariesData
}

let cachedCompletedDownloads: any[] = []
let cachedInstalledAppids: string[] = []
let cachedCompletedTimestamp = 0

function getSteamDownloads() {
  const libraries = getSteamLibrariesCached()
  const active: any[] = []
  const nowSec = Math.floor(Date.now() / 1000)
  const nowMs = Date.now()

  // Extract recent download rate from content_log.txt
  let latestRate = 0
  const contentLogPath = path.join(os.homedir(), '.local/share/Steam/logs/content_log.txt')
  if (fs.existsSync(contentLogPath)) {
    try {
      const logText = fs.readFileSync(contentLogPath, 'utf8')
      const matches = [...logText.matchAll(/(?:Current download rate:\s+([\d\.]+)\s+Mbps|now\s+([\d\.]+)\))/g)]
      for (let i = matches.length - 1; i >= Math.max(0, matches.length - 4); i--) {
        const val = parseFloat(matches[i][1] || matches[i][2] || '0')
        if (val > 0) {
          latestRate = val
          break
        }
      }
    } catch {}
  }

  // Real-time disk write speed from Steam kernel I/O
  const liveIoMbps = getSteamLiveIoRateMbps()

  const seenAppids = new Set<string>()

  for (const lib of libraries) {
    if (!lib.available || !lib.path) continue
    const steamappsDir = path.join(lib.path, 'steamapps')
    if (!fs.existsSync(steamappsDir)) continue

    // 1. Check active downloading folders
    const downloadingDir = path.join(steamappsDir, 'downloading')
    if (fs.existsSync(downloadingDir)) {
      try {
        const entries = fs.readdirSync(downloadingDir)
        if (entries.length === 0) continue
        for (const appid of entries) {
          if (!/^\d+$/.test(appid)) continue
          if (IGNORED_STEAM_APPIDS.has(appid)) continue

          const appDir = path.join(downloadingDir, appid)
          const manifestPath = path.join(steamappsDir, `appmanifest_${appid}.acf`)
          let name = `App ${appid}`
          let bytesToDownload = 0
          let bytesToStage = 0

          if (fs.existsSync(manifestPath)) {
            try {
              const manifest = fs.readFileSync(manifestPath, 'utf8')
              name = manifest.match(/"name"\s+"([^"]+)"/)?.[1] || name
              bytesToDownload = parseInt(manifest.match(/"BytesToDownload"\s+"(\d+)"/)?.[1] || '0', 10)
              bytesToStage = parseInt(manifest.match(/"BytesToStage"\s+"(\d+)"/)?.[1] || '0', 10)
            } catch {}
          }

          if (name.includes('Steamworks Common Redistributables') || name.startsWith('Steam Linux Runtime')) {
            continue
          }

          seenAppids.add(appid)

          // Measure folder size with resilience and memory retention
          let bytesDownloaded = getDirectorySizeBytes(appDir)
          if (bytesDownloaded === 0 && lastKnownBytesDownloaded.has(appid)) {
            bytesDownloaded = lastKnownBytesDownloaded.get(appid)!
          } else if (bytesDownloaded > 0) {
            lastKnownBytesDownloaded.set(appid, bytesDownloaded)
          }

          // Per-app byte growth delta speed
          let appRateMbps = 0
          const prevApp = appSpeedTracker.get(appid)
          const nowMs = Date.now()
          if (prevApp && prevApp.bytes > 0 && bytesDownloaded >= prevApp.bytes) {
            const dtSec = (nowMs - prevApp.time) / 1000
            if (dtSec >= 0.5) {
              const deltaBytes = bytesDownloaded - prevApp.bytes
              const calcMbps = ((deltaBytes * 8) / (1024 * 1024)) / dtSec
              appRateMbps = parseFloat(calcMbps.toFixed(1))
              appSpeedTracker.set(appid, { bytes: bytesDownloaded, time: nowMs, speedMbps: appRateMbps })
            } else {
              appRateMbps = prevApp.speedMbps
            }
          } else {
            appSpeedTracker.set(appid, { bytes: bytesDownloaded, time: nowMs, speedMbps: appRateMbps })
          }

          // Effective rate: choose highest reliable non-zero signal
          const candidates = [liveIoMbps, appRateMbps, latestRate].filter((r) => r > 1)
          const appEffectiveRate = candidates.length > 0 ? Math.max(...candidates) : (latestRate > 0 ? latestRate : 0)

          // Target total is the uncompressed staged size on disk (BytesToStage)
          const targetTotal = bytesToStage > 0 ? bytesToStage : (bytesToDownload > 0 ? bytesToDownload : 0)

          let progress = 0
          let state: 'downloading' | 'staging' = 'downloading'

          if (targetTotal > 0) {
            progress = Math.min(100, Math.round((bytesDownloaded / targetTotal) * 100))
            if (bytesToDownload > 0 && bytesDownloaded >= bytesToDownload) {
              state = progress >= 99 ? 'staging' : 'downloading'
            }
          }

          active.push({
            appid,
            gameId: `steam-${appid}`,
            name,
            state,
            bytesDownloaded,
            bytesTotal: targetTotal,
            progressPercent: progress,
            downloadRateMbps: appEffectiveRate,
            coverUrl: ['4126040', '4704690', '480', '2315050'].includes(appid)
              ? `/steam-assets/${appid}/library_capsule.jpg`
              : `https://shared.steamstatic.com/store_item_assets/steam/apps/${appid}/library_600x900.jpg`
          })
        }
      } catch {}
    }

    // 2. Check recently updated appmanifests (installed in the last 7 days) with 15s cache
    if (nowMs - cachedCompletedTimestamp > 15000) {
      cachedCompletedDownloads = []
      cachedInstalledAppids = []

      for (const l of libraries) {
        if (!l.available || !l.path) continue
        const sDir = path.join(l.path, 'steamapps')
        if (!fs.existsSync(sDir)) continue

        try {
          const manifests = fs.readdirSync(sDir).filter((f) => f.startsWith('appmanifest_') && f.endsWith('.acf'))
          for (const mFile of manifests) {
            const mPath = path.join(sDir, mFile)
            try {
              const content = fs.readFileSync(mPath, 'utf8')
              const appid = mFile.match(/appmanifest_(\d+)\.acf/)?.[1]
              if (!appid || IGNORED_STEAM_APPIDS.has(appid)) continue

              const stateFlags = content.match(/"StateFlags"\s+"(\d+)"/)?.[1]
              if (stateFlags === '4') {
                cachedInstalledAppids.push(appid)
              }

              const lastUpdated = parseInt(content.match(/"LastUpdated"\s+"(\d+)"/)?.[1] || '0', 10)
              const name = content.match(/"name"\s+"([^"]+)"/)?.[1] || `App ${appid}`
              if (name.includes('Steamworks Common Redistributables') || name.startsWith('Steam Linux Runtime')) {
                continue
              }

              const bytesTotal = parseInt(content.match(/"SizeOnDisk"\s+"(\d+)"/)?.[1] || '0', 10)
              if (stateFlags === '4' && nowSec - lastUpdated < 604800 && lastUpdated > 0) {
                cachedCompletedDownloads.push({
                  appid,
                  gameId: `steam-${appid}`,
                  name,
                  state: 'completed',
                  bytesDownloaded: bytesTotal,
                  bytesTotal,
                  progressPercent: 100,
                  completedAt: lastUpdated,
                  coverUrl: ['4126040', '4704690', '480', '2315050'].includes(appid)
                    ? `/steam-assets/${appid}/library_capsule.jpg`
                    : `https://shared.steamstatic.com/store_item_assets/steam/apps/${appid}/library_600x900.jpg`
                })
              }
            } catch {}
          }
        } catch {}
      }
      cachedCompletedTimestamp = nowMs
    }

    // Clean up any stale entries in lastKnownBytesDownloaded
    for (const cachedAppid of lastKnownBytesDownloaded.keys()) {
      if (!seenAppids.has(cachedAppid)) {
        lastKnownBytesDownloaded.delete(cachedAppid)
      }
    }

    const totalActiveRate = active.reduce((sum, a) => sum + (a.downloadRateMbps || 0), 0)

    const completed = [...cachedCompletedDownloads].filter((c) => !seenAppids.has(c.appid))
    completed.sort((a, b) => b.completedAt - a.completedAt)

    return {
      active,
      completed: completed.slice(0, 10),
      totalRateMbps: active.length > 0 ? (totalActiveRate > 0 ? totalActiveRate : latestRate) : 0,
      installedAppids: cachedInstalledAppids
    }
  }
}

function steamProcessPlugin(): Plugin {
  return {
    name: 'loom-steam-real-process',
    configureServer(server) {
      server.middlewares.use((req, res, next) => {
        if (!req.url?.startsWith('/api/steam')) {
          return next()
        }

        res.setHeader('Content-Type', 'application/json')

        // 1. GET /api/steam/status
        if (req.method === 'GET' && req.url === '/api/steam/status') {
          const state = getRunningSteamState()
          res.end(JSON.stringify({ success: true, ...state }))
          return
        }

        // 1a. GET /api/steam/scan
        if (req.method === 'GET' && req.url === '/api/steam/scan') {
          const accountData = scanLocalSteamLibrary()
          res.end(JSON.stringify(accountData))
          return
        }

        // 1b. GET /api/steam/libraries
        if (req.method === 'GET' && req.url === '/api/steam/libraries') {
          const libraries = getSteamLibraries()
          res.end(JSON.stringify({ success: true, libraries }))
          return
        }

        // 1c. GET /api/steam/downloads
        if (req.method === 'GET' && req.url === '/api/steam/downloads') {
          const downloads = getSteamDownloads()
          res.end(JSON.stringify({ success: true, ...downloads }))
          return
        }

        // 2. GET /api/steam/achievements/summaries
        if (req.method === 'GET' && req.url?.startsWith('/api/steam/achievements/summaries')) {
          const summariesMap: Record<string, any> = {}
          const userdataPath = path.join(os.homedir(), '.local/share/Steam/userdata')

          if (fs.existsSync(userdataPath)) {
            try {
              const uDirs = fs.readdirSync(userdataPath)
              for (const uDir of uDirs) {
                const cacheDir = path.join(userdataPath, uDir, 'config/librarycache')
                if (!fs.existsSync(cacheDir)) continue

                // 1. Check achievement_progress.json
                const progFile = path.join(cacheDir, 'achievement_progress.json')
                if (fs.existsSync(progFile)) {
                  try {
                    const progData = JSON.parse(fs.readFileSync(progFile, 'utf8'))
                    if (Array.isArray(progData.mapCache)) {
                      for (const [aid, info] of progData.mapCache) {
                        summariesMap[String(aid)] = info
                      }
                    }
                  } catch {}
                }

                // 2. Check individual <appid>.json files in librarycache
                try {
                  const cacheFiles = fs.readdirSync(cacheDir)
                  for (const cFile of cacheFiles) {
                    if (!cFile.endsWith('.json') || cFile === 'achievement_progress.json') continue
                    const appid = cFile.replace('.json', '')
                    if (!/^\d+$/.test(appid) || summariesMap[appid]) continue

                    try {
                      const content = JSON.parse(fs.readFileSync(path.join(cacheDir, cFile), 'utf8'))
                      if (Array.isArray(content)) {
                        for (const item of content) {
                          if (Array.isArray(item) && item[0] === 'achievements' && item[1]?.data) {
                            const data = item[1].data
                            const highlights = Array.isArray(data.vecHighlight) ? data.vecHighlight : []
                            const unachieved = Array.isArray(data.vecUnachieved) ? data.vecUnachieved : []
                            const achieved = Array.isArray(data.vecAchieved) ? data.vecAchieved : []
                            const unlocked = achieved.length + highlights.filter((h: any) => h.bAchieved).length
                            const total = unlocked + unachieved.length
                            if (total > 0) {
                              summariesMap[appid] = {
                                appid: parseInt(appid, 10),
                                unlocked,
                                total,
                                percentage: Math.round((unlocked / total) * 100),
                                all_unlocked: unlocked === total
                              }
                            }
                          }
                        }
                      }
                    } catch {}
                  }
                } catch {}
              }
            } catch {}
          }

          res.end(JSON.stringify({ success: true, summaries: summariesMap }))
          return
        }

        // 3. GET /api/steam/achievements?appid=...
        if (req.method === 'GET' && req.url?.startsWith('/api/steam/achievements')) {
          const urlObj = new URL(req.url, 'http://localhost')
          const appid = (urlObj.searchParams.get('appid') || '').replace(/^steam-/, '')
          if (!appid) {
            res.statusCode = 400
            res.end(JSON.stringify({ success: false, error: 'AppID manquant' }))
            return
          }

          // 1. Try local librarycache first (instant, works offline, accurate)
          const userdataPath = path.join(os.homedir(), '.local/share/Steam/userdata')
          let localAchievements: any[] = []

          if (fs.existsSync(userdataPath)) {
            try {
              for (const uDir of fs.readdirSync(userdataPath)) {
                const jsonPath = path.join(userdataPath, uDir, 'config/librarycache', `${appid}.json`)
                if (fs.existsSync(jsonPath)) {
                  try {
                    const content = JSON.parse(fs.readFileSync(jsonPath, 'utf8'))
                    if (Array.isArray(content)) {
                      for (const item of content) {
                        if (Array.isArray(item) && item[0] === 'achievements' && item[1]?.data) {
                          const data = item[1].data
                          const all = [
                            ...(Array.isArray(data.vecHighlight) ? data.vecHighlight : []),
                            ...(Array.isArray(data.vecAchieved) ? data.vecAchieved : []),
                            ...(Array.isArray(data.vecUnachieved) ? data.vecUnachieved : [])
                          ]
                          const seen = new Set<string>()
                          for (const a of all) {
                            if (!a.strID || seen.has(a.strID)) continue
                            seen.add(a.strID)
                            localAchievements.push({
                              id: a.strID,
                              name: a.strName || a.strID,
                              description: a.strDescription || '',
                              unlocked: Boolean(a.bAchieved),
                              unlockedAt: a.rtUnlocked ? new Date(a.rtUnlocked * 1000).toISOString() : undefined,
                              unlockTimestamp: a.rtUnlocked || undefined,
                              icon: a.strImage || '',
                              iconLocked: a.strImage || '',
                              rarityPercentage: Math.round(a.flAchieved || 0)
                            })
                          }
                        }
                      }
                    }
                  } catch {}
                  if (localAchievements.length > 0) break
                }
              }
            } catch {}
          }

          if (localAchievements.length > 0) {
            res.end(JSON.stringify({ success: true, appid, achievements: localAchievements }))
            return
          }

          ;(async () => {
            try {
              const xmlUrl = `https://steamcommunity.com/profiles/76561198085607978/stats/${appid}/?xml=1`
              const xml = await httpsGet(xmlUrl)
              const achievements = parseSteamAchievementsXml(xml)

              // Optional: Enrich with global rarity percentages
              try {
                const rarityUrl = `https://api.steampowered.com/ISteamUserStats/GetGlobalAchievementPercentagesForApp/v0002/?gameid=${appid}`
                const rarityRaw = await httpsGet(rarityUrl)
                const rarityJson = JSON.parse(rarityRaw)
                if (rarityJson?.achievementpercentages?.achievements) {
                  const rarities = new Map<string, number>()
                  for (const a of rarityJson.achievementpercentages.achievements) {
                    rarities.set(String(a.name).toLowerCase(), parseFloat(a.percent))
                  }
                  for (const ach of achievements) {
                    const r = rarities.get(ach.id.toLowerCase()) ?? rarities.get(ach.name.toLowerCase())
                    if (r !== undefined) {
                      ach.rarityPercentage = r
                    }
                  }
                }
              } catch (e) {
                // Ignore rarity network failure
              }

              res.end(JSON.stringify({ success: true, appid, achievements }))
            } catch (err: any) {
              res.statusCode = 500
              res.end(JSON.stringify({ success: false, error: err.message }))
            }
          })()
          return
        }

        // Parse JSON body for POST requests
        if (req.method === 'POST') {
          let body = ''
          req.on('data', (chunk) => {
            body += chunk
          })
          req.on('end', () => {
            try {
              const data = body ? JSON.parse(body) : {}
              const appid = String(data.appid || data.gameId || '').replace(/^steam-/, '')

              if (req.url === '/api/steam/launch') {
                if (!appid) {
                  res.statusCode = 400
                  res.end(JSON.stringify({ success: false, error: 'AppID manquant' }))
                  return
                }
                console.info(`[Loom::Steam] Lancement réel du jeu Steam AppID ${appid}...`)
                exec(`xdg-open steam://rungameid/${appid}`, (err) => {
                  if (err) {
                    console.warn(`[Loom::Steam] xdg-open failed, trying steam command fallback:`, err)
                    exec(`steam steam://rungameid/${appid}`)
                  }
                })
                res.end(JSON.stringify({ success: true, message: `Lancement transmis à Steam pour AppID ${appid}` }))
                return
              }

              if (req.url === '/api/steam/install') {
                if (!appid) {
                  res.statusCode = 400
                  res.end(JSON.stringify({ success: false, error: 'AppID manquant' }))
                  return
                }
                console.info(`[Loom::Steam] Lancement de l'assistant d'installation Steam pour AppID ${appid}...`)
                exec(`xdg-open steam://install/${appid}`, (err) => {
                  if (err) {
                    exec(`steam "steam://install/${appid}"`)
                  }
                })

                res.end(
                  JSON.stringify({
                    success: true,
                    message: `Assistant d'installation Steam ouvert pour AppID ${appid}`,
                    appid
                  })
                )
                return
              }

              if (req.url === '/api/steam/uninstall') {
                if (!appid) {
                  res.statusCode = 400
                  res.end(JSON.stringify({ success: false, error: 'AppID manquant' }))
                  return
                }
                console.info(`[Loom::Steam] Lancement de l'assistant de désinstallation Steam pour AppID ${appid}...`)
                exec(`xdg-open steam://uninstall/${appid}`, (err) => {
                  if (err) {
                    exec(`steam "steam://uninstall/${appid}"`)
                  }
                })

                res.end(
                  JSON.stringify({
                    success: true,
                    message: `Assistant de désinstallation Steam ouvert pour AppID ${appid}`,
                    appid
                  })
                )
                return
              }

              if (req.url === '/api/steam/stop') {
                const state = getRunningSteamState()
                if (state.pids.length > 0) {
                  console.info(`[Loom::Steam] Arrêt des processus Steam PID(s):`, state.pids)
                  for (const pid of state.pids) {
                    try {
                      process.kill(pid, 'SIGTERM')
                    } catch {}
                  }
                }
                res.end(JSON.stringify({ success: true, message: 'Demande d\'arrêt envoyée' }))
                return
              }

              res.statusCode = 404
              res.end(JSON.stringify({ success: false, error: 'Endpoint inconnu' }))
            } catch (err: any) {
              res.statusCode = 500
              res.end(JSON.stringify({ success: false, error: err.message }))
            }
          })
          return
        }

        next()
      })
    }
  }
}

// https://vitejs.dev/config/

function microsoftAuthPlugin(): Plugin {
  const CLIENT_ID = process.env.MICROSOFT_CLIENT_ID || 'c36a9fb6-4f2a-41ff-90bd-ae7cc92031eb'
  const AUTH_FILE = path.join(os.homedir(), '.local/share', 'loom', 'minecraft_account.json')

  function loadAccountFile() {
    if (fs.existsSync(AUTH_FILE)) {
      try {
        const acc = JSON.parse(fs.readFileSync(AUTH_FILE, 'utf8'))
        let updated = false
        if (acc && acc.uuid) {
          if (!acc.uuid.includes('-') && acc.uuid.length === 32) {
            acc.uuid = acc.uuid.replace(/^(\w{8})(\w{4})(\w{4})(\w{4})(\w{12})$/, '$1-$2-$3-$4-$5')
            updated = true
          }
          if (!acc.skin_url || acc.skin_url.includes('textures.minecraft.net') || !acc.skin_url.includes('/avatar/')) {
            if (!acc.raw_skin_url && acc.skin_url) {
              acc.raw_skin_url = acc.skin_url
            }
            acc.skin_url = `https://mc-heads.net/avatar/${acc.uuid}/64`
            updated = true
          }
          if ((!acc.xuid || !acc.client_id) && acc.access_token) {
            try {
              const payload = JSON.parse(Buffer.from(acc.access_token.split('.')[1], 'base64').toString('utf8'))
              acc.xuid = acc.xuid || payload.xuid || payload.xid || ''
              acc.client_id = acc.client_id || payload.aid || CLIENT_ID
              updated = true
            } catch {}
          }
          if (updated) {
            saveAccountFile(acc)
          }
        }
        return acc
      } catch {}
    }
    return null
  }

  function saveAccountFile(acc: any) {
    fs.mkdirSync(path.dirname(AUTH_FILE), { recursive: true })
    fs.writeFileSync(AUTH_FILE, JSON.stringify(acc, null, 2), 'utf8')
  }

  function removeAccountFile() {
    if (fs.existsSync(AUTH_FILE)) {
      try {
        fs.unlinkSync(AUTH_FILE)
      } catch {}
    }
  }

  function httpsPostForm(urlStr: string, formObj: Record<string, string>): Promise<any> {
    return new Promise((resolve, reject) => {
      const url = new URL(urlStr)
      const postData = new URLSearchParams(formObj).toString()
      const req = https.request(
        {
          hostname: url.hostname,
          port: 443,
          path: url.pathname + url.search,
          method: 'POST',
          family: 4,
          headers: {
            'Content-Type': 'application/x-www-form-urlencoded',
            'Content-Length': Buffer.byteLength(postData),
            'User-Agent': 'LoomLauncher/0.1.0'
          }
        },
        (res) => {
          let data = ''
          res.on('data', (c) => (data += c))
          res.on('end', () => {
            try {
              resolve(JSON.parse(data))
            } catch {
              resolve(data)
            }
          })
        }
      )
      req.on('error', reject)
      req.write(postData)
      req.end()
    })
  }

  function httpsPostJson(urlStr: string, bodyObj: any, headers: Record<string, string> = {}): Promise<any> {
    return new Promise((resolve, reject) => {
      const url = new URL(urlStr)
      const postData = JSON.stringify(bodyObj)
      const req = https.request(
        {
          hostname: url.hostname,
          port: 443,
          path: url.pathname + url.search,
          method: 'POST',
          family: 4,
          headers: {
            'Content-Type': 'application/json',
            Accept: 'application/json',
            'User-Agent': 'LoomLauncher/0.1.0',
            'Content-Length': Buffer.byteLength(postData),
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
      req.write(postData)
      req.end()
    })
  }

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

  async function exchangeMsToken(msAccessToken: string, msRefreshToken: string) {
    const xblRes = await httpsPostJson('https://user.auth.xboxlive.com/user/authenticate', {
      Properties: {
        AuthMethod: 'RPS',
        SiteName: 'user.auth.xboxlive.com',
        RpsTicket: `d=${msAccessToken}`
      },
      RelyingParty: 'http://auth.xboxlive.com',
      TokenType: 'JWT'
    })

    if (!xblRes.json?.Token) {
      throw new Error('Échec authentification XBL: ' + JSON.stringify(xblRes.json || xblRes.raw))
    }
    const xblToken = xblRes.json.Token
    const uhs = xblRes.json.DisplayClaims?.xui?.[0]?.uhs
    const xuid = xblRes.json.DisplayClaims?.xui?.[0]?.xid || ''
    if (!uhs) {
      throw new Error('Hash utilisateur XBL (uhs) introuvable.')
    }

    const xstsRes = await httpsPostJson('https://xsts.auth.xboxlive.com/xsts/authorize', {
      Properties: {
        SandboxId: 'RETAIL',
        UserTokens: [xblToken]
      },
      RelyingParty: 'rp://api.minecraftservices.com/',
      TokenType: 'JWT'
    })

    if (xstsRes.json?.XErr) {
      const code = xstsRes.json.XErr
      if (code === 2148916233) {
        throw new Error('Ce compte Microsoft ne possède pas de profil Xbox Live. Créez-en un sur xbox.com.')
      }
      if (code === 2148916238) {
        throw new Error("Ce compte est un compte enfant nécessitant l'accord d'un tuteur légal.")
      }
      throw new Error(`Erreur XSTS code ${code}`)
    }

    const xstsToken = xstsRes.json?.Token
    if (!xstsToken) {
      throw new Error('Jeton XSTS introuvable.')
    }

    const mojangRes = await httpsPostJson('https://api.minecraftservices.com/authentication/login_with_xbox', {
      identityToken: `XBL3.0 x=${uhs};${xstsToken}`
    })

    const mojangToken = mojangRes.json?.access_token
    const expiresIn = mojangRes.json?.expires_in || 86400
    if (!mojangToken) {
      throw new Error('Jeton Mojang manquant: ' + JSON.stringify(mojangRes.json || mojangRes.raw))
    }

    const profileRes = await httpsGetJson('https://api.minecraftservices.com/minecraft/profile', {
      Authorization: `Bearer ${mojangToken}`
    })

    if (profileRes.status === 404) {
      throw new Error('Ce compte Microsoft ne possède pas de licence Minecraft: Java Edition active.')
    }

    const rawId = profileRes.json?.id || '00000000-0000-0000-0000-000000000000'
    const uuid = rawId.includes('-')
      ? rawId
      : rawId.replace(/^(\w{8})(\w{4})(\w{4})(\w{4})(\w{12})$/, '$1-$2-$3-$4-$5')
    const name = profileRes.json?.name || 'MinecraftPlayer'
    const activeSkin =
      (profileRes.json?.skins || []).find((s: any) => s.state === 'ACTIVE') || profileRes.json?.skins?.[0]
    const rawSkinUrl = activeSkin?.url || ''
    const headAvatarUrl = `https://mc-heads.net/avatar/${uuid}/64`

    // Fetch official signed texture property from session server for singleplayer/offline skin rendering
    let userProperties: any[] = []
    try {
      const strippedUuid = uuid.replace(/-/g, '')
      const sessionProfile = await httpsGetJson(
        `https://sessionserver.mojang.com/session/minecraft/profile/${strippedUuid}?unsigned=false`
      )
      if (sessionProfile?.json?.properties && Array.isArray(sessionProfile.json.properties)) {
        userProperties = sessionProfile.json.properties
      }
    } catch {}

    const account = {
      uuid,
      name,
      xuid,
      client_id: CLIENT_ID,
      skin_url: headAvatarUrl,
      raw_skin_url: rawSkinUrl,
      access_token: mojangToken,
      refresh_token: msRefreshToken,
      expires_at: Math.floor(Date.now() / 1000) + expiresIn,
      user_properties: userProperties
    }

    saveAccountFile(account)
    return account
  }

  async function refreshAccount(refreshTok: string) {
    const tokenRes = await httpsPostForm('https://login.microsoftonline.com/consumers/oauth2/v2.0/token', {
      client_id: CLIENT_ID,
      grant_type: 'refresh_token',
      refresh_token: refreshTok,
      scope: 'XboxLive.SignIn XboxLive.offline_access'
    })

    if (!tokenRes.access_token) {
      throw new Error('Erreur lors du rafraîchissement Microsoft: ' + JSON.stringify(tokenRes))
    }

    return exchangeMsToken(tokenRes.access_token, tokenRes.refresh_token || refreshTok)
  }

  return {
    name: 'loom-microsoft-auth',
    configureServer(server) {
      server.middlewares.use(async (req, res, next) => {
        if (!req.url?.startsWith('/api/auth/microsoft')) {
          return next()
        }

        res.setHeader('Content-Type', 'application/json')

        // 1. GET /api/auth/microsoft/account
        if (req.method === 'GET' && req.url === '/api/auth/microsoft/account') {
          const account = loadAccountFile()
          if (!account) {
            res.end(JSON.stringify({ account: null }))
            return
          }

          const now = Math.floor(Date.now() / 1000)
          if (account.expires_at && account.expires_at <= now + 300 && account.refresh_token) {
            try {
              console.info('[Loom::Auth] Jeton proche expiration. Rafraîchissement automatique...')
              const refreshed = await refreshAccount(account.refresh_token)
              res.end(JSON.stringify({ account: refreshed }))
              return
            } catch (err: any) {
              console.warn('[Loom::Auth] Échec rafraîchissement jeton, maintien du compte hors-ligne:', err?.message)
              res.end(JSON.stringify({ account }))
              return
            }
          }

          res.end(JSON.stringify({ account }))
          return
        }

        // 2. POST /api/auth/microsoft/devicecode
        if (req.method === 'POST' && req.url === '/api/auth/microsoft/devicecode') {
          try {
            const data = await httpsPostForm('https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode', {
              client_id: CLIENT_ID,
              scope: 'XboxLive.SignIn XboxLive.offline_access'
            })
            if (!data || !data.user_code || !data.device_code) {
              const errDesc = data?.error_description || data?.error || 'Réponse inattendue de Microsoft Identity'
              console.error('[Loom::Auth] Devicecode rejection from MS:', data)
              res.statusCode = 400
              res.end(JSON.stringify({ error: errDesc }))
              return
            }
            res.end(
              JSON.stringify({
                user_code: data.user_code,
                device_code: data.device_code,
                verification_uri: data.verification_uri || 'https://microsoft.com/link',
                expires_in: data.expires_in || 900,
                interval: data.interval || 5,
                message: data.message
              })
            )
          } catch (err: any) {
            console.error('[Loom::Auth] Devicecode error:', err.message)
            res.statusCode = 500
            res.end(JSON.stringify({ error: err.message }))
          }
          return
        }

        // 3. POST /api/auth/microsoft/poll
        if (req.method === 'POST' && req.url === '/api/auth/microsoft/poll') {
          let body = ''
          req.on('data', (c) => (body += c))
          req.on('end', async () => {
            try {
              const parsed = body ? JSON.parse(body) : {}
              const deviceCode = parsed.device_code
              if (!deviceCode) {
                res.statusCode = 400
                res.end(JSON.stringify({ error: 'device_code manquant' }))
                return
              }

              const tokenRes = await httpsPostForm('https://login.microsoftonline.com/consumers/oauth2/v2.0/token', {
                client_id: CLIENT_ID,
                grant_type: 'urn:ietf:params:oauth:grant-type:device_code',
                device_code: deviceCode
              })

              if (tokenRes.error === 'authorization_pending' || tokenRes.error === 'slow_down') {
                res.end(JSON.stringify({ pending: true }))
                return
              }

              if (tokenRes.error) {
                res.end(
                  JSON.stringify({
                    pending: false,
                    error: tokenRes.error_description || tokenRes.error
                  })
                )
                return
              }

              if (tokenRes.access_token) {
                const account = await exchangeMsToken(tokenRes.access_token, tokenRes.refresh_token || '')
                res.end(JSON.stringify({ pending: false, account }))
                return
              }

              res.end(JSON.stringify({ pending: true }))
            } catch (err: any) {
              res.statusCode = 500
              res.end(JSON.stringify({ pending: false, error: err.message }))
            }
          })
          return
        }

        // 4. POST /api/auth/microsoft/logout
        if (req.method === 'POST' && req.url === '/api/auth/microsoft/logout') {
          removeAccountFile()
          res.end(JSON.stringify({ success: true }))
          return
        }

        next()
      })
    }
  }
}

function minecraftServerPlugin(): Plugin {
  return {
    name: 'loom-minecraft-server',
    configureServer(server) {
      server.middlewares.use(async (req, res, next) => {
        const handled = await handleMinecraftApi(req, res)
        if (!handled) {
          next()
        }
      })
    }
  }
}

const BNET_CONFIG_FILE = path.join(os.homedir(), '.local/share', 'loom', 'battlenet_config.json')

interface BattleNetEnvironment {
  prefix: string
  exe: string | null
  runner: string
  isInstalled: boolean
}

function resolveBattleNetEnvironment(): BattleNetEnvironment {
  if (fs.existsSync(BNET_CONFIG_FILE)) {
    try {
      const saved = JSON.parse(fs.readFileSync(BNET_CONFIG_FILE, 'utf8'))
      if (saved && saved.wine_prefix_path && fs.existsSync(saved.wine_prefix_path)) {
        let exe = saved.battlenet_exe_path
        if (!exe || !fs.existsSync(exe)) {
          const cand = path.join(saved.wine_prefix_path, 'drive_c/Program Files (x86)/Battle.net/Battle.net.exe')
          if (fs.existsSync(cand)) exe = cand
        }
        return {
          prefix: saved.wine_prefix_path,
          exe: exe && fs.existsSync(exe) ? exe : null,
          runner: saved.runner_executable || 'wine',
          isInstalled: Boolean(exe && fs.existsSync(exe))
        }
      }
    } catch {}
  }

  const candidatePrefixes = [
    path.join(os.homedir(), 'Games', 'battlenet'),
    path.join(os.homedir(), '.local/share/loom/prefixes/battlenet'),
    path.join(os.homedir(), '.wine'),
    path.join(os.homedir(), '.local/share/bottles/bottles/Battle-net'),
    path.join(os.homedir(), 'Games', 'battle-net')
  ]

  let resolvedPrefix = path.join(os.homedir(), '.local/share/loom/prefixes/battlenet')
  let resolvedExe: string | null = null

  for (const prefix of candidatePrefixes) {
    if (fs.existsSync(prefix)) {
      const candidates = [
        path.join(prefix, 'drive_c/Program Files (x86)/Battle.net/Battle.net.exe'),
        path.join(prefix, 'drive_c/Program Files (x86)/Battle.net/Battle.net Launcher.exe'),
        path.join(prefix, 'drive_c/Program Files/Battle.net/Battle.net.exe'),
        path.join(prefix, 'drive_c/Battle.net/Battle.net.exe')
      ]
      for (const cand of candidates) {
        if (fs.existsSync(cand)) {
          resolvedPrefix = prefix
          resolvedExe = cand
          break
        }
      }
      if (resolvedExe) break
    }
  }

  let runner = 'wine'
  const configInfoPath = path.join(resolvedPrefix, 'config_info')
  if (fs.existsSync(configInfoPath)) {
    try {
      const lines = fs.readFileSync(configInfoPath, 'utf8').split('\n')
      for (const line of lines) {
        const trimmed = line.trim()
        if (trimmed.includes('compatibilitytools.d') || trimmed.includes('runners/wine')) {
          const match = trimmed.match(/^(\/.*?\/(?:GE-Proton[^\/]+|wine-ge[^\/]+))(?:\/|$)/)
          if (match && match[1]) {
            const wineBin = path.join(match[1], 'files/bin/wine')
            const wineBinAlt = path.join(match[1], 'bin/wine')
            if (fs.existsSync(wineBin)) {
              runner = wineBin
              break
            } else if (fs.existsSync(wineBinAlt)) {
              runner = wineBinAlt
              break
            }
          }
        }
      }
    } catch {}
  }

  if (runner === 'wine') {
    const steamCompatDir = path.join(os.homedir(), '.local/share/Steam/compatibilitytools.d')
    if (fs.existsSync(steamCompatDir)) {
      try {
        const dirs = fs.readdirSync(steamCompatDir).filter((d) => d.startsWith('GE-Proton')).sort().reverse()
        for (const d of dirs) {
          const wineBin = path.join(steamCompatDir, d, 'files/bin/wine')
          if (fs.existsSync(wineBin)) {
            runner = wineBin
            break
          }
        }
      } catch {}
    }
  }

  if (runner === 'wine' && fs.existsSync('/usr/bin/wine')) {
    runner = '/usr/bin/wine'
  }

  return {
    prefix: resolvedPrefix,
    exe: resolvedExe,
    runner,
    isInstalled: resolvedExe !== null
  }
}

function detectGameLastPlayed(gameDir: string, subRel?: string): number | null {
  if (!fs.existsSync(gameDir)) return null
  let maxTime = 0
  const candidateSubPaths = [
    '_forever_/WTF/Config.wtf',
    '_forever_/Logs/Connection.log',
    '_forever_/Logs',
    '_forever_/WTF/Account',
    '_forever_/Wow.exe',
    '_retail_/WTF/Config.wtf',
    '_retail_/Logs',
    '_classic_/WTF/Config.wtf',
    '_classic_/Logs',
    'WTF/Config.wtf',
    'Logs',
    '.patch.result'
  ]
  if (subRel) candidateSubPaths.unshift(subRel)

  for (const rel of candidateSubPaths) {
    const full = path.join(gameDir, rel)
    if (fs.existsSync(full)) {
      try {
        const stats = fs.statSync(full)
        const mtimeSec = Math.floor(stats.mtimeMs / 1000)
        if (mtimeSec > maxTime) maxTime = mtimeSec
      } catch {}
    }
  }

  // Check persistent Loom last played file
  const lastPlayedFile = path.join(os.homedir(), '.local/share/loom/battlenet_last_played.json')
  if (fs.existsSync(lastPlayedFile)) {
    try {
      const map = JSON.parse(fs.readFileSync(lastPlayedFile, 'utf8'))
      const folderName = path.basename(gameDir)
      if (map[folderName] && map[folderName] > maxTime) maxTime = map[folderName]
      if (subRel && map[subRel] && map[subRel] > maxTime) maxTime = map[subRel]
    } catch {}
  }

  return maxTime > 0 ? maxTime : null
}

function isBattleNetRunning(): boolean {
  try {
    const out = execSync('pgrep -fa "Battle.net.exe"', { encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'] })
    return Boolean(out && out.trim().length > 0)
  } catch {
    return false
  }
}

function ensureBattleNetMinimizedConfig(prefix: string) {
  try {
    const usersDir = path.join(prefix, 'drive_c/users')
    if (!fs.existsSync(usersDir)) return
    const userFolders = fs.readdirSync(usersDir)
    for (const u of userFolders) {
      const cfgPath = path.join(usersDir, u, 'AppData/Roaming/Battle.net/Battle.net.config')
      if (fs.existsSync(cfgPath)) {
        try {
          const content = fs.readFileSync(cfgPath, 'utf8')
          const cfg = JSON.parse(content)
          if (!cfg.Client) cfg.Client = {}
          let changed = false
          if (cfg.Client.AutoStartMinimized !== 'true') { cfg.Client.AutoStartMinimized = 'true'; changed = true }
          if (cfg.Client.MinimizedOnStartup !== 'true') { cfg.Client.MinimizedOnStartup = 'true'; changed = true }
          if (cfg.Client.GameLaunchWindowBehavior !== '2') { cfg.Client.GameLaunchWindowBehavior = '2'; changed = true }
          if (cfg.Client.HideOnClose !== 'true') { cfg.Client.HideOnClose = 'true'; changed = true }
          if (cfg.Client.RestoreWindowOnGameEnd !== 'false') { cfg.Client.RestoreWindowOnGameEnd = 'false'; changed = true }
          if (changed) {
            fs.writeFileSync(cfgPath, JSON.stringify(cfg, null, 4), 'utf8')
            console.info(`[Loom::BattleNet] Configuration Battle.net mise à jour pour démarrage réduit dans le system tray (${cfgPath})`)
          }
        } catch (e) {
          console.warn(`[Loom::BattleNet] Impossible de mettre à jour ${cfgPath}:`, e)
        }
      }
    }
  } catch (err) {
    console.warn('[Loom::BattleNet] Erreur ensureBattleNetMinimizedConfig:', err)
  }
}

function battlenetPlugin(): Plugin {
  const bnetDir = path.join(os.homedir(), '.local/share/loom/prefixes/battlenet')
  const BNET_AUTH_FILE = path.join(os.homedir(), '.local/share', 'loom', 'battlenet_account.json')
  const BNET_EXTERNAL_GAMES_FILE = path.join(os.homedir(), '.local/share', 'loom', 'battlenet_external_games.json')
  const envFilePath = path.join(__dirname, '.env')
  const runningBnetProcesses = new Map<string, number>()

  function scanInstalledBattleNetGames(prefix: string) {
    const games: any[] = []
    const roots = [
      path.join(prefix, 'drive_c/Program Files (x86)'),
      path.join(prefix, 'drive_c/Program Files')
    ]

    for (const root of roots) {
      if (!fs.existsSync(root)) continue

      // Specific Deep Detection for World of Warcraft & its sub-games / flavors
      const wowDir = path.join(root, 'World of Warcraft')
      if (fs.existsSync(wowDir)) {
        let wowVersion = '1.60.1.70790'
        const buildInfo = path.join(wowDir, '.build.info')
        if (fs.existsSync(buildInfo)) {
          try {
            const bContent = fs.readFileSync(buildInfo, 'utf8')
            const lines = bContent.split('\n')
            if (lines.length >= 2) {
              const headers = lines[0].split('|').map((h) => h.split('!')[0].trim().toLowerCase())
              const values = lines[1].split('|')
              const vIdx = headers.indexOf('version')
              if (vIdx !== -1 && values[vIdx]) wowVersion = values[vIdx].trim()
            }
          } catch {}
        }

        const WOW_FLAVORS = [
          {
            id: 'forever',
            dir: '_forever_',
            exe: 'Wow.exe',
            name: 'World of Warcraft: Forever',
            productUid: 'wowf',
            aliasUid: 'wow',
            desc: "World of Warcraft: Forever sous Linux. Réinterprétation permanente d'Azeroth au niveau 60 (Classic+) avec nouvelles zones et donjons.",
            cover: 'https://images.igdb.com/igdb/image/upload/t_cover_big/co7bkq.jpg',
            igdbId: 417650,
            igdbSlug: 'world-of-warcraft-forever',
            igdbUrl: 'https://www.igdb.com/games/world-of-warcraft-forever'
          },
          {
            id: 'retail',
            dir: '_retail_',
            exe: 'Wow.exe',
            name: 'World of Warcraft',
            productUid: 'wow',
            aliasUid: 'wow',
            desc: 'Explorez Azeroth dans The War Within. Affrontez les menaces souterraines et unissez les factions.',
            cover: 'https://images.igdb.com/igdb/image/upload/t_cover_big/co8b7k.jpg',
            igdbId: 123,
            igdbSlug: 'world-of-warcraft',
            igdbUrl: 'https://www.igdb.com/games/world-of-warcraft'
          },
          {
            id: 'classic',
            dir: '_classic_',
            exe: 'WowClassic.exe',
            name: 'World of Warcraft Classic',
            productUid: 'wow_classic',
            aliasUid: 'wow_classic',
            desc: "Revivez l'expérience originelle et authentique de World of Warcraft Classic.",
            cover: 'https://images.igdb.com/igdb/image/upload/t_cover_big/co1trz.jpg',
            igdbId: 75379,
            igdbSlug: 'world-of-warcraft-classic',
            igdbUrl: 'https://www.igdb.com/games/world-of-warcraft-classic'
          },
          {
            id: 'classic_era',
            dir: '_classic_era_',
            exe: 'WowClassic.exe',
            name: 'World of Warcraft Classic: Season of Discovery',
            productUid: 'wow_classic_era',
            aliasUid: 'wow_classic_era',
            desc: 'World of Warcraft Classic: Season of Discovery, runes de classe et raids réinventés.',
            cover: 'https://images.igdb.com/igdb/image/upload/t_cover_big/co7bkt.jpg',
            igdbId: 275175,
            igdbSlug: 'world-of-warcraft-classic-season-of-discovery',
            igdbUrl: 'https://www.igdb.com/games/world-of-warcraft-classic-season-of-discovery'
          }
        ]

        const installedFlavors = WOW_FLAVORS.filter((fl) => fs.existsSync(path.join(wowDir, fl.dir, fl.exe)))

        if (installedFlavors.length > 0) {
          for (const fl of installedFlavors) {
            const lp = detectGameLastPlayed(wowDir, path.join(fl.dir, 'WTF/Config.wtf'))
            games.push({
              product_uid: fl.productUid,
              name: fl.name,
              install_path: wowDir,
              executable_relative_path: path.join(fl.dir, fl.exe),
              version_build: wowVersion,
              is_external: false,
              is_installed: true,
              last_played: lp,
              sub_game: fl.id,
              summary: fl.desc,
              cover_url: fl.cover,
              igdb_id: fl.igdbId,
              igdb_slug: fl.igdbSlug,
              igdb_url: fl.igdbUrl
            })
          }
        } else {
          // Fallback root check
          const rootExe = path.join(wowDir, 'Wow.exe')
          if (fs.existsSync(rootExe)) {
            const lp = detectGameLastPlayed(wowDir, 'WTF/Config.wtf')
            games.push({
              product_uid: 'wow',
              name: 'World of Warcraft',
              install_path: wowDir,
              executable_relative_path: 'Wow.exe',
              version_build: wowVersion,
              is_external: false,
              is_installed: true,
              last_played: lp,
              sub_game: 'retail'
            })
          }
        }
      }

      // Other Blizzard Games
      const OTHER_GAMES = [
        {
          dirNames: ['Overwatch'],
          uid: 'pro',
          name: 'Overwatch 2',
          exes: ['_retail_/Overwatch.exe', 'Overwatch.exe']
        },
        {
          dirNames: ['Diablo IV'],
          uid: 'fenris',
          name: 'Diablo IV',
          exes: ['Diablo IV.exe', 'Diablo IV Launcher.exe']
        },
        {
          dirNames: ['Diablo II Resurrected', 'Diablo II: Resurrected'],
          uid: 'osi',
          name: 'Diablo II: Resurrected',
          exes: ['D2R.exe']
        },
        {
          dirNames: ['Diablo III'],
          uid: 'diablo3',
          name: 'Diablo III',
          exes: ['x64/Diablo III64.exe', 'Diablo III.exe']
        },
        {
          dirNames: ['Warcraft III'],
          uid: 'w3',
          name: 'Warcraft III: Reforged',
          exes: ['_retail_/x86_64/Warcraft III.exe', 'x86_64/Warcraft III.exe', 'Warcraft III.exe']
        },
        {
          dirNames: ['Hearthstone'],
          uid: 'hs',
          name: 'Hearthstone',
          exes: ['Hearthstone.exe']
        },
        {
          dirNames: ['StarCraft II'],
          uid: 'sc2',
          name: 'StarCraft II',
          exes: ['Support64/SC2Switcher_x64.exe', 'StarCraft II.exe']
        },
        {
          dirNames: ['StarCraft'],
          uid: 's1',
          name: 'StarCraft: Remastered',
          exes: ['x86_64/StarCraft.exe', 'StarCraft.exe']
        },
        {
          dirNames: ['Heroes of the Storm'],
          uid: 'hero',
          name: 'Heroes of the Storm',
          exes: ['Support64/HeroesSwitcher_x64.exe', 'Heroes of the Storm.exe']
        },
        {
          dirNames: ['Call of Duty'],
          uid: 'odin',
          name: 'Call of Duty: Warzone',
          exes: ['ModernWarfare.exe', 'CoD.exe']
        }
      ]

      for (const def of OTHER_GAMES) {
        if (games.some((g) => g.product_uid === def.uid)) continue
        for (const dirName of def.dirNames) {
          const gameDir = path.join(root, dirName)
          if (fs.existsSync(gameDir)) {
            let foundExe: string | null = null
            for (const relExe of def.exes) {
              if (fs.existsSync(path.join(gameDir, relExe))) {
                foundExe = relExe
                break
              }
            }
            let version: string | null = null
            const buildInfo = path.join(gameDir, '.build.info')
            if (fs.existsSync(buildInfo)) {
              try {
                const bContent = fs.readFileSync(buildInfo, 'utf8')
                const lines = bContent.split('\n')
                if (lines.length >= 2) {
                  const headers = lines[0].split('|').map((h) => h.split('!')[0].trim().toLowerCase())
                  const values = lines[1].split('|')
                  const vIdx = headers.indexOf('version')
                  if (vIdx !== -1 && values[vIdx]) {
                    version = values[vIdx].trim()
                  }
                }
              } catch {}
            }

            if (foundExe) {
              const lp = detectGameLastPlayed(gameDir, foundExe)
              games.push({
                product_uid: def.uid,
                name: def.name,
                install_path: gameDir,
                executable_relative_path: foundExe,
                version_build: version,
                is_external: false,
                is_installed: true,
                last_played: lp
              })
              break
            }
          }
        }
      }
    }

    if (fs.existsSync(BNET_EXTERNAL_GAMES_FILE)) {
      try {
        const extList = JSON.parse(fs.readFileSync(BNET_EXTERNAL_GAMES_FILE, 'utf8'))
        if (Array.isArray(extList)) {
          for (const eg of extList) {
            if (!games.some((g) => g.product_uid === eg.product_uid) && fs.existsSync(eg.install_path)) {
              games.push(eg)
            }
          }
        }
      } catch {}
    }

    return games
  }

  function loadBnetCredentials(): { clientId: string | null; clientSecret: string | null } {
    let clientId: string | null = process.env.BLIZZARD_CLIENT_ID || null
    let clientSecret: string | null = process.env.BLIZZARD_CLIENT_SECRET || null

    if (fs.existsSync(envFilePath)) {
      try {
        const content = fs.readFileSync(envFilePath, 'utf8')
        for (const line of content.split('\n')) {
          const trimmed = line.trim()
          if (trimmed.startsWith('#') || !trimmed) continue
          const eqIdx = trimmed.indexOf('=')
          if (eqIdx !== -1) {
            const key = trimmed.substring(0, eqIdx).trim()
            const val = trimmed.substring(eqIdx + 1).trim().replace(/^["']|["']$/g, '')
            if (key === 'BLIZZARD_CLIENT_ID' && val) clientId = val
            if (key === 'BLIZZARD_CLIENT_SECRET' && val) clientSecret = val
          }
        }
      } catch {}
    }
    return { clientId, clientSecret }
  }

  function saveBnetCredentials(clientId: string, clientSecret: string) {
    let content = ''
    if (fs.existsSync(envFilePath)) {
      content = fs.readFileSync(envFilePath, 'utf8')
    }

    const lines = content.split('\n')
    let foundId = false
    let foundSecret = false
    const newLines = lines.map((line) => {
      const trimmed = line.trim()
      if (trimmed.startsWith('BLIZZARD_CLIENT_ID=')) {
        foundId = true
        return `BLIZZARD_CLIENT_ID="${clientId.trim()}"`
      }
      if (trimmed.startsWith('BLIZZARD_CLIENT_SECRET=')) {
        foundSecret = true
        return `BLIZZARD_CLIENT_SECRET="${clientSecret.trim()}"`
      }
      return line
    })

    if (!foundId) {
      newLines.push(`BLIZZARD_CLIENT_ID="${clientId.trim()}"`)
    }
    if (!foundSecret) {
      newLines.push(`BLIZZARD_CLIENT_SECRET="${clientSecret.trim()}"`)
    }

    fs.writeFileSync(envFilePath, newLines.filter(Boolean).join('\n') + '\n', 'utf8')
    process.env.BLIZZARD_CLIENT_ID = clientId.trim()
    process.env.BLIZZARD_CLIENT_SECRET = clientSecret.trim()
  }

  function loadBnetAccount() {
    if (fs.existsSync(BNET_AUTH_FILE)) {
      try {
        const acc = JSON.parse(fs.readFileSync(BNET_AUTH_FILE, 'utf8'))
        if (
          acc &&
          (acc.account_id?.startsWith('demo-') ||
            acc.battletag?.includes('LoomPlayer') ||
            acc.battletag?.includes('LoomChampion'))
        ) {
          removeBnetAccount()
          return null
        }
        // Sanitize broken legacy avatar URLs
        if (acc && acc.avatar_url && acc.avatar_url.includes('guildrecruitment.world')) {
          acc.avatar_url = `https://ui-avatars.com/api/?name=${encodeURIComponent(
            (acc.battletag || 'Player').split('#')[0]
          )}&background=0284c7&color=ffffff&bold=true&size=128`
          saveBnetAccount(acc)
        }
        return acc
      } catch {}
    }
    return null
  }

  function saveBnetAccount(acc: any) {
    fs.mkdirSync(path.dirname(BNET_AUTH_FILE), { recursive: true })
    fs.writeFileSync(BNET_AUTH_FILE, JSON.stringify(acc, null, 2), 'utf8')
  }

  function removeBnetAccount() {
    if (fs.existsSync(BNET_AUTH_FILE)) {
      try {
        fs.unlinkSync(BNET_AUTH_FILE)
      } catch {}
    }
  }

  function bnetPostForm(urlStr: string, formObj: Record<string, string>, extraHeaders: Record<string, string> = {}): Promise<any> {
    return new Promise((resolve, reject) => {
      const url = new URL(urlStr)
      const postData = new URLSearchParams(formObj).toString()
      const req = https.request(
        {
          hostname: url.hostname,
          port: 443,
          path: url.pathname + url.search,
          method: 'POST',
          family: 4,
          headers: {
            'Content-Type': 'application/x-www-form-urlencoded',
            'Content-Length': Buffer.byteLength(postData),
            'User-Agent': 'LoomLauncher/0.1.0',
            ...extraHeaders
          }
        },
        (res) => {
          let data = ''
          res.on('data', (c) => (data += c))
          res.on('end', () => {
            try {
              resolve(JSON.parse(data))
            } catch {
              resolve(data)
            }
          })
        }
      )
      req.on('error', reject)
      req.write(postData)
      req.end()
    })
  }

  function bnetGetJson(urlStr: string, headers: Record<string, string> = {}): Promise<any> {
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

  const pendingBnetLogins = new Map<
    string,
    {
      resolve: (acc: any) => void
      reject: (err: any) => void
      timer: NodeJS.Timeout
    }
  >()

  return {
    name: 'loom-battlenet-server',
    configureServer(server) {
      server.middlewares.use(async (req, res, next) => {
        if (!req.url?.startsWith('/api/battlenet') && !req.url?.startsWith('/api/auth/battlenet')) {
          return next()
        }

        // 1. GET /api/auth/battlenet/credentials
        if (req.method === 'GET' && req.url === '/api/auth/battlenet/credentials') {
          res.setHeader('Content-Type', 'application/json')
          const creds = loadBnetCredentials()
          const configured = !!(creds.clientId && creds.clientSecret)
          res.end(JSON.stringify({ configured, clientId: creds.clientId }))
          return
        }

        // 2. POST /api/auth/battlenet/credentials
        if (req.method === 'POST' && req.url === '/api/auth/battlenet/credentials') {
          res.setHeader('Content-Type', 'application/json')
          let body = ''
          req.on('data', (c) => (body += c))
          req.on('end', () => {
            try {
              const parsed = body ? JSON.parse(body) : {}
              const { clientId, clientSecret } = parsed
              if (!clientId || !clientSecret) {
                res.statusCode = 400
                res.end(JSON.stringify({ error: 'Client ID et Client Secret requis' }))
                return
              }
              saveBnetCredentials(clientId, clientSecret)
              res.end(JSON.stringify({ success: true }))
            } catch (err: any) {
              res.statusCode = 500
              res.end(JSON.stringify({ error: err.message }))
            }
          })
          return
        }

        // 3. POST /api/auth/battlenet/login
        if (req.method === 'POST' && req.url === '/api/auth/battlenet/login') {
          res.setHeader('Content-Type', 'application/json')
          const creds = loadBnetCredentials()
          if (!creds.clientId || !creds.clientSecret) {
            res.statusCode = 400
            res.end(JSON.stringify({ error: 'CREDENTIALS_REQUIRED' }))
            return
          }

          const state = Math.random().toString(36).substring(2) + Date.now().toString(36)
          const redirectUri = 'http://localhost:5173/api/auth/battlenet/callback'
          const authUrl = `https://oauth.battle.net/authorize?client_id=${encodeURIComponent(
            creds.clientId
          )}&redirect_uri=${encodeURIComponent(
            redirectUri
          )}&response_type=code&scope=openid%20wow.profile%20sc2.profile%20d3.profile&state=${state}`

          console.info('[Loom::BlizzardOAuth] Ouverture du navigateur sur:', authUrl)

          // Spawn browser on Linux / Mac / Win
          const openCmd =
            process.platform === 'linux' ? 'xdg-open' : process.platform === 'darwin' ? 'open' : 'start ""'
          exec(`${openCmd} "${authUrl}"`, (err) => {
            if (err) console.warn('[Loom::BlizzardOAuth] Erreur xdg-open:', err.message)
          })

          // Wait for callback up to 120s
          const accountPromise = new Promise((resolve, reject) => {
            const timer = setTimeout(() => {
              pendingBnetLogins.delete(state)
              reject(new Error("Délai d'attente dépassé (120s)"))
            }, 120000)

            pendingBnetLogins.set(state, { resolve, reject, timer })
          })

          try {
            const account = await accountPromise
            res.end(JSON.stringify(account))
          } catch (err: any) {
            res.statusCode = 408
            res.end(JSON.stringify({ error: err.message || 'Délai dépassé' }))
          }
          return
        }

        // 4. GET /api/auth/battlenet/callback
        if (req.method === 'GET' && req.url?.startsWith('/api/auth/battlenet/callback')) {
          const parsedUrl = new URL(req.url, 'http://localhost:5173')
          const code = parsedUrl.searchParams.get('code')
          const state = parsedUrl.searchParams.get('state')

          if (!code || !state || !pendingBnetLogins.has(state)) {
            res.statusCode = 400
            res.setHeader('Content-Type', 'text/html; charset=UTF-8')
            res.end(`<!DOCTYPE html>
<html><body style="background:#09090b;color:#f87171;font-family:sans-serif;text-align:center;padding:50px;">
<h2>Erreur de validation OAuth Blizzard</h2>
<p>Paramètres de redirection invalides ou expirés.</p></body></html>`)
            return
          }

          const pending = pendingBnetLogins.get(state)!
          clearTimeout(pending.timer)
          pendingBnetLogins.delete(state)

          const creds = loadBnetCredentials()
          if (!creds.clientId || !creds.clientSecret) {
            res.statusCode = 400
            res.end('Identifiants Blizzard introuvables')
            pending.reject(new Error('CREDENTIALS_REQUIRED'))
            return
          }

          const redirectUri = 'http://localhost:5173/api/auth/battlenet/callback'
          const basicAuth = Buffer.from(`${creds.clientId}:${creds.clientSecret}`).toString('base64')

          try {
            const tokenData = await bnetPostForm(
              'https://oauth.battle.net/token',
              {
                grant_type: 'authorization_code',
                code,
                redirect_uri: redirectUri
              },
              {
                Authorization: `Basic ${basicAuth}`
              }
            )

            if (!tokenData?.access_token) {
              const errMsg = tokenData?.error_description || tokenData?.error || 'Échec obtention token Blizzard'
              res.statusCode = 400
              res.setHeader('Content-Type', 'text/html; charset=UTF-8')
              res.end(`<!DOCTYPE html>
<html><body style="background:#09090b;color:#f87171;font-family:sans-serif;text-align:center;padding:50px;">
<h2>Échec échange de token Blizzard</h2><p>${errMsg}</p></body></html>`)
              pending.reject(new Error(errMsg))
              return
            }

            // Fetch official UserInfo
            const userInfoRes = await bnetGetJson('https://oauth.battle.net/oauth/userinfo', {
              Authorization: `Bearer ${tokenData.access_token}`
            })

            const userInfo = userInfoRes.json || {}
            const battletag = userInfo.battletag || 'BlizzardPlayer'
            const accountId = String(userInfo.id || userInfo.sub || battletag)
            let avatarUrl = `https://ui-avatars.com/api/?name=${encodeURIComponent(
              battletag.split('#')[0]
            )}&background=0284c7&color=ffffff&bold=true&size=128`

            // Try to resolve real WoW character avatar
            try {
              const wowRes = await bnetGetJson(
                'https://eu.api.blizzard.com/profile/user/wow?namespace=profile-eu&locale=fr_FR',
                { Authorization: `Bearer ${tokenData.access_token}` }
              )
              if (wowRes?.json?.wow_accounts) {
                let highestChar: any = null
                for (const wacc of wowRes.json.wow_accounts) {
                  for (const c of wacc.characters || []) {
                    if (!highestChar || (c.level || 0) > (highestChar.level || 0)) {
                      highestChar = c
                    }
                  }
                }
                if (highestChar?.realm?.slug && highestChar?.name) {
                  const mediaRes = await bnetGetJson(
                    `https://eu.api.blizzard.com/profile/wow/character/${highestChar.realm.slug}/${encodeURIComponent(
                      highestChar.name.toLowerCase()
                    )}/character-media?namespace=profile-eu`,
                    { Authorization: `Bearer ${tokenData.access_token}` }
                  )
                  const avAsset = (mediaRes?.json?.assets || []).find((a: any) => a.key === 'avatar')
                  if (avAsset?.value) {
                    avatarUrl = avAsset.value
                  }
                }
              }
            } catch {}

            const account = {
              account_id: accountId,
              battletag,
              avatar_url: avatarUrl,
              access_token: tokenData.access_token,
              refresh_token: tokenData.refresh_token || '',
              expires_at: Math.floor(Date.now() / 1000) + (tokenData.expires_in || 86400)
            }

            saveBnetAccount(account)
            pending.resolve(account)

            res.setHeader('Content-Type', 'text/html; charset=UTF-8')
            res.end(`<!DOCTYPE html>
<html lang="fr">
<head>
  <meta charset="utf-8">
  <title>Loom - Authentification Blizzard Réussie</title>
  <style>
    body {
      margin: 0;
      background: #09090b;
      color: #f4f4f5;
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
      display: flex;
      align-items: center;
      justify-content: center;
      height: 100vh;
    }
    .card {
      background: #18181b;
      border: 1px solid #27272a;
      border-radius: 16px;
      padding: 32px 40px;
      max-width: 440px;
      text-align: center;
      box-shadow: 0 20px 40px rgba(0,0,0,0.6);
    }
    .badge {
      display: inline-block;
      padding: 4px 12px;
      border-radius: 9999px;
      font-size: 12px;
      font-weight: 700;
      text-transform: uppercase;
      background: rgba(245, 158, 11, 0.15);
      color: #f59e0b;
      border: 1px solid rgba(245, 158, 11, 0.4);
      margin-bottom: 16px;
    }
    h1 {
      margin: 0 0 8px;
      font-size: 22px;
      font-weight: 800;
      color: #fafafa;
    }
    .tag {
      font-size: 17px;
      font-weight: 700;
      color: #f59e0b;
      margin: 4px 0 16px;
    }
    p {
      color: #a1a1aa;
      font-size: 14px;
      line-height: 1.5;
      margin: 0 0 20px;
    }
    .hint {
      color: #71717a;
      font-size: 12px;
    }
  </style>
</head>
<body>
  <div class="card">
    <div class="badge">Blizzard OAuth 2.0</div>
    <h1>Connexion Réussie !</h1>
    <div class="tag">${battletag}</div>
    <p>Votre compte officiel Blizzard Battle.net est maintenant lié à Loom Launcher.</p>
    <div class="hint">Vous pouvez fermer cet onglet et retourner sur l'application.</div>
  </div>
  <script>
    setTimeout(() => {
      try { window.close() } catch (e) {}
    }, 1800)
  </script>
</body>
</html>`)
          } catch (err: any) {
            console.error('[Loom::BlizzardOAuth] Callback error:', err)
            res.statusCode = 500
            res.setHeader('Content-Type', 'text/html; charset=UTF-8')
            res.end(`<!DOCTYPE html><html><body style="background:#09090b;color:#f87171;padding:50px;text-align:center;">
<h2>Erreur interne OAuth</h2><p>${err.message}</p></body></html>`)
            pending.reject(err)
          }
          return
        }

        // 5. GET /api/auth/battlenet/account
        if (req.method === 'GET' && req.url === '/api/auth/battlenet/account') {
          res.setHeader('Content-Type', 'application/json')
          const account = loadBnetAccount()
          if (!account) {
            res.end(JSON.stringify({ account: null }))
            return
          }

          const now = Math.floor(Date.now() / 1000)
          if (account.expires_at && account.expires_at <= now + 300 && account.refresh_token) {
            const creds = loadBnetCredentials()
            if (creds.clientId && creds.clientSecret) {
              try {
                const basicAuth = Buffer.from(`${creds.clientId}:${creds.clientSecret}`).toString('base64')
                const refreshData = await bnetPostForm(
                  'https://oauth.battle.net/token',
                  {
                    grant_type: 'refresh_token',
                    refresh_token: account.refresh_token
                  },
                  {
                    Authorization: `Basic ${basicAuth}`
                  }
                )
                if (refreshData?.access_token) {
                  account.access_token = refreshData.access_token
                  if (refreshData.refresh_token) account.refresh_token = refreshData.refresh_token
                  account.expires_at = now + (refreshData.expires_in || 86400)
                  saveBnetAccount(account)
                }
              } catch {}
            }
          }

          res.end(JSON.stringify({ account }))
          return
        }

        // 6. POST /api/auth/battlenet/logout
        if (req.method === 'POST' && req.url === '/api/auth/battlenet/logout') {
          res.setHeader('Content-Type', 'application/json')
          removeBnetAccount()
          res.end(JSON.stringify({ success: true }))
          return
        }

        res.setHeader('Content-Type', 'application/json')

        // 7. GET /api/battlenet/config
        if (req.method === 'GET' && req.url === '/api/battlenet/config') {
          const env = resolveBattleNetEnvironment()
          res.end(
            JSON.stringify({
              id: 'default',
              wine_prefix_path: env.prefix,
              runner_executable: env.runner,
              battlenet_exe_path: env.exe || path.join(env.prefix, 'drive_c/Program Files (x86)/Battle.net/Battle.net.exe'),
              enable_gamemode: 1,
              enable_mangohud: 0,
              custom_env_vars: null
            })
          )
          return
        }

        // 7b. POST /api/battlenet/config
        if (req.method === 'POST' && req.url === '/api/battlenet/config') {
          let body = ''
          req.on('data', (c) => (body += c))
          req.on('end', () => {
            try {
              const parsed = body ? JSON.parse(body) : {}
              fs.mkdirSync(path.dirname(BNET_CONFIG_FILE), { recursive: true })
              fs.writeFileSync(BNET_CONFIG_FILE, JSON.stringify(parsed.config || parsed, null, 2), 'utf8')
              res.end(JSON.stringify({ success: true }))
            } catch (err: any) {
              res.statusCode = 500
              res.end(JSON.stringify({ error: err.message }))
            }
          })
          return
        }

        // 8. GET /api/battlenet/games
        if (req.method === 'GET' && req.url === '/api/battlenet/games') {
          const env = resolveBattleNetEnvironment()
          const games = scanInstalledBattleNetGames(env.prefix)
          res.end(JSON.stringify(games))
          return
        }

        // 8b. POST /api/battlenet/games/external
        if (req.method === 'POST' && req.url === '/api/battlenet/games/external') {
          let body = ''
          req.on('data', (c) => (body += c))
          req.on('end', () => {
            try {
              const parsed = body ? JSON.parse(body) : {}
              const dirPath = parsed.path
              if (!dirPath || !fs.existsSync(dirPath)) {
                res.statusCode = 400
                res.end(JSON.stringify({ error: 'Chemin de dossier invalide ou introuvable' }))
                return
              }
              const folderName = path.basename(dirPath)
              let uid = 'external'
              let gameName = folderName
              const lower = folderName.toLowerCase()
              if (lower.includes('warcraft') && !lower.includes('reforged')) {
                uid = 'wow'
                gameName = 'World of Warcraft'
              } else if (lower.includes('overwatch')) {
                uid = 'pro'
                gameName = 'Overwatch 2'
              } else if (lower.includes('diablo iv') || lower.includes('diablo 4')) {
                uid = 'fenris'
                gameName = 'Diablo IV'
              } else if (lower.includes('diablo ii') || lower.includes('diablo 2')) {
                uid = 'osi'
                gameName = 'Diablo II: Resurrected'
              } else if (lower.includes('diablo iii') || lower.includes('diablo 3')) {
                uid = 'diablo3'
                gameName = 'Diablo III'
              } else if (lower.includes('starcraft ii') || lower.includes('starcraft 2')) {
                uid = 'sc2'
                gameName = 'StarCraft II'
              } else if (lower.includes('hearthstone')) {
                uid = 'hs'
                gameName = 'Hearthstone'
              }

              let foundExe = 'Game.exe'
              try {
                const files = fs.readdirSync(dirPath)
                const exe = files.find((f: string) => f.endsWith('.exe'))
                if (exe) foundExe = exe
              } catch {}

              const extGame = {
                product_uid: uid,
                name: gameName,
                install_path: dirPath,
                executable_relative_path: foundExe,
                version_build: '1.0.0.0',
                is_external: true,
                is_installed: true,
                last_played: null
              }

              let extList: any[] = []
              if (fs.existsSync(BNET_EXTERNAL_GAMES_FILE)) {
                try {
                  extList = JSON.parse(fs.readFileSync(BNET_EXTERNAL_GAMES_FILE, 'utf8'))
                  if (!Array.isArray(extList)) extList = []
                } catch {}
              }
              extList = extList.filter((g) => g.product_uid !== uid)
              extList.push(extGame)
              fs.mkdirSync(path.dirname(BNET_EXTERNAL_GAMES_FILE), { recursive: true })
              fs.writeFileSync(BNET_EXTERNAL_GAMES_FILE, JSON.stringify(extList, null, 2), 'utf8')

              res.end(JSON.stringify(extGame))
            } catch (err: any) {
              res.statusCode = 500
              res.end(JSON.stringify({ error: err.message }))
            }
          })
          return
        }

        // 9. GET /api/battlenet/owned or /api/auth/battlenet/owned
        if (
          req.method === 'GET' &&
          (req.url?.startsWith('/api/battlenet/owned') || req.url?.startsWith('/api/auth/battlenet/owned'))
        ) {
          const catalogue = [
            // --- Free-to-Play Games ---
            {
              product_uid: 'pro',
              name: 'Overwatch 2',
              is_owned: true,
              is_installed: false,
              install_path: null,
              version_build: null,
              cover_url: 'https://images.igdb.com/igdb/image/upload/t_cover_big/co885f.jpg',
              banner_url: 'https://shared.fastly.steamstatic.com/store_item_assets/steam/apps/2357570/library_hero.jpg',
              summary: 'Jeu de tir par équipe palpitant mettant en scène des héros aux pouvoirs uniques. Gratuit pour tous.',
              stats: { isFreeToPlay: true }
            },
            {
              product_uid: 'hs',
              name: 'Hearthstone',
              is_owned: true,
              is_installed: false,
              install_path: null,
              version_build: null,
              cover_url: 'https://images.igdb.com/igdb/image/upload/t_cover_big/co1sh2.jpg',
              banner_url: 'https://blz-contentstack-images.akamaized.net/v3/assets/bltc965041283bac56c/blt2a00940d7f28339b/64bea585e26c5443e04bc2a4/Masthead.png',
              summary: 'Jeu de cartes stratégique effréné dans la taverne de Warcraft. Duels intenses, sorts dévastateurs et serviteurs légendaires.',
              stats: { isFreeToPlay: true }
            },
            {
              product_uid: 'sc2',
              name: 'StarCraft II',
              is_owned: true,
              is_installed: false,
              install_path: null,
              version_build: null,
              cover_url: 'https://images.igdb.com/igdb/image/upload/t_cover_big/co1tnn.jpg',
              banner_url: 'https://blz-contentstack-images.akamaized.net/v3/assets/blt9c12f249ac15c7ec/bltb017c0048472fd04/69646379b1b0ff9682e39241/terran_background_2600.webp',
              summary: 'Le monument mondial de la stratégie en temps réel. Menez Terrans, Protoss ou Zergs vers la victoire. Campagne & Multi gratuits.',
              stats: { isFreeToPlay: true }
            },
            {
              product_uid: 'hero',
              name: 'Heroes of the Storm',
              is_owned: true,
              is_installed: false,
              install_path: null,
              version_build: null,
              cover_url: 'https://lutris.net/media/igdb/hots_grid_264x352.webp',
              banner_url: 'https://blz-contentstack-images.akamaized.net/v3/assets/blt9c12f249ac15c7ec/blta8d048de9718bc10/6a97a0ae2924f59b018aa920/masthead_desktop.png',
              summary: 'Les héros emblématiques des univers Warcraft, StarCraft et Diablo réunis dans des batailles d\'arène intenses.',
              stats: { isFreeToPlay: true }
            },
            {
              product_uid: 'anbs',
              name: 'Diablo Immortal',
              is_owned: true,
              is_installed: false,
              install_path: null,
              version_build: null,
              cover_url: 'https://images.igdb.com/igdb/image/upload/t_cover_big/co2pwy.jpg',
              banner_url: 'https://blz-contentstack-images.akamaized.net/v3/assets/blt9c12f249ac15c7ec/bltd1d54d0ae64bba83/6a2053f8927dbbbf1e03bc77/di-masthead-bg_XL_warlock_2600.webp',
              summary: 'MMO Action-RPG impitoyable se déroulant entre Diablo II et Diablo III. Affrontez les serviteurs de Skarn.',
              stats: { isFreeToPlay: true }
            },
            {
              product_uid: 'odin',
              name: 'Call of Duty: Warzone',
              is_owned: true,
              is_installed: false,
              install_path: null,
              version_build: null,
              cover_url: 'https://images.igdb.com/igdb/image/upload/t_cover_big/coa8id.jpg',
              banner_url: 'https://shared.fastly.steamstatic.com/store_item_assets/steam/apps/1938090/library_hero.jpg',
              summary: 'L\'expérience Battle Royale massive et gratuite de Call of Duty. Combats tactiques et guerre totale.',
              stats: { isFreeToPlay: true }
            },

            // --- Major Blizzard Licences ---
            {
              product_uid: 'wow',
              name: 'World of Warcraft',
              is_owned: true,
              is_installed: false,
              install_path: null,
              version_build: null,
              cover_url: 'https://images.igdb.com/igdb/image/upload/t_cover_big/co8b7k.jpg',
              banner_url: 'https://blz-contentstack-images.akamaized.net/v3/assets/blt9c12f249ac15c7ec/bltfe806acfa8672377/6a87bb854b66aa218691cf56/BG_Masthead_2600_Desktop.jpg',
              summary: 'Plongez dans Azeroth avec The War Within. Explorez les profondeurs inconnues et défendez votre faction.',
              stats: null
            },
            {
              product_uid: 'wowf',
              name: 'World of Warcraft: Forever',
              is_owned: true,
              is_installed: false,
              install_path: null,
              version_build: null,
              cover_url: 'https://images.igdb.com/igdb/image/upload/t_cover_big/co7bkq.jpg',
              banner_url: 'https://blz-contentstack-images.akamaized.net/v3/assets/blt9c12f249ac15c7ec/bltfe806acfa8672377/6a87bb854b66aa218691cf56/BG_Masthead_2600_Desktop.jpg',
              summary: "World of Warcraft: Forever sous Linux. Réinterprétation permanente d'Azeroth au niveau 60 (Classic+) avec nouvelles zones et donjons.",
              stats: null
            },
            {
              product_uid: 'wow_classic',
              name: 'World of Warcraft Classic',
              is_owned: true,
              is_installed: false,
              install_path: null,
              version_build: null,
              cover_url: 'https://images.igdb.com/igdb/image/upload/t_cover_big/co1trz.jpg',
              banner_url: 'https://blz-contentstack-images.akamaized.net/v3/assets/blt9c12f249ac15c7ec/bltfe806acfa8672377/6a87bb854b66aa218691cf56/BG_Masthead_2600_Desktop.jpg',
              summary: "Revivez l'expérience authentique de WoW originel et Classic Progression.",
              stats: null
            },
            {
              product_uid: 'wow_classic_era',
              name: 'World of Warcraft Classic: Season of Discovery',
              is_owned: true,
              is_installed: false,
              install_path: null,
              version_build: null,
              cover_url: 'https://images.igdb.com/igdb/image/upload/t_cover_big/co7bkt.jpg',
              banner_url: 'https://blz-contentstack-images.akamaized.net/v3/assets/blt9c12f249ac15c7ec/bltfe806acfa8672377/6a87bb854b66aa218691cf56/BG_Masthead_2600_Desktop.jpg',
              summary: "World of Warcraft Classic: Season of Discovery, runes de classe et raids réinventés.",
              stats: null
            },
            {
              product_uid: 'fenris',
              name: 'Diablo IV',
              is_owned: true,
              is_installed: false,
              install_path: null,
              version_build: null,
              cover_url: 'https://images.igdb.com/igdb/image/upload/t_cover_big/co69sm.jpg',
              banner_url: 'https://shared.fastly.steamstatic.com/store_item_assets/steam/apps/2344520/library_hero.jpg',
              summary: "L'obscurité s'étend sur Sanctuaire. Combattez les hordes de Lilith dans cet Action-RPG d'anthologie.",
              stats: null
            },
            {
              product_uid: 'osi',
              name: 'Diablo II: Resurrected',
              is_owned: true,
              is_installed: false,
              install_path: null,
              version_build: null,
              cover_url: 'https://images.igdb.com/igdb/image/upload/t_cover_big/co3gbk.jpg',
              banner_url: 'https://blz-contentstack-images.akamaized.net/v3/assets/blt9c12f249ac15c7ec/blt6c60900c9bb4c4d8/6972a545d855f0a07c5d7824/masthead_overview_2600.png',
              summary: 'Le classique intemporel sublimé en 4K avec audio 7.1 remasterisé fidèle au chef d\'œuvre.',
              stats: null
            },
            {
              product_uid: 'diablo3',
              name: 'Diablo III',
              is_owned: true,
              is_installed: false,
              install_path: null,
              version_build: null,
              cover_url: 'https://images.igdb.com/igdb/image/upload/t_cover_big/co2l1u.jpg',
              banner_url: 'https://blz-contentstack-images.akamaized.net/v3/assets/blt9c12f249ac15c7ec/blt9843230b12d957ea/6a8f5e2b2924f5fb788aa851/masthead_2600.png',
              summary: 'Retournez à la Nouvelle-Tristram et repoussez l\'invasion des armées démoniaques des Enfers.',
              stats: null
            },
            {
              product_uid: 'w3',
              name: 'Warcraft III: Reforged',
              is_owned: true,
              is_installed: false,
              install_path: null,
              version_build: null,
              cover_url: 'https://images.igdb.com/igdb/image/upload/t_cover_big/co1rrv.jpg',
              banner_url: 'https://blz-contentstack-images.akamaized.net/v3/assets/blt9c12f249ac15c7ec/blt668d39048ffb606f/6a9792a96cec0d4229b141a2/masthead_desktop.png',
              summary: "Revivez l'épopée fondatrice de Warcraft avec des graphismes remasterisés et l'éditeur de cartes culte.",
              stats: null
            },
            {
              product_uid: 's1',
              name: 'StarCraft: Remastered',
              is_owned: true,
              is_installed: false,
              install_path: null,
              version_build: null,
              cover_url: 'https://images.igdb.com/igdb/image/upload/t_cover_big/co3wal.jpg',
              banner_url: 'https://blz-contentstack-images.akamaized.net/v3/assets/blt9c12f249ac15c7ec/bltb017c0048472fd04/69646379b1b0ff9682e39241/terran_background_2600.webp',
              summary: 'Le classique originel remasterisé en 4K avec audio haute fidélité.',
              stats: null
            },
            {
              product_uid: 'w1r',
              name: 'Warcraft I: Remastered',
              is_owned: true,
              is_installed: false,
              install_path: null,
              version_build: null,
              cover_url: 'https://images.igdb.com/igdb/image/upload/t_cover_big/co1xup.jpg',
              banner_url: 'https://blz-contentstack-images.akamaized.net/v3/assets/blt9c12f249ac15c7ec/blt668d39048ffb606f/6a9792a96cec0d4229b141a2/masthead_desktop.png',
              summary: 'L\'origine légendaire du conflit Humains vs Orcs dans une version modernisée et fluide.',
              stats: null
            },
            {
              product_uid: 'w2r',
              name: 'Warcraft II: Remastered',
              is_owned: true,
              is_installed: false,
              install_path: null,
              version_build: null,
              cover_url: 'https://images.igdb.com/igdb/image/upload/t_cover_big/co3wix.jpg',
              banner_url: 'https://blz-contentstack-images.akamaized.net/v3/assets/blt9c12f249ac15c7ec/blt668d39048ffb606f/6a9792a96cec0d4229b141a2/masthead_desktop.png',
              summary: 'Tides of Darkness et Beyond the Dark Portal avec des graphismes rafraîchis et audio remasterisé.',
              stats: null
            },
            {
              product_uid: 'rtro',
              name: 'Blizzard Arcade Collection',
              is_owned: true,
              is_installed: false,
              install_path: null,
              version_build: null,
              cover_url: 'https://lutris.net/media/igdb/blizarcade.jpg',
              banner_url: 'https://blz-contentstack-images.akamaized.net/v3/assets/blt9c12f249ac15c7ec/blta8d048de9718bc10/6a97a0ae2924f59b018aa920/masthead_desktop.png',
              summary: 'The Lost Vikings, Blackthorne et Rock N\' Roll Racing dans leurs versions définitives.',
              stats: null
            }
          ]

          // If connected, attempt to fetch real WoW characters
          const acc = loadBnetAccount()
          if (acc?.access_token) {
            try {
              const wowRes = await bnetGetJson(
                'https://eu.api.blizzard.com/profile/user/wow?namespace=profile-eu&locale=fr_FR',
                {
                  Authorization: `Bearer ${acc.access_token}`
                }
              )
              if (wowRes?.json?.wow_accounts) {
                let charCount = 0
                let highestChar: any = null
                for (const wacc of wowRes.json.wow_accounts) {
                  for (const c of wacc.characters || []) {
                    charCount++
                    if (!highestChar || (c.level || 0) > (highestChar.level || 0)) {
                      highestChar = c
                    }
                  }
                }
                if (charCount > 0 && highestChar) {
                  const mainChar = `${highestChar.name} (${highestChar.playable_class?.name || ''} Niv. ${highestChar.level || 80}) - ${highestChar.realm?.name || ''}`
                  const statObj = {
                    charactersCount: charCount,
                    character: mainChar,
                    profileSynced: true
                  } as any
                  const wowGame = catalogue.find((g) => g.product_uid === 'wow')
                  if (wowGame) wowGame.stats = statObj
                  const wowfGame = catalogue.find((g) => g.product_uid === 'wowf')
                  if (wowfGame) wowfGame.stats = statObj
                }
              }
            } catch {}
          }

          // Merge locally detected installed games into catalogue
          try {
            const env = resolveBattleNetEnvironment()
            const detectedGames = scanInstalledBattleNetGames(env.prefix)
            for (const item of catalogue) {
              const detected = detectedGames.find(
                (d) => d.product_uid.toLowerCase() === item.product_uid.toLowerCase()
              )
              if (detected) {
                item.name = detected.name
                item.is_installed = true
                item.install_path = detected.install_path
                item.version_build = detected.version_build
                item.last_played = detected.last_played
                if (detected.summary) item.summary = detected.summary
                if (detected.cover_url) item.cover_url = detected.cover_url
              }
            }
          } catch {}

          res.end(JSON.stringify(catalogue))
          return
        }

        // 10. POST /api/battlenet/launch
        if (req.method === 'POST' && req.url.startsWith('/api/battlenet/launch')) {
          res.setHeader('Content-Type', 'application/json')
          let body = ''
          req.on('data', (c) => (body += c))
          req.on('end', () => {
            try {
              const parsed = body ? JSON.parse(body) : {}
              const productUid = String(parsed.productUid || parsed.product_uid || '').toLowerCase().trim()
              const env = resolveBattleNetEnvironment()

              if (!env.isInstalled || !env.exe) {
                res.statusCode = 404
                res.end(
                  JSON.stringify({
                    error: 'BATTLENET_NOT_INSTALLED',
                    message: "Le client Battle.net n'est pas encore installé. Veuillez d'abord installer Battle.net."
                  })
                )
                return
              }

              // 1. Assurer que Battle.net est configuré pour démarrer et rester réduit dans le system tray
              ensureBattleNetMinimizedConfig(env.prefix)

              const isRunning = isBattleNetRunning()
              console.info(`[Loom::BattleNet] Demande de lancement pour '${productUid}'. Battle.net est ${isRunning ? 'DÉJÀ EN COURS D\'EXÉCUTION (en arrière-plan)' : 'ARRÊTÉ (démarrage silencieux en system tray)'}`)

              // Normaliser l'UID interne Battle.net (WoW Forever = wowf)
              let bnetLaunchUid = productUid
              if (bnetLaunchUid === 'wow_forever') bnetLaunchUid = 'wowf'

              const hasGamemode = fs.existsSync('/usr/bin/gamemoderun')
              const hasMangohud = fs.existsSync('/usr/bin/mangohud')

              let rootBin = env.runner
              const cmdArgs: string[] = []

              if (hasGamemode) {
                rootBin = '/usr/bin/gamemoderun'
                if (hasMangohud) {
                  cmdArgs.push('mangohud')
                }
                cmdArgs.push(env.runner)
              } else if (hasMangohud) {
                rootBin = '/usr/bin/mangohud'
                cmdArgs.push(env.runner)
              }

              cmdArgs.push(env.exe)

              if (!isRunning) {
                // Battle.net n'est pas encore lancé :
                // On démarre avec --autostarted pour qu'il s'ouvre silencieusement réduit dans le system tray
                // sans JAMAIS afficher sa fenêtre principale, tout en transmettant l'ordre de lancement du jeu.
                cmdArgs.push('--autostarted')
              }

              if (bnetLaunchUid) {
                cmdArgs.push(`--exec=launch ${bnetLaunchUid}`)
              }

              const spawnEnv: NodeJS.ProcessEnv = {
                ...process.env,
                WINEPREFIX: env.prefix,
                WINEARCH: 'win64',
                DXVK_ASYNC: '1',
                VKD3D_CONFIG: 'dxr11,dxr'
              }

              console.info(`[Loom::BattleNet] Lancement : ${rootBin} ${cmdArgs.join(' ')} (WINEPREFIX=${env.prefix})`)

              const child = spawn(rootBin, cmdArgs, {
                env: spawnEnv,
                detached: true,
                stdio: 'ignore'
              })
              child.unref()

              if (child.pid) {
                runningBnetProcesses.set(productUid || 'battlenet', child.pid)
              }

              // Si Battle.net a été démarré à froid, relance de confirmation après 4s au cas où le premier démarrage
              // chargeait les modules internes avant de consommer la file d'attente
              if (!isRunning && bnetLaunchUid) {
                setTimeout(() => {
                  try {
                    console.info(`[Loom::BattleNet] Relance silencieuse de confirmation pour ${bnetLaunchUid}...`)
                    const reminder = spawn(env.runner, [env.exe!, `--exec=launch ${bnetLaunchUid}`], {
                      env: spawnEnv,
                      detached: true,
                      stdio: 'ignore'
                    })
                    reminder.unref()
                  } catch (e) {
                    console.warn('[Loom::BattleNet] Relance silencieuse ignorée:', e)
                  }
                }, 4000)
              }

              // Persist last played timestamp
              try {
                const lastPlayedFile = path.join(os.homedir(), '.local/share/loom/battlenet_last_played.json')
                let lpMap: Record<string, number> = {}
                if (fs.existsSync(lastPlayedFile)) {
                  try {
                    lpMap = JSON.parse(fs.readFileSync(lastPlayedFile, 'utf8'))
                  } catch {}
                }
                const nowSec = Math.floor(Date.now() / 1000)
                lpMap[productUid] = nowSec
                lpMap['World of Warcraft'] = nowSec
                lpMap['_forever_/WTF/Config.wtf'] = nowSec
                fs.mkdirSync(path.dirname(lastPlayedFile), { recursive: true })
                fs.writeFileSync(lastPlayedFile, JSON.stringify(lpMap, null, 2), 'utf8')
              } catch {}

              res.end(JSON.stringify({ success: true, pid: child.pid, message: `Battle.net lancé pour ${productUid}` }))
            } catch (err: any) {
              res.statusCode = 500
              res.end(JSON.stringify({ error: err.message || 'Erreur lors du lancement de Battle.net' }))
            }
          })
          return
        }

        // 11. POST /api/battlenet/kill
        if (req.method === 'POST' && req.url.startsWith('/api/battlenet/kill')) {
          res.setHeader('Content-Type', 'application/json')
          let body = ''
          req.on('data', (c) => (body += c))
          req.on('end', () => {
            try {
              const parsed = body ? JSON.parse(body) : {}
              const productUid = String(parsed.productUid || parsed.product_uid || '').toLowerCase().trim()
              const pid = runningBnetProcesses.get(productUid) || runningBnetProcesses.get('battlenet')
              if (pid) {
                try {
                  process.kill(pid, 'SIGTERM')
                } catch {}
                runningBnetProcesses.delete(productUid)
              }
              const env = resolveBattleNetEnvironment()
              try {
                execSync(`WINEPREFIX="${env.prefix}" wineserver -k`, { stdio: 'ignore' })
              } catch {}
              res.end(JSON.stringify({ success: true }))
            } catch (err: any) {
              res.statusCode = 500
              res.end(JSON.stringify({ error: err.message }))
            }
          })
          return
        }

        // 12. GET /api/battlenet/running
        if (req.method === 'GET' && req.url.startsWith('/api/battlenet/running')) {
          res.setHeader('Content-Type', 'application/json')
          const activeGames: string[] = []

          // Check known PIDs
          for (const [uid, pid] of runningBnetProcesses.entries()) {
            try {
              process.kill(pid, 0)
              activeGames.push(uid)
            } catch {
              runningBnetProcesses.delete(uid)
            }
          }

          // Check system processes for running Blizzard binaries via /proc inspection (avoids false positives)
          try {
            const procDirs = fs.readdirSync('/proc')
            for (const pid of procDirs) {
              if (!/^\d+$/.test(pid) || parseInt(pid, 10) === process.pid) continue
              try {
                const cmdline = fs.readFileSync(path.join('/proc', pid, 'cmdline'))
                if (cmdline.length === 0) continue
                const args = cmdline.toString('utf8').split('\0').filter(Boolean)
                if (args.length === 0) continue
                const exe0 = (args[0] || '').toLowerCase()
                if (exe0.includes('node') || exe0.includes('python') || exe0.includes('bash') || exe0.includes('grep')) continue

                const fullCmd = args.join(' ').toLowerCase()

                const uidIdx = args.indexOf('-uid')
                if (uidIdx !== -1 && args[uidIdx + 1]) {
                  const uid = args[uidIdx + 1].trim().toLowerCase()
                  if (uid && !activeGames.includes(uid)) activeGames.push(uid)
                }

                if (fullCmd.includes('_forever_\\wow.exe') || fullCmd.includes('_forever_/wow.exe')) {
                  if (!activeGames.includes('wowf')) activeGames.push('wowf')
                } else if (fullCmd.includes('_retail_\\wow.exe') || fullCmd.includes('_retail_/wow.exe')) {
                  if (!activeGames.includes('wow')) activeGames.push('wow')
                } else if (fullCmd.includes('_classic_\\wowclassic.exe') || fullCmd.includes('_classic_/wowclassic.exe')) {
                  if (!activeGames.includes('wow_classic')) activeGames.push('wow_classic')
                } else if (fullCmd.includes('_classic_era_\\wowclassic.exe') || fullCmd.includes('_classic_era_/wowclassic.exe')) {
                  if (!activeGames.includes('wow_classic_era')) activeGames.push('wow_classic_era')
                } else if (fullCmd.includes('diablo iv.exe')) {
                  if (!activeGames.includes('fenris')) activeGames.push('fenris')
                } else if (fullCmd.includes('overwatch.exe')) {
                  if (!activeGames.includes('pro')) activeGames.push('pro')
                } else if (fullCmd.includes('d2r.exe')) {
                  if (!activeGames.includes('osi')) activeGames.push('osi')
                }
              } catch {}
            }
          } catch {}

          res.end(JSON.stringify(activeGames))
          return
        }

        // 13. POST /api/battlenet/install-client
        if (req.method === 'POST' && req.url.startsWith('/api/battlenet/install-client')) {
          res.setHeader('Content-Type', 'application/json')
          const setupUrl =
            'https://www.battle.net/download/getInstallerForGame?os=win&locale=frFR&version=LIVE&gameProgram=BATTLENET_APP'
          const prefix = path.join(os.homedir(), '.local/share/loom/prefixes/battlenet')
          const setupExe = path.join(prefix, 'Battle.net-Setup.exe')

          fs.mkdirSync(prefix, { recursive: true })

          exec(`curl -L -s -o "${setupExe}" "${setupUrl}"`, (curlErr) => {
            if (curlErr) {
              res.statusCode = 500
              res.end(JSON.stringify({ error: "Échec du téléchargement de l'installateur Battle.net" }))
              return
            }

            const env = resolveBattleNetEnvironment()
            const runner = env.runner || 'wine'
            const child = spawn(runner, [setupExe], {
              env: {
                ...process.env,
                WINEPREFIX: prefix,
                WINEARCH: 'win64'
              },
              detached: true,
              stdio: 'ignore'
            })
            child.unref()

            res.end(JSON.stringify({ success: true, message: 'Installateur Battle.net lancé' }))
          })
          return
        }

        next()
      })
    }
  }
}

function wowAddonsPlugin(): Plugin {
  const KNOWN_ADDON_CURSEFORGE_MAP: Record<string, number> = {
    auctionator: 6124,
    questie: 334372,
    details: 61284,
    buggrabber: 13344,
    bugsack: 6273,
    dialogueui: 989270,
    tradeskillmaster: 26886,
    minimapbuttonbutton: 446096,
    rarescanner: 84208,
    bagnon: 1592,
    weakauras: 65387,
    plater: 100547,
    tomtom: 18808,
    dbm: 3358,
    dbmcore: 3358,
    bigwigs: 2382,
    bigwigscore: 2382,
    leatrix_plus: 94855,
    bartender4: 13501,
    ellesmereui: 1477613,
    openmap: 1735937,
    foreverauras: 65387,
    spoken: 1700375,
    naowhforever: 1721187,
    questiedb: 1087401,
    spokenallfrfr: 1746506,
    spokenquestsaudiosharedfrfr: 1717800,
    spokenquestsaudioalliancefrfr: 1743719,
    spokenquestsaudiohordefrfr: 1743737,
    spokenquestsaudiogossipfrfr: 1743728,
    spokenbooksaudiofrfr: 1723284,
    spokenzonesaudiofrfr: 1713125
  }

  function cleanAddonVersionString(v: string): string {
    if (!v) return ''
    return v
      .replace(/^v/i, '')
      .replace(/\.zip$/i, '')
      .replace(/[-_](forever|classic|retail|mainline)$/i, '')
      .replace(/^(Auctionator|Questie|EllesmereUI|BugSack|OpenMap|DialogueUI|TradeSkillMaster|BugGrabber|Spoken|NaowhForever|SpokenAll|SpokenQuestsAudio|SpokenBooksAudio|SpokenZonesAudio)[-_]/i, '')
      .trim()
  }

  function parseAddonVersionNumbers(v: string): number[] {
    const cleaned = cleanAddonVersionString(v)
    const matches = cleaned.match(/\d+/g)
    if (!matches) return []
    return matches.map((m) => parseInt(m, 10))
  }

  function isAddonVersionNewer(remoteStr: string, localStr: string): boolean {
    if (!remoteStr || !localStr) return false
    const cr = cleanAddonVersionString(remoteStr).toLowerCase()
    const cl = cleanAddonVersionString(localStr).toLowerCase()

    if (cr === cl) return false

    const rn = parseAddonVersionNumbers(remoteStr)
    const ln = parseAddonVersionNumbers(localStr)

    if (rn.length === 0 || ln.length === 0) return false

    const minLen = Math.min(rn.length, ln.length)
    for (let i = 0; i < minLen; i++) {
      if (rn[i] > ln[i]) return true
      if (rn[i] < ln[i]) return false
    }

    if (rn.length > ln.length && rn.slice(ln.length).some((n) => n > 0)) {
      return true
    }

    return false
  }

  async function scanWowInstances() {
    const instances: any[] = []
    const env = resolveBattleNetEnvironment()
    const roots = [
      path.join(env.prefix, 'drive_c/Program Files (x86)/World of Warcraft'),
      path.join(env.prefix, 'drive_c/Program Files/World of Warcraft'),
      '/home/k0bus/Games/battlenet/drive_c/Program Files (x86)/World of Warcraft'
    ]
    const validRoots = [...new Set(roots)].filter((r) => fs.existsSync(r))
    if (validRoots.length === 0) return []
    const wowDir = validRoots[0]

    const FLAVORS = [
      {
        id: 'forever',
        name: 'World of Warcraft: Forever',
        flavor: 'forever',
        subDir: '_forever_',
        versionType: 88568,
        defaultVersion: '1.60.1.70790',
        icon: 'https://images.igdb.com/igdb/image/upload/t_cover_big/co7bkq.jpg'
      },
      {
        id: 'retail',
        name: 'World of Warcraft (Retail)',
        flavor: 'retail',
        subDir: '_retail_',
        versionType: 517,
        defaultVersion: '11.0.2',
        icon: 'https://images.igdb.com/igdb/image/upload/t_cover_big/co8b7k.jpg'
      },
      {
        id: 'classic',
        name: 'World of Warcraft Classic',
        flavor: 'classic',
        subDir: '_classic_',
        versionType: 67408,
        defaultVersion: '1.15.5',
        icon: 'https://images.igdb.com/igdb/image/upload/t_cover_big/co1trz.jpg'
      },
      {
        id: 'classic_era',
        name: 'WoW Classic: Season of Discovery',
        flavor: 'classic_era',
        subDir: '_classic_era_',
        versionType: 67408,
        defaultVersion: '1.15.5',
        icon: 'https://images.igdb.com/igdb/image/upload/t_cover_big/co7bkt.jpg'
      }
    ]

    for (const fl of FLAVORS) {
      const instPath = path.join(wowDir, fl.subDir)
      const isInstalled = fs.existsSync(instPath)
      const addonsPath = path.join(instPath, 'Interface/AddOns')
      let addonsCount = 0
      if (fs.existsSync(addonsPath)) {
        try {
          addonsCount = fs.readdirSync(addonsPath, { withFileTypes: true }).filter((d) => d.isDirectory()).length
        } catch {}
      }

      let lp: number | null = null
      try {
        lp = detectGameLastPlayed(wowDir, path.join(fl.subDir, 'WTF/Config.wtf'))
      } catch {}

      instances.push({
        id: fl.id,
        name: fl.name,
        flavor: fl.flavor,
        path: instPath,
        addonsPath,
        version: fl.defaultVersion,
        gameVersionTypeId: fl.versionType,
        installed: isInstalled,
        addonsCount,
        icon: fl.icon,
        lastPlayed: lp
      })
    }

    return instances
  }

  function getDirectorySizeBytes(dirPath: string): number {
    let total = 0
    try {
      const entries = fs.readdirSync(dirPath, { withFileTypes: true })
      for (const entry of entries) {
        const full = path.join(dirPath, entry.name)
        if (entry.isDirectory()) {
          total += getDirectorySizeBytes(full)
        } else if (entry.isFile()) {
          try {
            total += fs.statSync(full).size
          } catch {}
        }
      }
    } catch {}
    return total
  }

  async function getInstalledWowAddons(addonsPath: string, checkUpdates = true, targetGameVersionTypeId?: number) {
    if (!fs.existsSync(addonsPath)) {
      try {
        fs.mkdirSync(addonsPath, { recursive: true })
      } catch {
        return []
      }
    }

    if (!targetGameVersionTypeId) {
      if (addonsPath.includes('_forever_')) targetGameVersionTypeId = 88568
      else if (addonsPath.includes('_classic_') || addonsPath.includes('_classic_era_')) targetGameVersionTypeId = 67408
      else if (addonsPath.includes('_retail_')) targetGameVersionTypeId = 517
    }

    const entries = fs.readdirSync(addonsPath, { withFileTypes: true })

    interface RawAddonMeta {
      folderName: string
      folderPath: string
      title: string
      version: string
      author?: string
      notes?: string
      website?: string
      category?: string
      interfaceVersion?: string
      group?: string
      xPartOf?: string
      xChildOf?: string
      globeMain?: string
      globePost: string[]
      dependencies: string[]
      loadOnDemand: boolean
      curseforgeId?: number
      fileId?: number
      sizeBytes: number
      lastModified: number
      enabled: boolean
      meta: any
      iconUrl?: string
    }

    const rawList: RawAddonMeta[] = []

    for (const entry of entries) {
      if (!entry.isDirectory()) continue
      const folderName = entry.name
      const folderPath = path.join(addonsPath, folderName)

      let files: string[] = []
      try {
        files = fs.readdirSync(folderPath)
      } catch {
        continue
      }

      const isEnabled = !files.some((f) => f.endsWith('.toc.disabled')) || files.some((f) => f.endsWith('.toc'))
      let tocCandidate: string | undefined
      if (targetGameVersionTypeId === 88568 || addonsPath.includes('_forever_')) {
        tocCandidate =
          files.find((f) => f.toLowerCase() === `${folderName.toLowerCase()}_forever.toc`) ||
          files.find((f) => f.toLowerCase() === `${folderName.toLowerCase()}_vanilla.toc`) ||
          files.find((f) => f.toLowerCase() === `${folderName.toLowerCase()}_classic.toc`)
      } else if (targetGameVersionTypeId === 67408 || addonsPath.includes('_classic_')) {
        tocCandidate =
          files.find((f) => f.toLowerCase() === `${folderName.toLowerCase()}_classic.toc`) ||
          files.find((f) => f.toLowerCase() === `${folderName.toLowerCase()}_vanilla.toc`)
      } else if (targetGameVersionTypeId === 517 || addonsPath.includes('_retail_')) {
        tocCandidate =
          files.find((f) => f.toLowerCase() === `${folderName.toLowerCase()}_mainline.toc`) ||
          files.find((f) => f.toLowerCase() === `${folderName.toLowerCase()}_retail.toc`)
      }

      if (!tocCandidate) {
        tocCandidate =
          files.find((f) => f.toLowerCase() === `${folderName.toLowerCase()}.toc`) ||
          files.find((f) => f.toLowerCase().startsWith(folderName.toLowerCase()) && f.endsWith('.toc')) ||
          files.find((f) => f.endsWith('.toc')) ||
          files.find((f) => f.endsWith('.toc.disabled'))
      }

      let title = folderName
      let version = '1.0.0'
      let author = ''
      let notes = ''
      let website = ''
      let category = ''
      let interfaceVersion = ''
      let group = ''
      let xPartOf = ''
      let xChildOf = ''
      let globeMain = ''
      const globePost: string[] = []
      const dependencies: string[] = []
      let loadOnDemand = false
      let tocCurseId: number | undefined

      if (tocCandidate) {
        try {
          const content = fs.readFileSync(path.join(folderPath, tocCandidate), 'utf8')
          for (const line of content.split('\n')) {
            const trimmed = line.trim()
            if (trimmed.startsWith('## Title:')) {
              title = trimmed
                .replace(/^## Title:\s*/, '')
                .replace(/\|c[0-9a-fA-F]{8}/gi, '')
                .replace(/\|r/gi, '')
                .replace(/\|c[0-9a-fA-F]{6}/gi, '')
                .replace(/\|T[^|]+\|t/gi, '')
                .replace(/<[^>]+>/g, '')
                .trim()
            } else if (trimmed.startsWith('## Version:')) {
              version = trimmed.replace(/^## Version:\s*/, '').trim()
            } else if (trimmed.startsWith('## Author:')) {
              author = trimmed.replace(/^## Author:\s*/, '').trim()
            } else if (trimmed.startsWith('## Notes-frFR:')) {
              notes = trimmed.replace(/^## Notes-frFR:\s*/, '').trim()
            } else if (trimmed.startsWith('## Notes:') && !notes) {
              notes = trimmed.replace(/^## Notes:\s*/, '').trim()
            } else if (trimmed.startsWith('## X-Website:') || trimmed.startsWith('## Website:')) {
              website = trimmed.replace(/^## (X-Website|Website):\s*/, '').trim()
            } else if (trimmed.startsWith('## Category-frFR:') || trimmed.startsWith('## Category:') || trimmed.startsWith('## X-Category:')) {
              category = trimmed.replace(/^## (Category-frFR|Category|X-Category):\s*/, '').trim()
            } else if (trimmed.startsWith('## Interface:')) {
              interfaceVersion = trimmed.replace(/^## Interface:\s*/, '').trim()
            } else if (trimmed.startsWith('## Group:')) {
              group = trimmed.replace(/^## Group:\s*/, '').trim()
            } else if (trimmed.startsWith('## X-Part-Of:') || trimmed.startsWith('## Part-Of:')) {
              xPartOf = trimmed.replace(/^## (X-Part-Of|Part-Of):\s*/, '').trim()
            } else if (trimmed.startsWith('## X-Child-Of:')) {
              xChildOf = trimmed.replace(/^## X-Child-Of:\s*/, '').trim()
            } else if (trimmed.startsWith('## Globe-Main:')) {
              globeMain = trimmed.replace(/^## Globe-Main:\s*/, '').trim()
            } else if (trimmed.startsWith('## Globe-Post:')) {
              const posts = trimmed.replace(/^## Globe-Post:\s*/, '').split(',')
              for (const p of posts) {
                if (p.trim()) globePost.push(p.trim())
              }
            } else if (
              trimmed.startsWith('## Dependencies:') ||
              trimmed.startsWith('## RequiredDeps:') ||
              trimmed.startsWith('## Dependency:')
            ) {
              const deps = trimmed.replace(/^## (Dependencies|RequiredDeps|Dependency):\s*/, '').split(',')
              for (const d of deps) {
                const clean = d.trim().replace(/^!+/, '')
                if (clean) dependencies.push(clean)
              }
            } else if (trimmed.startsWith('## LoadOnDemand:')) {
              loadOnDemand = trimmed.includes('1')
            } else if (trimmed.startsWith('## X-Curse-Project-ID:')) {
              const m = trimmed.match(/(\d+)/)
              if (m) tocCurseId = parseInt(m[1], 10)
            }
          }
        } catch {}
      }

      if (version === '1.0.0' || title.toLowerCase().includes('not supported')) {
        const otherTocs = files.filter((f) => (f.endsWith('.toc') || f.endsWith('.toc.disabled')) && f !== tocCandidate)
        for (const ot of otherTocs) {
          try {
            const otContent = fs.readFileSync(path.join(folderPath, ot), 'utf8')
            const vMatch = otContent.match(/## Version:\s*([^\r\n]+)/)
            const tMatch = otContent.match(/## Title:\s*([^\r\n]+)/)
            if (vMatch && version === '1.0.0') {
              version = vMatch[1].trim()
            }
            if (tMatch && (!title || title === folderName || title.toLowerCase().includes('not supported'))) {
              title = tMatch[1]
                .replace(/^## Title:\s*/, '')
                .replace(/\|c[0-9a-fA-F]{8}/gi, '')
                .replace(/\|r/gi, '')
                .replace(/\|c[0-9a-fA-F]{6}/gi, '')
                .replace(/\|T[^|]+\|t/gi, '')
                .replace(/<[^>]+>/g, '')
                .trim()
            }
          } catch {}
        }
      }

      let meta: any = {}
      const metaFile = path.join(folderPath, '.loom_addon.json')
      if (fs.existsSync(metaFile)) {
        try {
          meta = JSON.parse(fs.readFileSync(metaFile, 'utf8'))
        } catch {}
      }

      if (version === '1.0.0' || title.toLowerCase().includes('not supported')) {
        const otherTocs = files.filter((f) => (f.endsWith('.toc') || f.endsWith('.toc.disabled')) && f !== tocCandidate)
        for (const ot of otherTocs) {
          try {
            const otContent = fs.readFileSync(path.join(folderPath, ot), 'utf8')
            const vMatch = otContent.match(/## Version:\s*([^\r\n]+)/)
            const tMatch = otContent.match(/## Title:\s*([^\r\n]+)/)
            if (vMatch && version === '1.0.0') {
              version = vMatch[1].trim()
            }
            if (tMatch && (!title || title === folderName || title.toLowerCase().includes('not supported'))) {
              title = tMatch[1]
                .replace(/^## Title:\s*/, '')
                .replace(/\|c[0-9a-fA-F]{8}/gi, '')
                .replace(/\|r/gi, '')
                .replace(/\|c[0-9a-fA-F]{6}/gi, '')
                .replace(/\|T[^|]+\|t/gi, '')
                .replace(/<[^>]+>/g, '')
                .trim()
            }
          } catch {}
        }
      }

      let sizeBytes = 0
      let lastModified = 0
      try {
        const stats = fs.statSync(folderPath)
        lastModified = Math.floor(stats.mtimeMs / 1000)
        sizeBytes = getDirectorySizeBytes(folderPath)
      } catch {}

      const cleanKey = folderName.replace(/[^a-zA-Z0-9]/g, '').toLowerCase()
      const inferredId = meta.modId || tocCurseId || KNOWN_ADDON_CURSEFORGE_MAP[cleanKey] || undefined

      rawList.push({
        folderName,
        folderPath,
        title: title || folderName,
        version: version || '1.0.0',
        author: author || undefined,
        notes: notes || undefined,
        website: website || undefined,
        category: category || undefined,
        interfaceVersion: interfaceVersion || undefined,
        group: group || undefined,
        xPartOf: xPartOf || undefined,
        xChildOf: xChildOf || undefined,
        globeMain: globeMain || undefined,
        globePost,
        dependencies,
        loadOnDemand,
        curseforgeId: inferredId,
        fileId: meta.fileId || undefined,
        sizeBytes,
        lastModified,
        enabled: isEnabled,
        meta,
        iconUrl: meta.iconUrl || undefined
      })
    }

    // Helper to determine if an addon is an independent audio / voice / sound / language / translation pack
    function isAudioOrLanguagePack(folderName: string, title?: string): boolean {
      const n = folderName.toLowerCase()
      const t = (title || '').toLowerCase()
      if (
        n.includes('audio') ||
        n.includes('voice') ||
        n.includes('sound') ||
        t.includes('audio') ||
        t.includes('voice') ||
        t.includes('vocal') ||
        n.endsWith('_frfr') ||
        n.endsWith('_dede') ||
        n.endsWith('_eses') ||
        n.endsWith('_esmx') ||
        n.endsWith('_zhcn') ||
        n.endsWith('_zhtw') ||
        n.endsWith('_ruru') ||
        n.endsWith('_kokr') ||
        n.endsWith('_ptbr') ||
        n.endsWith('_itit')
      ) {
        if (n.endsWith('locales') && !n.includes('audio')) {
          return false
        }
        return true
      }
      return false
    }

    // --- MODRINTH / CURSEFORGE GROUPING ENGINE ---
    const folderMap = new Map<string, RawAddonMeta>()
    for (const r of rawList) {
      folderMap.set(r.folderName.toLowerCase(), r)
    }

    const childrenSet = new Set<string>()
    const parentToChildren = new Map<string, string[]>()

    // Special bundle pairings
    const PAIRINGS = [
      { parent: 'bugsack', child: '!buggrabber' },
      { parent: 'questie', child: 'questiedb' },
      { parent: 'tradeskillmaster', child: 'tradeskillmaster_apphelper' }
    ]

    for (const pair of PAIRINGS) {
      if (folderMap.has(pair.parent) && folderMap.has(pair.child)) {
        const pName = folderMap.get(pair.parent)!.folderName
        const cName = folderMap.get(pair.child)!.folderName
        const list = parentToChildren.get(pName) || []
        if (!list.includes(cName)) list.push(cName)
        parentToChildren.set(pName, list)
        childrenSet.add(cName)
      }
    }

    // Pass 0: Globe-Post in parent
    for (const r of rawList) {
      for (const childName of r.globePost) {
        const cand = folderMap.get(childName.toLowerCase())
        if (cand && cand.folderName !== r.folderName) {
          const list = parentToChildren.get(r.folderName) || []
          if (!list.includes(cand.folderName)) list.push(cand.folderName)
          parentToChildren.set(r.folderName, list)
          childrenSet.add(cand.folderName)
        }
      }
    }

    // Pass 1: Explicit Globe-Main / X-Child-Of
    for (const r of rawList) {
      if (childrenSet.has(r.folderName)) continue
      const targetName = r.globeMain || r.xChildOf
      if (targetName) {
        const cand = folderMap.get(targetName.toLowerCase())
        if (cand && cand.folderName !== r.folderName) {
          const list = parentToChildren.get(cand.folderName) || []
          if (!list.includes(r.folderName)) list.push(r.folderName)
          parentToChildren.set(cand.folderName, list)
          childrenSet.add(r.folderName)
        }
      }
    }

    // Pass 2: Explicit Group / X-Part-Of (NEVER for audio/voice/language packs)
    for (const r of rawList) {
      if (childrenSet.has(r.folderName)) continue
      if (isAudioOrLanguagePack(r.folderName, r.title)) continue

      const targetName = r.group || r.xPartOf
      if (targetName) {
        const cand = folderMap.get(targetName.toLowerCase())
        if (cand && cand.folderName !== r.folderName) {
          const sameBase = r.folderName.toLowerCase().startsWith(cand.folderName.toLowerCase())
          const hasDep = r.dependencies.some((d) => d.toLowerCase() === cand.folderName.toLowerCase())
          const sameVer = r.version && cand.version && r.version === cand.version
          if (sameBase || hasDep || sameVer || r.loadOnDemand) {
            const list = parentToChildren.get(cand.folderName) || []
            if (!list.includes(r.folderName)) list.push(r.folderName)
            parentToChildren.set(cand.folderName, list)
            childrenSet.add(r.folderName)
          }
        }
      }
    }

    // Pass 3: Dependencies + LoadOnDemand or strict prefix with '_' or '-'
    for (const r of rawList) {
      if (childrenSet.has(r.folderName)) continue
      if (isAudioOrLanguagePack(r.folderName, r.title)) continue

      for (const d of r.dependencies) {
        const cand = folderMap.get(d.toLowerCase())
        if (cand && cand.folderName !== r.folderName) {
          const candBase = cand.folderName.replace(/[-_]Core$/i, '').toLowerCase()
          const rLower = r.folderName.toLowerCase()
          if (
            r.loadOnDemand ||
            rLower.startsWith(candBase + '_') ||
            rLower.startsWith(candBase + '-')
          ) {
            const list = parentToChildren.get(cand.folderName) || []
            if (!list.includes(r.folderName)) list.push(r.folderName)
            parentToChildren.set(cand.folderName, list)
            childrenSet.add(r.folderName)
            break
          }
        }
      }
    }

    // Pass 4: Strict prefix matching with '_' or '-' ONLY (never bare string matching)
    for (const r of rawList) {
      if (childrenSet.has(r.folderName)) continue
      if (isAudioOrLanguagePack(r.folderName, r.title)) continue

      const rLower = r.folderName.replace(/^!+/, '').toLowerCase()
      const potentialParents = rawList
        .filter((p) => p.folderName !== r.folderName && !childrenSet.has(p.folderName))
        .sort((a, b) => b.folderName.length - a.folderName.length)

      for (const cand of potentialParents) {
        const candClean = cand.folderName.replace(/^!+/, '').toLowerCase()
        const candBase = candClean.replace(/[-_]core$/i, '')

        if (rLower.startsWith(candBase + '_') || rLower.startsWith(candBase + '-')) {
          const sameVer = r.version && cand.version && r.version === cand.version
          if (sameVer || r.loadOnDemand || r.dependencies.some((d) => d.toLowerCase() === candClean)) {
            const list = parentToChildren.get(cand.folderName) || []
            if (!list.includes(r.folderName)) list.push(r.folderName)
            parentToChildren.set(cand.folderName, list)
            childrenSet.add(r.folderName)
            break
          }
        }
      }
    }

    // Final Assembly: Assemble primary addons
    const addons: any[] = []

    for (const r of rawList) {
      if (childrenSet.has(r.folderName)) continue

      const secondaryFolders = parentToChildren.get(r.folderName) || []
      let totalBytes = r.sizeBytes
      let anyChildEnabled = r.enabled
      let bestCurseId = r.curseforgeId
      let bestIconUrl = r.iconUrl

      for (const sec of secondaryFolders) {
        const cMeta = folderMap.get(sec.toLowerCase())
        if (cMeta) {
          totalBytes += cMeta.sizeBytes
          if (cMeta.enabled) anyChildEnabled = true
          if (!bestCurseId && cMeta.curseforgeId) bestCurseId = cMeta.curseforgeId
          if (!bestIconUrl && cMeta.iconUrl) bestIconUrl = cMeta.iconUrl
        }
      }

      const mb = totalBytes / (1024 * 1024)
      const sizeFormatted = mb < 0.1 ? `${Math.round(totalBytes / 1024)} Ko` : `${mb.toFixed(1)} Mo`

      addons.push({
        id: r.folderName.toLowerCase(),
        folderName: r.folderName,
        title: r.title,
        version: r.version,
        author: r.author,
        notes: r.notes,
        website: r.website,
        category: r.category,
        interfaceVersion: r.interfaceVersion,
        sizeBytes: totalBytes,
        sizeFormatted,
        lastModified: r.lastModified,
        enabled: anyChildEnabled,
        curseforgeId: bestCurseId,
        fileId: r.fileId,
        updateAvailable: false,
        latestVersion: r.meta?.latestVersion || undefined,
        downloadUrl: r.meta?.downloadUrl || undefined,
        iconUrl: bestIconUrl,
        secondaryFolders
      })
    }

    // Check updates against CurseForge if requested
    if (checkUpdates) {
      const modIdsToCheck = [...new Set(addons.map((a) => a.curseforgeId).filter(Boolean))] as number[]
      if (modIdsToCheck.length > 0) {
        try {
          const cfUrl = 'https://api.curseforge.com/v1/mods'
          const cfKey = process.env.CURSEFORGE_API_KEY || '$2a$10$bL4bIL5pUWqfcO7KQtnMReakwtfHbNKh6v1uTpKlzhwoueEJQnPnm'
          const cfRes = await fetch(cfUrl, {
            method: 'POST',
            headers: {
              'Content-Type': 'application/json',
              'x-api-key': cfKey
            },
            body: JSON.stringify({ modIds: modIdsToCheck })
          })

          if (cfRes.ok) {
            const cfData: any = await cfRes.json()
            const cfMods = cfData?.data || []

            for (const cfMod of cfMods) {
              const matchedAddons = addons.filter((a) => a.curseforgeId === cfMod.id)
              const icon = cfMod.logo?.thumbnailUrl || cfMod.logo?.url

              // Locate best file for target game version
              let targetFileId: number | undefined
              let targetIndexFileName: string | undefined

              if (targetGameVersionTypeId && Array.isArray(cfMod.latestFilesIndexes)) {
                const matchingIndexes = cfMod.latestFilesIndexes.filter((idx: any) => idx.gameVersionTypeId === targetGameVersionTypeId)
                if (matchingIndexes.length > 0) {
                  const sorted = [...matchingIndexes].sort((a: any, b: any) => {
                    if (a.releaseType === 1 && b.releaseType !== 1) return -1
                    if (b.releaseType === 1 && a.releaseType !== 1) return 1
                    return (b.fileId || 0) - (a.fileId || 0)
                  })
                  targetFileId = sorted[0].fileId
                  targetIndexFileName = sorted[0].filename
                } else if (targetGameVersionTypeId === 88568) {
                  // Fallback for Forever to Classic (67408)
                  const classicIndexes = cfMod.latestFilesIndexes.filter((idx: any) => idx.gameVersionTypeId === 67408)
                  if (classicIndexes.length > 0) {
                    const sorted = [...classicIndexes].sort((a: any, b: any) => {
                      if (a.releaseType === 1 && b.releaseType !== 1) return -1
                      if (b.releaseType === 1 && a.releaseType !== 1) return 1
                      return (b.fileId || 0) - (a.fileId || 0)
                    })
                    targetFileId = sorted[0].fileId
                    targetIndexFileName = sorted[0].filename
                  }
                }
              }

              let bestFile: any = null
              if (targetFileId && Array.isArray(cfMod.latestFiles)) {
                bestFile = cfMod.latestFiles.find((f: any) => f.id === targetFileId)
              }

              if (!bestFile && targetGameVersionTypeId && Array.isArray(cfMod.latestFiles)) {
                const matchingFiles = cfMod.latestFiles.filter((f: any) =>
                  f.sortableGameVersions?.some((sgv: any) => sgv.gameVersionTypeId === targetGameVersionTypeId)
                )
                if (matchingFiles.length > 0) {
                  bestFile = matchingFiles.sort((a: any, b: any) => (b.id || 0) - (a.id || 0))[0]
                } else if (targetGameVersionTypeId === 88568) {
                  const classicFiles = cfMod.latestFiles.filter((f: any) =>
                    f.sortableGameVersions?.some((sgv: any) => sgv.gameVersionTypeId === 67408)
                  )
                  if (classicFiles.length > 0) {
                    bestFile = classicFiles.sort((a: any, b: any) => (b.id || 0) - (a.id || 0))[0]
                  }
                }
              }

              if (!bestFile && Array.isArray(cfMod.latestFiles) && cfMod.latestFiles.length > 0) {
                bestFile = [...cfMod.latestFiles].sort((a: any, b: any) => (b.id || 0) - (a.id || 0))[0]
              }

              for (const a of matchedAddons) {
                if (icon && !a.iconUrl) a.iconUrl = icon

                const remoteDisplayName = bestFile?.displayName || ''
                const remoteFileName = bestFile?.fileName || targetIndexFileName || ''
                const remoteFileId = bestFile?.id || targetFileId
                const remoteDownloadUrl =
                  bestFile?.downloadUrl ||
                  (remoteFileId && remoteFileName
                    ? `https://edge.forgecdn.net/files/${Math.floor(remoteFileId / 1000)}/${remoteFileId % 1000}/${encodeURIComponent(remoteFileName)}`
                    : undefined)

                if (remoteDisplayName || remoteFileName) {
                  a.latestVersion = cleanAddonVersionString(remoteDisplayName || remoteFileName)
                  if (remoteDownloadUrl) a.downloadUrl = remoteDownloadUrl

                  // Update check logic:
                  // 1. If we have local fileId and remote has higher fileId
                  if (a.fileId && remoteFileId && remoteFileId > a.fileId) {
                    a.updateAvailable = true
                  }
                  // 2. Strict semantic comparison: remote must be strictly newer
                  else if (
                    isAddonVersionNewer(remoteDisplayName, a.version) ||
                    isAddonVersionNewer(remoteFileName, a.version)
                  ) {
                    a.updateAvailable = true
                  } else {
                    a.updateAvailable = false
                  }
                }
              }
            }
          }
        } catch (e) {
          console.warn('[WoWAddons] Update check error:', e)
        }
      }
    }

    return addons
  }

  return {
    name: 'loom-wow-addons-server',
    configureServer(server) {
      server.middlewares.use(async (req, res, next) => {
        if (!req.url?.startsWith('/api/wow/')) {
          return next()
        }

        // 1. GET /api/wow/instances
        if (req.method === 'GET' && req.url === '/api/wow/instances') {
          res.setHeader('Content-Type', 'application/json')
          const insts = await scanWowInstances()
          res.end(JSON.stringify(insts))
          return
        }

        // 2. GET /api/wow/addons?instancePath=...
        if (req.method === 'GET' && req.url.startsWith('/api/wow/addons')) {
          res.setHeader('Content-Type', 'application/json')
          const u = new URL(req.url, 'http://localhost:5173')
          let instPath = u.searchParams.get('instancePath') || ''
          let addonsPath = ''

          if (instPath) {
            addonsPath = instPath.endsWith('AddOns') ? instPath : path.join(instPath, 'Interface/AddOns')
          } else {
            const insts = await scanWowInstances()
            const active = insts.find((i) => i.installed) || insts[0]
            addonsPath = active ? active.addonsPath : ''
          }

          if (!addonsPath) {
            res.end(JSON.stringify([]))
            return
          }

          const checkUp = u.searchParams.get('checkUpdates') !== 'false'
          const gameVersionTypeIdParam = u.searchParams.get('gameVersionTypeId')
          const targetGameVersionTypeId = gameVersionTypeIdParam ? parseInt(gameVersionTypeIdParam, 10) : undefined
          const addons = await getInstalledWowAddons(addonsPath, checkUp, targetGameVersionTypeId)
          res.end(JSON.stringify(addons))
          return
        }

        // 3. POST /api/wow/addons/install
        if (req.method === 'POST' && req.url === '/api/wow/addons/install') {
          res.setHeader('Content-Type', 'application/json')
          let body = ''
          req.on('data', (c) => (body += c))
          req.on('end', async () => {
            try {
              const { instancePath, downloadUrl, modId, fileId, modName, version, iconUrl } = JSON.parse(body)
              if (!downloadUrl) {
                res.statusCode = 400
                res.end(JSON.stringify({ error: 'downloadUrl obligatoire' }))
                return
              }

              const targetAddonsPath = instancePath.endsWith('AddOns')
                ? instancePath
                : path.join(instancePath, 'Interface/AddOns')

              if (!fs.existsSync(targetAddonsPath)) {
                fs.mkdirSync(targetAddonsPath, { recursive: true })
              }

              const tmpZip = path.join(os.tmpdir(), `loom_addon_${Date.now()}_${Math.random().toString(36).slice(2)}.zip`)

              // Download zip
              await new Promise<void>((resolve, reject) => {
                exec(`curl -L -s -o "${tmpZip}" "${downloadUrl}"`, (err) => {
                  if (err) reject(err)
                  else resolve()
                })
              })

              // List extracted folders
              let topFolders: string[] = []
              try {
                const zipListing = execSync(`unzip -Z1 "${tmpZip}"`, { encoding: 'utf8' })
                const set = new Set<string>()
                for (const line of zipListing.split('\n')) {
                  const top = line.trim().split('/')[0]
                  if (top) set.add(top)
                }
                topFolders = Array.from(set)
              } catch {}

              // Extract
              execSync(`unzip -o -q "${tmpZip}" -d "${targetAddonsPath}"`)

              try {
                fs.unlinkSync(tmpZip)
              } catch {}

              // Save .loom_addon.json in extracted directories
              const metaObj = {
                modId,
                fileId,
                name: modName,
                version,
                iconUrl,
                installedAt: Math.floor(Date.now() / 1000)
              }

              for (const f of topFolders) {
                const fPath = path.join(targetAddonsPath, f)
                if (fs.existsSync(fPath) && fs.statSync(fPath).isDirectory()) {
                  try {
                    fs.writeFileSync(path.join(fPath, '.loom_addon.json'), JSON.stringify(metaObj, null, 2))
                  } catch {}
                }
              }

              res.end(JSON.stringify({ success: true, folders: topFolders }))
            } catch (err: any) {
              console.error('[WoWAddons] Install error:', err)
              res.statusCode = 500
              res.end(JSON.stringify({ error: err?.message || "Échec de l'installation de l'addon" }))
            }
          })
          return
        }

        // 4. POST /api/wow/addons/uninstall
        if (req.method === 'POST' && req.url === '/api/wow/addons/uninstall') {
          res.setHeader('Content-Type', 'application/json')
          let body = ''
          req.on('data', (c) => (body += c))
          req.on('end', () => {
            try {
              const { instancePath, folderName, secondaryFolders = [] } = JSON.parse(body)
              const targetAddonsPath = instancePath.endsWith('AddOns')
                ? instancePath
                : path.join(instancePath, 'Interface/AddOns')

              const allToRemove = [folderName, ...secondaryFolders]
              for (const f of allToRemove) {
                const p = path.join(targetAddonsPath, f)
                if (fs.existsSync(p)) {
                  fs.rmSync(p, { recursive: true, force: true })
                }
              }

              res.end(JSON.stringify({ success: true }))
            } catch (err: any) {
              res.statusCode = 500
              res.end(JSON.stringify({ error: err?.message || 'Erreur lors de la désinstallation' }))
            }
          })
          return
        }

        // 5. POST /api/wow/addons/toggle
        if (req.method === 'POST' && req.url === '/api/wow/addons/toggle') {
          res.setHeader('Content-Type', 'application/json')
          let body = ''
          req.on('data', (c) => (body += c))
          req.on('end', () => {
            try {
              const { instancePath, folderName, secondaryFolders = [], enabled } = JSON.parse(body)
              const targetAddonsPath = instancePath.endsWith('AddOns')
                ? instancePath
                : path.join(instancePath, 'Interface/AddOns')

              const allToToggle = [folderName, ...secondaryFolders]
              for (const f of allToToggle) {
                const folderPath = path.join(targetAddonsPath, f)
                if (fs.existsSync(folderPath)) {
                  const files = fs.readdirSync(folderPath)
                  if (enabled) {
                    for (const file of files) {
                      if (file.endsWith('.toc.disabled')) {
                        const dest = file.replace(/\.toc\.disabled$/, '.toc')
                        fs.renameSync(path.join(folderPath, file), path.join(folderPath, dest))
                      }
                    }
                  } else {
                    for (const file of files) {
                      if (file.endsWith('.toc')) {
                        fs.renameSync(path.join(folderPath, file), path.join(folderPath, file + '.disabled'))
                      }
                    }
                  }
                }
              }

              res.end(JSON.stringify({ success: true, enabled }))
            } catch (err: any) {
              res.statusCode = 500
              res.end(JSON.stringify({ error: err?.message || 'Erreur modification statut addon' }))
            }
          })
          return
        }

        // 6. POST /api/wow/addons/open-folder
        if (req.method === 'POST' && req.url === '/api/wow/addons/open-folder') {
          res.setHeader('Content-Type', 'application/json')
          let body = ''
          req.on('data', (c) => (body += c))
          req.on('end', () => {
            try {
              const { path: dirPath } = JSON.parse(body)
              if (dirPath && fs.existsSync(dirPath)) {
                exec(`xdg-open "${dirPath}"`)
              }
              res.end(JSON.stringify({ success: true }))
            } catch {
              res.end(JSON.stringify({ success: false }))
            }
          })
          return
        }

        // 7. POST & GET /api/wow/export
        if ((req.method === 'POST' && req.url === '/api/wow/export') || (req.method === 'GET' && req.url?.startsWith('/api/wow/export'))) {
          const handleExport = async (instancePath: string, flavor = 'wow', gameVersionTypeId?: number, gameName?: string) => {
            let tempDir = ''
            try {
              if (!instancePath || !fs.existsSync(instancePath)) {
                res.statusCode = 400
                res.setHeader('Content-Type', 'application/json')
                res.end(JSON.stringify({ error: `Chemin d'instance WoW introuvable : ${instancePath}` }))
                return
              }

              const instanceRoot = instancePath.endsWith('AddOns')
                ? path.dirname(path.dirname(instancePath))
                : instancePath
              const addonsPath = path.join(instanceRoot, 'Interface/AddOns')
              const wtfPath = path.join(instanceRoot, 'WTF')
              const fontsPath = path.join(instanceRoot, 'Fonts')
              const interfacePath = path.join(instanceRoot, 'Interface')
              const flavorInfoPath = path.join(instanceRoot, '.flavor.info')

              // 1. Get installed addons
              const targetGameVersionTypeId = Number(gameVersionTypeId) || undefined
              const installedAddons = await getInstalledWowAddons(addonsPath, false, targetGameVersionTypeId)

              // 2. Read flavor info if exists
              let flavorInfo: string | null = null
              if (fs.existsSync(flavorInfoPath)) {
                try {
                  flavorInfo = fs.readFileSync(flavorInfoPath, 'utf8').trim()
                } catch {}
              }

              // 3. Create temporary staging directory
              tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'loom-wow-backup-'))

              // 4. Copy WTF folder if exists
              let hasWtf = false
              if (fs.existsSync(wtfPath) && fs.statSync(wtfPath).isDirectory()) {
                const destWtf = path.join(tempDir, 'WTF')
                fs.cpSync(wtfPath, destWtf, { recursive: true })
                hasWtf = true
              }

              // 5. Copy Fonts folder (preserving custom TTF / OTF / fonts, ignoring engine runtime *.slug caches)
              let hasFonts = false
              if (fs.existsSync(fontsPath) && fs.statSync(fontsPath).isDirectory()) {
                const destFonts = path.join(tempDir, 'Fonts')
                fs.mkdirSync(destFonts, { recursive: true })
                const fontFiles = fs.readdirSync(fontsPath)
                for (const f of fontFiles) {
                  if (f.endsWith('.slug') || f.endsWith('.slugo')) continue
                  const srcFile = path.join(fontsPath, f)
                  if (fs.statSync(srcFile).isFile()) {
                    fs.copyFileSync(srcFile, path.join(destFonts, f))
                    hasFonts = true
                  }
                }
                if (!hasFonts) {
                  try { fs.rmdirSync(destFonts) } catch {}
                }
              }

              // 6. Copy custom addons (addons without CurseForge ID)
              const customAddons: any[] = []
              const customAddonsDir = path.join(tempDir, 'custom_addons')
              for (const addon of installedAddons) {
                if (!addon.curseforgeId) {
                  if (!fs.existsSync(customAddonsDir)) {
                    fs.mkdirSync(customAddonsDir, { recursive: true })
                  }
                  const allFolders = [addon.folderName, ...(addon.secondaryFolders || [])]
                  for (const f of allFolders) {
                    const srcFolder = path.join(addonsPath, f)
                    if (fs.existsSync(srcFolder) && fs.statSync(srcFolder).isDirectory()) {
                      fs.cpSync(srcFolder, path.join(customAddonsDir, f), { recursive: true })
                    }
                  }
                  customAddons.push({
                    title: addon.title,
                    folderName: addon.folderName,
                    version: addon.version,
                    author: addon.author,
                    secondaryFolders: addon.secondaryFolders || []
                  })
                }
              }

              // 7. Copy Interface extras (anything in Interface/ that is not AddOns/)
              let hasInterfaceExtras = false
              if (fs.existsSync(interfacePath) && fs.statSync(interfacePath).isDirectory()) {
                const interfaceItems = fs.readdirSync(interfacePath)
                const extrasDir = path.join(tempDir, 'interface_extras')
                for (const item of interfaceItems) {
                  if (item.toLowerCase() === 'addons') continue
                  if (!fs.existsSync(extrasDir)) fs.mkdirSync(extrasDir, { recursive: true })
                  const srcItem = path.join(interfacePath, item)
                  fs.cpSync(srcItem, path.join(extrasDir, item), { recursive: true })
                  hasInterfaceExtras = true
                }
              }

              // 8. Build loom-manifest.json
              const manifest = {
                format: 'loom-wow-backup',
                version: 1,
                exportedAt: new Date().toISOString(),
                exportedBy: 'Loom Game Hub',
                game: {
                  flavor,
                  name: gameName || 'World of Warcraft',
                  gameVersionTypeId: targetGameVersionTypeId || null,
                  flavorInfo
                },
                summary: {
                  totalAddons: installedAddons.length,
                  curseforgeAddonsCount: installedAddons.filter((a) => a.curseforgeId).length,
                  customAddonsCount: customAddons.length,
                  hasWtfBackup: hasWtf,
                  hasFontsBackup: hasFonts,
                  hasInterfaceExtras
                },
                addons: installedAddons.map((a) => ({
                  title: a.title,
                  folderName: a.folderName,
                  version: a.version,
                  author: a.author,
                  curseforgeId: a.curseforgeId || null,
                  fileId: a.fileId || null,
                  downloadUrl: a.downloadUrl || null,
                  websiteUrl: a.websiteUrl || null,
                  enabled: a.enabled,
                  secondaryFolders: a.secondaryFolders || [],
                  isCustom: !a.curseforgeId
                })),
                customAddons
              }

              fs.writeFileSync(path.join(tempDir, 'loom-manifest.json'), JSON.stringify(manifest, null, 2), 'utf8')

              // 9. Compress into ZIP archive
              const zipFileName = `loom-wow-backup-${flavor}-${new Date().toISOString().slice(0, 10)}.zip`
              const zipFilePath = path.join(os.tmpdir(), `loom_export_${Date.now()}.zip`)

              execSync(`zip -r -q "${zipFilePath}" .`, { cwd: tempDir })

              // 10. Send file
              const zipBuffer = fs.readFileSync(zipFilePath)
              res.setHeader('Content-Type', 'application/zip')
              res.setHeader('Content-Disposition', `attachment; filename="${zipFileName}"`)
              res.setHeader('Content-Length', zipBuffer.length)
              res.end(zipBuffer)

              try { fs.unlinkSync(zipFilePath) } catch {}
            } catch (err: any) {
              console.error('[WoWExport] Export error:', err)
              res.statusCode = 500
              res.setHeader('Content-Type', 'application/json')
              res.end(JSON.stringify({ error: err?.message || "Échec de l'exportation du profil WoW" }))
            } finally {
              if (tempDir && fs.existsSync(tempDir)) {
                try { fs.rmSync(tempDir, { recursive: true, force: true }) } catch {}
              }
            }
          }

          if (req.method === 'GET') {
            const u = new URL(req.url, 'http://localhost:5173')
            const instPath = u.searchParams.get('instancePath') || ''
            const flavor = u.searchParams.get('flavor') || 'wow'
            const gvId = u.searchParams.get('gameVersionTypeId') ? Number(u.searchParams.get('gameVersionTypeId')) : undefined
            const gName = u.searchParams.get('gameName') || undefined
            await handleExport(instPath, flavor, gvId, gName)
            return
          } else {
            let body = ''
            req.on('data', (c) => (body += c))
            req.on('end', async () => {
              try {
                const { instancePath, flavor, gameVersionTypeId, gameName } = JSON.parse(body)
                await handleExport(instancePath, flavor, gameVersionTypeId, gameName)
              } catch (e: any) {
                res.statusCode = 400
                res.setHeader('Content-Type', 'application/json')
                res.end(JSON.stringify({ error: e?.message || 'Données de requête invalides' }))
              }
            })
            return
          }
        }

        next()
      })
    }
  }
}

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [vue(), steamProcessPlugin(), microsoftAuthPlugin(), minecraftServerPlugin(), battlenetPlugin(), wowAddonsPlugin()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src')
    }
  },
  server: {
    host: '127.0.0.1',
    port: 5173,
    strictPort: true,
    proxy: {
      '/igdb-api': {
        target: 'https://api.igdb.com/v4',
        changeOrigin: true,
        rewrite: (path) => path.replace(/^\/igdb-api/, '')
      },
      '/twitch-oauth': {
        target: 'https://id.twitch.tv/oauth2',
        changeOrigin: true,
        rewrite: (path) => path.replace(/^\/twitch-oauth/, '')
      },
      '/modrinth-api': {
        target: 'https://api.modrinth.com/v2',
        changeOrigin: true,
        rewrite: (path) => path.replace(/^\/modrinth-api/, '')
      },
      '/curseforge-api': {
        target: 'https://api.curseforge.com/v1',
        changeOrigin: true,
        headers: {
          'x-api-key': process.env.CURSEFORGE_API_KEY || '$2a$10$bL4bIL5pUWqfcO7KQtnMReakwtfHbNKh6v1uTpKlzhwoueEJQnPnm'
        },
        rewrite: (path) => path.replace(/^\/curseforge-api/, '')
      }
    }
  },
  clearScreen: false
})
