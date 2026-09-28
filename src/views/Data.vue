<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { ChevronRight, FileUp, Loader, MapPin, Save, Search, Sprout, Trash2, UploadCloud, X } from '@lucide/vue'
import CalligraphText from '../components/CalligraphText.vue'
import { exportBackup, getToken, importExcel, listExploitations, resetData, searchExploitations, type Exploitation, type ImportResult, type SearchCategory } from '../composables/useAuth'

const router = useRouter()

const today = new Intl.DateTimeFormat('fr-FR', {
  day: 'numeric',
  month: 'long',
  year: 'numeric'
}).format(new Date())

const fiches = ref<Exploitation[]>([])
const searchQuery = ref('')
const searchResults = ref<Exploitation[]>([])
const searching = ref(false)
const searchCategory = ref<SearchCategory>('all')
const categories: { value: SearchCategory; label: string }[] = [
  { value: 'all', label: 'Tous' },
  { value: 'operateur', label: 'Opérateur' },
  { value: 'localisation', label: 'Village' },
  { value: 'culture', label: 'Culture' },
  { value: 'unite', label: 'Unité' },
  { value: 'identifiant', label: 'ID' },
  { value: 'inspecteur', label: 'Inspecteur' }
]
let searchTimer: number | undefined
let searchSeq = 0
const fileInput = ref<HTMLInputElement | null>(null)
const dragOver = ref(false)
const importing = ref(false)
const error = ref('')
const lastImport = ref<ImportResult | null>(null)
const saving = ref(false)
const resetting = ref(false)
const saveMsg = ref('')
const resetMsg = ref('')

const searchingActive = computed(() => searchQuery.value.trim().length > 0)

const formatNum = (n: number | null | undefined, digits = 1) =>
  n == null ? '—' : new Intl.NumberFormat('fr-FR', { maximumFractionDigits: digits }).format(n)

const exploitantName = (f: Exploitation) =>
  [f.operateur_prenom, f.operateur_nom].filter(Boolean).join(' ') || 'Opérateur non renseigné'

const localisation = (f: Exploitation) =>
  [f.village, f.district, f.region_inspection].filter(Boolean).join(' · ') || '—'

const sortBy = ref<'date' | 'name' | 'area'>('date')
const displayed = computed(() => {
  if (searchingActive.value) return searchResults.value
  const list = [...fiches.value]
  switch (sortBy.value) {
    case 'name':
      return list.sort((a, b) => exploitantName(a).localeCompare(exploitantName(b), 'fr'))
    case 'area':
      return list.sort((a, b) => (b.superficie_total ?? -1) - (a.superficie_total ?? -1))
    default:
      return list
  }
})

const load = async () => {
  const token = getToken()
  if (!token) return
  try {
    fiches.value = await listExploitations(token)
  } catch (e) {
    error.value = typeof e === 'string' ? e : 'Impossible de charger les données'
  }
}

const onSearchInput = () => {
  window.clearTimeout(searchTimer)
  const q = searchQuery.value.trim()
  if (!q) {
    searchSeq++
    searching.value = false
    searchResults.value = []
    return
  }
  searching.value = true
  searchTimer = window.setTimeout(async () => {
    const token = getToken()
    if (!token) return
    const seq = ++searchSeq
    const cat = searchCategory.value
    try {
      const results = await searchExploitations(token, q, cat)
      if (seq !== searchSeq) return
      searchResults.value = results
    } catch (e) {
      if (seq !== searchSeq) return
      searchResults.value = []
      error.value = typeof e === 'string' ? e : 'Recherche impossible'
    } finally {
      if (seq === searchSeq) searching.value = false
    }
  }, 180)
}

const setCategory = (c: SearchCategory) => {
  if (searchCategory.value === c) return
  searchCategory.value = c
  if (searchingActive.value) onSearchInput()
}

const clearSearch = () => {
  window.clearTimeout(searchTimer)
  searchSeq++
  searchQuery.value = ''
  searchResults.value = []
  searchCategory.value = 'all'
  searching.value = false
}

onBeforeUnmount(() => window.clearTimeout(searchTimer))

onMounted(load)

