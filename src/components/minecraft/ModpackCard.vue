<script setup lang="ts">
import { ref } from 'vue'
import { Download, Heart, ArrowDownToLine, Check } from 'lucide-vue-next'
import type { ModpackCatalogItem } from '@/models/minecraft'
import { useMinecraftStore } from '@/stores/minecraftStore'
import UiBadge from '@/components/ui/UiBadge.vue'
import UiButton from '@/components/ui/UiButton.vue'

interface Props {
  modpack: ModpackCatalogItem
}

const props = defineProps<Props>()

const minecraftStore = useMinecraftStore()
const isInstalling = ref(false)
const isInstalled = ref(false)

async function install() {
  if (isInstalling.value || isInstalled.value) return
  isInstalling.value = true
  try {
    await minecraftStore.installModpack(props.modpack)
    isInstalled.value = true
  } finally {
    isInstalling.value = false
  }
}

function formatNumber(num: number) {
  if (num >= 1000000) return (num / 1000000).toFixed(1) + 'M'
  if (num >= 1000) return (num / 1000).toFixed(0) + 'k'
  return num.toString()
}
</script>

<template>
  <div class="group rounded-2xl bg-zinc-900/60 border border-zinc-800/80 hover:border-amber-500/40 transition-all duration-300 overflow-hidden flex flex-col shadow-lg select-none">
    <!-- Banner Header -->
    <div class="relative h-28 w-full overflow-hidden bg-zinc-950">
      <img
        :src="modpack.bannerUrl"
        :alt="modpack.title"
        class="w-full h-full object-cover transition-transform duration-500 group-hover:scale-105"
        loading="lazy"
      />
      <div class="absolute inset-0 bg-gradient-to-t from-zinc-900 via-zinc-900/40 to-transparent" />

      <!-- Source Badge (Modrinth / CurseForge) -->
      <div class="absolute top-2.5 right-2.5">
        <UiBadge
          :variant="modpack.source === 'modrinth' ? 'emerald' : 'rose'"
          size="xs"
        >
          {{ modpack.source === 'modrinth' ? 'Modrinth' : 'CurseForge' }}
        </UiBadge>
      </div>

      <!-- Icon & Title Overlay -->
      <div class="absolute -bottom-3 left-4 flex items-end gap-3">
        <img
          :src="modpack.iconUrl"
          :alt="modpack.title"
          class="w-14 h-14 rounded-xl border-2 border-zinc-800 shadow-xl object-cover bg-zinc-900"
        />
      </div>
    </div>

    <!-- Body Info -->
    <div class="pt-6 p-4 flex flex-col justify-between flex-1 gap-3">
      <div>
        <div class="flex items-baseline justify-between gap-2">
          <h4 class="font-bold text-sm text-zinc-100 group-hover:text-amber-400 transition-colors line-clamp-1">
            {{ modpack.title }}
          </h4>
        </div>
        <p class="text-[11px] text-zinc-400 mt-0.5">Par <span class="text-zinc-300 font-medium">{{ modpack.author }}</span></p>

        <p class="text-xs text-zinc-400 mt-2 line-clamp-2 leading-relaxed">
          {{ modpack.summary }}
        </p>

        <!-- Categories & Loader badges -->
        <div class="flex flex-wrap items-center gap-1.5 mt-3">
          <UiBadge variant="amber" size="xs">
            {{ modpack.loader }}
          </UiBadge>
          <UiBadge variant="zinc" size="xs">
            MC {{ modpack.mcVersion }}
          </UiBadge>
          <span
            v-for="cat in modpack.categories.slice(0, 2)"
            :key="cat"
            class="text-[10px] text-zinc-500 font-medium px-1.5 py-0.5 rounded bg-zinc-950/60 border border-zinc-800"
          >
            {{ cat }}
          </span>
        </div>
      </div>

      <!-- Stats & 1-Click Install Button -->
      <div class="pt-3 border-t border-zinc-800/60 flex items-center justify-between text-xs">
        <div class="flex items-center gap-3 text-zinc-400 font-mono text-[11px]">
          <span class="flex items-center gap-1">
            <Download class="w-3.5 h-3.5 text-zinc-500" />
            {{ formatNumber(modpack.downloads) }}
          </span>
          <span class="flex items-center gap-1">
            <Heart class="w-3.5 h-3.5 text-rose-500/80" />
            {{ formatNumber(modpack.likes) }}
          </span>
        </div>

        <UiButton
          :variant="isInstalled ? 'secondary' : 'primary'"
          size="sm"
          :loading="isInstalling"
          :disabled="isInstalled"
          @click="install"
        >
          <template #prefix>
            <Check v-if="isInstalled" class="w-3.5 h-3.5 text-emerald-400" />
            <ArrowDownToLine v-else class="w-3.5 h-3.5" />
          </template>
          {{ isInstalled ? 'Installé' : 'Installer 1-Clic' }}
        </UiButton>
      </div>
    </div>
  </div>
</template>
