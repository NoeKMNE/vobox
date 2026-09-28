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

const TOKEN_KEY = 'agrobox_token'

export function getToken(): string | null {
  return localStorage.getItem(TOKEN_KEY)
}

export function setToken(token: string) {
  localStorage.setItem(TOKEN_KEY, token)
}

export function clearToken() {
  localStorage.removeItem(TOKEN_KEY)
}

export function register(name: string, email: string, password: string): Promise<Session> {
  return invoke<Session>('register', { name, email, password })
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
  quantite_disponible_kg: number
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

export interface Fiche {
  id: number
  reference: string
  titre: string
  produit: string
  quantite_demandee_kg: number
  quantite_attribuee_kg: number
  statut: string
  created_at: string
  items: FicheItem[]
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

export function listFiches(token: string): Promise<FicheSummary[]> {
  return invoke<FicheSummary[]>('list_fiches', { token })
}

export function getFiche(token: string, id: number): Promise<Fiche> {
  return invoke<Fiche>('get_fiche', { token, id })
}