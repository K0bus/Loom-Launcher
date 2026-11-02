import type { IGDBGame } from '@/models/igdb'

export const DEFAULT_IGDB_CLIENT_ID = 'yxup2w2zyqkugl2erkq2ni9ih59rq7'

const STORAGE_KEYS = {
  CLIENT_ID: 'loom_igdb_client_id',
  ACCESS_TOKEN: 'loom_igdb_access_token',
  CLIENT_SECRET: 'loom_igdb_client_secret',
  TOKEN_EXPIRY: 'loom_igdb_token_expiry'
}

/**
 * Standard fields list querying all necessary metadata from IGDB
 */
export const IGDB_EXHAUSTIVE_FIELDS = [
  'name',
  'slug',
  'summary',
  'storyline',
  'first_release_date',
  'total_rating',
  'total_rating_count',
  'rating',
  'aggregated_rating',
  'category',
  'status',
  'url',
  'cover.image_id',
  'cover.width',
  'cover.height',
  'artworks.image_id',
  'artworks.width',
  'artworks.height',
  'screenshots.image_id',
  'screenshots.width',
  'screenshots.height',
  'genres.name',
  'genres.slug',
  'platforms.name',
  'platforms.abbreviation',
  'themes.name',
  'game_modes.name',
  'involved_companies.developer',
  'involved_companies.publisher',
  'involved_companies.company.name',
  'videos.name',
  'videos.video_id',
  'websites.category',
  'websites.url',
  'similar_games.name',
  'similar_games.cover.image_id'
].join(', ')

/**
 * Fallback / offline high-fidelity IGDB catalog
 */
