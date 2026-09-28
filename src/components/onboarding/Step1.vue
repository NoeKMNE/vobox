<script setup lang="ts">
import { LayoutDashboard, Leaf, Sprout, TrendingUp } from '@lucide/vue'
import CalligraphText from '../CalligraphText.vue'
import TiltCard from './TiltCard.vue'

defineProps<{
  onNext: () => void
  onPrevious: () => void
}>()

const features = [
  { icon: LayoutDashboard, title: 'Dashboard complet', text: 'Toutes les infos de votre exploitation au même endroit' },
  { icon: Sprout, title: 'Suivi des cultures', text: 'L’évolution de vos cultures en temps réel' },
  { icon: TrendingUp, title: 'Analyses & prévisions', text: 'Des décisions guidées par vos données' }
]
</script>

<template>
  <div class="step" data-step="1">
    <TiltCard class="hero" :max-tilt="6">
      <div class="onb-card hero-card">
        <div class="hero-glow"></div>
        <span class="onb-chip">Étape 1 · Bienvenue</span>
        <div class="hero-icon">
          <Leaf :size="34" />
        </div>
        <h1><CalligraphText text="Bienvenue sur AgroBox" :initial="true" /></h1>
        <p class="hero-text">
          <CalligraphText
            text="Votre assistant intelligent pour la gestion agricole. Suivez vos cultures, optimisez vos rendements et prenez de meilleures décisions."
            :initial="true"
          />
        </p>
      </div>
    </TiltCard>

    <div class="onb-mini-row">
      <TiltCard v-for="f in features" :key="f.title" :max-tilt="14">
        <button type="button" class="onb-mini" @click="onNext">
          <span class="icon"><component :is="f.icon" :size="18" /></span>
          <strong>{{ f.title }}</strong>
          <span>{{ f.text }}</span>
        </button>
      </TiltCard>
    </div>
  </div>
</template>

<style scoped>
.step {
  width: 100%;
  max-width: 640px;
  margin: 0 auto;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 18px;
  padding: 8px 0 20px;
}

.hero-card {
  width: calc(100% - 16px);
  text-align: center;
  overflow: hidden;
}

.hero-glow {
  position: absolute;
  top: -80px;
  left: 50%;
  transform: translateX(-50%);
  width: 300px;
  height: 200px;
  border-radius: 999px;
  background: radial-gradient(closest-side, var(--accent-bg), transparent);
  filter: blur(18px);
  pointer-events: none;
}

.hero-icon {
  position: relative;
  width: 76px;
  height: 76px;
  margin: 20px auto 18px;
  display: grid;
  place-items: center;
  border-radius: 22px;
  background: linear-gradient(135deg, var(--accent) 0%, color-mix(in srgb, var(--accent) 65%, #000 35%) 100%);
  color: #fff;
  box-shadow:
    inset 0 1px 0 rgba(255, 255, 255, 0.25),
    0 12px 28px -6px color-mix(in srgb, var(--accent) 65%, transparent);
}

h1 {
  font-size: 30px;
  font-weight: 700;
  color: var(--text-h);
  margin: 0 0 12px;
}

.hero-text {
  font-size: 15px;
  line-height: 1.65;
  color: var(--text);
  margin: 0 auto;
  max-width: 42ch;
}

@media (max-width: 480px) {
  .onb-card {
    padding: 26px 20px;
  }
  h1 {
    font-size: 24px;
  }
}
</style>