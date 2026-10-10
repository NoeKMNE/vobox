<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import {
  BookOpenCheck,
  ChevronDown,
  Loader,
  Mic,
  Send,
  Sparkles,
  Square,
  Volume2
} from '@lucide/vue'
import {
  assistantChat,
  getAiSettings,
  KNOWN_MODELS,
  saveAiSettings,
  type AgentResponse,
  type AiMessage,
  type AiSettings,
  type ProposedAction
} from '../composables/useAssistant'
import {
  genererFiche,
  genererFicheManuelle,
  getToken,
  type Fiche
} from '../composables/useAuth'
import {
  isSttSupported,
  isTtsSupported,
  speak,
  startDictation,
  stopDictation,
  stopSpeaking
} from '../composables/useVoice'

const today = new Intl.DateTimeFormat('fr-FR', {
  day: 'numeric',
  month: 'long',
  year: 'numeric'
}).format(new Date())

interface DisplayMessage {
  role: 'user' | 'assistant'
  content: string
  thinking?: boolean
  error?: boolean
  actions?: ProposedAction[]
  actionState?: 'idle' | 'working'
}

const messages = ref<DisplayMessage[]>([])
const draft = ref('')
const interimText = ref('')
const sending = ref(false)
const error = ref('')
const isRecording = ref(false)
const sttOk = isSttSupported()
const ttsOk = isTtsSupported()
const speakOnReply = ref(localStorage.getItem('Vobox_tts') !== '0')

const onTtsToggle = () => {
  try {
    localStorage.setItem('Vobox_tts', speakOnReply.value ? '1' : '0')
  } catch {
    // stockage indisponible : on garde le choix en mémoire
  }
  stopSpeaking()
}
const playingId = ref<number | null>(null)
const router = useRouter()

const goGuide = () => router.push({ name: 'Guide' })

const settings = ref<AiSettings | null>(null)
const selectedModel = ref('deepseek-chat')

const suggestions = [
  'Quel est l\'état global de mes exploitations ?',
  'Liste mes fiches de traçabilité',
  'Quels produits sont disponibles aujourd\'hui ?',
  'Crée une fiche de 200 kg de cacao'
]

const canSend = computed(() => draft.value.trim().length > 0 && !sending.value)
const needsSetup = computed(() => !settings.value?.ready)

const loadSettings = async () => {
  const token = getToken()
  if (!token) return
  try {
    settings.value = await getAiSettings(token)
    selectedModel.value = KNOWN_MODELS.includes(settings.value.model)
      ? settings.value.model
      : 'deepseek-chat'
  } catch (e) {
    error.value = typeof e === 'string' ? e : 'Réglages IA indisponibles'
  }
}

const changeModel = async () => {
  const token = getToken()
  if (!token || !settings.value) return
  settings.value.model = selectedModel.value
  try {
    await saveAiSettings(token, { model: selectedModel.value })
  } catch (e) {
    error.value = typeof e === 'string' ? e : 'Enregistrement du modèle impossible'
  }
}

const fillSuggestion = (s: string) => {
  draft.value = s
}

const apiHistory = (): AiMessage[] =>
  messages.value
    .filter((m) => !m.thinking && !m.error)
    .map((m) => ({ role: m.role, content: m.content }))

const sendMessage = async () => {
  const token = getToken()
  const content = draft.value.trim()
  if (!token || !content || sending.value) return
  if (!settings.value?.ready) {
    error.value =
      "Connexion au service IA indisponible. Vérifiez votre connexion puis déconnectez-vous et reconnectez-vous."
    return
  }
  draft.value = ''
  messages.value.push({ role: 'user', content })
  const placeholderIndex = messages.value.push({
    role: 'assistant',
    content: '',
    thinking: true
  }) - 1
  sending.value = true
  error.value = ''
  try {
    const history = apiHistory()
    const res: AgentResponse = await assistantChat(token, history)
    const msg = messages.value[placeholderIndex]
    if (msg) {
      msg.thinking = false
      msg.content = res.reply
      msg.actions = res.actions
    }
    if (speakOnReply.value && res.reply) speakReply(placeholderIndex, res.reply)
  } catch (e) {
    const msg = messages.value[placeholderIndex]
    if (msg) {
      msg.thinking = false
      msg.error = true
      msg.content = typeof e === 'string' ? e : 'Impossible de joindre l\'assistant'
    }
  } finally {
    sending.value = false
  }
}

