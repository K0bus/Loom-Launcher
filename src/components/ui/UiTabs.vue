<script setup lang="ts">
export interface TabItem {
  id: string
  label: string
  badge?: string | number
  icon?: any
}

interface Props {
  modelValue: string
  tabs: TabItem[]
  variant?: 'underline' | 'pills'
}

withDefaults(defineProps<Props>(), {
  variant: 'underline'
})

const emit = defineEmits<{
  (e: 'update:modelValue', id: string): void
}>()
</script>

<template>
  <div
    :class="[
      'flex items-center gap-1 overflow-x-auto no-scrollbar',
      variant === 'underline' ? 'border-b border-zinc-800/80 pb-px' : 'bg-zinc-900/80 p-1 rounded-xl border border-zinc-800/80'
    ]"
  >
    <button
      v-for="tab in tabs"
      :key="tab.id"
      type="button"
      @click="emit('update:modelValue', tab.id)"
      :class="[
        'flex items-center gap-2 px-4 py-2 text-sm font-medium transition-all select-none whitespace-nowrap',
        variant === 'underline'
          ? [
              '-mb-px border-b-2',
              modelValue === tab.id
                ? 'border-amber-500 text-amber-400 font-semibold'
                : 'border-transparent text-zinc-400 hover:text-zinc-200 hover:border-zinc-700'
            ]
          : [
              'rounded-lg',
              modelValue === tab.id
                ? 'bg-amber-500 text-zinc-950 font-semibold shadow-md shadow-amber-500/10'
                : 'text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800/50'
            ]
      ]"
    >
      <component :is="tab.icon" v-if="tab.icon" class="w-4 h-4" />
      <span>{{ tab.label }}</span>
      <span
        v-if="tab.badge !== undefined"
        :class="[
          'px-1.5 py-0.2 rounded-full text-xs font-mono font-bold',
          modelValue === tab.id && variant === 'pills'
            ? 'bg-zinc-950/20 text-zinc-950'
            : 'bg-zinc-800 text-zinc-300'
        ]"
      >
        {{ tab.badge }}
      </span>
    </button>
  </div>
</template>
