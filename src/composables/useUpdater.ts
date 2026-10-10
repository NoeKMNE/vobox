import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { ref } from 'vue'

export interface UpdateInfo {
  available: boolean
  current_version: string
  version: string | null
  notes: string | null
  pub_date: string | null
}

export type UpdateStatus =
  | 'idle'
  | 'checking'
  | 'available'
  | 'downloading'
  | 'ready'
  | 'uptodate'
  | 'error'

export const updateStatus = ref<UpdateStatus>('idle')
export const updateInfo = ref<UpdateInfo | null>(null)
export const updateError = ref('')
export const updateProgress = ref<{ downloaded: number; total: number | null }>({
  downloaded: 0,
  total: null
})

let listening = false

const ensureListeners = async () => {
  if (listening) return
  try {
    await listen<{ downloaded: number; total: number | null }>(
      'Vobox-update-progress',
      (e) => {
        updateProgress.value = e.payload
      }
    )
    await listen('Vobox-update-installed', () => {
      updateStatus.value = 'ready'
    })
    listening = true
  } catch {
    // hors Tauri (navigateur) : pas d'événements, on ignore
  }
}

const isBusy = () => updateStatus.value === 'checking' || updateStatus.value === 'downloading'

/** Vérifie une mise à jour. Au démarrage (manual=false) : silencieux sauf si dispo. */
export async function checkForUpdate(manual = false) {
  if (isBusy()) return
  updateError.value = ''
  updateStatus.value = 'checking'
  try {
    await ensureListeners()
    const info = await invoke<UpdateInfo>('check_update')
    updateInfo.value = info
    updateStatus.value = info.available ? 'available' : manual ? 'uptodate' : 'idle'
  } catch (e) {
    updateError.value = typeof e === 'string' ? e : 'Vérification impossible'
    updateStatus.value = manual ? 'error' : 'idle'
  }
}

/** Vérification silencieuse au lancement de l'app (popup uniquement si dispo). */
export function initUpdater() {
  void checkForUpdate(false)
}

export async function installUpdate() {
  if (isBusy()) return
  updateError.value = ''
  updateStatus.value = 'downloading'
  updateProgress.value = { downloaded: 0, total: null }
  try {
    await invoke('install_update')
    // Sur Windows l'installeur se lance et ferme l'app ici même.
    updateStatus.value = 'ready'
  } catch (e) {
    updateError.value = typeof e === 'string' ? e : 'Installation impossible'
    updateStatus.value = 'error'
  }
}

export function dismissUpdate() {
  if (updateStatus.value !== 'downloading' && updateStatus.value !== 'checking') {
    updateStatus.value = 'idle'
  }
}

export function restartApp() {
  invoke('restart_app').catch(() => {})
}

export function formatBytes(n: number) {
  if (n < 1024) return `${n} o`
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} Ko`
  return `${(n / 1024 / 1024).toFixed(1)} Mo`
}
