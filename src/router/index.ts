import { createRouter, createWebHistory } from 'vue-router'
import LibraryView from '@/views/LibraryView.vue'
import GameDetailView from '@/views/GameDetailView.vue'
import MinecraftStudioView from '@/views/MinecraftStudioView.vue'
import AccountsView from '@/views/AccountsView.vue'
import SettingsView from '@/views/SettingsView.vue'

const routes = [
  {
    path: '/',
    redirect: '/games/hollow-knight-silksong'
  },
  {
    path: '/library',
    name: 'library',
    component: LibraryView
  },
  {
    path: '/games/:id',
    name: 'game-detail',
    component: GameDetailView,
    props: true
  },
  {
    path: '/minecraft',
    name: 'minecraft-studio',
    component: MinecraftStudioView
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
