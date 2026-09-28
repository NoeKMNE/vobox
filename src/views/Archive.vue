<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { ArrowDown, ArrowUp, FileText, Loader } from '@lucide/vue'
import CalligraphText from '../components/CalligraphText.vue'
import { getToken, listFiches, type FicheSummary } from '../composables/useAuth'

const router = useRouter()

const fiches = ref<FicheSummary[]>([])
const loading = ref(false)
const error = ref('')
const triDesc = ref(true)

const formatKg = (kg: number) => {
  const tonnesVal = kg / 1000
  return `${new Intl.NumberFormat('fr-FR', { maximumFractionDigits: 2 }).format(Math.round(tonnesVal * 100) / 100)} t`
}

const triees = computed(() =>
  [...fiches.value].sort((a, b) =>
    triDesc.value ? b.created_at.localeCompare(a.created_at) : a.created_at.localeCompare(b.created_at)
  )
)

const formatDate = (iso: string) => {
  const [y, m, d] = iso.slice(0, 10).split('-').map(Number)
  if (!y || !m || !d) return iso
  return new Intl.DateTimeFormat('fr-FR', { day: 'numeric', month: 'long', year: 'numeric' }).format(new Date(y, m - 1, d))
}

const load = async () => {
  const token = getToken()
  if (!token) return
  loading.value = true
  error.value = ''
  try {
    fiches.value = await listFiches(token)
  } catch (e) {
    error.value = typeof e === 'string' ? e : 'Chargement impossible'
  } finally {
    loading.value = false
  }
}

const voir = (id: number) => {
  router.push({ name: 'Sheet', query: { id: String(id) } })
}

onMounted(load)
</script>

<template>
  <div class="archive-page">
    <header class="page-header">
      <div class="page-header-text">
        <h1><CalligraphText text="Archives" :initial="true" /></h1>
        <p>Historique des fiches de traçabilité</p>
      </div>
    </header>

    <div class="archive-scroll">
      <p v-if="error" class="msg error">{{ error }}</p>

      <section class="history-card">
        <div class="history-toolbar">
          <h2>Fiches de traçabilité</h2>
          <div class="sort-group" role="group" aria-label="Tri par date">
            <button
              class="sort-btn"
              :class="{ active: triDesc }"
              title="Plus récentes d'abord"
              @click="triDesc = true"
            >
              <ArrowDown :size="13" />
              <span>Récentes</span>
            </button>
            <button
              class="sort-btn"
              :class="{ active: !triDesc }"
              title="Plus anciennes d'abord"
              @click="triDesc = false"
            >
              <ArrowUp :size="13" />
              <span>Anciennes</span>
            </button>
          </div>
        </div>

        <p v-if="loading" class="empty"><Loader class="spin" :size="14" /> Chargement…</p>
        <p v-else-if="!fiches.length" class="empty">Aucune fiche générée pour le moment.</p>

        <div v-else class="history-list">
          <div v-for="h in triees" :key="h.id" class="history-row">
            <FileText :size="16" class="row-icon" />
            <div class="history-main">
              <div class="history-top">
                <span class="history-ref">{{ h.reference }}</span>
                <span class="history-stat" :class="h.statut">{{ h.statut === 'complete' ? 'Complète' : 'Partielle' }}</span>
              </div>
              <span class="history-prod">{{ h.titre }} · {{ h.produit }} · {{ formatKg(h.quantite_attribuee_kg) }} sur demande {{ formatKg(h.quantite_demandee_kg) }}</span>
              <span class="history-meta">{{ h.nb_producteurs }} producteur{{ h.nb_producteurs > 1 ? 's' : '' }} · {{ formatDate(h.created_at) }}</span>
            </div>
            <button class="view-btn" @click="voir(h.id)">
              Voir
            </button>
          </div>
        </div>
      </section>
    </div>
  </div>
</template>

<style scoped>
.archive-page {
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
  font-size: 14px;
}

.archive-scroll {
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

.history-card {
  background: linear-gradient(180deg, var(--bg) 0%, rgba(255, 255, 255, 0.5) 100%);
  border: 1px solid var(--border);
  border-radius: 20px;
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.06);
  padding: 18px;
}

.history-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  flex-wrap: wrap;
  margin-bottom: 14px;
}

.history-toolbar h2 {
  margin: 0;
  font-size: 16px;
  font-weight: 700;
  color: var(--text-h);
}

.sort-group {
  display: inline-flex;
  gap: 4px;
  padding: 3px;
  background: var(--bg);
  border: 1px solid var(--border);
  border-radius: 999px;
}

.sort-btn {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 5px 12px;
  border: none;
  border-radius: 999px;
  background: transparent;
  color: var(--text);
  font-size: 12px;
  font-weight: 700;
  cursor: pointer;
  transition: background 0.15s ease, color 0.15s ease;
}

.sort-btn:hover {
  color: var(--text-h);
}

.sort-btn.active {
  background: var(--accent);
  color: #fff;
}

.empty {
  margin: 0;
  color: var(--text);
  font-size: 13.5px;
  display: inline-flex;
  align-items: center;
  gap: 8px;
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
}

.history-row {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 14px;
  background: var(--bg);
  border: 1px solid var(--border);
  border-left: 3px solid var(--accent);
  border-radius: 14px;
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
  font-weight: 700;
  font-size: 13.5px;
  color: var(--text-h);
}

.history-stat {
  font-size: 10.5px;
  font-weight: 700;
  padding: 2px 8px;
  border-radius: 999px;
}

.history-stat.complete {
  color: #16a34a;
  background: rgba(34, 197, 94, 0.12);
}

.history-stat.partielle {
  color: #d97706;
  background: rgba(245, 158, 11, 0.14);
}

.history-prod {
  font-size: 12.5px;
  color: var(--accent);
  font-weight: 600;
  text-transform: capitalize;
}

.history-meta {
  font-size: 12px;
  color: var(--text);
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