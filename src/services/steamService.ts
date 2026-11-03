import type { Game } from '@/models/game'
import detectedGamesRaw from './steam_detected_games.json'

export interface RealSteamGame {
  appid: string
  name: string
  slug: string
  installed: boolean
  playtimeMinutes: number
  lastPlayedTimestamp: number
  sizeGb: number
  installDir?: string
  coverUrl: string
  bannerUrl: string
}

export interface RealSteamAccount {
  steamId64: string
  accountName: string
  personaName: string
  installPath: string
  detected: boolean
  installedCount: number
  uninstalledCount: number
  totalGamesCount: number
  installedGames: RealSteamGame[]
  uninstalledGames: RealSteamGame[]
  allGames: RealSteamGame[]
}

/**
 * Builds Steam official asset URLs
 */
export function getSteamCoverUrl(appid: string): string {
  return `https://shared.steamstatic.com/store_item_assets/steam/apps/${appid}/library_600x900.jpg`
}

export function getSteamBannerUrl(appid: string): string {
  return `https://shared.steamstatic.com/store_item_assets/steam/apps/${appid}/library_hero.jpg`
}

export function getSteamHeaderUrl(appid: string): string {
  return `https://shared.steamstatic.com/store_item_assets/steam/apps/${appid}/header.jpg`
}

/**
 * Maps raw detected items to RealSteamGame
 */
const parsedAllGames: RealSteamGame[] = (detectedGamesRaw as Array<{
  appid: string
  name: string
  slug: string
  installed: boolean
  playtimeMinutes: number
  lastPlayedTimestamp: number
  sizeGb: number
  installDir?: string
}>).map((g) => ({
  ...g,
  coverUrl: getSteamCoverUrl(g.appid),
  bannerUrl: getSteamBannerUrl(g.appid)
}))

const parsedInstalled = parsedAllGames.filter((g) => g.installed)
const parsedUninstalled = parsedAllGames.filter((g) => !g.installed)

export const REAL_STEAM_DATA: RealSteamAccount = {
  steamId64: '76561198085607978',
  accountName: 'ik0bus',
  personaName: 'K0bus',
  installPath: '/home/k0bus/.local/share/Steam',
  detected: true,
  installedCount: parsedInstalled.length,
  uninstalledCount: parsedUninstalled.length,
  totalGamesCount: parsedAllGames.length,
  installedGames: parsedInstalled,
  uninstalledGames: parsedUninstalled,
  allGames: parsedAllGames
}

import { MOCK_IGDB_CATALOG } from './igdbService'

/**
 * Convert a RealSteamGame to Loom's unified Game model
 */
