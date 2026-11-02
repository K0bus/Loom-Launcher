export type AccountPlatform = 'steam' | 'epic' | 'gog' | 'microsoft'

export interface ConnectedAccount {
  id: string
  platform: AccountPlatform
  username: string
  displayName: string
  avatarUrl: string
  connected: boolean
  connectedAt?: string
  gamesCount: number
  email?: string
  authType: 'oauth' | 'device_code' | 'token'
  deviceCodeState?: {
    userCode: string
    verificationUri: string
    expiresIn: number
  }
}