const speakReply = (index: number, text: string) => {
  if (!ttsOk) return
  if (playingId.value === index) {
    stopSpeaking()
    playingId.value = null
    return
  }
  stopSpeaking()
  playingId.value = index
  speak(text, () => {
    if (playingId.value === index) playingId.value = null
  })
}

const confirmAction = async (msg: DisplayMessage, action: ProposedAction) => {
  const token = getToken()
  if (!token || msg.actionState === 'working') return
  msg.actionState = 'working'
  try {
    let fiche: Fiche
    if (action.tool === 'generer_fiche_manuelle') {
      const produit = String(action.args.produit ?? '')
      const allocations = Array.isArray(action.args.allocations)
        ? (action.args.allocations as [number, number][])
        : []
      fiche = await genererFicheManuelle(
        token,
        produit,
        allocations,
        action.args.titre ? String(action.args.titre) : undefined
      )
    } else {
      const produit = String(action.args.produit ?? '')
      const quantiteKg = Number(action.args.quantite_kg ?? 0)
      fiche = await genererFiche(
        token,
        produit,
        quantiteKg,
        action.args.titre ? String(action.args.titre) : undefined
      )
    }
    msg.actions = []
    msg.actionState = 'idle'
    const qte = new Intl.NumberFormat('fr-FR', { maximumFractionDigits: 1 }).format(
      fiche.quantite_attribuee_kg
    )
    messages.value.push({
      role: 'assistant',
      content: `Fiche ${fiche.reference} créée : ${fiche.produit}, ${qte} kg attribués. Retrouvez-la dans Fiche et Archives.`
    })
  } catch (e) {
    const reason = typeof e === 'string' ? e : 'Action impossible'
    messages.value.push({ role: 'assistant', content: `⚠️ ${reason}`, error: true })
  } finally {
    msg.actionState = 'idle'
  }
}

const toggleRecording = () => {
  if (!sttOk) {
    error.value = 'Reconnaissance vocale indisponible dans cette fenêtre'
    return
  }
  error.value = ''
  if (isRecording.value) {
    stopDictation()
    return
  }
  interimText.value = ''
  const ok = startDictation({
    onInterim: (text) => (interimText.value = text),
    onFinal: (text) => {
      draft.value = text.trim()
      interimText.value = ''
    },
    onEnd: () => {
      isRecording.value = false
      interimText.value = ''
    },
    onError: () => {
      isRecording.value = false
      interimText.value = ''
    }
  })
  if (ok) isRecording.value = true
  else error.value = 'Impossible de démarrer la dictée'
}

const autoResize = (e: Event) => {
  const el = e.target as HTMLTextAreaElement
  el.style.height = 'auto'
  el.style.height = el.scrollHeight + 'px'
}

onMounted(() => {
  void loadSettings()
})
</script>