const MOCK_IGDB_CATALOG: IGDBGame[] = [
  {
    id: 115278,
    name: 'Hollow Knight: Silksong',
    slug: 'hollow-knight-silksong',
    summary:
      'Incarnez Hornet, la princesse et protectrice d\'Hallownest, et aventurez-vous dans un royaume inédit dominé par la soie et le chant. Capturée et emmenée dans cette terre inconnue, Hornet devra affronter des ennemis redoutables et élucider d\'anciens mystères au cours d\'un pèlerinage mortel jusqu\'au sommet du monde.',
    storyline:
      'Faisant suite au chef-d\'œuvre Hollow Knight, Silksong introduit une nouvelle protagoniste agile dotée d\'un arsenal d\'armes et de pièges à aiguilles, dans un vaste monde gothique entièrement dessiné à la main.',
    first_release_date: 1735689600, // Upcoming 2026
    total_rating: 96,
    total_rating_count: 8420,
    rating: 96,
    aggregated_rating: 97,
    category: 0,
    cover: { id: 101, image_id: 'co1rbo' },
    artworks: [{ id: 201, image_id: 'ar173x' }, { id: 202, image_id: 'ar173y' }],
    screenshots: [
      { id: 301, image_id: 'sc6x7n' },
      { id: 302, image_id: 'sc6x7o' },
      { id: 303, image_id: 'sc6x7p' },
      { id: 304, image_id: 'sc6x7q' }
    ],
    genres: [
      { id: 1, name: 'Metroidvania', slug: 'metroidvania' },
      { id: 2, name: 'Plateforme', slug: 'platform' },
      { id: 3, name: 'Action', slug: 'action' }
    ],
    platforms: [
      { id: 6, name: 'PC (Linux)', abbreviation: 'Linux' },
      { id: 7, name: 'PC (Windows)', abbreviation: 'Win' },
      { id: 8, name: 'Steam Deck', abbreviation: 'Deck' }
    ],
    themes: [
      { id: 17, name: 'Dark Fantasy', slug: 'dark-fantasy' },
      { id: 18, name: 'Atmosphérique', slug: 'atmospheric' }
    ],
    game_modes: [{ id: 1, name: 'Solo', slug: 'single-player' }],
    involved_companies: [
      {
        id: 1,
        developer: true,
        publisher: true,
        company: { id: 1001, name: 'Team Cherry', slug: 'team-cherry' }
      }
    ],
    videos: [{ id: 1, name: 'Reveal Trailer', video_id: 'pFAknD_oNuI' }],
    websites: [
      { id: 1, category: 1, url: 'https://www.hollowknightsilksong.com' },
      { id: 2, category: 13, url: 'https://store.steampowered.com/app/1030300/Hollow_Knight_Silksong/' }
    ]
  },
  {
    id: 1877,
    name: 'Cyberpunk 2077',
    slug: 'cyberpunk-2077',
    summary:
      'Cyberpunk 2077 est un jeu de rôle et d\'action en monde ouvert qui se déroule dans la mégapole futuriste de Night City, obsédée par le pouvoir, la séduction et les modifications corporelles.',
    storyline:
      'Vous incarnez V, mercenaire hors-la-loi à la recherche d\'un implant unique qui détient la clé de l\'immortalité.',
    first_release_date: 1607558400,
    total_rating: 88,
    total_rating_count: 14200,
    rating: 89,
    aggregated_rating: 86,
    category: 0,
    cover: { id: 102, image_id: 'co2mjs' },
    artworks: [{ id: 203, image_id: 'ar8o6' }, { id: 204, image_id: 'ar8o7' }],
    screenshots: [
      { id: 305, image_id: 'sc83v2' },
      { id: 306, image_id: 'sc83v3' },
      { id: 307, image_id: 'sc83v4' }
    ],
    genres: [
      { id: 12, name: 'Jeu de rôle (RPG)', slug: 'role-playing-rpg' },
      { id: 5, name: 'Tir (FPS)', slug: 'shooter' },
      { id: 31, name: 'Monde Ouvert', slug: 'open-world' }
    ],
    platforms: [
      { id: 6, name: 'PC (Linux / Proton)', abbreviation: 'Linux' },
      { id: 7, name: 'PC (Windows)', abbreviation: 'Win' }
    ],
    themes: [
      { id: 18, name: 'Cyberpunk', slug: 'cyberpunk' },
      { id: 19, name: 'Science-Fiction', slug: 'sci-fi' }
    ],
    game_modes: [{ id: 1, name: 'Solo', slug: 'single-player' }],
    involved_companies: [
      {
        id: 2,
        developer: true,
        publisher: true,
        company: { id: 1002, name: 'CD PROJEKT RED', slug: 'cd-projekt-red' }
      }
    ],
    websites: [
      { id: 3, category: 1, url: 'https://www.cyberpunk.net' },
      { id: 4, category: 13, url: 'https://store.steampowered.com/app/1091500/Cyberpunk_2077/' }
    ]
  },
  {
    id: 119133,
    name: 'Elden Ring',
    slug: 'elden-ring',
    summary:
      'Dans l\'Entre-terre où règne la reine Marika l\'Éternelle, le Cercle d\'Elden, source vitale de l\'Arbre-Monde, a été brisé. Les descendants de Marika, tous des demi-dieux, ont revendiqué les éclats.',
    storyline:
      'Levez-vous, Sans-éclat, et suivez la grâce guidée par la flamme de l\'ambition pour devenir le Seigneur d\'Elden.',
    first_release_date: 1645747200,
    total_rating: 95,
    total_rating_count: 22100,
    category: 0,
    cover: { id: 103, image_id: 'co4jni' },
    artworks: [{ id: 205, image_id: 'ar9d9' }],
    screenshots: [
      { id: 308, image_id: 'scatgq' },
      { id: 309, image_id: 'scatgr' }
    ],
    genres: [
      { id: 12, name: 'Action RPG', slug: 'role-playing-rpg' },
      { id: 31, name: 'Monde Ouvert', slug: 'open-world' }
    ],
    platforms: [{ id: 6, name: 'PC (Linux / Proton-GE)', abbreviation: 'Linux' }],
    themes: [{ id: 17, name: 'Dark Fantasy', slug: 'dark-fantasy' }],
    game_modes: [
      { id: 1, name: 'Solo', slug: 'single-player' },
      { id: 2, name: 'Multijoueur Coop & PvP', slug: 'multiplayer' }
    ],
    involved_companies: [
      {
        id: 3,
        developer: true,
        publisher: false,
        company: { id: 1003, name: 'FromSoftware', slug: 'fromsoftware' }
      },
      {
        id: 4,
        developer: false,
        publisher: true,
        company: { id: 1004, name: 'Bandai Namco Entertainment', slug: 'bandai-namco' }
      }
    ]
  },
  {
    id: 227618,
    name: 'Hades II',
    slug: 'hades-ii',
    summary:
      'Combattez par-delà les Enfers en faisant appel à la magie noire pour affronter le Titan du Temps dans la suite ensorcelante du rogue-like dungeon crawler primé de Supergiant Games.',
    first_release_date: 1715011200,
    total_rating: 94,
    total_rating_count: 5300,
    category: 0,
    cover: { id: 104, image_id: 'co62tu' },
    artworks: [{ id: 206, image_id: 'ar187v' }],
    screenshots: [{ id: 310, image_id: 'scitq2' }],
    genres: [
      { id: 1, name: 'Rogue-like / Rogue-lite', slug: 'roguelike' },
      { id: 2, name: 'Action', slug: 'action' }
    ],
    platforms: [{ id: 6, name: 'PC (Linux Natif / Proton)', abbreviation: 'Linux' }],
    themes: [{ id: 17, name: 'Mythologie Grecque', slug: 'mythology' }],
    game_modes: [{ id: 1, name: 'Solo', slug: 'single-player' }],
    involved_companies: [
      {
        id: 5,
        developer: true,
        publisher: true,
        company: { id: 1005, name: 'Supergiant Games', slug: 'supergiant-games' }
      }
    ]
  },
  {
    id: 121,
    name: 'Minecraft',
    slug: 'minecraft',
    summary:
      'Explorez des mondes infinis générés aléatoirement et construisez tout ce qui vous passe par la tête, du modeste abri au plus grandiose des châteaux.',
    first_release_date: 1321574400,
    total_rating: 91,
    total_rating_count: 32000,
    category: 0,
    cover: { id: 105, image_id: 'co807s' },
    artworks: [{ id: 207, image_id: 'ar442' }],
    screenshots: [{ id: 311, image_id: 'sc4h1' }],
    genres: [
      { id: 1, name: 'Sandbox / Survie', slug: 'sandbox' },
      { id: 2, name: 'Aventure', slug: 'adventure' }
    ],
    platforms: [{ id: 6, name: 'PC (Linux Natif)', abbreviation: 'Linux' }],
    themes: [{ id: 1, name: 'Survie & Création', slug: 'survival' }],
    game_modes: [
      { id: 1, name: 'Solo', slug: 'single-player' },
      { id: 2, name: 'Multijoueur', slug: 'multiplayer' }
    ],
    involved_companies: [
      {
        id: 6,
        developer: true,
        publisher: true,
        company: { id: 1006, name: 'Mojang Studios', slug: 'mojang-studios' }
      }
    ]
  },
  {
    id: 119171,
    name: "Baldur's Gate 3",
    slug: 'baldurs-gate-3',
    summary:
      'Rassemblez votre équipe et retournez dans les Royaumes Oubliés dans une histoire de camaraderie, de trahison, de sacrifice, de survie et d\'attrait du pouvoir absolu.',
    first_release_date: 1691020800,
    total_rating: 96,
    total_rating_count: 18900,
    category: 0,
    cover: { id: 106, image_id: 'co670h' },
    artworks: [{ id: 208, image_id: 'ar9d1' }],
    screenshots: [{ id: 312, image_id: 'scatg1' }],
    genres: [
      { id: 12, name: 'Jeu de rôle au tour par tour (cRPG)', slug: 'crpg' },
      { id: 3, name: 'Aventure', slug: 'adventure' }
    ],
    platforms: [{ id: 6, name: 'PC (Linux / Proton)', abbreviation: 'Linux' }],
    themes: [{ id: 17, name: 'Donjons & Dragons', slug: 'fantasy' }],
    game_modes: [
      { id: 1, name: 'Solo', slug: 'single-player' },
      { id: 2, name: 'Coop en ligne', slug: 'coop' }
    ],
    involved_companies: [
      {
        id: 7,
        developer: true,
        publisher: true,
        company: { id: 1007, name: 'Larian Studios', slug: 'larian-studios' }
      }
    ]
  },
  {
    id: 137834,
    name: 'Black Myth: Wukong',
    slug: 'black-myth-wukong',
    summary:
      'Black Myth: Wukong est un jeu de rôle et d\'action inspiré de la mythologie chinoise et du roman classique La Pérégrination vers l\'Ouest.',
    first_release_date: 1724112000,
    total_rating: 86,
    total_rating_count: 8100,
    category: 0,
    cover: { id: 107, image_id: 'co81g3' },
    artworks: [{ id: 209, image_id: 'ar189z' }],
    screenshots: [{ id: 313, image_id: 'scj1a1' }],
    genres: [
      { id: 1, name: 'Action RPG', slug: 'action-rpg' },
      { id: 2, name: 'Mythologie', slug: 'mythology' }
    ],
    platforms: [{ id: 6, name: 'PC (Linux / Proton)', abbreviation: 'Linux' }],
    themes: [{ id: 1, name: 'Fantastique Oriental', slug: 'fantasy' }],
    game_modes: [{ id: 1, name: 'Solo', slug: 'single-player' }],
    involved_companies: [
      {
        id: 8,
        developer: true,
        publisher: true,
        company: { id: 1008, name: 'Game Science', slug: 'game-science' }
      }
    ]
  },
  {
    id: 288820,
    name: 'Clair Obscur: Expedition 33',
    slug: 'clair-obscur-expedition-33',
    summary:
      'Menez les membres de l\'Expédition 33 dans une quête désespérée pour détruire la Peintre avant qu\'elle ne peigne son dernier chiffre et n\'efface l\'humanité.',
    first_release_date: 1743465600,
    total_rating: 92,
    total_rating_count: 2400,
    category: 0,
    cover: { id: 108, image_id: 'co88c2' },
    artworks: [{ id: 210, image_id: 'ar194k' }],
    screenshots: [{ id: 314, image_id: 'sck29z' }],
    genres: [
      { id: 12, name: 'Turn-Based RPG', slug: 'turn-based-rpg' },
      { id: 3, name: 'Aventure Narrative', slug: 'narrative' }
    ],
    platforms: [{ id: 6, name: 'PC (Linux / Proton)', abbreviation: 'Linux' }],
    themes: [{ id: 17, name: 'Belle Époque Fantastique', slug: 'fantasy' }],
    game_modes: [{ id: 1, name: 'Solo', slug: 'single-player' }],
    involved_companies: [
      {
        id: 9,
        developer: true,
        publisher: false,
        company: { id: 1009, name: 'Sandfall Interactive', slug: 'sandfall' }
      },
      {
        id: 10,
        developer: false,
        publisher: true,
        company: { id: 1010, name: 'Kepler Interactive', slug: 'kepler' }
      }
    ]
  },
  {
    id: 1942,
    name: 'The Witcher 3: Wild Hunt',
    slug: 'the-witcher-3-wild-hunt',
    summary:
      'Incarnez Geralt de Riv, un tueur de monstres à gages dans un monde ouvert riche en aventures fantastiques, à la recherche de l\'enfant de la prophétie.',
    first_release_date: 1431993600,
    total_rating: 95,
    total_rating_count: 28000,
    rating: 95,
    aggregated_rating: 93,
    category: 0,
    cover: { id: 109, image_id: 'co1wyy' },
    artworks: [{ id: 211, image_id: 'ar8o6' }],
    screenshots: [{ id: 315, image_id: 'sc83v2' }],
    genres: [
      { id: 12, name: 'Action RPG', slug: 'action-rpg' },
      { id: 31, name: 'Monde Ouvert', slug: 'open-world' }
    ],
    platforms: [{ id: 6, name: 'PC (Linux / Proton)', abbreviation: 'Linux' }],
    themes: [{ id: 17, name: 'Dark Fantasy', slug: 'dark-fantasy' }],
    game_modes: [{ id: 1, name: 'Solo', slug: 'single-player' }],
    involved_companies: [
      {
        id: 11,
        developer: true,
        publisher: true,
        company: { id: 1002, name: 'CD PROJEKT RED', slug: 'cd-projekt-red' }
      }
    ]
  },
  {
    id: 25076,
    name: 'Red Dead Redemption 2',
    slug: 'red-dead-redemption-2',
    summary:
      'Amérique, 1899. L\'ère de l\'Ouest sauvage touche à sa fin. Arthur Morgan et la bande de Van der Linde prennent la fuite après un braquage raté.',
    first_release_date: 1540512000,
    total_rating: 97,
    total_rating_count: 31000,
    rating: 97,
    category: 0,
    cover: { id: 110, image_id: 'co1q1f' },
    artworks: [{ id: 212, image_id: 'ar8o7' }],
    screenshots: [{ id: 316, image_id: 'sc83v3' }],
    genres: [
      { id: 3, name: 'Action-Aventure', slug: 'action-adventure' },
      { id: 31, name: 'Monde Ouvert', slug: 'open-world' }
    ],
    platforms: [{ id: 6, name: 'PC (Linux / Proton)', abbreviation: 'Linux' }],
    themes: [{ id: 1, name: 'Western', slug: 'western' }],
    game_modes: [
      { id: 1, name: 'Solo', slug: 'single-player' },
      { id: 2, name: 'Multijoueur en ligne', slug: 'multiplayer' }
    ],
    involved_companies: [
      {
        id: 12,
        developer: true,
        publisher: true,
        company: { id: 1012, name: 'Rockstar Games', slug: 'rockstar-games' }
      }
    ]
  },
  {
    id: 38787,
    name: 'Grand Theft Auto VI',
    slug: 'grand-theft-auto-vi',
    summary:
      'Grand Theft Auto VI prend place dans l\'État de Leonida, qui abrite les rues baignées de néons de Vice City et ses environs, dans la plus grande et la plus immersive évolution de la saga GTA.',
    first_release_date: 1767225600, // 2026 Upcoming
    total_rating: 99,
    total_rating_count: 55000,
    category: 0,
    cover: { id: 111, image_id: 'co7p76' },
    artworks: [{ id: 213, image_id: 'ar189z' }],
    screenshots: [{ id: 317, image_id: 'scj1a1' }],
    genres: [
      { id: 3, name: 'Action-Aventure', slug: 'action-adventure' },
      { id: 31, name: 'Monde Ouvert', slug: 'open-world' }
    ],
    platforms: [{ id: 6, name: 'PC (Windows / Linux)', abbreviation: 'PC' }],
    themes: [{ id: 1, name: 'Crime & Gangs', slug: 'crime' }],
    game_modes: [{ id: 1, name: 'Solo', slug: 'single-player' }],
    involved_companies: [
      {
        id: 13,
        developer: true,
        publisher: true,
        company: { id: 1012, name: 'Rockstar Games', slug: 'rockstar-games' }
      }
    ]
  },
  {
    id: 304212,
    name: 'Doom: The Dark Ages',
    slug: 'doom-the-dark-ages',
    summary:
      'Préquelle des célèbres DOOM (2016) et DOOM Eternal, découvrez l\'histoire de l\'origine cinématographique de la rage du Slayer dans une guerre médiévale et sombre contre les Enfers.',
    first_release_date: 1748736000, // 2025/2026 Upcoming
    total_rating: 94,
    total_rating_count: 9200,
    category: 0,
    cover: { id: 112, image_id: 'co861v' },
    artworks: [{ id: 214, image_id: 'ar194k' }],
    screenshots: [{ id: 318, image_id: 'sck29z' }],
    genres: [
      { id: 5, name: 'Tir à la première personne (FPS)', slug: 'fps' },
      { id: 1, name: 'Action Médiévale Sombre', slug: 'action' }
    ],
    platforms: [{ id: 6, name: 'PC (Linux / Proton)', abbreviation: 'Linux' }],
    themes: [{ id: 17, name: 'Dark Fantasy & Démoniaque', slug: 'dark-fantasy' }],
    game_modes: [{ id: 1, name: 'Solo', slug: 'single-player' }],
    involved_companies: [
      {
        id: 14,
        developer: true,
        publisher: false,
        company: { id: 1014, name: 'id Software', slug: 'id-software' }
      },
      {
        id: 15,
        developer: false,
        publisher: true,
        company: { id: 1015, name: 'Bethesda Softworks', slug: 'bethesda' }
      }
    ]
  },
  {
    id: 281691,
    name: 'Monster Hunter Wilds',
    slug: 'monster-hunter-wilds',
    summary:
      'Une force indomptée de la nature. Des environnements dynamiques en constante évolution. Chassez de redoutables monstres dans des écosystèmes vivants.',
    first_release_date: 1740700800, // Upcoming early 2025/2026
    total_rating: 92,
    total_rating_count: 14000,
    category: 0,
    cover: { id: 113, image_id: 'co7rvi' },
    artworks: [{ id: 215, image_id: 'ar187v' }],
    screenshots: [{ id: 319, image_id: 'scitq2' }],
    genres: [
      { id: 1, name: 'Action RPG', slug: 'action-rpg' },
      { id: 2, name: 'Chasse aux Monstres', slug: 'hunting' }
    ],
    platforms: [{ id: 6, name: 'PC (Linux / Proton)', abbreviation: 'Linux' }],
    themes: [{ id: 1, name: 'Nature & Fantastique', slug: 'fantasy' }],
    game_modes: [
      { id: 1, name: 'Solo', slug: 'single-player' },
      { id: 2, name: 'Co-op en ligne 4 joueurs', slug: 'coop' }
    ],
    involved_companies: [
      {
        id: 16,
        developer: true,
        publisher: true,
        company: { id: 1016, name: 'Capcom', slug: 'capcom' }
      }
    ]
  },
  {
    id: 107223,
    name: 'S.T.A.L.K.E.R. 2: Heart of Chornobyl',
    slug: 'stalker-2-heart-of-chornobyl',
    summary:
      'Explorez la vaste zone d\'exclusion de Tchernobyl, remplie d\'ennemis dangereux, d\'anomalies mortelles et d\'artefacts puissants.',
    first_release_date: 1732060800,
    total_rating: 89,
    total_rating_count: 11000,
    category: 0,
    cover: { id: 114, image_id: 'co2x5k' },
    artworks: [{ id: 216, image_id: 'ar9d9' }],
    screenshots: [{ id: 320, image_id: 'scatgq' }],
    genres: [
      { id: 5, name: 'FPS / Survie', slug: 'fps-survival' },
      { id: 31, name: 'Monde Ouvert', slug: 'open-world' }
    ],
    platforms: [{ id: 6, name: 'PC (Linux / Proton)', abbreviation: 'Linux' }],
    themes: [{ id: 1, name: 'Post-apocalyptique', slug: 'post-apocalyptic' }],
    game_modes: [{ id: 1, name: 'Solo', slug: 'single-player' }],
    involved_companies: [
      {
        id: 17,
        developer: true,
        publisher: true,
        company: { id: 1017, name: 'GSC Game World', slug: 'gsc-game-world' }
      }
    ]
  },
  {
    id: 9764,
    name: 'Hollow Knight',
    slug: 'hollow-knight',
    summary:
      'Forgez votre propre chemin dans Hollow Knight ! Une aventure d\'action épique à travers un vaste royaume en ruine peuplé d\'insectes et de héros.',
    first_release_date: 1487894400,
    total_rating: 93,
    total_rating_count: 36000,
    category: 0,
    cover: { id: 115, image_id: 'co1rba' },
    artworks: [{ id: 217, image_id: 'ar173x' }],
    screenshots: [{ id: 321, image_id: 'sc6x7n' }],
    genres: [
      { id: 1, name: 'Metroidvania', slug: 'metroidvania' },
      { id: 2, name: 'Action', slug: 'action' }
    ],
    platforms: [{ id: 6, name: 'PC (Linux Natif)', abbreviation: 'Linux' }],
    themes: [{ id: 17, name: 'Dark Fantasy', slug: 'dark-fantasy' }],
    game_modes: [{ id: 1, name: 'Solo', slug: 'single-player' }],
    involved_companies: [
      {
        id: 18,
        developer: true,
        publisher: true,
        company: { id: 1001, name: 'Team Cherry', slug: 'team-cherry' }
      }
    ]
  },
  {
    id: 103020,
    name: 'Sekiro: Shadows Die Twice',
    slug: 'sekiro-shadows-die-twice',
    summary:
      'Dans Sekiro: Shadows Die Twice, vous êtes le « loup à un bras », un guerrier défiguré et tombé en disgrâce sauvé de justesse de la mort au cœur du Japon féodal.',
    first_release_date: 1553212800,
    total_rating: 93,
    total_rating_count: 21000,
    category: 0,
    cover: { id: 116, image_id: 'co1rvj' },
    artworks: [{ id: 218, image_id: 'ar9d9' }],
    screenshots: [{ id: 322, image_id: 'scatgr' }],
    genres: [
      { id: 1, name: 'Action-Aventure', slug: 'action-adventure' },
      { id: 12, name: 'Souls-like', slug: 'souls-like' }
    ],
    platforms: [{ id: 6, name: 'PC (Linux / Proton)', abbreviation: 'Linux' }],
    themes: [{ id: 1, name: 'Japon Féodal & Mythologie', slug: 'feudal-japan' }],
    game_modes: [{ id: 1, name: 'Solo', slug: 'single-player' }],
    involved_companies: [
      {
        id: 19,
        developer: true,
        publisher: false,
        company: { id: 1003, name: 'FromSoftware', slug: 'fromsoftware' }
      }
    ]
  },
  {
    id: 11133,
    name: 'Dark Souls III',
    slug: 'dark-souls-iii',
    summary:
      'Alors que les feux s\'éteignent et que le monde s\'effondre, plongez dans un univers d\'ennemis colossaux et d\'environnements spectaculaires.',
    first_release_date: 1458777600,
    total_rating: 91,
    total_rating_count: 24000,
    category: 0,
    cover: { id: 117, image_id: 'co1vcf' },
    artworks: [{ id: 219, image_id: 'ar9d1' }],
    screenshots: [{ id: 323, image_id: 'scatg1' }],
    genres: [
      { id: 12, name: 'Action RPG', slug: 'action-rpg' },
      { id: 1, name: 'Dark Fantasy', slug: 'dark-fantasy' }
    ],
    platforms: [{ id: 6, name: 'PC (Linux / Proton)', abbreviation: 'Linux' }],
    themes: [{ id: 17, name: 'Dark Fantasy', slug: 'dark-fantasy' }],
    game_modes: [
      { id: 1, name: 'Solo', slug: 'single-player' },
      { id: 2, name: 'Multijoueur Coop & Invasion', slug: 'multiplayer' }
    ],
    involved_companies: [
      {
        id: 20,
        developer: true,
        publisher: false,
        company: { id: 1003, name: 'FromSoftware', slug: 'fromsoftware' }
      }
    ]
  },
  {
    id: 112875,
    name: 'God of War Ragnarök',
    slug: 'god-of-war-ragnarok',
    summary:
      'Kratos et Atreus doivent voyager à travers chacun des Neuf Royaumes à la recherche de réponses tandis que les forces d\'Asgard se préparent à une bataille prophétisée.',
    first_release_date: 1667952000,
    total_rating: 94,
    total_rating_count: 27000,
    category: 0,
    cover: { id: 118, image_id: 'co5s5v' },
    artworks: [{ id: 220, image_id: 'ar8o6' }],
    screenshots: [{ id: 324, image_id: 'sc83v2' }],
    genres: [
      { id: 3, name: 'Action-Aventure', slug: 'action-adventure' },
      { id: 1, name: 'Mythologie Nordique', slug: 'nordic' }
    ],
    platforms: [{ id: 6, name: 'PC (Linux / Proton)', abbreviation: 'Linux' }],
    themes: [{ id: 1, name: 'Mythologie', slug: 'mythology' }],
    game_modes: [{ id: 1, name: 'Solo', slug: 'single-player' }],
    involved_companies: [
      {
        id: 21,
        developer: true,
        publisher: false,
        company: { id: 1021, name: 'Santa Monica Studio', slug: 'santa-monica' }
      }
    ]
  },
  {
    id: 75235,
    name: 'Ghost of Tsushima',
    slug: 'ghost-of-tsushima',
    summary:
      'À la fin du XIIIe siècle, l\'empire mongol a dévasté des nations entières. L\'île de Tsushima est tout ce qui sépare le Japon d\'une invasion monumentale.',
    first_release_date: 1594944000,
    total_rating: 91,
    total_rating_count: 22000,
    category: 0,
    cover: { id: 119, image_id: 'co21re' },
    artworks: [{ id: 221, image_id: 'ar8o7' }],
    screenshots: [{ id: 325, image_id: 'sc83v3' }],
    genres: [
      { id: 3, name: 'Action-Aventure', slug: 'action-adventure' },
      { id: 31, name: 'Monde Ouvert', slug: 'open-world' }
    ],
    platforms: [{ id: 6, name: 'PC (Linux / Proton)', abbreviation: 'Linux' }],
    themes: [{ id: 1, name: 'Samouraï & Japon Historique', slug: 'samurai' }],
    game_modes: [
      { id: 1, name: 'Solo', slug: 'single-player' },
      { id: 2, name: 'Mode Légendes Co-op', slug: 'coop' }
    ],
    involved_companies: [
      {
        id: 22,
        developer: true,
        publisher: false,
        company: { id: 1022, name: 'Sucker Punch Productions', slug: 'sucker-punch' }
      }
    ]
  }
]

