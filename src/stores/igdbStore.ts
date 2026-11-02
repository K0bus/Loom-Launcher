import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { IGDBGame } from '@/models/igdb'
import { mapIGDBGameToLoomGame } from '@/models/igdb'
import { igdbService } from '@/services/igdbService'
import { useGamesStore } from '@/stores/gamesStore'
import { useToastStore } from '@/stores/toastStore'

export type IGDBTab = 'popular' | 'top_rated' | 'upcoming' | 'search'

export const useIgdbStore = defineStore('igdb', () => {
  const gamesStore = useGamesStore()
  const toast = useToastStore()

  // Credentials
  const credentials = ref(igdbService.getCredentials())

  // Navigation & results
  const activeTab = ref<IGDBTab>('popular')
  const searchQuery = ref('')
  const searchResults = ref<IGDBGame[]>([])
  const isSearching = ref(false)
  const selectedGame = ref<IGDBGame | null>(null)

  // Connection status info
  const isLive = ref(false)
  const connectionMessage = ref('')
  const isTesting = ref(false)

  const isConfigured = computed(() => Boolean(credentials.value.clientId))
  const hasToken = computed(() => Boolean(credentials.value.accessToken))

  /**
   * Load catalog according to active tab
   */
  async function loadCatalog(tab: IGDBTab = activeTab.value) {
    activeTab.value = tab
    isSearching.value = true

    try {
      if (tab === 'popular') {
        searchResults.value = await igdbService.getPopularGames(20)
      } else if (tab === 'top_rated') {
        searchResults.value = await igdbService.getTopRatedGames(20)
      } else if (tab === 'upcoming') {
        searchResults.value = await igdbService.getUpcomingGames(20)
      } else if (tab === 'search' && searchQuery.value.trim()) {
        searchResults.value = await igdbService.searchGames(searchQuery.value, 20)
      }
    } catch (err: any) {
      console.error('[IGDB Store] Error loading catalog:', err)
      toast.show(`Erreur IGDB : ${err.message}`, 'error')
    } finally {
      isSearching.value = false
    }
  }

  /**
   * Live search on IGDB
   */
  async function search(query: string) {
    searchQuery.value = query
    if (!query.trim()) {
      return loadCatalog('popular')
    }

    activeTab.value = 'search'
    isSearching.value = true
    try {
      searchResults.value = await igdbService.searchGames(query, 24)
    } catch (err: any) {
      toast.show(`Erreur de recherche : ${err.message}`, 'error')
    } finally {
      isSearching.value = false
    }
  }

  /**
   * Import an IGDB title directly into Loom Library
   */
  function importToLibrary(igdbGame: IGDBGame): boolean {
    const existing = gamesStore.games.find(
      (g) => g.igdbId === igdbGame.id || g.slug === igdbGame.slug || g.title.toLowerCase() === igdbGame.name.toLowerCase()
    )

    if (existing) {
      toast.show(`${igdbGame.name} est déjà présent dans votre bibliothèque !`, 'info')
      return false
    }

    const newGame = mapIGDBGameToLoomGame(igdbGame, {
      installed: false,
      isFavorite: false,
      playtimeMinutes: 0
    })

    gamesStore.games.unshift(newGame)
    toast.show(`${newGame.title} importé avec succès depuis l'API IGDB !`, 'success')
    return true
  }

  /**
   * Synchronize an existing game with fresh IGDB data
   */
  async function syncGameMetadata(loomGameId: string, igdbGame: IGDBGame): Promise<boolean> {
    const game = gamesStore.getGameById(loomGameId)
    if (!game) {
      toast.show('Jeu introuvable dans la bibliothèque.', 'error')
      return false
    }

    const updated = mapIGDBGameToLoomGame(igdbGame, {
      id: game.id,
      platform: game.platform,
      installed: game.installed,
      installPath: game.installPath,
      playtimeMinutes: game.playtimeMinutes,
      lastPlayed: game.lastPlayed,
      achievements: game.achievements,
      config: game.config,
      isFavorite: game.isFavorite
    })

    // Merge updated metadata
    Object.assign(game, updated)
    toast.show(`Métadonnées IGDB de ${game.title} synchronisées avec succès !`, 'success')
    return true
  }

  /**
   * Save API settings
   */
  function saveCredentials(clientId: string, accessToken?: string, clientSecret?: string) {
    igdbService.saveCredentials(clientId, accessToken, clientSecret)
    credentials.value = igdbService.getCredentials()
    toast.show('Identifiants IGDB enregistrés !', 'success')
  }

  /**
   * Request automatic Twitch token
   */
  async function generateTwitchToken(clientSecret: string) {
    try {
      const res = await igdbService.fetchTwitchToken(credentials.value.clientId, clientSecret)
      credentials.value = igdbService.getCredentials()
      toast.show('Nouveau Bearer Token Twitch généré et activé !', 'success')
      return res
    } catch (e: any) {
      toast.show(`Échec de génération du token : ${e.message}`, 'error')
      throw e
    }
  }

  /**
   * Test IGDB API connection
   */
  async function testConnection() {
    isTesting.value = true
    try {
      const res = await igdbService.testConnection()
      isLive.value = res.live
      connectionMessage.value = res.message
      if (res.success) {
        toast.show(res.message, res.live ? 'success' : 'info')
      } else {
        toast.show(res.message, 'error')
      }
      return res
    } finally {
      isTesting.value = false
    }
  }

  return {
    credentials,
    activeTab,
    searchQuery,
    searchResults,
    isSearching,
    selectedGame,
    isLive,
    connectionMessage,
    isTesting,
    isConfigured,
    hasToken,
    loadCatalog,
    search,
    importToLibrary,
    syncGameMetadata,
    saveCredentials,
    generateTwitchToken,
    testConnection
  }
})