<template>
  <div class="assistant-page">
    <header class="page-header">
      <div class="page-header-text">
        <h1>Assistant IA</h1>
        <p>Votre assistant agricole et votre suivi en temps réel</p>
      </div>
      <div class="header-actions">
        <span class="header-chip">{{ today }}</span>
      </div>
    </header>

    <main class="chat-scroll">
      <div v-if="messages.length === 0" class="empty-state">
        <div v-if="needsSetup" class="empty-hero">
          <div class="empty-icon"><Sparkles :size="22" /></div>
          <h2>Assistant indisponible</h2>
          <p>
            L'accès au service IA n'a pas pu être initialisé. Vérifiez votre connexion
            Internet puis déconnectez-vous et reconnectez-vous.
          </p>
          <button type="button" class="guide-link" @click="goGuide">
            <BookOpenCheck :size="15" /> Voir le tutoriel de prise en main
          </button>
        </div>
        <div v-else class="empty-hero">
          <div class="empty-icon"><Sparkles :size="22" /></div>
          <h2>Bonjour 👋</h2>
          <p>Posez vos questions ou demandez des actions précises sur vos données.</p>
          <button type="button" class="guide-link" @click="goGuide">
            <BookOpenCheck :size="15" /> Revoir le tutoriel
          </button>
          <div class="chips">
            <button
              v-for="(s, i) in suggestions"
              :key="i"
              type="button"
              class="chip"
              @click="fillSuggestion(s)"
            >
              {{ s }}
            </button>
          </div>
        </div>
      </div>

      <div v-else class="messages">
        <div
          v-for="(m, i) in messages"
          :key="i"
          class="bubble-row"
          :class="m.role"
        >
          <div v-if="m.thinking" class="bubble thinking">
            <span class="typing-dot" />
            <span class="typing-dot" />
            <span class="typing-dot" />
          </div>
          <div v-else class="bubble" :class="{ error: m.error }">
            <span class="bubble-text">{{ m.content }}</span>
            <div v-if="m.role === 'assistant' && m.actions && m.actions.length" class="action-card">
              <p v-for="(a, j) in m.actions" :key="j" class="action-item">
                <span class="action-label">{{ a.label }}</span>
                <button
                  type="button"
                  class="action-confirm"
                  :disabled="m.actionState === 'working'"
                  @click="confirmAction(m, a)"
                >
                  <Loader v-if="m.actionState === 'working'" :size="13" class="spin" />
                  {{ m.actionState === 'working' ? '…' : 'Confirmer' }}
                </button>
              </p>
            </div>
          </div>
          <div v-if="m.role === 'assistant' && !m.error && m.content && ttsOk" class="bubble-foot">
            <button
              type="button"
              class="speak-btn"
              :class="{ active: playingId === i }"
              :title="playingId === i ? 'Arrêter' : 'Écouter'"
              aria-label="Écouter la réponse"
              @click="speakReply(i, m.content)"
            >
              <Volume2 :size="14" />
            </button>
          </div>
        </div>
        <p v-if="error" class="error-line">{{ error }}</p>
      </div>
    </main>

    <footer class="composer-zone">
      <div class="chat-stack">
        <div v-if="settings?.ready" class="model-select">
          <span class="model-dot" />
          <select v-model="selectedModel" @change="changeModel">
            <option v-for="m in KNOWN_MODELS" :key="m" :value="m">{{ m }}</option>
          </select>
          <ChevronDown :size="14" />
        </div>

        <div class="composer" :class="{ recording: isRecording }">
          <button
            type="button"
            class="action-btn mic-btn"
            :class="{ active: isRecording }"
            :title="!sttOk ? 'Dictée indisponible' : isRecording ? 'Arrêter la dictée' : 'Dicter'"
            :aria-label="isRecording ? 'Arrêter la dictée' : 'Démarrer la dictée'"
            @click="toggleRecording"
          >
            <Mic :size="18" />
          </button>

          <div v-if="!isRecording" class="input-box">
            <textarea
              v-model="draft"
              aria-label="Message"
              placeholder="Écrire au spécialiste agricole..."
              rows="1"
              @input="autoResize"
              @keydown.enter.exact.prevent="sendMessage"
              @keydown.enter.shift.prevent="() => {}"
            />
          </div>

          <div v-else class="wave-box" aria-label="Onde vocale">
            <span class="interim" v-if="interimText">{{ interimText }}</span>
            <template v-else>
              <span
                v-for="n in 16"
                :key="n"
                class="wave-bar"
                :style="{ animationDelay: `${n * 0.05}s` }"
              />
            </template>
          </div>

          <button
            v-if="isRecording"
            type="button"
            class="action-btn stop-btn"
            aria-label="Arrêter la dictée"
            title="Arrêter"
            @click="stopDictation"
          >
            <Square :size="16" />
          </button>

          <button
            class="send-button"
            type="button"
            aria-label="Envoyer"
            :disabled="!canSend"
            @click="sendMessage"
          >
            <Send :size="18" />
          </button>
        </div>

        <div class="composer-options">
          <label v-if="ttsOk" class="tts-toggle" title="Lire les réponses à voix haute">
            <input v-model="speakOnReply" type="checkbox" @change="onTtsToggle" />
            <span>Réponses vocales</span>
          </label>
        </div>
      </div>
    </footer>
  </div>
