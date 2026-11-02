import type { Game } from '@/models/game'

export type PlatformId = 'steam' | 'epic' | 'gog' | 'xbox' | 'battlenet' | 'minecraft' | 'manual'

export interface Achievement {
  id: string
  name: string
  description: string
  unlocked: boolean
  unlockedAt?: string
  unlockTimestamp?: number
  icon: string
  iconLocked?: string
  rarityPercentage: number
  hidden?: boolean
}

export interface GameAchievementSummary {
  gameId: string
  platform: PlatformId
  unlockedCount: number
  totalCount: number
  percentage: number
  allUnlocked: boolean
}

/**
 * Interface contract for platform achievement providers.
 * Any platform (Steam, Epic, GOG, Xbox, Battle.net) implements this contract.
 */
export interface PlatformAchievementProvider {
  readonly platform: PlatformId

  /**
   * Fast summary retrieval for all owned games on this platform (unlocked count, total count).
   */
  getSummaries(): Promise<Map<string, GameAchievementSummary>>

  /**
   * Fast summary for a single game.
   */
  getGameSummary(game: Game): Promise<GameAchievementSummary | null>

  /**
   * Full list of achievements (with names, descriptions, unlock dates, icons, rarity) for a game.
   */
  getGameAchievements(game: Game): Promise<Achievement[]>
}

/**
 * High-level platform service contract for integrating future platform features
 * (Library discovery, authentication, game launching, achievements, cloud saves).
 */
export interface PlatformService {
  readonly platform: PlatformId
  readonly name: string
  readonly isConnected: boolean
  readonly achievementProvider?: PlatformAchievementProvider
}
