import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { ConnectedAccount, AccountPlatform } from '@/models/account'
import { steamService } from '@/services/steamService'
import { ipc } from '@/services/ipc'
import { useMinecraftStore } from '@/stores/minecraftStore'
import { useBattleNetStore } from '@/stores/useBattleNetStore'

export const useAccountsStore = defineStore('accounts', () => {
  const realSteam = steamService.getDetectedAccount()

  const accounts = ref<ConnectedAccount[]>([
    {
      id: 'acc-steam',
      platform: 'steam',
      username: realSteam.accountName,
      displayName: realSteam.personaName,
      avatarUrl: '',
      connected: realSteam.detected,
      connectedAt: 'Détecté sur ~/.local/share/Steam',
      gamesCount: realSteam.totalGamesCount,
      authType: 'oauth'
    },
    {
      id: 'acc-epic',
      platform: 'epic',
      username: '',
      displayName: 'Epic Games',
      avatarUrl: '',
      connected: false,
      gamesCount: 0,
      authType: 'oauth'
    },
    {
      id: 'acc-xbox',
      platform: 'xbox',
      username: '',
      displayName: 'Xbox / Microsoft',
      avatarUrl: '',
      connected: false,
      gamesCount: 0,
      authType: 'device_code'
    },
    {
      id: 'acc-gog',
      platform: 'gog',
      username: '',
      displayName: 'GOG Galaxy',
      avatarUrl: '',
      connected: false,
      gamesCount: 0,
      authType: 'oauth'
    },
    {
      id: 'acc-battlenet',
      platform: 'battlenet',
      username: '',
      displayName: 'Battle.net',
      avatarUrl: '',
      connected: false,
      gamesCount: 0,
      authType: 'oauth'
    }
  ])

  function getAccount(platform: AccountPlatform) {
    return accounts.value.find((a) => a.platform === platform)
  }

  async function syncMicrosoftAccount() {
    try {
      const ms = await ipc.getMinecraftAuth()
      const xboxAcc = getAccount('xbox')
      if (xboxAcc) {
        if (ms) {
          xboxAcc.connected = true
          xboxAcc.username = ms.name
          xboxAcc.displayName = `${ms.name} (Xbox Live)`
          xboxAcc.avatarUrl = ms.uuid ? `https://mc-heads.net/avatar/${ms.uuid}/64` : (ms.skin_url || '')
          xboxAcc.connectedAt = 'Session active'
        } else {
          xboxAcc.connected = false
          xboxAcc.username = ''
          xboxAcc.displayName = 'Xbox / Microsoft'
          xboxAcc.avatarUrl = ''
        }
      }
    } catch {}
  }

  async function syncBattleNetAccount() {
    try {
      const bnetStore = useBattleNetStore()
      const acc = await bnetStore.loadAccount()
      const bnetCard = getAccount('battlenet')
      if (bnetCard) {
        if (acc) {
          bnetCard.connected = true
          bnetCard.username = acc.battletag
          bnetCard.displayName = `${acc.battletag} (Blizzard)`
          bnetCard.avatarUrl = acc.avatar_url || ''
          bnetCard.connectedAt = 'Session Battle.net liée'
          bnetCard.gamesCount = bnetStore.totalGamesCount
        } else {
          bnetCard.connected = false
          bnetCard.username = ''
          bnetCard.displayName = 'Battle.net'
          bnetCard.avatarUrl = ''
          bnetCard.connectedAt = undefined
          bnetCard.gamesCount = bnetStore.installedCount
        }
      }
    } catch {}
  }

  syncMicrosoftAccount()
  syncBattleNetAccount()

  function toggleConnection(platform: AccountPlatform) {
    const acc = getAccount(platform)
    if (!acc) return

    if (acc.connected) {
      if (platform === 'xbox' || platform === 'microsoft') {
        ipc.logoutMinecraftAuth().then(() => syncMicrosoftAccount())
      } else if (platform === 'battlenet') {
        const bnetStore = useBattleNetStore()
        bnetStore.logoutBlizzard().then(() => syncBattleNetAccount())
      } else {
        acc.connected = false
      }
    } else {
      if (platform === 'microsoft' || platform === 'xbox') {
        const mcStore = useMinecraftStore()
        mcStore.startMicrosoftLogin().then(() => {
          setTimeout(syncMicrosoftAccount, 3000)
        })
      } else if (platform === 'battlenet') {
        const bnetStore = useBattleNetStore()
        if (!bnetStore.credentialsConfigured) {
          bnetStore.openAuthModal()
        } else {
          bnetStore.loginBlizzard().then(() => syncBattleNetAccount()).catch(() => {})
        }
      } else {
        acc.connected = true
        acc.connectedAt = 'À l\'instant'
      }
    }
  }

  return {
    accounts,
    getAccount,
    syncMicrosoftAccount,
    syncBattleNetAccount,
    toggleConnection
  }
})