const readFileAsBase64 = (file: File) =>
  new Promise<string>((resolve, reject) => {
    const reader = new FileReader()
    reader.onload = () => {
      const result = reader.result as string
      resolve(result.split(',')[1] ?? '')
    }
    reader.onerror = () => reject(reader.error)
    reader.readAsDataURL(file)
  })

const handleFile = async (file?: File | null) => {
  if (!file || importing.value) return
  importing.value = true
  error.value = ''
  saveMsg.value = ''
  lastImport.value = null
  try {
    const token = getToken()
    if (!token) throw new Error('Session expirée')
    const b64 = await readFileAsBase64(file)
    const result = await importExcel(token, b64, file.name)
    lastImport.value = result
    clearSearch()
    await load()
  } catch (e) {
    error.value = typeof e === 'string' ? e : 'Erreur lors de l’import du fichier'
  } finally {
    importing.value = false
    if (fileInput.value) fileInput.value.value = ''
  }
}

const openPicker = () => fileInput.value?.click()

const onDrop = (e: DragEvent) => {
  dragOver.value = false
  handleFile(e.dataTransfer?.files?.[0])
}

const openFiche = (id: number) => router.push({ name: 'Exploitation', params: { id } })

const doSave = async () => {
  const token = getToken()
  if (!token) return
  saving.value = true
  saveMsg.value = ''
  error.value = ''
  try {
    const path = await exportBackup(token)
    saveMsg.value = `Sauvegarde créée : ${path}`
  } catch (e) {
    error.value = typeof e === 'string' ? e : 'Sauvegarde impossible'
  } finally {
    saving.value = false
  }
}

const doReset = async () => {
  if (!window.confirm('Vider toutes les données importées (exploitations, cultures, unités agricoles) ?')) return
  const token = getToken()
  if (!token) return
  resetting.value = true
  resetMsg.value = ''
  error.value = ''
  try {
    const total = await resetData(token)
    fiches.value = []
    lastImport.value = null
    clearSearch()
    resetMsg.value = `Réinitialisation terminée (${total} enregistrement${total > 1 ? 's' : ''} supprimé${total > 1 ? 's' : ''})`
  } catch (e) {
    error.value = typeof e === 'string' ? e : 'Réinitialisation impossible'
  } finally {
    resetting.value = false
  }
}
</script>

