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

/**
 * Convert a RealSteamGame to Loom's unified Game model
 */
export function mapSteamGameToLoomGame(sGame: RealSteamGame): Game {
  const isInstalled = sGame.installed
  const formattedDate =
    sGame.lastPlayedTimestamp > 0
      ? new Date(sGame.lastPlayedTimestamp * 1000).toLocaleDateString('fr-FR', {
          year: 'numeric',
          month: 'short',
          day: 'numeric'
        })
      : 'Jamais lancé'

  return {
    id: `steam-${sGame.appid}`,
    title: sGame.name,
    slug: sGame.slug,
    bannerUrl: sGame.bannerUrl,
    coverUrl: sGame.coverUrl,
    iconUrl: `https://shared.steamstatic.com/store_item_assets/steam/apps/${sGame.appid}/capsule_231x87.jpg`,
    platform: 'steam',
    genre: ['Steam'],
    playtimeMinutes: sGame.playtimeMinutes,
    lastPlayed: formattedDate,
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
    developer: 'Valve / Steam Partner',
    publisher: 'Steam',
    releaseYear: 2024,
    description: `Jeu officiel Steam (${isInstalled ? 'Installé sur le disque local' : 'Dans la bibliothèque Steam, prêt à être installé'}). AppID Steam: ${sGame.appid}`,
    screenshots: [sGame.bannerUrl, getSteamHeaderUrl(sGame.appid)],
    inLibrary: true,
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

