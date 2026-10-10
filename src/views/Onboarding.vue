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
  background: var(--surface);
  touch-action: pan-y;
  overflow-x: hidden;
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
  background: var(--secondary-container);
  border-radius: var(--shape-full);
  overflow: hidden;
}

.progress-fill {
  height: 100%;
  background: var(--primary);
  border-radius: var(--shape-full);
  transition: width 0.3s cubic-bezier(0.2, 0, 0, 1);
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
  background: var(--surface-container-highest);
  color: var(--on-surface-variant);
  transition: background var(--motion), color var(--motion);
}

.step-indicator span.active {
  background: var(--primary);
  color: var(--on-primary);
}

.step-indicator span.completed {
  background: var(--primary-container);
  color: var(--on-primary-container);
}

.skip-btn {
  background: none;
  border: none;
  color: var(--primary);
  font-size: 14px;
  font-weight: 600;
  letter-spacing: 0.1px;
  cursor: pointer;
  padding: 10px 16px;
  border-radius: var(--shape-full);
  transition: background var(--motion);
}

.skip-btn:hover {
  background: var(--primary-container);
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
  border-top: 1px solid var(--outline-variant);
  background: var(--surface-container-low);
}

.btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  height: 52px;
  padding: 0 28px;
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

.btn-primary:hover {
  box-shadow: var(--elev-1);
  filter: brightness(1.06);
}

.btn-secondary {
  background: var(--secondary-container);
  color: var(--on-secondary-container);
}

.btn-secondary:hover {
  box-shadow: var(--elev-1);
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