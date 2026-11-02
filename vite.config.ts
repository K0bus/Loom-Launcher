import { defineConfig, type Plugin } from 'vite'
import vue from '@vitejs/plugin-vue'
import path from 'path'
import fs from 'fs'
import os from 'os'
import { exec } from 'child_process'

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
