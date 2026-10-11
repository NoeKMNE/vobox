import { invoke } from '@tauri-apps/api/core'

export interface User {
  id: number
  name: string
  email: string
  onboarding_done: boolean
  avatar: string | null
}

export interface Session {
  token: string
  user: User
}

const TOKEN_KEY = 'Vobox_token'

export function getToken(): string | null {
  return localStorage.getItem(TOKEN_KEY)
}

export function setToken(token: string) {
  localStorage.setItem(TOKEN_KEY, token)
}

export function clearToken() {
  localStorage.removeItem(TOKEN_KEY)
}

export function register(name: string, email: string, password: string, telemetryConsent = false): Promise<Session> {
  return invoke<Session>('register', { name, email, password, telemetryConsent })
}

export function getTelemetryConsent(): Promise<boolean> {
  return invoke<boolean>('get_telemetry_consent')
}

export function setTelemetryConsent(enabled: boolean): Promise<void> {
  return invoke('set_telemetry_consent', { enabled })
}

export function login(email: string, password: string): Promise<Session> {
  return invoke<Session>('login', { email, password })
}

export function logout(token: string): Promise<void> {
  return invoke('logout', { token })
}

export function getSession(token: string): Promise<User> {
  return invoke<User>('get_session', { token })
}

export function completeOnboarding(token: string): Promise<void> {
  return invoke('complete_onboarding', { token })
}

export function updateProfile(token: string, name: string, email: string): Promise<User> {
  return invoke<User>('update_profile', { token, name, email })
}

export function updateAvatar(token: string, avatar: string | null): Promise<User> {
  return invoke<User>('update_avatar', { token, avatar })
}

export interface Exploitation {
  id: number
  id_interne: string | null
  id_national: string | null
  village: string | null
  district: string | null
  region_inspection: string | null
  superficie_total: number | null
  type_exploitation: string | null
  nb_unites: number | null
  nb_cultures: number | null
  operateur_prenom: string | null
  operateur_nom: string | null
  operateur_telephone: string | null
  operateur_id_national: string | null
  operateur_genre: string | null
  operateur_annee_nais: number | null
  proprietaire_prenom: string | null
  proprietaire_nom: string | null
  proprietaire_telephone: string | null
  proprietaire_id_national: string | null
  proprietaire_genre: string | null
  travailleurs_permanents: number | null
  travailleurs_temporaires: number | null
  inspecteur_nom: string | null
  inspecteur_annee: number | null
  inspecteur_mois: number | null
  inspecteur_jour: number | null
  cultures_count: number
  unites_count: number
  created_at: string
}

export interface ImportResult {
  exploitations_added: number
  cultures_added: number
  unites_added: number
  rows_updated: number
  rows_deleted: number
  exploitations: Exploitation[]
  sheets: { name: string; rows: number; sample: string[][] }[]
}

export function importExcel(token: string, fileB64: string, fileName: string): Promise<ImportResult> {
  return invoke<ImportResult>('import_excel', { token, fileB64, fileName })
}

export function listExploitations(token: string): Promise<Exploitation[]> {
  return invoke<Exploitation[]>('list_exploitations', { token })
}

export type SearchCategory =
  | 'all'
  | 'operateur'
  | 'localisation'
  | 'culture'
  | 'unite'
  | 'identifiant'
  | 'inspecteur'

export function searchExploitations(
  token: string,
  query: string,
  category: SearchCategory = 'all'
): Promise<Exploitation[]> {
  return invoke<Exploitation[]>('search_exploitations', { token, query, category })
}

export interface Culture {
  id: number
  id_interne: string | null
  produit: string | null
  variete: string | null
  superficie_ha: number | null
  recolte_estimee: number | null
  recolte_precedente: number | null
  volume_vendu: number | null
}

export interface Unite {
  id: number
  id_interne: string | null
  identifiant_unite: string | null
  superficie_ha: number | null
  latitude: number | null
  longitude: number | null
}

export interface ExploitationDetail {
  exploitation: Exploitation
  cultures: Culture[]
  unites: Unite[]
}

export function getExploitation(token: string, id: number): Promise<ExploitationDetail> {
  return invoke<ExploitationDetail>('get_exploitation', { token, id })
}

export function exportBackup(token: string): Promise<string> {
  return invoke<string>('export_backup', { token })
}

export function resetData(token: string): Promise<number> {
  return invoke<number>('reset_data', { token })
}

