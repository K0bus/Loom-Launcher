<script setup lang="ts">
import { ref } from 'vue'
import { Terminal, Trash2, Copy, Check } from 'lucide-vue-next'
import type { MinecraftInstance } from '@/models/minecraft'
import UiButton from '@/components/ui/UiButton.vue'

interface Props {
  instance: MinecraftInstance
}

const props = defineProps<Props>()

const copied = ref(false)

function copyLogs() {
  const fullText = props.instance.logs
    .map((l) => `[${l.timestamp}] [${l.source}/${l.level}]: ${l.message}`)
    .join('\n')
  navigator.clipboard.writeText(fullText)
  copied.value = true
  setTimeout(() => (copied.value = false), 2000)
}

function clearLogs() {
  props.instance.logs.length = 0
}

const levelColors = {
  INFO: 'text-sky-400',
  WARN: 'text-amber-400 font-semibold',
  ERROR: 'text-rose-400 font-bold',
  DEBUG: 'text-zinc-500'
}
</script>

<template>
  <div class="space-y-3">
    <!-- Header / Action bar -->
    <div class="flex items-center justify-between">
      <div class="flex items-center gap-2 text-xs font-mono text-zinc-400">
        <Terminal class="w-4 h-4 text-amber-400" />
        <span>Console de sortie d'exécution Minecraft / JVM</span>
      </div>

      <div class="flex items-center gap-2">
        <UiButton variant="ghost" size="sm" @click="copyLogs">
          <template #prefix>
            <Check v-if="copied" class="w-3.5 h-3.5 text-emerald-400" />
            <Copy v-else class="w-3.5 h-3.5" />
          </template>
          {{ copied ? 'Copié' : 'Copier les logs' }}
        </UiButton>
        <UiButton variant="ghost" size="sm" @click="clearLogs">
          <template #prefix>
            <Trash2 class="w-3.5 h-3.5" />
          </template>
          Effacer
        </UiButton>
      </div>
    </div>

    <!-- Terminal Window -->
    <div class="bg-zinc-950 border border-zinc-800 rounded-xl p-4 font-mono text-xs overflow-y-auto max-h-[480px] space-y-1.5 shadow-inner">
      <div
        v-for="(log, idx) in instance.logs"
        :key="idx"
        class="flex items-start gap-2 leading-relaxed"
      >
        <span class="text-zinc-600 select-none">[{{ log.timestamp }}]</span>
        <span class="text-zinc-500 font-medium">[{{ log.source }}]:</span>
        <span :class="levelColors[log.level]">{{ log.message }}</span>
      </div>

      <div v-if="instance.logs.length === 0" class="text-zinc-600 italic py-8 text-center">
        Aucun log enregistré pour le moment. Démarrez l'instance pour voir le flux en temps réel.
      </div>
    </div>
  </div>
</template>
