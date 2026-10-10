<script setup lang="ts">
import { computed, ref } from 'vue'
import { useRouter } from 'vue-router'
import {
  Archive,
  ArrowLeft,
  ArrowRight,
  BookOpenCheck,
  Check,
  Database,
  FileText,
  MousePointerClick,
  RotateCcw,
  Settings,
  Sparkles
} from '@lucide/vue'

const router = useRouter()

interface GuideStep {
  id: string
  title: string
  subtitle: string
  points: string[]
  cta: string
  routeName: string
  visual: 'import' | 'assistant' | 'fiche' | 'archive' | 'data' | 'settings'
}

const steps: GuideStep[] = [
  {
    id: 'donnees',
    title: '1 · Importez vos données',
    subtitle: 'La base de tout : vos exploitations depuis Excel',
    points: [
      'Glissez votre fichier .xlsx dans la zone d’import de l’écran Données',
      'Vérifiez le récapitulatif : exploitations, cultures, unités agricoles',
      'Retrouvez chaque exploitation avec la recherche (opérateur, village, culture…)'
    ],
    cta: 'Ouvrir Données',
    routeName: 'Data',
    visual: 'import'
  },
  {
    id: 'assistant',
    title: '2 · Posez vos questions à l’assistant',
    subtitle: 'Interrogez vos données en langage naturel, à l’écrit ou à la voix',
    points: [
      'Configurez une fois votre clé API depuis l’écran Accueil (roue dentée)',
      'Utilisez les suggestions ou dictez au micro',
      'Validez les actions proposées (ex : créer une fiche) en un clic'
    ],
    cta: 'Ouvrir l’assistant',
    routeName: 'Main',
    visual: 'assistant'
  },
  {
    id: 'fiche',
    title: '3 · Générez une fiche de traçabilité',
    subtitle: 'Automatique ou manuelle, prête à imprimer avec QR code',
    points: [
      'Mode auto : choisissez un produit et une quantité, Vobox répartit',
      'Mode manuel : ajustez producteur par producteur',
      'Imprimez ou flashez le QR code de chaque lot'
    ],
    cta: 'Créer une fiche',
    routeName: 'Sheet',
    visual: 'fiche'
  },
  {
    id: 'archives',
    title: '4 · Retrouvez tout dans les Archives',
    subtitle: 'Chaque fiche générée est conservée et triable par date',
    points: [
      'Triez par date croissante / décroissante',
      'Rouvrez une fiche pour la réimprimer',
      'Suivez les participations par producteur'
    ],
    cta: 'Voir les archives',
    routeName: 'Archive',
    visual: 'archive'
  },
  {
    id: 'exploitations',
    title: '5 · Explorez vos exploitations',
    subtitle: 'Une carte d’identité complète par producteur',
    points: [
      'Cliquez une tuile dans Données pour ouvrir sa fiche',
      'Cultures, unités, superficies et localisation au même endroit',
      'Accédez à l’historique du producteur en un clic'
    ],
    cta: 'Explorer',
    routeName: 'Data',
    visual: 'data'
  },
  {
    id: 'reglages',
    title: '6 · Personnalisez l’app',
    subtitle: 'Thème, profil, mises à jour : faites-en votre outil',
    points: [
      'Choisissez votre thème (clair, sombre, bleu…)',
      'Ajoutez votre photo et vos infos de profil',
      'Restez à jour avec le canal stable ou bêta'
    ],
    cta: 'Ouvrir Paramètres',
    routeName: 'Settings',
    visual: 'settings'
  }
]

const STORAGE_KEY = 'Vobox_guide_progress'
const stored = ref<string[]>([])
try {
  stored.value = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? '[]')
} catch {
  stored.value = []
}

const activeIndex = ref(0)
const active = computed(() => steps[activeIndex.value])

const isDone = (id: string) => stored.value.includes(id)
const progress = computed(() =>
  Math.round((stored.value.length / steps.length) * 100)
)

const persist = () => {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(stored.value))
}

const markDone = (id: string) => {
  if (!isDone(id)) {
    stored.value = [...stored.value, id]
    persist()
  }
}

const reset = () => {
  stored.value = []
  persist()
  activeIndex.value = 0
}

const goStep = (i: number) => {
  activeIndex.value = Math.min(Math.max(i, 0), steps.length - 1)
}

const openStep = (step: GuideStep) => {
  markDone(step.id)
  router.push({ name: step.routeName })
}

const icons: Record<GuideStep['visual'], unknown> = {
  import: Database,
  assistant: Sparkles,
  fiche: FileText,
  archive: Archive,
  data: MousePointerClick,
  settings: Settings
}
</script>