export function mapSteamGameToLoomGame(sGame: RealSteamGame): Game {
  const isInstalled = sGame.installed
  const now = Math.floor(Date.now() / 1000)
  let cleanTimestamp = sGame.lastPlayedTimestamp || 0
  if (cleanTimestamp > now) {
    if (cleanTimestamp - 1814400 <= now) {
      cleanTimestamp -= 1814400
    } else {
      cleanTimestamp = now
    }
  }

  const formattedDate =
    cleanTimestamp > 0
      ? new Date(cleanTimestamp * 1000).toLocaleDateString('fr-FR', {
          year: 'numeric',
          month: 'short',
          day: 'numeric'
        })
      : 'Jamais lancé'

  const normalize = (s: string) => (s || '').toLowerCase().replace(/[^a-z0-9]/g, '')
  const normSlug = normalize(sGame.slug)
  const normTitle = normalize(sGame.name)
  const igdbMatch = MOCK_IGDB_CATALOG.find(
    (e) =>
      e.slug === sGame.slug ||
      normalize(e.slug) === normSlug ||
      normalize(e.name) === normTitle ||
      (e.id === 480 && (sGame.slug === 'spacewar' || sGame.appid === '480')) ||
      (e.id === 2315050 && (sGame.slug === 'we-were-here-tomorrow' || sGame.appid === '2315050')) ||
      (e.id === 4126040 && (sGame.slug === 'aniimo' || sGame.appid === '4126040')) ||
      (e.id === 4704690 && (sGame.slug === 'meccha-chameleon' || sGame.appid === '4704690'))
  )

  const devCompany = igdbMatch?.involved_companies?.find((c) => c.developer)?.company.name
  const pubCompany = igdbMatch?.involved_companies?.find((c) => c.publisher)?.company.name
  const releaseYear = igdbMatch?.first_release_date
    ? new Date(igdbMatch.first_release_date * 1000).getFullYear()
    : 2024
  const genres = igdbMatch?.genres?.map((g) => g.name) || ['Steam']
  const rating =
    igdbMatch?.total_rating || igdbMatch?.rating
      ? Math.round(igdbMatch.total_rating || igdbMatch.rating || 0)
      : undefined

  return {
    id: `steam-${sGame.appid}`,
    title: sGame.name,
    slug: sGame.slug,
    bannerUrl: igdbMatch?.artworks?.[0]?.url || sGame.bannerUrl,
    coverUrl: igdbMatch?.cover?.url || sGame.coverUrl,
    iconUrl: `https://shared.steamstatic.com/store_item_assets/steam/apps/${sGame.appid}/capsule_231x87.jpg`,
    platform: 'steam',
    genre: genres,
    playtimeMinutes: sGame.playtimeMinutes,
    lastPlayed: formattedDate,
    lastPlayedTimestamp: cleanTimestamp,
    installed: isInstalled,
    installPath: sGame.installDir
      ? `/home/k0bus/.local/share/Steam/steamapps/common/${sGame.installDir}`
      : undefined,
    sizeGb: sGame.sizeGb,
    achievements: [],
    news: [],
    config: {
      runner: 'native',
      gamemode: true,
      mangohud: false,
      gamescope: false,
      envVariables: {},
      launchArguments: ''
    },
    developer: devCompany || 'Valve / Steam Partner',
    publisher: pubCompany || 'Steam',
    releaseYear,
    description:
      igdbMatch?.summary ||
      `Jeu officiel Steam (${isInstalled ? 'Installé sur le disque local' : 'Dans la bibliothèque Steam, prêt à être installé'}). AppID Steam: ${sGame.appid}`,
    storyline: igdbMatch?.storyline,
    rating,
    ratingCount: igdbMatch?.total_rating_count,
    themes: igdbMatch?.themes?.map((t) => t.name),
    gameModes: igdbMatch?.game_modes?.map((m) => m.name),
    screenshots:
      igdbMatch?.screenshots
        ?.map((s) => s.url)
        .filter((u): u is string => Boolean(u)) || [
        sGame.bannerUrl,
        getSteamHeaderUrl(sGame.appid)
      ],
    inLibrary: true,
    igdbId: igdbMatch?.id,
    storeLinks: [
      {
        store: 'Steam',
        url: `https://store.steampowered.com/app/${sGame.appid}/`,
        price: 'Possédé sur Steam',
        icon: 'steam'
      }
    ]
  }
}

class SteamService {
  public getDetectedAccount(): RealSteamAccount {
    return REAL_STEAM_DATA
  }

  public getInstalledGames(): RealSteamGame[] {
    return REAL_STEAM_DATA.installedGames
  }

  public getUninstalledGames(): RealSteamGame[] {
    return REAL_STEAM_DATA.uninstalledGames
  }

  public getAllSteamGames(): RealSteamGame[] {
    return REAL_STEAM_DATA.allGames
  }

  public getLoomLibraryGames(): Game[] {
    return REAL_STEAM_DATA.allGames.map(mapSteamGameToLoomGame)
  }

  /**
   * Generates a steam:// install or launch URL
   */
  public getSteamLaunchUri(appid: string): string {
    return `steam://rungameid/${appid}`
  }

  public getSteamInstallUri(appid: string): string {
    return `steam://install/${appid}`
  }
}

export const steamService = new SteamService()

