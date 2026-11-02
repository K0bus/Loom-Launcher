<script setup lang="ts">
import { Globe, Sparkles } from 'lucide-vue-next'
import type { MinecraftInstance } from '@/models/minecraft'
import UiBadge from '@/components/ui/UiBadge.vue'

interface Props {
  instance: MinecraftInstance
}

defineProps<Props>()
</script>

<template>
  <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
    <!-- Worlds -->
    <div class="p-5 rounded-2xl bg-zinc-950/60 border border-zinc-800/80 space-y-4">
      <div class="flex items-center justify-between">
        <h4 class="font-bold text-sm text-zinc-200 flex items-center gap-2">
          <Globe class="w-4 h-4 text-amber-400" />
          Mondes Sauvegardés (Saves)
        </h4>
        <UiBadge variant="zinc" size="xs">{{ instance.worlds.length }} Mondes</UiBadge>
      </div>

      <div class="space-y-2">
        <div
          v-for="world in instance.worlds"
          :key="world.id"
          class="p-3 rounded-xl bg-zinc-900/60 border border-zinc-800/60 flex items-center justify-between"
        >
          <div>
            <div class="font-semibold text-xs text-zinc-100">{{ world.name }}</div>
            <div class="text-[11px] text-zinc-400 mt-0.5">
              Mode: <span class="capitalize text-amber-400/90">{{ world.gameMode }}</span> • {{ world.sizeMb }} Mo
            </div>
          </div>
          <span class="text-[10px] text-zinc-500 font-mono">{{ world.lastPlayed }}</span>
        </div>

        <div v-if="instance.worlds.length === 0" class="text-xs text-zinc-500 italic py-2">
          Aucun monde créé dans cette instance.
        </div>
      </div>
    </div>

    <!-- Resource & Shader Packs -->
    <div class="p-5 rounded-2xl bg-zinc-950/60 border border-zinc-800/80 space-y-4">
      <div class="flex items-center justify-between">
        <h4 class="font-bold text-sm text-zinc-200 flex items-center gap-2">
          <Sparkles class="w-4 h-4 text-amber-400" />
          Shaders & Packs de Textures
        </h4>
        <UiBadge variant="zinc" size="xs">Oculus / Iris</UiBadge>
      </div>

      <div class="space-y-2">
        <div
          v-for="shader in instance.shaderPacks"
          :key="shader.id"
          class="p-3 rounded-xl bg-zinc-900/60 border border-zinc-800/60 flex items-center justify-between"
        >
          <div>
            <div class="font-semibold text-xs text-zinc-100">{{ shader.name }}</div>
            <div class="text-[11px] text-zinc-400 mt-0.5">Par {{ shader.authors }} • Profil {{ shader.profile }}</div>
          </div>
          <UiBadge :variant="shader.enabled ? 'emerald' : 'zinc'" size="xs">
            {{ shader.enabled ? 'Actif' : 'Désactivé' }}
          </UiBadge>
        </div>

        <div
          v-for="rp in instance.resourcePacks"
          :key="rp.id"
          class="p-3 rounded-xl bg-zinc-900/60 border border-zinc-800/60 flex items-center justify-between"
        >
          <div>
            <div class="font-semibold text-xs text-zinc-100">{{ rp.name }}</div>
            <div class="text-[11px] text-zinc-400 mt-0.5">{{ rp.description }}</div>
          </div>
          <UiBadge :variant="rp.enabled ? 'amber' : 'zinc'" size="xs">
            Format v{{ rp.formatVersion }}
          </UiBadge>
        </div>

        <div v-if="instance.shaderPacks.length === 0 && instance.resourcePacks.length === 0" class="text-xs text-zinc-500 italic py-2">
          Aucun pack installé.
        </div>
      </div>
    </div>
  </div>
</template>
