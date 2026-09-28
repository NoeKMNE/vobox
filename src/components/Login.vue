<script setup lang="ts">
import { computed, ref } from 'vue'
import { useRouter } from 'vue-router'
import { login, register, setToken } from '../composables/useAuth'
import CalligraphText from './CalligraphText.vue'

const router = useRouter()

const fullName = ref('')
const email = ref('')
const password = ref('')
const isLoading = ref(false)
const error = ref('')
const isSignUp = ref(false)

const title = computed(() => (isSignUp.value ? 'Créer un compte' : 'Connexion'))
const subtitle = computed(() =>
  isSignUp.value ? 'Rejoignez AgroBox dès maintenant' : 'Connectez-vous à votre compte'
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
      ? await register(fullName.value, email.value, password.value)
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
}
</script>

<template>
  <section id="center" class="login-page">
    <div class="login-card">
      <div class="login-header">
        <h1>
          <CalligraphText :text="title" :initial="true" />
        </h1>
        <p><CalligraphText :text="subtitle" :initial="true" /></p>
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

        <button type="submit" class="btn btn-primary" :disabled="isLoading">
          <span v-if="isLoading" class="spinner"></span>
          <CalligraphText :text="isSignUp ? 'S’inscrire' : 'Se connecter'" />
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
}

.login-card {
  width: 100%;
  max-width: 400px;
  background: var(--bg);
  border: 1px solid var(--border);
  border-radius: 16px;
  padding: 40px 32px;
  box-shadow: var(--shadow);
}

.login-header {
  text-align: center;
  margin-bottom: 32px;
}

.login-header h1 {
  font-size: 28px;
  font-weight: 600;
  color: var(--text-h);
  margin: 0 0 8px;
}

.login-header p {
  color: var(--text);
  margin: 0;
  font-size: 16px;
}

.login-form {
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.form-group {
  display: flex;
  flex-direction: column;
  gap: 6px;
  text-align: left;
}

.form-group label {
  font-size: 14px;
  font-weight: 500;
  color: var(--text-h);
}

.form-group input {
  padding: 12px 16px;
  border: 1px solid var(--border);
  border-radius: 8px;
  font-size: 16px;
  background: var(--bg);
  color: var(--text-h);
  transition: border-color 0.2s, box-shadow 0.2s;
}

.form-group input:focus {
  outline: none;
  border-color: var(--accent);
  box-shadow: 0 0 0 3px var(--accent-bg);
}

.form-group input:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.error-message {
  color: #dc2626;
  font-size: 14px;
  margin: 0;
  text-align: center;
}

.btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 14px 24px;
  font-size: 16px;
  font-weight: 600;
  border-radius: 8px;
  border: none;
  cursor: pointer;
  transition: all 0.2s;
}

.btn-primary {
  background: var(--accent);
  color: white;
}

.btn-primary:hover:not(:disabled) {
  filter: brightness(1.1);
}

.btn-primary:disabled {
  opacity: 0.7;
  cursor: not-allowed;
}

.btn-link {
  background: none;
  border: none;
  color: var(--accent);
  font-weight: 600;
  padding: 0;
  font-size: 16px;
  cursor: pointer;
}

.btn-link:hover {
  text-decoration: underline;
}

.login-toggle {
  text-align: center;
  margin-top: 24px;
  color: var(--text);
  font-size: 15px;
}

.spinner {
  width: 18px;
  height: 18px;
  border: 2px solid transparent;
  border-top-color: white;
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