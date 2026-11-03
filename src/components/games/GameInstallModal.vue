<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import type { Game } from '@/models/game'
import { useGamesStore, type SteamLibraryFolder } from '@/stores/gamesStore'
import { useToastStore } from '@/stores/toastStore'

const props = defineProps<{
  game: Game
  isOpen: boolean
}>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'installed', game: Game): void
}>()

const gamesStore = useGamesStore()
const toast = useToastStore()

const libraries = ref<SteamLibraryFolder[]>([])
const selectedFolderIndex = ref<number>(0)
const createShortcut = ref<boolean>(true)
const isLoadingLibraries = ref(true)
const isSubmitting = ref(false)
const installSuccess = ref(false)

onMounted(async () => {
  try {
    const list = await gamesStore.getSteamLibraries()
    libraries.value = list
    // Select first available folder
    const available = list.find((l) => l.available)
    if (available) {
      selectedFolderIndex.value = available.index
    }
  } catch (e) {
    console.warn('[GameInstallModal] Erreur de récupération des bibliothèques:', e)
  } finally {
    isLoadingLibraries.value = false
  }
})

const selectedFolder = computed(() => {
  return libraries.value.find((l) => l.index === selectedFolderIndex.value)
})

const estimatedSize = computed(() => {
  if (props.game.sizeGb && props.game.sizeGb > 0) {
    return `${props.game.sizeGb.toFixed(1)} Go`
  }
  return 'Environ 5 à 25 Go'
})

async function handleConfirmInstall() {
  if (isSubmitting.value || installSuccess.value) return
  isSubmitting.value = true

  try {
    const res = await gamesStore.installGame(props.game.id, selectedFolderIndex.value)
    if (res && res.success) {
      installSuccess.value = true
      toast.show(
        `Téléchargement de ${props.game.title} lancé dans Steam (sans quitter Loom) !`,
        'success'
      )
      emit('installed', props.game)
      setTimeout(() => {
        emit('close')
      }, 1200)
    } else {
      toast.show(res?.message || 'Erreur lors de la transmission à Steam', 'error')
    }
  } catch (err: any) {
    toast.show(`Erreur d'installation : ${err.message}`, 'error')
  } finally {
    isSubmitting.value = false
  }
}
</script>

