import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type {
  MinecraftInstance,
  ModpackCatalogItem,
  MinecraftLoader
} from '@/models/minecraft'
import { ipc } from '@/services/ipc'

export const useMinecraftStore = defineStore('minecraft', () => {
  // Existing Instances
  const instances = ref<MinecraftInstance[]>([
    {
      id: 'inst-atm9',
      name: 'All The Mods 9 - To the Skies',
      icon: '🚀',
      gameVersion: '1.20.1',
      loader: 'neoforge',
      loaderVersion: 'NeoForge 47.1.106',
      playtimeMinutes: 4890,
      lastPlayed: 'Hier',
      allocatedRamGb: 8,
      status: 'idle',
      mods: [
        {
          id: 'mod-jei',
          name: 'Just Enough Items (JEI)',
          version: '15.3.0.4',
          author: 'mezz',
          enabled: true,
          description: 'Visualiseur complet de recettes d\'artisanat et d\'utilisations d\'objets.',
          updateAvailable: false,
          fileName: 'jei-1.20.1-15.3.0.4.jar',
          fileSizeMb: 1.4
        },
        {
          id: 'mod-sodium',
          name: 'Embeddium / Sodium Engine',
          version: '0.3.31',
          author: 'embeddedt',
          enabled: true,
          description: 'Moteur de rendu moderne décuplant les FPS et la stabilité sur GPU AMD & NVIDIA sous Linux.',
          updateAvailable: true,
          newVersion: '0.3.32',
          fileName: 'embeddium-0.3.31+mc1.20.1.jar',
          fileSizeMb: 2.1
        },
        {
          id: 'mod-create',
          name: 'Create',
          version: '0.5.1f',
          author: 'simibubi',
          enabled: true,
          description: 'Système cinématique d\'automatisation esthétique basé sur l\'énergie rotative.',
          updateAvailable: false,
          fileName: 'create-1.20.1-0.5.1.f.jar',
          fileSizeMb: 16.8
        },
        {
          id: 'mod-mekanism',
          name: 'Mekanism',
          version: '10.4.8',
          author: 'aidancbrady',
          enabled: true,
          description: 'Suite technologique industrielle de haute précision (purification de minerais x5, fusion nucléaire).',
          updateAvailable: false,
          fileName: 'Mekanism-1.20.1-10.4.8.jar',
          fileSizeMb: 14.2
        },
        {
          id: 'mod-oculus',
          name: 'Oculus (Iris Shaders)',
          version: '1.7.0',
          author: 'Asek3',
          enabled: true,
          description: 'Support matériel des shaders GLSL ultra-rapide compatible avec Embeddium.',
          updateAvailable: false,
          fileName: 'oculus-mc1.20.1-1.7.0.jar',
          fileSizeMb: 3.5
        },
        {
          id: 'mod-journeymap',
          name: 'JourneyMap',
          version: '5.9.18',
          author: 'techbrew',
          enabled: false,
          description: 'Cartographie en temps réel en jeu ou dans votre navigateur web local.',
          updateAvailable: false,
          fileName: 'journeymap-1.20.1-5.9.18.jar',
          fileSizeMb: 5.7
        }
      ],
      worlds: [
        {
          id: 'w-1',
          name: 'Skyblock Mega Complex',
          lastPlayed: 'Hier à 21:40',
          sizeMb: 184.2,
          gameMode: 'survival'
        },
        {
          id: 'w-2',
          name: 'Laboratoire Test Machines',
          lastPlayed: 'Il y a 5 jours',
          sizeMb: 42.1,
          gameMode: 'creative'
        }
      ],
      servers: [
        {
          id: 'srv-atm9-official',
          name: 'ATM9 Official Public Server',
          ip: 'atm9.allthemods.net',
          motd: 'All The Mods 9 — To the Skies [v1.20.1] • Quests & Economy',
          iconUrl: 'https://images.unsplash.com/photo-1618005182384-a83a8bd57fbe?q=80&w=120&auto=format&fit=crop',
          pingMs: 22,
          playersOnline: 142,
          playersMax: 250,
          status: 'online',
          version: 'NeoForge 1.20.1'
        },
        {
          id: 'srv-k0bus-realm',
          name: 'K0bus Modded Realm (Privé)',
          ip: 'play.k0bus-realm.fr:25565',
          motd: 'Serveur Survie ATM9 entre amis • Sauvegarde auto 6h',
          iconUrl: 'https://images.unsplash.com/photo-1542751371-adc38448a05e?q=80&w=120&auto=format&fit=crop',
          pingMs: 14,
          playersOnline: 5,
          playersMax: 20,
          status: 'online',
          version: 'NeoForge 1.20.1'
        },
        {
          id: 'srv-eu-community',
          name: 'EU Tech-Magic Network',
          ip: 'eu.techmagic-mc.com',
          motd: 'Maintenance programmée ce soir à 23h • Chunkloaded',
          pingMs: 46,
          playersOnline: 28,
          playersMax: 80,
          status: 'online',
          version: '1.20.1'
        }
      ],
      resourcePacks: [
        {
          id: 'rp-1',
          name: 'Faithful 32x HD',
          description: 'Textures vanilla sublimées en haute définition 32x32.',
          enabled: true,
          formatVersion: 15
        },
        {
          id: 'rp-2',
          name: 'Stay True (Biome Foliage)',
          description: 'Ajout de buissons, de feuilles volumétriques et de variations de textures.',
          enabled: true,
          formatVersion: 15
        }
      ],
      shaderPacks: [
        {
          id: 'sp-1',
          name: 'Complementary Reimagined r5.1.1',
          profile: 'Ultra (avec Ray Tracing simulé)',
          enabled: true,
          authors: 'EminGT'
        },
        {
          id: 'sp-2',
          name: 'BSL Shaders v8.2',
          profile: 'High',
          enabled: false,
          authors: 'Capt Tatsu'
        }
      ],
      logs: [
        { timestamp: '21:40:02', level: 'INFO', source: 'MinecraftClient', message: 'Setting user: K0bus_Gamer' },
        { timestamp: '21:40:04', level: 'INFO', source: 'NeoForge', message: 'NeoForge architecture initialized with 148 active mods' },
        { timestamp: '21:40:07', level: 'INFO', source: 'Embeddium', message: 'OpenGL 4.6 Core Profile context created via Mesa RADV' },
        { timestamp: '21:40:11', level: 'WARN', source: 'Oculus', message: 'Shader pack loaded with custom pipeline configuration' },
        { timestamp: '21:40:14', level: 'INFO', source: 'GameRenderer', message: 'World "Skyblock Mega Complex" loaded in 2.84s (FPS: 165)' }
      ]
    },
    {
      id: 'inst-vanilla-plus',
      name: 'Vanilla+ Performance 1.21',
      icon: '⚡',
      gameVersion: '1.21.4',
      loader: 'fabric',
      loaderVersion: 'Fabric Loader 0.16.9',
      playtimeMinutes: 1240,
      lastPlayed: 'Il y a 3 jours',
      allocatedRamGb: 6,
      status: 'idle',
      mods: [
        {
          id: 'mod-sodium-fab',
          name: 'Sodium',
          version: '0.6.0',
          author: 'JellySquid',
          enabled: true,
          description: 'Modern rendering engine for Minecraft Fabric.',
          updateAvailable: false,
          fileName: 'sodium-fabric-0.6.0.jar',
          fileSizeMb: 1.8
        },
        {
          id: 'mod-lithium',
          name: 'Lithium',
          version: '0.13.0',
          author: 'JellySquid',
          enabled: true,
          description: 'Optimisation de la physique du serveur, de l\'IA des entités et des chunks.',
          updateAvailable: false,
          fileName: 'lithium-fabric-0.13.0.jar',
          fileSizeMb: 0.9
        }
      ],
      worlds: [
        {
          id: 'w-vanilla',
          name: 'Survie 1.21 Hardcore',
          lastPlayed: 'Il y a 3 jours',
          sizeMb: 95.0,
          gameMode: 'hardcore'
        }
      ],
      servers: [
        {
          id: 'srv-vanilla-smp',
          name: 'SMP Vanilla 1.21.4 Pur',
          ip: 'smp.vanilla-friends.eu',
          motd: 'Survie multijoueur 1.21.4 sans reset • Whitelist active',
          iconUrl: 'https://images.unsplash.com/photo-1627856013091-fed6e4e30025?q=80&w=120&auto=format&fit=crop',
          pingMs: 18,
          playersOnline: 12,
          playersMax: 30,
          status: 'online',
          version: 'Fabric 1.21.4'
        },
        {
          id: 'srv-hypixel',
          name: 'Hypixel Network',
          ip: 'mc.hypixel.net',
          motd: 'Hypixel Network [1.8-1.21] • BedWars, SkyBlock & More',
          iconUrl: 'https://images.unsplash.com/photo-1607604276583-eef5d076aa5f?q=80&w=120&auto=format&fit=crop',
          pingMs: 34,
          playersOnline: 48320,
          playersMax: 100000,
          status: 'online',
          version: '1.21.4'
        },
        {
          id: 'srv-local-test',
          name: 'Serveur Local Développeur',
          ip: '127.0.0.1:25565',
          motd: 'Serveur local de test (hors ligne)',
          pingMs: 0,
          playersOnline: 0,
          playersMax: 10,
          status: 'offline',
          version: 'Local'
        }
      ],
      resourcePacks: [],
      shaderPacks: [],
      logs: []
    }
  ])

  // Modpack Catalog (Modrinth + CurseForge)
  const catalog = ref<ModpackCatalogItem[]>([
    {
      id: 'mp-prominence-2',
      slug: 'prominence-2-rpg',
      title: 'Prominence II [RPG]',
      author: 'LunapixelStudios',
      downloads: 1840000,
      likes: 42300,
      iconUrl: 'https://images.unsplash.com/photo-1579783902614-a3fb3927b675?q=80&w=300&auto=format&fit=crop',
      bannerUrl: 'https://images.unsplash.com/photo-1518709268805-4e9042af9f23?q=80&w=1200&auto=format&fit=crop',
      summary: 'L\'expérience RPG ultime avec arbre de talents complet, donjons épiques, système de combat d\'action et plus de 300 quêtes scénarisées.',
      loader: 'fabric',
      mcVersion: '1.20.1',
      source: 'modrinth',
      categories: ['RPG', 'Quêtes', 'Combat', 'Magie']
    },
    {
      id: 'mp-atm9',
      slug: 'all-the-mods-9',
      title: 'All the Mods 9 - ATM9',
      author: 'ATMTeam',
      downloads: 4210000,
      likes: 91400,
      iconUrl: 'https://images.unsplash.com/photo-1618005182384-a83a8bd57fbe?q=80&w=300&auto=format&fit=crop',
      bannerUrl: 'https://images.unsplash.com/photo-1542751371-adc38448a05e?q=80&w=1200&auto=format&fit=crop',
      summary: 'Le standard indétrônable des kitchen-sink modpacks avec plus de 400 mods, équilibrage complet et l\'étoile ATM à fabriquer.',
      loader: 'neoforge',
      mcVersion: '1.20.1',
      source: 'curseforge',
      categories: ['Tech', 'Magie', 'Kitchen Sink', 'Quêtes']
    },
    {
      id: 'mp-better-mc',
      slug: 'better-mc-forge',
      title: 'Better MC [FORGE] 1.20.1',
      author: 'SHXRKIE',
      downloads: 7890000,
      likes: 125000,
      iconUrl: 'https://images.unsplash.com/photo-1550745165-9bc0b252726f?q=80&w=300&auto=format&fit=crop',
      bannerUrl: 'https://images.unsplash.com/photo-1551103782-8ab07afd45c1?q=80&w=1200&auto=format&fit=crop',
      summary: 'Ce à quoi Minecraft 2 aurait dû ressembler. Nouveaux boss, dimensions mystérieuses, animaux réalistes et interface raffinée.',
      loader: 'forge',
      mcVersion: '1.20.1',
      source: 'curseforge',
      categories: ['Aventure', 'Exploration', 'Vanilla+']
    },
    {
      id: 'mp-fabulously-optimized',
      slug: 'fabulously-optimized',
      title: 'Fabulously Optimized',
      author: 'RobotKoer',
      downloads: 3200000,
      likes: 67000,
      iconUrl: 'https://images.unsplash.com/photo-1627856013091-fed6e4e30025?q=80&w=300&auto=format&fit=crop',
      bannerUrl: 'https://images.unsplash.com/photo-1607604276583-eef5d076aa5f?q=80&w=1200&auto=format&fit=crop',
      summary: 'Modpack Fabric léger axé sur les performances maximales et la parité complète avec les fonctionnalités OptiFine (Capes, Shaders, Zoom).',
      loader: 'fabric',
      mcVersion: '1.21.4',
      source: 'modrinth',
      categories: ['Optimisation', 'Shaders', 'Vanilla+']
    }
  ])

  // Active Instance selection
  const selectedInstanceId = ref<string>('inst-atm9')

  const activeInstance = computed(() => {
    return instances.value.find((i) => i.id === selectedInstanceId.value) || instances.value[0]
  })

  // Actions
  function selectInstance(id: string) {
    selectedInstanceId.value = id
  }

  function toggleMod(instanceId: string, modId: string) {
    const inst = instances.value.find((i) => i.id === instanceId)
    if (!inst) return
    const mod = inst.mods.find((m) => m.id === modId)
    if (mod) {
      mod.enabled = !mod.enabled
    }
  }

  async function launchInstance(instanceId: string) {
    const inst = instances.value.find((i) => i.id === instanceId)
    if (!inst) return
    inst.status = 'running'
    inst.logs.push({
      timestamp: new Date().toLocaleTimeString(),
      level: 'INFO',
      source: 'LoomLauncher',
      message: `Démarrage de l'instance "${inst.name}" avec Java 21 (${inst.allocatedRamGb} Go RAM alloués)`
    })

    try {
      await ipc.invoke('launch_game', {
        gameId: `minecraft:${instanceId}`,
        runner: 'native',
        allocatedRam: inst.allocatedRamGb
      })
    } catch (err) {
      console.error(err)
    }
  }

  async function stopInstance(instanceId: string) {
    const inst = instances.value.find((i) => i.id === instanceId)
    if (!inst) return
    inst.status = 'idle'
    inst.logs.push({
      timestamp: new Date().toLocaleTimeString(),
      level: 'INFO',
      source: 'LoomLauncher',
      message: `Instance "${inst.name}" arrêtée proprement.`
    })
  }

  function createInstance(params: {
    name: string
    icon: string
    gameVersion: string
    loader: MinecraftLoader
    allocatedRamGb: number
  }): MinecraftInstance {
    const newInstance: MinecraftInstance = {
      id: `inst-${Date.now()}`,
      name: params.name,
      icon: params.icon || '⛏️',
      gameVersion: params.gameVersion,
      loader: params.loader,
      loaderVersion: `${params.loader} latest`,
      playtimeMinutes: 0,
      allocatedRamGb: params.allocatedRamGb || 6,
      status: 'idle',
      mods: [],
      worlds: [],
      servers: [],
      resourcePacks: [],
      shaderPacks: [],
      logs: [
        {
          timestamp: new Date().toLocaleTimeString(),
          level: 'INFO',
          source: 'LoomLauncher',
          message: `Nouvelle instance créée : ${params.name} (${params.gameVersion} / ${params.loader})`
        }
      ]
    }
    instances.value.unshift(newInstance)
    selectedInstanceId.value = newInstance.id
    return newInstance
  }

  async function installModpack(pack: ModpackCatalogItem) {
    const newInstance = createInstance({
      name: pack.title,
      icon: '📦',
      gameVersion: pack.mcVersion,
      loader: pack.loader,
      allocatedRamGb: 8
    })

    await ipc.invoke('install_modpack', { modpackId: pack.id })
    return newInstance
  }

  return {
    instances,
    catalog,
    selectedInstanceId,
    activeInstance,
    selectInstance,
    toggleMod,
    launchInstance,
    stopInstance,
    createInstance,
    installModpack
  }
})
