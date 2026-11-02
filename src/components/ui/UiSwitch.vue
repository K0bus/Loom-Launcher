<script setup lang="ts">
interface Props {
  modelValue: boolean
  label?: string
  description?: string
  disabled?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  modelValue: false,
  label: '',
  description: '',
  disabled: false
})

const emit = defineEmits<{
  (e: 'update:modelValue', value: boolean): void
}>()

function toggle() {
  if (!props.disabled) {
    emit('update:modelValue', !props.modelValue)
  }
}
</script>

<template>
  <div
    class="flex items-center justify-between gap-4 cursor-pointer select-none py-1.5"
    :class="{ 'opacity-50 cursor-not-allowed': disabled }"
    @click="toggle"
  >
    <div v-if="label || description" class="flex-1">
      <div class="text-sm font-medium text-zinc-200">{{ label }}</div>
      <div v-if="description" class="text-xs text-zinc-400 mt-0.5">{{ description }}</div>
    </div>

    <button
      type="button"
      role="switch"
      :aria-checked="modelValue"
      :disabled="disabled"
      :class="[
        'relative inline-flex h-6 w-11 flex-shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none focus:ring-2 focus:ring-amber-500/40',
        modelValue ? 'bg-amber-500 shadow-sm shadow-amber-500/30' : 'bg-zinc-700/80'
      ]"
    >
      <span
        :class="[
          'pointer-events-none inline-block h-5 w-5 transform rounded-full bg-zinc-950 shadow ring-0 transition duration-200 ease-in-out',
          modelValue ? 'translate-x-5 bg-zinc-950' : 'translate-x-0 bg-zinc-300'
        ]"
      />
    </button>
  </div>
</template>
