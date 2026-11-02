import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { Game, GameFilter, RunnerType, GameConfig } from '@/models/game'
import { mapIGDBGameToLoomGame } from '@/models/igdb'
import { MOCK_IGDB_CATALOG } from '@/services/igdbService'
import { ipc } from '@/services/ipc'

export const useGamesStore = defineStore('games', () => {
  const games = ref<Game[]>([
    {
      id: 'cyberpunk-2077',
      title: 'Cyberpunk 2077: Phantom Liberty',
      slug: 'cyberpunk-2077',
      bannerUrl: 'https://images.unsplash.com/photo-1542751371-adc38448a05e?q=80&w=2070&auto=format&fit=crop',
      coverUrl: 'https://images.unsplash.com/photo-1579373903781-fd5c0c30c4cd?q=80&w=1074&auto=format&fit=crop',
      platform: 'steam',
      genre: ['Action RPG', 'Open World', 'Cyberpunk'],
      playtimeMinutes: 7420, // 123.6 hours
      lastPlayed: 'Il y a 2 heures',
      installed: true,
      installPath: '/mnt/games-nvme/SteamLibrary/steamapps/common/Cyberpunk 2077',
      sizeGb: 86.4,
      developer: 'CD PROJEKT RED',
      publisher: 'CD PROJEKT RED',
      releaseYear: 2020,
      description: 'Cyberpunk 2077 est un jeu de rôle et d\'action en monde ouvert qui se déroule dans la mégapole futuriste de Night City, obsédée par le pouvoir, la séduction et les modifications corporelles.',
      isFavorite: true,
      config: {
        runner: 'proton-ge',
        winePrefix: '~/.local/share/loom/prefixes/cyberpunk',
        gamemode: true,
        mangohud: true,
        gamescope: true,
        gamescopeResolution: '2560x1440@165',
        envVariables: {
          'RADV_PERFTEST': 'gpl',
          'ENABLE_VKBASALT': '1',
          'PROTON_ENABLE_NVAPI': '1'
        },
        launchArguments: '-high -novid'
      },
      screenshots: [
        'https://images.unsplash.com/photo-1542751371-adc38448a05e?q=80&w=1200&auto=format&fit=crop',
        'https://images.unsplash.com/photo-1511512578047-dfb367046420?q=80&w=1200&auto=format&fit=crop',
        'https://images.unsplash.com/photo-1538481199705-c710c4e965fc?q=80&w=1200&auto=format&fit=crop'
      ],
      achievements: [
        {
          id: 'ach-1',
          name: 'La Roue de la Fortune',
          description: 'Terminer la mission principale du Dogtown.',
          unlocked: true,
          unlockedAt: '28 Oct 2026',
          icon: '👑',
          rarityPercentage: 18.4
        },
        {
          id: 'ach-2',
          name: 'Légende Vivante',
          description: 'Atteindre le niveau de réputation maximal.',
          unlocked: true,
          unlockedAt: '15 Oct 2026',
          icon: '⭐',
          rarityPercentage: 32.1
        },
        {
          id: 'ach-3',
          name: 'Chrome Pur',
          description: 'Installer un implant cybernétique légendaire dans chaque emplacement.',
          unlocked: true,
          unlockedAt: '03 Oct 2026',
          icon: '⚡',
          rarityPercentage: 9.8
        },
        {
          id: 'ach-4',
          name: 'Névrose Cérébrale',
          description: 'Neutraliser 30 ennemis avec le piratage rapide sans vous faire repérer.',
          unlocked: false,
          icon: '🧠',
          rarityPercentage: 6.2
        }
      ],
      news: [
        {
          id: 'n-1',
          title: 'Patch 2.2 : Optimisations du Ray Reconstruction sous Linux & FSR 3.1',
          summary: 'La mise à jour 2.2 apporte des améliorations majeures du frametime sous Proton GE 9-20 et intègre la prise en charge native du Frame Generation.',
          date: 'Hier',
          author: 'CDPR Community'
        },
        {
          id: 'n-2',
          title: 'Night City Wire : Nouvel événement communautaire',
          summary: 'Participez au tournoi de photographie virtuelle et tentez de remporter des goodies physiques exclusifs signés par l\'équipe.',
          date: '24 Oct 2026',
          author: 'CDPR Events'
        }
      ]
    },
    {
      id: 'minecraft',
      title: 'Minecraft: Java Edition',
      slug: 'minecraft',
      bannerUrl: 'https://images.unsplash.com/photo-1627856013091-fed6e4e30025?q=80&w=2070&auto=format&fit=crop',
      coverUrl: 'https://images.unsplash.com/photo-1607604276583-eef5d076aa5f?q=80&w=1074&auto=format&fit=crop',
      platform: 'minecraft',
      genre: ['Sandbox', 'Survie', 'Aventure'],
      playtimeMinutes: 28400, // 473 hours
      lastPlayed: 'Hier à 22:15',
      installed: true,
      installPath: '~/.local/share/loom/instances',
      sizeGb: 12.8,
      developer: 'Mojang Studios',
      publisher: 'Xbox Game Studios',
      releaseYear: 2011,
      description: 'Explorez votre propre monde généré aléatoirement, survivez aux monstres la nuit et construisez les créations les plus extraordinaires grâce au support étendu des mods Fabric et NeoForge.',
      isFavorite: true,
      config: {
        runner: 'native',
        gamemode: true,
        mangohud: false,
        gamescope: false,
        envVariables: {
          'MESA_GL_VERSION_OVERRIDE': '4.6'
        },
        launchArguments: '-XX:+UseG1GC -XX:+ParallelRefProcEnabled'
      },
      screenshots: [
        'https://images.unsplash.com/photo-1627856013091-fed6e4e30025?q=80&w=1200&auto=format&fit=crop',
        'https://images.unsplash.com/photo-1579783902614-a3fb3927b675?q=80&w=1200&auto=format&fit=crop'
      ],
      achievements: [
        {
          id: 'mc-1',
          name: 'Libérez l\'Ender',
          description: 'Vaincre le dragon de l\'Ender dans le monde de l\'End.',
          unlocked: true,
          unlockedAt: '12 Sept 2026',
          icon: '🐉',
          rarityPercentage: 45.0
        },
        {
          id: 'mc-2',
          name: 'Balise Lumineuse',
          description: 'Créer et alimenter une balise au niveau maximum.',
          unlocked: true,
          icon: '💎',
          rarityPercentage: 22.3
        }
      ],
      news: [
        {
          id: 'mc-n1',
          title: 'Mise à jour Tricky Trials 1.21.4 disponible !',
          summary: 'Découvrez les nouvelles chambres d\'épreuves, le marteau lourd et les ajustements de combat avec le bundle enfin intégré.',
          date: 'Il y a 3 jours',
          author: 'Mojang Studios'
        }
      ]
    },
    {
      id: 'elden-ring',
      title: 'Elden Ring: Shadow of the Erdtree',
      slug: 'elden-ring',
      bannerUrl: 'https://images.unsplash.com/photo-1518709268805-4e9042af9f23?q=80&w=2084&auto=format&fit=crop',
      coverUrl: 'https://images.unsplash.com/photo-1563089145-599997674d42?q=80&w=1074&auto=format&fit=crop',
      platform: 'steam',
      genre: ['Action RPG', 'Souls-like', 'Dark Fantasy'],
      playtimeMinutes: 11200, // 186.6 hours
      lastPlayed: 'Il y a 3 jours',
      installed: true,
      installPath: '/mnt/games-nvme/SteamLibrary/steamapps/common/ELDEN RING',
      sizeGb: 64.2,
      developer: 'FromSoftware Inc.',
      publisher: 'Bandai Namco',
      releaseYear: 2022,
      description: 'L\'Ordre d\'or a été brisé. Dans l\'Entre-terre où règne la reine Marika l\'Éternelle, le Cercle d\'Elden a volé en éclats. Levez-vous, Sans-éclat.',
      isFavorite: true,
      config: {
        runner: 'proton-ge',
        winePrefix: '~/.local/share/loom/prefixes/eldenring',
        gamemode: true,
        mangohud: true,
        gamescope: false,
        envVariables: {
          'PROTON_NO_ESYNC': '0'
        },
        launchArguments: ''
      },
      screenshots: [
        'https://images.unsplash.com/photo-1518709268805-4e9042af9f23?q=80&w=1200&auto=format&fit=crop'
      ],
      achievements: [
        {
          id: 'er-1',
          name: 'Seigneur d\'Elden',
          description: 'Atteindre la fin Seigneur d\'Elden.',
          unlocked: true,
          icon: '🗡️',
          rarityPercentage: 27.5
        },
        {
          id: 'er-2',
          name: 'Malenia, Épée de Miquella',
          description: 'Terrasser Malenia, la déesse de la putréfaction.',
          unlocked: true,
          icon: '🌸',
          rarityPercentage: 35.8
        }
      ],
      news: [
        {
          id: 'er-n1',
          title: 'Calibrage 1.14 : Équilibrage des armes colossales et sorts de foi',
          summary: 'Ajustement des dégâts de posture et des frames d\'invulnérabilité pour les compétences de bouclier.',
          date: '19 Oct 2026',
          author: 'FromSoftware'
        }
      ]
    },
    {
      id: 'baldurs-gate-3',
      title: 'Baldur\'s Gate 3',
      slug: 'baldurs-gate-3',
      bannerUrl: 'https://images.unsplash.com/photo-1511512578047-dfb367046420?q=80&w=2070&auto=format&fit=crop',
      coverUrl: 'https://images.unsplash.com/photo-1550745165-9bc0b252726f?q=80&w=1074&auto=format&fit=crop',
      platform: 'steam',
      genre: ['CRPG', 'Tour par tour', 'Fantasy'],
      playtimeMinutes: 9800,
      lastPlayed: 'Il y a 5 jours',
      installed: true,
      sizeGb: 122.0,
      developer: 'Larian Studios',
      publisher: 'Larian Studios',
      releaseYear: 2023,
      description: 'Rassemblez votre groupe et retournez aux Royaumes Oubliés dans une histoire d\'amitié, de trahison, de sacrifice et de survie.',
      isFavorite: false,
      config: {
        runner: 'proton-experimental',
        gamemode: true,
        mangohud: false,
        gamescope: false,
        envVariables: {},
        launchArguments: '--skip-launcher'
      },
      screenshots: [],
      achievements: [],
      news: []
    },
    {
      id: 'hades-2',
      title: 'Hades II',
      slug: 'hades-2',
      bannerUrl: 'https://images.unsplash.com/photo-1551103782-8ab07afd45c1?q=80&w=2070&auto=format&fit=crop',
      coverUrl: 'https://images.unsplash.com/photo-1538481199705-c710c4e965fc?q=80&w=1074&auto=format&fit=crop',
      platform: 'steam',
      genre: ['Rogue-like', 'Action', 'Mythologie'],
      playtimeMinutes: 3450,
      lastPlayed: 'Il y a 1 semaine',
      installed: true,
      sizeGb: 14.5,
      developer: 'Supergiant Games',
      publisher: 'Supergiant Games',
      releaseYear: 2024,
      description: 'Battez-vous au-delà des Enfers en utilisant la magie noire pour affronter le Titan du Temps dans cette suite envoûtante.',
      isFavorite: false,
      config: {
        runner: 'proton-experimental',
        gamemode: true,
        mangohud: true,
        gamescope: false,
        envVariables: {},
        launchArguments: ''
      },
      screenshots: [],
      achievements: [],
      news: []
    },
    {
      id: 'hollow-knight-silksong',
      title: 'HOLLOW KNIGHT',
      subtitle: 'SILKSONG',
      slug: 'hollow-knight-silksong',
      bannerUrl: 'https://images.unsplash.com/photo-1578632767115-351597cf2477?q=80&w=2070&auto=format&fit=crop',
      coverUrl: 'https://images.unsplash.com/photo-1618005182384-a83a8bd57fbe?q=80&w=1074&auto=format&fit=crop',
      platform: 'steam',
      genre: ['Metroidvania', 'Action', 'Difficult', 'Atmospheric'],
      playtimeMinutes: 2406, // 40.1 hours
      lastPlayed: 'Yesterday',
      installed: true,
      installPath: '~/.local/share/wine/hk_ss',
      sizeGb: 18.4,
      developer: 'Team Cherry',
      publisher: 'Team Cherry',
      releaseYear: 2026,
      totalAchievementsCount: 52,
      cloudSynced: true,
      description: 'Découvrez un vaste royaume hanté dans Hollow Knight: Silksong ! Incarnez Hornet, la princesse protectrice d\'Hallownest.',
      isFavorite: true,
      config: {
        runner: 'proton-ge',
        winePrefix: '~/.local/share/wine/hk_ss',
        gamemode: true,
        mangohud: true,
        gamescope: true,
        gamescopeResolution: '2560x1440@165',
        envVariables: {
          'VKD3D_CONFIG': 'dxr',
          'MANGOHUD': '1'
        },
        launchArguments: ''
      },
      screenshots: [
        'https://images.unsplash.com/photo-1578632767115-351597cf2477?q=80&w=1200&auto=format&fit=crop',
        'https://images.unsplash.com/photo-1518709268805-4e9042af9f23?q=80&w=1200&auto=format&fit=crop'
      ],
      achievements: [
        {
          id: 'hk-ach-1',
          name: 'Needle Master',
          description: 'Acquired Hornet\'s pristine silk thread',
          unlocked: true,
          unlockedAt: 'Nov 01, 2026',
          icon: '🗡️',
          rarityPercentage: 14.2
        },
        {
          id: 'hk-ach-2',
          name: 'Citadel Descent',
          description: 'Cross the Great Gate of the Citadel',
          unlocked: true,
          unlockedAt: 'Oct 28, 2026',
          icon: '👑',
          rarityPercentage: 22.8
        },
        {
          id: 'hk-ach-3',
          name: 'Thread & Bone',
          description: 'Craft your first high-tier crest from ancient materials',
          unlocked: true,
          unlockedAt: 'Oct 24, 2026',
          icon: '🧵',
          rarityPercentage: 35.1
        },
        {
          id: 'hk-ach-4',
          name: 'Pharloom Cartographer',
          description: 'Map every district of the lower kingdom',
          unlocked: false,
          icon: '🗺️',
          rarityPercentage: 8.4
        },
        {
          id: 'hk-ach-5',
          name: 'Silk Reaver',
          description: 'Slay 50 bell-bearing automations in the deep crypts',
          unlocked: false,
          icon: '⚔️',
          rarityPercentage: 5.1
        }
      ],
      news: [
        {
          id: 'hk-patch-1',
          title: 'Vulkan Shader Pipeline Optimizations & Linux GameScope Fixes',
          summary: 'This update addresses asynchronous shader pipeline compilation stutters when traversing the Pharloom Citadel. Fully tested against Proton GE-9-20 and Mesa 24.2. Includes fixes for 16:10 resolutions on handheld displays.',
          date: '2 days ago',
          author: 'Team Cherry',
          tag: 'Patch Note 1.0.4b',
          hearts: 1420,
          comments: 284,
          type: 'patches'
        },
        {
          id: 'hk-news-2',
          title: 'Original Soundtrack Preview: Christopher Larkin on Hornet\'s Theme',
          summary: 'Explore the intricate acoustic strings and choral arrangements recorded for the Coral Forest biome. Features the Melbourne Symphony Orchestra live session.',
          date: 'Nov 28, 2026',
          author: 'Official Blog',
          tag: 'Community Spotlight',
          hearts: 3892,
          comments: 512,
          type: 'news'
        }
      ]
    }
  ])

  const runningGameId = ref<string | null>(null)
  const isLaunching = ref(false)

  // Current active game for details
  const activeGame = ref<Game | null>(games.value[0])

  // Getters
  const heroGame = computed(() => {
    return games.value.find((g) => g.id === 'cyberpunk-2077') || games.value[0]
  })

  const recentGames = computed(() => {
    return [...games.value].sort((a, b) => b.playtimeMinutes - a.playtimeMinutes).slice(0, 5)
  })

  const installedGames = computed(() => {
    return games.value.filter((g) => g.installed)
  })

  const allNews = computed(() => {
    const list: Array<{ game: Game; news: Game['news'][0] }> = []
    games.value.forEach((g) => {
      g.news.forEach((n) => {
        list.push({ game: g, news: n })
      })
    })
    return list.slice(0, 3)
  })

  const discoveredGames = ref<Map<string, Game>>(new Map())

  // Actions
  function getGameById(id: string): Game | undefined {
    // 1. Search in user's library
    const inLib = games.value.find((g) => g.id === id || g.slug === id)
    if (inLib) {
      if (inLib.inLibrary === undefined) inLib.inLibrary = true
      return inLib
    }

    // 2. Search in discovered cache
    if (discoveredGames.value.has(id)) {
      return discoveredGames.value.get(id)
    }

    // 3. If ID starts with 'igdb-'
    if (id.startsWith('igdb-')) {
      const numericId = parseInt(id.replace('igdb-', ''), 10)
      const foundInCatalog = MOCK_IGDB_CATALOG.find((g) => g.id === numericId)
      if (foundInCatalog) {
        const mapped = mapIGDBGameToLoomGame(foundInCatalog, { inLibrary: false })
        discoveredGames.value.set(id, mapped)
        return mapped
      }
    }

    // 4. Also check if ID matches any slug in MOCK_IGDB_CATALOG
    const foundBySlug = MOCK_IGDB_CATALOG.find((g) => g.slug === id)
    if (foundBySlug) {
      const mapped = mapIGDBGameToLoomGame(foundBySlug, { inLibrary: false })
      discoveredGames.value.set(mapped.id, mapped)
      discoveredGames.value.set(id, mapped)
      return mapped
    }

    return undefined
  }

  function registerDiscoveredGame(game: Game) {
    discoveredGames.value.set(game.id, game)
  }

  function addToLibrary(gameId: string): Game | undefined {
    const existing = games.value.find((g) => g.id === gameId)
    if (existing) {
      existing.inLibrary = true
      return existing
    }

    const discovered = discoveredGames.value.get(gameId) || getGameById(gameId)
    if (discovered) {
      discovered.inLibrary = true
      games.value.unshift(discovered)
      return discovered
    }
    return undefined
  }

  async function launchGame(id: string) {
    const game = getGameById(id)
    if (!game) return

    isLaunching.value = true
    try {
      await ipc.invoke('launch_game', {
        gameId: id,
        runner: game.config.runner,
        gamemode: game.config.gamemode,
        mangohud: game.config.mangohud
      })
      runningGameId.value = id
      game.lastPlayed = 'En cours d\'exécution'
    } catch (err) {
      console.error('Failed to launch game:', err)
    } finally {
      isLaunching.value = false
    }
  }

  async function stopGame(id: string) {
    if (runningGameId.value !== id) return
    try {
      await ipc.invoke('stop_game', { gameId: id })
      runningGameId.value = null
      const game = getGameById(id)
      if (game) game.lastPlayed = 'À l\'instant'
    } catch (err) {
      console.error('Failed to stop game:', err)
    }
  }

  function updateGameRunner(id: string, runner: RunnerType) {
    const game = getGameById(id)
    if (game) {
      game.config.runner = runner
    }
  }

  function updateGameConfig(id: string, partialConfig: Partial<GameConfig>) {
    const game = getGameById(id)
    if (game) {
      game.config = { ...game.config, ...partialConfig }
    }
  }

  function toggleFavorite(id: string) {
    const game = getGameById(id)
    if (game) {
      game.isFavorite = !game.isFavorite
    }
  }

  function filterGames(filters: GameFilter): Game[] {
    return games.value.filter((game) => {
      // Fuzzy search match
      if (filters.search) {
        const query = filters.search.toLowerCase().trim()
        const titleMatch = game.title.toLowerCase().includes(query)
        const genreMatch = game.genre.some((g) => g.toLowerCase().includes(query))
        if (!titleMatch && !genreMatch) return false
      }

      // Platform filter
      if (filters.platform && filters.platform !== 'all' && game.platform !== filters.platform) {
        return false
      }

      // Genre filter
      if (filters.genre && filters.genre !== 'all' && !game.genre.includes(filters.genre)) {
        return false
      }

      // Runner filter
      if (filters.runner && filters.runner !== 'all' && game.config.runner !== filters.runner) {
        return false
      }

      return true
    }).sort((a, b) => {
      let result = 0
      if (filters.sortBy === 'title') {
        result = a.title.localeCompare(b.title)
      } else if (filters.sortBy === 'playtime') {
        result = b.playtimeMinutes - a.playtimeMinutes
      } else if (filters.sortBy === 'size') {
        result = b.sizeGb - a.sizeGb
      } else {
        result = b.playtimeMinutes - a.playtimeMinutes
      }
      return filters.sortOrder === 'desc' ? result : -result
    })
  }

  return {
    games,
    runningGameId,
    isLaunching,
    activeGame,
    heroGame,
    recentGames,
    installedGames,
    allNews,
    getGameById,
    launchGame,
    stopGame,
    updateGameRunner,
    updateGameConfig,
    toggleFavorite,
    filterGames,
    discoveredGames,
    registerDiscoveredGame,
    addToLibrary
  }
})
