import { defineConfig, type Plugin } from 'vite'
import vue from '@vitejs/plugin-vue'
import path from 'path'
import fs from 'fs'
import os from 'os'
import https from 'https'
import { exec } from 'child_process'

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
                console.info(`[Loom::Steam] Demande d'installation réelle Steam AppID ${appid}...`)
                exec(`xdg-open steam://install/${appid}`)
                res.end(JSON.stringify({ success: true, message: `Installation transmise à Steam pour AppID ${appid}` }))
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
