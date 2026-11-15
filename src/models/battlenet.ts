export interface BattleNetGame {
  product_uid: string
  name: string
  install_path: string
  executable_relative_path: string
  version_build?: string | null
  is_external: boolean
  is_installed: boolean
  last_played?: number | null
  playtime_seconds?: number | null
}

export interface BattleNetAccount {
  account_id: string
  battletag: string
  avatar_url?: string | null
  access_token: string
  refresh_token: string
  expires_at: number
}

export interface BattleNetConfig {
  id: string
  wine_prefix_path: string
  runner_executable: string
  battlenet_exe_path: string
  enable_gamemode: boolean
  enable_mangohud: boolean
  custom_env_vars?: string | null
}

export interface BlizzardGameEntitlement {
  product_uid: string
  name: string
  is_owned: boolean
  is_installed: boolean
  install_path?: string | null
  version_build?: string | null
  cover_url?: string | null
  banner_url?: string | null
  summary?: string | null
  stats?: Record<string, any> | null
}
