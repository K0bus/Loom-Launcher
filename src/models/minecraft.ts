export type MinecraftLoader = 'vanilla' | 'fabric' | 'neoforge' | 'forge' | 'quilt'

export interface MinecraftMod {
  id: string
  name: string
  version: string
  author: string
  enabled: boolean
  description: string
  iconUrl?: string
  updateAvailable?: boolean
  newVersion?: string
  fileName: string
  fileSizeMb: number
}

export interface MinecraftWorld {
  id: string
  name: string
  lastPlayed: string
  sizeMb: number
  gameMode: 'survival' | 'creative' | 'hardcore'
  iconUrl?: string
}

export interface ResourcePack {
  id: string
  name: string
  description: string
  enabled: boolean
  formatVersion: number
  iconUrl?: string
}

export interface ShaderPack {
  id: string
  name: string
  profile: string
  enabled: boolean
  authors: string
}

export interface LogEntry {
  timestamp: string
  level: 'INFO' | 'WARN' | 'ERROR' | 'DEBUG'
  source: string
  message: string
}

export interface MinecraftInstance {
  id: string
  name: string
  icon: string
  gameVersion: string
  loader: MinecraftLoader
  loaderVersion: string
  playtimeMinutes: number
  lastPlayed?: string
  allocatedRamGb: number
  javaPath?: string
  jvmArgs?: string
  customResolution?: { width: number; height: number }
  mods: MinecraftMod[]
  worlds: MinecraftWorld[]
  resourcePacks: ResourcePack[]
  shaderPacks: ShaderPack[]
  logs: LogEntry[]
  status: 'idle' | 'running' | 'updating'
}

export interface ModpackCatalogItem {
  id: string
  slug: string
  title: string
  author: string
  downloads: number
  likes: number
  iconUrl: string
  bannerUrl: string
  summary: string
  loader: MinecraftLoader
  mcVersion: string
  source: 'modrinth' | 'curseforge'
  categories: string[]
}
