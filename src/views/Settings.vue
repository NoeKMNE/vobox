<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'
import { Camera, Shield, Trash2, TriangleAlert } from '@lucide/vue'
import { useTheme, type Theme } from '../composables/useTheme'
import { checkForUpdate, updateError, updateStatus } from '../composables/useUpdater'
import { getVersion } from '@tauri-apps/api/app'
import { getInclureNouveaux, getSession, getTelemetryConsent, getToken, resetData, setInclureNouveaux, setTelemetryConsent, updateAvatar, updateProfile } from '../composables/useAuth'

const { currentTheme, setTheme } = useTheme()

const avatar = ref('')
const profile = ref<{ name: string; email: string } | null>(null)
const editName = ref('')
const editEmail = ref('')
const saving = ref(false)
const saveMessage = ref('')
const saveError = ref('')
const fileInput = ref<HTMLInputElement | null>(null)

const appVersion = ref('')

const telemetryEnabled = ref(false)
const telemetrySaving = ref(false)

const inclureNouveaux = ref(true)
const inclureNouveauxSaving = ref(false)

const toggleInclureNouveaux = async () => {
  const next = !inclureNouveaux.value
  inclureNouveauxSaving.value = true
  try {
    await setInclureNouveaux(next)
    inclureNouveaux.value = next
  } catch {
    // le toggle reste sur son état précédent
  } finally {
    inclureNouveauxSaving.value = false
  }
}

const toggleTelemetry = async () => {
  const next = !telemetryEnabled.value
  telemetrySaving.value = true
  try {
    await setTelemetryConsent(next)
    telemetryEnabled.value = next
  } catch {
    // le toggle reste sur son état précédent
  } finally {
    telemetrySaving.value = false
  }
}

const showDanger = ref(false)
const countdown = ref(0)
const deleting = ref(false)
const dangerDone = ref('')
const dangerError = ref('')
const COMPTE_REBOURS = 10
let dangerTimer: ReturnType<typeof setInterval> | null = null

const stopDangerTimer = () => {
  if (dangerTimer) {
    clearInterval(dangerTimer)
    dangerTimer = null
  }
}

const openDanger = () => {
  showDanger.value = true
  deleting.value = false
  dangerDone.value = ''
  dangerError.value = ''
  countdown.value = COMPTE_REBOURS
  stopDangerTimer()
  dangerTimer = setInterval(() => {
    countdown.value -= 1
    if (countdown.value <= 0) stopDangerTimer()
  }, 1000)
}

const closeDanger = () => {
  stopDangerTimer()
  showDanger.value = false
}

const confirmDanger = async () => {
  const token = getToken()
  if (!token || countdown.value > 0 || deleting.value) return
  deleting.value = true
  dangerError.value = ''
  try {
    await resetData(token)
    dangerDone.value = 'Toutes les données ont été supprimées.'
  } catch (e) {
    dangerError.value = typeof e === 'string' ? e : e instanceof Error ? e.message : 'Suppression impossible'
  } finally {
    deleting.value = false
  }
}

onUnmounted(stopDangerTimer)

const initials = () => {
  const name = profile.value?.name?.trim() || 'A'
  const parts = name.split(/\s+/)
  return (parts[0]?.[0] || 'A') + (parts[1]?.[0] || '')
}

onMounted(async () => {
  const token = getToken()
  if (!token) return
  try {
    const user = await getSession(token)
    profile.value = { name: user.name, email: user.email }
    editName.value = user.name
    editEmail.value = user.email
    avatar.value = user.avatar ?? ''
  } catch {
    profile.value = null
  }
  getVersion()
    .then((v) => (appVersion.value = v))
    .catch(() => {})
  getTelemetryConsent()
    .then((v) => (telemetryEnabled.value = v))
    .catch(() => {})
  getInclureNouveaux()
    .then((v) => (inclureNouveaux.value = v))
    .catch(() => {})
})

