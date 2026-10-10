<script setup lang="ts">
import { computed } from 'vue'
import { ArrowDownToLine, Loader, RotateCcw, X } from '@lucide/vue'
import {
  dismissUpdate,
  formatBytes,
  installUpdate,
  restartApp,
  updateError,
  updateInfo,
  updateProgress,
  updateStatus
} from '../composables/useUpdater'

const visible = computed(
  () =>
    updateStatus.value === 'available' ||
    updateStatus.value === 'downloading' ||
    updateStatus.value === 'ready' ||
    updateStatus.value === 'error'
)

const percent = computed(() => {
  const { downloaded, total } = updateProgress.value
  if (!total || total <= 0) return null
  return Math.min(100, Math.round((downloaded / total) * 100))
})

const title = computed(() => {
  switch (updateStatus.value) {
    case 'downloading':
      return 'Téléchargement de la mise à jour…'
    case 'ready':
      return 'Mise à jour prête'
    case 'error':
      return 'Mise à jour impossible'
    default:
      return 'Nouvelle version disponible'
  }
})
</script>

<template>
  <div v-if="visible" class="update-overlay" @click.self="dismissUpdate">
    <div class="update-modal" role="alertdialog" aria-modal="true" aria-labelledby="update-title">
      <button
        v-if="updateStatus !== 'downloading'"
        type="button"
        class="modal-close"
        aria-label="Fermer"
        @click="dismissUpdate"
      >
        <X :size="16" />
      </button>

      <div class="update-icon" aria-hidden="true">
        <ArrowDownToLine :size="22" />
      </div>

      <h3 id="update-title">{{ title }}</h3>
      <p v-if="updateInfo?.version" class="update-version">
        v{{ updateInfo.current_version }} → <b>v{{ updateInfo.version }}</b>
        <span v-if="updateInfo.pub_date" class="update-date">· {{ updateInfo.pub_date }}</span>
      </p>

      <p v-if="updateInfo?.notes" class="update-notes">{{ updateInfo.notes }}</p>

      <div v-if="updateStatus === 'downloading'" class="progress-wrap">
        <div class="progress-track" role="progressbar" :aria-valuenow="percent ?? undefined" aria-valuemin="0" aria-valuemax="100">
          <div
            class="progress-fill"
            :class="{ indeterminate: percent === null }"
            :style="percent !== null ? { width: percent + '%' } : undefined"
          />
        </div>
        <span class="progress-label">
          <template v-if="percent !== null">{{ percent }} %</template>
          <template v-else>{{ formatBytes(updateProgress.downloaded) }} téléchargés…</template>
        </span>
      </div>

      <p v-if="updateStatus === 'ready'" class="ready-text">
        La mise à jour est installée. Redémarrez Vobox pour l’appliquer.
      </p>
      <p v-if="updateStatus === 'error' && updateError" class="error-text">{{ updateError }}</p>

      <div class="update-actions">
        <button
          v-if="updateStatus === 'available' || updateStatus === 'error'"
          type="button"
          class="ghost-btn"
          @click="dismissUpdate"
        >
          Plus tard
        </button>
        <button
          v-if="updateStatus === 'available' || updateStatus === 'error'"
          type="button"
          class="primary-btn"
          @click="installUpdate"
        >
          Installer la mise à jour
        </button>
        <button
          v-if="updateStatus === 'ready'"
          type="button"
          class="primary-btn"
          @click="restartApp"
        >
          <RotateCcw :size="15" /> Redémarrer maintenant
        </button>
        <span v-if="updateStatus === 'downloading'" class="download-hint">
          <Loader :size="14" class="spin" /> Ne fermez pas l’application…
        </span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.update-overlay {
  position: fixed;
  inset: 0;
  z-index: 60;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 20px;
  background: rgba(0, 0, 0, 0.5);
}
.update-modal {
  position: relative;
  width: min(440px, 100%);
  padding: 26px 24px 22px;
  text-align: center;
  border-radius: var(--shape-xl);
  background: var(--surface-container-low);
  box-shadow: var(--elev-3);
}
.modal-close {
  position: absolute;
  top: 10px;
  right: 10px;
  width: 32px;
  height: 32px;
  display: grid;
  place-items: center;
  border: none;
  border-radius: 50%;
  background: transparent;
  color: var(--on-surface-variant);
  cursor: pointer;
}
.modal-close:hover {
  background: var(--surface-container-high);
}
.update-icon {
  width: 52px;
  height: 52px;
  margin: 0 auto 12px;
  display: grid;
  place-items: center;
  border-radius: var(--shape-lg);
  background: var(--primary-container);
  color: var(--on-primary-container);
}
.update-modal h3 {
  margin: 0 0 6px;
  font-size: 19px;
  color: var(--on-surface);
}
.update-version {
  margin: 0 0 8px;
  font-size: 14px;
  color: var(--on-surface-variant);
}
.update-version b {
  color: var(--primary);
}
.update-date {
  font-size: 12.5px;
}
.update-notes {
  margin: 0 0 14px;
  padding: 10px 12px;
  font-size: 13px;
  line-height: 1.55;
  text-align: left;
  white-space: pre-wrap;
  color: var(--on-surface-variant);
  background: var(--surface-container-high);
  border-radius: var(--shape-md);
  max-height: 140px;
  overflow-y: auto;
}
.progress-wrap {
  display: flex;
  align-items: center;
  gap: 10px;
  margin: 4px 0 12px;
}
.progress-track {
  flex: 1;
  height: 8px;
  border-radius: 99px;
  background: var(--surface-container-highest);
  overflow: hidden;
}
.progress-fill {
  height: 100%;
  border-radius: 99px;
  background: var(--primary);
  transition: width 200ms ease;
}
.progress-fill.indeterminate {
  width: 40%;
  animation: slide 1.1s ease-in-out infinite alternate;
}
@keyframes slide {
  from { transform: translateX(-30%); }
  to { transform: translateX(180%); }
}
.progress-label {
  min-width: 110px;
  text-align: right;
  font-size: 12px;
  font-weight: 700;
  color: var(--on-surface-variant);
}
.ready-text {
  margin: 0 0 12px;
  font-size: 13.5px;
  color: var(--on-surface-variant);
}
.error-text {
  margin: 0 0 12px;
  font-size: 13px;
  color: var(--error);
}
.update-actions {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 10px;
  flex-wrap: wrap;
}
.ghost-btn {
  padding: 10px 18px;
  border-radius: var(--shape-full);
  border: 1px solid var(--outline-variant);
  background: transparent;
  color: var(--on-surface);
  font-size: 13.5px;
  font-weight: 600;
  cursor: pointer;
}
.ghost-btn:hover {
  border-color: var(--primary);
  color: var(--primary);
}
.primary-btn {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  padding: 11px 22px;
  border: none;
  border-radius: var(--shape-full);
  background: var(--primary);
  color: var(--on-primary);
  font-size: 13.5px;
  font-weight: 700;
  cursor: pointer;
}
.primary-btn:hover {
  box-shadow: var(--elev-1);
}
.download-hint {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  font-size: 12.5px;
  color: var(--on-surface-variant);
}
.spin {
  animation: spin 0.9s linear infinite;
}
@keyframes spin {
  to { transform: rotate(360deg); }
}
</style>
