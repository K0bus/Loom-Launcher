<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from 'vue'
import {
  X,
  ExternalLink,
  Download,
  Check,
  Loader2,
  Image as ImageIcon,
  FileText,
  Layers,
  Globe,
  Bug,
  BookOpen,
  Code,
  MessageSquare,
  Calendar,
  HardDrive,
  ChevronDown,
  ChevronLeft,
  ChevronRight,
  ArrowUpCircle,
  Package,
  AlertCircle,
  Search
} from 'lucide-vue-next'
import { useMinecraftStore } from '@/stores/minecraft'
import type { UnifiedModFullDetails, UnifiedModVersionItem } from '@/models/modding'
import UiBadge from '@/components/ui/UiBadge.vue'
import UiButton from '@/components/ui/UiButton.vue'

interface Props {
  modelValue: boolean
  provider: string
  modId: string
  installedVersion?: string
  instanceId?: string
  contentType?: 'mod' | 'resourcepack' | 'shader'
}

const props = defineProps<Props>()
const emit = defineEmits<{
  (e: 'update:modelValue', value: boolean): void
  (e: 'installed'): void
  (e: 'updated'): void
}>()

const minecraftStore = useMinecraftStore()

const isLoading = ref(false)
const loadError = ref<string | null>(null)
const modDetails = ref<UnifiedModFullDetails | null>(null)
const activeTab = ref<'description' | 'gallery' | 'versions'>('description')

// Versions tab state
const onlyCompatible = ref(true)
const versionSearch = ref('')
const expandedChangelogs = ref<Record<string, boolean>>({})
const loadingChangelogs = ref<Record<string, boolean>>({})
const changelogData = ref<Record<string, string>>({})
const installingVersionId = ref<string | null>(null)

// Lightbox state for gallery
const lightboxIndex = ref<number | null>(null)
const currentLightboxImage = computed(() => {
  if (lightboxIndex.value === null || !modDetails.value) return null
  return modDetails.value.gallery[lightboxIndex.value] || null
})

function prevLightboxImage() {
  if (lightboxIndex.value === null || !modDetails.value?.gallery.length) return
  lightboxIndex.value = (lightboxIndex.value - 1 + modDetails.value.gallery.length) % modDetails.value.gallery.length
}

function nextLightboxImage() {
  if (lightboxIndex.value === null || !modDetails.value?.gallery.length) return
  lightboxIndex.value = (lightboxIndex.value + 1) % modDetails.value.gallery.length
}

const currentInstance = computed(() => {
  if (props.instanceId) {
    return minecraftStore.instances.find((i) => i.id === props.instanceId)
  }
  return minecraftStore.activeInstance
})

const installedModRecord = computed(() => {
  if (!currentInstance.value) return null
  const pool = [
    ...(currentInstance.value.mods || []),
    ...(currentInstance.value.resourcePacks || []),
    ...(currentInstance.value.shaderPacks || [])
  ]
  return (
    pool.find(
      (m) =>
        m.remoteModId === props.modId ||
        (modDetails.value && m.name.toLowerCase() === modDetails.value.name.toLowerCase()) ||
        (modDetails.value?.slug && m.fileName.toLowerCase().includes(modDetails.value.slug.toLowerCase()))
    ) || null
  )
})

const isInstalled = computed(() => Boolean(installedModRecord.value))

const hasUpdate = computed(() => {
  return Boolean(installedModRecord.value?.updateAvailable)
})

function formatDownloads(count: number): string {
  if (!count) return '0'
  if (count >= 1_000_000) return `${(count / 1_000_000).toFixed(1)}M`
  if (count >= 1_000) return `${(count / 1_000).toFixed(0)}k`
  return String(count)
}

function formatBytes(bytes: number): string {
  if (!bytes) return '0 Mo'
  const mb = bytes / (1024 * 1024)
  if (mb < 0.1) {
    return `${(bytes / 1024).toFixed(0)} Ko`
  }
  return `${mb.toFixed(1)} Mo`
}

function formatDate(isoDate?: string): string {
  if (!isoDate) return ''
  try {
    const d = new Date(isoDate)
    return d.toLocaleDateString('fr-FR', {
      year: 'numeric',
      month: 'short',
      day: 'numeric'
    })
  } catch {
    return isoDate
  }
}

async function fetchDetails() {
  if (!props.modId || !props.provider) return
  isLoading.value = true
  loadError.value = null
  expandedChangelogs.value = {}
  loadingChangelogs.value = {}
  changelogData.value = {}

  try {
    modDetails.value = await minecraftStore.getModDetails(props.provider, props.modId)
  } catch (err: any) {
    console.error('[ModDetailsModal] Erreur chargement détails:', err)
    loadError.value = err.message || 'Impossible de récupérer les détails de ce mod.'
  } finally {
    isLoading.value = false
  }
}

watch(
  () => [props.modelValue, props.modId, props.provider],
  ([open]) => {
    if (open) {
      activeTab.value = 'description'
      fetchDetails()
    } else {
      lightboxIndex.value = null
    }
  },
  { immediate: true }
)

