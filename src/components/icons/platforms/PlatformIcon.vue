<script setup lang="ts">
import { computed } from 'vue'
import SteamIcon from './SteamIcon.vue'
import EpicIcon from './EpicIcon.vue'
import XboxIcon from './XboxIcon.vue'
import GogIcon from './GogIcon.vue'
import BattleNetIcon from './BattleNetIcon.vue'
import type { AccountPlatform } from '@/models/account'

interface Props {
  platform: AccountPlatform | 'battlenet' | 'battle.net' | 'xbox' | 'microsoft' | string
  className?: string
  colored?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  className: 'w-6 h-6',
  colored: false
})

const normalizedPlatform = computed(() => {
  const p = (props.platform || '').toLowerCase().trim()
  if (p === 'battle.net' || p === 'battlenet' || p === 'blizzard') return 'battlenet'
  if (p === 'microsoft' || p === 'xbox') return 'xbox'
  if (p === 'epic' || p === 'epicgames' || p === 'epic-games') return 'epic'
  if (p === 'gog' || p === 'galaxy') return 'gog'
  if (p === 'steam') return 'steam'
  return p
})
</script>

<template>
  <SteamIcon
    v-if="normalizedPlatform === 'steam'"
    :class-name="className"
    :colored="colored"
  />
  <EpicIcon
    v-else-if="normalizedPlatform === 'epic'"
    :class-name="className"
    :colored="colored"
  />
  <XboxIcon
    v-else-if="normalizedPlatform === 'xbox'"
    :class-name="className"
    :colored="colored"
  />
  <GogIcon
    v-else-if="normalizedPlatform === 'gog'"
    :class-name="className"
    :colored="colored"
  />
  <BattleNetIcon
    v-else-if="normalizedPlatform === 'battlenet'"
    :class-name="className"
    :colored="colored"
  />
  <!-- Fallback generic controller icon -->
  <svg
    v-else
    viewBox="0 0 24 24"
    :class="className"
    fill="none"
    stroke="currentColor"
    stroke-width="2"
    stroke-linecap="round"
    stroke-linejoin="round"
  >
    <line x1="6" x2="10" y1="12" y2="12" />
    <line x1="8" x2="8" y1="10" y2="14" />
    <line x1="15" x2="15.01" y1="13" y2="13" />
    <line x1="18" x2="18.01" y1="11" y2="11" />
    <rect width="20" height="12" x="2" y="6" rx="6" />
  </svg>
</template>
