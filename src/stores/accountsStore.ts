import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { ConnectedAccount, AccountPlatform } from '@/models/account'
import { ipc } from '@/services/ipc'

export const useAccountsStore = defineStore('accounts', () => {
  const accounts = ref<ConnectedAccount[]>([
    {
      id: 'acc-steam',
      platform: 'steam',
      username: 'k0bus_arch',
      displayName: 'K0bus',
      avatarUrl: 'https://images.unsplash.com/photo-1534528741775-53994a69daeb?q=80&w=250&auto=format&fit=crop',
      connected: true,
      connectedAt: '12 Janvier 2026',
      gamesCount: 142,
      authType: 'oauth'
    },
    {
      id: 'acc-microsoft',
      platform: 'microsoft',
      username: 'kobus_live@outlook.com',
      displayName: 'K0bus_Gamer (Xbox)',
      avatarUrl: 'https://images.unsplash.com/photo-1535713875002-d1d0cf377fde?q=80&w=250&auto=format&fit=crop',
      connected: true,
      connectedAt: '04 Février 2026',
      gamesCount: 1, // Minecraft Java & Bedrock
      authType: 'device_code'
    },
    {
      id: 'acc-gog',
      platform: 'gog',
      username: 'k0bus_galaxy',
      displayName: 'K0bus Galaxy',
      avatarUrl: 'https://images.unsplash.com/photo-1570295999919-56ceb5ecca61?q=80&w=250&auto=format&fit=crop',
      connected: false,
      gamesCount: 18,
      authType: 'oauth'
    },
    {
      id: 'acc-epic',
      platform: 'epic',
      username: 'k0bus_unreal',
      displayName: 'K0bus Epic',
      avatarUrl: 'https://images.unsplash.com/photo-1507003211169-0a1dd7228f2d?q=80&w=250&auto=format&fit=crop',
      connected: false,
      gamesCount: 45,
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
        const msAcc = getAccount('microsoft')
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
      if (platform === 'microsoft') {
        startMicrosoftDeviceCodeFlow()
      } else {
        // Direct mock login for Steam, Epic, GOG
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