const filteredVersions = computed<UnifiedModVersionItem[]>(() => {
  if (!modDetails.value?.versions) return []
  let list = modDetails.value.versions

  if (onlyCompatible.value && currentInstance.value) {
    const mcVer = currentInstance.value.gameVersion
    const loader = (currentInstance.value.loader || '').toLowerCase()

    list = list.filter((v) => {
      const matchVer =
        v.game_versions.length === 0 ||
        v.game_versions.some((gv) => gv === mcVer || gv.startsWith(mcVer) || mcVer.startsWith(gv))
      const matchLoader =
        v.loaders.length === 0 ||
        v.loaders.some((l) => l.toLowerCase() === loader || (loader === 'neoforge' && l.toLowerCase() === 'forge'))
      return matchVer && matchLoader
    })
  }

  if (versionSearch.value.trim()) {
    const q = versionSearch.value.trim().toLowerCase()
    list = list.filter(
      (v) =>
        v.name.toLowerCase().includes(q) ||
        v.version_number.toLowerCase().includes(q) ||
        (v.filename && v.filename.toLowerCase().includes(q))
    )
  }

  return list
})

function hasChangelog(id: string): boolean {
  const content = changelogData.value[id]
  if (!content) return false
  const stripped = content.replace(/<[^>]*>/g, '').trim()
  return stripped.length > 0 || content.includes('<img')
}

async function toggleChangelog(ver: UnifiedModVersionItem) {
  const isCurrentlyOpen = Boolean(expandedChangelogs.value[ver.id])
  if (isCurrentlyOpen) {
    expandedChangelogs.value = { ...expandedChangelogs.value, [ver.id]: false }
    return
  }

  // Open the accordion
  expandedChangelogs.value = { ...expandedChangelogs.value, [ver.id]: true }

  // If already loaded in changelogData, don't re-fetch
  if (changelogData.value[ver.id] !== undefined) {
    return
  }

  // If the version already carries an HTML changelog (common for Modrinth)
  if (ver.changelog && ver.changelog.trim().length > 15 && !ver.changelog.includes('github.com')) {
    changelogData.value = { ...changelogData.value, [ver.id]: ver.changelog }
    return
  }

  const effectiveProvider = (modDetails.value?.provider || props.provider || 'curseforge').toLowerCase()
  const effectiveModId = String(modDetails.value?.id || props.modId)

  loadingChangelogs.value = { ...loadingChangelogs.value, [ver.id]: true }
  try {
    const raw = await minecraftStore.getVersionChangelog(effectiveProvider, effectiveModId, ver.id)
    changelogData.value = { ...changelogData.value, [ver.id]: raw || '' }
  } catch (err) {
    console.warn('[ModDetailsModal] Erreur chargement changelog version:', ver.id, err)
    changelogData.value = { ...changelogData.value, [ver.id]: '' }
  } finally {
    loadingChangelogs.value = { ...loadingChangelogs.value, [ver.id]: false }
  }
}

async function installVersion(version: UnifiedModVersionItem) {
  if (!currentInstance.value) return
  installingVersionId.value = version.id

  try {
    await minecraftStore.installModFromCatalog(currentInstance.value.id, {
      id: modDetails.value?.id || props.modId,
      slug: modDetails.value?.slug || '',
      name: modDetails.value?.name || '',
      summary: modDetails.value?.summary || '',
      author: modDetails.value?.author || '',
      icon_url: modDetails.value?.icon_url,
      downloads: modDetails.value?.downloads || 0,
      provider: (props.provider as any) || 'curseforge',
      categories: modDetails.value?.categories || [],
      loaders: version.loaders,
      game_versions: version.game_versions,
      latest_version: version.id,
      project_type: props.contentType || (modDetails.value as any)?.project_type || 'mod'
    })
    emit('installed')
  } catch (err) {
    console.error('[ModDetailsModal] Erreur installation version:', err)
  } finally {
    installingVersionId.value = null
  }
}

async function triggerDirectUpdate() {
  if (!currentInstance.value || !installedModRecord.value) return
  const m = installedModRecord.value
  installingVersionId.value = m.id
  try {
    await minecraftStore.updateMod(currentInstance.value.id, m.id, m.newFileId, m.newDownloadUrl)
    emit('updated')
  } catch (err) {
    console.error('[ModDetailsModal] Erreur mise à jour:', err)
  } finally {
    installingVersionId.value = null
  }
}

async function triggerQuickInstall() {
  if (!currentInstance.value || !modDetails.value) return
  const targetVer = filteredVersions.value[0] || modDetails.value.versions[0]
  if (targetVer) {
    await installVersion(targetVer)
  }
}

function close() {
  emit('update:modelValue', false)
}

function handleKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape') {
    if (lightboxIndex.value !== null) {
      lightboxIndex.value = null
    } else if (props.modelValue) {
      close()
    }
  }
}

onMounted(() => {
  window.addEventListener('keydown', handleKeydown)
})

onUnmounted(() => {
  window.removeEventListener('keydown', handleKeydown)
})
</script>

