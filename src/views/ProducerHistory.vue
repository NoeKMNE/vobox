<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { ArrowLeft, ChevronDown, FileText, Loader, Search } from '@lucide/vue'
import {
  getToken,
  historiqueProducteur,
  listExploitations,
  producteursParticipants,
  type Exploitation,
  type HistoriqueProducteur,
  type ResumeParticipation
} from '../composables/useAuth'

const route = useRoute()
const router = useRouter()

const exploitants = ref<Exploitation[]>([])
const participants = ref<Record<number, ResumeParticipation>>({})
const recherche = ref('')
const erreurChargement = ref('')
const expanded = ref<Set<number>>(new Set())
const historiques = ref<Record<number, HistoriqueProducteur>>({})
const chargement = ref<Record<number, boolean>>({})
const erreurs = ref<Record<number, string>>({})

const formatNum = (n: number, digits = 0) =>
  new Intl.NumberFormat('fr-FR', { maximumFractionDigits: digits }).format(n)

const formatKg = (kg: number) => {
  if (kg < 1000) return `${formatNum(Math.round(kg))} kg`
  const tonnesVal = kg / 1000
  const arrondi = Math.round(tonnesVal * 100) / 100
  return `${formatNum(arrondi, 2)} t`
}

const formatDate = (iso: string) => {
  const [y, m, d] = iso.slice(0, 10).split('-').map(Number)
  if (!y || !m || !d) return iso
  return new Intl.DateTimeFormat('fr-FR', { day: 'numeric', month: 'long', year: 'numeric' }).format(new Date(y, m - 1, d))
}

const DIACRITIQUES = new RegExp('[\\u0300-\\u036f]', 'g')

const normaliser = (s: string) => s.normalize('NFD').replace(DIACRITIQUES, '').toLowerCase()

const exploitantNom = (e: Exploitation) =>
  [e.operateur_prenom, e.operateur_nom].filter(Boolean).join(' ') ||
  [e.proprietaire_prenom, e.proprietaire_nom].filter(Boolean).join(' ') ||
  'Nom non renseigné'

const exploitantLocalisation = (e: Exploitation) =>
  [e.village, e.district].filter(Boolean).join(' · ')

const exploitantsTriee = computed(() =>
  [...exploitants.value].sort((a, b) => exploitantNom(a).localeCompare(exploitantNom(b), 'fr'))
)

const producteursFiltres = computed(() => {
  const base = exploitantsTriee.value.filter((e) => participants.value[e.id] != null)
  const q = normaliser(recherche.value.trim())
  if (!q) return base
  return base.filter((e) =>
    normaliser(`${e.id_interne ?? ''} ${exploitantNom(e)} ${exploitantLocalisation(e)}`).includes(q)
  )
})

const aParticipants = computed(() => Object.keys(participants.value).length > 0)

const resume = (id: number): ResumeParticipation => participants.value[id]

const participationsTriees = (id: number) =>
  [...(historiques.value[id]?.participations ?? [])].sort((a, b) =>
    b.created_at.localeCompare(a.created_at)
  )

const totalHistorique = (id: number) =>
  (historiques.value[id]?.participations ?? []).reduce((s, p) => s + p.quantite_kg, 0)

const loadDonnees = async () => {
  const token = getToken()
  if (!token) return
  try {
    const [exps, parts] = await Promise.all([listExploitations(token), producteursParticipants(token)])
    exploitants.value = exps
    participants.value = Object.fromEntries(parts.map((p) => [p.exploitation_id, p]))
    const preselect = Number(route.query.exploitant)
    if (preselect && participants.value[preselect]) {
      await basculer(preselect)
    }
  } catch (e) {
    erreurChargement.value = typeof e === 'string' ? e : 'Impossible de charger les producteurs'
  }
}