</template>

<style scoped>
.assistant-page {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  padding: 8px 8px 16px;
}

.page-header {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 8px 12px 18px;
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

.header-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.header-chip {
  flex-shrink: 0;
  padding: 8px 16px;
  background: var(--surface-container-high);
  border: none;
  border-radius: var(--shape-full);
  color: var(--on-surface-variant);
  font-size: 13px;
  font-weight: 600;
  letter-spacing: 0.1px;
}

.gear-btn {
  width: 40px;
  height: 40px;
  display: grid;
  place-items: center;
  border: none;
  border-radius: var(--shape-full);
  background: transparent;
  color: var(--on-surface-variant);
  cursor: pointer;
  transition: background var(--motion), color var(--motion);
}

.gear-btn:hover,
.gear-btn.open {
  color: var(--on-primary-container);
  background: var(--primary-container);
}

.chat-scroll {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 4px 12px 16px;
}

.empty-state {
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
}

.empty-hero {
  max-width: 460px;
  text-align: center;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 10px;
}

.empty-icon {
  width: 64px;
  height: 64px;
  display: grid;
  place-items: center;
  border-radius: var(--shape-lg);
  color: var(--on-primary-container);
  background: var(--primary-container);
}

.empty-hero h2 {
  margin: 0;
  font-size: 22px;
  font-weight: 700;
  color: var(--on-surface);
}

.empty-hero p {
  margin: 0;
  color: var(--on-surface-variant);
  font-size: 14px;
  line-height: 1.55;
}

.setup-open-btn {
  margin-top: 6px;
  height: 44px;
  padding: 0 24px;
  border: none;
  border-radius: var(--shape-full);
  background: var(--primary);
  color: var(--on-primary);
  font-weight: 600;
  letter-spacing: 0.1px;
  cursor: pointer;
  font-size: 14px;
  transition: box-shadow var(--motion);
}

.setup-open-btn:hover {
  box-shadow: var(--elev-1);
}

.guide-link {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  border: none;
  background: transparent;
  color: var(--primary);
  font-size: 13.5px;
  font-weight: 700;
  cursor: pointer;
  padding: 8px 12px;
  border-radius: var(--shape-full);
}
.guide-link:hover {
  background: var(--primary-container);
}

.chips {
  margin-top: 8px;
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  justify-content: center;
}

.chip {
  padding: 9px 16px;
  border: 1px solid var(--outline);
  border-radius: var(--shape-full);
  background: var(--surface);
  color: var(--on-surface);
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: border-color var(--motion), color var(--motion), background var(--motion);
}

.chip:hover {
  border-color: var(--primary);
  color: var(--primary);
  background: var(--surface-container-low);
}

.setup-card {
  margin: 0 auto 18px;
  max-width: 480px;
  padding: 20px;
  border: none;
  border-radius: var(--shape-lg);
  background: var(--surface-container-low);
  box-shadow: var(--elev-1);
}

.setup-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
  font-weight: 600;
  letter-spacing: 0.1px;
  color: var(--on-surface);
  margin-bottom: 16px;
}

.setup-title svg {
  color: var(--primary);
}

.setup-ok {
  margin-left: auto;
  display: inline-flex;
  align-items: center;
  gap: 4px;
  color: #0ca678;
  font-size: 12px;
  font-weight: 600;
}

.setup-row {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-bottom: 14px;
}

.setup-row label {
  font-size: 12px;
  font-weight: 500;
  letter-spacing: 0.4px;
  color: var(--on-surface-variant);
}

.setup-row input,
.setup-row select {
  padding: 12px 14px;
  border: 1px solid transparent;
  border-bottom-color: var(--outline);
  border-radius: var(--shape-xs) var(--shape-xs) 0 0;
  background: var(--surface-container-high);
  color: var(--on-surface);
  font-size: 13px;
  outline: none;
  transition: background var(--motion), border-color var(--motion);
}

