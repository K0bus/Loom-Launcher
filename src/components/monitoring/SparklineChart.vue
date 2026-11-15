<script setup lang="ts">
import { computed } from 'vue'

interface Props {
  data: number[]
  height?: number
  color?: 'amber' | 'emerald' | 'rose' | 'sky' | 'purple' | 'zinc'
  min?: number
  max?: number
  unit?: string
  label?: string
  showCurrentValue?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  height: 48,
  color: 'amber',
  unit: '',
  label: '',
  showCurrentValue: false
})

const colorMap = {
  amber: {
    stroke: '#f59e0b',
    fillStart: 'rgba(245, 158, 11, 0.25)',
    fillEnd: 'rgba(245, 158, 11, 0.0)'
  },
  emerald: {
    stroke: '#10b981',
    fillStart: 'rgba(16, 185, 129, 0.25)',
    fillEnd: 'rgba(16, 185, 129, 0.0)'
  },
  rose: {
    stroke: '#f43f5e',
    fillStart: 'rgba(244, 63, 94, 0.25)',
    fillEnd: 'rgba(244, 63, 94, 0.0)'
  },
  sky: {
    stroke: '#38bdf8',
    fillStart: 'rgba(56, 189, 248, 0.25)',
    fillEnd: 'rgba(56, 189, 248, 0.0)'
  },
  purple: {
    stroke: '#c084fc',
    fillStart: 'rgba(192, 132, 252, 0.25)',
    fillEnd: 'rgba(192, 132, 252, 0.0)'
  },
  zinc: {
    stroke: '#71717a',
    fillStart: 'rgba(113, 113, 122, 0.25)',
    fillEnd: 'rgba(113, 113, 122, 0.0)'
  }
}

const theme = computed(() => colorMap[props.color] || colorMap.amber)

// Generate unique id for gradient to avoid collisions
const gradientId = computed(() => `sparkline-grad-${props.color}-${Math.random().toString(36).substring(2, 9)}`)

const currentVal = computed(() => {
  if (props.data.length === 0) return 0
  return props.data[props.data.length - 1]
})

const pathData = computed(() => {
  const points = props.data
  if (!points || points.length === 0) {
    return { line: '', area: '', lastPoint: null }
  }

  const effectiveMin = props.min !== undefined ? props.min : Math.min(...points, 0)
  const calcMax = Math.max(...points, 1)
  const effectiveMax = props.max !== undefined ? Math.max(props.max, calcMax) : calcMax
  const range = (effectiveMax - effectiveMin) || 1

  const svgWidth = 200
  const svgHeight = props.height
  const paddingY = 4
  const availableHeight = svgHeight - paddingY * 2

  const step = points.length > 1 ? svgWidth / (points.length - 1) : svgWidth

  const coords = points.map((val, idx) => {
    const x = idx * step
    const normalized = (val - effectiveMin) / range
    const clampedNorm = Math.max(0, Math.min(1, normalized))
    const y = svgHeight - paddingY - clampedNorm * availableHeight
    return { x, y }
  })

  // Build SVG path
  let line = `M ${coords[0].x.toFixed(1)},${coords[0].y.toFixed(1)}`
  for (let i = 1; i < coords.length; i++) {
    // Smooth Bezier curve control points
    const prev = coords[i - 1]
    const cur = coords[i]
    const cp1x = prev.x + (cur.x - prev.x) / 2
    const cp1y = prev.y
    const cp2x = prev.x + (cur.x - prev.x) / 2
    const cp2y = cur.y
    line += ` C ${cp1x.toFixed(1)},${cp1y.toFixed(1)} ${cp2x.toFixed(1)},${cp2y.toFixed(1)} ${cur.x.toFixed(1)},${cur.y.toFixed(1)}`
  }

  const last = coords[coords.length - 1]
  const area = `${line} L ${last.x.toFixed(1)},${svgHeight} L ${coords[0].x.toFixed(1)},${svgHeight} Z`

  return { line, area, lastPoint: last }
})
</script>

<template>
  <div class="relative w-full flex flex-col justify-end overflow-hidden">
    <!-- Header info if provided -->
    <div v-if="label || showCurrentValue" class="flex items-center justify-between text-xs mb-1 px-1">
      <span v-if="label" class="text-zinc-400 font-medium text-[11px]">{{ label }}</span>
      <span v-if="showCurrentValue" class="font-mono font-bold text-zinc-100 text-[11px]">
        {{ currentVal }} <span class="text-zinc-400 text-[10px]">{{ unit }}</span>
      </span>
    </div>

    <!-- SVG Chart -->
    <svg
      viewBox="0 0 200 48"
      preserveAspectRatio="none"
      class="w-full h-full overflow-visible transition-all duration-300"
      :style="{ height: `${height}px` }"
    >
      <defs>
        <linearGradient :id="gradientId" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" :stop-color="theme.fillStart" />
          <stop offset="100%" :stop-color="theme.fillEnd" />
        </linearGradient>
      </defs>

      <!-- Area fill -->
      <path
        v-if="pathData.area"
        :d="pathData.area"
        :fill="`url(#${gradientId})`"
      />

      <!-- Line stroke -->
      <path
        v-if="pathData.line"
        :d="pathData.line"
        fill="none"
        :stroke="theme.stroke"
        stroke-width="1.8"
        stroke-linecap="round"
        stroke-linejoin="round"
      />

      <!-- Current Head Indicator Dot -->
      <circle
        v-if="pathData.lastPoint"
        :cx="pathData.lastPoint.x"
        :cy="pathData.lastPoint.y"
        r="2.5"
        :fill="theme.stroke"
        class="transition-transform duration-200"
      />
    </svg>
  </div>
</template>
