<script setup lang="ts">
import { computed } from 'vue'

interface Props {
  variant?: 'amber' | 'zinc' | 'emerald' | 'sky' | 'rose' | 'purple'
  size?: 'xs' | 'sm' | 'md'
  dot?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  variant: 'zinc',
  size: 'sm',
  dot: false
})

const variantClasses = computed(() => {
  switch (props.variant) {
    case 'amber':
      return 'bg-amber-500/15 text-amber-300 border-amber-500/30'
    case 'emerald':
      return 'bg-emerald-500/15 text-emerald-300 border-emerald-500/30'
    case 'sky':
      return 'bg-sky-500/15 text-sky-300 border-sky-500/30'
    case 'rose':
      return 'bg-rose-500/15 text-rose-300 border-rose-500/30'
    case 'purple':
      return 'bg-purple-500/15 text-purple-300 border-purple-500/30'
    case 'zinc':
    default:
      return 'bg-zinc-800/80 text-zinc-300 border-zinc-700/60'
  }
})

const dotColor = computed(() => {
  switch (props.variant) {
    case 'amber': return 'bg-amber-400'
    case 'emerald': return 'bg-emerald-400'
    case 'sky': return 'bg-sky-400'
    case 'rose': return 'bg-rose-400'
    case 'purple': return 'bg-purple-400'
    case 'zinc':
    default: return 'bg-zinc-400'
  }
})

const sizeClasses = computed(() => {
  switch (props.size) {
    case 'xs': return 'text-[10px] px-1.5 py-0.5 font-medium'
    case 'md': return 'text-xs px-2.5 py-1 font-semibold'
    case 'sm':
    default: return 'text-[11px] px-2 py-0.5 font-medium'
  }
})
</script>

<template>
  <span
    :class="[
      'inline-flex items-center gap-1.5 rounded-full border tracking-wide uppercase font-mono select-none',
      variantClasses,
      sizeClasses
    ]"
  >
    <span
      v-if="dot"
      :class="['h-1.5 w-1.5 rounded-full animate-pulse', dotColor]"
    />
    <slot />
  </span>
</template>
