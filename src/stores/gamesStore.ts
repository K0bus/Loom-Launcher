import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { Game, GameFilter, RunnerType, GameConfig, Achievement } from '@/models/game'
import { mapIGDBGameToLoomGame } from '@/models/igdb'
import { MOCK_IGDB_CATALOG, igdbService } from '@/services/igdbService'
import { steamService } from '@/services/steamService'
import { achievementService } from '@/services/achievementService'
import { ipc } from '@/services/ipc'

export const useGamesStore = defineStore('games', () => {
  const STORAGE_KEY_LIBRARY = 'loom_real_library_games'

  function applyIGDBMatch(game: Game, igdbMatch: any) {
    const devCompany = igdbMatch.involved_companies?.find((c: any) => c.developer)?.company?.name
    const pubCompany = igdbMatch.involved_companies?.find((c: any) => c.publisher)?.company?.name
    const year = igdbMatch.first_release_date
      ? new Date(igdbMatch.first_release_date * 1000).getFullYear()
      : game.releaseYear

    if (devCompany) game.developer = devCompany
    if (pubCompany) game.publisher = pubCompany
    if (year) game.releaseYear = year
    if (igdbMatch.summary) game.description = igdbMatch.summary
    if (igdbMatch.storyline) game.storyline = igdbMatch.storyline
    if (igdbMatch.genres && igdbMatch.genres.length > 0) {
      game.genre = igdbMatch.genres.map((g: any) => g.name)
    }
    if (igdbMatch.total_rating || igdbMatch.rating) {
      game.rating = Math.round(igdbMatch.total_rating || igdbMatch.rating || 0)
      game.ratingCount = igdbMatch.total_rating_count
    }
    if (igdbMatch.themes && igdbMatch.themes.length > 0) {
      game.themes = igdbMatch.themes.map((t: any) => t.name)
    }
    if (igdbMatch.game_modes && igdbMatch.game_modes.length > 0) {
      game.gameModes = igdbMatch.game_modes.map((m: any) => m.name)
    }
    if (igdbMatch.screenshots && igdbMatch.screenshots.length > 0) {
      game.screenshots = igdbMatch.screenshots.map((s: any) => s.url)
    }
    if (igdbMatch.artworks && igdbMatch.artworks.length > 0 && igdbMatch.artworks[0].url) {
      game.bannerUrl = igdbMatch.artworks[0].url
    }
    if (igdbMatch.cover?.url) {
      game.coverUrl = igdbMatch.cover.url
    }
    game.igdbId = igdbMatch.id
  }

  function enrichWithCatalogIfAvailable(game: Game) {
    if (game.igdbId && game.developer !== 'Valve / Steam Partner') {
      return
    }
    const normalize = (s: string) => s.toLowerCase().replace(/[^a-z0-9]/g, '')
    const gameNormSlug = normalize(game.slug)
    const gameNormTitle = normalize(game.title)
    const catalogMatch = MOCK_IGDB_CATALOG.find((entry) => {
      const entryNormSlug = normalize(entry.slug)
      const entryNormName = normalize(entry.name)
      return (
        entry.slug === game.slug ||
        entryNormSlug === gameNormSlug ||
        entryNormName === gameNormTitle ||
        (entry.id === 480 && (game.slug === 'spacewar' || game.id === 'steam-480'))
      )
    })
    if (catalogMatch) {
      applyIGDBMatch(game, catalogMatch)
    }
  }

  function loadPersistedLibrary(): Game[] {
    try {
      const fresh = steamService.getLoomLibraryGames()
      fresh.forEach(enrichWithCatalogIfAvailable)

      const saved = localStorage.getItem(STORAGE_KEY_LIBRARY)
      if (saved) {
        const parsed = JSON.parse(saved)
        if (Array.isArray(parsed) && parsed.length > 0) {
          const now = Math.floor(Date.now() / 1000)
          // Refresh if library count expanded (e.g. from 294 to 860), has future timestamps, or missing IGDB match for known games
          if (
            parsed.length < fresh.length ||
            parsed.some((g) => g.platform === 'steam' && g.lastPlayedTimestamp === undefined) ||
            parsed.some((g) => (g.lastPlayedTimestamp || 0) > now) ||
            parsed.some(
              (g) =>
                (g.slug === 'aniimo' ||
                  g.slug === 'meccha-chameleon' ||
                  g.slug === 'spacewar' ||
                  g.slug === 'we-were-here-tomorrow') &&
                (!g.igdbId ||
                  g.developer === 'Valve / Steam Partner' ||
                  !g.coverUrl?.startsWith('/steam-assets/'))
            )
          ) {
            try {
              localStorage.setItem(STORAGE_KEY_LIBRARY, JSON.stringify(fresh))
            } catch {}
            return fresh
          }
          parsed.forEach(enrichWithCatalogIfAvailable)
          return parsed
        }
      }
      try {
        localStorage.setItem(STORAGE_KEY_LIBRARY, JSON.stringify(fresh))
      } catch {}
      return fresh
    } catch (e) {
      console.warn('[gamesStore] Failed to load persisted library:', e)
      const fresh = steamService.getLoomLibraryGames()
      fresh.forEach(enrichWithCatalogIfAvailable)
      return fresh
    }
  }

  function saveLibrary() {
    try {
      localStorage.setItem(STORAGE_KEY_LIBRARY, JSON.stringify(games.value))
    } catch (e) {
      console.warn('[gamesStore] Failed to save library:', e)
    }
  }

  // Real library state: populated with real detected Steam games
  const games = ref<Game[]>(loadPersistedLibrary())
  const isScanningLibrary = ref(false)
  const lastScannedAt = ref<Date | null>(new Date())

  const runningGameId = ref<string | null>(null)
  const isLaunching = ref(false)
  const launchingGameId = ref<string | null>(null)

  // Current active game for details
  const activeGame = ref<Game | null>(null)

  /**
   * Synchronise l'état réel des processus Steam
   * Ne met JAMAIS un état factice : interroge Steam via IPC/API
   */
  async function checkRunningGames(): Promise<void> {
    try {
      const res = await ipc.invoke<{ success: boolean; runningAppId: string | null; pids?: number[] }>('get_running_games')
      if (res && res.runningAppId) {
        const fullId = `steam-${res.runningAppId}`
        runningGameId.value = fullId
        launchingGameId.value = null
        const game = getGameById(fullId)
        if (game) {
          game.lastPlayed = 'En cours d\'exécution'
        }
      } else {
        if (runningGameId.value) {
          const prev = getGameById(runningGameId.value)
          if (prev) {
            prev.lastPlayed = 'À l\'instant'
          }
          runningGameId.value = null
        }
      }
    } catch (e) {
      console.warn('[gamesStore] Failed to poll running games state:', e)
    }
  }

  // Polling automatique de l'état réel de Steam et chargement des succès
  if (typeof window !== 'undefined') {
    checkRunningGames()
    setInterval(checkRunningGames, 2000)
    syncAchievementSummaries()
  }

  // Getters
  const heroGame = computed<Game | null>(() => {
    return recentGames.value[0] || games.value[0] || null
  })

  const recentGames = computed(() => {
    return [...games.value]
      .filter((g) => (g.lastPlayedTimestamp || 0) > 0)
      .sort((a, b) => (b.lastPlayedTimestamp || 0) - (a.lastPlayedTimestamp || 0))
      .slice(0, 8)
  })

  const installedGames = computed(() => {
    return games.value.filter((g) => g.installed)
  })

  const uninstalledGames = computed(() => {
    return games.value.filter((g) => !g.installed)
  })

  const installedCount = computed(() => installedGames.value.length)
  const uninstalledCount = computed(() => uninstalledGames.value.length)
  const totalCount = computed(() => games.value.length)

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
  function isGameInLibrary(slugOrId: string): boolean {
    return games.value.some(
      (g) => g.slug === slugOrId || g.id === slugOrId || (g.igdbId && `igdb-${g.igdbId}` === slugOrId)
    )
  }

  function getGameBySlug(slug: string): Game | undefined {
    return (
      games.value.find((g) => g.slug === slug) ||
      discoveredGames.value.get(slug) ||
      getGameById(slug)
    )
  }

  function getGameById(idOrSlug: string): Game | undefined {
    // 1. Search in user's real library (by slug, id, or igdb-${igdbId})
    const inLib = games.value.find(
      (g) => g.slug === idOrSlug || g.id === idOrSlug || (g.igdbId && `igdb-${g.igdbId}` === idOrSlug)
    )
    if (inLib) {
      if (inLib.inLibrary === undefined) inLib.inLibrary = true
      return inLib
    }

    // 2. Search in discovered cache
    if (discoveredGames.value.has(idOrSlug)) {
      return discoveredGames.value.get(idOrSlug)
    }

    // 3. Search in mock catalog fallback by slug or ID
    const foundBySlug = MOCK_IGDB_CATALOG.find(
      (g) => g.slug.toLowerCase() === idOrSlug.toLowerCase()
    )
    if (foundBySlug) {
      const mapped = mapIGDBGameToLoomGame(foundBySlug, {
        inLibrary: isGameInLibrary(foundBySlug.slug)
      })
      discoveredGames.value.set(mapped.slug, mapped)
      discoveredGames.value.set(mapped.id, mapped)
      discoveredGames.value.set(idOrSlug, mapped)
      return mapped
    }

    // 4. If ID starts with 'igdb-' or is numeric
    if (idOrSlug.startsWith('igdb-')) {
      const numericId = parseInt(idOrSlug.replace('igdb-', ''), 10)
      const foundInCatalog = MOCK_IGDB_CATALOG.find((g) => g.id === numericId)
      if (foundInCatalog) {
        const mapped = mapIGDBGameToLoomGame(foundInCatalog, {
          inLibrary: isGameInLibrary(foundInCatalog.slug)
        })
        discoveredGames.value.set(mapped.slug, mapped)
        discoveredGames.value.set(mapped.id, mapped)
        discoveredGames.value.set(idOrSlug, mapped)
        return mapped
      }
    }

    return undefined
  }

  async function fetchAndRegisterGame(idOrSlug: string): Promise<Game | undefined> {
    const existing = getGameById(idOrSlug) || getGameBySlug(idOrSlug)
    if (existing) {
      return enrichGameWithIGDB(existing)
    }

    // 1. Direct query by slug to IGDB
    try {
      const liveGame = await igdbService.getGameBySlug(idOrSlug)
      if (liveGame) {
        const mapped = mapIGDBGameToLoomGame(liveGame, {
          inLibrary: isGameInLibrary(liveGame.slug)
        })
        discoveredGames.value.set(mapped.slug, mapped)
        discoveredGames.value.set(mapped.id, mapped)
        discoveredGames.value.set(idOrSlug, mapped)
        return mapped
      }
    } catch (e) {
      console.warn('[gamesStore] Failed to fetch game by slug from IGDB:', e)
    }

    // 2. If it's an ID like 'igdb-12345'
    if (idOrSlug.startsWith('igdb-')) {
      const numericId = parseInt(idOrSlug.replace('igdb-', ''), 10)
      if (!isNaN(numericId)) {
        try {
          const liveGame = await igdbService.getGameById(numericId)
          if (liveGame) {
            const mapped = mapIGDBGameToLoomGame(liveGame, {
              inLibrary: isGameInLibrary(liveGame.slug)
            })
            discoveredGames.value.set(mapped.slug, mapped)
            discoveredGames.value.set(mapped.id, mapped)
            discoveredGames.value.set(idOrSlug, mapped)
            return mapped
          }
        } catch (e) {
          console.warn('[gamesStore] Failed to fetch live game from IGDB by ID:', e)
        }
      }
    }

    return undefined
  }

  /**
   * Associe et enrichit un jeu local ou Steam avec les métadonnées officielles IGDB
   */
  async function enrichGameWithIGDB(gameOrSlug: Game | string): Promise<Game | undefined> {
    const game =
      typeof gameOrSlug === 'string'
        ? getGameBySlug(gameOrSlug) || getGameById(gameOrSlug)
        : gameOrSlug
    if (!game) return undefined

    // Si le jeu est déjà enrichi avec IGDB
    if (game.igdbId && game.developer !== 'Valve / Steam Partner') {
      return game
    }

    // 0. Vérification prioritaire dans le catalogue enrichi (Aniimo, Meccha Chameleon, Spacewar, We Were Here, etc.)
    enrichWithCatalogIfAvailable(game)
    if (game.igdbId && game.developer !== 'Valve / Steam Partner') {
      saveLibrary()
      return game
    }

    try {
      // 1. Recherche par slug
      let igdbMatch = await igdbService.getGameBySlug(game.slug)

      // 2. Recherche par slug normalisé
      if (!igdbMatch && game.title) {
        const altSlug = game.title
          .toLowerCase()
          .normalize('NFD')
          .replace(/[\u0300-\u036f]/g, '')
          .replace(/[^a-z0-9]+/g, '-')
          .replace(/(^-|-$)/g, '')
        if (altSlug && altSlug !== game.slug) {
          igdbMatch = await igdbService.getGameBySlug(altSlug)
        }
      }

      // 3. Recherche textuelle IGDB par titre (uniquement si correspondance exacte pour éviter les associations incohérentes)
      if (!igdbMatch && game.title) {
        const results = await igdbService.searchGames(game.title, 5)
        if (results && results.length > 0) {
          const normTitle = game.title.toLowerCase().replace(/[^a-z0-9]/g, '')
          const exact = results.find(
            (r) =>
              r.name.toLowerCase() === game.title.toLowerCase() ||
              r.slug.toLowerCase() === game.slug.toLowerCase() ||
              r.name.toLowerCase().replace(/[^a-z0-9]/g, '') === normTitle
          )
          if (exact) {
            igdbMatch = exact
          }
        }
      }

      if (igdbMatch) {
        applyIGDBMatch(game, igdbMatch)
        saveLibrary()
        return game
      }
    } catch (e) {
      console.warn(`[gamesStore] Failed to enrich game ${game.id} with IGDB:`, e)
    }

    return game
  }

  function registerDiscoveredGame(game: Game) {
    discoveredGames.value.set(game.slug, game)
    discoveredGames.value.set(game.id, game)
    if (game.igdbId) {
      discoveredGames.value.set(`igdb-${game.igdbId}`, game)
    }
  }

  function addToLibrary(gameOrSlug: Game | string): Game | undefined {
    let gameToAdd: Game | undefined
    if (typeof gameOrSlug === 'string') {
      gameToAdd = discoveredGames.value.get(gameOrSlug) || getGameById(gameOrSlug)
    } else {
      gameToAdd = gameOrSlug
    }

    if (!gameToAdd) return undefined

    const existingIndex = games.value.findIndex(
      (g) => g.slug === gameToAdd!.slug || g.id === gameToAdd!.id
    )
    if (existingIndex >= 0) {
      games.value[existingIndex].inLibrary = true
      saveLibrary()
      return games.value[existingIndex]
    }

    gameToAdd.inLibrary = true
    games.value.unshift(gameToAdd)
    saveLibrary()
    return gameToAdd
  }

  function removeFromLibrary(slugOrId: string): boolean {
    const idx = games.value.findIndex((g) => g.slug === slugOrId || g.id === slugOrId)
    if (idx >= 0) {
      games.value[idx].inLibrary = false
      games.value.splice(idx, 1)
      saveLibrary()
      return true
    }
    return false
  }

  async function syncAchievementSummaries(): Promise<void> {
    try {
      const summaries = await achievementService.loadAllSummaries()
      let hasChanges = false
      for (const game of games.value) {
        const appid = game.id.replace('steam-', '')
        const summary = summaries.get(game.id) || summaries.get(appid)
        if (summary) {
          game.totalAchievementsCount = summary.totalCount
          game.unlockedAchievementsCount = summary.unlockedCount
          hasChanges = true
        }
      }
      if (hasChanges) {
        saveLibrary()
      }
    } catch (e) {
      console.warn('[gamesStore] Failed to sync achievement summaries:', e)
    }
  }

  async function loadGameAchievements(gameIdOrSlug: string): Promise<Achievement[]> {
    const game = getGameBySlug(gameIdOrSlug) || getGameById(gameIdOrSlug)
    if (!game) return []

    try {
      const achievements = await achievementService.getAchievementsForGame(game)
      if (achievements && achievements.length > 0) {
        game.achievements = achievements
        game.totalAchievementsCount = achievements.length
        game.unlockedAchievementsCount = achievements.filter((a) => a.unlocked).length
        saveLibrary()
      }
      return achievements
    } catch (e) {
      console.warn(`[gamesStore] Failed to load achievements for ${gameIdOrSlug}:`, e)
      return game.achievements || []
    }
  }

  async function fetchLibrary(): Promise<void> {
    isScanningLibrary.value = true
    try {
      const realGames = steamService.getLoomLibraryGames()
      games.value = realGames
      await syncAchievementSummaries()
      saveLibrary()
      lastScannedAt.value = new Date()
    } finally {
      isScanningLibrary.value = false
    }
  }

  function installGame(id: string) {
    const game = getGameById(id)
    if (!game) return
    if (game.platform === 'steam') {
      const appid = game.id.replace('steam-', '')
      const installUri = steamService.getSteamInstallUri(appid)
      window.open(installUri, '_blank')
    }
  }

  async function launchGame(id: string) {
    const game = getGameById(id)
    if (!game) return

    // If game is not installed, trigger install flow
    if (!game.installed) {
      installGame(id)
      return
    }

    isLaunching.value = true
    launchingGameId.value = id
    try {
      await ipc.invoke('launch_game', {
        gameId: id,
        runner: game.config.runner,
        gamemode: game.config.gamemode,
        mangohud: game.config.mangohud
      })

      // Check real Steam process state after slight delays
      setTimeout(checkRunningGames, 1200)
      setTimeout(checkRunningGames, 3000)
      setTimeout(checkRunningGames, 6000)

      // Timeout safety: if after 15s no process detected, clear starting indicator
      setTimeout(() => {
        if (launchingGameId.value === id && runningGameId.value !== id) {
          launchingGameId.value = null
        }
      }, 15000)
    } catch (err) {
      console.error('Failed to launch game via Steam:', err)
      launchingGameId.value = null
    } finally {
      isLaunching.value = false
    }
  }

  async function stopGame(id: string) {
    try {
      await ipc.invoke('stop_game', { gameId: id })
      setTimeout(checkRunningGames, 800)
      setTimeout(checkRunningGames, 2000)
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
      // Installation status filter
      if (filters.status && filters.status !== 'all') {
        if (filters.status === 'installed' && !game.installed) return false
        if (filters.status === 'uninstalled' && game.installed) return false
      }

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
        const order = filters.sortOrder === 'desc' ? -1 : 1
        return a.title.localeCompare(b.title, undefined, { sensitivity: 'base' }) * order
      } else if (filters.sortBy === 'playtime') {
        result = (b.playtimeMinutes || 0) - (a.playtimeMinutes || 0)
      } else if (filters.sortBy === 'size') {
        result = (b.sizeGb || 0) - (a.sizeGb || 0)
      } else {
        // 'lastPlayed' : tri par date réelle de dernier lancement (lastPlayedTimestamp)
        const aTime = a.lastPlayedTimestamp || 0
        const bTime = b.lastPlayedTimestamp || 0
        if (aTime !== bTime) {
          result = bTime - aTime
        } else {
          result = (b.playtimeMinutes || 0) - (a.playtimeMinutes || 0)
        }
      }
      return filters.sortOrder === 'desc' ? result : -result
    })
  }

  return {
    games,
    runningGameId,
    launchingGameId,
    checkRunningGames,
    isLaunching,
    isScanningLibrary,
    lastScannedAt,
    activeGame,
    heroGame,
    recentGames,
    installedGames,
    uninstalledGames,
    installedCount,
    uninstalledCount,
    totalCount,
    allNews,
    isGameInLibrary,
    getGameById,
    getGameBySlug,
    fetchAndRegisterGame,
    enrichGameWithIGDB,
    fetchLibrary,
    syncAchievementSummaries,
    loadGameAchievements,
    installGame,
    launchGame,
    stopGame,
    updateGameRunner,
    updateGameConfig,
    toggleFavorite,
    filterGames,
    discoveredGames,
    registerDiscoveredGame,
    addToLibrary,
    removeFromLibrary
  }
})
