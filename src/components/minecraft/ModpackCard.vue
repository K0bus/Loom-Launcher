<script setup lang="ts">
import { ref, computed } from 'vue'
import { Download, Heart, ArrowDownToLine, Check } from 'lucide-vue-next'
import type { ModpackCatalogItem } from '@/models/minecraft'
import { useMinecraftStore } from '@/stores/minecraft'
import { useDownloadsStore } from '@/stores/downloadsStore'
import UiBadge from '@/components/ui/UiBadge.vue'
import UiButton from '@/components/ui/UiButton.vue'

interface Props {
  modpack: ModpackCatalogItem
}

const props = defineProps<Props>()
const emit = defineEmits<{
  (e: 'installed', instanceId?: string): void
}>()

const minecraftStore = useMinecraftStore()
const downloadsStore = useDownloadsStore()
const isSuccess = ref(false)

const isInstalling = computed(() => Boolean(minecraftStore.installingModpackIds[props.modpack.id]))

const isAlreadyInstalled = computed(() => {
  return minecraftStore.instances.some(
    (inst) => inst.name?.trim().toLowerCase() === props.modpack.title?.trim().toLowerCase()
  )
})

// Check active download for this modpack in downloads store
const activeDownload = computed(() => {
  return downloadsStore.activeModDownloads.find(
    (d) =>
      d.remoteModId === props.modpack.id ||
      d.instanceName === props.modpack.title ||
      (d.name && d.name.includes(props.modpack.title))
  )
})

async function install() {
  if (isInstalling.value || isSuccess.value || isAlreadyInstalled.value) return
  try {
    const inst = await minecraftStore.installModpack(props.modpack)
    isSuccess.value = true
    emit('installed', inst?.id)
  } catch (err) {
    console.error('Failed to install modpack:', err)
  }
}

function formatNumber(num: number) {
  if (!num) return '0'
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
        :src="modpack.bannerUrl || modpack.iconUrl || 'https://images.igdb.com/igdb/image/upload/t_1080p/co8fu7.jpg'"
        :alt="modpack.title"
        class="w-full h-full object-cover transition-transform duration-500 group-hover:scale-105"
        loading="lazy"
      />
      <div class="absolute inset-0 bg-gradient-to-t from-zinc-900 via-zinc-900/40 to-transparent" />

      <!-- Source Badge (Modrinth / CurseForge) -->
      <div class="absolute top-2.5 right-2.5">
        <UiBadge
          :variant="modpack.source === 'modrinth' ? 'emerald' : 'amber'"
          size="xs"
        >
          {{ modpack.source === 'modrinth' ? 'Modrinth' : 'CurseForge' }}
        </UiBadge>
      </div>

      <!-- Icon & Title Overlay -->
      <div class="absolute -bottom-3 left-4 flex items-end gap-3">
        <img
          :src="modpack.iconUrl || 'https://images.igdb.com/igdb/image/upload/t_cover_big/co8fu7.jpg'"
          :alt="modpack.title"
          class="w-14 h-14 rounded-xl border-2 border-zinc-800 shadow-xl object-cover bg-zinc-900"
        />
      </div>
    </div>

    <!-- Body Info -->
    <div class="pt-6 p-4 flex flex-col justify-between flex-1 gap-3">
      <div>
        <div class="flex items-baseline justify-between gap-2">
          <h4 class="font-bold text-sm text-zinc-100 group-hover:text-amber-400 transition-colors line-clamp-1" :title="modpack.title">
            {{ modpack.title }}
          </h4>
        </div>
        <p class="text-[11px] text-zinc-400 mt-0.5">Par <span class="text-zinc-300 font-medium">{{ modpack.author }}</span></p>

        <p class="text-xs text-zinc-400 mt-2 line-clamp-2 leading-relaxed" :title="modpack.summary">
          {{ modpack.summary || 'Aucune description fournie.' }}
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
            v-for="cat in (modpack.categories || []).slice(0, 2)"
            :key="cat"
            class="text-[10px] text-zinc-500 font-medium px-1.5 py-0.5 rounded bg-zinc-950/60 border border-zinc-800"
          >
            {{ cat }}
          </span>
        </div>
      </div>

      <!-- Stats & 1-Click Install Button / Progress -->
      <div class="pt-3 border-t border-zinc-800/60 flex items-center justify-between text-xs gap-3">
        <div class="flex items-center gap-3 text-zinc-400 font-mono text-[11px] shrink-0">
          <span class="flex items-center gap-1">
            <Download class="w-3.5 h-3.5 text-zinc-500" />
            {{ formatNumber(modpack.downloads) }}
          </span>
          <span class="flex items-center gap-1">
            <Heart class="w-3.5 h-3.5 text-rose-500/80" />
            {{ formatNumber(modpack.likes) }}
          </span>
        </div>

        <!-- Download in Progress -->
        <div v-if="activeDownload" class="flex-1 max-w-[160px]">
          <div class="flex items-center justify-between text-[10px] text-amber-400 font-mono mb-1">
            <span class="truncate pr-1">{{ activeDownload.fileName || 'Téléchargement...' }}</span>
            <span class="font-bold shrink-0">{{ activeDownload.progressPercent }}%</span>
          </div>
          <div class="h-2 w-full bg-zinc-950 rounded-full overflow-hidden border border-zinc-800">
            <div
              class="h-full bg-gradient-to-r from-amber-500 to-amber-400 transition-all duration-300"
              :style="{ width: `${activeDownload.progressPercent}%` }"
            />
          </div>
        </div>

        <UiButton
          v-else-if="isSuccess || isAlreadyInstalled"
          variant="secondary"
          size="sm"
          disabled
        >
          <template #prefix>
            <Check class="w-3.5 h-3.5 text-emerald-400" />
          </template>
          Installé !
        </UiButton>

        <UiButton
          v-else
          variant="primary"
          size="sm"
          :loading="isInstalling"
          @click="install"
        >
          <template #prefix>
            <ArrowDownToLine class="w-3.5 h-3.5" />
          </template>
          Installer 1-Clic
        </UiButton>
      </div>
    </div>
  </div>
</template>
