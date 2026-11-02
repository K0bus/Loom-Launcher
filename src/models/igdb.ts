import type { Game } from './game'

export interface IGDBCover {
  id: number
  image_id: string
  url?: string
  width?: number
  height?: number
}

export interface IGDBArtwork {
  id: number
  image_id: string
  url?: string
  width?: number
  height?: number
}

export interface IGDBScreenshot {
  id: number
  image_id: string
  url?: string
  width?: number
  height?: number
}

export interface IGDBGenre {
  id: number
  name: string
  slug: string
}

export interface IGDBPlatform {
  id: number
  name: string
  abbreviation?: string
}

export interface IGDBTheme {
  id: number
  name: string
  slug: string
}

export interface IGDBGameMode {
  id: number
  name: string
  slug: string
}

export interface IGDBInvolvedCompany {
  id: number
  developer: boolean
  publisher: boolean
  company: {
    id: number
    name: string
    slug?: string
  }
}

export interface IGDBVideo {
  id: number
  name: string
  video_id: string
}

export interface IGDBWebsite {
  id: number
  category: number
  url: string
}

export interface IGDBSimilarGame {
  id: number
  name: string
  cover?: {
    id: number
    image_id: string
  }
}

export interface IGDBGame {
  id: number
  name: string
  slug: string
  summary?: string
  storyline?: string
  first_release_date?: number
  total_rating?: number
  total_rating_count?: number
  rating?: number
  aggregated_rating?: number
  category?: number
  status?: number
  url?: string
  cover?: IGDBCover
  artworks?: IGDBArtwork[]
  screenshots?: IGDBScreenshot[]
  genres?: IGDBGenre[]
  platforms?: IGDBPlatform[]
  themes?: IGDBTheme[]
  game_modes?: IGDBGameMode[]
  involved_companies?: IGDBInvolvedCompany[]
  videos?: IGDBVideo[]
  websites?: IGDBWebsite[]
  similar_games?: IGDBSimilarGame[]
}

export type IGDBImageSize =
  | 'cover_small'
  | 'cover_big'
  | 'screenshot_med'
  | 'screenshot_big'
  | 'screenshot_huge'
  | 'logo_med'
  | '720p'
  | '1080p'
  | 'micro'
  | 'thumb'

export function getIGDBImageUrl(imageId?: string, size: IGDBImageSize = '1080p'): string {
  if (!imageId) return ''
  return `https://images.igdb.com/igdb/image/upload/t_${size}/${imageId}.jpg`
}

export function getIGDBCoverUrl(imageId?: string, size: IGDBImageSize = 'cover_big'): string {
  if (!imageId) return ''
  return `https://images.igdb.com/igdb/image/upload/t_${size}/${imageId}.jpg`
}

export function getIGDBArtworkUrl(imageId?: string, size: IGDBImageSize = '1080p'): string {
  if (!imageId) return ''
  return `https://images.igdb.com/igdb/image/upload/t_${size}/${imageId}.jpg`
}

export function getIGDBScreenshotUrl(imageId?: string, size: IGDBImageSize = 'screenshot_huge'): string {
  if (!imageId) return ''
  return `https://images.igdb.com/igdb/image/upload/t_${size}/${imageId}.jpg`
}

const WEBSITE_CATEGORIES: Record<number, string> = {
  1: 'Official',
  2: 'Wikia',
  3: 'Wikipedia',
  4: 'Facebook',
  5: 'Twitter / X',
  6: 'Twitch',
  8: 'Instagram',
  9: 'YouTube',
  10: 'iPhone',
  11: 'iPad',
  12: 'Android',
  13: 'Steam',
  14: 'Reddit',
  15: 'Itch.io',
  16: 'Epic Games',
  17: 'GOG',
  18: 'Discord'
}

export function getWebsiteCategoryLabel(catId: number): string {
  return WEBSITE_CATEGORIES[catId] || 'Lien'
}

/**
 * Transforms an IGDBGame record into Loom's internal Game model
 */