<template>
  <div class="guide-page">
    <header class="page-header">
      <div class="page-header-text">
        <p class="eyebrow"><BookOpenCheck :size="14" /> Prise en main</p>
        <h1>Guide visuel Vobox</h1>
        <p>6 étapes pour passer de votre fichier Excel à une fiche imprimée.</p>
      </div>
      <div class="header-side">
        <span class="header-chip">{{ stored.length }} / {{ steps.length }} vues</span>
        <button type="button" class="ghost-btn" @click="reset">
          <RotateCcw :size="14" /> Recommencer
        </button>
      </div>
    </header>

    <div class="progress-wrap" role="progressbar" :aria-valuenow="progress" aria-valuemin="0" aria-valuemax="100">
      <div class="progress-track">
        <div class="progress-fill" :style="{ width: progress + '%' }" />
      </div>
      <span class="progress-label">{{ progress }} %</span>
    </div>

    <div class="guide-layout">
      <nav class="stepper" aria-label="Étapes du tutoriel">
        <button
          v-for="(s, i) in steps"
          :key="s.id"
          type="button"
          class="step-btn"
          :class="{ active: i === activeIndex, done: isDone(s.id) }"
          @click="goStep(i)"
        >
          <span class="step-dot">
            <Check v-if="isDone(s.id)" :size="13" />
            <span v-else>{{ i + 1 }}</span>
          </span>
          <span class="step-label">{{ s.title.slice(4) }}</span>
        </button>
      </nav>

      <section class="step-card" :key="active.id">
        <div class="visual" :class="'visual-' + active.visual" aria-hidden="true">
          <div class="mock-window">
            <span class="mock-bar"><i /><i /><i /></span>
            <div class="mock-body">
              <template v-if="active.visual === 'import'">
                <span class="mock-drop">Glissez votre .xlsx ici</span>
                <span class="mock-row"><i class="ok" />12 exploitations importées</span>
                <span class="mock-row"><i class="ok" />34 cultures · 58 unités</span>
              </template>
              <template v-else-if="active.visual === 'assistant'">
                <span class="mock-bubble user">Quel est l’état de mes exploitations ?</span>
                <span class="mock-bubble bot">3 fiches récentes · 200 kg de cacao dispo…</span>
                <span class="mock-chips"><i />Créer une fiche<i />Voir les stocks</span>
              </template>
              <template v-else-if="active.visual === 'fiche'">
                <span class="mock-doc">FICHE AGX-2026-014 · Cacao · 200 kg</span>
                <span class="mock-qr">QR</span>
                <span class="mock-row"><i class="ok" />Prête à imprimer</span>
              </template>
              <template v-else-if="active.visual === 'archive'">
                <span class="mock-row">AGX-2026-014 · 12 sept.</span>
                <span class="mock-row">AGX-2026-013 · 8 sept.</span>
                <span class="mock-row">AGX-2026-012 · 2 sept.</span>
              </template>
              <template v-else-if="active.visual === 'data'">
                <span class="mock-row"><i class="ok" />A. Diallo · 4,2 ha</span>
                <span class="mock-row"><i class="ok" />M. Sow · 2,8 ha</span>
                <span class="mock-row"><i class="ok" />F. Bah · 6,1 ha</span>
              </template>
              <template v-else>
                <span class="mock-row">Thème · Clair / Sombre / Bleu</span>
                <span class="mock-row">Profil · Photo + nom</span>
                <span class="mock-row">Mises à jour · Stable / Bêta</span>
              </template>
            </div>
          </div>
          <span class="visual-icon"><component :is="icons[active.visual]" :size="22" /></span>
        </div>

        <div class="step-body">
          <h2>{{ active.title }}</h2>
          <p class="subtitle">{{ active.subtitle }}</p>
          <ul class="points">
            <li v-for="(p, i) in active.points" :key="i">{{ p }}</li>
          </ul>
          <div class="step-actions">
            <button type="button" class="primary-btn" @click="openStep(active)">
              {{ active.cta }}
            </button>
            <button
              v-if="!isDone(active.id)"
              type="button"
              class="ghost-btn"
              @click="markDone(active.id)"
            >
              <Check :size="14" /> Marquer comme vu
            </button>
            <span v-else class="done-badge"><Check :size="14" /> Étape vue</span>
          </div>
          <div class="nav-row">
            <button
              type="button"
              class="nav-btn"
              :disabled="activeIndex === 0"
              @click="goStep(activeIndex - 1)"
            >
              <ArrowLeft :size="15" /> Précédent
            </button>
            <span class="counter">{{ activeIndex + 1 }} / {{ steps.length }}</span>
            <button
              type="button"
              class="nav-btn"
              :disabled="activeIndex === steps.length - 1"
              @click="goStep(activeIndex + 1)"
            >
              Suivant <ArrowRight :size="15" />
            </button>
          </div>
        </div>
      </section>
    </div>

    <section class="faq">
      <h2>Questions fréquentes</h2>
      <div class="faq-grid">
        <details open>
          <summary>Par où commencer ?</summary>
          <p>Importez d’abord votre Excel dans <b>Données</b>, puis demandez à l’assistant « Quel est l’état global ? » pour vérifier que tout est bien chargé.</p>
        </details>
        <details>
          <summary>L’assistant ne répond pas, que faire ?</summary>
          <p>Ouvrez <b>Accueil → roue dentée</b> et renseignez l’URL + la clé API + le modèle. Sans clé, l’assistant ne peut pas fonctionner.</p>
        </details>
        <details>
          <summary>Comment imprimer une fiche ?</summary>
          <p>Générez-la dans <b>Fiche</b> (auto ou manuel), puis utilisez le bouton d’impression. Le QR code permet de retrouver la fiche à tout moment.</p>
        </details>
        <details>
          <summary>Où sont mes données ?</summary>
          <p>Tout est stocké <b>en local</b> sur votre poste (base SQLite du dossier applicatif). Pensez au bouton <b>Sauvegarder</b> dans Données pour exporter un backup JSON.</p>
        </details>
      </div>
    </section>
  </div>
