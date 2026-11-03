<script setup lang="ts">
import { useRouter } from 'vue-router'
import { useDownloadsStore, type SteamDownloadItem } from '@/stores/downloadsStore'
import { useGamesStore } from '@/stores/gamesStore'
import { useToastStore } from '@/stores/toastStore'

defineProps<{
  open: boolean
}>()

const emit = defineEmits<{
  (e: 'close'): void
}>()

const router = useRouter()
const downloadsStore = useDownloadsStore()
const gamesStore = useGamesStore()
const toast = useToastStore()

function formatBytes(bytes: number): string {
  if (!bytes || bytes <= 0) return '0 Go'
  const gb = bytes / (1024 ** 3)
  if (gb < 0.1) {
    const mb = bytes / (1024 ** 2)
    return `${mb.toFixed(0)} Mo`
  }
  return `${gb.toFixed(1)} Go`
}

function formatDate(timestamp?: number): string {
  if (!timestamp) return 'Récemment'
  const diffSec = Math.floor(Date.now() / 1000) - timestamp
  if (diffSec < 60) return 'À l\'instant'
  if (diffSec < 3600) return `Il y a ${Math.floor(diffSec / 60)} min`
  if (diffSec < 86400) return `Il y a ${Math.floor(diffSec / 3600)} h`
  return `Il y a ${Math.floor(diffSec / 86400)} j`
}

function goToGame(item: SteamDownloadItem) {
  const game =
    gamesStore.getGameById(item.gameId) ||
    gamesStore.getGameById(item.appid)
  if (game?.slug) {
    router.push(`/games/${game.slug}`)
  } else {
    router.push(`/games/${item.gameId}`)
  }
  emit('close')
}

async function handleLaunch(item: SteamDownloadItem, event: Event) {
  event.stopPropagation()
  toast.show(`Lancement de ${item.name}...`, 'info')
  await gamesStore.launchGame(item.gameId)
  emit('close')
}
</script>

<template>
  <div v-if="open" class="downloads-popover-container" @click.stop>
    <!-- Header -->
    <div class="popover-header">
      <div class="header-title-group">
        <svg class="header-icon" viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" />
        </svg>
        <span class="header-title">Téléchargements Steam</span>
        <span v-if="downloadsStore.isDownloading" class="active-badge">
          {{ downloadsStore.activeCount }} actif{{ downloadsStore.activeCount > 1 ? 's' : '' }}
        </span>
      </div>

      <div class="header-right">
        <span v-if="downloadsStore.isDownloading" class="speed-pill">
          <svg viewBox="0 0 24 24" width="12" height="12" fill="currentColor">
            <path d="M13 10V3L4 14h7v7l9-11h-7z" />
          </svg>
          {{ downloadsStore.formattedSpeed }}
        </span>
        <button class="btn-close" @click="emit('close')" aria-label="Fermer">
          <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>
    </div>

    <!-- Body -->
    <div class="popover-body">
      <!-- Section 1 : Téléchargements Actifs -->
      <div v-if="downloadsStore.activeDownloads.length > 0" class="downloads-section">
        <div class="section-label">En cours</div>

        <div class="items-list">
          <div
            v-for="item in downloadsStore.activeDownloads"
            :key="item.appid"
            class="download-card active-card"
            @click="goToGame(item)"
          >
            <div class="card-thumb">
              <img :src="item.coverUrl" :alt="item.name" class="thumb-img" />
              <div class="thumb-pulse" />
            </div>

            <div class="card-content">
              <div class="card-top">
                <span class="game-name" :title="item.name">{{ item.name }}</span>
                <span class="percent-text font-mono">{{ item.progressPercent }}%</span>
              </div>

              <!-- Animated Progress Bar -->
              <div class="progress-track">
                <div
                  class="progress-fill"
                  :style="{ width: `${item.progressPercent}%` }"
                  :class="{ staging: item.state === 'staging' }"
                />
              </div>

              <div class="card-bottom">
                <span class="bytes-text">
                  {{ formatBytes(item.bytesDownloaded) }} / {{ formatBytes(item.bytesTotal) }}
                </span>
                <span class="state-pill" :class="item.state">
                  {{ item.state === 'staging' ? 'Finalisation...' : 'Téléchargement...' }}
                </span>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Section 2 : Récemment Terminés -->
      <div v-if="downloadsStore.completedDownloads.length > 0" class="downloads-section">
        <div class="section-label">Récemment installés</div>

        <div class="items-list">
          <div
            v-for="item in downloadsStore.completedDownloads"
            :key="item.appid"
            class="download-card completed-card"
            @click="goToGame(item)"
          >
            <div class="card-thumb small">
              <img :src="item.coverUrl" :alt="item.name" class="thumb-img" />
              <div class="done-check">
                <svg viewBox="0 0 24 24" width="10" height="10" fill="none" stroke="currentColor">
                  <polyline points="20 6 9 17 4 12" stroke-width="3" stroke-linecap="round" stroke-linejoin="round" />
                </svg>
              </div>
            </div>

            <div class="card-content">
              <div class="card-top">
                <span class="game-name" :title="item.name">{{ item.name }}</span>
                <span class="date-text">{{ formatDate(item.completedAt) }}</span>
              </div>
              <div class="card-bottom">
                <span class="bytes-text">{{ formatBytes(item.bytesTotal) }}</span>
                <button class="btn-play-quick" @click="handleLaunch(item, $event)">
                  <svg viewBox="0 0 24 24" width="10" height="10" fill="currentColor">
                    <polygon points="5 3 19 12 5 21 5 3" />
                  </svg>
                  Jouer
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Empty State -->
      <div
        v-if="downloadsStore.activeDownloads.length === 0 && downloadsStore.completedDownloads.length === 0"
        class="empty-state"
      >
        <div class="empty-icon-wrap">
          <svg viewBox="0 0 24 24" width="24" height="24" fill="none" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" />
          </svg>
        </div>
        <p class="empty-title">Aucun téléchargement actif</p>
        <p class="empty-subtitle">Les jeux lancés en téléchargement depuis Steam ou Loom s'afficheront ici en direct.</p>
      </div>
    </div>
  </div>
