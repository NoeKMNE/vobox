<script setup lang="ts">
import { computed, onMounted, onUnmounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { completeOnboarding, getToken } from '../composables/useAuth'
import OnboardingStep1 from '../components/onboarding/Step1.vue'
import OnboardingStep2 from '../components/onboarding/Step2.vue'
import OnboardingStep3 from '../components/onboarding/Step3.vue'

const route = useRoute()
const router = useRouter()

const step = computed(() => Number(route.params.step))
const totalSteps = 3

const steps = [
  { id: 1, component: OnboardingStep1 },
  { id: 2, component: OnboardingStep2 },
  { id: 3, component: OnboardingStep3 }
]

const currentStepData = computed(() => steps.find(s => s.id === step.value) || steps[0])

const finish = async () => {
  const token = getToken()
  if (token) {
    try {
      await completeOnboarding(token)
    } catch {
      // on laisse passer : la navigation reste possible
    }
  }
  router.push({ name: 'Main' })
}

const next = async () => {
  if (step.value < totalSteps) {
    router.push({ name: 'Onboarding', params: { step: step.value + 1 } })
  } else {
    await finish()
  }
}

const previous = () => {
  if (step.value > 1) {
    router.push({ name: 'Onboarding', params: { step: step.value - 1 } })
  }
}

const skip = async () => {
  await finish()
}

const handleKeyDown = (e: KeyboardEvent) => {
  if (e.key === 'ArrowRight' || e.key === 'ArrowDown') {
    e.preventDefault()
    next()
  } else if (e.key === 'ArrowLeft' || e.key === 'ArrowUp') {
    e.preventDefault()
    previous()
  } else if (e.key === 'Escape') {
    skip()
  }
}

onMounted(() => {
  window.addEventListener('keydown', handleKeyDown)
})

onUnmounted(() => {
  window.removeEventListener('keydown', handleKeyDown)
})

let touchStartX = 0

const handleTouchStart = (e: TouchEvent) => {
  touchStartX = e.touches[0].clientX
}

const handleTouchEnd = (e: TouchEvent) => {
  const touchEndX = e.changedTouches[0].clientX
  const diff = touchStartX - touchEndX
  if (Math.abs(diff) > 50) {
    if (diff > 0) next()
    else previous()
  }
}
</script>

<template>
  <div
    class="onboarding-container"
    @touchstart.passive="handleTouchStart"
    @touchend="handleTouchEnd"
  >
    <header class="onboarding-header">
      <div class="progress-bar">
        <div
          class="progress-fill"
          :style="{ width: `${(step / totalSteps) * 100}%` }"
        ></div>
      </div>
      <div class="step-indicator">
        <span v-for="s in totalSteps" :key="s" :class="{ active: s === step, completed: s < step }">
          {{ s }}
        </span>
      </div>
      <button v-if="step < totalSteps" @click="skip" class="skip-btn">Passer</button>
    </header>

    <main class="onboarding-content">
      <component :is="currentStepData.component" :on-next="next" :on-previous="previous" />
    </main>

    <footer v-if="step < totalSteps" class="onboarding-footer">
      <button v-if="step > 1" @click="previous" class="btn btn-secondary">
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <polyline points="15 18 9 12 15 6" />
        </svg>
        Précédent
      </button>
      <button @click="next" class="btn btn-primary">
        {{ step === totalSteps ? 'Commencer' : 'Suivant' }}
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <polyline points="9 18 15 12 9 6" />
        </svg>
      </button>
    </footer>
  </div>
</template>

<style scoped>
.onboarding-container {
  position: relative;
  min-height: 100svh;
  display: flex;
  flex-direction: column;
  background: var(--bg);
  touch-action: pan-y;
  overflow-x: hidden;
}

.onboarding-container::before {
  content: '';
  position: fixed;
  inset: -25%;
  z-index: 0;
  pointer-events: none;
  background:
    radial-gradient(620px 420px at 12% 8%, color-mix(in srgb, var(--accent) 16%, transparent), transparent 70%),
    radial-gradient(720px 520px at 88% 88%, color-mix(in srgb, var(--accent) 13%, transparent), transparent 70%),
    radial-gradient(420px 320px at 72% 12%, color-mix(in srgb, var(--accent) 9%, transparent), transparent 60%);
}

.onboarding-header,
.onboarding-content,
.onboarding-footer {
  position: relative;
  z-index: 1;
}

.onboarding-header {
  padding: 24px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
}

.progress-bar {
  flex: 1;
  height: 4px;
  background: color-mix(in srgb, var(--accent) 18%, var(--border));
  border-radius: 2px;
  overflow: hidden;
}

.progress-fill {
  height: 100%;
  background: linear-gradient(90deg, var(--accent) 0%, color-mix(in srgb, var(--accent) 75%, #fff 25%) 100%);
  border-radius: 2px;
  transition: width 0.3s ease;
}

.step-indicator {
  display: flex;
  gap: 8px;
}

.step-indicator span {
  width: 28px;
  height: 28px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 12px;
  font-weight: 600;
  background: var(--border);
  color: var(--text);
  transition: all 0.2s;
}

.step-indicator span.active {
  background: var(--accent);
  color: white;
}

.step-indicator span.completed {
  background: var(--accent);
  color: white;
}

.skip-btn {
  background: none;
  border: none;
  color: var(--text);
  font-size: 14px;
  cursor: pointer;
  padding: 8px 12px;
  border-radius: 6px;
  transition: background 0.2s;
}

.skip-btn:hover {
  background: var(--accent-bg);
  color: var(--accent);
}

.onboarding-content {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 8px 24px 32px;
  overflow-y: auto;
}

.onboarding-footer {
  padding: 20px 24px 28px;
  display: flex;
  justify-content: space-between;
  gap: 16px;
  border-top: 1px solid var(--border);
  background: var(--bg);
}

.btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 14px 28px;
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

.btn-primary:hover {
  filter: brightness(1.1);
}

.btn-secondary {
  background: var(--accent-bg);
  color: var(--accent);
  border: 1px solid var(--accent-border);
}

.btn-secondary:hover {
  background: var(--accent-border);
}

.btn svg {
  flex-shrink: 0;
}

@media (max-width: 480px) {
  .onboarding-header {
    padding: 16px;
  }
  .onboarding-footer {
    padding: 16px;
    flex-direction: column-reverse;
  }
  .btn {
    width: 100%;
  }
  .step-indicator span {
    width: 24px;
    height: 24px;
    font-size: 11px;
  }
}
</style>