<template>
  <div
    v-if="modelValue"
    class="fixed inset-0 z-50 flex items-center justify-center p-3 sm:p-6 bg-zinc-950/80 backdrop-blur-md animate-in fade-in duration-200"
    @click.self="close"
  >
    <div
      class="bg-zinc-900 border border-zinc-800 rounded-3xl w-full max-w-5xl max-h-[92vh] flex flex-col shadow-2xl overflow-hidden relative"
      @click.stop
    >
      <!-- Close button -->
      <button
        type="button"
        @click="close"
        class="absolute right-4 top-4 z-20 p-2.5 rounded-2xl bg-zinc-950/80 border border-zinc-800 text-zinc-400 hover:text-zinc-100 hover:bg-zinc-800 transition-all shadow-md"
        title="Fermer (Échap)"
      >
        <X class="w-5 h-5" />
      </button>

      <!-- Loading State -->
      <div v-if="isLoading" class="p-20 text-center space-y-4 my-auto">
        <Loader2 class="w-10 h-10 text-amber-500 animate-spin mx-auto" />
        <p class="text-sm font-mono text-zinc-400">
          Chargement des détails et des versions du mod...
        </p>
      </div>

      <!-- Error State -->
      <div v-else-if="loadError" class="p-16 text-center space-y-4 my-auto">
        <AlertCircle class="w-12 h-12 text-rose-500 mx-auto" />
        <h3 class="text-base font-bold text-zinc-200">Erreur lors de la récupération</h3>
        <p class="text-xs text-zinc-400 max-w-md mx-auto">{{ loadError }}</p>
        <UiButton variant="outline" size="sm" @click="fetchDetails">
          Réessayer
        </UiButton>
      </div>

      <!-- Main Mod Content -->
      <template v-else-if="modDetails">
        <!-- MOD HEADER HERO -->
        <div class="relative bg-gradient-to-b from-zinc-850 to-zinc-900 border-b border-zinc-800 p-6 sm:p-8 shrink-0">
          <div class="flex flex-col sm:flex-row items-start sm:items-center gap-5 sm:gap-6">
            <!-- Icon -->
            <div class="w-20 h-20 sm:w-24 sm:h-24 rounded-2xl bg-zinc-950 border border-zinc-700/80 p-2 shrink-0 overflow-hidden shadow-xl flex items-center justify-center">
              <img
                v-if="modDetails.icon_url"
                :src="modDetails.icon_url"
                :alt="modDetails.name"
                class="w-full h-full object-cover rounded-xl"
              />
              <Package v-else class="w-10 h-10 text-zinc-600" />
            </div>

            <!-- Info & Badges -->
            <div class="flex-1 min-w-0 space-y-2">
              <div class="flex flex-wrap items-center gap-2.5">
                <h2 class="text-xl sm:text-2xl font-black text-zinc-100 truncate tracking-tight">
                  {{ modDetails.name }}
                </h2>
                <UiBadge
                  :variant="modDetails.provider === 'modrinth' ? 'emerald' : 'amber'"
                  size="xs"
                  class="font-mono uppercase"
                >
                  {{ modDetails.provider }}
                </UiBadge>
                <UiBadge v-if="isInstalled" variant="emerald" size="xs" class="font-mono">
                  <Check class="w-3 h-3 inline mr-1" />
                  Installé {{ installedModRecord?.version ? `v${installedModRecord.version}` : '' }}
                </UiBadge>
                <UiBadge v-if="hasUpdate" variant="amber" size="xs" class="font-mono animate-pulse">
                  <ArrowUpCircle class="w-3 h-3 inline mr-1" />
                  Mise à jour {{ installedModRecord?.newVersion }}
                </UiBadge>
              </div>

              <div class="flex flex-wrap items-center gap-3 text-xs text-zinc-400 font-mono">
                <span>par <strong class="text-zinc-200">{{ modDetails.author }}</strong></span>
                <span>•</span>
                <span>{{ formatDownloads(modDetails.downloads) }} Téléchargements</span>
                <template v-if="modDetails.follows">
                  <span>•</span>
                  <span>{{ formatDownloads(modDetails.follows) }} Favoris</span>
                </template>
              </div>

              <!-- Categories -->
              <div class="flex flex-wrap items-center gap-1.5 pt-1">
                <span
                  v-for="cat in modDetails.categories.slice(0, 5)"
                  :key="cat"
                  class="px-2.5 py-0.5 rounded-lg bg-zinc-950/80 border border-zinc-800 text-[11px] font-mono text-zinc-400"
                >
                  {{ cat }}
                </span>
              </div>
            </div>

            <!-- Top Actions -->
            <div class="flex items-center gap-3 self-stretch sm:self-auto sm:shrink-0 justify-end pt-2 sm:pt-0">
              <UiButton
                v-if="hasUpdate"
                variant="primary"
                size="md"
                @click="triggerDirectUpdate"
                :loading="installingVersionId === installedModRecord?.id"
              >
                <template #prefix>
                  <ArrowUpCircle class="w-4 h-4 text-zinc-950" />
                </template>
                Mettre à jour
              </UiButton>

              <UiButton
                v-else-if="!isInstalled && currentInstance"
                variant="primary"
                size="md"
                @click="triggerQuickInstall"
                :loading="installingVersionId !== null"
              >
                <template #prefix>
                  <Download class="w-4 h-4 text-zinc-950" />
                </template>
                Installer le mod
              </UiButton>
            </div>
          </div>

          <!-- External Links Bar -->
          <div
            v-if="modDetails.links && (modDetails.links.website || modDetails.links.wiki || modDetails.links.issues || modDetails.links.source || modDetails.links.discord)"
            class="flex flex-wrap items-center gap-2 pt-4 mt-4 border-t border-zinc-800/60"
          >
            <a
              v-if="modDetails.links.website"
              :href="modDetails.links.website"
              target="_blank"
              rel="noopener noreferrer"
              class="inline-flex items-center gap-1.5 px-3 py-1 rounded-xl bg-zinc-950/60 hover:bg-zinc-800 border border-zinc-800 text-xs font-mono text-zinc-400 hover:text-zinc-200 transition-colors"
            >
              <Globe class="w-3.5 h-3.5 text-amber-400" />
              Site Web
              <ExternalLink class="w-3 h-3 text-zinc-600" />
            </a>
            <a
              v-if="modDetails.links.wiki"
              :href="modDetails.links.wiki"
              target="_blank"
              rel="noopener noreferrer"
              class="inline-flex items-center gap-1.5 px-3 py-1 rounded-xl bg-zinc-950/60 hover:bg-zinc-800 border border-zinc-800 text-xs font-mono text-zinc-400 hover:text-zinc-200 transition-colors"
            >
              <BookOpen class="w-3.5 h-3.5 text-blue-400" />
              Wiki
              <ExternalLink class="w-3 h-3 text-zinc-600" />
            </a>
            <a
              v-if="modDetails.links.source"
              :href="modDetails.links.source"
              target="_blank"
              rel="noopener noreferrer"
              class="inline-flex items-center gap-1.5 px-3 py-1 rounded-xl bg-zinc-950/60 hover:bg-zinc-800 border border-zinc-800 text-xs font-mono text-zinc-400 hover:text-zinc-200 transition-colors"
            >
              <Code class="w-3.5 h-3.5 text-purple-400" />
              Code Source
              <ExternalLink class="w-3 h-3 text-zinc-600" />
            </a>
            <a
              v-if="modDetails.links.issues"
              :href="modDetails.links.issues"
              target="_blank"
              rel="noopener noreferrer"
              class="inline-flex items-center gap-1.5 px-3 py-1 rounded-xl bg-zinc-950/60 hover:bg-zinc-800 border border-zinc-800 text-xs font-mono text-zinc-400 hover:text-zinc-200 transition-colors"
            >
              <Bug class="w-3.5 h-3.5 text-rose-400" />
              Issues
              <ExternalLink class="w-3 h-3 text-zinc-600" />
            </a>
            <a
              v-if="modDetails.links.discord"
              :href="modDetails.links.discord"
              target="_blank"
              rel="noopener noreferrer"
              class="inline-flex items-center gap-1.5 px-3 py-1 rounded-xl bg-zinc-950/60 hover:bg-zinc-800 border border-zinc-800 text-xs font-mono text-zinc-400 hover:text-zinc-200 transition-colors"
            >
              <MessageSquare class="w-3.5 h-3.5 text-indigo-400" />
              Discord
              <ExternalLink class="w-3 h-3 text-zinc-600" />
            </a>
          </div>
        </div>

        <!-- TABS HEADER -->
        <div class="flex items-center gap-2 px-6 border-b border-zinc-800 bg-zinc-950/50 shrink-0">
          <button
            type="button"
            @click="activeTab = 'description'"
            :class="[
              'flex items-center gap-2 py-3.5 px-4 text-xs font-mono font-bold border-b-2 transition-all',
              activeTab === 'description'
                ? 'border-amber-500 text-amber-400 bg-amber-500/5'
                : 'border-transparent text-zinc-400 hover:text-zinc-200'
            ]"
          >
            <FileText class="w-4 h-4" />
            Description
          </button>

          <button
            type="button"
            @click="activeTab = 'gallery'"
            :class="[
              'flex items-center gap-2 py-3.5 px-4 text-xs font-mono font-bold border-b-2 transition-all',
              activeTab === 'gallery'
                ? 'border-amber-500 text-amber-400 bg-amber-500/5'
                : 'border-transparent text-zinc-400 hover:text-zinc-200'
            ]"
          >
            <ImageIcon class="w-4 h-4" />
            Galerie
            <span
              v-if="modDetails.gallery.length > 0"
              class="ml-1 px-1.5 py-0.5 rounded-full bg-zinc-800 text-[10px] text-zinc-300"
            >
              {{ modDetails.gallery.length }}
            </span>
          </button>

          <button
            type="button"
            @click="activeTab = 'versions'"
            :class="[
              'flex items-center gap-2 py-3.5 px-4 text-xs font-mono font-bold border-b-2 transition-all',
              activeTab === 'versions'
                ? 'border-amber-500 text-amber-400 bg-amber-500/5'
                : 'border-transparent text-zinc-400 hover:text-zinc-200'
            ]"
          >
            <Layers class="w-4 h-4" />
            Versions & Fichiers
            <span
              v-if="modDetails.versions.length > 0"
              class="ml-1 px-1.5 py-0.5 rounded-full bg-zinc-800 text-[10px] text-zinc-300"
            >
              {{ modDetails.versions.length }}
            </span>
          </button>
        </div>

        <!-- TAB CONTENT (SCROLLABLE BODY) -->
        <div class="flex-1 overflow-y-auto p-6 sm:p-8 space-y-6">
          <!-- 1. TAB DESCRIPTION -->
          <div v-if="activeTab === 'description'" class="space-y-6">
            <div
              v-if="modDetails.description"
              class="prose prose-invert max-w-none text-sm text-zinc-300 leading-relaxed font-sans [&_a]:text-amber-400 [&_a:hover]:underline [&_img]:rounded-2xl [&_img]:max-h-[500px] [&_img]:mx-auto [&_img]:my-4 [&_table]:border-collapse [&_th]:border [&_th]:border-zinc-800 [&_th]:p-2.5 [&_td]:border [&_td]:border-zinc-800 [&_td]:p-2.5 [&_code]:bg-zinc-950 [&_code]:px-1.5 [&_code]:py-0.5 [&_code]:rounded-md [&_code]:text-amber-300 [&_code]:font-mono [&_pre]:bg-zinc-950 [&_pre]:p-4 [&_pre]:rounded-2xl [&_h1]:text-xl [&_h1]:font-black [&_h1]:text-zinc-100 [&_h2]:text-lg [&_h2]:font-bold [&_h2]:text-zinc-200 [&_h3]:text-base [&_h3]:font-bold [&_h3]:text-zinc-300 [&_ul]:list-disc [&_ul]:pl-5 [&_ol]:list-decimal [&_ol]:pl-5"
              v-html="modDetails.description"
            />
            <p v-else class="text-zinc-500 text-sm italic">
              {{ modDetails.summary || 'Aucune description détaillée disponible.' }}
            </p>
          </div>

          <!-- 2. TAB GALERIE -->
          <div v-else-if="activeTab === 'gallery'" class="space-y-4">
            <div
              v-if="modDetails.gallery.length > 0"
              class="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 gap-4"
            >
              <div
                v-for="(img, idx) in modDetails.gallery"
                :key="img.url"
                @click="lightboxIndex = idx"
                class="group relative rounded-2xl overflow-hidden border border-zinc-800 bg-zinc-950 cursor-pointer aspect-video shadow-md hover:border-amber-500/50 transition-all hover:scale-[1.02]"
              >
                <img
                  :src="img.url"
                  :alt="img.title || modDetails.name"
                  class="w-full h-full object-cover group-hover:opacity-90 transition-opacity"
                  loading="lazy"
                />
                <div class="absolute inset-0 bg-gradient-to-t from-zinc-950/90 via-transparent to-transparent opacity-0 group-hover:opacity-100 transition-opacity flex flex-col justify-end p-3">
                  <div class="font-bold text-xs text-zinc-100 truncate">
                    {{ img.title || 'Aperçu' }}
                  </div>
                  <div v-if="img.description" class="text-[10px] text-zinc-400 line-clamp-1">
                    {{ img.description }}
                  </div>
                </div>
              </div>
            </div>

            <div v-else class="py-16 text-center text-zinc-500 space-y-2">
              <ImageIcon class="w-10 h-10 mx-auto text-zinc-700" />
              <p class="text-sm">Aucune capture d'écran dans la galerie pour ce mod.</p>
            </div>
          </div>

          <!-- 3. TAB VERSIONS & FICHIERS -->
          <div v-else-if="activeTab === 'versions'" class="space-y-5">
            <!-- Filter & Search Controls Bar -->
            <div class="flex flex-col md:flex-row md:items-center justify-between gap-3.5 bg-zinc-950/70 p-4 rounded-2xl border border-zinc-800/80 shadow-sm">
              <div class="flex flex-wrap items-center gap-3">
                <label class="group flex items-center gap-2.5 text-xs font-mono text-zinc-300 hover:text-white cursor-pointer select-none transition-colors">
                  <input
                    type="checkbox"
                    v-model="onlyCompatible"
                    class="w-4 h-4 rounded-md border-zinc-700 text-amber-500 focus:ring-amber-500/20 bg-zinc-900 accent-amber-500 transition-colors"
                  />
                  <span class="font-bold">Compatible uniquement</span>
                </label>

                <div v-if="currentInstance" class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-xl bg-amber-500/10 border border-amber-500/30 text-[11px] font-mono text-amber-300 font-bold">
                  <span class="w-1.5 h-1.5 rounded-full bg-amber-400" />
                  {{ currentInstance.gameVersion }} • {{ currentInstance.loader }}
                </div>

                <UiBadge variant="zinc" size="xs" class="font-mono text-zinc-400">
                  {{ filteredVersions.length }} {{ filteredVersions.length > 1 ? 'versions trouvées' : 'version trouvée' }}
                </UiBadge>
              </div>

              <!-- Search filter input -->
              <div class="relative w-full md:w-72">
                <input
                  v-model="versionSearch"
                  type="text"
                  placeholder="Rechercher une version..."
                  class="w-full bg-zinc-900/90 border border-zinc-800 rounded-xl pl-9 pr-7 py-2 text-xs text-zinc-200 placeholder-zinc-500 focus:outline-none focus:border-amber-500 transition-all font-mono shadow-inner"
                />
                <Search class="w-3.5 h-3.5 text-zinc-500 absolute left-3 top-1/2 -translate-y-1/2 pointer-events-none" />
                <button
                  v-if="versionSearch"
                  type="button"
                  @click="versionSearch = ''"
                  class="absolute right-2.5 top-1/2 -translate-y-1/2 text-zinc-500 hover:text-zinc-300 p-0.5 text-xs font-mono"
                >
                  ×
                </button>
              </div>
            </div>

            <!-- Versions list -->
            <div v-if="filteredVersions.length > 0" class="space-y-3.5">
              <div
                v-for="ver in filteredVersions"
                :key="ver.id"
                :class="[
                  'group rounded-2xl border transition-all duration-200 overflow-hidden shadow-sm',
                  expandedChangelogs[ver.id]
                    ? 'bg-zinc-950/90 border-amber-500/40 ring-1 ring-amber-500/20'
                    : 'bg-zinc-950/60 border-zinc-800/80 hover:border-zinc-700 hover:bg-zinc-950/80'
                ]"
              >
                <!-- Clickable Card Header for toggle -->
                <div
                  @click="toggleChangelog(ver)"
                  class="p-4 sm:p-5 flex flex-col md:flex-row md:items-center justify-between gap-4 cursor-pointer select-none"
                >
                  <!-- Left side : Name, status, metadata -->
                  <div class="space-y-2 min-w-0 flex-1">
                    <div class="flex flex-wrap items-center gap-2.5">
                      <h4 class="font-bold text-sm sm:text-base text-zinc-100 group-hover:text-amber-400 transition-colors truncate">
                        {{ ver.name }}
                      </h4>

                      <!-- Release type badge -->
                      <UiBadge
                        :variant="ver.release_type === 'release' ? 'emerald' : ver.release_type === 'beta' ? 'amber' : 'rose'"
                        size="xs"
                        class="uppercase font-mono text-[9px] tracking-wider px-2 py-0.5"
                      >
                        {{ ver.release_type || 'release' }}
                      </UiBadge>

                      <!-- Version number badge -->
                      <span
                        v-if="ver.version_number && ver.version_number !== ver.name"
                        class="px-2 py-0.5 rounded-lg bg-zinc-900 border border-zinc-800 text-[10px] font-mono text-zinc-400"
                      >
                        v{{ ver.version_number }}
                      </span>

                      <!-- Installed badge -->
                      <span
                        v-if="installedModRecord && (installedModRecord.remoteFileId === ver.id || installedModRecord.version === ver.version_number)"
                        class="px-2.5 py-0.5 rounded-lg bg-emerald-500/10 border border-emerald-500/30 text-[11px] font-mono text-emerald-400 font-bold flex items-center gap-1.5"
                      >
                        <Check class="w-3.5 h-3.5" />
                        Installée
                      </span>
                    </div>

                    <!-- Meta specs row -->
                    <div class="flex flex-wrap items-center gap-x-4 gap-y-1.5 text-xs font-mono text-zinc-400">
                      <span v-if="ver.date_published" class="flex items-center gap-1.5 text-zinc-400">
                        <Calendar class="w-3.5 h-3.5 text-zinc-500" />
                        {{ formatDate(ver.date_published) }}
                      </span>

                      <span v-if="ver.files?.[0]?.size_bytes" class="flex items-center gap-1.5 text-zinc-400">
                        <HardDrive class="w-3.5 h-3.5 text-zinc-500" />
                        {{ formatBytes(ver.files[0].size_bytes) }}
                      </span>

                      <span v-if="ver.downloads" class="flex items-center gap-1.5 text-zinc-400">
                        <Download class="w-3.5 h-3.5 text-zinc-500" />
                        {{ formatDownloads(ver.downloads) }} dls
                      </span>

                      <span v-if="ver.filename" class="hidden lg:flex items-center gap-1.5 text-zinc-500 truncate max-w-xs" :title="ver.filename">
                        <Package class="w-3.5 h-3.5 text-zinc-600 shrink-0" />
                        <span class="truncate">{{ ver.filename }}</span>
                      </span>
                    </div>
                  </div>

                  <!-- Right side: Actions -->
                  <div class="flex items-center gap-2.5 shrink-0 self-end md:self-center" @click.stop>
                    <!-- Expand Patchnote button -->
                    <button
                      type="button"
                      @click="toggleChangelog(ver)"
                      :class="[
                        'px-3 py-1.5 rounded-xl border text-xs font-mono font-medium transition-all flex items-center gap-1.5 active:scale-95 shadow-sm',
                        expandedChangelogs[ver.id]
                          ? 'bg-amber-500/15 border-amber-500/40 text-amber-300 shadow-amber-500/10'
                          : 'bg-zinc-900 hover:bg-zinc-800/90 border-zinc-800 text-zinc-300 hover:text-white'
                      ]"
                      title="Afficher ou masquer les notes de version"
                    >
                      <Loader2 v-if="loadingChangelogs[ver.id]" class="w-3.5 h-3.5 animate-spin text-amber-400" />
                      <FileText v-else class="w-3.5 h-3.5 text-amber-400" />
                      <span>Patchnote</span>
                      <ChevronDown
                        :class="[
                          'w-3.5 h-3.5 transition-transform duration-200',
                          expandedChangelogs[ver.id] ? 'rotate-180 text-amber-400' : 'text-zinc-500'
                        ]"
                      />
                    </button>

                    <!-- Install button -->
                    <UiButton
                      v-if="currentInstance"
                      variant="primary"
                      size="sm"
                      @click="installVersion(ver)"
                      :loading="installingVersionId === ver.id"
                      class="shadow-md"
                    >
                      <template #prefix>
                        <Download class="w-3.5 h-3.5" />
                      </template>
                      Installer
                    </UiButton>
                  </div>
                </div>

                <!-- Loaders & Compatible MC Versions sub-bar -->
                <div class="px-4 sm:px-5 pb-3.5 pt-2.5 border-t border-zinc-900/90 flex flex-wrap items-center gap-2 bg-zinc-950/40">
                  <div class="flex items-center gap-1.5">
                    <span
                      v-for="l in ver.loaders"
                      :key="l"
                      class="px-2.5 py-0.5 rounded-md bg-amber-500/10 border border-amber-500/30 text-[10px] font-mono font-bold text-amber-300 uppercase tracking-wide"
                    >
                      {{ l }}
                    </span>
                  </div>

                  <div class="h-3 w-px bg-zinc-800 mx-1 hidden sm:block" />

                  <div class="flex flex-wrap items-center gap-1.5">
                    <span
                      v-for="gv in ver.game_versions.slice(0, 8)"
                      :key="gv"
                      class="px-2 py-0.5 rounded-md bg-zinc-900/80 border border-zinc-800 text-[10px] font-mono text-zinc-300"
                    >
                      MC {{ gv }}
                    </span>
                    <span
                      v-if="ver.game_versions.length > 8"
                      class="text-[10px] font-mono text-zinc-500 font-bold px-1"
                    >
                      +{{ ver.game_versions.length - 8 }} versions
                    </span>
                  </div>
                </div>

                <!-- Unfolded Patchnote / Changelog -->
                <div
                  v-if="expandedChangelogs[ver.id]"
                  class="px-4 sm:px-5 pb-5 pt-3 border-t border-zinc-900 bg-zinc-900/25 animate-in fade-in duration-200"
                >
                  <div class="patchnote-container p-4 sm:p-5 rounded-2xl bg-zinc-900/95 border border-zinc-800/80 shadow-inner space-y-3">
                    <div class="flex items-center justify-between pb-2 border-b border-zinc-800">
                      <div class="flex items-center gap-2">
                        <FileText class="w-4 h-4 text-amber-400" />
                        <span class="text-xs font-mono font-bold uppercase tracking-wider text-amber-300">
                          Notes de version (Changelog)
                        </span>
                      </div>
                      <span class="text-[11px] font-mono text-zinc-400 bg-zinc-950 px-2 py-0.5 rounded-md border border-zinc-800">
                        {{ ver.name }}
                      </span>
                    </div>

                    <!-- Loading Changelog -->
                    <div v-if="loadingChangelogs[ver.id]" class="py-6 text-center space-y-2">
                      <Loader2 class="w-5 h-5 text-amber-400 animate-spin mx-auto" />
                      <p class="text-xs font-mono text-zinc-400">
                        Récupération des notes de version depuis {{ modDetails?.provider || props.provider || 'curseforge' }}...
                      </p>
                    </div>

                    <!-- Changelog Content -->
                    <div
                      v-else-if="hasChangelog(ver.id)"
                      class="patchnote-content text-xs text-zinc-300 font-sans leading-relaxed max-h-[450px] overflow-y-auto pr-3 space-y-2 select-text"
                      v-html="changelogData[ver.id]"
                    />

                    <!-- Empty Changelog State -->
                    <div v-else class="py-6 text-center text-zinc-500 space-y-1.5 font-mono">
                      <Package class="w-6 h-6 mx-auto text-zinc-600" />
                      <p class="text-xs text-zinc-400">
                        Aucune note de version spécifique n'a été rédigée par l'auteur pour cette release.
                      </p>
                    </div>
                  </div>
                </div>
              </div>
            </div>

            <div v-else class="py-16 text-center text-zinc-500 space-y-2">
              <Package class="w-8 h-8 mx-auto text-zinc-700" />
              <p class="text-sm">Aucune version ne correspond à vos filtres.</p>
              <button
                v-if="onlyCompatible"
                type="button"
                @click="onlyCompatible = false"
                class="text-xs text-amber-400 hover:underline font-mono"
              >
                Afficher toutes les versions (toutes plateformes & versions MC)
              </button>
            </div>
          </div>
        </div>
      </template>

      <!-- LIGHTBOX FULLSCREEN PREVIEW -->
      <div
        v-if="currentLightboxImage && modDetails"
        class="fixed inset-0 z-60 bg-zinc-950/95 flex flex-col items-center justify-center p-4 backdrop-blur-lg"
        @click.self="lightboxIndex = null"
      >
        <button
          type="button"
          @click="lightboxIndex = null"
          class="absolute top-6 right-6 p-3 rounded-2xl bg-zinc-900 border border-zinc-800 text-zinc-300 hover:text-white"
        >
          <X class="w-6 h-6" />
        </button>

        <button
          v-if="modDetails.gallery.length > 1"
          type="button"
          @click.stop="prevLightboxImage"
          class="absolute left-6 top-1/2 -translate-y-1/2 p-3 rounded-2xl bg-zinc-900 border border-zinc-800 text-zinc-300 hover:text-white"
        >
          <ChevronLeft class="w-6 h-6" />
        </button>

        <button
          v-if="modDetails.gallery.length > 1"
          type="button"
          @click.stop="nextLightboxImage"
          class="absolute right-6 top-1/2 -translate-y-1/2 p-3 rounded-2xl bg-zinc-900 border border-zinc-800 text-zinc-300 hover:text-white"
        >
          <ChevronRight class="w-6 h-6" />
        </button>

        <div class="max-w-5xl max-h-[80vh] flex flex-col items-center gap-3">
          <img
            :src="currentLightboxImage.url"
            :alt="currentLightboxImage.title || 'Aperçu'"
            class="max-w-full max-h-[75vh] object-contain rounded-2xl shadow-2xl border border-zinc-800"
          />
          <div class="text-center font-mono">
            <div class="font-bold text-sm text-zinc-200">
              {{ currentLightboxImage.title || `Image ${(lightboxIndex || 0) + 1} / ${modDetails.gallery.length}` }}
            </div>
            <div v-if="currentLightboxImage.description" class="text-xs text-zinc-400 mt-0.5">
              {{ currentLightboxImage.description }}
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
:deep(.patchnote-content) {
  word-break: break-word;
}
:deep(.patchnote-content h1),
:deep(.patchnote-content h2),
:deep(.patchnote-content h3),
:deep(.patchnote-content h4) {
  font-weight: 700;
  color: #f4f4f5;
  margin-top: 0.85rem;
  margin-bottom: 0.4rem;
}
:deep(.patchnote-content h1) { font-size: 1.05rem; }
:deep(.patchnote-content h2) { font-size: 0.95rem; }
:deep(.patchnote-content h3) { font-size: 0.875rem; color: #fbbf24; }
:deep(.patchnote-content h4) { font-size: 0.8rem; }
:deep(.patchnote-content p) {
  margin-bottom: 0.5rem;
  line-height: 1.6;
}
:deep(.patchnote-content ul) {
  list-style-type: disc !important;
  padding-left: 1.5rem !important;
  margin-top: 0.35rem !important;
  margin-bottom: 0.65rem !important;
}
:deep(.patchnote-content ol) {
  list-style-type: decimal !important;
  padding-left: 1.5rem !important;
  margin-top: 0.35rem !important;
  margin-bottom: 0.65rem !important;
}
:deep(.patchnote-content li) {
  margin-bottom: 0.35rem !important;
  display: list-item !important;
  line-height: 1.5;
}
:deep(.patchnote-content a) {
  color: #fbbf24 !important;
  text-decoration: underline !important;
  text-underline-offset: 2px !important;
}
:deep(.patchnote-content a:hover) {
  color: #fef08a !important;
}
:deep(.patchnote-content code) {
  background-color: #09090b;
  border: 1px solid #27272a;
  padding: 0.1rem 0.35rem;
  border-radius: 0.375rem;
  color: #fef08a;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  font-size: 0.8em;
}
:deep(.patchnote-content pre) {
  background-color: #09090b;
  border: 1px solid #27272a;
  padding: 0.75rem;
  border-radius: 0.5rem;
  overflow-x: auto;
  margin-bottom: 0.5rem;
}
:deep(.patchnote-content blockquote) {
  border-left: 3px solid #f59e0b;
  padding-left: 0.75rem;
  color: #a1a1aa;
  font-style: italic;
  margin-bottom: 0.5rem;
}
:deep(.patchnote-content hr) {
  border-color: #27272a;
  margin: 0.75rem 0;
}
:deep(.patchnote-content table) {
  width: 100%;
  border-collapse: collapse;
  margin-bottom: 0.5rem;
}
:deep(.patchnote-content th),
:deep(.patchnote-content td) {
  border: 1px solid #27272a;
  padding: 0.35rem 0.5rem;
  text-align: left;
}
:deep(.patchnote-content th) {
  background-color: #18181b;
  color: #f4f4f5;
  font-weight: 600;
}
</style>