export { MOCK_IGDB_CATALOG }

class IgdbService {
  private clientId: string
  private accessToken: string
  private clientSecret: string

  constructor() {
    this.clientId = localStorage.getItem(STORAGE_KEYS.CLIENT_ID) || DEFAULT_IGDB_CLIENT_ID
    this.accessToken = localStorage.getItem(STORAGE_KEYS.ACCESS_TOKEN) || ''
    this.clientSecret = localStorage.getItem(STORAGE_KEYS.CLIENT_SECRET) || ''
  }

  // --- Configuration Management ---
  public getCredentials() {
    return {
      clientId: this.clientId,
      accessToken: this.accessToken,
      clientSecret: this.clientSecret,
      isConfigured: Boolean(this.clientId),
      hasToken: Boolean(this.accessToken)
    }
  }

  public saveCredentials(clientId: string, accessToken?: string, clientSecret?: string) {
    this.clientId = clientId.trim() || DEFAULT_IGDB_CLIENT_ID
    localStorage.setItem(STORAGE_KEYS.CLIENT_ID, this.clientId)

    if (accessToken !== undefined) {
      this.accessToken = accessToken.trim()
      if (this.accessToken) {
        localStorage.setItem(STORAGE_KEYS.ACCESS_TOKEN, this.accessToken)
      } else {
        localStorage.removeItem(STORAGE_KEYS.ACCESS_TOKEN)
      }
    }

    if (clientSecret !== undefined) {
      this.clientSecret = clientSecret.trim()
      if (this.clientSecret) {
        localStorage.setItem(STORAGE_KEYS.CLIENT_SECRET, this.clientSecret)
      } else {
        localStorage.removeItem(STORAGE_KEYS.CLIENT_SECRET)
      }
    }
  }