.setup-row input:focus,
.setup-row select:focus {
  border-bottom: 2px solid var(--primary);
  background: var(--surface-container-highest);
}

.setup-actions {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 10px;
}

.setup-save {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  height: 40px;
  padding: 0 20px;
  border: none;
  border-radius: var(--shape-full);
  background: var(--primary);
  color: var(--on-primary);
  font-size: 13px;
  font-weight: 600;
  letter-spacing: 0.1px;
  cursor: pointer;
  transition: box-shadow var(--motion);
}

.setup-save:hover:not(:disabled) {
  box-shadow: var(--elev-1);
}

.setup-save:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.setup-error {
  color: var(--error);
  font-size: 12px;
}

.setup-hint {
  margin: 14px 0 0;
  font-size: 12px;
  color: var(--on-surface-variant);
  line-height: 1.5;
}

.messages {
  max-width: 720px;
  margin: 0 auto;
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.bubble-row {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 4px;
}

.bubble-row.user {
  align-items: flex-end;
}

.bubble {
  max-width: 78%;
  padding: 12px 16px;
  border-radius: 20px;
  font-size: 14px;
  line-height: 1.55;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
}

.bubble-text {
  font-weight: 400;
}

.bubble-row.user .bubble {
  background: var(--primary-container);
  color: var(--on-primary-container);
  border-bottom-right-radius: 6px;
}

.bubble-row.assistant .bubble {
  background: var(--surface-container-high);
  color: var(--on-surface);
  border-bottom-left-radius: 6px;
}

.bubble.error {
  background: var(--error-container);
  color: var(--on-error-container);
}

.bubble-foot {
  display: flex;
  gap: 8px;
  padding-left: 8px;
}

.speak-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  width: 32px;
  height: 32px;
  border: none;
  border-radius: var(--shape-full);
  background: transparent;
  color: var(--on-surface-variant);
  cursor: pointer;
  transition: background var(--motion), color var(--motion);
}

.speak-btn:hover,
.speak-btn.active {
  color: var(--on-secondary-container);
  background: var(--secondary-container);
}

.thinking {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 14px 16px;
}

.typing-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--accent);
  animation: typingBlink 1.2s ease-in-out infinite;
}

.typing-dot:nth-child(2) {
  animation-delay: 0.15s;
}

.typing-dot:nth-child(3) {
  animation-delay: 0.3s;
}

@keyframes typingBlink {
  0%,
  100% {
    opacity: 0.25;
    transform: translateY(0);
  }
  50% {
    opacity: 1;
    transform: translateY(-3px);
  }
}

.action-card {
  margin-top: 12px;
  padding-top: 12px;
  border-top: 1px solid var(--outline-variant);
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.action-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin: 0;
}

.action-label {
  font-size: 13px;
  font-weight: 600;
}

.action-confirm {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  height: 34px;
  padding: 0 18px;
  border: none;
  border-radius: var(--shape-full);
  background: var(--primary);
  color: var(--on-primary);
  font-size: 12px;
  font-weight: 600;
  letter-spacing: 0.1px;
  cursor: pointer;
  transition: box-shadow var(--motion);
}

.action-confirm:hover:not(:disabled) {
  box-shadow: var(--elev-1);
}

.action-confirm:disabled {
  opacity: 0.5;
  cursor: wait;
}

.error-line {
  margin: 6px 0 0;
  text-align: center;
  font-size: 13px;
  color: var(--error);
}

.composer-zone {
  flex-shrink: 0;
  display: flex;
  justify-content: center;
  padding: 10px 12px 0;
}

