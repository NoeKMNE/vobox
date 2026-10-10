<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import { ArrowLeft, Loader, Printer, Stamp } from '@lucide/vue'
import {
  cloturerSemestre,
  getToken,
  rapportSemestre,
  type RapportSemestre
} from '../composables/useAuth'

const router = useRouter()

const now = new Date()
const mois = now.getMonth()
const isS2 = mois >= 2 && mois <= 7
const defaultSemestre = isS2 ? 'S2' : 'S1'
const defaultAnnee = isS2 ? now.getFullYear() : mois >= 8 ? now.getFullYear() : now.getFullYear() - 1

const semestre = ref(defaultSemestre)
const annee = ref(defaultAnnee)
const rapport = ref<RapportSemestre | null>(null)
const loading = ref(false)
const error = ref('')
const message = ref('')
const cloturant = ref(false)

const titreSemestre = computed(() =>
  semestre.value === 'S1'
    ? `Semestre S1 · sept. ${annee.value} → fév. ${annee.value + 1}`
    : `Semestre S2 · mars → août ${annee.value}`
)

const formatKg = (n: number) =>
  `${new Intl.NumberFormat('fr-FR', { maximumFractionDigits: 1 }).format(n)} kg`

const load = async () => {
  const token = getToken()
  if (!token) return
  loading.value = true
  error.value = ''
  message.value = ''
  try {
    rapport.value = await rapportSemestre(token, semestre.value, annee.value)
  } catch (e) {
    error.value = typeof e === 'string' ? e : 'Chargement impossible'
    rapport.value = null
  } finally {
    loading.value = false
  }
}

watch([semestre, annee], load)
onMounted(load)

const imprimer = () => window.print()

const cloturer = async () => {
  const token = getToken()
  if (!token || cloturant.value) return
  const ok = window.confirm(
    `Enregistrer la clôture du semestre ${semestre.value} ${annee.value} ?\n` +
      'Estimation, alloué et reste seront figés par culture (réinscriptible).'
  )
  if (!ok) return
  cloturant.value = true
  error.value = ''
  message.value = ''
  try {
    const n = await cloturerSemestre(token, semestre.value, annee.value)
    message.value = `Semestre clôturé : ${n} culture${n > 1 ? 's' : ''} enregistrée${n > 1 ? 's' : ''}.`
    await load()
  } catch (e) {
    error.value = typeof e === 'string' ? e : 'Clôture impossible'
  } finally {
    cloturant.value = false
  }
}
</script>

<template>
  <div class="rapport-page">
    <header class="page-header no-print">
      <button type="button" class="back-btn" aria-label="Retour aux archives" @click="router.push({ name: 'Archive' })">
        <ArrowLeft :size="16" />
      </button>
      <div class="page-header-text">
        <h1>Restes du semestre</h1>
        <p>Estimation − alloué : ce que chaque producteur peut encore livrer</p>
      </div>
      <div class="page-actions">
        <select v-model="semestre" class="sem-select" aria-label="Semestre">
          <option value="S1">S1 (sept → fév)</option>
          <option value="S2">S2 (mars → août)</option>
        </select>
        <input
          v-model.number="annee"
          type="number"
          class="sem-select year-input"
          aria-label="Année de début de semestre"
          min="2000"
          max="2100"
        />
        <button type="button" class="action-btn" :disabled="cloturant" @click="cloturer">
          <Stamp :size="15" />{{ cloturant ? 'En cours…' : 'Clôturer' }}
        </button>
        <button type="button" class="action-btn primary" @click="imprimer">
          <Printer :size="15" />Imprimer
        </button>
      </div>
    </header>

    <div class="print-title">
      <h1>Vobox — Restes du semestre {{ rapport?.semestre }} {{ rapport?.annee }}</h1>
      <p>{{ titreSemestre }}</p>
    </div>

    <p v-if="error" class="msg error no-print">{{ error }}</p>
    <p v-if="message" class="msg ok no-print">{{ message }}</p>
    <div v-if="loading" class="loading no-print"><Loader :size="22" class="spin" /> Chargement…</div>

    <template v-if="rapport && !loading">
      <p class="sem-line">{{ titreSemestre }}</p>

      <section class="kpi-row">
        <div class="kpi">
          <span class="kpi-label">Estimé semestre</span>
          <b>{{ formatKg(rapport.estime_kg) }}</b>
        </div>
        <div class="kpi">
          <span class="kpi-label">Alloué (fiches)</span>
          <b>{{ formatKg(rapport.alloue_kg) }}</b>
        </div>
        <div class="kpi highlight">
          <span class="kpi-label">Reste livrable</span>
          <b>{{ formatKg(rapport.reste_kg) }}</b>
        </div>
        <div v-if="rapport.cloture_le" class="kpi closed">
          <span class="kpi-label">Clôturé le</span>
          <b>{{ rapport.cloture_le.slice(0, 10) }}</b>
        </div>
      </section>

      <section class="report-card">
        <h2>Par produit</h2>
        <div class="table-wrap">
          <table>
            <thead>
              <tr><th>Produit</th><th class="num">Estimé</th><th class="num">Alloué</th><th class="num">Reste</th><th class="num">Producteurs</th></tr>
            </thead>
            <tbody>
              <tr v-for="p in rapport.par_produit" :key="p.produit">
                <td class="cap">{{ p.produit }}</td>
                <td class="num">{{ formatKg(p.estime_kg) }}</td>
                <td class="num">{{ formatKg(p.alloue_kg) }}</td>
                <td class="num"><b>{{ formatKg(p.reste_kg) }}</b></td>
                <td class="num">{{ p.nb_producteurs }}</td>
              </tr>
              <tr v-if="!rapport.par_produit.length">
                <td colspan="5" class="empty-cell">Aucune culture sur ce semestre.</td>
              </tr>
            </tbody>
          </table>
        </div>
      </section>

      <section class="report-card">
        <h2>Par producteur</h2>
        <div class="table-wrap">
          <table>
            <thead>
              <tr><th>Producteur</th><th>Produit</th><th class="num">Estimé</th><th class="num">Alloué</th><th class="num">Reste</th></tr>
            </thead>
            <tbody>
              <tr v-for="p in rapport.par_producteur" :key="p.exploitation_id + p.produit">
                <td>{{ p.operateur }} <span v-if="p.id_interne" class="tile-id">#{{ p.id_interne }}</span></td>
                <td class="cap">{{ p.produit }}</td>
                <td class="num">{{ formatKg(p.estime_kg) }}</td>
                <td class="num">{{ formatKg(p.alloue_kg) }}</td>
                <td class="num"><b>{{ formatKg(p.reste_kg) }}</b></td>
              </tr>
              <tr v-if="!rapport.par_producteur.length">
                <td colspan="5" class="empty-cell">Aucun producteur sur ce semestre.</td>
              </tr>
            </tbody>
          </table>
        </div>
      </section>
    </template>
  </div>
