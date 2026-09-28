<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { ArrowLeft, CheckCircle2, FileUp, Info, Loader, MapPin, Printer, QrCode as QrCodeIcon } from '@lucide/vue'
import CalligraphText from '../components/CalligraphText.vue'
import QrCode from '../components/QrCode.vue'
import {
  genererFiche,
  getFiche,
  getToken,
  produitsDisponibles,
  type Fiche,
  type FicheItem,
  type ProduitStock
} from '../composables/useAuth'

const route = useRoute()
const router = useRouter()

const today = new Intl.DateTimeFormat('fr-FR', {
  day: 'numeric',
  month: 'long',
  year: 'numeric'
}).format(new Date())

const TITRE_DEFAUT = 'Fiche de traçabilité produit'

const produits = ref<ProduitStock[]>([])
const produit = ref('')
const tonnes = ref<number | null>(20)
const titre = ref(TITRE_DEFAUT)
const fiche = ref<Fiche | null>(null)
const loading = ref(false)
const generating = ref(false)
const reloading = ref(false)
const error = ref('')

const kgDemandes = computed(() => (tonnes.value && tonnes.value > 0 ? tonnes.value * 1000 : null))
const produitDispo = computed(() => produits.value.find((p) => p.produit === produit.value))
const statutLabel = computed(() => (fiche.value?.statut === 'complete' ? 'Commande complète' : 'Commande partielle'))

const formatNum = (n: number | null | undefined, digits = 0) =>
  n == null ? '—' : new Intl.NumberFormat('fr-FR', { maximumFractionDigits: digits }).format(n)

const formatKg = (kg: number) => {
  const tonnesVal = kg / 1000
  const arrondi = Math.round(tonnesVal * 100) / 100
  return `${formatNum(arrondi)} t (${formatNum(Math.round(kg))} kg)`
}

const nomProducteur = (item: FicheItem) => {
  const e = item.exploitation
  const nom = [e.operateur_prenom, e.operateur_nom].filter(Boolean).join(' ')
  return nom || [e.proprietaire_prenom, e.proprietaire_nom].filter(Boolean).join(' ') || 'Non renseigné'
}

const telephoneProducteur = (item: FicheItem) => {
  const e = item.exploitation
  return e.operateur_telephone || e.proprietaire_telephone || null
}

const localisationProducteur = (item: FicheItem) => {
  const e = item.exploitation
  return [e.village, e.district, e.region_inspection].filter(Boolean).join(' · ') || '—'
}

const coordonnees = (u: { latitude: number | null; longitude: number | null }) =>
  u.latitude != null && u.longitude != null
    ? `${u.latitude.toFixed(4)}, ${u.longitude.toFixed(4)}`
    : null

const parcelleTexte = (item: FicheItem) =>
  item.unites
    .map(
      (u) =>
        `${u.identifiant_unite ?? 'unité'}` +
        `${u.superficie_ha != null ? ` · ${formatNum(u.superficie_ha, 2)} ha` : ''}`
    )
    .join('  |  ') || '—'

const qrPayload = (item: FicheItem) => {
  const f = fiche.value
  if (!f) return ''
  const e = item.exploitation
  return [
    'AGROBOX · FICHE DE TRACABILITÉ',
    `FICHE: ${f.reference}`,
    `PRODUIT: ${f.produit}`,
    `QUANTITÉ ALLOUÉE: ${formatNum(Math.round(item.quantite_kg))} kg`,
    `DATE: ${f.created_at.slice(0, 10)}`,
    `ID PRODUCTEUR: ${e.id_interne ?? e.id}`,
    `NOM: ${nomProducteur(item)}`,
    `TÉLÉPHONE: ${telephoneProducteur(item) ?? '—'}`,
    `LOCALISATION: ${localisationProducteur(item)}`,
    `SUPERFICIE: ${e.superficie_total != null ? `${formatNum(e.superficie_total, 2)} ha` : '—'}`,
    `PARCELS: ${item.unites
      .map(
        (u) =>
          `${u.identifiant_unite ?? 'unité'}` +
          `${u.superficie_ha != null ? ` · ${formatNum(u.superficie_ha, 2)} ha` : ''}` +
          `${coordonnees(u) ? ` · ${coordonnees(u)}` : ''}`
      )
      .join(' | ') || '—'}`
  ].join('\n')
}

