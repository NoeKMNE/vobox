<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { ChevronRight, Download, Eye, FileSpreadsheet, FileUp, Loader, MapPin, Save, Search, Sprout, Trash2, UploadCloud, X } from '@lucide/vue'
import { analyserFicheImport, exportBackup, getToken, getWorkingExcel, importExcel, importerFicheHistorique, listExploitations, resetData, searchExploitations, workingExcelExists, type Exploitation, type FicheAnalysis, type FicheImportRequest, type ImportResult, type SearchCategory, type WorkingExcel } from '../composables/useAuth'

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
const workingExcelAvailable = ref(false)
const loadingWorkingExcel = ref(false)
const workingExcel = ref<WorkingExcel | null>(null)
const ficheAnalysis = ref<FicheAnalysis | null>(null)
const analysingFiche = ref(false)
const savingFiche = ref(false)
const ficheDragOver = ref(false)
const ficheError = ref('')
const ficheSuccess = ref('')
const ficheFileInput = ref<HTMLInputElement | null>(null)

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

const checkWorkingExcel = async () => {
  const token = getToken()
  if (!token) return
  try {
    workingExcelAvailable.value = await workingExcelExists(token)
  } catch (e) {
    error.value = typeof e === 'string' ? e : 'Vérification du fichier Excel impossible'
  }
}

onMounted(() => {
  void load()
  void checkWorkingExcel()
})

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
    workingExcelAvailable.value = true
    clearSearch()
    await load()
  } catch (e) {
    error.value = typeof e === 'string' ? e : 'Erreur lors de l’import du fichier'
  } finally {
    importing.value = false
    if (fileInput.value) fileInput.value.value = ''
  }
}

const analyseFicheFile = async (file?: File | null) => {
  if (!file || analysingFiche.value || savingFiche.value) return
  ficheError.value = ''
  ficheSuccess.value = ''
  ficheAnalysis.value = null
  const extension = file.name.split('.').pop()?.toLowerCase()
  if (!['xlsx', 'xls', 'jpg', 'jpeg', 'png', 'webp'].includes(extension ?? '')) {
    ficheError.value = 'Formats acceptés : XLSX, XLS, JPG, PNG et WebP.'
    return
  }
  if (file.size > 12 * 1024 * 1024) {
    ficheError.value = 'Le fichier ne doit pas dépasser 12 Mo.'
    return
  }
  analysingFiche.value = true
  try {
    const token = getToken()
    if (!token) throw new Error('Session expirée')
    const fileB64 = await readFileAsBase64(file)
    ficheAnalysis.value = await analyserFicheImport(token, fileB64, file.name, file.type)
  } catch (e) {
    ficheError.value = typeof e === 'string' ? e : 'Analyse de la fiche impossible'
  } finally {
    analysingFiche.value = false
    if (ficheFileInput.value) ficheFileInput.value.value = ''
  }
}

const producteursPourLigne = () => {
  if (!ficheAnalysis.value) return []
  const produit = ficheAnalysis.value.produit.trim().toLocaleLowerCase('fr')
  return ficheAnalysis.value.producteurs.filter((p) => p.produit.trim().toLocaleLowerCase('fr') === produit)
}

const ligneAssociee = (line: FicheAnalysis['lignes'][number]) =>
  producteursPourLigne().some((p) => p.culture_id === line.culture_id)

const quantiteProposee = computed(() =>
  ficheAnalysis.value?.lignes.reduce((total, line) => total + line.quantite_kg, 0) ?? 0
)

const ficheValide = computed(() => {
  const analysis = ficheAnalysis.value
  return Boolean(
    analysis &&
    analysis.produit.trim() &&
    analysis.lignes.length &&
    analysis.lignes.every((line) =>
      ligneAssociee(line) && Number.isFinite(line.quantite_kg) && line.quantite_kg > 0
    ) &&
    (analysis.quantite_demandee_kg == null ||
      (Number.isFinite(analysis.quantite_demandee_kg) && analysis.quantite_demandee_kg >= quantiteProposee.value))
  )
})

