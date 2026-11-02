<script setup lang="ts">
import { computed } from 'vue'

interface Props {
  value: number // 0 to 100
  max?: number
  height?: 'xs' | 'sm' | 'md' | 'lg'
  variant?: 'amber' | 'emerald' | 'rose' | 'sky'
  showLabel?: boolean
  label?: string
}

const props = withDefaults(defineProps<Props>(), {
  max: 100,
  height: 'sm',
  variant: 'amber',
  showLabel: false,
  label: ''
})

const percentage = computed(() => {
  const p = (props.value / props.max) * 100
  return Math.min(100, Math.max(0, Math.round(p)))
})

const heightClasses = {
  xs: 'h-1',
  sm: 'h-1.5',
  md: 'h-2.5',
  lg: 'h-4'
}

const barColorClasses = {
  amber: 'bg-gradient-to-r from-amber-500 to-amber-400 shadow-[0_0_12px_rgba(245,158,11,0.5)]',
  emerald: 'bg-gradient-to-r from-emerald-500 to-emerald-400 shadow-[0_0_12px_rgba(16,185,129,0.5)]',
  rose: 'bg-gradient-to-r from-rose-500 to-rose-400 shadow-[0_0_12px_rgba(244,63,94,0.5)]',
  sky: 'bg-gradient-to-r from-sky-500 to-sky-400 shadow-[0_0_12px_rgba(14,165,233,0.5)]',
}
</script>

<template>
  <div class="w-full">
    <div v-if="showLabel || label" class="flex items-center justify-between text-xs mb-1.5 text-zinc-400">
      <span class="font-medium text-zinc-300">{{ label }}</span>
      <span class="font-mono text-zinc-400">{{ percentage }}%</span>
    </div>
    
    <div :class="['w-full bg-zinc-800/80 rounded-full overflow-hidden border border-zinc-700/40', heightClasses[height]]">
      <div
        :class="['h-full rounded-full transition-all duration-500 ease-out', barColorClasses[variant]]"
        :style="{ width: `${percentage}%` }"
      />
    </div>
  </div>
</template>