const basculer = async (id: number) => {
  if (expanded.value.has(id)) {
    expanded.value.delete(id)
    return
  }
  expanded.value.add(id)
  if (historiques.value[id] || chargement.value[id]) return
  const token = getToken()
  if (!token) return
  chargement.value[id] = true
  try {
    historiques.value[id] = await historiqueProducteur(token, id)
  } catch (e) {
    erreurs.value[id] = typeof e === 'string' ? e : "Impossible de charger l'historique"
  } finally {
    chargement.value[id] = false
  }
}

const voir = (id: number) => {
  router.push({ name: 'Sheet', query: { id: String(id) } })
}

const retour = () => {
  router.push({ name: 'Archive' })
}

onMounted(loadDonnees)
</script>

<template>
  <div class="archive-page">
    <header class="page-header">
      <button class="back-btn" @click="retour" title="Retour aux archives">
        <ArrowLeft :size="16" />
      </button>
      <div class="page-header-text">
        <h1>Participations</h1>
        <p>Historique des participations de chaque producteur aux fiches de traçabilité</p>
      </div>
    </header>

    <div class="archive-scroll">
      <p v-if="erreurChargement" class="msg error">{{ erreurChargement }}</p>

      <div class="filters-row">
        <label class="search-wrap">
          <Search :size="14" class="search-icon" />
          <input
            v-model="recherche"
            class="search-input"
            type="search"
            placeholder="Rechercher un producteur (nom, identifiant, village…)"
          />
        </label>
      </div>

      <p v-if="!producteursFiltres.length" class="empty">
        {{
          !exploitants.length
            ? 'Aucun producteur importé.'
            : !aParticipants
              ? "Aucun producteur n'a encore participé à une fiche."
              : 'Aucun producteur trouvé.'
        }}
      </p>

      <div v-else class="producer-list">
        <div
          v-for="e in producteursFiltres"
          :key="e.id"
          class="producer-card"
          :class="{ open: expanded.has(e.id) }"
        >
          <button class="producer-head" :aria-expanded="expanded.has(e.id)" @click="basculer(e.id)">
            <span class="producer-meta">
              <strong>{{ exploitantNom(e) }}</strong>
              <small>{{ e.id_interne || `#${e.id}` }}<template v-if="exploitantLocalisation(e)"> · {{ exploitantLocalisation(e) }}</template></small>
            </span>
            <span class="producer-stats">{{ resume(e.id).nb_participations }} · {{ formatKg(resume(e.id).total_kg) }}</span>
            <ChevronDown :size="16" class="chev" />
          </button>

          <div v-if="expanded.has(e.id)" class="producer-body">
            <p v-if="erreurs[e.id]" class="msg error">{{ erreurs[e.id] }}</p>
            <p v-else-if="chargement[e.id]" class="empty"><Loader class="spin" :size="14" /> Chargement…</p>
            <template v-else-if="historiques[e.id]">
              <p v-if="!historiques[e.id].participations.length" class="empty">
                Aucune participation à une fiche.
              </p>
              <div v-else>
                <div class="part-summary">
                  <span>{{ historiques[e.id].nb_participations }} participation{{ historiques[e.id].nb_participations > 1 ? 's' : '' }}</span>
                  <span class="part-total">Total : <b>{{ formatKg(totalHistorique(e.id)) }}</b></span>
                </div>
                <div class="history-list">
                  <div v-for="hp in participationsTriees(e.id)" :key="hp.fiche_id" class="history-row">
                    <FileText :size="16" class="row-icon" />
                    <div class="history-main">
                      <div class="history-top">
                        <span class="history-ref">{{ hp.reference }}</span>
                        <span class="history-stat" :class="hp.statut">{{ hp.statut === 'complete' ? 'Complète' : 'Partielle' }}</span>
                        <span class="part-qty">{{ formatKg(hp.quantite_kg) }}</span>
                      </div>
                      <span class="history-prod">{{ hp.titre }} · {{ hp.produit }}</span>
                      <span class="history-meta">{{ formatDate(hp.created_at) }}</span>
                    </div>
                    <button class="view-btn" @click="voir(hp.fiche_id)">
                      Voir
                    </button>
                  </div>
                </div>
              </div>
            </template>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.archive-page {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
}

.page-header {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 18px 24px 12px;
}