const saveProfile = async () => {
  const token = getToken()
  if (!token) return
  saving.value = true
  saveMessage.value = ''
  saveError.value = ''
  try {
    const user = await updateProfile(token, editName.value, editEmail.value)
    profile.value = { name: user.name, email: user.email }
    saveMessage.value = 'Profil enregistré'
  } catch (e) {
    saveError.value =
      typeof e === 'string' ? e : e instanceof Error ? e.message : 'Une erreur est survenue'
  } finally {
    saving.value = false
  }
}

const openPicker = () => fileInput.value?.click()

const onFileChange = (e: Event) => {
  const input = e.target as HTMLInputElement
  const file = input.files?.[0]
  if (!file || !file.type.startsWith('image/')) return
  const reader = new FileReader()
  reader.onload = async () => {
    const dataUrl = reader.result as string
    await persistAvatar(dataUrl)
  }
  reader.readAsDataURL(file)
  input.value = ''
}

const removeAvatar = async () => {
  await persistAvatar(null)
}

const persistAvatar = async (dataUrl: string | null) => {
  const token = getToken()
  if (!token) return
  saveMessage.value = ''
  saveError.value = ''
  try {
    const user = await updateAvatar(token, dataUrl)
    avatar.value = user.avatar ?? ''
    saveMessage.value = dataUrl ? 'Photo de profil enregistrée' : 'Photo supprimée'
  } catch (e) {
    saveError.value =
      typeof e === 'string' ? e : e instanceof Error ? e.message : 'Une erreur est survenue'
  }
}

const themeOptions: { value: Theme; label: string; description: string; swatch: string[] }[] = [
  { value: 'light', label: 'Clair', description: 'Fond blanc, teintes vives', swatch: ['#ffffff', '#aa3bff'] },
  { value: 'dark', label: 'Sombre', description: 'Fond sombre reposant', swatch: ['#16171d', '#c084fc'] },
  { value: 'mono', label: 'Noir & Blanc', description: 'Monochrome élégant', swatch: ['#ffffff', '#000000'] },
  { value: 'bleu', label: 'Bleu', description: 'Océan frais et serein', swatch: ['#f5f9fd', '#1c7ed6'] },
  { value: 'neon', label: 'Néon', description: 'Dark agritech, accents néon', swatch: ['#070b12', '#00e5ff'] }
]
</script>

