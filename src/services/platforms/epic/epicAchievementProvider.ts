import type { Game } from '@/models/game'
import type {
  PlatformAchievementProvider,
  PlatformId,
  Achievement,
  GameAchievementSummary
} from '@/services/platforms/types'

/**
 * Stub implementation for Epic Games Store achievements.
 * Will be wired to Legendary CLI / Epic EOS Web API when Epic account linking is active.
 */
export class EpicAchievementProvider implements PlatformAchievementProvider {
  public readonly platform: PlatformId = 'epic'

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

export const epicAchievementProvider = new EpicAchievementProvider()