.page-header-text h1 {
  margin: 0;
  font-size: 26px;
}

.page-header-text p {
  margin: 2px 0 0;
  font-size: 13px;
  color: var(--text);
}

.back-btn {
  flex-shrink: 0;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 36px;
  border-radius: 10px;
  border: 1px solid var(--border);
  background: var(--bg);
  color: var(--text-h);
  cursor: pointer;
}

.back-btn:hover {
  border-color: var(--accent);
  color: var(--accent);
}

.archive-scroll {
  flex: 1;
  overflow-y: auto;
  padding: 0 24px 24px;
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.filters-row {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

.search-wrap {
  flex: 1 1 240px;
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  border-radius: 12px;
  border: 1px solid var(--border);
  background: var(--bg);
}

.search-wrap:focus-within {
  border-color: var(--accent);
}

.search-icon {
  flex-shrink: 0;
  color: var(--text);
}

.search-input {
  flex: 1;
  min-width: 0;
  border: none;
  outline: none;
  background: transparent;
  color: var(--text-h);
  font-size: 13px;
}

.search-input::placeholder {
  color: var(--text);
}

.producer-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.producer-card {
  background: var(--bg);
  border: 1px solid var(--border);
  border-radius: 14px;
  overflow: hidden;
}

.producer-card.open {
  border-color: var(--accent-border);
}

.producer-head {
  width: 100%;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 11px 14px;
  border: none;
  background: transparent;
  cursor: pointer;
  text-align: left;
}

.producer-meta {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.producer-meta strong {
  font-size: 13.5px;
  color: var(--text-h);
}

.producer-meta small {
  font-size: 11.5px;
  color: var(--text);
}

.producer-stats {
  flex-shrink: 0;
  font-size: 11.5px;
  font-weight: 700;
  color: var(--accent);
  background: var(--accent-bg);
  padding: 2px 9px;
  border-radius: 999px;
  white-space: nowrap;
}

.chev {
  flex-shrink: 0;
  color: var(--text);
  transition: transform 0.15s ease;
}

.producer-card.open .chev {
  transform: rotate(180deg);
  color: var(--accent);
}

.producer-body {
  border-top: 1px solid var(--border);
  padding: 6px 14px 14px;
}

.part-summary {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  font-size: 12.5px;
  color: var(--text);
  padding: 8px 4px;
}

.part-total b {
  color: var(--accent);
}

.empty {
  padding: 18px 4px;
  font-size: 13px;
  color: var(--text);
  display: flex;
  align-items: center;
  gap: 8px;
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

.spin {
  animation: spin 0.9s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

.history-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-top: 4px;
}

.history-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  border: 1px solid var(--border);
  border-radius: 12px;
}

.row-icon {
  flex-shrink: 0;
  color: var(--accent);
}

.history-main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.history-top {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

.history-ref {
  font-size: 13px;
  font-weight: 700;
  color: var(--text-h);
}

.history-stat {
  font-size: 10.5px;
  font-weight: 700;
  padding: 1px 8px;
  border-radius: 999px;
  background: var(--accent-bg);
  color: var(--accent);
}

.history-prod {
  font-size: 12.5px;
  color: var(--text-h);
}

.history-meta {
  font-size: 11.5px;
  color: var(--text);
}

.part-qty {
  font-size: 11.5px;
  font-weight: 700;
  color: var(--accent);
  background: var(--accent-bg);
  padding: 2px 9px;
  border-radius: 999px;
}

.view-btn {
  flex-shrink: 0;
  padding: 7px 14px;
  border-radius: 10px;
  border: 1px solid var(--border);
  background: var(--accent-bg);
  color: var(--accent);
  font-size: 13px;
  font-weight: 700;
  cursor: pointer;
  transition: filter 0.15s ease;
}

.view-btn:hover {
  filter: brightness(1.05);
}

@media (max-width: 480px) {
  .history-row {
    align-items: flex-start;
    flex-wrap: wrap;
  }

  .view-btn {
    margin-left: auto;
  }
}
</style>
