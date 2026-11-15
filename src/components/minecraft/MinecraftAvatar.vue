<script setup lang="ts">
import { ref, computed, watch, onMounted } from 'vue'
import { UserCheck } from 'lucide-vue-next'

const props = withDefaults(
  defineProps<{
    uuid?: string
    name?: string
    skinUrl?: string
    rawSkinUrl?: string
    size?: number
    rounded?: string
  }>(),
  {
    size: 28,
    rounded: 'rounded-xl'
  }
)

const currentSrcIndex = ref(0)
const hasError = ref(false)
const useCanvas = ref(false)
const canvasRef = ref<HTMLCanvasElement | null>(null)

// Candidate sources in order of preference
const sources = computed(() => {
  const list: string[] = []
  const identifier = props.uuid || props.name

  if (identifier) {
    // 1. mc-heads.net 2D/3D head with helmet overlay
    list.push(`https://mc-heads.net/avatar/${identifier}/64`)
    // 2. minotar.net helm fallback
    list.push(`https://minotar.net/helm/${identifier}/64`)
    // 3. crafatar avatar with overlay fallback
    list.push(`https://crafatar.com/avatars/${identifier}?size=64&overlay`)
  }

  // 4. skin_url if provided and is an avatar URL
  if (props.skinUrl && props.skinUrl.includes('/avatar/')) {
    if (!list.includes(props.skinUrl)) list.push(props.skinUrl)
  }

  return list
})

const activeImgSrc = computed(() => {
  if (currentSrcIndex.value < sources.value.length) {
    return sources.value[currentSrcIndex.value]
  }
  return ''
})

function handleImgError() {
  if (currentSrcIndex.value + 1 < sources.value.length) {
    currentSrcIndex.value++
  } else {
    // Try canvas extraction from raw skin texture if available
    const textureUrl = props.rawSkinUrl || props.skinUrl
    if (textureUrl && (textureUrl.includes('textures.minecraft.net') || textureUrl.startsWith('data:'))) {
      drawHeadFromSkinTexture(textureUrl)
    } else {
      hasError.value = true
    }
  }
}

function drawHeadFromSkinTexture(textureUrl: string) {
  const img = new Image()
  img.crossOrigin = 'anonymous'
  img.onload = () => {
    if (!canvasRef.value) return
    const canvas = canvasRef.value
    canvas.width = 64
    canvas.height = 64
    const ctx = canvas.getContext('2d')
    if (!ctx) return

    ctx.imageSmoothingEnabled = false
    // 1. Draw base face (8, 8, 8, 8)
    ctx.drawImage(img, 8, 8, 8, 8, 0, 0, 64, 64)
    // 2. Draw outer hat/hair layer (40, 8, 8, 8)
    ctx.drawImage(img, 40, 8, 8, 8, 0, 0, 64, 64)

    useCanvas.value = true
    hasError.value = false
  }
  img.onerror = () => {
    hasError.value = true
    useCanvas.value = false
  }
  img.src = textureUrl
}

// Reset state when props change
watch(
  () => [props.uuid, props.name, props.skinUrl, props.rawSkinUrl],
  () => {
    currentSrcIndex.value = 0
    hasError.value = false
    useCanvas.value = false
  }
)

onMounted(() => {
  // If the provided skinUrl is already a raw texture and no uuid is given, draw from texture
  if (!props.uuid && !props.name && props.rawSkinUrl) {
    drawHeadFromSkinTexture(props.rawSkinUrl)
  }
})
</script>

<template>
  <div
    class="relative inline-flex items-center justify-center overflow-hidden bg-zinc-900 border border-zinc-800 shadow-sm shrink-0 select-none"
    :class="rounded"
    :style="{ width: `${size}px`, height: `${size}px` }"
  >
    <!-- 1. Canvas extracted head (offline/custom texture fallback) -->
    <canvas
      v-show="useCanvas"
      ref="canvasRef"
      class="w-full h-full [image-rendering:pixelated]"
      :style="{ width: `${size}px`, height: `${size}px` }"
    />

    <!-- 2. Standard CDN Minecraft Head Avatar (mc-heads / minotar with helm layer) -->
    <img
      v-if="!useCanvas && !hasError && activeImgSrc"
      :src="activeImgSrc"
      :alt="name || 'Minecraft Player Head'"
      class="w-full h-full object-cover [image-rendering:pixelated]"
      loading="lazy"
      @error="handleImgError"
    />

    <!-- 3. Fallback when completely unreachable or no account -->
    <div
      v-else-if="hasError || (!useCanvas && !activeImgSrc)"
      class="w-full h-full flex items-center justify-center bg-zinc-800 text-zinc-400"
    >
      <UserCheck :style="{ width: `${Math.round(size * 0.55)}px`, height: `${Math.round(size * 0.55)}px` }" class="text-amber-400" />
    </div>
  </div>
</template>
