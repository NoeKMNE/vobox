<script setup lang="ts">
import { computed, ref } from 'vue'
import { useRouter } from 'vue-router'
import { login, register, setToken } from '../composables/useAuth'
import { Leaf } from '@lucide/vue'

const router = useRouter()

const fullName = ref('')
const email = ref('')
const password = ref('')
const telemetryConsent = ref(false)
const isLoading = ref(false)
const error = ref('')
const isSignUp = ref(false)

const title = computed(() => (isSignUp.value ? 'Créer un compte' : 'Connexion'))
const subtitle = computed(() =>
  isSignUp.value ? 'Rejoignez Vobox dès maintenant' : 'Connectez-vous à votre compte'
)

const handleSubmit = async () => {
  error.value = ''
  if (isSignUp.value && !fullName.value.trim()) {
    error.value = 'Veuillez entrer votre nom complet'
    return
  }
  if (!email.value || !password.value) {
    error.value = 'Veuillez remplir tous les champs'
    return
  }
  isLoading.value = true
  try {
    const session = isSignUp.value
      ? await register(fullName.value, email.value, password.value, telemetryConsent.value)
      : await login(email.value, password.value)
    setToken(session.token)
    if (session.user.onboarding_done) {
      router.push({ name: 'Main' })
    } else {
      router.push({ name: 'Onboarding', params: { step: 1 } })
    }
  } catch (e) {
    error.value = typeof e === 'string' ? e : e instanceof Error ? e.message : 'Une erreur est survenue'
  } finally {
    isLoading.value = false
  }
}

const toggleMode = () => {
  isSignUp.value = !isSignUp.value
  error.value = ''
  fullName.value = ''
  email.value = ''
  password.value = ''
  telemetryConsent.value = false
}
</script>

<template>
  <section id="center" class="login-page">
    <div class="login-card">
      <div class="login-header">
        <div class="login-mark" aria-hidden="true">
          <Leaf :size="32" />
        </div>
        <h1>{{ title }}</h1>
        <p>{{ subtitle }}</p>
      </div>

      <form @submit.prevent="handleSubmit" class="login-form">
        <div v-if="isSignUp" class="form-group">
          <label for="fullName">Nom complet</label>
          <input
            id="fullName"
            v-model="fullName"
            type="text"
            autocomplete="name"
            required
            :disabled="isLoading"
            placeholder="Jean Dupont"
          />
        </div>

        <div class="form-group">
          <label for="email">Email</label>
          <input
            id="email"
            v-model="email"
            type="email"
            autocomplete="email"
            required
            :disabled="isLoading"
            placeholder="votre@email.com"
          />
        </div>

        <div class="form-group">
          <label for="password">Mot de passe</label>
          <input
            id="password"
            v-model="password"
            type="password"
            autocomplete="current-password"
            required
            :disabled="isLoading"
            placeholder="••••••••"
          />
        </div>

        <p v-if="error" class="error-message">{{ error }}</p>

        <label v-if="isSignUp" class="consent-row">
          <input v-model="telemetryConsent" type="checkbox" :disabled="isLoading" />
          <span>
            J’accepte la collecte de données d’usage (télémétrie) pour aider à
            améliorer Vobox. <b>Optionnel</b>, modifiable à tout moment dans Paramètres.
          </span>
        </label>

        <details v-if="isSignUp" class="consent-details">
          <summary>Quelles données sont collectées ?</summary>
          <ul>
            <li>Événements d’usage uniquement : ouverture de l’app, connexion, imports (nombres), fiches générées (statut).</li>
            <li><b>Jamais collectés :</b> mot de passe, email, contenus de fichiers, clés API, noms de producteurs.</li>
            <li>Conservées <b>localement</b> sur votre poste ; aucune transmission réseau par défaut (un endpoint de collecte peut être configuré plus tard).</li>
          </ul>
        </details>

        <button type="submit" class="btn btn-primary" :disabled="isLoading">
          <span v-if="isLoading" class="spinner"></span>
          {{ isSignUp ? 'S’inscrire' : 'Se connecter' }}
        </button>
      </form>

      <p class="login-toggle">
        {{ isSignUp ? 'Déjà un compte ?' : "Pas encore de compte ?" }}
        <button type="button" @click="toggleMode" class="btn-link">
          {{ isSignUp ? 'Se connecter' : "S'inscrire" }}
        </button>
      </p>
    </div>
  </section>