<template>
  <div class="data-page">
    <header class="page-header">
      <div class="page-header-text">
        <h1><CalligraphText text="Données" :initial="true" /></h1>
        <p>Importez vos fichiers Excel pour charger vos exploitations agricoles</p>
      </div>
      <div class="page-actions">
        <span class="header-chip">{{ today }}</span>
        <button class="action-btn" :disabled="saving" @click="doSave">
          <Save :size="15" />{{ saving ? 'En cours…' : 'Sauvegarder' }}
        </button>
        <button class="action-btn danger" :disabled="resetting || !fiches.length" @click="doReset">
          <Trash2 :size="15" />{{ resetting ? '…' : 'Réinitialiser' }}
        </button>
      </div>
    </header>

    <div
      class="dropzone"
      :class="{ over: dragOver, busy: importing }"
      role="button"
      tabindex="0"
      @click="openPicker"
      @keydown.enter="openPicker"
      @dragover.prevent="dragOver = true"
      @dragleave="dragOver = false"
      @drop.prevent="onDrop"
    >
      <input
        ref="fileInput"
        type="file"
        accept=".xlsx,.xls"
        class="file-input"
        @change="handleFile(($event.target as HTMLInputElement).files?.[0])"
      />
      <span class="dropzone-icon"><UploadCloud :size="26" /></span>
      <p class="dropzone-title">
        {{ importing ? 'Chargement des données…' : 'Glissez votre fichier Excel ici' }}
      </p>
      <p class="dropzone-sub">
        {{ importing ? 'Analyse et enregistrement en cours' : 'ou cliquez pour choisir un fichier (.xlsx)' }}
      </p>
      <Loader v-if="importing" class="spin" :size="30" />
    </div>

    <p v-if="error" class="msg error">{{ error }}</p>
    <p v-if="saveMsg" class="msg ok save-msg">{{ saveMsg }}</p>
    <p v-if="resetMsg" class="msg ok">{{ resetMsg }}</p>

    <div v-if="lastImport" class="msg ok">
      {{ lastImport.exploitations_added }} exploitation{{ lastImport.exploitations_added > 1 ? 's' : '' }} importée{{ lastImport.exploitations_added > 1 ? 's' : '' }},
      {{ lastImport.cultures_added }} culture{{ lastImport.cultures_added > 1 ? 's' : '' }},
      {{ lastImport.unites_added }} unité{{ lastImport.unites_added > 1 ? 's' : '' }} agricole{{ lastImport.unites_added > 1 ? 's' : '' }}
    </div>

    <details v-if="lastImport && lastImport.sheets.length" class="debug-box">
      <summary>Structure détectée dans le fichier</summary>
      <div v-for="s in lastImport.sheets" :key="s.name" class="debug-sheet">
        <p class="debug-title">— Feuille « {{ s.name }} » ({{ s.rows }} lignes)</p>
        <pre v-for="(r, i) in s.sample" :key="i" class="debug-row">{{ r.join(' | ') }}</pre>
      </div>
    </details>

    <section v-if="displayed.length || searchingActive" class="fiches">
      <div class="fiches-head">
        <div class="searchbar">
          <Search :size="15" class="search-icon" />
          <input
            v-model="searchQuery"
            class="search-input"
            type="search"
            placeholder="Rechercher : opérateur, village, ID, culture, inspecteur…"
            aria-label="Rechercher dans les données"
            autocomplete="off"
            spellcheck="false"
            @input="onSearchInput"
            @keydown.esc="clearSearch"
          />
          <Loader v-if="searching" class="search-loading" :size="14" />
          <button v-else-if="searchingActive" class="search-clear" aria-label="Effacer la recherche" @click="clearSearch">
            <X :size="13" />
          </button>
        </div>
        <div class="category-row" role="group" aria-label="Catégorie de recherche">
          <button
            v-for="c in categories"
            :key="c.value"
            class="category-chip"
            :class="{ active: searchCategory === c.value }"
            @click="setCategory(c.value)"
          >
            {{ c.label }}
          </button>
        </div>
        <div class="fiches-meta">
          <h2 class="fiches-title">
          <template v-if="searchingActive">
            {{ displayed.length }} résultat{{ displayed.length > 1 ? 's' : '' }} pour « {{ searchQuery.trim() }} »
          </template>
          <template v-else>
            {{ displayed.length }} exploitation{{ displayed.length > 1 ? 's' : '' }} chargée{{ displayed.length > 1 ? 's' : '' }}
          </template>
          </h2>
          <select
            v-if="!searchingActive && displayed.length"
            v-model="sortBy"
            class="sort-select"
            aria-label="Trier les exploitations"
          >
            <option value="date">Récents</option>
            <option value="name">Opérateur A–Z</option>
            <option value="area">Superficie</option>
          </select>
        </div>
      </div>

      <p v-if="searchingActive && !displayed.length && !searching" class="no-result">
        Aucun résultat pour « {{ searchQuery.trim() }} ».
      </p>

      <div v-else class="fiche-list">
        <article
          v-for="f in displayed"
          :key="f.id"
          class="tile"
          role="button"
          tabindex="0"
          @click="openFiche(f.id)"
          @keydown.enter="openFiche(f.id)"
        >
          <span class="tile-id">#{{ f.id_interne || f.id }}</span>

          <div class="tile-main">
            <p class="tile-name" :title="exploitantName(f)">{{ exploitantName(f) }}</p>
            <p class="tile-line"><MapPin :size="12" />{{ localisation(f) }}</p>
          </div>

          <div class="tile-side">
            <span class="tile-superf"><Sprout :size="12" />{{ formatNum(f.superficie_total) }} ha</span>
            <div class="tile-counts">
              <span class="count badge-culture">{{ f.cultures_count }} c.</span>
              <span class="count badge-unite">{{ f.unites_count }} u.</span>
            </div>
          </div>

          <ChevronRight class="tile-arrow" :size="16" />
        </article>
      </div>
    </section>

    <section v-else-if="!error" class="empty-state">
      <span class="empty-icon"><FileUp :size="22" /></span>
      <p>Aucune exploitation chargée pour le moment.</p>
    </section>
  </div>
</template>

