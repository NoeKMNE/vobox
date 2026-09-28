<script setup lang="ts">
import { History, Map, PenLine } from '@lucide/vue'
import CalligraphText from '../CalligraphText.vue'
import TiltCard from './TiltCard.vue'

defineProps<{
  onNext: () => void
  onPrevious: () => void
}>()

const features = [
  { icon: Map, title: 'Cartographie', text: 'Visualisez vos parcelles sur le terrain' },
  { icon: History, title: 'Historique des cultures', text: 'Retrouvez les rotations des saisons passées' },
  { icon: PenLine, title: 'Notes terrain', text: 'Gardez vos observations à portée de main' }
]

const parcels = [0, 1, 2, 3, 4, 5]
</script>

<template>
  <div class="step" data-step="2">
    <TiltCard class="hero" :max-tilt="6">
      <div class="onb-card hero-card">
        <span class="onb-chip">Étape 2 · Parcelles</span>
        <div class="hero-icon">
          <Map :size="34" />
        </div>
        <h1><CalligraphText text="Gérez vos parcelles" :initial="true" /></h1>
        <p class="hero-text">
          <CalligraphText
            text="Ajoutez vos champs, définissez vos cultures et suivez l’évolution de chaque parcelle tout au long de la saison."
            :initial="true"
          />
        </p>

        <div class="parcel-map">
          <div v-for="(p, i) in parcels" :key="p" class="parcel" :class="`parcel-${i}`">
            <span class="parcel-soil"></span>
            <span class="parcel-rows"></span>
          </div>
        </div>
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

.hero-icon {
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

/* ---------- Mini-maquette de parcelles ---------- */
.parcel-map {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 12px;
  margin: 26px auto 0;
  max-width: 420px;
}

.parcel {
  position: relative;
  aspect-ratio: 1.25;
  border-radius: 14px;
  border: 1px solid rgba(0, 0, 0, 0.08);
  overflow: hidden;
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.35), 0 6px 16px -6px rgba(0, 0, 0, 0.25);
  transition: transform 0.2s ease;
}

.parcel:hover {
  transform: translateY(-3px);
}

.parcel-soil {
  position: absolute;
  inset: 0;
  background: linear-gradient(145deg, #b7e4a8 0%, #74c469 100%);
}

.parcel-rows {
  position: absolute;
  inset: 0;
  background: repeating-linear-gradient(90deg, transparent 0 10px, rgba(255, 255, 255, 0.28) 10px 12px);
}

.parcel-0 .parcel-soil { background: linear-gradient(145deg, #d0ecb8 0%, #8fd373 100%); }
.parcel-1 .parcel-soil { background: linear-gradient(145deg, #a8d98b 0%, #5fb057 100%); }
.parcel-2 .parcel-soil { background: linear-gradient(145deg, #e8d696 0%, #c9a84e 100%); }
.parcel-3 .parcel-soil { background: linear-gradient(145deg, #c2e59a 0%, #79c45d 100%); }
.parcel-4 .parcel-soil { background: linear-gradient(145deg, #b3d9a4 0%, #6cbc5e 100%); }
.parcel-5 .parcel-soil { background: linear-gradient(145deg, #dfcb92 0%, #b99243 100%); }

.parcel-5 {
  border-color: var(--accent-border);
  box-shadow:
    inset 0 1px 0 rgba(255, 255, 255, 0.4),
    0 0 0 2px var(--accent-bg),
    0 10px 22px -8px color-mix(in srgb, var(--accent) 60%, transparent);
}

@media (max-width: 480px) {
  .onb-card {
    padding: 26px 20px;
  }
  h1 {
    font-size: 24px;
  }
  .parcel-map {
    gap: 8px;
  }
}
</style>