<script setup lang="ts">
interface Props {
  modelValue?: string | number
  placeholder?: string
  type?: string
  disabled?: boolean
  error?: string
  id?: string
}

withDefaults(defineProps<Props>(), {
  modelValue: '',
  placeholder: '',
  type: 'text',
  disabled: false,
  error: '',
  id: undefined
})

const emit = defineEmits<{
  (e: 'update:modelValue', value: string): void
  (e: 'enter'): void
}>()

function onInput(event: Event) {
  const target = event.target as HTMLInputElement
  emit('update:modelValue', target.value)
}
</script>

<template>
  <div class="relative w-full">
    <div class="relative flex items-center">
      <div v-if="$slots.prefix" class="absolute left-3.5 flex items-center pointer-events-none text-zinc-500">
        <slot name="prefix" />
      </div>

      <input
        :id="id"
        :type="type"
        :value="modelValue"
        :placeholder="placeholder"
        :disabled="disabled"
        @input="onInput"
        @keydown.enter="emit('enter')"
        :class="[
          'w-full bg-zinc-900/80 text-zinc-100 placeholder-zinc-500 text-sm rounded-lg border transition-all duration-200 outline-none focus:ring-2 focus:ring-amber-500/30',
          $slots.prefix ? 'pl-10' : 'pl-3.5',
          $slots.suffix ? 'pr-10' : 'pr-3.5',
          'py-2.5',
          error
            ? 'border-rose-500/80 focus:border-rose-500'
            : 'border-zinc-800/80 focus:border-amber-500/70',
          disabled ? 'opacity-50 cursor-not-allowed' : ''
        ]"
      />

      <div v-if="$slots.suffix" class="absolute right-3.5 flex items-center text-zinc-500">
        <slot name="suffix" />
      </div>
    </div>
    
    <p v-if="error" class="text-xs text-rose-400 mt-1 pl-1">
      {{ error }}
    </p>
  </div>
</template>
