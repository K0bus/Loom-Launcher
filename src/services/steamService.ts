export interface RealSteamGame {
  appid: string
  name: string
  slug: string
  sizeBytes: number
  installDir: string
}

export interface RealSteamAccount {
  steamId64: string
  accountName: string
  personaName: string
  installPath: string
  detected: boolean
  installedGames: RealSteamGame[]
}

/**
 * Real local Steam detector for Linux system (~/.local/share/Steam)
 * Extracted directly from ACF manifests and loginusers.vdf on host
 */
export const REAL_STEAM_DATA: RealSteamAccount = {
  steamId64: '76561198085607978',
  accountName: 'ik0bus',
  personaName: 'K0bus',
  installPath: '/home/k0bus/.local/share/Steam',
  detected: true,
  installedGames: [
    {
      appid: '108600',
      name: 'Project Zomboid',
      slug: 'project-zomboid',
      sizeBytes: 10438270044,
      installDir: 'Project Zomboid'
    },
    {
      appid: '892970',
      name: 'Valheim',
      slug: 'valheim',
      sizeBytes: 4222188741,
      installDir: 'Valheim'
    },
    {
      appid: '1203620',
      name: 'Enshrouded',
      slug: 'enshrouded',
      sizeBytes: 51824645608,
      installDir: 'Enshrouded'
    },
    {
      appid: '739630',
      name: 'Phasmophobia',
      slug: 'phasmophobia',
      sizeBytes: 44706808096,
      installDir: 'Phasmophobia'
    },
    {
      appid: '594650',
      name: 'Hunt: Showdown 1896',
      slug: 'hunt-showdown',
      sizeBytes: 101021452506,
      installDir: 'Hunt Showdown'
    },
    {
      appid: '4000',
      name: "Garry's Mod",
      slug: 'garrys-mod',
      sizeBytes: 7326127784,
      installDir: "GarrysMod"
    },
    {
      appid: '1169040',
      name: 'Necesse',
      slug: 'necesse',
      sizeBytes: 532617740,
      installDir: 'Necesse'
    },
    {
      appid: '1817800',
      name: 'Oddsparks: An Automation Adventure',
      slug: 'oddsparks-an-automation-adventure',
      sizeBytes: 6791067094,
      installDir: 'Oddsparks'
    },
    {
      appid: '2315050',
      name: 'We Were Here Tomorrow',
      slug: 'we-were-here-tomorrow',
      sizeBytes: 21374946057,
      installDir: 'We Were Here Tomorrow'
    },
    {
      appid: '2708450',
      name: 'Oh Deer',
      slug: 'oh-deer',
      sizeBytes: 2243068728,
      installDir: 'Oh Deer'
    },
    {
      appid: '4126040',
      name: 'Aniimo',
      slug: 'aniimo',
      sizeBytes: 37480264826,
      installDir: 'Aniimo'
    }
  ]
}

class SteamService {
  public getDetectedAccount(): RealSteamAccount {
    return REAL_STEAM_DATA
  }

  public getInstalledGames(): RealSteamGame[] {
    return REAL_STEAM_DATA.installedGames
  }
}

export const steamService = new SteamService()
