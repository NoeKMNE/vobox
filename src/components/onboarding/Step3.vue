<script setup lang="ts">
import { ref } from 'vue'
import { ArrowRight, Palette, RotateCcw } from '@lucide/vue'
import TiltCard from './TiltCard.vue'
import { ACCENTS, currentAccentHex, resetAccent, setAccent } from '../../composables/useAccent'
import { useTheme, type Theme } from '../../composables/useTheme'

defineProps<{
  onNext: () => void
  onPrevious: () => void
}>()

const { currentTheme, themes, setTheme } = useTheme()
const selected = ref(currentAccentHex())

const themeLabels: Record<Theme, string> = {
  light: 'Clair',
  dark: 'Sombre',
  mono: 'Mono',
  bleu: 'Bleu',
  neon: 'Néon'
}
const themeLabel = (t: Theme) => themeLabels[t]

const pickTheme = (t: Theme) => setTheme(t)

const pick = (hex: string) => {
  selected.value = hex
  setAccent(hex)
}

const reset = () => {
  resetAccent()
  selected.value = ACCENTS[0].hex
}
</script>

<template>
  <div class="step" data-step="3">
    <TiltCard class="hero" :max-tilt="5">
      <div class="onb-card hero-card">
        <span class="onb-chip">Étape 3 · Personnalisation</span>
        <div class="hero-icon">
          <Palette :size="34" />
        </div>
        <h1>Personnalisez l’interface</h1>
        <p class="hero-text">Choisissez un thème et une couleur d’accent : l’application s’adapte instantanément.</p>

        <div class="customize">
          <div class="control-group">
            <span class="control-label">Thème</span>
            <div class="theme-cards">
              <button
                v-for="t in themes"
                :key="t"
                type="button"
                class="theme-card"
                :class="{ active: currentTheme === t }"
                @click="pickTheme(t)"
              >
                <span class="theme-dot" :class="`theme-${t}`"></span>
                {{ themeLabel(t) }}
              </button>
            </div>
          </div>

          <div class="control-group">
            <span class="control-label">Couleur d’accent</span>
            <div class="swatches">
              <button
                v-for="a in ACCENTS"
                :key="a.hex"
                type="button"
                class="swatch"
                :class="{ active: selected === a.hex }"
                :style="{ background: a.hex }"
                :title="a.name"
                :aria-label="a.name"
                @click="pick(a.hex)"
              >
                <svg v-if="selected === a.hex" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3">
                  <polyline points="20 6 9 17 4 12" />
                </svg>
              </button>
            </div>
            <button type="button" class="reset-link" @click="reset">
              <RotateCcw :size="13" />
              Réinitialiser
            </button>
          </div>
        </div>

        <div class="preview-frame">
          <div class="chrome">
            <span class="dot dot-red"></span>
            <span class="dot dot-yellow"></span>
            <span class="dot dot-green"></span>
            <span class="url">app.Vobox.fr</span>
          </div>
          <div class="preview-body">
            <div class="preview-header">
              <span class="preview-logo">A</span>
              <span class="preview-name">Vobox</span>
              <span class="preview-menu"><span></span><span></span><span></span></span>
            </div>
            <div class="preview-content">
              <div class="stat accent"></div>
              <div class="stat"></div>
              <div class="row"></div>
              <div class="row short"></div>
            </div>
          </div>
        </div>

        <button type="button" class="onb-cta start-btn" @click="onNext">
          Commencer
          <ArrowRight :size="18" />
        </button>
      </div>
    </TiltCard>
  </div>
</template>

<style scoped>
.step {
  width: 100%;
  max-width: 660px;
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
  margin: 20px auto 16px;
  display: grid;
  place-items: center;
  border-radius: var(--shape-xl);
  background: var(--primary-container);
  color: var(--on-primary-container);
}

h1 {
  font-size: 28px;
  font-weight: 700;
  color: var(--text-h);
  margin: 0 0 10px;
}

.hero-text {
  font-size: 15px;
  line-height: 1.6;
  color: var(--text);
  margin: 0 auto;
  max-width: 46ch;
}

/* ---------- Contrôles ---------- */
.customize {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 20px;
  text-align: left;
  margin-top: 24px;
}