const updateFicheQuantity = (value: string) => {
  if (!ficheAnalysis.value) return
  const quantity = value.trim() ? Number(value) : null
  ficheAnalysis.value.quantite_demandee_kg =
    quantity != null && Number.isFinite(quantity) && quantity > 0 ? quantity : null
}

const updateLineQuantity = (line: FicheAnalysis['lignes'][number], value: string) => {
  line.quantite_kg = value.trim() ? Number(value) : 0
}

const saveImportedFiche = async () => {
  const analysis = ficheAnalysis.value
  const token = getToken()
  if (!analysis || !token || !ficheValide.value || savingFiche.value) return
  savingFiche.value = true
  ficheError.value = ''
  ficheSuccess.value = ''
  try {
    const request: FicheImportRequest = {
      reference: analysis.reference,
      titre: analysis.titre,
      produit: analysis.produit,
      date: analysis.date,
      quantite_demandee_kg: analysis.quantite_demandee_kg ?? quantiteProposee.value,
      lignes: analysis.lignes.map((line) => ({
        culture_id: line.culture_id!,
        quantite_kg: line.quantite_kg
      }))
    }
    const saved = await importerFicheHistorique(token, request)
    ficheSuccess.value = `Fiche ${saved.reference} enregistrée dans les archives avec ${saved.items.length} producteur${saved.items.length > 1 ? 's' : ''} et ${formatNum(saved.quantite_attribuee_kg)} kg.`
    ficheAnalysis.value = null
  } catch (e) {
    ficheError.value = typeof e === 'string' ? e : 'Enregistrement de la fiche impossible'
  } finally {
    savingFiche.value = false
  }
}

const openPicker = () => fileInput.value?.click()

const openWorkingExcel = async () => {
  const token = getToken()
  if (!token || loadingWorkingExcel.value) return
  loadingWorkingExcel.value = true
  error.value = ''
  try {
    workingExcel.value = await getWorkingExcel(token)
  } catch (e) {
    error.value = typeof e === 'string' ? e : 'Ouverture du fichier Excel impossible'
  } finally {
    loadingWorkingExcel.value = false
  }
}

