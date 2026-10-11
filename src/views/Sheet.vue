<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { ArrowLeft, CheckCircle2, FileUp, Info, Loader, MapPin, Printer, QrCode as QrCodeIcon } from '@lucide/vue'
import QrCode from '../components/QrCode.vue'
import watermarkUrl from '../assets/watermark.jpg'
import {
  genererFiche,
  genererFicheManuelle,
  getFiche,
  getToken,
  setFicheNbSacs,
  producteursPourFiche,
  produitsDisponibles,
  type Exploitation,
  type Fiche,
  type FicheItem,
  type ProducteurFiche,
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

const mode = ref<'auto' | 'manuel'>('auto')
const produits = ref<ProduitStock[]>([])
const produit = ref('')
const kg = ref<number | null>(20000)
const titre = ref(TITRE_DEFAUT)
const producteurs = ref<ProducteurFiche[]>([])
const quantites = ref<Record<number, number | null>>({})
const fiche = ref<Fiche | null>(null)
const loading = ref(false)
const generating = ref(false)
const reloading = ref(false)
const chargeProducteurs = ref(false)
const error = ref('')

const kgDemandes = computed(() => (kg.value && kg.value > 0 ? kg.value : null))
const produitDispo = computed(() => produits.value.find((p) => p.produit === produit.value))
const statutLabel = computed(() => (fiche.value?.statut === 'complete' ? 'Commande complète' : 'Commande partielle'))

const totalManuel = computed(() =>
  producteurs.value.reduce((acc, p) => acc + (quantites.value[p.culture_id] ?? 0), 0)
)
const hasManuel = computed(() => totalManuel.value > 0)

const generateDisabled = computed(() => {
  if (generating.value || !produit.value || !produits.value.length) return true
  if (mode.value === 'auto') return kgDemandes.value == null || kgDemandes.value <= 0
  return chargeProducteurs.value || !hasManuel.value
})

const formatNum = (n: number | null | undefined, digits = 0) =>
  n == null ? '—' : new Intl.NumberFormat('fr-FR', { maximumFractionDigits: digits }).format(n)

const formatKg = (kg: number) => {
  if (kg < 1000) return `${formatNum(Math.round(kg))} kg`
  const tonnesVal = kg / 1000
  const arrondi = Math.round(tonnesVal * 100) / 100
  return `${formatNum(arrondi)} t (${formatNum(Math.round(kg))} kg)`
}

const nbSacsSaisie = ref<number | null>(0)
const sauvegardeSacs = ref(false)
const sacsFeedback = ref('')
const sacsFeedbackErreur = ref(false)

const nomProducteur = (item: FicheItem) => nomExploitation(item.exploitation)

const nomExploitation = (e: Exploitation) => {
  const nom = [e.operateur_prenom, e.operateur_nom].filter(Boolean).join(' ')
  return nom || [e.proprietaire_prenom, e.proprietaire_nom].filter(Boolean).join(' ') || 'Non renseigné'
}

const telephoneProducteur = (item: FicheItem) => {
  const e = item.exploitation
  return e.operateur_telephone || e.proprietaire_telephone || null
}

const localisationProducteur = (item: FicheItem) => localisationExploitation(item.exploitation)

const localisationExploitation = (e: Exploitation) =>
  [e.village, e.district, e.region_inspection].filter(Boolean).join(' · ') || '—'

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
    'Vobox · FICHE DE TRACABILITÉ',
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
      const first = [...produits.value].sort((a, b) => b.quantite_livrable_kg - a.quantite_livrable_kg)[0]
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

const loadProducteurs = async () => {
  const token = getToken()
  if (!token || !produit.value) {
    producteurs.value = []
    quantites.value = {}
    return
  }
  chargeProducteurs.value = true
  try {
    producteurs.value = await producteursPourFiche(token, produit.value)
    quantites.value = {}
    for (const p of producteurs.value) quantites.value[p.culture_id] = null
  } catch (e) {
    error.value = typeof e === 'string' ? e : 'Chargement des producteurs impossible'
  } finally {
    chargeProducteurs.value = false
  }
}

const genererManuel = async () => {
  const token = getToken()
  if (!token || !produit.value || !hasManuel.value) return
  generating.value = true
  error.value = ''
  try {
    const allocations: Array<[number, number]> = producteurs.value
      .map((p) => [p.culture_id, quantites.value[p.culture_id]])
      .filter((pair): pair is [number, number] => typeof pair[1] === 'number' && pair[1] > 0)
    fiche.value = await genererFicheManuelle(token, produit.value, allocations, titre.value.trim() || undefined)
    router.replace({ query: {} })
    await load()
    await loadProducteurs()
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

const enregistrerNbSacs = async () => {
  const token = getToken()
  const ficheCourante = fiche.value
  const nbSacs = nbSacsSaisie.value
  sacsFeedback.value = ''
  if (!token || !ficheCourante) {
    sacsFeedback.value = 'Impossible d’enregistrer le nombre de sacs.'
    sacsFeedbackErreur.value = true
    return
  }
  if (nbSacs == null || !Number.isInteger(nbSacs) || nbSacs < 0) {
    sacsFeedback.value = 'Saisissez un nombre entier positif ou nul.'
    sacsFeedbackErreur.value = true
    return
  }

  sauvegardeSacs.value = true
  try {
    const ficheMiseAJour = await setFicheNbSacs(token, ficheCourante.id, nbSacs)
    if (fiche.value?.id === ficheMiseAJour.id) {
      fiche.value.nb_sacs = ficheMiseAJour.nb_sacs
    }
    nbSacsSaisie.value = ficheMiseAJour.nb_sacs
    sacsFeedback.value = 'Nombre de sacs enregistré.'
    sacsFeedbackErreur.value = false
  } catch (e) {
    sacsFeedback.value = typeof e === 'string' ? e : 'Enregistrement impossible.'
    sacsFeedbackErreur.value = true
  } finally {
    sauvegardeSacs.value = false
  }
}

const retour = () => {
  fiche.value = null
  router.replace({ query: {} })
  router.push({ name: 'Archive' })
}

const imprimer = () => window.print()

watch(fiche, () => {
  if (fiche.value) {
    nbSacsSaisie.value = fiche.value.nb_sacs
    sacsFeedback.value = ''
    window.scrollTo({ top: 0 })
  }
})

watch(
  () => [mode.value, produit.value] as const,
  () => {
    if (mode.value === 'manuel') loadProducteurs()
  }
)

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
        <h1>Fiche</h1>
        <p>Fiche de traçabilité produit pour une commande client</p>
      </div>
      <div class="header-actions">
        <span class="header-chip">{{ today }}</span>
        <button v-if="fiche" class="print-btn" title="Retour à l'historique" @click="retour">
          <ArrowLeft :size="15" />
          <span>Retour</span>
        </button>
        <button v-if="fiche" class="print-btn print-btn-primary" title="Imprimer la fiche" @click="imprimer">
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

        <div class="order-modes">
          <div class="mode-toggle">
            <button type="button" :class="['mode-btn', { active: mode === 'auto' }]" @click="mode = 'auto'">
              Répartition auto
            </button>
            <button type="button" :class="['mode-btn', { active: mode === 'manuel' }]" @click="mode = 'manuel'">
              Édition manuelle
            </button>
          </div>
          <p v-if="mode === 'manuel'" class="mode-hint">
            Choisissez les producteurs et saisissez les quantités ; la fiche est générée avec exactement cette répartition.
          </p>
        </div>

        <div class="order-fields">
          <label class="field">
            <span class="field-label">Produit</span>
            <select v-model="produit" class="field-input" :disabled="loading || !produits.length">
              <option v-if="!produits.length" value="">Aucun produit importé</option>
              <option v-for="p in produits" :key="p.produit" :value="p.produit">
                {{ p.produit }} — {{ formatKg(p.quantite_livrable_kg) }} livrables
              </option>
            </select>
          </label>

          <label v-if="mode === 'auto'" class="field">
            <span class="field-label">Quantité (kg)</span>
            <input
              v-model.number="kg"
              type="number"
              min="0"
              step="1"
              class="field-input"
              placeholder="ex. 20000"
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

          <button class="generate-btn" :disabled="generateDisabled" @click="mode === 'auto' ? generer() : genererManuel()">
            <Loader v-if="generating" class="spin" :size="16" />
            <template v-else>Générer la fiche</template>
          </button>
        </div>

        <div v-if="mode === 'manuel'" class="manual-block">
          <div class="manual-head">
            <span>Producteurs disponibles pour <b>{{ produit || '—' }}</b></span>
            <span class="manual-total">Total : {{ formatKg(totalManuel) }}</span>
          </div>
          <p v-if="chargeProducteurs" class="manual-empty">Chargement des producteurs…</p>
          <p v-else-if="!producteurs.length" class="manual-empty">
            Aucun producteur avec des quantités livrables pour ce produit.
          </p>
          <div v-else class="manual-list">
            <div v-for="p in producteurs" :key="p.culture_id" class="producer-pick">
              <div class="pp-main">
                <span class="pp-name">{{ nomExploitation(p.exploitation) }}</span>
                <span class="pp-id">
                  ID : {{ p.exploitation.id_interne || p.exploitation.id }} · {{ localisationExploitation(p.exploitation) }}
                </span>
              </div>
              <span v-if="p.participe_semaine" class="pp-warn" title="Ce producteur a déjà été affecté à une commande cette semaine.">
                <CheckCircle2 :size="11" />
                Déjà affecté cette semaine
              </span>
              <span class="pp-dispo">Livrable : {{ formatKg(p.quantite_livrable_kg) }}</span>
              <input
                v-model.number="quantites[p.culture_id]"
                type="number"
                min="0"
                step="1"
                class="field-input pp-input"
                placeholder="kg"
              />
            </div>
          </div>
        </div>

        <div v-if="produitDispo" class="order-info">
          <Info :size="14" />
          <span>
            Livrable cette saison : <b>{{ formatKg(produitDispo.quantite_livrable_kg) }}</b>
            chez {{ produitDispo.nb_exploitations }} producteur{{ produitDispo.nb_exploitations > 1 ? 's' : '' }}
            (estimation {{ formatKg(produitDispo.quantite_estimee_kg) }} − alloué {{ formatKg(produitDispo.quantite_allouee_kg) }})
          </span>
        </div>
      </section>

      <!-- ---------- Document de traçabilité ---------- -->
      <section v-if="fiche" class="fiche-doc">
        <img class="doc-watermark" :src="watermarkUrl" alt="" aria-hidden="true" />
        <div class="doc-head">
          <div class="doc-band">
            <span class="doc-eyebrow">Vobox · Traçabilité produit</span>
            <h2 class="doc-title">{{ fiche.titre }}</h2>
          </div>
          <div class="doc-meta">
            <p><span>Référence</span><b>{{ fiche.reference }}</b></p>
            <p><span>Date</span><b>{{ fiche.created_at.slice(0, 10) }}</b></p>
          </div>
        </div>

        <div class="doc-order">
          <div class="doc-field">
            <span>Type de produit</span>
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
          <div class="doc-field doc-field-sacs">
            <span>Nombre de sacs</span>
            <div class="sacs-hero">
              <div class="sacs-editor">
                <input
                  v-model.number="nbSacsSaisie"
                  type="number"
                  min="0"
                  step="1"
                  class="sacs-input"
                  aria-label="Nombre de sacs"
                />
                <span>sac{{ nbSacsSaisie === 1 ? '' : 's' }}</span>
                <button type="button" class="sacs-save-btn" :disabled="sauvegardeSacs || nbSacsSaisie == null" @click="enregistrerNbSacs">
                  {{ sauvegardeSacs ? 'Enregistrement…' : 'Enregistrer' }}
                </button>
              </div>
              <b class="sacs-print-count">
                {{ formatNum(fiche.nb_sacs) }} <small>sac{{ fiche.nb_sacs === 1 ? '' : 's' }}</small>
              </b>
              <span v-if="sacsFeedback" :class="['sacs-feedback', { error: sacsFeedbackErreur }]" role="status">
                {{ sacsFeedback }}
              </span>
            </div>
          </div>
          <div class="doc-field doc-field-statut">
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
            <QrCode :value="qrPayload(item)" :size="93" />
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
          Fiche générée par Vobox — chaque quantité affectée est verrouillée pour garantir l'unicité de la production.
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
  font-size: 28px;
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

.print-btn-primary {
  background: var(--accent);
  border-color: var(--accent);
  color: var(--on-primary);
  box-shadow: 0 8px 16px -8px var(--accent);
}

.print-btn-primary:hover {
  border-color: var(--accent);
  color: var(--on-primary);
  filter: brightness(1.08);
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
  color: var(--error);
  background: rgba(220, 38, 38, 0.08);
  border: 1px solid rgba(220, 38, 38, 0.25);
}

/* ---------- Carte commande ---------- */
.order-card,
.fiche-doc {
  background: var(--surface-container-low);
  border: none;
  border-radius: var(--shape-lg);
  box-shadow: var(--elev-1);
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
  height: 48px;
  padding: 0 24px;
  border-radius: var(--shape-full);
  border: none;
  background: var(--primary);
  color: var(--on-primary);
  font-size: 14px;
  font-weight: 600;
  letter-spacing: 0.1px;
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

.order-modes {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 10px;
  margin: 14px 0 2px;
}

.mode-toggle {
  display: inline-flex;
  gap: 3px;
  padding: 3px;
  background: var(--bg);
  border: 1px solid var(--border);
  border-radius: 999px;
}

.mode-btn {
  padding: 6px 14px;
  border: none;
  border-radius: 999px;
  background: transparent;
  color: var(--text);
  font-size: 12.5px;
  font-weight: 600;
  cursor: pointer;
  transition: background 0.15s ease, color 0.15s ease;
}

.mode-btn.active {
  background: var(--accent);
  color: #fff;
}

.mode-hint {
  margin: 0;
  font-size: 12px;
  color: var(--text);
}

.manual-block {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-top: 14px;
}

.manual-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  font-size: 12.5px;
  color: var(--text);
  padding: 0 2px;
}

.manual-head b {
  color: var(--text-h);
  text-transform: capitalize;
}

.manual-total {
  font-weight: 700;
  color: var(--accent);
}

.manual-empty {
  margin: 4px 0 0;
  font-size: 13px;
  color: var(--text);
}

.manual-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
  max-height: 320px;
  overflow-y: auto;
  padding: 2px;
}

.producer-pick {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 12px;
  background: var(--bg);
  border: 1px solid var(--border);
  border-radius: 10px;
}

.pp-main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.pp-name {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-h);
}

.pp-id {
  font-size: 11px;
  color: var(--text);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.pp-dispo {
  font-size: 11px;
  font-weight: 600;
  color: var(--text);
  white-space: nowrap;
}

.pp-warn {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-size: 10.5px;
  font-weight: 700;
  color: #d97706;
  background: rgba(245, 158, 11, 0.14);
  padding: 2px 8px;
  border-radius: 999px;
  white-space: nowrap;
}

.pp-input {
  width: 110px;
  min-width: 90px;
  padding: 6px 10px;
  font-size: 13px;
  text-align: right;
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
  position: relative;
  animation: cardIn 0.4s ease both;
  overflow: hidden;
  width: 100%;
  max-width: 880px;
  margin: 0 auto;
}

.doc-watermark {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  object-fit: contain;
  opacity: 0.1;
  pointer-events: none;
  z-index: 0;
  -webkit-print-color-adjust: exact;
  print-color-adjust: exact;
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
  padding: 14px 18px;
  background: var(--accent);
  color: #fff;
  -webkit-print-color-adjust: exact;
  print-color-adjust: exact;
}

.doc-band {
  display: flex;
  flex-direction: column;
  gap: 3px;
  min-width: 0;
}

.doc-eyebrow {
  font-size: 10px;
  font-weight: 600;
  letter-spacing: 0.16em;
  text-transform: uppercase;
  color: rgba(255, 255, 255, 0.92);
}

.doc-title {
  margin: 0;
  font-size: clamp(18px, 2.6vw, 24px);
  font-weight: 700;
  color: #fff;
  line-height: 1.2;
  letter-spacing: -0.01em;
}

.doc-meta {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
  justify-content: flex-end;
}

.doc-meta p {
  margin: 0;
  display: flex;
  flex-direction: column;
  gap: 1px;
  padding: 5px 10px;
  background: rgba(255, 255, 255, 0.12);
  border-radius: 8px;
  font-size: 12px;
  font-weight: 600;
}

.doc-meta span {
  font-size: 9.5px;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: rgba(255, 255, 255, 0.9);
  font-weight: 500;
}

.doc-meta b {
  color: #fff;
}

.doc-order {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 6px;
  padding: 12px 0 4px;
}

.doc-field {
  display: flex;
  flex-direction: column;
  gap: 2px;
  justify-content: center;
  padding: 8px 10px;
  background: var(--bg);
  border: 1px solid var(--border);
  border-radius: 8px;
}

.doc-field-statut {
  grid-column: 1 / -1;
}

.doc-field span {
  font-size: 9.5px;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--text);
  font-weight: 500;
}

.doc-field b {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-h);
  text-transform: capitalize;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.doc-field-sacs .sacs-hero {
  display: flex;
  align-items: baseline;
  gap: 8px;
  flex-wrap: wrap;
}

.doc-field-sacs .sacs-input {
  width: 86px;
  min-width: 0;
  padding: 4px 7px;
  border: 1px solid var(--border);
  border-radius: 7px;
  background: var(--surface);
  color: var(--text-h);
  font-size: 26px;
  font-weight: 800;
  line-height: 1.1;
}

.doc-field-sacs .sacs-editor {
  display: flex;
  align-items: center;
  gap: 6px;
  color: var(--text-h);
  font-size: 13px;
}

.doc-field-sacs .sacs-save-btn {
  padding: 5px 9px;
  border: 1px solid var(--accent-border);
  border-radius: 7px;
  background: var(--accent-bg);
  color: var(--accent);
  font-size: 11px;
  font-weight: 700;
  cursor: pointer;
}

.doc-field-sacs .sacs-save-btn:disabled {
  opacity: 0.6;
  cursor: wait;
}

.doc-field-sacs .sacs-feedback {
  font-size: 11px;
  color: var(--success, #16a34a);
}

.doc-field-sacs .sacs-feedback.error {
  color: var(--danger, #dc2626);
}

.doc-field-sacs .sacs-print-count {
  display: none;
  color: var(--accent);
  font-size: 26px;
  font-weight: 800;
  line-height: 1.1;
}

.doc-field-sacs .sacs-print-count small {
  font-size: 13px;
  font-weight: 600;
  text-transform: none;
}

.doc-field-sacs .sacs-hero > span {
  font-size: 13px;
}

.doc-status {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  font-size: 12px;
  width: fit-content;
  padding: 2px 10px;
  border-radius: 999px;
  -webkit-print-color-adjust: exact;
  print-color-adjust: exact;
}

.doc-status.complete {
  color: var(--success, #16a34a);
  background: color-mix(in srgb, var(--success, #16a34a) 12%, transparent);
}

.doc-status.partielle {
  color: var(--warning, #b45309);
  background: color-mix(in srgb, var(--warning, #b45309) 14%, transparent);
}

.doc-note {
  margin: 8px 0 2px;
  font-size: 12px;
  color: var(--warning, #b45309);
  background: color-mix(in srgb, var(--warning, #b45309) 8%, transparent);
  border-radius: 8px;
  padding: 8px 12px;
}

.doc-section {
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 14px 0 8px;
  font-size: 14px;
  font-weight: 600;
  color: var(--text-h);
}

.doc-count {
  font-size: 11px;
  font-weight: 600;
  color: var(--accent);
  background: var(--accent-bg);
  padding: 1px 8px;
  border-radius: 999px;
}

.producer-row {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 8px 12px;
  background: var(--bg);
  border: 1px solid var(--border);
  border-radius: 10px;
  margin-bottom: 6px;
}

.producer-qr {
  flex-shrink: 0;
  padding: 5px;
  background: #fff;
  border-radius: 8px;
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
  font-size: 10px;
  font-weight: 600;
  color: var(--accent);
  background: var(--accent-bg);
  padding: 1px 7px;
  border-radius: 999px;
}

.producer-id {
  font-family: 'Courier New', monospace;
  font-size: 11px;
  font-weight: 600;
  color: var(--text-h);
}

.producer-qty {
  margin-left: auto;
  font-size: 12px;
  font-weight: 600;
  color: var(--text-h);
  white-space: nowrap;
}

.producer-name {
  margin: 2px 0 1px;
  font-size: 13px;
  font-weight: 600;
  color: var(--text-h);
}

.producer-lines {
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.producer-line {
  display: flex;
  align-items: center;
  gap: 6px;
  margin: 0;
  font-size: 11.5px;
  color: var(--text);
}

.producer-line svg {
  flex-shrink: 0;
  color: var(--accent);
}

.doc-footer {
  margin: 10px 0 0;
  font-size: 11.5px;
  color: var(--text);
  border-top: 1px dashed var(--border);
  padding-top: 8px;
}

@media (max-width: 600px) {
  .page-header {
    flex-direction: column;
    align-items: flex-start;
  }

  .doc-order {
    grid-template-columns: repeat(2, minmax(0, 1fr));
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

  .doc-field-sacs .sacs-editor,
  .doc-field-sacs .sacs-feedback {
    display: none !important;
  }

  .doc-field-sacs .sacs-print-count {
    display: inline-block;
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