export interface ProduitStock {
  produit: string
  quantite_estimee_kg: number
  quantite_allouee_kg: number
  quantite_livrable_kg: number
  nb_exploitations: number
}

export function produitsDisponibles(token: string): Promise<ProduitStock[]> {
  return invoke<ProduitStock[]>('produits_disponibles', { token })
}

export interface FicheItem {
  culture_id: number
  quantite_kg: number
  exploitation: Exploitation
  unites: Unite[]
}

export interface ProducteurFiche {
  culture_id: number
  exploitation: Exploitation
  quantite_livrable_kg: number
  participe_semaine: boolean
}

export function getInclureNouveaux(): Promise<boolean> {
  return invoke<boolean>('get_inclure_nouveaux')
}

export function setInclureNouveaux(enabled: boolean): Promise<void> {
  return invoke('set_inclure_nouveaux', { enabled })
}

export interface Fiche {
  id: number
  reference: string
  titre: string
  produit: string
  quantite_demandee_kg: number
  quantite_attribuee_kg: number
  nb_sacs: number
  statut: string
  created_at: string
  items: FicheItem[]
}

export interface ResteProduit {
  produit: string
  estime_kg: number
  alloue_kg: number
  reste_kg: number
  nb_producteurs: number
}

export interface ResteProducteur {
  exploitation_id: number
  id_interne: string | null
  operateur: string
  produit: string
  estime_kg: number
  alloue_kg: number
  reste_kg: number
}

export interface RapportSemestre {
  semestre: string
  annee: number
  debut: string
  fin: string
  estime_kg: number
  alloue_kg: number
  reste_kg: number
  par_produit: ResteProduit[]
  par_producteur: ResteProducteur[]
  cloture_le: string | null
}

export function rapportSemestre(token: string, semestre: string, annee: number): Promise<RapportSemestre> {
  return invoke<RapportSemestre>('rapport_semestre', { token, semestre, annee })
}

export function cloturerSemestre(token: string, semestre: string, annee: number): Promise<number> {
  return invoke<number>('cloturer_semestre', { token, semestre, annee })
}

export interface FicheSummary {
  id: number
  reference: string
  titre: string
  produit: string
  quantite_demandee_kg: number
  quantite_attribuee_kg: number
  nb_producteurs: number
  statut: string
  created_at: string
}

export function genererFiche(
  token: string,
  produit: string,
  quantiteKg: number,
  titre?: string
): Promise<Fiche> {
  return invoke<Fiche>('generer_fiche', { token, produit, quantiteKg, titre })
}

export function producteursPourFiche(token: string, produit: string): Promise<ProducteurFiche[]> {
  return invoke<ProducteurFiche[]>('producteurs_pour_fiche', { token, produit })
}

export function genererFicheManuelle(
  token: string,
  produit: string,
  allocations: Array<[number, number]>,
  titre?: string
): Promise<Fiche> {
  return invoke<Fiche>('generer_fiche_manuelle', { token, produit, allocations, titre })
}

export function listFiches(token: string): Promise<FicheSummary[]> {
  return invoke<FicheSummary[]>('list_fiches', { token })
}

export function getFiche(token: string, id: number): Promise<Fiche> {
  return invoke<Fiche>('get_fiche', { token, id })
}

export function supprimerFiche(token: string, id: number): Promise<void> {
  return invoke('supprimer_fiche', { token, id })
}

export function setFicheNbSacs(token: string, id: number, nbSacs: number): Promise<Fiche> {
  return invoke<Fiche>('set_fiche_nb_sacs', { token, id, nbSacs })
}

export interface ParticipationProducteur {
  fiche_id: number
  reference: string
  titre: string
  produit: string
  statut: string
  created_at: string
  quantite_kg: number
}

export interface HistoriqueProducteur {
  total_kg: number
  nb_participations: number
  participations: ParticipationProducteur[]
}

export function historiqueProducteur(token: string, exploitationId: number): Promise<HistoriqueProducteur> {
  return invoke<HistoriqueProducteur>('historique_producteur', { token, exploitationId })
}

export interface ResumeParticipation {
  exploitation_id: number
  nb_participations: number
  total_kg: number
}

export function producteursParticipants(token: string): Promise<ResumeParticipation[]> {
  return invoke<ResumeParticipation[]>('producteurs_participants', { token })
}