<template>
  <div class="settings">
    <header class="page-header">
      <div class="page-header-text">
        <h1>Paramètres</h1>
        <p>Personnalisez votre expérience Vobox</p>
      </div>
    </header>

    <div class="settings-body">
      <div class="settings-main">
        <section class="settings-section">
          <h2>Apparence</h2>
          <div class="theme-grid">
            <button
              v-for="option in themeOptions"
              :key="option.value"
              class="theme-card"
              :class="{ active: currentTheme === option.value }"
              :aria-pressed="currentTheme === option.value"
              @click="setTheme(option.value)"
            >
              <div class="theme-swatch" :style="{ background: option.swatch[0] }">
                <span class="swatch-accent" :style="{ background: option.swatch[1] }"></span>
                <svg
                  v-if="currentTheme === option.value"
                  class="check"
                  width="20"
                  height="20"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="#fff"
                  stroke-width="3"
                >
                  <polyline points="20 6 9 17 4 12" />
                </svg>
              </div>
              <div class="theme-meta">
                <strong>{{ option.label }}</strong>
                <small>{{ option.description }}</small>
              </div>
            </button>
          </div>
          <div class="notif-card update-card">
            <div class="update-head">
              <strong>Mises à jour automatiques</strong>
              <small>Une fenêtre vous prévient quand une nouvelle version stable est disponible.</small>
            </div>
            <div class="notif-row">
              <div class="notif-meta">
                <strong>Canal de mise à jour</strong>
                <small>Stable — versions testées uniquement</small>
              </div>
              <span class="channel-badge">Stable</span>
            </div>
            <div class="notif-row">
              <div class="notif-meta">
                <strong>Vérifier maintenant</strong>
                <small v-if="updateStatus === 'uptodate'" class="ok-text">Vous êtes à jour.</small>
                <small v-else-if="updateStatus === 'checking'">Vérification en cours…</small>
                <small v-else-if="updateStatus === 'error'">{{ updateError || 'Vérification impossible' }}</small>
                <small v-else>Vérifiée automatiquement à chaque lancement</small>
              </div>
              <button
                class="check-btn"
                :disabled="updateStatus === 'checking' || updateStatus === 'downloading'"
                @click="checkForUpdate(true)"
              >
                Vérifier
              </button>
            </div>
            <div class="version-row">
              <span>Version installée</span>
              <code>v{{ appVersion }}</code>
            </div>
          </div>
        </section>

        <section class="settings-section">
          <h2>Télémétrie et confidentialité</h2>
          <div class="notif-card">
            <div class="notif-row">
              <div class="notif-meta">
                <strong>Données d’usage anonymisées</strong>
                <small>Ouvertures, imports (nombres), fiches (statut). Jamais : mot de passe, contenus, clés API.</small>
              </div>
              <button
                class="switch"
                role="switch"
                :aria-checked="telemetryEnabled"
                :disabled="telemetrySaving"
                :class="{ on: telemetryEnabled }"
                @click="toggleTelemetry"
                :aria-label="telemetryEnabled ? 'Désactiver la télémétrie' : 'Activer la télémétrie'"
              >
                <span class="switch-knob" />
              </button>
            </div>
            <p class="telemetry-note">
              {{ telemetryEnabled
                ? 'Activée : événements conservés localement, envoyés uniquement si un endpoint de collecte est configuré.'
                : 'Désactivée : aucun événement n’est enregistré.' }}
            </p>
          </div>
        </section>

        <section class="settings-section">
          <h2>Fiches de traçabilité</h2>
          <div class="notif-card">
            <div class="notif-row">
              <div class="notif-meta">
                <strong>Inclure les producteurs sans participation</strong>
                <small>Proposés dans les fiches même s’ils n’ont encore jamais participé.</small>
              </div>
              <button
                class="switch"
                role="switch"
                :aria-checked="inclureNouveaux"
                :disabled="inclureNouveauxSaving"
                :class="{ on: inclureNouveaux }"
                @click="toggleInclureNouveaux"
                :aria-label="inclureNouveaux ? 'Exclure les nouveaux producteurs' : 'Inclure les nouveaux producteurs'"
              >
                <span class="switch-knob" />
              </button>
            </div>
            <p class="telemetry-note">
              {{ inclureNouveaux
                ? 'Activé : tous les producteurs avec du stock livrable sont proposés.'
                : 'Désactivé : seuls les producteurs ayant déjà participé sont proposés.' }}
            </p>
          </div>
        </section>

        <section class="settings-section danger-section">
          <h2>Zone dangereuse</h2>
          <div class="notif-card danger-card">
            <div class="notif-row">
              <div class="notif-meta">
                <strong>Supprimer toutes les données</strong>
                <small>Fiches, producteurs, cultures et unités — irréversible</small>
              </div>
              <button class="danger-btn" @click="openDanger">
                <Trash2 :size="14" />
                <span>Supprimer…</span>
              </button>
            </div>
          </div>
        </section>
      </div>

      <div class="settings-side">
        <section class="profile-card">
          <div class="profile-head">
            <div class="avatar-wrap">
              <img v-if="avatar" :src="avatar" alt="Photo de profil" class="avatar-img" />
              <span v-else class="avatar-fallback">{{ initials() }}</span>
              <button class="avatar-edit" type="button" @click="openPicker" title="Changer la photo">
                <Camera :size="16" />
              </button>
              <input
                ref="fileInput"
                type="file"
                accept="image/*"
                class="avatar-input"
                @change="onFileChange"
              />
            </div>
            <h2 class="profile-name">{{ profile?.name || 'Utilisateur' }}</h2>
            <p class="profile-sub">{{ profile?.email || 'Non connecté' }}</p>
          </div>

          <div class="profile-body">
            <div class="profile-actions">
              <button v-if="avatar" type="button" class="photo-link" @click="removeAvatar">
                Supprimer la photo
              </button>
              <span v-else class="photo-hint">JPG · PNG · max 2 MB</span>
            </div>

            <form class="profile-form" @submit.prevent="saveProfile">
              <label class="field">
                <span>Nom</span>
                <input v-model="editName" type="text" :disabled="saving" placeholder="Jean Dupont" />
              </label>
              <label class="field">
                <span>Email</span>
                <input v-model="editEmail" type="email" :disabled="saving" placeholder="jean@Vobox.fr" />
              </label>
              <button class="save-btn" type="submit" :disabled="saving">
                <span v-if="saving" class="spinner"></span>
                {{ saving ? 'Enregistrement…' : 'Enregistrer' }}
              </button>
              <p v-if="saveMessage" class="save-msg ok">{{ saveMessage }}</p>
              <p v-if="saveError" class="save-msg err">{{ saveError }}</p>
            </form>

            <div class="profile-rows">
              <div class="profile-row">
                <span class="row-icon"><Shield :size="15" /></span>
                <span class="row-label">Rôle</span>
                <span class="row-value">Exploitant</span>
              </div>
            </div>
          </div>
        </section>
      </div>
    </div>
  </div>

  <div v-if="showDanger" class="danger-overlay" @click.self="closeDanger">
    <div class="danger-modal" role="alertdialog" aria-modal="true" aria-labelledby="danger-title">
      <div class="danger-icon"><TriangleAlert :size="22" /></div>
      <h3 id="danger-title">Supprimer toutes les données ?</h3>
      <p class="danger-text">
        Cette action supprime <b>définitivement</b> toutes les fiches de traçabilité, les producteurs,
        les cultures et les unités agricoles de ce compte. Elle est <b>irréversible</b>.
      </p>
      <p v-if="countdown > 0" class="danger-count">
        Réfléchissez bien… suppression possible dans <b>{{ countdown }} s</b>
      </p>
      <p v-else-if="!dangerDone" class="danger-count ready">Vous pouvez encore annuler.</p>
      <p v-if="dangerDone" class="save-msg ok">{{ dangerDone }}</p>
      <p v-if="dangerError" class="save-msg err">{{ dangerError }}</p>
      <div class="danger-actions">
        <button class="ghost-btn" :disabled="deleting" @click="closeDanger">Annuler</button>
        <button
          v-if="!dangerDone"
          class="danger-confirm"
          :disabled="countdown > 0 || deleting"
          @click="confirmDanger"
        >
          <span v-if="deleting" class="spinner"></span>
          {{ deleting ? 'Suppression…' : countdown > 0 ? `Supprimer (${countdown})` : 'Supprimer définitivement' }}
        </button>
        <button v-else class="save-btn" @click="closeDanger">Fermer</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.settings {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow-y: auto;
  padding: 8px 8px 32px;
  scrollbar-width: thin;
  scrollbar-color: var(--border) transparent;
}

