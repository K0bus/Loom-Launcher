<script setup lang="ts">
import { computed } from 'vue'

interface Props {
  variant?: 'primary' | 'secondary' | 'outline' | 'ghost' | 'danger'
  size?: 'sm' | 'md' | 'lg'
  disabled?: boolean
  loading?: boolean
  type?: 'button' | 'submit' | 'reset'
  block?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  variant: 'primary',
  size: 'md',
  disabled: false,
  loading: false,
  type: 'button',
  block: false,
})

const emit = defineEmits<{
  (e: 'click', event: MouseEvent): void
}>()

const baseClasses = 'inline-flex items-center justify-center font-medium transition-all duration-200 select-none rounded-lg focus:outline-none focus-visible:ring-2 focus-visible:ring-amber-500/50 disabled:opacity-50 disabled:cursor-not-allowed active:scale-[0.98]'

const variantClasses = computed(() => {
  switch (props.variant) {
    case 'primary':
      return 'bg-amber-500 hover:bg-amber-400 text-zinc-950 font-semibold shadow-lg shadow-amber-500/20 hover:shadow-amber-500/30 border border-amber-400/30'
    case 'secondary':
      return 'bg-zinc-800/90 hover:bg-zinc-700/90 text-zinc-100 border border-zinc-700/60 hover:border-zinc-600'
    case 'outline':
      return 'bg-transparent hover:bg-zinc-800/60 text-zinc-200 border border-zinc-700/80 hover:border-amber-500/50 hover:text-amber-300'
    case 'ghost':
      return 'bg-transparent hover:bg-zinc-800/50 text-zinc-400 hover:text-zinc-100'
    case 'danger':
      return 'bg-rose-600/90 hover:bg-rose-500 text-white shadow-lg shadow-rose-900/30 border border-rose-500/30'
    default:
      return ''
  }
})

const sizeClasses = computed(() => {
  switch (props.size) {
    case 'sm':
      return 'text-xs px-2.5 py-1.5 gap-1.5'
    case 'lg':
      return 'text-base px-6 py-3 gap-2.5'
    case 'md':
    default:
      return 'text-sm px-4 py-2 gap-2'
  }
})
</script>

<template>
  <button
    :type="type"
    :disabled="disabled || loading"
    :class="[
      baseClasses,
      variantClasses,
      sizeClasses,
      block ? 'w-full' : '',
    ]"
    @click="(e) => emit('click', e)"
  >
    <!-- Loading spinner -->
    <svg
      v-if="loading"
      class="animate-spin -ml-0.5 mr-2 h-4 w-4 text-current"
      xmlns="http://www.w3.org/2000/svg"
      fill="none"
      viewBox="0 0 24 24"
    >
      <circle
        class="opacity-25"
        cx="12"
        cy="12"
        r="10"
        stroke="currentColor"
        stroke-width="4"
      />
      <path
        class="opacity-75"
        fill="currentColor"
        d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
      />
    </svg>

    <slot name="prefix" />
    <slot />
    <slot name="suffix" />
  </button>
</template>