<template>
  <div v-if="isOpen" class="modal-overlay" @click.self="emit('close')">
    <div class="modal-card">
      <!-- Background artwork glow -->
      <div
        class="modal-backdrop-cover"
        :style="{ backgroundImage: `url(${game.bannerUrl || game.coverUrl})` }"
      ></div>
      <div class="modal-backdrop-overlay"></div>

      <!-- Header with Close Button -->
      <header class="modal-header">
        <div class="header-game-info">
          <div class="cover-thumbnail">
            <img :src="game.coverUrl" :alt="game.title" class="cover-img" />
          </div>
          <div class="header-meta">
            <div class="platform-badge">
              <svg class="steam-icon" viewBox="0 0 24 24" fill="currentColor">
                <path
                  d="M11.979 0C5.678 0 .511 4.86.022 11.037l6.432 2.658c.545-.371 1.203-.59 1.912-.59.063 0 .125.004.188.006l2.861-4.142V8.91c0-2.495 2.028-4.524 4.524-4.524 2.494 0 4.524 2.029 4.524 4.524s-2.03 4.524-4.524 4.524h-.105l-4.076 2.911c0 .052.005.105.005.159 0 1.875-1.515 3.396-3.39 3.396-1.635 0-3.016-1.173-3.331-2.727L.436 15.27C1.862 20.307 6.486 24 11.979 24c6.627 0 12-5.373 12-12S18.605 0 11.979 0zm-3.613 18.067c-1.025 0-1.859-.834-1.859-1.859s.834-1.859 1.859-1.859 1.859.834 1.859 1.859-.834 1.859-1.859 1.859zm7.573-9.157c-1.635 0-2.966-1.332-2.966-2.967s1.331-2.966 2.966-2.966 2.967 1.331 2.967 2.966-1.332 2.967-2.967 2.967z"
                />
              </svg>
              <span>Steam</span>
            </div>
            <h2 class="game-title">{{ game.title }}</h2>
            <p class="game-dev">{{ game.developer }}</p>
          </div>
        </div>

        <button class="btn-close" @click="emit('close')" aria-label="Fermer">
          <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </header>

      <!-- Modal Body -->
      <div class="modal-body">
        <!-- Notice badge: Silent installation -->
        <div class="silent-notice-box">
          <div class="notice-icon">
            <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z" />
            </svg>
          </div>
          <div class="notice-text">
            <strong>Installation directe sans quitter Loom</strong>
            <p>
              Loom envoie directement l'ordre de téléchargement au moteur Steam en arrière-plan.
              Vous n'avez pas besoin d'ouvrir ni de valider l'interface Steam.
            </p>
          </div>
        </div>

        <!-- Disk requirements summary -->
        <div class="requirements-row">
          <div class="req-item">
            <span class="req-label">Espace requis estimé</span>
            <span class="req-value">{{ estimatedSize }}</span>
          </div>
          <div class="req-item">
            <span class="req-label">Espace disponible sur le lecteur sélectionné</span>
            <span class="req-value highlight" v-if="selectedFolder">
              {{ selectedFolder.freeGb }} Go libres
            </span>
            <span class="req-value" v-else>Non déterminé</span>
          </div>
        </div>

        <!-- Destination Selection -->
        <div class="section-title-row">
          <h3 class="section-title">Emplacement d'installation Steam</h3>
          <span class="section-subtitle">Dossiers de bibliothèques détectés sur votre machine</span>
        </div>

        <div v-if="isLoadingLibraries" class="loading-state">
          <div class="spinner"></div>
          <span>Détection des disques et bibliothèques Steam...</span>
        </div>

        <div v-else-if="libraries.length === 0" class="empty-libraries">
          <p>Aucun dossier de bibliothèque Steam trouvé dans la configuration locale.</p>
        </div>

        <div v-else class="library-options-grid">
          <label
            v-for="folder in libraries"
            :key="folder.index"
            class="library-option-card"
            :class="{
              selected: selectedFolderIndex === folder.index,
              disabled: !folder.available
            }"
          >
            <input
              type="radio"
              name="steam-library-folder"
              :value="folder.index"
              v-model="selectedFolderIndex"
              :disabled="!folder.available"
              class="radio-input"
            />
            <div class="radio-custom">
              <div class="radio-dot"></div>
            </div>

            <div class="library-details">
              <div class="lib-top-row">
                <div class="lib-name-group">
                  <svg class="drive-icon" viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor">
                    <rect x="2" y="5" width="20" height="14" rx="2" stroke-width="2" />
                    <line x1="6" y1="14" x2="6" y2="14" stroke-width="3" stroke-linecap="round" />
                    <line x1="18" y1="14" x2="18" y2="14" stroke-width="3" stroke-linecap="round" />
                  </svg>
                  <span class="lib-name">{{ folder.label }}</span>
                  <span v-if="folder.isDefault" class="default-badge">Par défaut</span>
                </div>
                <span v-if="folder.available" class="lib-free-space">
                  {{ folder.freeGb }} Go libres
                </span>
                <span v-else class="offline-badge">
                  Non monté
                </span>
              </div>

              <div class="lib-path">{{ folder.path }}</div>

              <div v-if="folder.available" class="storage-bar-track">
                <div
                  class="storage-bar-fill"
                  :style="{ width: `${folder.usedPercent}%` }"
                  :class="{
                    danger: folder.usedPercent > 90,
                    warning: folder.usedPercent > 75
                  }"
                ></div>
              </div>
            </div>
          </label>
        </div>

        <!-- Options: Shortcuts -->
        <div class="options-group">
          <label class="checkbox-label">
            <input type="checkbox" v-model="createShortcut" />
            <span class="custom-checkbox">
              <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor">
                <polyline points="20 6 9 17 4 12" stroke-width="3" stroke-linecap="round" stroke-linejoin="round" />
              </svg>
            </span>
            <span class="option-text">Ajouter un raccourci Loom sur le bureau et le menu des applications</span>
          </label>
        </div>
      </div>

      <!-- Modal Footer -->
      <footer class="modal-footer">
        <button type="button" class="btn-cancel" @click="emit('close')" :disabled="isSubmitting">
          Annuler
        </button>

        <button
          type="button"
          class="btn-install-confirm"
          @click="handleConfirmInstall"
          :disabled="isSubmitting || (selectedFolder && !selectedFolder.available)"
          :class="{ success: installSuccess }"
        >
          <template v-if="isSubmitting">
            <div class="spinner-small"></div>
            <span>Envoi de l'ordre à Steam...</span>
          </template>
          <template v-else-if="installSuccess">
            <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor">
              <polyline points="20 6 9 17 4 12" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" />
            </svg>
            <span>Téléchargement lancé !</span>
          </template>
          <template v-else>
            <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" />
            </svg>
            <span>Lancer l'installation</span>
          </template>
        </button>
      </footer>
    </div>
  </div>
</template>

<style scoped>
.modal-overlay {
  position: fixed;
  inset: 0;
  z-index: 1000;
  background: rgba(8, 10, 15, 0.75);
  backdrop-filter: blur(12px);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 1.5rem;
  animation: fadeIn 0.2s cubic-bezier(0.16, 1, 0.3, 1);
}