/* ---------- Header ---------- */
.page-header {
  position: sticky;
  top: 0;
  z-index: 2;
  padding: 8px 12px 18px;
  background: var(--surface);
}

.page-header h1 {
  font-size: 28px;
  font-weight: 700;
  color: var(--on-surface);
  margin: 0 0 4px;
  line-height: 1.2;
}

.page-header p {
  margin: 0;
  color: var(--on-surface-variant);
  font-size: 14px;
}

/* ---------- Body ---------- */
.settings-body {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 320px;
  align-items: start;
  gap: 20px;
  max-width: 1120px;
  padding: 0 12px;
}

.settings-main {
  display: flex;
  flex-direction: column;
  gap: 20px;
  min-width: 0;
}

.settings-side {
  position: sticky;
  top: 12px;
  min-width: 0;
}

/* ---------- Carte Profil ---------- */
.profile-card {
  position: relative;
  border-radius: var(--shape-xl);
  border: none;
  background: var(--surface-container);
  box-shadow: var(--elev-1);
  overflow: hidden;
}

.profile-card::after {
  display: none;
}

.profile-head {
  padding: 28px 24px 22px;
  text-align: center;
}

.avatar-wrap {
  position: relative;
  width: 92px;
  height: 92px;
  margin: 0 auto 14px;
}