const load = async () => {
  const token = getToken()
  if (!token) return
  loading.value = true
  error.value = ''
  try {
    produits.value = await produitsDisponibles(token)
    if (!produit.value && produits.value.length) {
      const first = [...produits.value].sort((a, b) => b.quantite_disponible_kg - a.quantite_disponible_kg)[0]
      produit.value = first.produit
    }
  } catch (e) {
    error.value = typeof e === 'string' ? e : 'Chargement impossible'
  } finally {
    loading.value = false
  }
}

const generer = async () => {
  const token = getToken()
  if (!token || kgDemandes.value == null || !produit.value) return
  generating.value = true
  error.value = ''
  try {
    fiche.value = await genererFiche(token, produit.value, kgDemandes.value, titre.value.trim() || undefined)
    router.replace({ query: {} })
    await load()
  } catch (e) {
    error.value = typeof e === 'string' ? e : 'Génération impossible'
  } finally {
    generating.value = false
  }
}

const voirFiche = async (id: number) => {
  const token = getToken()
  if (!token) return
  reloading.value = true
  error.value = ''
  try {
    fiche.value = await getFiche(token, id)
  } catch (e) {
    error.value = typeof e === 'string' ? e : 'Lecture impossible'
  } finally {
    reloading.value = false
  }
}

const retour = () => {
  fiche.value = null
  router.replace({ query: {} })
  router.push({ name: 'Archive' })
}

const imprimer = () => window.print()

watch(fiche, () => {
  if (fiche.value) window.scrollTo({ top: 0 })
})

onMounted(() => {
  load()
  const id = Number(route.query.id)
  if (Number.isInteger(id) && id > 0) {
    voirFiche(id)
  }
})
</script>