<style scoped>
.data-page {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow-y: auto;
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

.dropzone {
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 6px;
  height: 190px;
  margin: 0 12px;
  padding: 24px;
  border: 1.5px dashed var(--border);
  border-radius: 22px;
  background: linear-gradient(180deg, var(--bg) 0%, rgba(255, 255, 255, 0.5) 100%);
  cursor: pointer;
  text-align: center;
  transition: border-color 0.2s ease, background 0.2s ease, transform 0.2s ease;
}

.dropzone:hover,
.dropzone.over {
  border-color: var(--accent);
  background: var(--accent-bg);
  transform: translateY(-1px);
}

.dropzone.busy {
  pointer-events: none;
}

.file-input {
  display: none;
}

.dropzone-icon {
  display: grid;
  place-items: center;
  width: 54px;
  height: 54px;
  border-radius: 16px;
  background: var(--accent-bg);
  color: var(--accent);
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.06);
}

.dropzone-title {
  margin: 6px 0 0;
  font-size: 16px;
  font-weight: 600;
  color: var(--text-h);
}

.dropzone-sub {
  margin: 0;
  font-size: 13px;
  color: var(--text);
}

.spin {
  position: absolute;
  top: 50%;
  left: 50%;
  margin: -16px 0 0 -16px;
  color: var(--accent);
  animation: spin 0.9s linear infinite;
}

.dropzone.busy .dropzone-icon,
.dropzone.busy .dropzone-title,
.dropzone.busy .dropzone-sub {
  opacity: 0.35;
}

.dropzone {
  position: relative;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

.msg {
  margin: 12px 12px 0;
  padding: 10px 14px;
  border-radius: 12px;
  font-size: 13px;
  font-weight: 500;
}

.msg.ok {
  background: rgba(34, 197, 94, 0.12);
  color: var(--green-ok, #16a34a);
  border: 1px solid rgba(34, 197, 94, 0.25);
}

.msg.error {
  background: rgba(220, 38, 38, 0.1);
  color: #dc2626;
  border: 1px solid rgba(220, 38, 38, 0.25);
}

.debug-box {
  flex-shrink: 0;
  margin: 12px 12px 0;
  padding: 12px 14px;
  background: rgba(0, 0, 0, 0.35);
  border: 1px solid var(--border);
  border-radius: 14px;
  color: var(--text);
  font-size: 12px;
}

.debug-box summary {
  cursor: pointer;
  font-weight: 600;
  color: var(--text-h);
}

.debug-sheet {
  margin-top: 8px;
}

.debug-title {
  margin: 0 0 4px;
  font-weight: 600;
  color: var(--text-h);
}

.debug-row {
  margin: 0 0 2px;
  padding: 2px 8px;
  border-radius: 6px;
  background: rgba(0, 0, 0, 0.25);
  white-space: pre-wrap;
  word-break: break-word;
  font-family: 'Courier New', monospace;
}

.page-actions {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

.action-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 8px 12px;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: var(--bg);
  color: var(--text-h);
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: border-color 0.2s ease, color 0.2s ease, transform 0.2s ease;
}

.action-btn:hover:not(:disabled) {
  border-color: var(--accent);
  color: var(--accent);
  transform: translateY(-1px);
}

.action-btn.danger:hover:not(:disabled) {
  border-color: #dc2626;
  color: #dc2626;
}

.action-btn:disabled {
  opacity: 0.5;
  cursor: default;
}

.save-msg {
  word-break: break-all;
}

.fiches {
  flex-shrink: 0;
  padding: 18px 12px 8px;
}

.fiches-head {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-bottom: 10px;
}

.searchbar {
  display: flex;
  align-items: center;
  gap: 7px;
  padding: 0 10px;
  height: 36px;
  background: var(--bg);
  border: 1px solid var(--border);
  border-radius: 11px;
  box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.03);
  transition: border-color 0.2s ease;
}

.searchbar:focus-within {
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
  opacity: 0.85;
}

.search-input::-webkit-search-cancel-button {
  display: none;
}

.category-row {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
}

.category-chip {
  padding: 4px 11px;
  border-radius: 999px;
  border: 1px solid var(--border);
  background: var(--bg);
  color: var(--text);
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: border-color 0.15s ease, color 0.15s ease, background 0.15s ease;
}

.category-chip:hover {
  color: var(--text-h);
  border-color: color-mix(in srgb, var(--accent) 45%, var(--border));
}

.category-chip.active {
  color: var(--accent);
  border-color: color-mix(in srgb, var(--accent) 55%, var(--border));
  background: var(--accent-bg);
}

.search-loading {
  flex-shrink: 0;
  color: var(--accent);
  animation: spin 0.9s linear infinite;
}

.search-clear {
  flex-shrink: 0;
  width: 22px;
  height: 22px;
  display: grid;
  place-items: center;
  border: none;
  border-radius: 7px;
  background: none;
  color: var(--text);
  cursor: pointer;
  transition: color 0.15s ease, background 0.15s ease;
}

.search-clear:hover {
  color: var(--text-h);
  background: var(--accent-bg);
}

.no-result {
  margin: 0;
  padding: 26px 0 38px;
  text-align: center;
  font-size: 13px;
  color: var(--text);
}

.fiches-title {
  margin: 0;
  font-size: 14px;
  font-weight: 600;
  color: var(--text-h);
}

.fiches-meta {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.sort-select {
  flex-shrink: 0;
  padding: 4px 8px;
  border-radius: 9px;
  border: 1px solid var(--border);
  background: var(--bg);
  color: var(--text-h);
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  outline: none;
  transition: border-color 0.15s ease, color 0.15s ease;
}

.sort-select:hover,
.sort-select:focus-visible {
  border-color: var(--accent);
  color: var(--accent);
}

.fiche-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.tile {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 12px;
  background: linear-gradient(180deg, var(--bg) 0%, rgba(255, 255, 255, 0.5) 100%);
  border: 1px solid var(--border);
  border-radius: 12px;
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.06);
  cursor: pointer;
  transition: transform 0.15s ease, box-shadow 0.15s ease, border-color 0.15s ease;
}

.tile:hover {
  transform: translateY(-1px);
  border-color: color-mix(in srgb, var(--accent) 40%, var(--border));
  box-shadow:
    inset 0 1px 0 rgba(255, 255, 255, 0.08),
    0 6px 14px -8px rgba(0, 0, 0, 0.18);
}

.tile:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}