.avatar-img,
.avatar-fallback {
  width: 100%;
  height: 100%;
  border-radius: var(--shape-xl);
  display: grid;
  place-items: center;
  object-fit: cover;
}

.avatar-fallback {
  font-size: 32px;
  font-weight: 700;
  color: var(--on-primary-container);
  background: var(--primary-container);
}

.avatar-img {
  border: 2px solid var(--bg);
}

.avatar-edit {
  position: absolute;
  right: -6px;
  bottom: -6px;
  width: 32px;
  height: 32px;
  display: grid;
  place-items: center;
  border: 2px solid var(--bg);
  border-radius: 50%;
  background: var(--text-h);
  color: var(--bg);
  cursor: pointer;
  transition: transform 0.2s ease, filter 0.2s ease;
}

.avatar-edit:hover {
  transform: scale(1.1);
  filter: brightness(1.15);
}

.avatar-input {
  display: none;
}

.profile-name {
  font-size: 18px;
  font-weight: 700;
  color: var(--text-h);
  margin: 0;
}

.profile-sub {
  margin: 4px 0 0;
  font-size: 13px;
  color: var(--text);
  overflow-wrap: anywhere;
}

.profile-body {
  padding: 0 24px 24px;
}

.profile-actions {
  display: flex;
  justify-content: center;
  padding: 12px 0 0;
  border-bottom: 1px solid var(--border);
  margin-bottom: 8px;
}

.photo-link {
  background: none;
  border: none;
  padding: 4px 8px;
  color: var(--error);
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: opacity 0.2s;
}

.photo-link:hover {
  opacity: 0.75;
}

.photo-hint {
  font-size: 11px;
  color: var(--text);
  opacity: 0.8;
}

.profile-form {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 16px 2px;
  border-bottom: 1px solid var(--border);
  margin-bottom: 8px;
}

.field {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.field span {
  font-size: 12px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text);
}

.field input {
  padding: 10px 14px;
  border: 1px solid var(--border);
  border-radius: 10px;
  font-size: 14px;
  background: var(--bg);
  color: var(--text-h);
  transition: border-color 0.2s, box-shadow 0.2s;
}

.field input:focus {
  outline: none;
  border-color: var(--accent);
  box-shadow: 0 0 0 3px var(--accent-bg);
}

.field input:disabled {
  opacity: 0.6;
}

.save-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  height: 44px;
  padding: 0 24px;
  font-size: 14px;
  font-weight: 600;
  letter-spacing: 0.1px;
  border: none;
  border-radius: var(--shape-full);
  cursor: pointer;
  color: var(--on-primary);
  background: var(--primary);
  transition: box-shadow var(--motion);
}

.save-btn:hover:not(:disabled) {
  box-shadow: var(--elev-1);
}

.save-btn:disabled {
  opacity: 0.7;
  cursor: not-allowed;
}

.save-msg {
  margin: 0;
  font-size: 13px;
  font-weight: 600;
}

.save-msg.ok {
  color: var(--success, #1a9e4b);
}

.save-msg.err {
  color: var(--error);
}

.spinner {
  width: 14px;
  height: 14px;
  border: 2px solid transparent;
  border-top-color: currentColor;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.profile-rows {
  display: flex;
  flex-direction: column;
}

.profile-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 11px 2px;
}

.profile-row + .profile-row {
  border-top: 1px solid var(--border);
}

.row-icon {
  width: 30px;
  height: 30px;
  flex-shrink: 0;
  display: grid;
  place-items: center;
  border-radius: 9px;
  color: var(--accent);
  background: var(--accent-bg);
}

.row-label {
  font-size: 13px;
  color: var(--text);
}

.row-value {
  margin-left: auto;
  font-size: 13px;
  font-weight: 600;
  color: var(--text-h);
  text-align: right;
  overflow-wrap: anywhere;
}

.settings-section {
  padding: 22px 24px;
  background: var(--surface-container-low);
  border: none;
  border-radius: var(--shape-lg);
  box-shadow: var(--elev-1);
}

.settings-section h2 {
  font-size: 14px;
  font-weight: 700;
  letter-spacing: 0.5px;
  text-transform: uppercase;
  color: var(--text);
  margin: 0 0 16px;
}

/* ---------- Thèmes ---------- */
.theme-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(190px, 1fr));
  gap: 14px;
}

