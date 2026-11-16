import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type {
  BattleNetGame,
  BattleNetAccount,
  BattleNetConfig,
  BlizzardGameEntitlement
} from '@/models/battlenet'
import type { Game } from '@/models/game'
import { ipc } from '@/services/ipc'
import { useToastStore } from '@/stores/toastStore'

// Authentic curated Blizzard artworks & banners
export interface BlizzardArtworkInfo {
  cover: string
  banner: string
  genre: string[]
  desc: string
  igdbId?: number
  igdbSlug?: string
  igdbUrl?: string
}

export const BLIZZARD_ARTWORK_MAP: Record<string, BlizzardArtworkInfo> = {
  wow: {
    cover: 'https://images.igdb.com/igdb/image/upload/t_cover_big/co8b7k.jpg',
    banner:
      'https://blz-contentstack-images.akamaized.net/v3/assets/blt9c12f249ac15c7ec/bltfe806acfa8672377/6a87bb854b66aa218691cf56/BG_Masthead_2600_Desktop.jpg',
    genre: ['MMORPG', 'Fantasy', 'Aventure'],
    desc: 'Explorez Azeroth dans The War Within. Affrontez les menaces souterraines et unissez les factions.',
    igdbId: 123,
    igdbSlug: 'world-of-warcraft',
    igdbUrl: 'https://www.igdb.com/games/world-of-warcraft'
  },
  wow_classic: {
    cover: 'https://images.igdb.com/igdb/image/upload/t_cover_big/co1trz.jpg',
    banner:
      'https://blz-contentstack-images.akamaized.net/v3/assets/blt9c12f249ac15c7ec/bltfe806acfa8672377/6a87bb854b66aa218691cf56/BG_Masthead_2600_Desktop.jpg',
    genre: ['MMORPG', 'Classique'],
    desc: "L'expérience originelle et authentique de World of Warcraft Classic.",
    igdbId: 75379,
    igdbSlug: 'world-of-warcraft-classic',
    igdbUrl: 'https://www.igdb.com/games/world-of-warcraft-classic'
  },
  wow_classic_era: {
    cover: 'https://images.igdb.com/igdb/image/upload/t_cover_big/co7bkt.jpg',
    banner:
      'https://blz-contentstack-images.akamaized.net/v3/assets/blt9c12f249ac15c7ec/bltfe806acfa8672377/6a87bb854b66aa218691cf56/BG_Masthead_2600_Desktop.jpg',
    genre: ['MMORPG', 'Season of Discovery', 'Runes'],
    desc: 'World of Warcraft Classic: Season of Discovery, runes de classe novatrices et raids réinventés.',
    igdbId: 275175,
    igdbSlug: 'world-of-warcraft-classic-season-of-discovery',
    igdbUrl: 'https://www.igdb.com/games/world-of-warcraft-classic-season-of-discovery'
  },
  wowf: {
    cover: 'https://images.igdb.com/igdb/image/upload/t_cover_big/co7bkq.jpg',
    banner:
      'https://blz-contentstack-images.akamaized.net/v3/assets/blt9c12f249ac15c7ec/bltfe806acfa8672377/6a87bb854b66aa218691cf56/BG_Masthead_2600_Desktop.jpg',
    genre: ['MMORPG', 'Forever', 'Classic+'],
    desc: "World of Warcraft: Forever sous Linux. Réinterprétation permanente d'Azeroth au niveau 60 (Classic+) avec nouvelles zones et donjons.",
    igdbId: 417650,
    igdbSlug: 'world-of-warcraft-forever',
    igdbUrl: 'https://www.igdb.com/games/world-of-warcraft-forever'
  },
  fenris: {
    cover: 'https://images.igdb.com/igdb/image/upload/t_cover_big/co69sm.jpg',
    banner:
      'https://shared.fastly.steamstatic.com/store_item_assets/steam/apps/2344520/library_hero.jpg',
    genre: ['Action-RPG', 'Hack\'n\'Slash', 'Dark Fantasy'],
    desc: 'Battez-vous contre les légions des Enfers et maîtrisez des compétences légendaires.'
  },
  pro: {
    cover: 'https://images.igdb.com/igdb/image/upload/t_cover_big/co885f.jpg',
    banner:
      'https://shared.fastly.steamstatic.com/store_item_assets/steam/apps/2357570/library_hero.jpg',
    genre: ['FPS', 'Hero Shooter', 'Gratuit', 'Free-to-Play', 'Compétitif'],
    desc: 'Un jeu de tir par équipe bourré d\'action avec un casting de héros en constante évolution. Gratuit pour tous.'
  },
  w3: {
    cover: 'https://images.igdb.com/igdb/image/upload/t_cover_big/co1rrv.jpg',
    banner:
      'https://blz-contentstack-images.akamaized.net/v3/assets/blt9c12f249ac15c7ec/blt668d39048ffb606f/6a9792a96cec0d4229b141a2/masthead_desktop.png',
    genre: ['STR', 'Stratégie', 'Fantasy'],
    desc: 'La refonte du chef-d\'œuvre de stratégie en temps réel de Blizzard.'
  },
  odin: {
    cover: 'https://images.igdb.com/igdb/image/upload/t_cover_big/coa8id.jpg',
    banner:
      'https://shared.fastly.steamstatic.com/store_item_assets/steam/apps/1938090/library_hero.jpg',
    genre: ['FPS', 'Battle Royale', 'Gratuit', 'Free-to-Play'],
    desc: 'Combats tactiques et guerre totale sur Call of Duty. Gratuit pour tous.'
  },
  hs: {
    cover: 'https://images.igdb.com/igdb/image/upload/t_cover_big/co1sh2.jpg',
    banner:
      'https://blz-contentstack-images.akamaized.net/v3/assets/bltc965041283bac56c/blt2a00940d7f28339b/64bea585e26c5443e04bc2a4/Masthead.png',
    genre: ['Cartes à collectionner', 'Stratégie', 'Gratuit', 'Free-to-Play'],
    desc: 'Prenez place dans la taverne pour des duels de cartes intenses et magiques. Gratuit pour tous.'
  },
  sc2: {
    cover: 'https://images.igdb.com/igdb/image/upload/t_cover_big/co1tnn.jpg',
    banner:
      'https://blz-contentstack-images.akamaized.net/v3/assets/blt9c12f249ac15c7ec/bltb017c0048472fd04/69646379b1b0ff9682e39241/terran_background_2600.webp',
    genre: ['STR', 'Science-Fiction', 'Gratuit', 'Free-to-Play', 'Compétitif'],
    desc: 'Commandez l\'une des trois puissantes races dans une guerre interstellaire sans merci. Campagne & Multi gratuits.'
  },
  hero: {
    cover: 'https://lutris.net/media/igdb/hots_grid_264x352.webp',
    banner:
      'https://blz-contentstack-images.akamaized.net/v3/assets/blt9c12f249ac15c7ec/blta8d048de9718bc10/6a97a0ae2924f59b018aa920/masthead_desktop.png',
    genre: ['MOBA', 'Arène', 'Gratuit', 'Free-to-Play'],
    desc: 'Batailles intenses réunissant les univers Warcraft, StarCraft, Diablo et Overwatch. Gratuit pour tous.'
  },
  anbs: {
    cover: 'https://images.igdb.com/igdb/image/upload/t_cover_big/co2pwy.jpg',
    banner:
      'https://blz-contentstack-images.akamaized.net/v3/assets/blt9c12f249ac15c7ec/bltd1d54d0ae64bba83/6a2053f8927dbbbf1e03bc77/di-masthead-bg_XL_warlock_2600.webp',
    genre: ['Action-RPG', 'Hack\'n\'Slash', 'Gratuit', 'Free-to-Play'],
    desc: 'MMO Action-RPG impitoyable se déroulant entre Diablo II et Diablo III. Gratuit pour tous.'
  },
  osi: {
    cover: 'https://images.igdb.com/igdb/image/upload/t_cover_big/co3gbk.jpg',
    banner:
      'https://blz-contentstack-images.akamaized.net/v3/assets/blt9c12f249ac15c7ec/blt6c60900c9bb4c4d8/6972a545d855f0a07c5d7824/masthead_overview_2600.png',
    genre: ['Action-RPG', 'Hack\'n\'Slash'],
    desc: 'Le classique intemporel Diablo II sublimé en 4K avec audio remasterisé.'
  },
  diablo3: {
    cover: 'https://images.igdb.com/igdb/image/upload/t_cover_big/co2l1u.jpg',
    banner:
      'https://blz-contentstack-images.akamaized.net/v3/assets/blt9c12f249ac15c7ec/blt9843230b12d957ea/6a8f5e2b2924f5fb788aa851/masthead_2600.png',
    genre: ['Action-RPG', 'Hack\'n\'Slash'],
    desc: 'Retournez à la Nouvelle-Tristram et repoussez l\'invasion des armées démoniaques.'
  },
  s1: {
    cover: 'https://images.igdb.com/igdb/image/upload/t_cover_big/co3wal.jpg',
    banner:
      'https://blz-contentstack-images.akamaized.net/v3/assets/blt9c12f249ac15c7ec/bltb017c0048472fd04/69646379b1b0ff9682e39241/terran_background_2600.webp',
    genre: ['STR', 'Science-Fiction'],
    desc: 'Le STR culte remasterisé avec graphismes haute définition.'
  },
  w1r: {
    cover: 'https://images.igdb.com/igdb/image/upload/t_cover_big/co1xup.jpg',
    banner:
      'https://blz-contentstack-images.akamaized.net/v3/assets/blt9c12f249ac15c7ec/blt668d39048ffb606f/6a9792a96cec0d4229b141a2/masthead_desktop.png',
    genre: ['STR', 'Rétro'],
    desc: 'L\'origine légendaire de la guerre entre Humains et Orcs, remasterisée en haute définition.'
  },
  w2r: {
    cover: 'https://images.igdb.com/igdb/image/upload/t_cover_big/co3wix.jpg',
    banner:
      'https://blz-contentstack-images.akamaized.net/v3/assets/blt9c12f249ac15c7ec/blt668d39048ffb606f/6a9792a96cec0d4229b141a2/masthead_desktop.png',
    genre: ['STR', 'Rétro'],
    desc: 'Tides of Darkness et Beyond the Dark Portal dans une version remasterisée moderne.'
  },
  rtro: {
    cover: 'https://lutris.net/media/igdb/blizarcade.jpg',
    banner:
      'https://blz-contentstack-images.akamaized.net/v3/assets/blt9c12f249ac15c7ec/blta8d048de9718bc10/6a97a0ae2924f59b018aa920/masthead_desktop.png',
    genre: ['Arcade', 'Rétro'],
    desc: 'The Lost Vikings, Blackthorne et Rock N\' Roll Racing dans leurs versions définitives.'
  }
}

