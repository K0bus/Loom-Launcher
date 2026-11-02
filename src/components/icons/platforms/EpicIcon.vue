<script setup lang="ts">
interface Props {
  className?: string
  colored?: boolean
}

withDefaults(defineProps<Props>(), {
  className: 'w-6 h-6',
  colored: false
})
</script>

<template>
  <svg
    viewBox="0 0 24 24"
    :class="[className, colored ? 'text-white' : 'currentColor']"
    fill="currentColor"
    aria-label="Epic Games"
    role="img"
  >
    <!-- Official Epic Games Shield -->
    <path
      d="M4.685 0C3.784 0 3.05.733 3.05 1.634v16.146c0 .493.22.957.604 1.267l7.731 5.67a1.642 1.642 0 0 0 1.948 0l7.731-5.67a1.64 1.64 0 0 0 .604-1.267V1.634C21.668.733 20.934 0 20.033 0H4.685zm3.606 4.394h8.044v2.541H8.291V4.394zm0 4.394h7.027v2.54H8.291V8.788zm0 4.394h8.044v2.542H8.291v-2.542z"
    />
  </svg>
</template>