</template>

<style scoped>
.downloads-popover-container {
  position: absolute;
  top: calc(100% + 8px);
  right: 0;
  width: 360px;
  max-width: 90vw;
  background: rgba(17, 20, 28, 0.95);
  backdrop-filter: blur(20px);
  border: 1px solid rgba(255, 255, 255, 0.12);
  border-radius: 16px;
  box-shadow: 0 20px 50px rgba(0, 0, 0, 0.65), 0 0 30px rgba(245, 158, 11, 0.1);
  z-index: 200;
  overflow: hidden;
  animation: popoverFadeIn 0.2s cubic-bezier(0.16, 1, 0.3, 1);
}

.popover-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.9rem 1.1rem;
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
  background: rgba(255, 255, 255, 0.02);
}

.header-title-group {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.header-icon {
  color: #f59e0b;
}

.header-title {
  font-size: 0.88rem;
  font-weight: 700;
  color: #fff;
  letter-spacing: -0.01em;
}

.active-badge {
  font-size: 0.68rem;
  font-weight: 700;
  padding: 0.15rem 0.45rem;
  border-radius: 9999px;
  background: rgba(245, 158, 11, 0.2);
  color: #fbbf24;
  border: 1px solid rgba(245, 158, 11, 0.4);
}

.header-right {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.speed-pill {
  display: inline-flex;
  align-items: center;
  gap: 0.3rem;
  font-size: 0.72rem;
  font-weight: 700;
  color: #38bdf8;
  background: rgba(56, 189, 248, 0.12);
  padding: 0.2rem 0.5rem;
  border-radius: 6px;
  font-family: monospace;
}

.btn-close {
  background: transparent;
  border: none;
  color: #64748b;
  cursor: pointer;
  padding: 0.2rem;
  border-radius: 6px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s;
}

.btn-close:hover {
  color: #fff;
  background: rgba(255, 255, 255, 0.08);
}

.popover-body {
  max-height: 420px;
  overflow-y: auto;
  padding: 0.75rem 0.9rem;
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.downloads-section {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
}

.section-label {
  font-size: 0.7rem;
  font-weight: 700;
  color: #64748b;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  padding-left: 0.25rem;
}

.items-list {
  display: flex;
  flex-direction: column;
  gap: 0.45rem;
}

.download-card {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.6rem 0.75rem;
  border-radius: 12px;
  background: rgba(255, 255, 255, 0.03);
  border: 1px solid rgba(255, 255, 255, 0.06);
  cursor: pointer;
  transition: all 0.2s ease;
}

.download-card:hover {
  background: rgba(255, 255, 255, 0.06);
  border-color: rgba(255, 255, 255, 0.14);
}

.active-card {
  border-color: rgba(245, 158, 11, 0.25);
  background: rgba(245, 158, 11, 0.05);
}

.card-thumb {
  position: relative;
  width: 44px;
  height: 60px;
  border-radius: 6px;
  overflow: hidden;
  flex-shrink: 0;
  box-shadow: 0 4px 10px rgba(0, 0, 0, 0.4);
}

.card-thumb.small {
  width: 36px;
  height: 48px;
}

.thumb-img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.thumb-pulse {
  position: absolute;
  inset: 0;
  background: radial-gradient(circle, rgba(245, 158, 11, 0.4) 0%, transparent 70%);
  animation: thumbPulse 2s infinite ease-in-out;
}

.done-check {
  position: absolute;
  bottom: 2px;
  right: 2px;
  background: #10b981;
  color: #fff;
  border-radius: 50%;
  width: 14px;
  height: 14px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.card-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 0.3rem;
  min-width: 0;
}

.card-top {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
}

.game-name {
  font-size: 0.82rem;
  font-weight: 600;
  color: #f1f5f9;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.percent-text {
  font-size: 0.75rem;
  font-weight: 700;
  color: #fbbf24;
}

.date-text {
  font-size: 0.7rem;
  color: #64748b;
}

.progress-track {
  width: 100%;
  height: 4px;
  background: rgba(255, 255, 255, 0.08);
  border-radius: 2px;
  overflow: hidden;
}

.progress-fill {
  height: 100%;
  background: linear-gradient(90deg, #f59e0b, #fbbf24);
  border-radius: 2px;
  transition: width 0.3s ease;
}

.progress-fill.staging {
  background: linear-gradient(90deg, #38bdf8, #818cf8);
}

.card-bottom {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.bytes-text {
  font-size: 0.72rem;
  color: #94a3b8;
  font-family: monospace;
}

.state-pill {
  font-size: 0.65rem;
  font-weight: 600;
  padding: 0.1rem 0.4rem;
  border-radius: 4px;
  background: rgba(245, 158, 11, 0.15);
  color: #fbbf24;
}

.state-pill.staging {
  background: rgba(56, 189, 248, 0.15);
  color: #38bdf8;
}

.btn-play-quick {
  display: inline-flex;
  align-items: center;
  gap: 0.3rem;
  background: rgba(16, 185, 129, 0.15);
  color: #34d399;
  border: 1px solid rgba(16, 185, 129, 0.3);
  font-size: 0.7rem;
  font-weight: 700;
  padding: 0.2rem 0.55rem;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.15s ease;
}

.btn-play-quick:hover {
  background: #10b981;
  color: #fff;
  border-color: #10b981;
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 2.5rem 1.5rem;
  text-align: center;
  gap: 0.4rem;
}

.empty-icon-wrap {
  width: 44px;
  height: 44px;
  border-radius: 12px;
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid rgba(255, 255, 255, 0.08);
  display: flex;
  align-items: center;
  justify-content: center;
  color: #64748b;
  margin-bottom: 0.3rem;
}

.empty-title {
  margin: 0;
  font-size: 0.85rem;
  font-weight: 600;
  color: #e2e8f0;
}

.empty-subtitle {
  margin: 0;
  font-size: 0.74rem;
  color: #64748b;
  line-height: 1.35;
}

@keyframes thumbPulse {
  0%, 100% { opacity: 0.2; }
  50% { opacity: 0.7; }
}

@keyframes popoverFadeIn {
  from { opacity: 0; transform: translateY(-6px) scale(0.98); }
  to { opacity: 1; transform: translateY(0) scale(1); }
}
</style>
