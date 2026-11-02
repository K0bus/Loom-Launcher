import type { Game } from '@/models/game'
import type {
  PlatformAchievementProvider,
  PlatformId,
  Achievement,
  GameAchievementSummary
} from '@/services/platforms/types'

export class SteamAchievementProvider implements PlatformAchievementProvider {
  public readonly platform: PlatformId = 'steam'

  private memoryCache = new Map<string, { timestamp: number; achievements: Achievement[] }>()
  private summariesCache: Map<string, GameAchievementSummary> | null = null
  private readonly CACHE_TTL_MS = 5 * 60 * 1000 // 5 minutes cache

  /**
   * Fast retrieval of all achievement summaries from Steam local cache
   */
  public async getSummaries(): Promise<Map<string, GameAchievementSummary>> {
    if (this.summariesCache && this.summariesCache.size > 0) {
      return this.summariesCache
    }

    const resultMap = new Map<string, GameAchievementSummary>()

    try {
      const resp = await fetch('/api/steam/achievements/summaries')
      if (resp.ok) {
        const data = await resp.json()
        if (data.success && data.summaries) {
          for (const [appid, info] of Object.entries<any>(data.summaries)) {
            const summary: GameAchievementSummary = {
              gameId: `steam-${appid}`,
              platform: 'steam',
              unlockedCount: info.unlocked || 0,
              totalCount: info.total || 0,
              percentage: Math.round(info.percentage || 0),
              allUnlocked: Boolean(info.all_unlocked)
            }
            resultMap.set(appid, summary)
            resultMap.set(`steam-${appid}`, summary)
          }
        }
      }
    } catch (err) {
      console.warn('[SteamAchievementProvider] Failed to fetch summaries:', err)
    }

    this.summariesCache = resultMap
    return resultMap
  }

  /**
   * Retrieve summary for a single game
   */
  public async getGameSummary(game: Game): Promise<GameAchievementSummary | null> {
    const appid = this.extractAppId(game)
    if (!appid) return null

    const summaries = await this.getSummaries()
    return summaries.get(appid) || summaries.get(game.id) || null
  }

  /**
   * Full detailed achievements list for a Steam game
   */
  public async getGameAchievements(game: Game): Promise<Achievement[]> {
    const appid = this.extractAppId(game)
    if (!appid) return []

    // 1. Check in-memory cache
    const cached = this.memoryCache.get(appid)
    if (cached && Date.now() - cached.timestamp < this.CACHE_TTL_MS) {
      return cached.achievements
    }

    // 2. Check localStorage cache
    const storageKey = `loom_steam_achievements_${appid}`
    try {
      const persisted = localStorage.getItem(storageKey)
      if (persisted) {
        const parsed = JSON.parse(persisted)
        if (parsed.timestamp && Date.now() - parsed.timestamp < this.CACHE_TTL_MS && Array.isArray(parsed.achievements)) {
          this.memoryCache.set(appid, { timestamp: parsed.timestamp, achievements: parsed.achievements })
          return parsed.achievements
        }
      }
    } catch {}

    // 3. Fetch from API
    try {
      const resp = await fetch(`/api/steam/achievements?appid=${appid}`)
      if (!resp.ok) {
        throw new Error(`HTTP error ${resp.status}`)
      }

      const data = await resp.json()
      if (data.success && Array.isArray(data.achievements)) {
        const achievements: Achievement[] = data.achievements

        // Cache in memory and localStorage
        const cacheEntry = { timestamp: Date.now(), achievements }
        this.memoryCache.set(appid, cacheEntry)
        try {
          localStorage.setItem(storageKey, JSON.stringify(cacheEntry))
        } catch {}

        return achievements
      }
    } catch (err) {
      console.warn(`[SteamAchievementProvider] Failed to fetch achievements for appid ${appid}:`, err)
    }

    return []
  }

  /**
   * Extract raw numeric appid from Game model or ID
   */
  public extractAppId(game: Game): string | null {
    if (game.id.startsWith('steam-')) {
      return game.id.replace('steam-', '')
    }
    // Also check description or config if available
    const match = game.description?.match(/AppID Steam: (\d+)/)
    if (match) return match[1]

    if (/^\d+$/.test(game.id)) {
      return game.id
    }

    return null
  }
}

export const steamAchievementProvider = new SteamAchievementProvider()