<template>
  <div class="sheet-page">
    <header class="page-header">
      <div class="page-header-text">
        <h1><CalligraphText text="Fiche" :initial="true" /></h1>
        <p>Fiche de traçabilité produit pour une commande client</p>
      </div>
      <div class="header-actions">
        <span class="header-chip">{{ today }}</span>
        <button v-if="fiche" class="print-btn" title="Retour à l'historique" @click="retour">
          <ArrowLeft :size="15" />
          <span>Retour</span>
        </button>
        <button v-if="fiche" class="print-btn" title="Imprimer la fiche" @click="imprimer">
          <Printer :size="15" />
          <span>Imprimer</span>
        </button>
      </div>
    </header>

    <div class="sheet-scroll">
      <p v-if="error" class="msg error">{{ error }}</p>

      <!-- ---------- Ordre client ---------- -->
      <section class="order-card">
        <div class="order-title">
          <span class="order-icon"><FileUp :size="17" /></span>
          <div>
            <h2>Commande client</h2>
            <p>Saisissez le produit et la quantité demandée ; le logiciel sélectionne les producteurs couvrant la demande.</p>
          </div>
        </div>

        <div class="order-fields">
          <label class="field">
            <span class="field-label">Produit</span>
            <select v-model="produit" class="field-input" :disabled="loading || !produits.length">
              <option v-if="!produits.length" value="">Aucun produit importé</option>
              <option v-for="p in produits" :key="p.produit" :value="p.produit">
                {{ p.produit }} — {{ formatKg(p.quantite_disponible_kg) }} dispo
              </option>
            </select>
          </label>

          <label class="field">
            <span class="field-label">Quantité (tonnes)</span>
            <input
              v-model.number="tonnes"
              type="number"
              min="0"
              step="0.5"
              class="field-input"
              placeholder="ex. 20"
            />
          </label>

          <label class="field field-titre">
            <span class="field-label">Titre de la fiche</span>
            <input
              v-model="titre"
              type="text"
              maxlength="120"
              class="field-input"
              :placeholder="TITRE_DEFAUT"
            />
          </label>

          <button v-if="produits.length" class="generate-btn" :disabled="generating || !produit || kgDemandes == null || kgDemandes <= 0" @click="generer">
            <Loader v-if="generating" class="spin" :size="16" />
            <template v-else>Générer la fiche</template>
          </button>
        </div>

        <div v-if="produitDispo" class="order-info">
          <Info :size="14" />
          <span>
            Disponible : <b>{{ formatKg(produitDispo.quantite_disponible_kg) }}</b>
            chez {{ produitDispo.nb_exploitations }} producteur{{ produitDispo.nb_exploitations > 1 ? 's' : '' }}
            (estimé {{ formatKg(produitDispo.quantite_estimee_kg) }} − alloué {{ formatKg(produitDispo.quantite_allouee_kg) }})
          </span>
        </div>
      </section>

      <!-- ---------- Document de traçabilité ---------- -->
      <section v-if="fiche" class="fiche-doc">
        <div class="doc-head">
          <div class="doc-band">
            <span class="doc-eyebrow">AgroBox · Traçabilité produit</span>
            <h2 class="doc-title">{{ fiche.titre }}</h2>
          </div>
          <div class="doc-meta">
            <p><span>Référence</span><b>{{ fiche.reference }}</b></p>
            <p><span>Date</span><b>{{ fiche.created_at.slice(0, 10) }}</b></p>
          </div>
        </div>

        <div class="doc-order">
          <div class="doc-field">
            <span>Produit commandé</span>
            <b>{{ fiche.produit }}</b>
          </div>
          <div class="doc-field">
            <span>Quantité demandée</span>
            <b>{{ formatKg(fiche.quantite_demandee_kg) }}</b>
          </div>
          <div class="doc-field">
            <span>Quantité affectée</span>
            <b>{{ formatKg(fiche.quantite_attribuee_kg) }}</b>
          </div>
          <div class="doc-field">
            <span>Statut</span>
            <b :class="['doc-status', fiche.statut]">
              <CheckCircle2 :size="13" />
              {{ statutLabel }}
            </b>
          </div>
        </div>

        <p v-if="fiche.statut === 'partielle'" class="doc-note">
          La demande dépasse l'estimation disponible : la production restante est affectée. Relancez une commande pour le reliquat.
        </p>

        <h2 class="doc-section">
          Producteurs impliqués
          <span class="doc-count">{{ fiche.items.length }}</span>
        </h2>

        <div v-for="(item, i) in fiche.items" :key="item.culture_id" class="producer-row">
          <div class="producer-qr">
            <QrCode :value="qrPayload(item)" :size="56" />
          </div>

          <div class="producer-main">
            <div class="producer-head">
              <span class="producer-num">#{{ i + 1 }}</span>
              <span class="producer-id">ID : {{ item.exploitation.id_interne || item.exploitation.id }}</span>
              <span class="producer-qty">{{ formatKg(item.quantite_kg) }}</span>
            </div>
            <p class="producer-name">{{ nomProducteur(item) }}</p>
            <div class="producer-lines">
              <p class="producer-line">
                <MapPin :size="12" />{{ localisationProducteur(item) }}<template v-if="telephoneProducteur(item)"> · Tél : {{ telephoneProducteur(item) }}</template>
              </p>
              <p class="producer-line"><QrCodeIcon :size="12" />{{ parcelleTexte(item) }}</p>
            </div>
          </div>
        </div>

        <p class="doc-footer">
          Fiche générée par AgroBox — chaque quantité affectée est verrouillée pour garantir l'unicité de la production.
        </p>
      </section>
    </div>
  </div>
</template>

