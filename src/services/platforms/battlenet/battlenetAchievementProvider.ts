import type { Game } from '@/models/game'
import type {
  PlatformAchievementProvider,
  PlatformId,
  Achievement,
  GameAchievementSummary
} from '@/services/platforms/types'

/**
 * Stub implementation for Battle.net achievements.
 * Will be connected to Blizzard Battle.net Community API when Battle.net account linking is active.
 */
export class BattleNetAchievementProvider implements PlatformAchievementProvider {
  public readonly platform: PlatformId = 'battlenet'

  public async getSummaries(): Promise<Map<string, GameAchievementSummary>> {
    return new Map()
  }

  public async getGameSummary(_game: Game): Promise<GameAchievementSummary | null> {
    return null
  }

  public async getGameAchievements(_game: Game): Promise<Achievement[]> {
    return []
  }
}

export const battleNetAchievementProvider = new BattleNetAchievementProvider()
