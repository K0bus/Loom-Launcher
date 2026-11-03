export interface IpcResponse<T = any> {
  success: boolean
  data?: T
  error?: string
}

class IpcBridge {
  private _isTauri: boolean | null = null

  /**
   * Detects whether running inside a Tauri v2 runtime.
   */
  public isTauri(): boolean {
    if (this._isTauri !== null) return this._isTauri
    this._isTauri = typeof window !== 'undefined' && Boolean(window.__TAURI_INTERNALS__)
    return this._isTauri
  }

  /**
   * Generic invoke method that routes to Tauri v2 core invoke if available,
   * or falls back to simulated mock handlers.
   */
  public async invoke<T = any>(command: string, args: Record<string, any> = {}): Promise<T> {
    const viteApiCommands = ['get_steam_downloads', 'get_steam_libraries', 'install_game']

    if (this.isTauri() && !viteApiCommands.includes(command)) {
      try {
        const { invoke } = await import('@tauri-apps/api/core')
        return await invoke<T>(command, args)
      } catch (err: any) {
        console.warn(`[IPC::Tauri] Error executing command '${command}', using fallback:`, err)
        return this.handleMockCommand<T>(command, args)
      }
    }

    // Fallback or Vite dev server API execution
    return this.handleMockCommand<T>(command, args)
  }

  /**
   * Simulated mock dispatcher for web-mode development and preview
   */
  private async handleMockCommand<T>(command: string, args: Record<string, any>): Promise<T> {
    // Simulate brief asynchronous bridge delay (150ms)
    await new Promise((resolve) => setTimeout(resolve, 150))

    switch (command) {
      case 'launch_game': {
        const gameId = String(args.gameId || '')
        const appid = gameId.replace(/^steam-/, '')
        try {
          const resp = await fetch('/api/steam/launch', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ appid, gameId })
          })
          const data = await resp.json()
          return data as T
        } catch (e) {
          if (typeof window !== 'undefined' && appid) {
            window.location.href = `steam://rungameid/${appid}`
          }
          return { success: true } as T
        }
      }

      case 'install_game': {
        const gameId = String(args.gameId || args.appid || '')
        const appid = gameId.replace(/^steam-/, '')
        const folderIndex = args.folderIndex !== undefined ? args.folderIndex : 0
        try {
          const resp = await fetch('/api/steam/install', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ appid, folderIndex })
          })
          const data = await resp.json()
          return data as T
        } catch {
          return { success: true, appid, folderIndex } as T
        }
      }

      case 'get_steam_libraries': {
        try {
          const resp = await fetch('/api/steam/libraries')
          const data = await resp.json()
          return (data.libraries || []) as T
        } catch {
          return [] as T
        }
      }

      case 'get_steam_downloads': {
        try {
          const resp = await fetch('/api/steam/downloads')
          const data = await resp.json()
          return data as T
        } catch {
          return { success: true, active: [], completed: [], totalRateMbps: 0 } as T
        }
      }

      case 'stop_game': {
        const gameId = String(args.gameId || '')
        const appid = gameId.replace(/^steam-/, '')
        try {
          const resp = await fetch('/api/steam/stop', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ appid, gameId })
          })
          const data = await resp.json()
          return data as T
        } catch {
          return { success: true } as T
        }
      }

      case 'get_running_games': {
        try {
          const resp = await fetch('/api/steam/status')
          const data = await resp.json()
          return data as T
        } catch {
          return { success: true, runningAppId: null, pids: [] } as T
        }
      }

      case 'get_system_stats':
        return {
          cpuUsage: 18.4 + Math.sin(Date.now() / 1000) * 8,
          ramUsedGb: 14.2,
          ramTotalGb: 32.0,
          networkDownKbps: 4820,
          networkUpKbps: 340,
          disks: [
            {
              mountPoint: '/mnt/games-nvme',
              label: 'NVMe Jeux (Samsung 990 Pro)',
              freeGb: 482,
              totalGb: 2000,
              usedPercentage: 76,
              isGamePartition: true
            },
            {
              mountPoint: '/',
              label: 'Système Linux (Arch/Ext4)',
              freeGb: 124,
              totalGb: 500,
              usedPercentage: 75,
              isGamePartition: false
            }
          ],
          weeklyPlaytimeHours: 24.5,
          activeDownloadsCount: 0,
          notificationsCount: 2,
          appVersion: 'v0.1.0-alpha (Tauri v2 Engine)',
          tauriDetected: false
        } as T

      case 'install_modpack':
        return {
          success: true,
          instanceId: `inst-${Date.now()}`,
          message: `Modpack installé avec succès!`
        } as T

      case 'initiate_microsoft_device_code':
        return {
          userCode: 'B7X4-K9M2',
          verificationUri: 'https://microsoft.com/link',
          expiresIn: 900,
          message: 'Veuillez saisir ce code sur microsoft.com/link pour autoriser votre compte Minecraft/Xbox.'
        } as T

      case 'poll_microsoft_device_code':
        return {
          authenticated: true,
          gamertag: 'K0bus_Gamer',
          uuid: 'e713d81b-5682-411a-8c76-5fa45371b2d0'
        } as T

      default:
        console.warn(`[IPC::Mock] Unhandled command '${command}', returning empty success payload.`)
        return { success: true } as T
    }
  }
}

export const ipc = new IpcBridge()