const downloadWorkingExcel = () => {
  if (!workingExcel.value) return
  const binary = atob(workingExcel.value.file_b64)
  const bytes = Uint8Array.from(binary, (char) => char.charCodeAt(0))
  const url = URL.createObjectURL(
    new Blob([bytes], { type: 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet' })
  )
  const link = document.createElement('a')
  link.href = url
  link.download = 'vobox-mis-a-jour.xlsx'
  link.click()
  URL.revokeObjectURL(url)
}

const closeWorkingExcel = () => {
  workingExcel.value = null
}

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
        <h1>Données</h1>
        <p>Importez vos exploitations et les fiches de traçabilité déjà produites</p>
      </div>
      <div class="page-actions">
        <span class="header-chip">{{ today }}</span>
        <button
          v-if="workingExcelAvailable"
          class="action-btn"
          :disabled="loadingWorkingExcel"
          @click="openWorkingExcel"
        >
          <Loader v-if="loadingWorkingExcel" class="spin-inline" :size="15" />
          <Eye v-else :size="15" />
          {{ loadingWorkingExcel ? 'Chargement…' : 'Voir le fichier Excel' }}
        </button>
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
      {{ lastImport.unites_added }} unité{{ lastImport.unites_added > 1 ? 's' : '' }} agricole{{ lastImport.unites_added > 1 ? 's' : '' }}<template v-if="lastImport.rows_updated > 0 || lastImport.rows_deleted > 0">
        · {{ lastImport.rows_updated }} mise{{ lastImport.rows_updated > 1 ? 's' : '' }} à jour,
        {{ lastImport.rows_deleted }} supprimée{{ lastImport.rows_deleted > 1 ? 's' : '' }}</template
      >
    </div>

    <section class="historical-import">
      <div class="historical-heading">
        <div>
          <h2>Fiches de traçabilité déjà produites</h2>
          <p>Ajoutez une photo ou un Excel. Le fichier sera transmis au service IA pour préparer un aperçu à valider.</p>
        </div>
      </div>
      <div
        class="historical-drop"
        :class="{ over: ficheDragOver, busy: analysingFiche }"
        role="button"
        tabindex="0"
        @click="ficheFileInput?.click()"
        @keydown.enter="ficheFileInput?.click()"
        @dragover.prevent="ficheDragOver = true"
        @dragleave="ficheDragOver = false"
        @drop.prevent="ficheDragOver = false; analyseFicheFile($event.dataTransfer?.files?.[0])"
      >
        <input
          ref="ficheFileInput"
          type="file"
          accept=".xlsx,.xls,.jpg,.jpeg,.png,.webp"
          class="file-input"
          @change="analyseFicheFile(($event.target as HTMLInputElement).files?.[0])"
        />
        <UploadCloud :size="22" />
        <strong>{{ analysingFiche ? 'Analyse IA de la fiche…' : 'Glissez une fiche ici ou cliquez pour la choisir' }}</strong>
        <span>XLSX, XLS, JPG, PNG ou WebP · 12 Mo maximum</span>
        <Loader v-if="analysingFiche" class="spin-inline" :size="18" />
      </div>
      <p v-if="ficheError" class="msg error historical-message">{{ ficheError }}</p>
      <p v-if="ficheSuccess" class="msg ok historical-message">{{ ficheSuccess }}</p>

      <div v-if="ficheAnalysis" class="fiche-preview">
        <div class="preview-heading">
          <div>
            <h3>Vérifiez les informations extraites</h3>
            <p>Aucune donnée ne sera enregistrée avant votre validation.</p>
          </div>
          <span>{{ ficheAnalysis.lignes.length }} ligne{{ ficheAnalysis.lignes.length > 1 ? 's' : '' }}</span>
        </div>
        <div class="preview-fields">
          <label>Référence
            <input v-model="ficheAnalysis.reference" type="text" placeholder="Référence" />
          </label>
          <label>Titre
            <input v-model="ficheAnalysis.titre" type="text" placeholder="Titre de la fiche" />
          </label>
          <label>Produit
            <input v-model="ficheAnalysis.produit" type="text" placeholder="Produit" />
          </label>
          <label>Date
            <input v-model="ficheAnalysis.date" type="date" />
          </label>
          <label>Quantité demandée (kg)
            <input
              type="number"
              min="0"
              step="any"
              :value="ficheAnalysis.quantite_demandee_kg ?? ''"
              placeholder="Utiliser le total des producteurs"
              @input="updateFicheQuantity(($event.target as HTMLInputElement).value)"
            />
          </label>
          <div class="preview-total">Total producteurs : <strong>{{ formatNum(quantiteProposee) }} kg</strong></div>
        </div>
        <div class="preview-lines">
          <div v-for="(line, index) in ficheAnalysis.lignes" :key="index" class="preview-line">
            <div class="preview-source">
              <strong>{{ line.producteur || `Producteur ${index + 1}` }}</strong>
              <span>{{ line.produit || ficheAnalysis.produit }}</span>
            </div>
            <label class="preview-producer">Producteur correspondant
              <select v-model="line.culture_id">
                <option :value="null">À associer…</option>
                <option v-for="producer in producteursPourLigne()" :key="producer.culture_id" :value="producer.culture_id">
                  {{ producer.id_interne || `#${producer.exploitation_id}` }} · {{ producer.producteur || 'Nom non renseigné' }} · culture #{{ producer.culture_id }}
                </option>
              </select>
            </label>
            <label class="preview-quantity">Quantité (kg)
              <input
                type="number"
                min="0"
                step="any"
                :value="line.quantite_kg"
                @input="updateLineQuantity(line, ($event.target as HTMLInputElement).value)"
              />
            </label>
          </div>
        </div>
        <ul v-if="ficheAnalysis.incertitudes.length" class="preview-warnings">
          <li v-for="(warning, index) in ficheAnalysis.incertitudes" :key="index">{{ warning }}</li>
        </ul>
        <p v-if="!ficheValide" class="preview-hint">
          Associez chaque producteur à une culture du produit choisi et vérifiez les quantités avant l’enregistrement.
        </p>
        <div class="preview-actions">
          <button class="action-btn" type="button" @click="ficheAnalysis = null">Annuler</button>
          <button class="action-btn action-btn-primary" type="button" :disabled="!ficheValide || savingFiche" @click="saveImportedFiche">
            <Loader v-if="savingFiche" class="spin-inline" :size="15" />
            {{ savingFiche ? 'Enregistrement…' : 'Valider et enregistrer la fiche' }}
          </button>
        </div>
      </div>
    </section>

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
      <button type="button" class="guide-link" @click="router.push({ name: 'Guide' })">
        Voir comment importer pas à pas
      </button>
    </section>

    <div
      v-if="workingExcel"
      class="workbook-overlay"
      role="presentation"
      @click.self="closeWorkingExcel"
      @keydown.esc="closeWorkingExcel"
    >
      <section class="workbook-modal" role="dialog" aria-modal="true" aria-labelledby="workbook-title">
        <header class="workbook-header">
          <div class="workbook-title">
            <FileSpreadsheet :size="19" />
            <div>
              <h2 id="workbook-title">Fichier Excel mis à jour</h2>
              <p>Aperçu du classeur actuellement conservé dans Vobox</p>
            </div>
          </div>
          <button class="workbook-close" type="button" aria-label="Fermer" @click="closeWorkingExcel">
            <X :size="18" />
          </button>
        </header>

        <div class="workbook-content">
          <article v-for="sheet in workingExcel.sheets" :key="sheet.name" class="workbook-sheet">
            <h3>{{ sheet.name }} <span>{{ sheet.rows }} lignes</span></h3>
            <div v-if="sheet.sample.length" class="workbook-table-wrap">
              <table class="workbook-table">
                <tbody>
                  <tr v-for="(row, rowIndex) in sheet.sample" :key="rowIndex">
                    <td v-for="(cell, cellIndex) in row" :key="cellIndex">{{ cell || '—' }}</td>
                  </tr>
                </tbody>
              </table>
            </div>
            <p v-else class="workbook-empty">Aucune ligne non vide à afficher.</p>
            <p v-if="sheet.rows > sheet.sample.length" class="workbook-note">
              Aperçu limité aux {{ sheet.sample.length }} premières lignes non vides.
            </p>
          </article>
          <p v-if="!workingExcel.sheets.length" class="workbook-empty">Aucune feuille à afficher.</p>
        </div>

        <footer class="workbook-footer">
          <button class="action-btn" type="button" @click="closeWorkingExcel">Fermer</button>
          <button class="action-btn action-btn-primary" type="button" @click="downloadWorkingExcel">
            <Download :size="15" /> Télécharger le XLSX
          </button>
        </footer>
      </section>
    </div>
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

.header-chip {
  flex-shrink: 0;
  padding: 8px 14px;
  background: var(--surface-container-high);
  border: none;
  border-radius: 999px;
  color: var(--on-surface);
  font-size: 13px;
  font-weight: 600;
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
  border: 1.5px dashed var(--outline);
  border-radius: var(--shape-xl);
  background: var(--surface-container-low);
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
  width: 56px;
  height: 56px;
  border-radius: var(--shape-lg);
  background: var(--primary-container);
  color: var(--on-primary-container);
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
  background: color-mix(in srgb, var(--success, #16a34a) 12%, transparent);
  color: var(--success, var(--green-ok, #16a34a));
  border: 1px solid color-mix(in srgb, var(--success, #16a34a) 25%, transparent);
}

.msg.error {
  background: rgba(220, 38, 38, 0.1);
  color: var(--error);
  border: 1px solid rgba(220, 38, 38, 0.25);
}

.debug-box {
  flex-shrink: 0;
  margin: 12px 12px 0;
  padding: 12px 14px;
  background: var(--surface-container);
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
  background: var(--surface-container-high);
  white-space: pre-wrap;
  word-break: break-word;
  font-family: 'Courier New', monospace;
}

.historical-import {
  flex-shrink: 0;
  margin: 14px 12px 0;
  padding: 16px;
  border: 1px solid var(--border);
  border-radius: var(--shape-lg);
  background: var(--surface-container-low);
}

.historical-heading h2,
.preview-heading h3 {
  margin: 0;
  color: var(--text-h);
  font-size: 16px;
}

.historical-heading p,
.preview-heading p {
  margin: 4px 0 0;
  color: var(--text);
  font-size: 12.5px;
}

.historical-drop {
  min-height: 88px;
  margin-top: 12px;
  padding: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 10px;
  flex-wrap: wrap;
  border: 1.5px dashed var(--outline);
  border-radius: 12px;
  color: var(--accent);
  cursor: pointer;
  text-align: center;
  transition: border-color 0.2s ease, background 0.2s ease;
}

.historical-drop:hover,
.historical-drop.over {
  border-color: var(--accent);
  background: var(--accent-bg);
}

.historical-drop strong {
  color: var(--text-h);
  font-size: 13px;
}

.historical-drop span {
  color: var(--text);
  font-size: 11px;
}

.historical-drop.busy {
  opacity: 0.7;
  pointer-events: none;
}

.historical-message {
  margin: 10px 0 0;
}

.fiche-preview {
  margin-top: 14px;
  padding-top: 14px;
  border-top: 1px solid var(--border);
}

.preview-heading,
.preview-actions {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
}

.preview-heading > span {
  flex-shrink: 0;
  padding: 4px 9px;
  border-radius: 999px;
  background: var(--surface-container-high);
  color: var(--text);
  font-size: 11px;
}

.preview-fields {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
  gap: 10px;
  margin-top: 12px;
}

.preview-fields label,
.preview-line label {
  display: flex;
  flex-direction: column;
  gap: 5px;
  color: var(--text);
  font-size: 11px;
}

.preview-fields input,
.preview-line input,
.preview-line select {
  min-width: 0;
  width: 100%;
  padding: 7px 9px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--surface-container);
  color: var(--text-h);
  font: inherit;
  font-size: 12px;
}

.preview-total {
  align-self: end;
  padding: 7px 0;
  color: var(--text);
  font-size: 12px;
}

.preview-total strong {
  color: var(--text-h);
}

.preview-lines {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-top: 14px;
}

.preview-line {
  display: grid;
  grid-template-columns: minmax(140px, 1fr) minmax(200px, 2fr) minmax(120px, 0.8fr);
  align-items: end;
  gap: 10px;
  padding: 10px;
  border-radius: 10px;
  background: var(--surface-container);
}

.preview-source {
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 0;
  padding-bottom: 5px;
}

.preview-source strong {
  overflow-wrap: anywhere;
  color: var(--text-h);
  font-size: 12px;
}

.preview-source span {
  color: var(--text);
  font-size: 11px;
}

.preview-warnings {
  margin: 10px 0 0;
  padding: 9px 12px 9px 28px;
  border-radius: 9px;
  background: color-mix(in srgb, var(--warning, #d97706) 10%, transparent);
  color: var(--text);
  font-size: 12px;
}

.preview-hint {
  margin: 10px 0 0;
  color: var(--text);
  font-size: 11px;
}

.preview-actions {
  justify-content: flex-end;
  margin-top: 12px;
}

.preview-actions button:disabled {
  opacity: 0.55;
  cursor: not-allowed;
}

@media (max-width: 720px) {
  .preview-line {
    grid-template-columns: 1fr;
  }

  .preview-actions {
    align-items: stretch;
    flex-direction: column-reverse;
  }
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
  border-color: var(--error);
  color: var(--error);
}

.action-btn:disabled {
  opacity: 0.5;
  cursor: default;
}

.spin-inline {
  animation: spin 0.9s linear infinite;
}

.action-btn-primary {
  border-color: var(--accent);
  background: var(--accent);
  color: var(--on-primary, white);
}

.action-btn-primary:hover:not(:disabled) {
  color: var(--on-primary, white);
  filter: brightness(0.96);
}

.workbook-overlay {
  position: fixed;
  z-index: 1000;
  inset: 0;
  display: grid;
  place-items: center;
  padding: 20px;
  background: rgba(15, 23, 42, 0.48);
}

.workbook-modal {
  display: flex;
  flex-direction: column;
  width: min(900px, 100%);
  max-height: min(82vh, 760px);
  overflow: hidden;
  border: 1px solid var(--border);
  border-radius: 18px;
  background: var(--bg);
  color: var(--text-h);
  box-shadow: 0 24px 70px rgba(15, 23, 42, 0.24);
}

.workbook-header,
.workbook-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 16px 18px;
  border-bottom: 1px solid var(--border);
}

.workbook-title {
  display: flex;
  align-items: center;
  gap: 11px;
  color: var(--accent);
}

.workbook-title h2 {
  margin: 0;
  color: var(--text-h);
  font-size: 17px;
}

.workbook-title p {
  margin: 3px 0 0;
  color: var(--text);
  font-size: 12px;
}

.workbook-close {
  display: grid;
  flex: 0 0 auto;
  place-items: center;
  width: 34px;
  height: 34px;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: var(--bg);
  color: var(--text);
  cursor: pointer;
}

.workbook-content {
  display: flex;
  flex-direction: column;
  gap: 14px;
  overflow: auto;
  padding: 16px 18px;
}

.workbook-sheet {
  min-width: 0;
  padding: 12px;
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--surface-container-low);
}

.workbook-sheet h3 {
  display: flex;
  align-items: baseline;
  gap: 8px;
  margin: 0 0 9px;
  font-size: 14px;
}

.workbook-sheet h3 span,
.workbook-note,
.workbook-empty {
  color: var(--text);
  font-size: 12px;
  font-weight: 400;
}

.workbook-table-wrap {
  overflow: auto;
  max-width: 100%;
}

.workbook-table {
  border-collapse: collapse;
  font-size: 12px;
}

.workbook-table td {
  max-width: 260px;
  padding: 6px 9px;
  border: 1px solid var(--border);
  white-space: nowrap;
}

.workbook-table tr:first-child {
  background: var(--surface-container-high);
  font-weight: 600;
}

.workbook-note,
.workbook-empty {
  margin: 8px 0 0;
}

.workbook-footer {
  justify-content: flex-end;
  border-top: 1px solid var(--border);
  border-bottom: 0;
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
  padding: 12px;
  background: var(--surface-container-low);
  border: none;
  border-radius: var(--shape-md);
  box-shadow: var(--elev-1);
  cursor: pointer;
  transition: box-shadow var(--motion);
}

.tile:hover {
  box-shadow: var(--elev-2);
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
  color: var(--success, #16a34a);
  background: color-mix(in srgb, var(--success, #16a34a) 10%, transparent);
}

.badge-unite {
  color: var(--info, #2563eb);
  background: color-mix(in srgb, var(--info, #2563eb) 10%, transparent);
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

.guide-link {
  border: none;
  background: transparent;
  color: var(--accent);
  font-size: 13.5px;
  font-weight: 700;
  cursor: pointer;
  padding: 6px 12px;
  border-radius: 999px;
}

.guide-link:hover {
  background: var(--accent-bg);
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
