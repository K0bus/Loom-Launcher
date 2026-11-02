<script setup lang="ts">
import { ref, computed } from 'vue'
import { Search, FolderOpen, ArrowUpCircle } from 'lucide-vue-next'
import type { MinecraftInstance } from '@/models/minecraft'
import { useMinecraftStore } from '@/stores/minecraftStore'
import UiInput from '@/components/ui/UiInput.vue'
import UiBadge from '@/components/ui/UiBadge.vue'
import UiButton from '@/components/ui/UiButton.vue'

interface Props {
  instance: MinecraftInstance
}

const props = defineProps<Props>()

const minecraftStore = useMinecraftStore()
const searchMod = ref('')

const filteredMods = computed(() => {
  if (!searchMod.value.trim()) return props.instance.mods
  const q = searchMod.value.toLowerCase()
  return props.instance.mods.filter((m) =>
    m.name.toLowerCase().includes(q) || m.author.toLowerCase().includes(q) || m.fileName.toLowerCase().includes(q)
  )
})
</script>

<template>
  <div class="space-y-4">
    <!-- Toolbar -->
    <div class="flex items-center justify-between gap-4">
      <div class="max-w-xs w-full">
        <UiInput
          v-model="searchMod"
          placeholder="Filtrer les mods installés..."
        >
          <template #prefix>
            <Search class="w-4 h-4 text-zinc-500" />
          </template>
        </UiInput>
      </div>

      <div class="flex items-center gap-2">
        <UiBadge variant="amber" size="sm">
          {{ instance.mods.filter(m => m.enabled).length }} / {{ instance.mods.length }} Actifs
        </UiBadge>
        <UiButton variant="outline" size="sm">
          <template #prefix>
            <FolderOpen class="w-3.5 h-3.5" />
          </template>
          Dossier Mods
        </UiButton>
      </div>
    </div>

    <!-- Mod list -->
    <div class="border border-zinc-800/80 rounded-2xl overflow-hidden bg-zinc-950/40 divide-y divide-zinc-800/60">
      <div
        v-for="mod in filteredMods"
        :key="mod.id"
        class="p-4 flex items-center justify-between gap-4 hover:bg-zinc-900/50 transition-colors"
      >
        <div class="flex items-center gap-3.5 flex-1 min-w-0">
          <!-- Toggle switch -->
          <button
            type="button"
            @click="minecraftStore.toggleMod(instance.id, mod.id)"
            :class="[
              'relative inline-flex h-5 w-9 flex-shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 focus:outline-none',
              mod.enabled ? 'bg-amber-500' : 'bg-zinc-700'
            ]"
          >
            <span
              :class="[
                'pointer-events-none inline-block h-4 w-4 transform rounded-full bg-zinc-950 shadow transition duration-200',
                mod.enabled ? 'translate-x-4' : 'translate-x-0'
              ]"
            />
          </button>

          <div class="min-w-0 flex-1">
            <div class="flex items-center gap-2">
              <span
                :class="[
                  'font-bold text-sm truncate',
                  mod.enabled ? 'text-zinc-100' : 'text-zinc-500 line-through'
                ]"
              >
                {{ mod.name }}
              </span>
              <span class="text-xs text-zinc-500 font-mono">v{{ mod.version }}</span>
              <UiBadge v-if="mod.updateAvailable" variant="amber" size="xs">
                <ArrowUpCircle class="w-3 h-3 text-amber-400 inline mr-0.5" />
                Mise à jour {{ mod.newVersion }}
              </UiBadge>
            </div>
            <p class="text-xs text-zinc-400 truncate mt-0.5">
              {{ mod.description }}
            </p>
          </div>
        </div>

        <div class="text-right flex flex-col items-end flex-shrink-0 font-mono text-[11px] text-zinc-500">
          <span>{{ mod.fileName }}</span>
          <span>{{ mod.fileSizeMb }} Mo • {{ mod.author }}</span>
        </div>
      </div>

      <div v-if="filteredMods.length === 0" class="p-8 text-center text-zinc-500 text-sm">
        Aucun mod correspondant à votre recherche.
      </div>
    </div>
  </div>
</template>