</template>

<style scoped>
.login-page {
  min-height: 100svh;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 24px;
  box-sizing: border-box;
  background: var(--surface);
}

.login-card {
  width: 100%;
  max-width: 420px;
  background: var(--surface-container-low);
  border: none;
  border-radius: var(--shape-xl);
  padding: 40px 32px;
  box-shadow: var(--elev-1);
}

.login-header {
  text-align: center;
  margin-bottom: 32px;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
}

.login-mark {
  width: 64px;
  height: 64px;
  display: grid;
  place-items: center;
  border-radius: var(--shape-lg);
  background: var(--primary-container);
  color: var(--on-primary-container);
}

.login-header h1 {
  font-size: 28px;
  font-weight: 700;
  color: var(--on-surface);
  margin: 0;
}

.login-header p {
  color: var(--on-surface-variant);
  margin: 0;
  font-size: 15px;
}

.login-form {
  display: flex;
  flex-direction: column;
  gap: 18px;
}

.form-group {
  display: flex;
  flex-direction: column;
  gap: 6px;
  text-align: left;
}

.form-group label {
  font-size: 12px;
  font-weight: 500;
  letter-spacing: 0.4px;
  color: var(--on-surface-variant);
}

.form-group input {
  padding: 14px 16px;
  border: 1px solid transparent;
  border-bottom-color: var(--outline);
  border-radius: var(--shape-xs) var(--shape-xs) 0 0;
  font-size: 16px;
  background: var(--surface-container-high);
  color: var(--on-surface);
  transition: background var(--motion), border-color var(--motion), box-shadow var(--motion);
}

.form-group input:hover {
  background: color-mix(in srgb, var(--on-surface) 6%, var(--surface-container-high));
}

.form-group input:focus {
  outline: none;
  background: var(--surface-container-highest);
  border-bottom: 2px solid var(--primary);
  box-shadow: none;
}

.form-group input:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.error-message {
  color: var(--error);
  background: var(--error-container);
  padding: 10px 14px;
  border-radius: var(--shape-sm);
  font-size: 14px;
  margin: 0;
  text-align: center;
}

.btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 10px;
  height: 52px;
  padding: 0 24px;
  font-size: 15px;
  font-weight: 600;
  letter-spacing: 0.1px;
  border-radius: var(--shape-full);
  border: none;
  cursor: pointer;
  transition: box-shadow var(--motion), filter var(--motion);
}

.btn-primary {
  background: var(--primary);
  color: var(--on-primary);
}

.btn-primary:hover:not(:disabled) {
  box-shadow: var(--elev-1);
  filter: brightness(1.06);
}

.btn-primary:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.btn-link {
  background: none;
  border: none;
  color: var(--primary);
  font-weight: 600;
  padding: 0;
  font-size: 15px;
  cursor: pointer;
}

.btn-link:hover {
  text-decoration: underline;
}

.login-toggle {
  text-align: center;
  margin-top: 24px;
  color: var(--on-surface-variant);
  font-size: 15px;
}

.consent-row {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  font-size: 13px;
  line-height: 1.5;
  color: var(--on-surface-variant);
  cursor: pointer;
}

.consent-row input {
  margin-top: 2px;
  width: 17px;
  height: 17px;
  flex-shrink: 0;
  accent-color: var(--primary);
  cursor: pointer;
}

.consent-details {
  font-size: 12.5px;
  color: var(--on-surface-variant);
  background: var(--surface-container-high);
  border-radius: var(--shape-sm);
  padding: 10px 12px;
}

.consent-details summary {
  cursor: pointer;
  font-weight: 600;
  color: var(--on-surface);
}

.consent-details ul {
  margin: 8px 0 0;
  padding-left: 18px;
  display: flex;
  flex-direction: column;
  gap: 4px;
  line-height: 1.5;
}

.spinner {
  width: 18px;
  height: 18px;
  border: 2px solid transparent;
  border-top-color: currentColor;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

@media (max-width: 480px) {
  .login-card {
    padding: 24px 20px;
  }
}
</style>