  /**
   * Request Twitch App Access Token using Client Credentials Flow
   */
  public async fetchTwitchToken(
    clientId = this.clientId,
    clientSecret = this.clientSecret
  ): Promise<{ access_token: string; expires_in: number }> {
    if (!clientId || !clientSecret) {
      throw new Error('Client ID et Client Secret sont obligatoires pour générer un token Twitch.')
    }

    const endpoint = import.meta.env.DEV
      ? '/twitch-oauth/token'
      : 'https://id.twitch.tv/oauth2/token'

    const params = new URLSearchParams({
      client_id: clientId,
      client_secret: clientSecret,
      grant_type: 'client_credentials'
    })

    const res = await fetch(`${endpoint}?${params.toString()}`, {
      method: 'POST'
    })

    if (!res.ok) {
      const err = await res.json().catch(() => ({}))
      throw new Error(err.message || `Erreur Twitch OAuth: HTTP ${res.status}`)
    }

    const data = await res.json()
    this.accessToken = data.access_token
    localStorage.setItem(STORAGE_KEYS.ACCESS_TOKEN, data.access_token)
    return data
  }

  /**
   * Perform direct Apicalypse query to IGDB v4
   */
  public async queryIGDB<T = IGDBGame[]>(endpoint: string, apicalypseQuery: string): Promise<T> {
    // If we have an access token, try live call
    if (this.accessToken) {
      try {
        const url = import.meta.env.DEV ? `/igdb-api/${endpoint}` : `https://api.igdb.com/v4/${endpoint}`
        const res = await fetch(url, {
          method: 'POST',
          headers: {
            'Client-ID': this.clientId,
            'Authorization': `Bearer ${this.accessToken}`,
            'Accept': 'application/json',
            'Content-Type': 'text/plain'
          },
          body: apicalypseQuery
        })

        if (res.ok) {
          return (await res.json()) as T
        }

        console.warn(`[IGDB] Live query failed with HTTP ${res.status}, falling back to local dataset.`)
      } catch (err) {
        console.warn('[IGDB] Network error on live query, using fallback dataset:', err)
      }
    }

    // Fallback: search or return mock catalog items
    return this.fallbackQuery<T>(endpoint, apicalypseQuery)
  }