<style scoped>
.sheet-page {
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

.header-actions {
  display: flex;
  align-items: center;
  gap: 10px;
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

.print-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 8px 14px;
  border-radius: 999px;
  border: 1px solid var(--border);
  background: var(--bg);
  color: var(--text-h);
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: border-color 0.15s ease, color 0.15s ease;
}

.print-btn:hover {
  border-color: var(--accent);
  color: var(--accent);
}

.sheet-scroll {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.msg {
  margin: 0;
  padding: 10px 14px;
  border-radius: 12px;
  font-size: 13.5px;
}

.msg.error {
  color: #dc2626;
  background: rgba(220, 38, 38, 0.08);
  border: 1px solid rgba(220, 38, 38, 0.25);
}

/* ---------- Carte commande ---------- */
.order-card,
.fiche-doc {
  background: linear-gradient(180deg, var(--bg) 0%, rgba(255, 255, 255, 0.5) 100%);
  border: 1px solid var(--border);
  border-radius: 20px;
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.06);
  padding: 18px;
}

.order-title {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  margin-bottom: 14px;
}

.order-title h2 {
  margin: 0;
  font-size: 16px;
  font-weight: 700;
  color: var(--text-h);
}

.order-title p {
  margin: 2px 0 0;
  font-size: 13px;
  color: var(--text);
}

.order-icon {
  width: 36px;
  height: 36px;
  flex-shrink: 0;
  display: grid;
  place-items: center;
  border-radius: 12px;
  background: var(--accent-bg);
  color: var(--accent);
}

.order-fields {
  display: flex;
  align-items: flex-end;
  gap: 12px;
  flex-wrap: wrap;
}

.field {
  display: flex;
  flex-direction: column;
  gap: 6px;
  min-width: 0;
}

.field-label {
  font-size: 12px;
  font-weight: 600;
  color: var(--text);
}

.field-input {
  padding: 10px 12px;
  border-radius: 12px;
  border: 1px solid var(--border);
  background: var(--bg);
  color: var(--text-h);
  font-size: 14px;
  font-weight: 600;
  outline: none;
  min-width: 220px;
  transition: border-color 0.15s ease;
}

.field-input:focus {
  border-color: var(--accent);
}

.field-titre {
  flex: 1 1 260px;
}

.field-titre .field-input {
  width: 100%;
  min-width: 0;
  box-sizing: border-box;
}

.generate-btn {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 10px 18px;
  border-radius: 12px;
  border: none;
  background: linear-gradient(135deg, var(--accent) 0%, color-mix(in srgb, var(--accent) 70%, #000 30%) 100%);
  color: #fff;
  font-size: 14px;
  font-weight: 700;
  cursor: pointer;
  transition: transform 0.15s ease, box-shadow 0.15s ease, filter 0.15s ease;
}

.generate-btn:hover:not(:disabled) {
  transform: translateY(-1px);
  filter: brightness(1.06);
}

.generate-btn:disabled {
  opacity: 0.55;
  cursor: not-allowed;
}

.order-info {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 12px;
  font-size: 13px;
  color: var(--text);
}

.order-info svg {
  color: var(--accent);
  flex-shrink: 0;
}

.spin {
  animation: spin 0.9s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

/* ---------- Document ---------- */
.fiche-doc {
  animation: cardIn 0.4s ease both;
  overflow: hidden;
}

@keyframes cardIn {
  from {
    opacity: 0;
    transform: translateY(10px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.doc-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  flex-wrap: wrap;
  padding: 22px 24px;
  background: linear-gradient(
    135deg,
    var(--accent) 0%,
    color-mix(in srgb, var(--accent) 60%, #064e3b 40%) 100%
  );
  color: #fff;
  -webkit-print-color-adjust: exact;
  print-color-adjust: exact;
}

.doc-band {
  display: flex;
  flex-direction: column;
  gap: 6px;
  min-width: 0;
}

.doc-eyebrow {
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.16em;
  text-transform: uppercase;
  color: rgba(255, 255, 255, 0.85);
}

.doc-title {
  margin: 0;
  font-size: clamp(24px, 3.4vw, 32px);
  font-weight: 800;
  color: #fff;
  line-height: 1.15;
  letter-spacing: -0.01em;
}

.doc-meta {
  display: flex;
  gap: 10px;
  flex-wrap: wrap;
  justify-content: flex-end;
}

.doc-meta p {
  margin: 0;
  display: flex;
  flex-direction: column;
  gap: 1px;
  padding: 8px 12px;
  background: rgba(255, 255, 255, 0.14);
  border: 1px solid rgba(255, 255, 255, 0.28);
  border-radius: 12px;
  font-size: 12.5px;
  font-weight: 700;
}

.doc-meta span {
  font-size: 10px;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: rgba(255, 255, 255, 0.75);
  font-weight: 600;
}

.doc-meta b {
  color: #fff;
}

.doc-order {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
  gap: 8px;
  padding: 12px 0;
}

.doc-field {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 10px 12px;
  background: linear-gradient(180deg, var(--bg) 0%, rgba(255, 255, 255, 0.55) 100%);
  border: 1px solid var(--border);
  border-left: 3px solid var(--accent);
  border-radius: 12px;
}

.doc-field span {
  font-size: 10.5px;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text);
  font-weight: 600;
}

.doc-field b {
  font-size: 14px;
  color: var(--text-h);
  text-transform: capitalize;
}

.doc-status {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  font-size: 12.5px;
  width: fit-content;
  padding: 3px 10px;
  border-radius: 999px;
  -webkit-print-color-adjust: exact;
  print-color-adjust: exact;
}

.doc-status.complete {
  color: #16a34a;
  background: rgba(34, 197, 94, 0.14);
  border: 1px solid rgba(34, 197, 94, 0.35);
}

.doc-status.partielle {
  color: #d97706;
  background: rgba(245, 158, 11, 0.16);
  border: 1px solid rgba(245, 158, 11, 0.4);
}

.doc-note {
  margin: 0 0 6px;
  font-size: 12.5px;
  color: #d97706;
  background: rgba(245, 158, 11, 0.08);
  border: 1px solid rgba(245, 158, 11, 0.25);
  border-radius: 10px;
  padding: 9px 12px;
}

.doc-section {
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 12px 0 8px;
  font-size: 15px;
  font-weight: 700;
  color: var(--text-h);
}

.doc-count {
  font-size: 12px;
  font-weight: 700;
  color: var(--accent);
  background: var(--accent-bg);
  padding: 2px 9px;
  border-radius: 999px;
}

.producer-row {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 8px 12px;
  background: linear-gradient(180deg, var(--bg) 0%, rgba(255, 255, 255, 0.6) 100%);
  border: 1px solid var(--border);
  border-left: 4px solid var(--accent);
  border-radius: 14px;
  margin-bottom: 6px;
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.06);
}

.producer-qr {
  flex-shrink: 0;
  padding: 6px;
  background: #fff;
  border-radius: 10px;
  box-shadow: inset 0 0 0 1px var(--border);
}

.producer-main {
  flex: 1;
  min-width: 0;
}

.producer-head {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

.producer-num {
  font-family: 'Courier New', monospace;
  font-size: 10.5px;
  font-weight: 700;
  color: #fff;
  background: linear-gradient(135deg, var(--accent) 0%, color-mix(in srgb, var(--accent) 70%, #000 30%) 100%);
  padding: 1px 8px;
  border-radius: 999px;
}

.producer-id {
  font-family: 'Courier New', monospace;
  font-size: 11px;
  font-weight: 700;
  color: var(--text-h);
}

.producer-qty {
  margin-left: auto;
  font-size: 12.5px;
  font-weight: 700;
  color: var(--accent);
}

.producer-name {
  margin: 3px 0 2px;
  font-size: 14px;
  font-weight: 700;
  color: var(--text-h);
}

.producer-lines {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.producer-line {
  display: flex;
  align-items: center;
  gap: 6px;
  margin: 0;
  font-size: 12px;
  color: var(--text);
}

.producer-line svg {
  flex-shrink: 0;
  color: var(--accent);
}

.doc-footer {
  margin: 10px 0 0;
  font-size: 12px;
  color: var(--text);
  border-top: 1px dashed var(--border);
  padding-top: 10px;
}

@media (max-width: 600px) {
  .page-header {
    flex-direction: column;
    align-items: flex-start;
  }

  .page-header-text {
    min-width: 0;
  }

  .producer-row {
    flex-direction: column;
    align-items: center;
  }
}

@media (max-width: 560px) {
  .order-fields {
    align-items: stretch;
  }

  .field {
    flex: 1 1 100%;
  }

  .field-input {
    width: 100%;
    min-width: 0;
  }

  .producer-qty {
    margin-left: 0;
  }
}

@media print {
  .page-header,
  .order-card,
  .msg {
    display: none !important;
  }

  .sheet-page {
    padding: 0;
    overflow: visible;
  }

  .sheet-scroll {
    overflow: visible;
  }

  .fiche-doc {
    box-shadow: none;
    border: none;
    padding: 0;
  }

  .producer-row {
    break-inside: avoid;
    box-shadow: none;
  }
}
</style>

<style>
@media print {
  .app-shell > .sidebar {
    display: none !important;
  }

  .fiche-doc * {
    -webkit-print-color-adjust: exact !important;
    print-color-adjust: exact !important;
  }

  .shell-main {
    overflow: visible !important;
  }
}
</style>