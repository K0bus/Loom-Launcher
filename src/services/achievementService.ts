import type { Game } from '@/models/game'
import type {
  Achievement,
  GameAchievementSummary,
  PlatformAchievementProvider,
  PlatformId
} from '@/services/platforms/types'
import { steamAchievementProvider } from '@/services/platforms/steam/steamAchievementProvider'
import { epicAchievementProvider } from '@/services/platforms/epic/epicAchievementProvider'
import { gogAchievementProvider } from '@/services/platforms/gog/gogAchievementProvider'
import { xboxAchievementProvider } from '@/services/platforms/xbox/xboxAchievementProvider'
import { battleNetAchievementProvider } from '@/services/platforms/battlenet/battlenetAchievementProvider'

export class AchievementService {
  private providers = new Map<PlatformId, PlatformAchievementProvider>()
  private summariesCache = new Map<string, GameAchievementSummary>()
  private isInitialized = false

  constructor() {
    // Register built-in platform providers
    this.registerProvider(steamAchievementProvider)
    this.registerProvider(epicAchievementProvider)
    this.registerProvider(gogAchievementProvider)
    this.registerProvider(xboxAchievementProvider)
    this.registerProvider(battleNetAchievementProvider)
  }

  /**
   * Register a new or replacement achievement provider for a platform
   */
  public registerProvider(provider: PlatformAchievementProvider): void {
    this.providers.set(provider.platform, provider)
  }

  /**
   * Get the registered provider for a platform
   */
  public getProvider(platform: PlatformId): PlatformAchievementProvider | undefined {
    return this.providers.get(platform)
  }

  /**
   * Retrieve all summaries across all active providers
   */
  public async loadAllSummaries(): Promise<Map<string, GameAchievementSummary>> {
    const combined = new Map<string, GameAchievementSummary>()

    const promises = Array.from(this.providers.values()).map(async (provider) => {
      try {
        const summaries = await provider.getSummaries()
        for (const [key, val] of summaries.entries()) {
          combined.set(key, val)
          this.summariesCache.set(key, val)
        }
      } catch (err) {
        console.warn(`[AchievementService] Failed to load summaries for platform ${provider.platform}:`, err)
      }
    })

    await Promise.all(promises)
    this.isInitialized = true
    return combined
  }

  /**
   * Check if summaries have been initialized
   */
  public isReady(): boolean {
    return this.isInitialized
  }

  /**
   * Get cached summary if already loaded, or fetch it
   */
  public getCachedSummary(gameIdOrAppId: string): GameAchievementSummary | undefined {
    return this.summariesCache.get(gameIdOrAppId)
  }

  /**
   * Retrieve summary for a specific game
   */
  public async getSummaryForGame(game: Game): Promise<GameAchievementSummary | null> {
    const cached = this.summariesCache.get(game.id)
    if (cached) return cached

    const provider = this.getProvider(game.platform as PlatformId)
    if (!provider) return null

    const summary = await provider.getGameSummary(game)
    if (summary) {
      this.summariesCache.set(game.id, summary)
      this.summariesCache.set(summary.gameId, summary)
    }
    return summary
  }

  /**
   * Retrieve complete list of achievements for a game
   */
  public async getAchievementsForGame(game: Game): Promise<Achievement[]> {
    const provider = this.getProvider(game.platform as PlatformId)
    if (!provider) {
      return game.achievements || []
    }

    try {
      const achievements = await provider.getGameAchievements(game)
      if (achievements && achievements.length > 0) {
        // Update local summary cache
        const unlockedCount = achievements.filter((a) => a.unlocked).length
        const totalCount = achievements.length
        const percentage = totalCount > 0 ? Math.round((unlockedCount / totalCount) * 100) : 0

        const summary: GameAchievementSummary = {
          gameId: game.id,
          platform: game.platform as PlatformId,
          unlockedCount,
          totalCount,
          percentage,
          allUnlocked: unlockedCount === totalCount && totalCount > 0
        }
        this.summariesCache.set(game.id, summary)
      }
      return achievements
    } catch (err) {
      console.warn(`[AchievementService] Failed to get achievements for game ${game.id}:`, err)
      return game.achievements || []
    }
  }
}

export const achievementService = new AchievementService()
