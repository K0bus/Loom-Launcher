export interface WowCharacter {
  name: string
  fullName: string
  realm: string
  account: string
  level?: number
  class?: string
  race?: string
  faction?: string
  lastPlayed?: number
}

export interface WowInstance {
  id: string
  name: string
  flavor: 'forever' | 'retail' | 'classic' | 'classic_era' | 'ptr'
  path: string
  addonsPath: string
  version: string
  gameVersionTypeId: number
  installed: boolean
  addonsCount: number
  icon?: string
  lastPlayed?: number
}

export interface InstalledWowAddon {
  id: string
  folderName: string
  title: string
  version: string
  author?: string
  notes?: string
  website?: string
  category?: string
  interfaceVersion?: string
  sizeBytes: number
  sizeFormatted: string
  lastModified: number
  enabled: boolean
  updateAvailable?: boolean
  latestVersion?: string
  downloadUrl?: string
  curseforgeId?: number
  fileId?: number
  latestFileId?: number
  iconUrl?: string
  secondaryFolders?: string[]
}

export interface WowAddonCategory {
  id: number
  name: string
  slug: string
  icon?: string
}

export interface WowAddonSearchResult {
  id: number
  gameId: number
  name: string
  slug: string
  summary: string
  downloadCount: number
  websiteUrl?: string
  iconUrl?: string
  author: string
  authors: Array<{ name: string; url?: string }>
  categories: Array<{ id: number; name: string; slug: string }>
  latestFiles: Array<{
    id: number
    displayName: string
    fileName: string
    downloadUrl: string
    gameVersions: string[]
    fileLength: number
    fileDate: string
    releaseType: number // 1 = release, 2 = beta, 3 = alpha
  }>
  dateModified?: string
  isInstalled?: boolean
  installedVersion?: string
  updateAvailable?: boolean
}

export interface WowAddonFullDetails extends WowAddonSearchResult {
  descriptionHtml?: string
  screenshots?: Array<{
    id: number
    title: string
    description: string
    thumbnailUrl: string
    url: string
  }>
}