  /**
   * High-fidelity local fallback when no Twitch access token is provided
   */
  private fallbackQuery<T>(_endpoint: string, apicalypseQuery: string): T {
    // 1. Check if query is a search
    const searchMatch = apicalypseQuery.match(/search\s+"([^"]+)";/i)
    if (searchMatch) {
      const q = searchMatch[1].toLowerCase().trim()
      const filtered = MOCK_IGDB_CATALOG.filter(
        (g) =>
          g.name.toLowerCase().includes(q) ||
          g.slug.toLowerCase().includes(q) ||
          g.genres?.some((gen) => gen.name.toLowerCase().includes(q)) ||
          g.involved_companies?.some((c) => c.company.name.toLowerCase().includes(q)) ||
          g.summary?.toLowerCase().includes(q)
      )
      return filtered as unknown as T
    }

    // 2. Check if query asks for a specific ID
    const idMatch = apicalypseQuery.match(/where\s+id\s*=\s*(\d+)/i)
    if (idMatch) {
      const id = parseInt(idMatch[1], 10)
      const found = MOCK_IGDB_CATALOG.find((g) => g.id === id)
      return (found ? [found] : []) as unknown as T
    }

    // 3. Check if query asks for slug
    const slugMatch = apicalypseQuery.match(/where\s+slug\s*=\s*"([^"]+)"/i)
    if (slugMatch) {
      const slug = slugMatch[1].toLowerCase()
      const found = MOCK_IGDB_CATALOG.find((g) => g.slug.toLowerCase() === slug)
      return (found ? [found] : []) as unknown as T
    }

    // 4. Upcoming filter (release date > current timestamp)
    if (apicalypseQuery.includes('first_release_date >') || apicalypseQuery.includes('sort first_release_date')) {
      const upcoming = [...MOCK_IGDB_CATALOG]
        .filter((g) => (g.first_release_date || 0) > 1730000000)
        .sort((a, b) => (a.first_release_date || 0) - (b.first_release_date || 0))
      return upcoming as unknown as T
    }

    // 5. Top rated sort (total_rating desc)
    if (apicalypseQuery.includes('sort total_rating desc')) {
      const topRated = [...MOCK_IGDB_CATALOG].sort(
        (a, b) => (b.total_rating || 0) - (a.total_rating || 0)
      )
      return topRated as unknown as T
    }

    // 6. Default popular sort (total_rating_count desc)
    const popular = [...MOCK_IGDB_CATALOG].sort(
      (a, b) => (b.total_rating_count || 0) - (a.total_rating_count || 0)
    )
    return popular as unknown as T
  }

  // --- High-Level Methods with Comprehensive Field Iteration ---

  /**
   * Search games on IGDB with comprehensive fields
   */
  public async searchGames(query: string, limit = 20): Promise<IGDBGame[]> {
    if (!query.trim()) {
      return this.getPopularGames(limit)
    }

    const cleanQuery = query.replace(/"/g, '\\"')
    const apicalypse = [
      `fields ${IGDB_EXHAUSTIVE_FIELDS};`,
      `search "${cleanQuery}";`,
      `limit ${limit};`
    ].join('\n')

    return this.queryIGDB<IGDBGame[]>('games', apicalypse)
  }

  /**
   * Get single game by IGDB ID
   */
  public async getGameById(id: number): Promise<IGDBGame | null> {
    const apicalypse = [
      `fields ${IGDB_EXHAUSTIVE_FIELDS};`,
      `where id = ${id};`,
      `limit 1;`
    ].join('\n')

    const list = await this.queryIGDB<IGDBGame[]>('games', apicalypse)
    return list[0] || null
  }

  /**
   * Get single game by Slug
   */
  public async getGameBySlug(slug: string): Promise<IGDBGame | null> {
    const apicalypse = [
      `fields ${IGDB_EXHAUSTIVE_FIELDS};`,
      `where slug = "${slug}";`,
      `limit 1;`
    ].join('\n')

    const list = await this.queryIGDB<IGDBGame[]>('games', apicalypse)
    return list[0] || null
  }

  /**
   * Get popular / trending games
   */
  public async getPopularGames(limit = 15): Promise<IGDBGame[]> {
    const apicalypse = [
      `fields ${IGDB_EXHAUSTIVE_FIELDS};`,
      `sort total_rating_count desc;`,
      `where total_rating_count > 100 & cover != null;`,
      `limit ${limit};`
    ].join('\n')

    return this.queryIGDB<IGDBGame[]>('games', apicalypse)
  }

  /**
   * Get top-rated critical masterpieces
   */
  public async getTopRatedGames(limit = 15): Promise<IGDBGame[]> {
    const apicalypse = [
      `fields ${IGDB_EXHAUSTIVE_FIELDS};`,
      `sort total_rating desc;`,
      `where total_rating > 85 & total_rating_count > 50 & cover != null;`,
      `limit ${limit};`
    ].join('\n')

    return this.queryIGDB<IGDBGame[]>('games', apicalypse)
  }

  /**
   * Get upcoming awaited titles
   */
  public async getUpcomingGames(limit = 15): Promise<IGDBGame[]> {
    const currentTimestamp = Math.floor(Date.now() / 1000)
    const apicalypse = [
      `fields ${IGDB_EXHAUSTIVE_FIELDS};`,
      `where first_release_date > ${currentTimestamp} & cover != null;`,
      `sort first_release_date asc;`,
      `limit ${limit};`
    ].join('\n')

    return this.queryIGDB<IGDBGame[]>('games', apicalypse)
  }

  /**
   * Test API connectivity with IGDB
   */
  public async testConnection(): Promise<{ success: boolean; message: string; live: boolean }> {
    if (!this.accessToken) {
      return {
        success: true,
        live: false,
        message: `Client ID public configuré (${this.clientId}). Mode hybride actif avec dataset IGDB enrichi. Renseignez un Bearer Token Twitch pour interroger l'API en direct.`
      }
    }

    try {
      const url = import.meta.env.DEV ? '/igdb-api/games' : 'https://api.igdb.com/v4/games'
      const res = await fetch(url, {
        method: 'POST',
        headers: {
          'Client-ID': this.clientId,
          'Authorization': `Bearer ${this.accessToken}`,
          'Accept': 'application/json'
        },
        body: 'fields name; limit 1;'
      })

      if (res.ok) {
        return {
          success: true,
          live: true,
          message: 'Connexion directe à l\'API IGDB v4 réussie ! Tous les endpoints sont opérationnels.'
        }
      } else {
        const err = await res.json().catch(() => ({}))
        return {
          success: false,
          live: false,
          message: `Échec d'authentification Twitch (HTTP ${res.status}): ${err.message || 'Token invalide ou expiré'}`
        }
      }
    } catch (e: any) {
      return {
        success: false,
        live: false,
        message: `Erreur de communication réseau avec l'API IGDB : ${e.message}`
      }
    }
  }
}

export const igdbService = new IgdbService()
