<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { Camera, Shield } from '@lucide/vue'
import CalligraphText from '../components/CalligraphText.vue'
import { useTheme, type Theme } from '../composables/useTheme'
import { customCursorEnabled, setCustomCursor } from '../composables/useCursor'
import { getSession, getToken, updateAvatar, updateProfile } from '../composables/useAuth'

const { currentTheme, setTheme } = useTheme()

const avatar = ref('')
const profile = ref<{ name: string; email: string } | null>(null)
const editName = ref('')
const editEmail = ref('')
const saving = ref(false)
const saveMessage = ref('')
const saveError = ref('')
const fileInput = ref<HTMLInputElement | null>(null)

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
  { value: 'tropical', label: 'Tropical', description: 'Vert végétal et ensoleillé', swatch: ['#f7fbf2', '#0ea86a'] },
  {
    value: 'emerald',
    label: 'Émeraude',
    description: 'Vert profond et ambre',
    swatch: ['#f8faf7', 'linear-gradient(90deg, #16a36a 0%, #62e6a5 55%, #f2a93b 100%)']
  }
]
</script>

<template>
  <div class="settings">
    <header class="page-header">
      <div class="page-header-text">
        <h1><CalligraphText text="Paramètres" :initial="true" /></h1>
        <p>Personnalisez votre expérience AgroBox</p>
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
          <div class="notif-card cursor-card">
            <div class="notif-row">
              <div class="notif-meta">
                <strong>Curseur personnalisé</strong>
                <small
                  >Flèche aux couleurs de votre accent — désactivé : curseur Windows par défaut</small
                >
              </div>
              <button
                class="switch"
                :class="{ on: customCursorEnabled }"
                :aria-checked="customCursorEnabled"
                role="switch"
                @click="setCustomCursor(!customCursorEnabled)"
              >
                <span class="knob"></span>
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
                <input v-model="editEmail" type="email" :disabled="saving" placeholder="jean@agrobox.fr" />
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
  background: linear-gradient(180deg, var(--code-bg) 68%, transparent);
}

.page-header h1 {
  font-size: 30px;
  font-weight: 700;
  color: var(--text-h);
  margin: 0 0 4px;
  line-height: 1.15;
}

.page-header p {
  margin: 0;
  color: var(--text);
  font-size: 15px;
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
  border-radius: 24px;
  border: 1px solid var(--border);
  background: linear-gradient(180deg, var(--bg) 0%, var(--social-bg) 100%);
  box-shadow:
    inset 0 1px 0 rgba(255, 255, 255, 0.08),
    0 1px 2px rgba(0, 0, 0, 0.06),
    0 12px 32px -8px rgba(0, 0, 0, 0.18),
    0 32px 64px -24px rgba(0, 0, 0, 0.22);
  overflow: hidden;
}

.profile-card::after {
  content: '';
  position: absolute;
  inset: 0;
  border-radius: inherit;
  pointer-events: none;
  box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.04);
}

.profile-head {
  padding: 28px 24px 22px;
  text-align: center;
  background: radial-gradient(260px 140px at 50% -20px, var(--accent-bg), transparent 75%);
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
  border-radius: 28px;
  display: grid;
  place-items: center;
  object-fit: cover;
}

.avatar-fallback {
  font-size: 30px;
  font-weight: 800;
  color: #fff;
  background: linear-gradient(135deg, var(--accent) 0%, color-mix(in srgb, var(--accent) 65%, #000 35%) 100%);
  box-shadow:
    inset 0 1px 0 rgba(255, 255, 255, 0.3),
    0 12px 28px -6px color-mix(in srgb, var(--accent) 65%, transparent);
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
  color: #dc2626;
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
  padding: 10px 16px;
  font-size: 14px;
  font-weight: 700;
  border: none;
  border-radius: 10px;
  cursor: pointer;
  color: #fff;
  background: linear-gradient(135deg, var(--accent) 0%, color-mix(in srgb, var(--accent) 70%, #000 30%) 100%);
  box-shadow: 0 8px 20px -8px color-mix(in srgb, var(--accent) 70%, transparent);
  transition: transform 0.2s ease, filter 0.2s ease;
}

.save-btn:hover:not(:disabled) {
  transform: translateY(-1px);
  filter: brightness(1.06);
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
  color: #1a9e4b;
}

.save-msg.err {
  color: #dc2626;
}

.spinner {
  width: 14px;
  height: 14px;
  border: 2px solid transparent;
  border-top-color: #fff;
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
  background: var(--bg);
  border: 1px solid var(--border);
  border-radius: 20px;
  box-shadow:
    inset 0 1px 0 rgba(255, 255, 255, 0.05),
    0 6px 20px -14px rgba(0, 0, 0, 0.2);
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

/* ---------- Curseur ---------- */
.cursor-card {
  margin-top: 14px;
}

/* ---------- Lignes avec switch (curseur) ---------- */
.notif-card {
  display: flex;
  flex-direction: column;
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

.switch {
  position: relative;
  width: 46px;
  height: 26px;
  flex-shrink: 0;
  border: 1px solid var(--border);
  border-radius: 999px;
  background: var(--code-bg);
  cursor: pointer;
  transition: background 0.25s ease, border-color 0.25s ease;
}

.switch .knob {
  position: absolute;
  top: 3px;
  left: 3px;
  width: 18px;
  height: 18px;
  border-radius: 50%;
  background: var(--text);
  transition: transform 0.25s ease, background 0.25s ease;
}

.switch.on {
  background: var(--accent);
  border-color: var(--accent);
}

.switch.on .knob {
  transform: translateX(20px);
  background: #fff;
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
</style>