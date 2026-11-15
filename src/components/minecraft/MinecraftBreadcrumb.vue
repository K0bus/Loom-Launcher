<script setup lang="ts">
import { ChevronRight } from 'lucide-vue-next'
import { useRouter } from 'vue-router'

export interface BreadcrumbItem {
  label: string
  to?: string
  active?: boolean
  icon?: any
}

defineProps<{
  items: BreadcrumbItem[]
}>()

const router = useRouter()

function handleClick(item: BreadcrumbItem) {
  if (item.to && !item.active) {
    router.push(item.to)
  }
}
</script>

<template>
  <nav
    aria-label="Fil d'Ariane"
    class="inline-flex items-center gap-2 px-4 py-2 rounded-2xl bg-zinc-950/70 border border-zinc-800/80 backdrop-blur-md text-xs font-mono select-none shadow-sm"
  >
    <div
      v-for="(item, idx) in items"
      :key="item.label"
      class="inline-flex items-center gap-2"
    >
      <span
        v-if="idx > 0"
        class="text-zinc-600 flex items-center"
      >
        <ChevronRight class="w-3.5 h-3.5 text-zinc-600" />
      </span>

      <button
        v-if="item.to && !item.active"
        type="button"
        @click="handleClick(item)"
        class="text-zinc-400 hover:text-amber-400 transition-colors flex items-center gap-1.5 focus:outline-none"
      >
        <component :is="item.icon" v-if="item.icon" class="w-3.5 h-3.5 text-amber-400" />
        <span>{{ item.label }}</span>
      </button>

      <span
        v-else
        :class="[
          'flex items-center gap-1.5 font-bold',
          item.active ? 'text-amber-400' : 'text-zinc-200'
        ]"
      >
        <component :is="item.icon" v-if="item.icon" class="w-3.5 h-3.5 text-amber-400" />
        <span>{{ item.label }}</span>
      </span>
    </div>
  </nav>
</template>
