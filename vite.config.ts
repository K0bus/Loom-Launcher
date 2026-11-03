import { defineConfig, type Plugin } from 'vite'
import vue from '@vitejs/plugin-vue'
import path from 'path'
import fs from 'fs'
import os from 'os'
import https from 'https'
import { exec, execSync } from 'child_process'

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
  const logPath = path.join(os.homedir(), '.local/share/Steam/logs/gameprocess_log.txt')
  if (!fs.existsSync(logPath)) return { runningAppId: null, pids: [] }

  try {
    const content = fs.readFileSync(logPath, 'utf8')
    const lines = content.split('\n').slice(-300)
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

    // Secondary safety check: inspect /proc/*/environ for SteamAppId
    const procEntries = fs.readdirSync('/proc')
    for (const entry of procEntries) {
      if (!/^\d+$/.test(entry)) continue
      try {
        const env = fs.readFileSync(`/proc/${entry}/environ`)
        const idx = env.indexOf('SteamAppId=')
        if (idx !== -1) {
          const end = env.indexOf(0, idx)
          const val = env.subarray(idx + 11, end !== -1 ? end : undefined).toString('utf8')
          if (/^\d+$/.test(val)) {
            return { runningAppId: val, pids: [parseInt(entry, 10)] }
          }
        }
      } catch {
        // ignore unreadable processes
      }
    }
  } catch (err) {
    console.warn('[Vite::SteamState] Error reading steam process logs:', err)
  }

  return { runningAppId: null, pids: [] }
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

// 2. Real-time network speed from Steam process I/O (/proc/<steam_pid>/io)
let steamPidCache: { pid: number | null; checkedAt: number } = { pid: null, checkedAt: 0 }
let steamIoTracker: { pid: number; rchar: number; time: number; speedMbps: number } | null = null

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

function getSteamLiveNetworkRateMbps(): number {
  const pid = getSteamProcessPid()
  if (!pid) return 0
  try {
    const ioContent = fs.readFileSync(`/proc/${pid}/io`, 'utf8')
    const rcharMatch = ioContent.match(/rchar:\s+(\d+)/)
    if (!rcharMatch) return 0
    const rchar = parseInt(rcharMatch[1], 10)
    const now = Date.now()

    if (!steamIoTracker || steamIoTracker.pid !== pid) {
      steamIoTracker = { pid, rchar, time: now, speedMbps: 0 }
      return 0
    }

    const dtSec = (now - steamIoTracker.time) / 1000
    if (dtSec >= 0.4) {
      const deltaBytes = Math.max(0, rchar - steamIoTracker.rchar)
      const rawMbps = ((deltaBytes * 8) / (1024 * 1024)) / dtSec
      const smoothMbps = steamIoTracker.speedMbps > 0
        ? (steamIoTracker.speedMbps * 0.4) + (rawMbps * 0.6)
        : rawMbps
      const finalMbps = parseFloat(smoothMbps.toFixed(1))
      steamIoTracker = { pid, rchar, time: now, speedMbps: finalMbps }
      return finalMbps
    }

    return steamIoTracker.speedMbps
  } catch {
    return 0
  }
}

function getSteamDownloads() {
  const libraries = getSteamLibraries()
  const active: any[] = []
  const completed: any[] = []

  // Check content_log.txt for baseline fallback
  let latestRate = 0
  const contentLogPath = path.join(os.homedir(), '.local/share/Steam/logs/content_log.txt')
  if (fs.existsSync(contentLogPath)) {
    try {
      const logText = fs.readFileSync(contentLogPath, 'utf8')
      const matches = [...logText.matchAll(/Current download rate:\s+([\d\.]+)\s+Mbps/g)]
      if (matches.length > 0) {
        latestRate = parseFloat(matches[matches.length - 1][1])
      }
    } catch {}
  }

  // Real-time network speed from Steam kernel I/O
  const liveNetworkMbps = getSteamLiveNetworkRateMbps()
  const effectiveTotalRate = liveNetworkMbps > 0 ? liveNetworkMbps : latestRate

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
            downloadRateMbps: effectiveTotalRate,
            coverUrl: ['4126040', '4704690', '480', '2315050'].includes(appid)
              ? `/steam-assets/${appid}/library_capsule.jpg`
              : `https://shared.steamstatic.com/store_item_assets/steam/apps/${appid}/library_600x900.jpg`
          })
        }
      } catch {}
    }

    // 2. Check recently updated appmanifests (installed in the last 7 days)
    try {
      const manifests = fs.readdirSync(steamappsDir).filter((f) => f.startsWith('appmanifest_') && f.endsWith('.acf'))
      const now = Math.floor(Date.now() / 1000)

      for (const mFile of manifests) {
        const mPath = path.join(steamappsDir, mFile)
        try {
          const content = fs.readFileSync(mPath, 'utf8')
          const appid = mFile.match(/appmanifest_(\d+)\.acf/)?.[1]
          if (!appid || seenAppids.has(appid) || IGNORED_STEAM_APPIDS.has(appid)) continue

          const lastUpdated = parseInt(content.match(/"LastUpdated"\s+"(\d+)"/)?.[1] || '0', 10)
          const stateFlags = content.match(/"StateFlags"\s+"(\d+)"/)?.[1]
          const name = content.match(/"name"\s+"([^"]+)"/)?.[1] || `App ${appid}`
          if (name.includes('Steamworks Common Redistributables') || name.startsWith('Steam Linux Runtime')) {
            continue
          }

          const bytesTotal = parseInt(content.match(/"SizeOnDisk"\s+"(\d+)"/)?.[1] || '0', 10)

          if (stateFlags === '4' && now - lastUpdated < 604800 && lastUpdated > 0) {
            seenAppids.add(appid)
            // Clean up memory cache for completed apps
            lastKnownBytesDownloaded.delete(appid)

            completed.push({
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

  // Clean up any stale entries in lastKnownBytesDownloaded
  for (const cachedAppid of lastKnownBytesDownloaded.keys()) {
    if (!seenAppids.has(cachedAppid)) {
      lastKnownBytesDownloaded.delete(cachedAppid)
    }
  }

  completed.sort((a, b) => b.completedAt - a.completedAt)
  return {
    active,
    completed: completed.slice(0, 10),
    totalRateMbps: active.length > 0 ? effectiveTotalRate : 0
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
          const filePath = path.join(
            os.homedir(),
            '.local/share/Steam/userdata/125342250/config/librarycache/achievement_progress.json'
          )
          if (fs.existsSync(filePath)) {
            try {
              const data = JSON.parse(fs.readFileSync(filePath, 'utf8'))
              const map: Record<string, any> = {}
              if (Array.isArray(data.mapCache)) {
                for (const [aid, info] of data.mapCache) {
                  map[String(aid)] = info
                }
              }
              res.end(JSON.stringify({ success: true, summaries: map }))
              return
            } catch (e: any) {
              res.statusCode = 500
              res.end(JSON.stringify({ success: false, error: e.message }))
              return
            }
          }
          res.end(JSON.stringify({ success: true, summaries: {} }))
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
export default defineConfig({
  plugins: [vue(), steamProcessPlugin()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src')
    }
  },
  server: {
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
      }
    }
  },
  clearScreen: false
})
