export type ProviderKind = 'modrinth' | 'curseforge' | 'local' | 'nexus'

export interface LaunchExecutable {
  id: string
  label: string
  description?: string
  is_default: boolean
}

export interface GameModdingManifest {
  game_id: string
  game_name: string
  supported_providers: ProviderKind[]
  supports_load_order: boolean
  supports_conflict_resolution: boolean
  mods_directory: string
  file_extensions: string[]
  executables: LaunchExecutable[]
}

export interface UnifiedModSearchResult {
  id: string
  slug: string
  name: string
  summary: string
  author: string
  icon_url?: string
  downloads: number
  provider: ProviderKind
  latest_version?: string
  categories: string[]
  loaders: string[]
  game_versions: string[]
  project_type?: 'mod' | 'resourcepack' | 'shader'
}

export interface RemoteDependency {
  remote_mod_id: string
  remote_version_id?: string
  name?: string
  is_required: boolean
  dependency_type: string
}

export interface ModFileInfo {
  id: string
  filename: string
  url: string
  size_bytes: number
  sha1?: string
  sha512?: string
  is_primary: boolean
}

export interface ModVersionInfo {
  id: string
  mod_id: string
  name: string
  version_number: string
  game_versions: string[]
  loaders: string[]
  files: ModFileInfo[]
  dependencies: RemoteDependency[]
}

export interface GameProfileRecord {
  id: string
  game_id: string
  name: string
  game_version: string
  modloader?: string
  created_at: number
}

export interface ProfileModRecord {
  id: string
  profile_id: string
  provider: string
  remote_mod_id: string
  remote_file_id: string
  name: string
  file_path: string
  filename?: string
  load_priority: number
  is_enabled: boolean
  version?: string
  author?: string
  description?: string
  icon_url?: string
  file_size_mb?: number
  update_available?: boolean
  new_version?: string
  new_file_id?: string
  new_download_url?: string
}

export interface UnifiedModGalleryItem {
  url: string
  title?: string
  description?: string
}

export interface UnifiedModVersionItem {
  id: string
  name: string
  version_number: string
  release_type?: 'release' | 'beta' | 'alpha'
  game_versions: string[]
  loaders: string[]
  date_published?: string
  downloads?: number
  changelog?: string
  filename?: string
  files: Array<{
    id?: string
    filename: string
    url: string
    size_bytes: number
    primary?: boolean
  }>
}

export interface UnifiedModFullDetails {
  id: string
  slug: string
  name: string
  summary: string
  description: string
  icon_url?: string
  banner_url?: string
  author: string
  authors?: Array<{ name: string; role?: string }>
  provider: ProviderKind
  downloads: number
  follows?: number
  categories: string[]
  loaders: string[]
  game_versions: string[]
  gallery: UnifiedModGalleryItem[]
  versions: UnifiedModVersionItem[]
  links?: {
    website?: string
    wiki?: string
    issues?: string
    source?: string
    discord?: string
  }
}

export interface ModDependencyRecord {
  mod_id: string
  depends_on_remote_id: string
  is_required: boolean
}

export interface FileConflictRuleRecord {
  profile_id: string
  relative_path: string
  winning_mod_id: string
}

export interface MinecraftAccount {
  uuid: string
  name: string
  skin_url?: string
  raw_skin_url?: string
  access_token: string
  refresh_token: string
  expires_at: number
  xuid?: string
  client_id?: string
}

export interface DeviceCodeResponse {
  user_code: string
  device_code: string
  verification_uri: string
  expires_in: number
  interval: number
  message: string
}

export interface PollAuthResult {
  pending: boolean
  account?: MinecraftAccount
  error?: string
}

export interface MinecraftLogEvent {
  instance_id: string
  timestamp: string
  level: 'INFO' | 'WARN' | 'ERROR' | 'DEBUG'
  source: string
  message: string
}

export interface MinecraftExitEvent {
  instance_id: string
  exit_code?: number
}