</template>

<style scoped>
.rapport-page {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  padding: 8px 8px 32px;
}
.page-header {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 8px 12px 18px;
  flex-wrap: wrap;
}
.back-btn {
  width: 36px;
  height: 36px;
  flex-shrink: 0;
  display: grid;
  place-items: center;
  border: 1px solid var(--border);
  border-radius: 50%;
  background: var(--bg);
  color: var(--text-h);
  cursor: pointer;
}
.back-btn:hover {
  border-color: var(--accent);
  color: var(--accent);
}
.page-header-text { flex: 1; min-width: 200px; }
.page-header h1 {
  font-size: 28px;
  font-weight: 700;
  color: var(--text-h);
  margin: 0 0 4px;
}
.page-header p { margin: 0; color: var(--text); font-size: 14px; }
.page-actions { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
.sem-select {
  padding: 8px 12px;
  border-radius: 10px;
  border: 1px solid var(--border);
  background: var(--bg);
  color: var(--text-h);
  font-size: 13px;
  font-weight: 600;
  outline: none;
}
.year-input { width: 90px; }
.action-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 8px 14px;
  border: 1px solid var(--border);
  border-radius: 999px;
  background: var(--bg);
  color: var(--text-h);
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
}
.action-btn:hover:not(:disabled) { border-color: var(--accent); color: var(--accent); }
.action-btn.primary { background: var(--accent); border-color: var(--accent); color: #fff; }
.action-btn.primary:hover { color: #fff; filter: brightness(1.08); }
.action-btn:disabled { opacity: 0.5; cursor: wait; }
.print-title { display: none; }
.sem-line {
  margin: 0 12px 12px;
  font-size: 13px;
  font-weight: 600;
  color: var(--text);
}
.kpi-row {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
  gap: 10px;
  padding: 0 12px 14px;
}
.kpi {
  padding: 14px 16px;
  border-radius: var(--shape-md);
  background: var(--surface-container-low);
  box-shadow: var(--elev-1);
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.kpi-label { font-size: 11px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.06em; color: var(--text); }
.kpi b { font-size: 20px; color: var(--text-h); }
.kpi.highlight { background: var(--accent-bg); }
.kpi.highlight b { color: var(--accent); }
.kpi.closed b { font-size: 15px; }
.report-card {
  margin: 0 12px 14px;
  padding: 16px 18px;
  border-radius: var(--shape-lg);
  background: var(--surface-container-low);
  box-shadow: var(--elev-1);
}
.report-card h2 { margin: 0 0 10px; font-size: 16px; color: var(--text-h); }
.table-wrap { overflow-x: auto; }
table { width: 100%; border-collapse: collapse; font-size: 13px; }
th, td { padding: 8px 10px; text-align: left; border-bottom: 1px solid var(--border); color: var(--text-h); }
th { font-size: 11px; text-transform: uppercase; letter-spacing: 0.05em; color: var(--text); }
.num { text-align: right; white-space: nowrap; }
.cap { text-transform: capitalize; }
.empty-cell { text-align: center; color: var(--text); padding: 18px; }
.tile-id {
  font-family: 'Courier New', monospace;
  font-size: 11px;
  font-weight: 700;
  color: var(--accent);
  background: var(--accent-bg);
  padding: 2px 7px;
  border-radius: 7px;
  margin-left: 6px;
}
.msg {
  margin: 0 12px 10px;
  padding: 10px 14px;
  border-radius: 12px;
  font-size: 13px;
  font-weight: 500;
}
.msg.ok {
  background: color-mix(in srgb, var(--success, #16a34a) 12%, transparent);
  color: var(--success, #16a34a);
}
.msg.error { background: rgba(220, 38, 38, 0.1); color: var(--error); }
.loading {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 30px;
  color: var(--text);
}
.spin { animation: spin 0.9s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }
@media (max-width: 560px) {
  .page-header h1 { font-size: 24px; }
}
</style>

<style>
@media print {
  .rapport-page .no-print { display: none !important; }
  .rapport-page .print-title { display: block !important; margin: 0 0 12px; }
  .rapport-page .print-title h1 { font-size: 20px; }
  .rapport-page { overflow: visible; }
}
</style>