.control-group {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.control-label {
  font-size: 12px;
  font-weight: 700;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  color: var(--text);
}

.theme-cards {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}

.theme-card {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 9px 14px;
  border-radius: 12px;
  border: 1px solid var(--border);
  background: var(--bg);
  color: var(--text-h);
  font: inherit;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: border-color 0.2s, box-shadow 0.2s, transform 0.2s;
}

.theme-card:hover {
  transform: translateY(-2px);
}

.theme-card.active {
  border-color: var(--accent);
  box-shadow: 0 0 0 3px var(--accent-bg);
}

.theme-dot {
  width: 16px;
  height: 16px;
  border-radius: 50%;
  border: 1px solid var(--border);
}

.theme-light { background: #fff; }
.theme-dark { background: #16171d; }
.theme-mono { background: #fff; box-shadow: inset 0 0 0 2px #000; }
.theme-bleu { background: #f5f9fd; box-shadow: inset 0 0 0 2px #1c7ed6; }
.theme-neon { background: #070b12; box-shadow: inset 0 0 0 2px #00e5ff; }

.swatches {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 10px;
  max-width: 220px;
}

.swatch {
  aspect-ratio: 1;
  border-radius: 50%;
  border: none;
  cursor: pointer;
  display: grid;
  place-items: center;
  color: #fff;
  transition: transform 0.2s ease, box-shadow 0.2s ease;
}

.swatch:hover {
  transform: scale(1.12);
}

.swatch.active {
  box-shadow:
    0 0 0 2px var(--bg),
    0 0 0 4px var(--text-h),
    0 6px 14px -4px rgba(0, 0, 0, 0.35);
  transform: scale(1.1);
}

.reset-link {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  align-self: flex-start;
  background: none;
  border: none;
  padding: 0;
  color: var(--text);
  font-size: 13px;
  cursor: pointer;
  transition: color 0.2s;
}

.reset-link:hover {
  color: var(--accent);
}

/* ---------- Aperçu ---------- */
.preview-frame {
  margin: 26px auto 0;
  max-width: 560px;
  border-radius: 16px;
  border: 1px solid var(--border);
  background: var(--code-bg);
  overflow: hidden;
  text-align: left;
  box-shadow: 0 16px 40px -16px rgba(0, 0, 0, 0.35);
}

.chrome {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 10px 14px;
  border-bottom: 1px solid var(--border);
  background: var(--bg);
}

.chrome .dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
}

.dot-red { background: #ff5f57; }
.dot-yellow { background: #febc2e; }
.dot-green { background: #28c840; }

.chrome .url {
  flex: 1;
  margin-left: 10px;
  padding: 4px 12px;
  border-radius: 8px;
  background: var(--code-bg);
  color: var(--text);
  font-size: 12px;
  text-align: center;
}

.preview-body {
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.preview-header {
  display: flex;
  align-items: center;
  gap: 10px;
}

.preview-logo {
  width: 30px;
  height: 30px;
  display: grid;
  place-items: center;
  border-radius: var(--shape-sm);
  font-size: 14px;
  font-weight: 700;
  color: var(--on-primary);
  background: var(--primary);
}

.preview-name {
  font-size: 14px;
  font-weight: 700;
  color: var(--text-h);
}

.preview-menu {
  margin-left: auto;
  display: flex;
  gap: 5px;
}

.preview-menu span {
  width: 5px;
  height: 5px;
  border-radius: 50%;
  background: var(--text);
  opacity: 0.4;
}

.preview-content {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 12px;
}

.stat {
  height: 54px;
  border-radius: var(--shape-md);
  background: var(--surface-container-high);
  border: none;
}

.stat.accent {
  background: var(--primary-container);
  border: none;
}

.row {
  grid-column: 1 / -1;
  height: 22px;
  border-radius: 8px;
  background: var(--bg);
  border: 1px solid var(--border);
}

.row.short {
  width: 62%;
}

/* ---------- CTA ---------- */
.start-btn {
  margin-top: 28px;
}

@media (max-width: 640px) {
  .customize {
    grid-template-columns: 1fr;
    gap: 16px;
  }
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