export function mapIGDBGameToLoomGame(igdb: IGDBGame, defaults?: Partial<Game>): Game {
  const dev = igdb.involved_companies?.find((c) => c.developer)?.company.name || 'Studio Indépendant'
  const pub = igdb.involved_companies?.find((c) => c.publisher)?.company.name || dev

  // Banner priority: 1st artwork 1080p -> 1st screenshot 1080p -> cover
  const primaryBanner =
    (igdb.artworks && igdb.artworks.length > 0 && getIGDBArtworkUrl(igdb.artworks[0].image_id, '1080p')) ||
    (igdb.screenshots && igdb.screenshots.length > 0 && getIGDBScreenshotUrl(igdb.screenshots[0].image_id, '1080p')) ||
    (igdb.cover && getIGDBCoverUrl(igdb.cover.image_id, '1080p')) ||
    'https://images.unsplash.com/photo-1542751371-adc38448a05e?q=80&w=2070&auto=format&fit=crop'

  const primaryCover =
    (igdb.cover && getIGDBCoverUrl(igdb.cover.image_id, 'cover_big')) ||
    'https://images.unsplash.com/photo-1579373903781-fd5c0c30c4cd?q=80&w=1074&auto=format&fit=crop'

  const releaseYear = igdb.first_release_date
    ? new Date(igdb.first_release_date * 1000).getFullYear()
    : new Date().getFullYear()

  const screenshots = (igdb.screenshots || [])
    .slice(0, 6)
    .map((s) => getIGDBScreenshotUrl(s.image_id, 'screenshot_huge'))

  const websites = (igdb.websites || []).map((w) => ({
    category: getWebsiteCategoryLabel(w.category),
    url: w.url
  }))

  const videos = (igdb.videos || []).map((v) => ({
    name: v.name || 'Bande-annonce officielle',
    videoId: v.video_id
  }))

  const rating = igdb.total_rating ? Math.round(igdb.total_rating) : undefined

  return {
    id: defaults?.id || `igdb-${igdb.id}`,
    title: igdb.name,
    slug: igdb.slug || igdb.name.toLowerCase().replace(/[^a-z0-9]+/g, '-'),
    bannerUrl: defaults?.bannerUrl || primaryBanner,
    coverUrl: defaults?.coverUrl || primaryCover,
    iconUrl: defaults?.iconUrl,
    platform: defaults?.platform || 'steam',
    genre: igdb.genres?.map((g) => g.name) || ['Aventure'],
    playtimeMinutes: defaults?.playtimeMinutes ?? 0,
    lastPlayed: defaults?.lastPlayed,
    installed: defaults?.installed ?? false,
    installPath: defaults?.installPath,
    sizeGb: defaults?.sizeGb ?? (Math.floor(15 + (igdb.id % 60) * 1.5)),
    achievements: defaults?.achievements || [],
    news: defaults?.news || [
      {
        id: `news-${igdb.id}-1`,
        title: `Fiche officielle synchronisée via l'API IGDB`,
        summary: igdb.summary?.slice(0, 160) + '...' || 'Métadonnées et captures récupérées depuis la base IGDB.',
        date: 'Aujourd\'hui',
        author: dev
      }
    ],
    config: defaults?.config || {
      runner: 'proton-ge',
      winePrefix: `~/.local/share/loom/prefixes/${igdb.slug}`,
      gamemode: true,
      mangohud: true,
      gamescope: false,
      envVariables: {
        RADV_PERFTEST: 'gpl',
        ENABLE_VKBASALT: '1'
      },
      launchArguments: ''
    },
    developer: dev,
    publisher: pub,
    releaseYear,
    description: igdb.summary || igdb.storyline || 'Aucune description disponible pour ce titre.',
    screenshots: screenshots.length > 0 ? screenshots : [primaryBanner],
    isFavorite: defaults?.isFavorite ?? false,
    subtitle: igdb.genres?.[0]?.name ? `${igdb.genres[0].name} par ${dev}` : `Par ${dev}`,
    cloudSynced: defaults?.cloudSynced ?? true,
    igdbId: igdb.id,
    rating,
    ratingCount: igdb.total_rating_count,
    storyline: igdb.storyline,
    themes: igdb.themes?.map((t) => t.name) || [],
    gameModes: igdb.game_modes?.map((m) => m.name) || [],
    websites,
    videos
  }
}
