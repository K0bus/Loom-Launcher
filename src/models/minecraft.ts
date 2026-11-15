export type MinecraftLoader = 'vanilla' | 'fabric' | 'neoforge' | 'forge' | 'quilt'

export interface MinecraftInstanceRecord {
  id: string
  name: string
  mc_version: string
  modloader: string
  loader_version?: string
  min_ram_mb: number
  max_ram_mb: number
  java_path?: string
  jvm_args: string
  banner_url?: string | null
  cover_url?: string | null
  icon?: string | null
  playtime_seconds?: number
  last_played?: number | null
  created_at: number
}

export interface MinecraftGlobalStats {
  playtimeSeconds: number
  playtimeMinutes: number
  lastPlayed?: string | null
  sessionsCount: number
}

export interface MinecraftPlaytimeTickEvent {
  instance_id: string
  elapsed_seconds: number
  instance_playtime_seconds: number
  global_playtime_seconds: number
  instance_last_played?: number
  global_last_played?: number
}

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
  newFileId?: string
  newDownloadUrl?: string
  fileName: string
  fileSizeMb: number
  loadPriority?: number
  provider?: string // 'modrinth' | 'curseforge' | 'local'
  remoteModId?: string
  remoteFileId?: string
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

export interface MinecraftServer {
  id: string
  name: string
  ip: string
  motd?: string
  iconUrl?: string
  pingMs: number
  playersOnline: number
  playersMax: number
  status: 'online' | 'offline' | 'unreachable'
  version?: string
}

export type MinecraftContentType = 'mod' | 'resourcepack' | 'shader'

export interface MinecraftInstance {
  id: string
  name: string
  icon: string
  bannerUrl?: string | null
  coverUrl?: string | null
  gameVersion: string
  loader: MinecraftLoader
  loaderVersion: string
  playtimeSeconds: number
  playtimeMinutes: number
  lastPlayed?: string
  allocatedRamGb: number
  javaPath?: string
  jvmArgs?: string
  customResolution?: { width: number; height: number }
  mods: MinecraftMod[]
  worlds: MinecraftWorld[]
  servers: MinecraftServer[]
  resourcePacks: MinecraftMod[]
  shaderPacks: MinecraftMod[]
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
