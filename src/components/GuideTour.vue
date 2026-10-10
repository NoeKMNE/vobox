<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { ArrowRight, BookOpenCheck, Sparkles, X } from '@lucide/vue'

const STORAGE_KEY = 'Vobox_guide_seen'
const router = useRouter()

const visible = ref(false)
try {
  visible.value = localStorage.getItem(STORAGE_KEY) !== '1'
} catch {
  visible.value = true
}

const slide = ref(0)
const slides = [
  {
    title: 'Bienvenue sur Vobox 👋',
    text: 'En 3 gestes : importez votre Excel, générez une fiche, retrouvez-la dans les archives.'
  },
  {
    title: '1 · Données → Fiche → Archives',
    text: 'Glissez votre .xlsx dans Données, créez une fiche dans Fiche, réimprimez-la depuis Archives.'
  },
  {
    title: '2 · L’assistant vous guide',
    text: 'Depuis Accueil, posez vos questions à l’écrit ou à la voix. Configurez juste votre clé API une fois.'
  }
]

const next = () => {
  if (slide.value < slides.length - 1) slide.value++
  else openGuide()
}

const dismiss = () => {
  try {
    localStorage.setItem(STORAGE_KEY, '1')
  } catch { /* stockage indisponible */ }
  visible.value = false
}

const openGuide = () => {
  dismiss()
  router.push({ name: 'Guide' })
}
</script>

<template>
  <div v-if="visible" class="tour" role="dialog" aria-label="Mini tutoriel de prise en main">
    <button type="button" class="tour-close" aria-label="Fermer le tutoriel" @click="dismiss">
      <X :size="15" />
    </button>
    <p class="tour-eyebrow"><Sparkles :size="13" /> Prise en main · {{ slide + 1 }}/{{ slides.length }}</p>
    <h3>{{ slides[slide].title }}</h3>
    <p class="tour-text">{{ slides[slide].text }}</p>
    <div class="tour-dots" aria-hidden="true">
      <span v-for="(_, i) in slides" :key="i" :class="{ on: i === slide }" />
    </div>
    <div class="tour-actions">
      <button type="button" class="link-btn" @click="dismiss">Passer</button>
      <button type="button" class="ghost-btn" @click="openGuide">
        <BookOpenCheck :size="14" /> Guide complet
      </button>
      <button type="button" class="primary-btn" @click="next">
        {{ slide === slides.length - 1 ? 'C’est parti' : 'Suivant' }} <ArrowRight :size="14" />
      </button>
    </div>
  </div>
</template>

<style scoped>
.tour {
  position: fixed;
  right: 18px;
  bottom: 18px;
  z-index: 40;
  width: min(360px, calc(100vw - 36px));
  padding: 16px 16px 14px;
  border-radius: var(--shape-lg);
  background: var(--surface-container-low);
  box-shadow: var(--elev-3);
  border: 1px solid var(--outline-variant);
}
.tour-close {
  position: absolute; top: 8px; right: 8px;
  width: 30px; height: 30px;
  display: grid; place-items: center;
  border: none; border-radius: 50%;
  background: transparent; color: var(--on-surface-variant);
  cursor: pointer;
}
.tour-close:hover { background: var(--surface-container-high); }
.tour-eyebrow {
  display: flex; align-items: center; gap: 6px;
  margin: 0 0 6px;
  font-size: 11px; font-weight: 800;
  letter-spacing: 0.6px; text-transform: uppercase;
  color: var(--primary);
}
.tour h3 { margin: 0 0 6px; font-size: 16px; color: var(--on-surface); }
.tour-text { margin: 0 0 10px; font-size: 13.5px; line-height: 1.55; color: var(--on-surface-variant); }
.tour-dots { display: flex; gap: 6px; margin-bottom: 12px; }
.tour-dots span { width: 22px; height: 4px; border-radius: 99px; background: var(--outline-variant); }
.tour-dots span.on { background: var(--primary); }
.tour-actions { display: flex; align-items: center; gap: 8px; }
.link-btn { border: none; background: none; color: var(--on-surface-variant); font-size: 13px; font-weight: 600; cursor: pointer; padding: 4px; }
.link-btn:hover { color: var(--on-surface); }
.ghost-btn, .primary-btn {
  display: inline-flex; align-items: center; gap: 6px;
  border-radius: var(--shape-full);
  font-size: 12.5px; font-weight: 700; cursor: pointer;
  padding: 9px 14px;
}
.ghost-btn { border: 1px solid var(--outline-variant); background: transparent; color: var(--on-surface); margin-left: auto; }
.primary-btn { border: none; background: var(--primary); color: var(--on-primary); }
</style>