.theme-card {
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 12px;
  background: var(--code-bg);
  border: 2px solid var(--border);
  border-radius: 14px;
  cursor: pointer;
  text-align: left;
  transition: border-color 0.2s, box-shadow 0.2s, transform 0.2s;
}

.theme-card:hover {
  transform: translateY(-2px);
  box-shadow: var(--shadow);
}

.theme-card.active {
  border-color: var(--accent);
  background: var(--accent-bg);
}

.theme-swatch {
  position: relative;
  height: 80px;
  border-radius: 10px;
  border: 1px solid var(--border);
  display: flex;
  align-items: flex-start;
  padding: 12px;
  box-sizing: border-box;
  overflow: hidden;
}

.swatch-accent {
  position: absolute;
  left: 0;
  right: 0;
  bottom: 0;
  height: 24px;
}

.check {
  position: absolute;
  right: 8px;
  top: 8px;
  background: var(--accent);
  border-radius: 50%;
  padding: 3px;
  box-sizing: content-box;
  box-shadow: 0 2px 6px rgba(0, 0, 0, 0.25);
}

.theme-meta {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.theme-meta strong {
  font-size: 15px;
  color: var(--text-h);
}

.theme-meta small {
  font-size: 13px;
  color: var(--text);
}

/* ---------- Mises �� jour ---------- */
/* ---------- Mises à jour ---------- */
.update-card {
  margin-top: 14px;
}

.update-head {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 6px 4px 2px;
}

.update-head strong {
  font-size: 15px;
  color: var(--text-h);
}

.update-head small {
  font-size: 13px;
  color: var(--text);
}

.segment {
  display: inline-flex;
  gap: 4px;
  padding: 4px;
  border: 1px solid var(--border);
  border-radius: 999px;
  background: var(--code-bg);
  flex-shrink: 0;
}

.segment-btn {
  padding: 5px 14px;
  border: none;
  border-radius: 999px;
  font-size: 13px;
  font-weight: 600;
  color: var(--text);
  background: none;
  cursor: pointer;
  transition: background 0.2s, color 0.2s;
}

.segment-btn.active {
  background: var(--accent);
  color: var(--on-primary);
}

.channel-badge {
  flex-shrink: 0;
  padding: 5px 14px;
  border-radius: 999px;
  font-size: 13px;
  font-weight: 600;
  background: var(--accent);
  color: var(--on-primary);
}

.check-btn {
  flex-shrink: 0;
  padding: 8px 18px;
  border: none;
  border-radius: var(--shape-full);
  background: var(--secondary-container);
  color: var(--on-secondary-container);
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: box-shadow var(--motion);
}

.check-btn:hover:not(:disabled) {
  box-shadow: var(--elev-1);
}

.check-btn:disabled {
  opacity: 0.5;
  cursor: wait;
}

.ok-text {
  color: var(--success, #16a34a);
  font-weight: 600;
}

.version-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 14px 4px 10px;
  border-top: 1px solid var(--border);
}

.version-row span {
  font-size: 13px;
  color: var(--text);
}

.version-row code {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-h);
  background: var(--code-bg);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 3px 10px;
}

/* ---------- Lignes avec switch (curseur) ---------- */
.notif-card {
  display: flex;
  flex-direction: column;
}

.telemetry-note {
  margin: 0;
  padding: 4px 4px 6px;
  font-size: 12.5px;
  color: var(--text);
  line-height: 1.5;
}

