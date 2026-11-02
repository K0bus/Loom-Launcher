import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { ConnectedAccount, AccountPlatform } from '@/models/account'
import { steamService } from '@/services/steamService'
import { ipc } from '@/services/ipc'

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
      gamesCount: realSteam.installedGames.length,
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

  const deviceCodeModalOpen = ref(false)
  const isAuthorizing = ref(false)
  const currentDeviceCode = ref<{
    userCode: string
    verificationUri: string
    expiresIn: number
  } | null>(null)

  function getAccount(platform: AccountPlatform) {
    return accounts.value.find((a) => a.platform === platform)
  }

  async function startMicrosoftDeviceCodeFlow() {
    isAuthorizing.value = true
    try {
      const res = await ipc.invoke<{
        userCode: string
        verificationUri: string
        expiresIn: number
      }>('initiate_microsoft_device_code')

      currentDeviceCode.value = res
      deviceCodeModalOpen.value = true
    } catch (err) {
      console.error('Error starting MS OAuth flow:', err)
    } finally {
      isAuthorizing.value = false
    }
  }

  async function confirmMicrosoftDeviceCode() {
    isAuthorizing.value = true
    try {
      const res = await ipc.invoke<{
        authenticated: boolean
        gamertag: string
        uuid: string
      }>('poll_microsoft_device_code', {
        userCode: currentDeviceCode.value?.userCode
      })

      if (res.authenticated) {
        const msAcc = getAccount('xbox') || getAccount('microsoft')
        if (msAcc) {
          msAcc.connected = true
          msAcc.displayName = `${res.gamertag} (Xbox)`
          msAcc.connectedAt = 'À l\'instant'
        }
        deviceCodeModalOpen.value = false
        currentDeviceCode.value = null
      }
    } finally {
      isAuthorizing.value = false
    }
  }

  function toggleConnection(platform: AccountPlatform) {
    const acc = getAccount(platform)
    if (!acc) return

    if (acc.connected) {
      acc.connected = false
    } else {
      if (platform === 'microsoft' || platform === 'xbox') {
        startMicrosoftDeviceCodeFlow()
      } else {
        acc.connected = true
        acc.connectedAt = 'À l\'instant'
      }
    }
  }

  return {
    accounts,
    deviceCodeModalOpen,
    isAuthorizing,
    currentDeviceCode,
    getAccount,
    startMicrosoftDeviceCodeFlow,
    confirmMicrosoftDeviceCode,
    toggleConnection
  }
})