.tile-id {
  flex-shrink: 0;
  font-family: 'Courier New', monospace;
  font-size: 11px;
  font-weight: 700;
  color: var(--accent);
  background: var(--accent-bg);
  padding: 3px 8px;
  border-radius: 7px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 180px;
}

.tile-main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 3px;
}

.tile-name {
  margin: 0;
  font-size: 13.5px;
  font-weight: 700;
  color: var(--text-h);
  line-height: 1.25;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.tile-line {
  display: flex;
  align-items: center;
  gap: 5px;
  margin: 0;
  font-size: 12px;
  color: var(--text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.tile-line svg {
  flex-shrink: 0;
  color: var(--accent);
}

.tile-side {
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 5px;
}

.tile-superf {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  font-size: 12.5px;
  font-weight: 600;
  color: var(--text-h);
  white-space: nowrap;
}

.tile-superf svg {
  color: var(--accent);
}

.tile-counts {
  display: flex;
  gap: 4px;
}

.count {
  font-size: 10.5px;
  font-weight: 600;
  padding: 2px 7px;
  border-radius: 999px;
  border: 1px solid var(--border);
}

.badge-culture {
  color: var(--green-ok, #16a34a);
  background: rgba(34, 197, 94, 0.1);
}

.badge-unite {
  color: #2563eb;
  background: rgba(37, 99, 235, 0.1);
}

.tile-arrow {
  flex-shrink: 0;
  color: var(--text);
  transition: transform 0.15s ease, color 0.15s ease;
}

.tile:hover .tile-arrow {
  transform: translateX(2px);
  color: var(--accent);
}

.empty-state {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  color: var(--text);
  font-size: 14px;
}

.empty-icon {
  display: grid;
  place-items: center;
  width: 46px;
  height: 46px;
  border-radius: 14px;
  background: var(--bg);
  border: 1px solid var(--border);
  color: var(--text);
}

@media (max-width: 480px) {
  .page-header {
    flex-direction: column;
    align-items: flex-start;
  }

  .tile {
    gap: 8px;
    padding: 9px 10px;
  }

  .tile-superf {
    display: none;
  }

  .tile-id {
    max-width: 110px;
  }
}
</style>