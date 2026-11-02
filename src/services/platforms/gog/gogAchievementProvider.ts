import type { Game } from '@/models/game'
import type {
  PlatformAchievementProvider,
  PlatformId,
  Achievement,
  GameAchievementSummary
} from '@/services/platforms/types'

/**
 * Stub implementation for GOG Galaxy achievements.
 * Will be connected to GOG Galaxy API / comet API when GOG account linking is active.
 */
export class GogAchievementProvider implements PlatformAchievementProvider {
  public readonly platform: PlatformId = 'gog'

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

export const gogAchievementProvider = new GogAchievementProvider()