</template>

<style scoped>
.guide-page {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  padding: 8px 8px 32px;
}
.page-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
  padding: 8px 12px 12px;
  flex-wrap: wrap;
}
.eyebrow {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  margin: 0 0 6px;
  font-size: 12px;
  font-weight: 700;
  letter-spacing: 0.6px;
  text-transform: uppercase;
  color: var(--primary);
}
.page-header h1 {
  margin: 0 0 4px;
  font-size: 28px;
  font-weight: 700;
  color: var(--on-surface);
}
.page-header p { margin: 0; color: var(--on-surface-variant); font-size: 14px; }
.header-side { display: flex; align-items: center; gap: 8px; }
.header-chip {
  padding: 8px 14px;
  border-radius: var(--shape-full);
  background: var(--surface-container-high);
  font-size: 13px;
  font-weight: 600;
  color: var(--on-surface-variant);
}
.ghost-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 8px 14px;
  border-radius: var(--shape-full);
  border: 1px solid var(--outline-variant);
  background: transparent;
  color: var(--on-surface-variant);
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
}
.ghost-btn:hover { border-color: var(--primary); color: var(--primary); }
.progress-wrap { display: flex; align-items: center; gap: 10px; padding: 0 12px 14px; }
.progress-track {
  flex: 1;
  height: 8px;
  border-radius: 99px;
  background: var(--surface-container-high);
  overflow: hidden;
}
.progress-fill {
  height: 100%;
  border-radius: 99px;
  background: var(--primary);
  transition: width 300ms ease;
}
.progress-label { font-size: 12px; font-weight: 700; color: var(--on-surface-variant); min-width: 40px; text-align: right; }
.guide-layout {
  display: grid;
  grid-template-columns: 220px minmax(0, 1fr);
  gap: 16px;
  padding: 0 12px;
  align-items: start;
}
.stepper { display: flex; flex-direction: column; gap: 6px; }
.step-btn {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  border: none;
  border-radius: var(--shape-md);
  background: transparent;
  color: var(--on-surface-variant);
  font-size: 13.5px;
  font-weight: 600;
  text-align: left;
  cursor: pointer;
}
.step-btn:hover { background: var(--surface-container-low); }
.step-btn.active { background: var(--secondary-container); color: var(--on-secondary-container); }
.step-dot {
  width: 26px; height: 26px; flex-shrink: 0;
  display: grid; place-items: center;
  border-radius: 50%;
  background: var(--surface-container-high);
  font-size: 12px; font-weight: 700;
}
.step-btn.active .step-dot { background: var(--primary); color: var(--on-primary); }
.step-btn.done .step-dot { background: #16a34a; color: #fff; }
.step-label { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.step-card {
  display: grid;
  grid-template-columns: minmax(0, 300px) minmax(0, 1fr);
  gap: 0;
  border-radius: var(--shape-lg);
  background: var(--surface-container-low);
  box-shadow: var(--elev-1);
  overflow: hidden;
}
.visual {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 24px 18px;
  background: linear-gradient(135deg, var(--primary-container), var(--secondary-container));
}
.mock-window {
  width: 100%;
  max-width: 250px;
  border-radius: 14px;
  background: var(--surface-container-lowest, #fff);
  box-shadow: var(--elev-2);
  overflow: hidden;
}
.mock-bar { display: flex; gap: 5px; padding: 10px 12px; background: rgba(0,0,0,0.05); }
.mock-bar i { width: 9px; height: 9px; border-radius: 50%; background: #c4c4c4; }
.mock-bar i:first-child { background: #f06666; }
.mock-bar i:nth-child(2) { background: #f5b83d; }
.mock-bar i:nth-child(3) { background: #3ecf6f; }
.mock-body { display: flex; flex-direction: column; gap: 8px; padding: 14px 12px; }
.mock-drop {
  border: 1.5px dashed var(--primary);
  border-radius: 10px;
  padding: 14px 10px;
  text-align: center;
  font-size: 12px;
  font-weight: 700;
  color: var(--primary);
  background: color-mix(in srgb, var(--primary) 8%, transparent);
}
.mock-row {
  display: flex; align-items: center; gap: 7px;
  font-size: 12px; font-weight: 600;
  background: var(--surface-container-high);
  border-radius: 8px; padding: 8px 10px;
  color: var(--on-surface);
}
.mock-row .ok { width: 14px; height: 14px; border-radius: 50%; background: #16a34a; flex-shrink: 0; }
.mock-bubble { font-size: 12px; padding: 8px 11px; border-radius: 12px; max-width: 90%; }
.mock-bubble.user { align-self: flex-end; background: var(--primary); color: #fff; border-bottom-right-radius: 4px; }
.mock-bubble.bot { align-self: flex-start; background: var(--surface-container-high); border-bottom-left-radius: 4px; }
.mock-chips { display: flex; gap: 6px; font-size: 11px; font-weight: 700; color: var(--primary); }
.mock-chips i { width: 8px; height: 8px; border-radius: 50%; background: var(--primary); align-self: center; }
.mock-doc { font-size: 12px; font-weight: 800; background: #1d1b20; color: #fff; border-radius: 8px; padding: 10px; text-align: center; }
.mock-qr {
  align-self: center;
  width: 64px; height: 64px;
  display: grid; place-items: center;
  border: 2px solid #1d1b20; border-radius: 10px;
  font-weight: 800; letter-spacing: 1px;
  background: repeating-conic-gradient(#1d1b20 0 25%, #fff 0 50%) 0 0 / 12px 12px;
  color: #fff; text-shadow: 0 1px 3px #000;
}
.visual-icon {
  position: absolute; right: 12px; bottom: 12px;
  width: 40px; height: 40px;
  display: grid; place-items: center;
  border-radius: 50%;
  background: var(--primary); color: var(--on-primary);
  box-shadow: var(--elev-1);
}
.step-body { padding: 22px 22px 18px; display: flex; flex-direction: column; gap: 10px; }
.step-body h2 { margin: 0; font-size: 20px; color: var(--on-surface); }
.subtitle { margin: 0; color: var(--on-surface-variant); font-size: 14px; }
.points { margin: 4px 0 0; padding-left: 18px; display: flex; flex-direction: column; gap: 6px; color: var(--on-surface); font-size: 14px; line-height: 1.5; }
.step-actions { display: flex; align-items: center; gap: 10px; margin-top: 8px; flex-wrap: wrap; }
.primary-btn {
  height: 42px; padding: 0 22px;
  border: none; border-radius: var(--shape-full);
  background: var(--primary); color: var(--on-primary);
  font-weight: 700; font-size: 14px; cursor: pointer;
}
.primary-btn:hover { box-shadow: var(--elev-1); }
.done-badge { display: inline-flex; align-items: center; gap: 6px; font-size: 13px; font-weight: 700; color: #16a34a; }
.nav-row { display: flex; align-items: center; justify-content: space-between; gap: 10px; margin-top: 10px; padding-top: 12px; border-top: 1px solid var(--outline-variant); }
.nav-btn {
  display: inline-flex; align-items: center; gap: 6px;
  border: none; background: transparent;
  color: var(--primary); font-weight: 700; font-size: 13px; cursor: pointer;
  padding: 6px 4px;
}
.nav-btn:disabled { opacity: 0.35; cursor: default; }
.counter { font-size: 12px; font-weight: 700; color: var(--on-surface-variant); }
.faq { padding: 22px 12px 0; max-width: 900px; }
.faq h2 { font-size: 17px; color: var(--on-surface); margin: 0 0 10px; }
.faq-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(240px, 1fr)); gap: 10px; }
.faq details {
  background: var(--surface-container-low);
  border-radius: var(--shape-md);
  padding: 12px 14px;
  box-shadow: var(--elev-1);
}
.faq summary { font-weight: 700; font-size: 13.5px; cursor: pointer; color: var(--on-surface); }
.faq p { font-size: 13px; color: var(--on-surface-variant); line-height: 1.55; }
@media (max-width: 900px) {
  .guide-layout { grid-template-columns: 1fr; }
  .stepper { flex-direction: row; overflow-x: auto; padding-bottom: 4px; }
  .step-btn { flex-shrink: 0; }
  .step-label { display: none; }
  .step-card { grid-template-columns: 1fr; }
}
</style>