export const useBattleNetStore = defineStore('battlenet', () => {
  const toast = useToastStore()

  // State
  const account = ref<BattleNetAccount | null>(null)
  const installedGames = ref<BattleNetGame[]>([])
  const ownedGames = ref<BlizzardGameEntitlement[]>([])
  const runningGames = ref<string[]>([])
  const config = ref<BattleNetConfig | null>(null)

  const isLoading = ref(false)
  const isScanning = ref(false)
  const isLaunching = ref(false)
  const launchingUid = ref<string | null>(null)

  // OAuth Credentials & Modal State
  const isAuthModalOpen = ref(false)
  const credentialsConfigured = ref(false)
  const configuredClientId = ref<string | null>(null)
  const isAuthenticating = ref(false)

  // Getters
  const isConnected = computed(() => account.value !== null)
  const battletag = computed(() => account.value?.battletag || '')
  const avatarUrl = computed(() => account.value?.avatar_url || '')
  const installedCount = computed(() => installedGames.value.filter((g) => g.is_installed).length)
  const totalGamesCount = computed(() => {
    // Unique UIDs across owned and installed
    const uids = new Set<string>()
    installedGames.value.forEach((g) => uids.add(g.product_uid))
    ownedGames.value.forEach((g) => uids.add(g.product_uid))
    return uids.size
  })

  // Conversion helpers: convert BattleNetGame or BlizzardGameEntitlement to Loom Game model
  function convertToLoomGame(bGame: BattleNetGame | BlizzardGameEntitlement): Game {
    const uid = bGame.product_uid.toLowerCase()
    const id = `bnet-${uid}`
    const isInstalled = 'is_installed' in bGame ? bGame.is_installed : false
    const installPath = 'install_path' in bGame && bGame.install_path ? bGame.install_path : undefined
    const version = 'version_build' in bGame && bGame.version_build ? bGame.version_build : undefined

    const lastPlayedTs = ('last_played' in bGame && bGame.last_played) ? bGame.last_played : undefined
    const now = Math.floor(Date.now() / 1000)

    let formattedLastPlayed: string | undefined = undefined
    if (lastPlayedTs && lastPlayedTs > 0) {
      const diffSec = now - lastPlayedTs
      if (diffSec < 120) {
        formattedLastPlayed = "À l'instant"
      } else if (diffSec < 3600) {
        const mins = Math.floor(diffSec / 60)
        formattedLastPlayed = `Il y a ${mins} min`
      } else if (diffSec < 86400) {
        const hours = Math.floor(diffSec / 3600)
        formattedLastPlayed = `Il y a ${hours} h`
      } else if (diffSec < 172800) {
        const d = new Date(lastPlayedTs * 1000)
        const timeStr = d.toLocaleTimeString('fr-FR', { hour: '2-digit', minute: '2-digit' })
        formattedLastPlayed = `Hier à ${timeStr}`
      } else {
        formattedLastPlayed = new Date(lastPlayedTs * 1000).toLocaleDateString('fr-FR', {
          day: 'numeric',
          month: 'short',
          year: 'numeric'
        })
      }
    }

    // Authentic curated artwork and metadata
    const art = BLIZZARD_ARTWORK_MAP[uid] || {
      cover: 'cover_url' in bGame && bGame.cover_url ? bGame.cover_url : 'https://images.igdb.com/igdb/image/upload/t_cover_big/co8b7k.jpg',
      banner: 'banner_url' in bGame && bGame.banner_url ? bGame.banner_url : 'https://blz-contentstack-images.akamaized.net/v3/assets/blt9c12f249ac15c7ec/bltfe806acfa8672377/6a87bb854b66aa218691cf56/BG_Masthead_2600_Desktop.jpg',
      genre: ['Battle.net'],
      desc: 'Jeu Blizzard Battle.net sous Linux.'
    }

    const statsObj = 'stats' in bGame && bGame.stats ? bGame.stats : undefined

    let subtitle = version ? `Build ${version}` : 'Plateforme Battle.net'
    if (bGame.name.includes('Forever') || uid === 'wowf') {
      subtitle = version ? `Édition Forever • Build ${version}` : 'Édition Forever'
    } else if (uid === 'wow_classic_era' || bGame.name.includes('Season of Discovery')) {
      subtitle = version ? `Season of Discovery • Build ${version}` : 'Season of Discovery'
    } else if (uid === 'wow_classic' || bGame.name.includes('Classic')) {
      subtitle = version ? `WoW Classic • Build ${version}` : 'WoW Classic'
    }

    const launchUid = (uid === 'wow_forever') ? 'wowf' : uid

    const playtimeSec = ('playtime_seconds' in bGame && bGame.playtime_seconds) ? bGame.playtime_seconds : 0
    const playtimeMinutes = Math.floor(playtimeSec / 60)

    return {
      id,
      title: bGame.name,
      slug: `bnet-${uid}`,
      igdbId: art.igdbId,
      bannerUrl: art.banner,
      coverUrl: art.cover,
      platform: 'battlenet',
      genre: art.genre,
      playtimeMinutes,
      lastPlayed: formattedLastPlayed,
      lastPlayedTimestamp: lastPlayedTs,
      installed: isInstalled,
      installPath,
      sizeGb: isInstalled ? 45.0 : 0,
      developer: 'Blizzard Entertainment',
      publisher: 'Blizzard Entertainment',
      releaseYear: 2024,
      description: ('summary' in bGame && (bGame as any).summary) ? (bGame as any).summary : art.desc,
      screenshots: [art.banner],
      achievements: [],
      news: [],
      config: {
        runner: 'wine-staging',
        gamemode: config.value ? config.value.enable_gamemode : true,
        mangohud: config.value ? config.value.enable_mangohud : false,
        gamescope: false,
        envVariables: {
          WINEPREFIX: config.value ? config.value.wine_prefix_path : '~/.local/share/loom/prefixes/battlenet',
          DXVK_ASYNC: '1',
          VKD3D_CONFIG: 'dxr11,dxr'
        },
        launchArguments: `--exec="launch ${launchUid}"`
      },
      subtitle,
      inLibrary: true,
      themes: statsObj ? [JSON.stringify(statsObj)] : []
    }
  }

  // Actions
  async function checkCredentials() {
    try {
      const res = await ipc.getBlizzardCredentials()
      credentialsConfigured.value = res.configured
      configuredClientId.value = res.clientId || null
      return res
    } catch {
      credentialsConfigured.value = false
      configuredClientId.value = null
      return { configured: false, clientId: null }
    }
  }

  async function saveCredentials(clientId: string, clientSecret: string) {
    isLoading.value = true
    try {
      await ipc.saveBlizzardCredentials(clientId, clientSecret)
      credentialsConfigured.value = true
      configuredClientId.value = clientId
      toast.show('Identifiants Blizzard enregistrés avec succès.', 'success')
      return true
    } catch (e: any) {
      toast.show(e?.message || 'Erreur enregistrement identifiants', 'error')
      throw e
    } finally {
      isLoading.value = false
    }
  }

  function openAuthModal() {
    isAuthModalOpen.value = true
  }

  function closeAuthModal() {
    isAuthModalOpen.value = false
  }

  async function loadAccount() {
    try {
      await checkCredentials()
      const acc = await ipc.getBlizzardAccountInfo()
      if (
        acc &&
        (acc.account_id?.startsWith('demo-') ||
          acc.battletag?.includes('LoomPlayer') ||
          acc.battletag?.includes('LoomChampion'))
      ) {
        await logoutBlizzard()
        account.value = null
        return null
      }
      if (acc && acc.avatar_url && acc.avatar_url.includes('guildrecruitment.world')) {
        acc.avatar_url = `https://ui-avatars.com/api/?name=${encodeURIComponent(
          (acc.battletag || 'Player').split('#')[0]
        )}&background=0284c7&color=ffffff&bold=true&size=128`
      }
      account.value = acc
      return acc
    } catch (e) {
      console.warn('[BattleNetStore] Échec chargement compte Blizzard:', e)
      return null
    }
  }

  async function loginBlizzard(skipPrompt = false) {
    if (!credentialsConfigured.value && !skipPrompt) {
      openAuthModal()
      return null
    }

    isLoading.value = true
    isAuthenticating.value = true
    try {
      toast.show('Ouverture du flux officiel Blizzard OAuth 2.0...', 'info')
      const acc = await ipc.startBlizzardOAuthFlow()
      if (acc && acc.avatar_url && acc.avatar_url.includes('guildrecruitment.world')) {
        acc.avatar_url = `https://ui-avatars.com/api/?name=${encodeURIComponent(
          (acc.battletag || 'Player').split('#')[0]
        )}&background=0284c7&color=ffffff&bold=true&size=128`
      }
      account.value = acc
      closeAuthModal()
      toast.show(`Compte Blizzard lié avec succès : ${acc.battletag}`, 'success')
      await loadOwnedGames()
      return acc
    } catch (e: any) {
      console.error('[BattleNetStore] Erreur OAuth2 Blizzard:', e)
      if (e?.message === 'CREDENTIALS_REQUIRED') {
        openAuthModal()
        toast.show("Veuillez renseigner vos identifiants d'API Blizzard.", 'info')
      } else {
        toast.show(e?.message || 'Échec de la connexion Blizzard OAuth2', 'error')
      }
      throw e
    } finally {
      isLoading.value = false
      isAuthenticating.value = false
    }
  }

  async function logoutBlizzard() {
    try {
      await ipc.disconnectBlizzardAccount()
      account.value = null
      ownedGames.value = []
      toast.show('Compte Blizzard déconnecté.', 'info')
    } catch (e: any) {
      toast.show(e?.message || 'Erreur lors de la déconnexion', 'error')
    }
  }

  async function loadConfig() {
    try {
      const cfg = await ipc.getBattleNetConfig()
      config.value = cfg
      return cfg
    } catch (e) {
      console.warn('[BattleNetStore] Erreur chargement configuration:', e)
      return null
    }
  }

  async function saveConfig(cfg: BattleNetConfig) {
    try {
      await ipc.saveBattleNetConfig(cfg)
      config.value = cfg
      toast.show('Configuration Battle.net enregistrée.', 'success')
    } catch (e: any) {
      toast.show(e?.message || 'Erreur enregistrement configuration', 'error')
    }
  }

  async function scanLocalGames(silent = false) {
    isScanning.value = true
    try {
      const games = await ipc.scanBattleNetGames()
      installedGames.value = games
      if (!silent) {
        toast.show(`${games.length} jeu(x) Battle.net détecté(s) localement.`, 'success')
      }
      return games
    } catch (e: any) {
      console.warn('[BattleNetStore] Erreur scan local:', e)
      if (!silent) {
        toast.show(e?.message || 'Erreur lors de l\'analyse des jeux Battle.net', 'error')
      }
      return []
    } finally {
      isScanning.value = false
    }
  }

  async function addExternalGame(path: string) {
    try {
      const game = await ipc.addExternalBattleNetGame(path)
      // Update local list
      const idx = installedGames.value.findIndex((g) => g.product_uid === game.product_uid)
      if (idx >= 0) {
        installedGames.value[idx] = game
      } else {
        installedGames.value.push(game)
      }
      toast.show(`Jeu externe '${game.name}' ajouté avec succès !`, 'success')
      return game
    } catch (e: any) {
      console.error('[BattleNetStore] Erreur ajout externe:', e)
      toast.show(e?.message || 'Dossier invalide ou jeu non reconnu', 'error')
      throw e
    }
  }

  async function loadOwnedGames() {
    try {
      const entitlements = await ipc.getBlizzardOwnedGames()
      ownedGames.value = entitlements
      return entitlements
    } catch (e) {
      console.warn('[BattleNetStore] Erreur chargement catalogue Blizzard:', e)
      return []
    }
  }

  function getProductTitle(uid: string): string {
    const map: Record<string, string> = {
      wowf: 'World of Warcraft: Forever',
      wow: 'World of Warcraft',
      wow_classic: 'World of Warcraft Classic',
      wow_classic_era: 'World of Warcraft Classic: Season of Discovery',
      fenris: 'Diablo IV',
      pro: 'Overwatch 2',
      osi: 'Diablo II: Resurrected',
      diablo3: 'Diablo III',
      w3: 'Warcraft III: Reforged',
      hs: 'Hearthstone',
      sc2: 'StarCraft II',
      s1: 'StarCraft: Remastered',
      hero: 'Heroes of the Storm',
      anbs: 'Diablo Immortal',
      odin: 'Call of Duty: Warzone',
      w1r: 'Warcraft I: Remastered',
      w2r: 'Warcraft II: Remastered',
      rtro: 'Blizzard Arcade Collection'
    }
    return map[uid.toLowerCase()] || uid
  }

  async function launchGame(productUid: string) {
    isLaunching.value = true
    launchingUid.value = productUid
    const title = getProductTitle(productUid)
    try {
      toast.show(`Lancement de ${title}...`, 'info')
      await ipc.launchBattleNetGame(productUid)
      toast.show(`Lancement transmis pour ${title}`, 'success')
    } catch (e: any) {
      console.error('[BattleNetStore] Erreur lancement:', e)
      if (e?.message?.includes('BATTLENET_NOT_INSTALLED') || e?.message?.includes('non encore installé')) {
        toast.show("Le client Battle.net n'est pas encore installé. Vous pouvez l'installer depuis les Comptes.", 'error')
      } else {
        toast.show(e?.message || 'Erreur lors du lancement de Battle.net', 'error')
      }
      throw e
    } finally {
      isLaunching.value = false
      launchingUid.value = null
    }
  }

  async function installBattleNetClient() {
    isLoading.value = true
    try {
      toast.show("Téléchargement et démarrage de l'installateur officiel Battle.net...", 'info')
      await ipc.installBattleNetClient()
      toast.show("L'installateur Battle.net a été lancé.", 'success')
    } catch (e: any) {
      toast.show(e?.message || "Erreur lors du lancement de l'installateur", 'error')
    } finally {
      isLoading.value = false
    }
  }

  async function killGame(productUid: string) {
    try {
      await ipc.killBattleNetGame(productUid)
      runningGames.value = runningGames.value.filter((uid) => uid !== productUid)
      toast.show(`Arrêt propre effectué pour ${productUid}`, 'info')
    } catch (e: any) {
      toast.show(e?.message || 'Erreur lors de l\'arrêt forcé', 'error')
    }
  }

  async function checkRunningGames() {
    try {
      const running = await ipc.getRunningBattleNetGames()
      runningGames.value = running
    } catch (e) {
      console.warn('[BattleNetStore] Erreur polling jeux actifs:', e)
    }
  }

  // Initial load
  loadAccount()
  loadConfig()
  scanLocalGames(true).then(() => {
    loadOwnedGames()
  })

  // Periodic polling for active processes
  if (typeof window !== 'undefined') {
    checkRunningGames()
    setInterval(checkRunningGames, 3000)
  }

  return {
    account,
    installedGames,
    ownedGames,
    runningGames,
    config,
    isLoading,
    isScanning,
    isLaunching,
    launchingUid,
    isAuthModalOpen,
    credentialsConfigured,
    configuredClientId,
    isAuthenticating,
    checkCredentials,
    saveCredentials,
    openAuthModal,
    closeAuthModal,
    isConnected,
    battletag,
    avatarUrl,
    installedCount,
    totalGamesCount,
    convertToLoomGame,
    loadAccount,
    loginBlizzard,
    logoutBlizzard,
    loadConfig,
    saveConfig,
    scanLocalGames,
    addExternalGame,
    loadOwnedGames,
    launchGame,
    installBattleNetClient,
    killGame,
    checkRunningGames
  }
})
