import { createRouter, createWebHistory } from 'vue-router'
import LibraryView from '@/views/LibraryView.vue'
import GameDetailView from '@/views/GameDetailView.vue'
import ModdingView from '@/views/ModdingView.vue'
import MinecraftStudioView from '@/views/MinecraftStudioView.vue'
import WowModdingView from '@/views/WowModdingView.vue'
import AccountsView from '@/views/AccountsView.vue'
import SettingsView from '@/views/SettingsView.vue'

const routes = [
  {
    path: '/',
    redirect: '/library'
  },
  {
    path: '/library',
    name: 'library',
    component: LibraryView
  },
  {
    path: '/games/:slug',
    alias: ['/games/:id', '/game/:slug'],
    name: 'game-detail',
    component: GameDetailView,
    props: true
  },
  {
    path: '/modding',
    name: 'modding',
    component: ModdingView
  },
  {
    path: '/modding/minecraft',
    alias: ['/minecraft'],
    name: 'minecraft-studio',
    component: MinecraftStudioView
  },
  {
    path: '/modding/minecraft/:instanceId',
    alias: ['/minecraft/:instanceId'],
    name: 'minecraft-instance',
    component: MinecraftStudioView,
    props: true
  },
  {
    path: '/modding/wow',
    alias: ['/wow', '/modding/world-of-warcraft'],
    name: 'wow-modding',
    component: WowModdingView
  },
  {
    path: '/accounts',
    name: 'accounts',
    component: AccountsView
  },
  {
    path: '/settings',
    name: 'settings',
    component: SettingsView
  }
]

export const router = createRouter({
  history: createWebHistory(),
  routes,
  scrollBehavior() {
    return { top: 0 }
  }
})