.switch {
  position: relative;
  flex-shrink: 0;
  width: 48px;
  height: 28px;
  border: none;
  border-radius: 999px;
  background: var(--surface-container-highest);
  cursor: pointer;
  transition: background var(--motion);
  padding: 0;
}

.switch .switch-knob {
  position: absolute;
  top: 3px;
  left: 3px;
  width: 22px;
  height: 22px;
  border-radius: 50%;
  background: var(--on-surface-variant);
  transition: transform var(--motion), background var(--motion);
}

.switch.on {
  background: var(--primary);
}

.switch.on .switch-knob {
  transform: translateX(20px);
  background: var(--on-primary);
}

.switch:disabled {
  opacity: 0.6;
  cursor: wait;
}

.notif-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 14px 4px;
}

.notif-row + .notif-row {
  border-top: 1px solid var(--border);
}

.notif-meta {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.notif-meta strong {
  font-size: 15px;
  color: var(--text-h);
}

.notif-meta small {
  font-size: 13px;
  color: var(--text);
}

/* ---------- Responsive ---------- */
@media (max-width: 900px) {
  .settings-body {
    grid-template-columns: 1fr;
  }
  .settings-side {
    position: static;
  }
}

@media (max-width: 480px) {
  .page-header h1 {
    font-size: 24px;
  }
  .settings-section {
    padding: 18px 16px;
  }
}

/* ---------- Zone dangereuse ---------- */
.danger-section h2 {
  color: var(--error);
}

.danger-card {
  border-color: rgba(220, 38, 38, 0.35);
}

.danger-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 8px 14px;
  border-radius: 999px;
  border: 1px solid rgba(220, 38, 38, 0.5);
  background: rgba(220, 38, 38, 0.08);
  color: var(--error);
  font-size: 13px;
  font-weight: 700;
  cursor: pointer;
  white-space: nowrap;
}

.danger-btn:hover {
  background: rgba(220, 38, 38, 0.16);
}

.danger-overlay {
  position: fixed;
  inset: 0;
  z-index: 50;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 20px;
  background: rgba(0, 0, 0, 0.5);
}

.danger-modal {
  width: min(440px, 100%);
  background: var(--bg);
  border: 1px solid rgba(220, 38, 38, 0.4);
  border-radius: 16px;
  padding: 22px;
  box-shadow: 0 24px 60px -12px rgba(0, 0, 0, 0.4);
}

.danger-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 44px;
  height: 44px;
  border-radius: 12px;
  background: rgba(220, 38, 38, 0.1);
  color: var(--error);
  margin-bottom: 10px;
}

.danger-modal h3 {
  margin: 0 0 8px;
  font-size: 18px;
  color: var(--text-h);
}

.danger-text {
  margin: 0 0 12px;
  font-size: 13.5px;
  color: var(--text);
  line-height: 1.5;
}

.danger-count {
  margin: 0 0 12px;
  font-size: 14px;
  font-weight: 600;
  color: var(--error);
}

.danger-count.ready {
  color: var(--text);
  font-weight: 500;
}

.danger-actions {
  display: flex;
  gap: 10px;
  justify-content: flex-end;
  margin-top: 4px;
}

.ghost-btn {
  padding: 9px 16px;
  border-radius: 10px;
  border: 1px solid var(--border);
  background: transparent;
  color: var(--text-h);
  font-size: 13.5px;
  font-weight: 600;
  cursor: pointer;
}

.ghost-btn:hover:not(:disabled) {
  border-color: var(--accent);
  color: var(--accent);
}

.danger-confirm {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 9px 16px;
  border-radius: 10px;
  border: none;
  background: var(--error);
  color: #fff;
  font-size: 13.5px;
  font-weight: 700;
  cursor: pointer;
}

.danger-confirm:hover:not(:disabled) {
  filter: brightness(1.08);
}

.danger-confirm:disabled {
  opacity: 0.55;
  cursor: not-allowed;
}
</style>
