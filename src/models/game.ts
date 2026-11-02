export type RunnerType = 'native' | 'proton-ge' | 'proton-experimental' | 'wine-staging' | 'lutris' | 'custom'

export interface Achievement {
  id: string
  name: string
  description: string
  unlocked: boolean
  unlockedAt?: string
  icon: string
  rarityPercentage: number
}

export interface GameNews {
  id: string
  title: string
  summary: string
  date: string
  author: string
  url?: string
  tag?: string
  hearts?: number
  comments?: number
  type?: 'news' | 'patches'
}

export interface GameConfig {
  runner: RunnerType
  customRunnerPath?: string
  winePrefix?: string
  gamemode: boolean
  mangohud: boolean
  gamescope: boolean
  gamescopeResolution?: string
  envVariables: Record<string, string>
  launchArguments: string
}

export interface Game {
  id: string
  title: string
  slug: string
  bannerUrl: string // 21:9 format
  coverUrl: string  // 3:4 poster format
  iconUrl?: string
  platform: 'steam' | 'epic' | 'gog' | 'minecraft' | 'manual'
  genre: string[]
  playtimeMinutes: number
  lastPlayed?: string
  installed: boolean
  installPath?: string
  sizeGb: number
  achievements: Achievement[]
  news: GameNews[]
  config: GameConfig
  developer: string
  publisher: string
  releaseYear: number
  description: string
  screenshots: string[]
  isFavorite?: boolean
  subtitle?: string
  totalAchievementsCount?: number
  cloudSynced?: boolean
  igdbId?: number
  rating?: number
  ratingCount?: number
  storyline?: string
  themes?: string[]
  gameModes?: string[]
  websites?: Array<{ category: string; url: string }>
  videos?: Array<{ name: string; videoId: string }>
  inLibrary?: boolean
  storeLinks?: Array<{ store: string; url: string; price?: string; icon?: string }>
}

export interface GameFilter {
  search: string
  platform?: string
  genre?: string
  runner?: string
  status?: 'all' | 'installed' | 'uninstalled'
  sortBy: 'lastPlayed' | 'playtime' | 'title' | 'size'
  sortOrder: 'asc' | 'desc'
}