.modal-card {
  position: relative;
  width: 100%;
  max-width: 640px;
  background: #11141c;
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 18px;
  box-shadow: 0 25px 60px rgba(0, 0, 0, 0.65), 0 0 40px rgba(99, 102, 241, 0.1);
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.modal-backdrop-cover {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 180px;
  background-size: cover;
  background-position: center;
  opacity: 0.18;
  filter: blur(16px);
  pointer-events: none;
}

.modal-backdrop-overlay {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 200px;
  background: linear-gradient(180deg, rgba(17, 20, 28, 0.2) 0%, #11141c 100%);
  pointer-events: none;
}

.modal-header {
  position: relative;
  z-index: 1;
  padding: 1.5rem 1.75rem 1.25rem;
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
}

.header-game-info {
  display: flex;
  align-items: center;
  gap: 1.25rem;
}

.cover-thumbnail {
  width: 68px;
  height: 98px;
  border-radius: 8px;
  overflow: hidden;
  box-shadow: 0 8px 20px rgba(0, 0, 0, 0.5);
  border: 1px solid rgba(255, 255, 255, 0.15);
  flex-shrink: 0;
}

.cover-img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.header-meta {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.platform-badge {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  background: rgba(255, 255, 255, 0.08);
  padding: 0.2rem 0.55rem;
  border-radius: 6px;
  font-size: 0.75rem;
  font-weight: 600;
  color: #a5b4fc;
  width: fit-content;
}

.steam-icon {
  width: 13px;
  height: 13px;
}

.game-title {
  margin: 0;
  font-size: 1.35rem;
  font-weight: 700;
  color: #fff;
  letter-spacing: -0.01em;
}

.game-dev {
  margin: 0;
  font-size: 0.85rem;
  color: #94a3b8;
}

.btn-close {
  background: rgba(255, 255, 255, 0.06);
  border: 1px solid rgba(255, 255, 255, 0.08);
  color: #94a3b8;
  width: 34px;
  height: 34px;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: all 0.2s;
}

.btn-close:hover {
  background: rgba(255, 255, 255, 0.12);
  color: #fff;
}

.modal-body {
  position: relative;
  z-index: 1;
  padding: 1.5rem 1.75rem;
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
  max-height: 65vh;
  overflow-y: auto;
}

.silent-notice-box {
  display: flex;
  gap: 0.85rem;
  align-items: flex-start;
  padding: 0.9rem 1.1rem;
  background: rgba(99, 102, 241, 0.1);
  border: 1px solid rgba(99, 102, 241, 0.25);
  border-radius: 12px;
}

.notice-icon {
  color: #818cf8;
  flex-shrink: 0;
  margin-top: 2px;
}

.notice-text strong {
  display: block;
  font-size: 0.88rem;
  color: #c7d2fe;
  margin-bottom: 0.2rem;
}

.notice-text p {
  margin: 0;
  font-size: 0.8rem;
  line-height: 1.4;
  color: #94a3b8;
}

.requirements-row {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 1rem;
  background: rgba(255, 255, 255, 0.03);
  border: 1px solid rgba(255, 255, 255, 0.06);
  border-radius: 10px;
  padding: 0.85rem 1.1rem;
}

.req-item {
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
}

.req-label {
  font-size: 0.75rem;
  color: #64748b;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  font-weight: 600;
}

.req-value {
  font-size: 0.95rem;
  font-weight: 600;
  color: #e2e8f0;
}

.req-value.highlight {
  color: #38bdf8;
}

.section-title-row {
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
}

.section-title {
  margin: 0;
  font-size: 0.92rem;
  font-weight: 600;
  color: #f1f5f9;
}

.section-subtitle {
  font-size: 0.78rem;
  color: #64748b;
}

.library-options-grid {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.library-option-card {
  position: relative;
  display: flex;
  align-items: center;
  gap: 1rem;
  padding: 0.95rem 1.15rem;
  background: rgba(255, 255, 255, 0.03);
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 12px;
  cursor: pointer;
  transition: all 0.2s ease;
}

.library-option-card:hover:not(.disabled) {
  background: rgba(255, 255, 255, 0.06);
  border-color: rgba(255, 255, 255, 0.16);
}

.library-option-card.selected {
  background: rgba(99, 102, 241, 0.12);
  border-color: #6366f1;
  box-shadow: 0 0 20px rgba(99, 102, 241, 0.15);
}

.library-option-card.disabled {
  opacity: 0.45;
  cursor: not-allowed;
}

.radio-input {
  display: none;
}

.radio-custom {
  width: 18px;
  height: 18px;
  border-radius: 50%;
  border: 2px solid rgba(255, 255, 255, 0.3);
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  transition: all 0.2s;
}

.library-option-card.selected .radio-custom {
  border-color: #6366f1;
}

.radio-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: #6366f1;
  opacity: 0;
  transform: scale(0.5);
  transition: all 0.2s;
}

.library-option-card.selected .radio-dot {
  opacity: 1;
  transform: scale(1);
}

.library-details {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
}

.lib-top-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.lib-name-group {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.drive-icon {
  color: #818cf8;
}

.lib-name {
  font-size: 0.9rem;
  font-weight: 600;
  color: #f8fafc;
}

.default-badge {
  font-size: 0.7rem;
  font-weight: 600;
  background: rgba(56, 189, 248, 0.15);
  color: #38bdf8;
  padding: 0.15rem 0.45rem;
  border-radius: 4px;
}

.offline-badge {
  font-size: 0.7rem;
  font-weight: 600;
  background: rgba(239, 68, 68, 0.15);
  color: #ef4444;
  padding: 0.15rem 0.45rem;
  border-radius: 4px;
}

.lib-free-space {
  font-size: 0.85rem;
  font-weight: 600;
  color: #38bdf8;
}

.lib-path {
  font-size: 0.76rem;
  color: #64748b;
  font-family: monospace;
}

.storage-bar-track {
  width: 100%;
  height: 4px;
  background: rgba(255, 255, 255, 0.08);
  border-radius: 2px;
  overflow: hidden;
  margin-top: 0.2rem;
}

.storage-bar-fill {
  height: 100%;
  background: linear-gradient(90deg, #6366f1, #38bdf8);
  border-radius: 2px;
  transition: width 0.3s;
}

.storage-bar-fill.warning {
  background: #f59e0b;
}

.storage-bar-fill.danger {
  background: #ef4444;
}

.loading-state {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 1.5rem;
  background: rgba(255, 255, 255, 0.02);
  border-radius: 10px;
  color: #94a3b8;
  font-size: 0.85rem;
}

.options-group {
  padding-top: 0.5rem;
}

.checkbox-label {
  display: inline-flex;
  align-items: center;
  gap: 0.65rem;
  cursor: pointer;
  user-select: none;
}

.checkbox-label input {
  display: none;
}

.custom-checkbox {
  width: 18px;
  height: 18px;
  border-radius: 4px;
  border: 1px solid rgba(255, 255, 255, 0.25);
  background: rgba(255, 255, 255, 0.04);
  display: flex;
  align-items: center;
  justify-content: center;
  color: transparent;
  transition: all 0.2s;
}

.checkbox-label input:checked + .custom-checkbox {
  background: #6366f1;
  border-color: #6366f1;
  color: #fff;
}

.option-text {
  font-size: 0.82rem;
  color: #cbd5e1;
}

.modal-footer {
  position: relative;
  z-index: 1;
  padding: 1.1rem 1.75rem;
  background: rgba(0, 0, 0, 0.2);
  border-top: 1px solid rgba(255, 255, 255, 0.06);
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 0.85rem;
}

.btn-cancel {
  background: transparent;
  border: 1px solid rgba(255, 255, 255, 0.1);
  color: #94a3b8;
  padding: 0.65rem 1.25rem;
  border-radius: 10px;
  font-size: 0.85rem;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;
}

.btn-cancel:hover:not(:disabled) {
  background: rgba(255, 255, 255, 0.06);
  color: #f1f5f9;
}

.btn-install-confirm {
  display: inline-flex;
  align-items: center;
  gap: 0.55rem;
  background: linear-gradient(135deg, #6366f1 0%, #4f46e5 100%);
  border: none;
  color: #fff;
  padding: 0.65rem 1.4rem;
  border-radius: 10px;
  font-size: 0.88rem;
  font-weight: 600;
  cursor: pointer;
  box-shadow: 0 4px 15px rgba(99, 102, 241, 0.35);
  transition: all 0.2s ease;
}

.btn-install-confirm:hover:not(:disabled) {
  transform: translateY(-1px);
  box-shadow: 0 6px 20px rgba(99, 102, 241, 0.5);
  filter: brightness(1.1);
}

.btn-install-confirm:disabled {
  opacity: 0.5;
  cursor: not-allowed;
  transform: none;
}

.btn-install-confirm.success {
  background: #10b981;
  box-shadow: 0 4px 15px rgba(16, 185, 129, 0.4);
}

.spinner, .spinner-small {
  border: 2px solid rgba(255, 255, 255, 0.2);
  border-top-color: currentColor;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

.spinner {
  width: 18px;
  height: 18px;
  color: #6366f1;
}

.spinner-small {
  width: 14px;
  height: 14px;
  color: #fff;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

@keyframes fadeIn {
  from { opacity: 0; transform: scale(0.97); }
  to { opacity: 1; transform: scale(1); }
}
</style>
