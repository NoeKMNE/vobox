<script setup lang="ts">
import { computed, ref } from 'vue'
import { ChevronDown, Mic, Send, Square } from '@lucide/vue'
import CalligraphText from '../components/CalligraphText.vue'

const today = new Intl.DateTimeFormat('fr-FR', {
  day: 'numeric',
  month: 'long',
  year: 'numeric'
}).format(new Date())

const draft = ref('')
const isRecording = ref(false)
const model = ref('AgroBox Pro')
const models = ['AgroBox Pro', 'AgroBox Plus', 'AgroBox Éco']

const canSend = computed(() => draft.value.trim().length > 0)

const toggleRecording = () => {
  isRecording.value = !isRecording.value
}

const stopRecording = () => {
  isRecording.value = false
}

const sendMessage = () => {
  draft.value = ''
}

const autoResize = (e: Event) => {
  const el = e.target as HTMLTextAreaElement
  el.style.height = 'auto'
  el.style.height = el.scrollHeight + 'px'
}
</script>

<template>
  <div class="chat-input-page">
    <header class="page-header">
      <div class="page-header-text">
        <h1><CalligraphText text="Accueil" :initial="true" /></h1>
        <p>Votre assistant agricole et votre suivi en temps réel</p>
      </div>
      <span class="header-chip">{{ today }}</span>
    </header>

    <div class="composer-zone">
      <div class="chat-stack">
        <div class="model-select">
          <span class="model-dot" />
          <select v-model="model">
            <option v-for="m in models" :key="m" :value="m">{{ m }}</option>
          </select>
          <ChevronDown :size="14" />
        </div>

        <div class="composer" :class="{ recording: isRecording }">
        <button
          type="button"
          class="action-btn mic-btn"
          :class="{ active: isRecording }"
          :title="isRecording ? 'Arrêter la dictée' : 'Dicter'"
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
          <span
            v-for="n in 16"
            :key="n"
            class="wave-bar"
            :style="{ animationDelay: `${n * 0.05}s` }"
          />
        </div>

        <button
          v-if="isRecording"
          type="button"
          class="action-btn stop-btn"
          aria-label="Arrêter la dictée"
          title="Arrêter"
          @click="stopRecording"
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
    </div>
    </div>
  </div>
</template>

<style scoped>
.chat-input-page {
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

.header-chip {
  flex-shrink: 0;
  padding: 8px 14px;
  background: var(--bg);
  border: 1px solid var(--border);
  border-radius: 999px;
  color: var(--text-h);
  font-size: 13px;
  font-weight: 600;
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.04);
}

.composer-zone {
  flex: 1;
  min-height: 0;
  display: flex;
  align-items: center;
  justify-content: center;
}

.chat-stack {
  width: 100%;
  max-width: 680px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.model-select {
  align-self: flex-end;
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 6px 10px 6px 12px;
  background: var(--bg);
  border: 1px solid var(--border);
  border-radius: 12px;
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.04);
}

.model-dot {
  width: 8px;
  height: 8px;
  flex-shrink: 0;
  border-radius: 50%;
  background: linear-gradient(
    135deg,
    var(--accent) 0%,
    color-mix(in srgb, var(--accent) 60%, #ffffff 40%) 100%
  );
}

.model-select select {
  border: none;
  background: transparent;
  color: var(--text-h);
  font-size: 13px;
  font-weight: 600;
  outline: none;
  cursor: pointer;
  appearance: none;
  padding: 0;
}

.model-select svg {
  color: var(--text);
  flex-shrink: 0;
}

.composer {
  width: 100%;
  max-width: 680px;
  display: flex;
  align-items: flex-end;
  gap: 10px;
  padding: 10px 12px;
  background: linear-gradient(180deg, var(--bg) 0%, rgba(255, 255, 255, 0.5) 100%);
  border: 1px solid var(--border);
  border-radius: 18px;
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.3);
  animation: cardIn 0.55s ease both;
  transition: border-color 0.2s ease, box-shadow 0.2s ease;
}

.composer.recording {
  border-color: color-mix(in srgb, var(--accent) 45%, transparent);
  box-shadow:
    inset 0 1px 0 rgba(255, 255, 255, 0.3),
    0 0 0 4px color-mix(in srgb, var(--accent) 12%, transparent);
}

@keyframes cardIn {
  from {
    opacity: 0;
    transform: translateY(12px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
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
  color: var(--text);
}

.action-btn,
.send-button {
  flex-shrink: 0;
  display: grid;
  place-items: center;
  border: none;
  border-radius: 12px;
  cursor: pointer;
  transition: transform 0.2s ease, background 0.2s ease, color 0.2s ease;
}

.action-btn {
  width: 40px;
  height: 40px;
  background: var(--accent-bg);
  color: var(--text-h);
}

.mic-btn.active {
  background: linear-gradient(
    135deg,
    var(--accent) 0%,
    color-mix(in srgb, var(--accent) 60%, #ffffff 40%) 100%
  );
  color: white;
  animation: micPulse 1.2s ease-in-out infinite;
}

@keyframes micPulse {
  0%,
  100% {
    box-shadow: 0 0 0 0 color-mix(in srgb, var(--accent) 45%, transparent);
  }
  50% {
    box-shadow: 0 0 0 8px transparent;
  }
}

.stop-btn {
  background: rgba(220, 38, 38, 0.1);
  color: #dc2626;
}

.stop-btn:hover {
  background: rgba(220, 38, 38, 0.16);
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

.wave-bar {
  width: 4px;
  height: 100%;
  border-radius: 999px;
  background: linear-gradient(
    180deg,
    var(--accent) 0%,
    color-mix(in srgb, var(--accent) 80%, #ffffff 20%) 100%
  );
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
  width: 42px;
  height: 42px;
  background: linear-gradient(
    135deg,
    var(--accent) 0%,
    color-mix(in srgb, var(--accent) 70%, #ffffff 30%) 100%
  );
  color: white;
  box-shadow: 0 10px 18px -10px var(--accent);
}

.send-button:disabled {
  opacity: 0.45;
  cursor: not-allowed;
  transform: none;
  box-shadow: none;
}

.send-button:not(:disabled):active {
  transform: scale(0.9);
}

.send-button:hover:not(:disabled) {
  transform: translateY(-1px);
}

@media (max-width: 480px) {
  .page-header {
    flex-direction: column;
    align-items: flex-start;
  }
}
</style>