.chat-stack {
  width: 100%;
  max-width: 680px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.model-select {
  align-self: flex-end;
  display: inline-flex;
  align-items: center;
  gap: 8px;
  height: 36px;
  padding: 0 12px;
  background: var(--surface-container-high);
  border: 1px solid var(--outline-variant);
  border-radius: var(--shape-full);
}

.model-dot {
  width: 8px;
  height: 8px;
  flex-shrink: 0;
  border-radius: 50%;
  background: var(--primary);
}

.model-select select {
  border: none;
  background: transparent;
  color: var(--on-surface);
  font-size: 13px;
  font-weight: 600;
  outline: none;
  cursor: pointer;
  appearance: none;
  padding: 0;
}

.model-select svg {
  color: var(--on-surface-variant);
  flex-shrink: 0;
}

.composer {
  width: 100%;
  display: flex;
  align-items: flex-end;
  gap: 8px;
  padding: 8px 10px;
  background: var(--surface-container-high);
  border: 1px solid transparent;
  border-radius: var(--shape-xl);
  transition: border-color var(--motion), box-shadow var(--motion);
}

.composer:focus-within {
  border-color: var(--primary);
}

.composer.recording {
  border-color: var(--primary);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--primary) 16%, transparent);
}

.input-box {
  flex: 1;
  min-width: 0;
}

.input-box textarea {
  width: 100%;
  border: none;
  background: transparent;
  color: var(--text-h);
  font: inherit;
  outline: none;
  resize: none;
  min-height: 24px;
  max-height: 160px;
  line-height: 1.5;
  overflow-y: auto;
  padding: 8px 0;
}

.input-box textarea::placeholder {
  color: var(--on-surface-variant);
}

.action-btn,
.send-button {
  flex-shrink: 0;
  display: grid;
  place-items: center;
  border: none;
  cursor: pointer;
  transition: background var(--motion), color var(--motion), opacity var(--motion);
}

.action-btn {
  width: 40px;
  height: 40px;
  border-radius: var(--shape-full);
  background: transparent;
  color: var(--on-surface-variant);
}

.action-btn:hover {
  background: color-mix(in srgb, var(--on-surface) 8%, transparent);
  color: var(--on-surface);
}

.mic-btn.active {
  background: var(--error-container);
  color: var(--on-error-container);
  animation: micPulse 1.2s ease-in-out infinite;
}

@keyframes micPulse {
  0%,
  100% {
    box-shadow: 0 0 0 0 color-mix(in srgb, var(--error) 45%, transparent);
  }
  50% {
    box-shadow: 0 0 0 8px transparent;
  }
}

.stop-btn {
  background: var(--error-container);
  color: var(--on-error-container);
}

.stop-btn:hover {
  background: color-mix(in srgb, var(--error) 24%, var(--error-container));
}

.action-btn:active {
  transform: scale(0.9);
}

.wave-box {
  flex: 1;
  min-width: 0;
  height: 40px;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 4px;
  padding: 0 4px;
  overflow: hidden;
}

.interim {
  font-size: 13px;
  color: var(--text-h);
  text-align: center;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.wave-bar {
  width: 4px;
  height: 100%;
  border-radius: 999px;
  background: var(--primary);
  transform-origin: center;
  animation: waveBounce 1s ease-in-out infinite;
}

@keyframes waveBounce {
  0%,
  100% {
    transform: scaleY(0.25);
  }
  50% {
    transform: scaleY(1);
  }
}

.send-button {
  width: 44px;
  height: 44px;
  border-radius: var(--shape-md);
  background: var(--primary);
  color: var(--on-primary);
}

.send-button:hover:not(:disabled) {
  box-shadow: var(--elev-1);
}

.send-button:active:not(:disabled) {
  transform: scale(0.94);
}

.send-button:disabled {
  background: color-mix(in srgb, var(--on-surface) 12%, transparent);
  color: var(--on-surface-variant);
  opacity: 0.6;
  cursor: not-allowed;
  box-shadow: none;
}

.composer-options {
  align-self: flex-end;
  display: flex;
  align-items: center;
  min-height: 18px;
}

.tts-toggle {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
  color: var(--on-surface-variant);
  cursor: pointer;
  user-select: none;
}

.tts-toggle input {
  accent-color: var(--primary);
  cursor: pointer;
}

.spin {
  animation: spin 0.9s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

@media (max-width: 480px) {
  .page-header {
    flex-direction: column;
    align-items: flex-start;
  }
}
</style>