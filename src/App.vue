<script setup lang="ts">
import { onMounted } from 'vue'
import DesktopWindowHeader from '@/components/layout/DesktopWindowHeader.vue'
import { useSystemStore } from '@/stores/systemStore'
import { useToastStore } from '@/stores/toastStore'

const systemStore = useSystemStore()
const toast = useToastStore()

onMounted(() => {
  systemStore.refreshStats()
})
</script>

<template>
  <div class="flex flex-col h-screen w-screen overflow-hidden bg-zinc-950 font-sans text-zinc-100 select-none">
    <!-- Top Floating Desktop Titlebar & Navigation -->
    <DesktopWindowHeader />

    <!-- Main Desktop Content Area -->
    <main class="flex-1 overflow-y-auto overflow-x-hidden relative">
      <router-view v-slot="{ Component }">
        <Transition
          enter-active-class="transition duration-200 ease-out"
          enter-from-class="opacity-0 translate-y-1"
          enter-to-class="opacity-100 translate-y-0"
          leave-active-class="transition duration-100 ease-in"
          leave-from-class="opacity-100 translate-y-0"
          leave-to-class="opacity-0 -translate-y-1"
          mode="out-in"
        >
          <component :is="Component" />
        </Transition>
      </router-view>
    </main>

    <!-- Global Desktop Toast Notification Container -->
    <div
      :class="[
        'fixed bottom-6 right-6 z-50 flex items-center gap-3 px-4 py-3 rounded-xl bg-zinc-900 border border-amber-500/40 text-xs text-white shadow-2xl transition-all duration-300',
        toast.visible ? 'translate-y-0 opacity-100 pointer-events-auto' : 'translate-y-12 opacity-0 pointer-events-none'
      ]"
    >
      <i
        :class="[
          'text-base',
          toast.type === 'success' ? 'ph-fill ph-check-circle text-amber-400' :
          toast.type === 'error' ? 'ph-fill ph-warning-circle text-rose-400' :
          'ph-fill ph-info text-sky-400'
        ]"
      />
      <span class="font-medium">{{ toast.message }}</span>
    </div>
  </div>
</template>

<style>
/* Global background */
html, body {
  background-color: #09090b;
  overflow: hidden;
}
</style>
