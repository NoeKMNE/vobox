use std::collections::HashSet;
use std::path::Path;
use std::sync::Mutex;

use argon2::password_hash::rand_core::{OsRng, RngCore};
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

use crate::excel::{ParsedWorkbook, SheetDump};

/// Part de l'estimation annuelle allouée à la période S1 (1 sept → fin fév).
const S1_RATIO: f64 = 0.7;
/// Part de l'estimation annuelle allouée à la période S2 (1 mars → fin août).
const S2_RATIO: f64 = 0.3;
/// Semaines d'un semestre : mensuel = semestriel / 6, hebdo = mensuel / 4 → semestriel / 24.
const WEEKS_PER_SEMESTER: f64 = 24.0;

/// Nombre de jours écoulés depuis 1970-01-01 (calendrier civil).
fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = y - if m <= 2 { 1 } else { 0 };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m as i64 + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// Convertit un nombre de jours depuis 1970-01-01 en date civile (année, mois, jour).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// Date « aujourd'hui » en UTC (année, mois, jour).
fn today_utc() -> (i64, u32, u32) {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    civil_from_days((secs / 86_400) as i64)
}

/// Période de l'année pour une date : 1 = S1 (sept → fév), 2 = S2 (mars → août).
fn period_of(_y: i64, m: u32) -> u8 {
    if (3..=8).contains(&m) { 2 } else { 1 }
}

/// Part de l'estimation annuelle applicable à la période.
fn period_ratio(period: u8) -> f64 {
    if period == 1 { S1_RATIO } else { S2_RATIO }
}

/// Estimation hebdomadaire dérivée de l'estimation annuelle d'une culture.
fn weekly_estimate(annual_kg: f64, period: u8) -> f64 {
    annual_kg * period_ratio(period) / WEEKS_PER_SEMESTER
}

/// Début de journée il y a `days` jours au format « YYYY-MM-DD » (UTC).
fn date_start_days_ago(days: i64) -> String {
    let (y, m, d) = today_utc();
    let (ay, am, ad) = civil_from_days(days_from_civil(y, m, d) - days);
    format!("{ay:04}-{am:02}-{ad:02}")
}

/// Début de la semaine civile courante (lundi) au format « YYYY-MM-DD » (UTC).
/// Sert à l'indicateur « participé cette semaine » (la carence, elle, reste à 14 jours).
fn current_week_start() -> String {
    let (y, m, d) = today_utc();
    let days = days_from_civil(y, m, d);
    let from_monday = ((days + 3) % 7 + 7) % 7; // 1970-01-01 (jeudi) → 3 jours après lundi
    let (wy, wm, wd) = civil_from_days(days - from_monday);
    format!("{wy:04}-{wm:02}-{wd:02}")
}

/// Semestre d'une date : S1 = sept→fév (année = année du 1er sept),
/// S2 = mars→août (année = année civile).
#[cfg(test)]
fn semestre_pour_date(y: i64, m: u32) -> (&'static str, i64) {
    if (3..=8).contains(&m) {
        ("S2", y)
    } else if m >= 9 {
        ("S1", y)
    } else {
        ("S1", y - 1)
    }
}

/// Bornes [début, fin) d'un semestre au format « YYYY-MM-DD ».
fn bornes_semestre(semestre: &str, annee: i64) -> Result<(String, String), String> {
    match semestre {
        "S1" => Ok((format!("{annee:04}-09-01"), format!("{:04}-03-01", annee + 1))),
        "S2" => Ok((format!("{annee:04}-03-01"), format!("{annee:04}-09-01"))),
        _ => Err("Semestre invalide (attendu S1 ou S2)".into()),
    }
}

fn ratio_semestre(semestre: &str) -> Result<f64, String> {
    match semestre {
        "S1" => Ok(S1_RATIO),
        "S2" => Ok(S2_RATIO),
        _ => Err("Semestre invalide (attendu S1 ou S2)".into()),
    }
}

fn valider_annee(annee: i64) -> Result<(), String> {
    if (2000..=2100).contains(&annee) {
        Ok(())
    } else {
        Err("Année invalide".into())
    }
}

/// Semaines à cumuler pour un semestre : écoulées si courant, semestre complet
/// si passé, zéro si futur.
fn semaines_pour_semestre(debut: &str, fin: &str) -> f64 {
    let (y, m, d) = today_utc();
    let auj = format!("{y:04}-{m:02}-{d:02}");
    if auj.as_str() < debut {
        0.0
    } else if auj.as_str() >= fin {
        WEEKS_PER_SEMESTER
    } else {
        saison_courante().1
    }
}

/// Saison courante : S1 = 1er sept → fin fév, S2 = 1er mars → fin août.
/// Retourne (début_saison « YYYY-MM-DD », semaines cumulées).
/// Le stock livrable cumule l'estimation hebdomadaire semaine par semaine depuis
/// le début de la saison (semaine courante incluse), plafonné à un semestre
/// complet (`WEEKS_PER_SEMESTER`). Au changement de saison, le cumul repart de zéro.
fn saison_courante() -> (String, f64) {
    let (y, m, d) = today_utc();
    let (sy, sm) = if (3..=8).contains(&m) {
        (y, 3)
    } else if m >= 9 {
        (y, 9)
    } else {
        (y - 1, 9)
    };
    let debut = format!("{sy:04}-{sm:02}-01");
    let jours = days_from_civil(y, m, d) - days_from_civil(sy, sm, 1);
    let semaines = ((jours / 7) + 1).clamp(1, WEEKS_PER_SEMESTER as i64) as f64;
    (debut, semaines)
}

/// Marge aléatoire appliquée aux quantités par producteur (±5 %).
const MARGE_MIN: f64 = 0.95;
const MARGE_MAX: f64 = 1.05;

/// Générateur pseudo-aléatoire minimal (SplitMix64).
/// Évite une dépendance `rand` pour ce besoin ciblé : la graine vient de l'OS
/// en production (`from_entropy`), et peut être fixée dans les tests (`seeded`).
struct MargeRng(u64);

impl MargeRng {
    fn from_entropy() -> Self {
        let mut buf = [0u8; 8];
        OsRng.fill_bytes(&mut buf);
        Self(u64::from_le_bytes(buf))
    }

    #[cfg(test)]
    fn seeded(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }

    /// Facteur de marge dans [MARGE_MIN, MARGE_MAX].
    fn facteur_marge(&mut self) -> f64 {
        MARGE_MIN + (self.next_u64() as f64 / u64::MAX as f64) * (MARGE_MAX - MARGE_MIN)
    }
}

/// Répartition gloutonne avec marge aléatoire ±5 % par producteur :
/// chaque part est calculée sur la disponibilité margée (`dispo × facteur`),
/// plafonnée au reste à couvrir. Le total attribué ne dépasse jamais la demande.
/// Retourne (allocations `(culture_id, exploitation_id, quantite)`, total).
///
/// Conservée pour les tests du générateur de marge : la production utilise
/// `repartir_quantites_pleine` (commandes honorées intégralement).
#[cfg(test)]
fn repartir_quantites(
    lignes: &[(i64, i64, f64)],
    quantite_kg: f64,
    rng: &mut MargeRng,
) -> (Vec<(i64, i64, f64)>, f64) {
    let mut reste = quantite_kg;
    let mut total = 0.0;
    let mut out = Vec::new();
    for (culture_id, exploitation_id, dispo) in lignes {
        if reste <= 0.0 {
            break;
        }
        let part = (dispo * rng.facteur_marge()).min(reste);
        if part <= 0.0 {
            continue;
        }
        reste -= part;
        total += part;
        out.push((*culture_id, *exploitation_id, part));
    }
    (out, total)
}

/// Variante Vobox : toutes les commandes sont honorées en intégralité.
/// 1. Remplissage glouton : chaque producteur livre au plus sa disponibilité
///    (±5 %), dans l'ordre décroissant des capacités — une commande dans le
///    livrable cumulé est donc servie sans la concentrer sur 1-2 producteurs
///    qui seuls détiennent le stock.
/// 2. Surplus éventuel (demande au-delà du livrable cumulé) : réparti au
///    prorata des estimations initiales (`poids`), pour couvrir exactement la
///    demande. Retourne (allocations, total ≈ demande).
fn repartir_quantites_pleine(
    lignes: &[(i64, i64, f64, f64)],
    poids: &[f64],
    total_poids: f64,
    quantite_kg: f64,
    rng: &mut MargeRng,
) -> (Vec<(i64, i64, f64)>, f64) {
    if lignes.is_empty() {
        return (Vec::new(), 0.0);
    }
    let n = lignes.len();
    let mut quant = vec![0.0_f64; n];
    let mut reste = quantite_kg;
    // 1) Glouton : chaque producteur livre au plus sa disponibilité (±5 %).
    for (i, (_, _, dispo, _)) in lignes.iter().enumerate() {
        if reste <= 1e-9 {
            break;
        }
        let part = (dispo.max(0.0) * rng.facteur_marge()).min(reste);
        if part <= 0.0 {
            continue;
        }
        quant[i] = part;
        reste -= part;
    }
    // 2) Surplus au-delà du livrable cumulé : au prorata des estimations
    //    initiales, normalisé pour couvrir exactement le reste.
    if reste > 1e-9 {
        let denom = if total_poids > 0.0 { total_poids } else { n as f64 };
        let mut brut = 0.0;
        let mut parts_brut = vec![0.0_f64; n];
        for (i, w) in poids.iter().enumerate() {
            let wi = if total_poids > 0.0 { *w } else { 1.0 };
            if wi <= 0.0 {
                continue;
            }
            let part = (wi / denom) * rng.facteur_marge();
            if part <= 0.0 {
                continue;
            }
            brut += part;
            parts_brut[i] = part;
        }
        if brut > 0.0 {
            let facteur = reste / brut;
            for (i, p) in parts_brut.iter().enumerate() {
                if *p > 0.0 {
                    quant[i] += *p * facteur;
                }
            }
        }
    }
    let allocations: Vec<(i64, i64, f64)> = lignes
        .iter()
        .zip(quant.iter())
        .filter(|(_, q)| **q > 0.0)
        .map(|((cid, eid, _, _), q)| (*cid, *eid, *q))
        .collect();
    let total: f64 = allocations.iter().map(|(_, _, q)| q).sum();
    (allocations, total)
}

#[derive(Serialize, Clone, Debug)]
pub struct Exploitation {
    pub id: i64,
    pub id_interne: Option<String>,
    pub id_national: Option<String>,
    pub village: Option<String>,
    pub district: Option<String>,
    pub region_inspection: Option<String>,
    pub superficie_total: Option<f64>,
    pub type_exploitation: Option<String>,
    pub nb_unites: Option<i64>,
    pub nb_cultures: Option<i64>,
    pub operateur_prenom: Option<String>,
    pub operateur_nom: Option<String>,
    pub operateur_telephone: Option<String>,
    pub operateur_id_national: Option<String>,
    pub operateur_genre: Option<String>,
    pub operateur_annee_nais: Option<i64>,
    pub proprietaire_prenom: Option<String>,
    pub proprietaire_nom: Option<String>,
    pub proprietaire_telephone: Option<String>,
    pub proprietaire_id_national: Option<String>,
    pub proprietaire_genre: Option<String>,
    pub travailleurs_permanents: Option<i64>,
    pub travailleurs_temporaires: Option<i64>,
    pub inspecteur_nom: Option<String>,
    pub inspecteur_annee: Option<i64>,
    pub inspecteur_mois: Option<i64>,
    pub inspecteur_jour: Option<i64>,
    pub cultures_count: u64,
    pub unites_count: u64,
    pub created_at: String,
}

#[derive(Serialize, Clone)]
pub struct ImportResult {
    pub exploitations_added: usize,
    pub cultures_added: usize,
    pub unites_added: usize,
    pub rows_updated: usize,
    pub rows_deleted: usize,
    pub exploitations: Vec<Exploitation>,
    pub sheets: Vec<SheetDump>,
}

#[derive(Serialize, Clone)]
pub struct Culture {
    pub id: i64,
    pub id_interne: Option<String>,
    pub produit: Option<String>,
    pub variete: Option<String>,
    pub superficie_ha: Option<f64>,
    pub recolte_estimee: Option<f64>,
    pub recolte_precedente: Option<f64>,
    pub volume_vendu: Option<f64>,
}

#[derive(Serialize, Clone, Debug)]
pub struct Unite {
    pub id: i64,
    pub id_interne: Option<String>,
    pub identifiant_unite: Option<String>,
    pub superficie_ha: Option<f64>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

#[derive(Serialize, Clone)]
pub struct ExploitationDetail {
    pub exploitation: Exploitation,
    pub cultures: Vec<Culture>,
    pub unites: Vec<Unite>,
}

#[derive(Serialize, Clone)]
pub struct ExportExploitation {
    pub exploitation: Exploitation,
    pub cultures: Vec<Culture>,
    pub unites: Vec<Unite>,
}

#[derive(Serialize, Clone)]
pub struct ExportData {
    pub version: u32,
    pub exported_at: String,
    pub exploitations: Vec<ExportExploitation>,
}

#[derive(Serialize, Clone, Debug)]
pub struct ProduitStock {
    pub produit: String,
    pub quantite_estimee_kg: f64,
    pub quantite_allouee_kg: f64,
    /// Stock cumulé de la saison (estimation saisonnière − alloué saisonnier).
    pub quantite_livrable_kg: f64,
    pub nb_exploitations: u64,
}

#[derive(Serialize, Clone, Debug)]
pub struct FicheItem {
    pub culture_id: i64,
    pub quantite_kg: f64,
    pub exploitation: Exploitation,
    pub unites: Vec<Unite>,
}

#[derive(Serialize, Clone, Debug)]
pub struct ProducteurFiche {
    pub culture_id: i64,
    pub exploitation: Exploitation,
    /// Quantité livrable cumulée de la saison pour ce producteur.
    pub quantite_livrable_kg: f64,
    pub participe_semaine: bool,
}

#[derive(Serialize, Clone, Debug)]
pub struct ResteProduit {
    pub produit: String,
    pub estime_kg: f64,
    pub alloue_kg: f64,
    pub reste_kg: f64,
    pub nb_producteurs: usize,
}

#[derive(Serialize, Clone, Debug)]
pub struct ResteProducteur {
    pub exploitation_id: i64,
    pub id_interne: Option<String>,
    pub operateur: String,
    pub produit: String,
    pub estime_kg: f64,
    pub alloue_kg: f64,
    pub reste_kg: f64,
}

#[derive(Serialize, Clone, Debug)]
pub struct RapportSemestre {
    pub semestre: String,
    pub annee: i64,
    pub debut: String,
    pub fin: String,
    pub estime_kg: f64,
    pub alloue_kg: f64,
    pub reste_kg: f64,
    pub par_produit: Vec<ResteProduit>,
    pub par_producteur: Vec<ResteProducteur>,
    pub cloture_le: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
pub struct ParticipationProducteur {
    pub fiche_id: i64,
    pub reference: String,
    pub titre: String,
    pub produit: String,
    pub statut: String,
    pub created_at: String,
    pub quantite_kg: f64,
}

#[derive(Serialize, Clone, Debug)]
pub struct HistoriqueProducteur {
    pub total_kg: f64,
    pub nb_participations: usize,
    pub participations: Vec<ParticipationProducteur>,
}

#[derive(Serialize, Clone, Debug)]
pub struct ResumeParticipation {
    pub exploitation_id: i64,
    pub nb_participations: usize,
    pub total_kg: f64,
}

#[derive(Serialize, Clone, Debug)]
pub struct GlobalStats {
    pub nb_exploitations: i64,
    pub nb_unites: i64,
    pub nb_cultures: i64,
    pub superficie_total_ha: f64,
    pub nb_producteurs: i64,
    pub nb_fiches: i64,
    pub nb_allocations: i64,
    pub quantite_allouee_kg: f64,
    pub produit_plus_cultive: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
pub struct Fiche {
    pub id: i64,
    pub reference: String,
    pub titre: String,
    pub produit: String,
    pub quantite_demandee_kg: f64,
    pub quantite_attribuee_kg: f64,
    pub statut: String,
    pub created_at: String,
    pub items: Vec<FicheItem>,
}

#[derive(Serialize, Clone, Debug)]
pub struct FicheSummary {
    pub id: i64,
    pub reference: String,
    pub titre: String,
    pub produit: String,
    pub quantite_demandee_kg: f64,
    pub quantite_attribuee_kg: f64,
    pub nb_producteurs: usize,
    pub statut: String,
    pub created_at: String,
}

#[derive(Serialize, Clone)]
pub struct User {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub onboarding_done: bool,
    pub avatar: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct Session {
    pub token: String,
    pub user: User,
}

pub struct AiSession {
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SearchCategory {
    All,
    Operateur,
    Localisation,
    Culture,
    Unite,
    Identifiant,
    Inspecteur,
}

impl SearchCategory {
    fn column_filter(self) -> Option<&'static str> {
        match self {
            SearchCategory::All => None,
            SearchCategory::Operateur => Some("operateur"),
            SearchCategory::Localisation => Some("localisation"),
            SearchCategory::Culture => Some("culture"),
            SearchCategory::Unite => Some("unite"),
            SearchCategory::Identifiant => Some("identifiant"),
            SearchCategory::Inspecteur => Some("inspecteur"),
        }
    }
}

fn column_names(conn: &Connection, table: &str) -> rusqlite::Result<Vec<String>> {
    let sql = format!("PRAGMA table_info({table})");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], |r| r.get::<_, String>(1))?;
    rows.collect()
}

fn ensure_column(conn: &Connection, table: &str, column: &str, decl: &str) -> rusqlite::Result<()> {
    if !column_names(conn, table)?.iter().any(|n| n == column) {
        conn.execute_batch(&format!("ALTER TABLE {table} ADD COLUMN {column} {decl};"))?;
    }
    Ok(())
}

fn rename_column_if_exists(conn: &Connection, table: &str, old: &str, new: &str) -> rusqlite::Result<()> {
    let cols = column_names(conn, table)?;
    if cols.iter().any(|n| n == old) && !cols.iter().any(|n| n == new) {
        conn.execute_batch(&format!("ALTER TABLE {table} RENAME COLUMN {old} TO {new};"))?;
    }
    Ok(())
}

pub struct Db(pub Mutex<Connection>);

impl Db {
    pub fn init(db_path: &Path) -> rusqlite::Result<Self> {
        let conn = Connection::open(db_path)?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             CREATE TABLE IF NOT EXISTS users (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 name TEXT NOT NULL,
                 email TEXT NOT NULL UNIQUE,
                 password_hash TEXT NOT NULL,
                 onboarding_done INTEGER NOT NULL DEFAULT 0,
                 avatar TEXT,
                 created_at TEXT NOT NULL DEFAULT (datetime('now'))
             );
CREATE TABLE IF NOT EXISTS sessions (
                  id INTEGER PRIMARY KEY AUTOINCREMENT,
                  user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                  token TEXT NOT NULL UNIQUE,
                  created_at TEXT NOT NULL DEFAULT (datetime('now'))
              );
              CREATE TABLE IF NOT EXISTS settings (
                  key TEXT PRIMARY KEY,
                  value TEXT NOT NULL
              );
              CREATE TABLE IF NOT EXISTS ai_sessions (
                  user_id INTEGER PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
                  access_token TEXT NOT NULL,
                  refresh_token TEXT NOT NULL,
                  updated_at TEXT NOT NULL DEFAULT (datetime('now'))
              );
             CREATE TABLE IF NOT EXISTS exploitations (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                 id_interne TEXT,
                 id_national TEXT,
                 village TEXT,
                 district TEXT,
                 region_inspection TEXT,
                 superficie_total REAL,
                 type_exploitation TEXT,
                 nb_unites INTEGER,
                 nb_cultures INTEGER,
                 operateur_prenom TEXT,
                 operateur_nom TEXT,
                 operateur_telephone TEXT,
                 operateur_id_national TEXT,
                 operateur_genre TEXT,
                 operateur_annee_nais INTEGER,
                 proprietaire_prenom TEXT,
                 proprietaire_nom TEXT,
                 proprietaire_telephone TEXT,
                 proprietaire_id_national TEXT,
                 proprietaire_genre TEXT,
                 travailleurs_permanents INTEGER,
                 travailleurs_temporaires INTEGER,
                 inspecteur_nom TEXT,
                 inspecteur_annee INTEGER,
                 inspecteur_mois INTEGER,
                 inspecteur_jour INTEGER,
                 created_at TEXT NOT NULL DEFAULT (datetime('now')),
                 UNIQUE(user_id, id_interne)
             );
             CREATE TABLE IF NOT EXISTS cultures_certifiees (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 exploitation_id INTEGER NOT NULL REFERENCES exploitations(id) ON DELETE CASCADE,
                 id_interne TEXT,
                 produit TEXT,
                 variete TEXT,
                 superficie_ha REAL,
                 recolte_estimee REAL,
                 recolte_precedente REAL,
                 volume_vendu REAL,
                 created_at TEXT NOT NULL DEFAULT (datetime('now'))
             );
             CREATE TABLE IF NOT EXISTS unites_agricoles (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 exploitation_id INTEGER NOT NULL REFERENCES exploitations(id) ON DELETE CASCADE,
                 id_interne TEXT,
                 identifiant_unite TEXT,
                 superficie_ha REAL,
                 latitude REAL,
                 longitude REAL,
                 created_at TEXT NOT NULL DEFAULT (datetime('now'))
             );
             CREATE TABLE IF NOT EXISTS fiches_tracabilite (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                 reference TEXT NOT NULL UNIQUE,
                 titre TEXT NOT NULL DEFAULT 'Fiche de traçabilité produit',
                 produit TEXT NOT NULL,
                 quantite_demandee_kg REAL NOT NULL,
                 quantite_attribuee_kg REAL NOT NULL DEFAULT 0,
                 statut TEXT NOT NULL DEFAULT 'complete',
                 created_at TEXT NOT NULL DEFAULT (datetime('now'))
             );
             CREATE TABLE IF NOT EXISTS allocations (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 fiche_id INTEGER NOT NULL REFERENCES fiches_tracabilite(id) ON DELETE CASCADE,
                 culture_id INTEGER NOT NULL REFERENCES cultures_certifiees(id) ON DELETE CASCADE,
                 exploitation_id INTEGER NOT NULL REFERENCES exploitations(id) ON DELETE CASCADE,
                 quantite_kg REAL NOT NULL,
                 created_at TEXT NOT NULL DEFAULT (datetime('now')),
                 UNIQUE(fiche_id, culture_id)
             );
             CREATE INDEX IF NOT EXISTS idx_exploitations_user ON exploitations(user_id);
             CREATE INDEX IF NOT EXISTS idx_cultures_exploitation ON cultures_certifiees(exploitation_id);
             CREATE INDEX IF NOT EXISTS idx_unites_exploitation ON unites_agricoles(exploitation_id);
              CREATE INDEX IF NOT EXISTS idx_allocations_exploitation ON allocations(exploitation_id);
              CREATE INDEX IF NOT EXISTS idx_allocations_fiche ON allocations(fiche_id);
              -- Télémétrie d'usage (opt-in) : tampon local, synchronisé par lots si un
              -- endpoint est configuré. Aucune donnée personnelle n'y est stockée.
              CREATE TABLE IF NOT EXISTS telemetry_events (
                  id INTEGER PRIMARY KEY AUTOINCREMENT,
                  event TEXT NOT NULL,
                  props TEXT NOT NULL DEFAULT '{}',
                  created_at TEXT NOT NULL DEFAULT (datetime('now')),
                  synced INTEGER NOT NULL DEFAULT 0
              );
              CREATE INDEX IF NOT EXISTS idx_telemetry_pending ON telemetry_events(synced, id);
              -- Clôtures de semestre : reste enregistré par culture en fin de
              -- semestre (estimation du semestre − alloué sur le semestre).
              CREATE TABLE IF NOT EXISTS clotures_semestre (
                  id INTEGER PRIMARY KEY AUTOINCREMENT,
                  user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                  semestre TEXT NOT NULL,
                  annee INTEGER NOT NULL,
                  culture_id INTEGER NOT NULL REFERENCES cultures_certifiees(id) ON DELETE CASCADE,
                  produit TEXT NOT NULL,
                  estime_kg REAL NOT NULL,
                  alloue_kg REAL NOT NULL,
                  reste_kg REAL NOT NULL,
                  created_at TEXT NOT NULL DEFAULT (datetime('now')),
                  UNIQUE(user_id, semestre, annee, culture_id)
              );
              CREATE INDEX IF NOT EXISTS idx_clotures_user_sem ON clotures_semestre(user_id, semestre, annee);
             CREATE VIRTUAL TABLE IF NOT EXISTS fts_search USING fts5(
                 user_id UNINDEXED,
                 exploitation_id UNINDEXED,
                 operateur,
                 localisation,
                 culture,
                 unite,
                 identifiant,
                 inspecteur,
                 tokenize = 'unicode61 remove_diacritics 2'
             );",
        )?;
        rename_column_if_exists(
            &conn,
            "cultures_certifiees",
            "recolte_en_cours",
            "recolte_estimee",
        )?;
        ensure_column(
            &conn,
            "fiches_tracabilite",
            "titre",
            "TEXT DEFAULT 'Fiche de traçabilité produit'",
        )?;
        ensure_column(&conn, "users", "avatar", "TEXT")?;
        Ok(Db(Mutex::new(conn)))
    }

    pub fn register(&self, name: &str, email: &str, password: &str) -> Result<Session, String> {
        let name = name.trim();
        let email = email.trim().to_lowercase();

        if name.is_empty() {
            return Err("Veuillez entrer votre nom complet".into());
        }
        if !email.contains('@') || !email.contains('.') {
            return Err("Adresse email invalide".into());
        }
        if password.len() < 6 {
            return Err("Le mot de passe doit contenir au moins 6 caractères".into());
        }

        let salt = SaltString::generate(&mut OsRng);
        let password_hash = Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| format!("Erreur de hachage du mot de passe : {e}"))?
            .to_string();

        let conn = self.0.lock().map_err(|_| "Base de données verrouillée".to_string())?;

        let insert = conn.execute(
            "INSERT INTO users (name, email, password_hash) VALUES (?1, ?2, ?3)",
            params![name, email, password_hash],
        );
        match insert {
            Ok(_) => {}
            Err(rusqlite::Error::SqliteFailure(e, _))
                if e.code == rusqlite::ErrorCode::ConstraintViolation =>
            {
                return Err("Un compte existe déjà avec cet email".into());
            }
            Err(e) => return Err(format!("Erreur d'enregistrement : {e}")),
        }

        let id = conn.last_insert_rowid();
        let user = User {
            id,
            name: name.to_string(),
            email,
            onboarding_done: false,
            avatar: None,
        };

        let mut buf = [0u8; 32];
        OsRng.fill_bytes(&mut buf);
        let token: String = buf.iter().map(|b| format!("{b:02x}")).collect();

        conn.execute(
            "INSERT INTO sessions (user_id, token) VALUES (?1, ?2)",
            params![id, token],
        )
        .map_err(|e| format!("Erreur de création de session : {e}"))?;

        Ok(Session { token, user })
    }

    pub fn login(&self, email: &str, password: &str) -> Result<Session, String> {
        let email = email.trim().to_lowercase();

        let conn = self.0.lock().map_err(|_| "Base de données verrouillée".to_string())?;

        let row = conn
            .query_row(
                "SELECT id, name, email, password_hash, onboarding_done, avatar
                 FROM users WHERE email = ?1",
                params![email],
                |r| {
                    Ok((
                        r.get::<_, i64>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, String>(3)?,
                        r.get::<_, bool>(4)?,
                        r.get::<_, Option<String>>(5)?,
                    ))
                },
            )
            .map_err(|_| "Email ou mot de passe incorrect".to_string())?;

        let (id, name, email, password_hash, onboarding_done, avatar) = row;

        let parsed =
            PasswordHash::new(&password_hash).map_err(|_| "Email ou mot de passe incorrect".to_string())?;
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .map_err(|_| "Email ou mot de passe incorrect".to_string())?;

        let mut buf = [0u8; 32];
        OsRng.fill_bytes(&mut buf);
        let token: String = buf.iter().map(|b| format!("{b:02x}")).collect();

        conn.execute("DELETE FROM sessions WHERE user_id = ?1", params![id])
            .map_err(|e| format!("Erreur de création de session : {e}"))?;
        conn.execute(
            "INSERT INTO sessions (user_id, token) VALUES (?1, ?2)",
            params![id, token],
        )
        .map_err(|e| format!("Erreur de création de session : {e}"))?;

        Ok(Session {
            token,
            user: User {
                id,
                name,
                email,
                onboarding_done,
                avatar,
            },
        })
    }

    pub fn first_token(&self) -> Result<String, String> {
        let conn = self.0.lock().map_err(|_| "Base de données verrouillée".to_string())?;
        conn.query_row("SELECT token FROM sessions ORDER BY user_id ASC LIMIT 1", [], |r| r.get(0))
            .map_err(|e| format!("Session introuvable : {e}"))
    }

    pub fn validate(&self, token: &str) -> Result<User, String> {
        let conn = self.0.lock().map_err(|_| "Base de données verrouillée".to_string())?;
        let user_id: Option<i64> = conn
            .query_row(
                "SELECT user_id FROM sessions WHERE token = ?1",
                params![token],
                |r| r.get(0),
            )
            .optional()
            .map_err(|_| "Session invalide ou expirée".to_string())?;
        let user_id = user_id.ok_or_else(|| "Session invalide ou expirée".to_string())?;
        self.load_user(&conn, user_id)
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<(), String> {
        let conn = self.0.lock().map_err(|_| "Base de données verrouillée".to_string())?;
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )
        .map_err(|e| format!("Mise à jour du réglage impossible : {e}"))?;
        Ok(())
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>, String> {
        let conn = self.0.lock().map_err(|_| "Base de données verrouillée".to_string())?;
        let value: Option<String> = conn
            .query_row("SELECT value FROM settings WHERE key = ?1", params![key], |r| {
                r.get(0)
            })
            .optional()
            .map_err(|e| format!("Lecture du réglage impossible : {e}"))?;
        Ok(value)
    }

    pub fn remove_setting(&self, key: &str) -> Result<(), String> {
        let conn = self.0.lock().map_err(|_| "Base de données verrouillée".to_string())?;
        conn.execute("DELETE FROM settings WHERE key = ?1", params![key])
            .map_err(|e| format!("Suppression du réglage impossible : {e}"))?;
        Ok(())
    }

    pub fn set_ai_session(
        &self,
        user_id: i64,
        access_token: &str,
        refresh_token: &str,
    ) -> Result<(), String> {
        let conn = self.0.lock().map_err(|_| "Base de données verrouillée".to_string())?;
        conn.execute(
            "INSERT INTO ai_sessions (user_id, access_token, refresh_token)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(user_id) DO UPDATE SET
                 access_token = excluded.access_token,
                 refresh_token = excluded.refresh_token,
                 updated_at = datetime('now')",
            params![user_id, access_token, refresh_token],
        )
        .map_err(|e| format!("Enregistrement de la session IA impossible : {e}"))?;
        Ok(())
    }

    pub fn get_ai_session(&self, user_id: i64) -> Result<Option<AiSession>, String> {
        let conn = self.0.lock().map_err(|_| "Base de données verrouillée".to_string())?;
        conn.query_row(
            "SELECT access_token, refresh_token FROM ai_sessions WHERE user_id = ?1",
            params![user_id],
            |row| {
                Ok(AiSession {
                    access_token: row.get(0)?,
                    refresh_token: row.get(1)?,
                })
            },
        )
        .optional()
        .map_err(|e| format!("Lecture de la session IA impossible : {e}"))
    }

    const SETTING_TELEMETRY: &str = "telemetry_enabled";
    const SETTING_INCLURE_NOUVEAUX: &str = "inclure_nouveaux_producteurs";

    /// Les producteurs sans aucune participation passée sont-ils éligibles
    /// aux fiches ? Réglage global (Paramètres → Fiches), défaut : oui.
    pub fn inclure_nouveaux(&self) -> bool {
        self.get_setting(Self::SETTING_INCLURE_NOUVEAUX)
            .map(|v| v.as_deref() != Some("0"))
            .unwrap_or(true)
    }

    pub fn set_inclure_nouveaux(&self, enabled: bool) -> Result<(), String> {
        self.set_setting(Self::SETTING_INCLURE_NOUVEAUX, if enabled { "1" } else { "0" })
    }

    /// Fragment SQL excluant les producteurs sans participation (vide si inclus).
    /// À évaluer AVANT tout verrouillage (le Mutex n'est pas réentrant).
    fn filtre_nouveaux_sql(&self) -> &'static str {
        if self.inclure_nouveaux() {
            ""
        } else {
            " AND EXISTS (SELECT 1 FROM allocations a0 WHERE a0.exploitation_id = e.id)"
        }
    }

    /// Consentement télémétrie (opt-in, défaut : désactivé).
    pub fn telemetry_consent(&self) -> bool {
        self.get_setting(Self::SETTING_TELEMETRY)
            .map(|v| v.as_deref() == Some("1"))
            .unwrap_or(false)
    }

    pub fn set_telemetry_consent(&self, enabled: bool) -> Result<(), String> {
        self.set_setting(Self::SETTING_TELEMETRY, if enabled { "1" } else { "0" })
    }

    /// Enregistre un événement d'usage dans le tampon local.
    /// Sans effet si la télémétrie n'est pas activée. Ne jamais y mettre de
    /// données personnelles (mots de passe, emails, contenus de fichiers, clés API).
    pub fn track_event(&self, event: &str, props: &serde_json::Value) {
        if !self.telemetry_consent() {
            return;
        }
        let props_str = serde_json::to_string(props).unwrap_or_else(|_| "{}".into());
        let conn = match self.0.lock() {
            Ok(c) => c,
            Err(_) => return,
        };
        if let Err(e) = conn.execute(
            "INSERT INTO telemetry_events (event, props) VALUES (?1, ?2)",
            params![event, props_str],
        ) {
            log::warn!("Télémétrie : enregistrement impossible ({e})");
        }
    }

    /// Récupère jusqu'à `limit` événements non synchronisés (les plus anciens d'abord).
    pub fn take_pending_telemetry(&self, limit: i64) -> Result<Vec<(i64, String, String, String)>, String> {
        let conn = self.0.lock().map_err(|_| "Base de données verrouillée".to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT id, event, props, created_at FROM telemetry_events
                 WHERE synced = 0 ORDER BY id ASC LIMIT ?1",
            )
            .map_err(|e| format!("Lecture télémétrie impossible : {e}"))?;
        let rows = stmt
            .query_map(params![limit], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, String>(3)?))
            })
            .map_err(|e| format!("Lecture télémétrie impossible : {e}"))?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(|e| format!("Lecture télémétrie impossible : {e}"))?);
        }
        Ok(out)
    }

    pub fn mark_telemetry_synced(&self, ids: &[i64]) -> Result<(), String> {
        if ids.is_empty() {
            return Ok(());
        }
        let conn = self.0.lock().map_err(|_| "Base de données verrouillée".to_string())?;
        let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = format!("UPDATE telemetry_events SET synced = 1 WHERE id IN ({placeholders})");
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| format!("Mise à jour télémétrie impossible : {e}"))?;
        stmt.execute(rusqlite::params_from_iter(ids.iter()))
            .map_err(|e| format!("Mise à jour télémétrie impossible : {e}"))?;
        Ok(())
    }

    pub fn update_profile(&self, token: &str, name: &str, email: &str) -> Result<User, String> {
        let name = name.trim();
        let email = email.trim().to_lowercase();

        if name.is_empty() {
            return Err("Veuillez entrer votre nom complet".into());
        }
        if !email.contains('@') || !email.contains('.') {
            return Err("Adresse email invalide".into());
        }

        let conn = self.0.lock().map_err(|_| "Base de données verrouillée".to_string())?;
        let user_id = self.require_user(&conn, token)?;

        match conn.execute(
            "UPDATE users SET name = ?1, email = ?2 WHERE id = ?3",
            params![name, email, user_id],
        ) {
            Ok(_) => {}
            Err(rusqlite::Error::SqliteFailure(e, _))
                if e.code == rusqlite::ErrorCode::ConstraintViolation =>
            {
                return Err("Un compte existe déjà avec cet email".into());
            }
            Err(e) => return Err(format!("Erreur de mise à jour : {e}")),
        }

        self.load_user(&conn, user_id)
    }

    pub fn update_avatar(&self, token: &str, avatar: Option<String>) -> Result<User, String> {
        let conn = self.0.lock().map_err(|_| "Base de données verrouillée".to_string())?;
        let user_id = self.require_user(&conn, token)?;

        conn.execute(
            "UPDATE users SET avatar = ?1 WHERE id = ?2",
            params![avatar, user_id],
        )
        .map_err(|e| format!("Erreur de mise à jour : {e}"))?;

        self.load_user(&conn, user_id)
    }

    fn require_user(&self, conn: &Connection, token: &str) -> Result<i64, String> {
        conn.query_row(
            "SELECT user_id FROM sessions WHERE token = ?1",
            params![token],
            |r| r.get(0),
        )
        .optional()
        .map_err(|_| "Session invalide ou expirée".to_string())?
        .ok_or_else(|| "Session invalide ou expirée".to_string())
    }

    fn load_user(&self, conn: &Connection, user_id: i64) -> Result<User, String> {
        conn.query_row(
            "SELECT id, name, email, onboarding_done, avatar FROM users WHERE id = ?1",
            params![user_id],
            |r| {
                Ok(User {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    email: r.get(2)?,
                    onboarding_done: r.get(3)?,
                    avatar: r.get(4)?,
                })
            },
        )
        .map_err(|e| format!("Utilisateur introuvable : {e}"))
    }

    pub fn logout(&self, token: &str) {
        if let Ok(conn) = self.0.lock() {
            let _ = conn.execute(
                "DELETE FROM ai_sessions
                 WHERE user_id = (SELECT user_id FROM sessions WHERE token = ?1)",
                params![token],
            );
            let _ = conn.execute("DELETE FROM sessions WHERE token = ?1", params![token]);
        }
    }

    pub fn complete_onboarding(&self, token: &str) -> Result<(), String> {
        let conn = self.0.lock().map_err(|_| "Base de données verrouillée".to_string())?;
        let updated = conn
            .execute(
                "UPDATE users SET onboarding_done = 1
                 WHERE id = (SELECT user_id FROM sessions WHERE token = ?1)",
                params![token],
            )
            .map_err(|e| format!("Erreur de mise à jour : {e}"))?;

        if updated == 0 {
            return Err("Session invalide ou expirée".into());
        }
        Ok(())
    }

    pub fn import_workbook(&self, token: &str, data: &ParsedWorkbook) -> Result<ImportResult, String> {
        let conn = self.0.lock().map_err(|_| "Base de données verrouillée".to_string())?;
        let user_id = self.require_user(&conn, token)?;

        let mut exploitations_added = 0usize;
        let mut cultures_added = 0usize;
        let mut unites_added = 0usize;
        let mut rows_updated = 0usize;
        let mut rows_deleted = 0usize;
        let mut loaded_ids = Vec::new();

        // --- Exploitations : mise à jour si l'identifiant existe déjà ---
        for d in &data.exploitations {
            let Some(id_interne) = d.id_interne.clone() else {
                continue;
            };
            let updated = conn
                .execute(
                    "UPDATE exploitations SET
                         id_national = ?2, village = ?3, district = ?4, region_inspection = ?5,
                         superficie_total = ?6, type_exploitation = ?7, nb_unites = ?8, nb_cultures = ?9,
                         operateur_prenom = ?10, operateur_nom = ?11, operateur_telephone = ?12,
                         operateur_id_national = ?13, operateur_genre = ?14, operateur_annee_nais = ?15,
                         proprietaire_prenom = ?16, proprietaire_nom = ?17, proprietaire_telephone = ?18,
                         proprietaire_id_national = ?19, proprietaire_genre = ?20,
                         travailleurs_permanents = ?21, travailleurs_temporaires = ?22,
                         inspecteur_nom = ?23, inspecteur_annee = ?24, inspecteur_mois = ?25,
                         inspecteur_jour = ?26
                     WHERE user_id = ?1 AND id_interne = ?27",
                    params![
                        user_id,
                        d.id_national,
                        d.village,
                        d.district,
                        d.region_inspection,
                        d.superficie_total,
                        d.type_exploitation,
                        d.nb_unites,
                        d.nb_cultures,
                        d.operateur_prenom,
                        d.operateur_nom,
                        d.operateur_telephone,
                        d.operateur_id_national,
                        d.operateur_genre,
                        d.operateur_annee_nais,
                        d.proprietaire_prenom,
                        d.proprietaire_nom,
                        d.proprietaire_telephone,
                        d.proprietaire_id_national,
                        d.proprietaire_genre,
                        d.travailleurs_permanents,
                        d.travailleurs_temporaires,
                        d.inspecteur_nom,
                        d.inspecteur_annee,
                        d.inspecteur_mois,
                        d.inspecteur_jour,
                        id_interne,
                    ],
                )
                .map_err(|e| format!("Erreur d'import des exploitations : {e}"))?;
            if updated > 0 {
                rows_updated += 1;
            } else {
                let inserted = conn
                    .execute(
                        "INSERT OR IGNORE INTO exploitations (
                         user_id, id_interne, id_national, village, district, region_inspection,
                         superficie_total, type_exploitation, nb_unites, nb_cultures,
                         operateur_prenom, operateur_nom, operateur_telephone,
                         operateur_id_national, operateur_genre, operateur_annee_nais,
                         proprietaire_prenom, proprietaire_nom, proprietaire_telephone,
                         proprietaire_id_national, proprietaire_genre,
                         travailleurs_permanents, travailleurs_temporaires,
                         inspecteur_nom, inspecteur_annee, inspecteur_mois, inspecteur_jour
                     ) VALUES (
                         ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16,
                         ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27
                     )",
                        params![
                            user_id,
                            id_interne,
                            d.id_national,
                            d.village,
                            d.district,
                            d.region_inspection,
                            d.superficie_total,
                            d.type_exploitation,
                            d.nb_unites,
                            d.nb_cultures,
                            d.operateur_prenom,
                            d.operateur_nom,
                            d.operateur_telephone,
                            d.operateur_id_national,
                            d.operateur_genre,
                            d.operateur_annee_nais,
                            d.proprietaire_prenom,
                            d.proprietaire_nom,
                            d.proprietaire_telephone,
                            d.proprietaire_id_national,
                            d.proprietaire_genre,
                            d.travailleurs_permanents,
                            d.travailleurs_temporaires,
                            d.inspecteur_nom,
                            d.inspecteur_annee,
                            d.inspecteur_mois,
                            d.inspecteur_jour,
                        ],
                    )
                    .map_err(|e| format!("Erreur d'import des exploitations : {e}"))?;
                if inserted > 0 {
                    exploitations_added += 1;
                }
            }
            loaded_ids.push(id_interne);
        }

        let mut used_culture_ids: HashSet<i64> = HashSet::new();
        for c in &data.cultures {
            let Some(id_interne) = c.id_interne.clone() else {
                continue;
            };
            let explosion_id = self.exploitation_id(&conn, user_id, &id_interne);
            let Some(exploitation_id) = explosion_id else {
                continue;
            };
            // Upsert sur (exploitation, produit) : une culture existante non encore
            // reprise par ce fichier est mise à jour, sinon insertion.
            let existing_id: Option<i64> = match c.produit.clone() {
                Some(produit) => {
                    let mut stmt = conn
                        .prepare(
                            "SELECT id FROM cultures_certifiees
                             WHERE exploitation_id = ?1 AND produit = ?2 ORDER BY id ASC",
                        )
                        .map_err(|e| format!("Erreur d'import des cultures : {e}"))?;
                    let candidates: Vec<i64> = stmt
                        .query_map(params![exploitation_id, produit], |r| r.get(0))
                        .map_err(|e| format!("Erreur d'import des cultures : {e}"))?
                        .collect::<Result<_, _>>()
                        .map_err(|e| format!("Erreur d'import des cultures : {e}"))?;
                    candidates.into_iter().find(|id| !used_culture_ids.contains(id))
                }
                None => None,
            };
            if let Some(culture_id) = existing_id {
                conn.execute(
                    "UPDATE cultures_certifiees SET
                         id_interne = ?2, produit = ?3, variete = ?4, superficie_ha = ?5,
                         recolte_estimee = ?6, recolte_precedente = ?7, volume_vendu = ?8
                     WHERE id = ?1",
                    params![
                        culture_id,
                        id_interne,
                        c.produit,
                        c.variete,
                        c.superficie_ha,
                        c.recolte_estimee,
                        c.recolte_precedente,
                        c.volume_vendu,
                    ],
                )
                .map_err(|e| format!("Erreur d'import des cultures : {e}"))?;
                used_culture_ids.insert(culture_id);
                rows_updated += 1;
                continue;
            }
            let inserted = conn
                .execute(
                    "INSERT INTO cultures_certifiees (
                         exploitation_id, id_interne, produit, variete,
                         superficie_ha, recolte_estimee, recolte_precedente, volume_vendu
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        exploitation_id,
                        id_interne,
                        c.produit,
                        c.variete,
                        c.superficie_ha,
                        c.recolte_estimee,
                        c.recolte_precedente,
                        c.volume_vendu,
                    ],
                )
                .map_err(|e| format!("Erreur d'import des cultures : {e}"))?;
            cultures_added += inserted;
            used_culture_ids.insert(conn.last_insert_rowid());
        }

        let mut used_unite_ids: HashSet<i64> = HashSet::new();
        for u in &data.unites {
            let Some(id_interne) = u.id_interne.clone() else {
                continue;
            };
            let Some(exploitation_id) = self.exploitation_id(&conn, user_id, &id_interne) else {
                continue;
            };
            // Upsert sur (exploitation, identifiant d'unité).
            let existing_id: Option<i64> = match u.identifiant_unite.clone() {
                Some(identifiant) => {
                    let mut stmt = conn
                        .prepare(
                            "SELECT id FROM unites_agricoles
                             WHERE exploitation_id = ?1 AND identifiant_unite = ?2 ORDER BY id ASC",
                        )
                        .map_err(|e| format!("Erreur d'import des unités agricoles : {e}"))?;
                    let candidates: Vec<i64> = stmt
                        .query_map(params![exploitation_id, identifiant], |r| r.get(0))
                        .map_err(|e| format!("Erreur d'import des unités agricoles : {e}"))?
                        .collect::<Result<_, _>>()
                        .map_err(|e| format!("Erreur d'import des unités agricoles : {e}"))?;
                    candidates.into_iter().find(|id| !used_unite_ids.contains(id))
                }
                None => None,
            };
            if let Some(unite_id) = existing_id {
                conn.execute(
                    "UPDATE unites_agricoles SET
                         id_interne = ?2, identifiant_unite = ?3, superficie_ha = ?4,
                         latitude = ?5, longitude = ?6
                     WHERE id = ?1",
                    params![
                        unite_id,
                        id_interne,
                        u.identifiant_unite,
                        u.superficie_ha,
                        u.latitude,
                        u.longitude,
                    ],
                )
                .map_err(|e| format!("Erreur d'import des unités agricoles : {e}"))?;
                used_unite_ids.insert(unite_id);
                rows_updated += 1;
                continue;
            }
            let inserted = conn
                .execute(
                    "INSERT INTO unites_agricoles (
                         exploitation_id, id_interne, identifiant_unite, superficie_ha, latitude, longitude
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        exploitation_id,
                        id_interne,
                        u.identifiant_unite,
                        u.superficie_ha,
                        u.latitude,
                        u.longitude,
                    ],
                )
                .map_err(|e| format!("Erreur d'import des unités agricoles : {e}"))?;
            unites_added += inserted;
            used_unite_ids.insert(conn.last_insert_rowid());
        }

        // --- Lignes déjà utilisées par des fiches générées : intouchables ---
        let mut stmt = conn
            .prepare(
                "SELECT DISTINCT a.exploitation_id
                 FROM allocations a
                 JOIN fiches_tracabilite f ON f.id = a.fiche_id
                 WHERE f.user_id = ?1",
            )
            .map_err(|e| format!("Erreur d'import : {e}"))?;
        let allocated_exploitations: HashSet<i64> = stmt
            .query_map(params![user_id], |r| r.get(0))
            .map_err(|e| format!("Erreur d'import : {e}"))?
            .collect::<Result<_, _>>()
            .map_err(|e| format!("Erreur d'import : {e}"))?;
        let mut stmt = conn
            .prepare(
                "SELECT DISTINCT a.culture_id
                 FROM allocations a
                 JOIN fiches_tracabilite f ON f.id = a.fiche_id
                 WHERE f.user_id = ?1",
            )
            .map_err(|e| format!("Erreur d'import : {e}"))?;
        let allocated_cultures: HashSet<i64> = stmt
            .query_map(params![user_id], |r| r.get(0))
            .map_err(|e| format!("Erreur d'import : {e}"))?
            .collect::<Result<_, _>>()
            .map_err(|e| format!("Erreur d'import : {e}"))?;

        // --- Exploitations absentes du fichier (hors fiches) : suppression ---
        let file_set: HashSet<&str> = loaded_ids.iter().map(|s| s.as_str()).collect();
        let mut stmt = conn
            .prepare("SELECT id, id_interne FROM exploitations WHERE user_id = ?1")
            .map_err(|e| format!("Erreur d'import : {e}"))?;
        let existing: Vec<(i64, Option<String>)> = stmt
            .query_map(params![user_id], |r| Ok((r.get(0)?, r.get(1)?)))
            .map_err(|e| format!("Erreur d'import : {e}"))?
            .collect::<Result<_, _>>()
            .map_err(|e| format!("Erreur d'import : {e}"))?;
        for (exp_id, id_int) in existing {
            let in_file = id_int.as_deref().map(|s| file_set.contains(s)).unwrap_or(false);
            if in_file || allocated_exploitations.contains(&exp_id) {
                continue;
            }
            let cults: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM cultures_certifiees WHERE exploitation_id = ?1",
                    params![exp_id],
                    |r| r.get(0),
                )
                .map_err(|e| format!("Erreur d'import : {e}"))?;
            let unts: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM unites_agricoles WHERE exploitation_id = ?1",
                    params![exp_id],
                    |r| r.get(0),
                )
                .map_err(|e| format!("Erreur d'import : {e}"))?;
            conn.execute("DELETE FROM exploitations WHERE id = ?1", params![exp_id])
                .map_err(|e| format!("Erreur d'import : {e}"))?;
            rows_deleted += 1 + cults as usize + unts as usize;
        }

        // --- Cultures et unités absentes du fichier (hors fiches) : suppression ---
        let mut stmt = conn
            .prepare(
                "SELECT c.id FROM cultures_certifiees c
                 JOIN exploitations e ON e.id = c.exploitation_id
                 WHERE e.user_id = ?1",
            )
            .map_err(|e| format!("Erreur d'import : {e}"))?;
        let all_culture_ids: Vec<i64> = stmt
            .query_map(params![user_id], |r| r.get(0))
            .map_err(|e| format!("Erreur d'import : {e}"))?
            .collect::<Result<_, _>>()
            .map_err(|e| format!("Erreur d'import : {e}"))?;
        for cid in all_culture_ids {
            if used_culture_ids.contains(&cid) || allocated_cultures.contains(&cid) {
                continue;
            }
            conn.execute("DELETE FROM cultures_certifiees WHERE id = ?1", params![cid])
                .map_err(|e| format!("Erreur d'import : {e}"))?;
            rows_deleted += 1;
        }
        let mut stmt = conn
            .prepare(
                "SELECT u.id, u.exploitation_id FROM unites_agricoles u
                 JOIN exploitations e ON e.id = u.exploitation_id
                 WHERE e.user_id = ?1",
            )
            .map_err(|e| format!("Erreur d'import : {e}"))?;
        let all_unites: Vec<(i64, i64)> = stmt
            .query_map(params![user_id], |r| Ok((r.get(0)?, r.get(1)?)))
            .map_err(|e| format!("Erreur d'import : {e}"))?
            .collect::<Result<_, _>>()
            .map_err(|e| format!("Erreur d'import : {e}"))?;
        for (uid, exp_id) in all_unites {
            if used_unite_ids.contains(&uid) || allocated_exploitations.contains(&exp_id) {
                continue;
            }
            conn.execute("DELETE FROM unites_agricoles WHERE id = ?1", params![uid])
                .map_err(|e| format!("Erreur d'import : {e}"))?;
            rows_deleted += 1;
        }

        self.rebuild_search_index(&conn, user_id)?;

        let mut exploitations = Vec::new();
        for id_interne in loaded_ids {
            if let Ok(Some(e)) = self.load_exploitation(&conn, user_id, &id_interne) {
                exploitations.push(e);
            }
        }

        Ok(ImportResult {
            exploitations_added,
            cultures_added,
            unites_added,
            rows_updated,
            rows_deleted,
            exploitations,
            sheets: Vec::new(),
        })
    }

    pub fn list_exploitations(&self, token: &str) -> Result<Vec<Exploitation>, String> {
        let conn = self.0.lock().map_err(|_| "Base de données verrouillée".to_string())?;
        let user_id = self.require_user(&conn, token)?;

        let mut stmt = conn
            .prepare(
                "SELECT e.id, e.id_interne, e.id_national, e.village, e.district,
                        e.region_inspection, e.superficie_total, e.type_exploitation,
                        e.nb_unites, e.nb_cultures,
                        e.operateur_prenom, e.operateur_nom, e.operateur_telephone,
                        e.operateur_id_national, e.operateur_genre, e.operateur_annee_nais,
                        e.proprietaire_prenom, e.proprietaire_nom, e.proprietaire_telephone,
                        e.proprietaire_id_national, e.proprietaire_genre,
                        e.travailleurs_permanents, e.travailleurs_temporaires,
                        e.inspecteur_nom, e.inspecteur_annee, e.inspecteur_mois, e.inspecteur_jour,
                        (SELECT COUNT(*) FROM cultures_certifiees c WHERE c.exploitation_id = e.id),
                        (SELECT COUNT(*) FROM unites_agricoles u WHERE u.exploitation_id = e.id),
                        e.created_at
                 FROM exploitations e
                 WHERE e.user_id = ?1
                 ORDER BY e.created_at DESC, e.id DESC",
            )
            .map_err(|e| format!("Erreur de lecture des exploitations : {e}"))?;

        let rows = stmt
            .query_map(params![user_id], |r| {
                Ok(Exploitation {
                    id: r.get(0)?,
                    id_interne: r.get(1)?,
                    id_national: r.get(2)?,
                    village: r.get(3)?,
                    district: r.get(4)?,
                    region_inspection: r.get(5)?,
                    superficie_total: r.get(6)?,
                    type_exploitation: r.get(7)?,
                    nb_unites: r.get(8)?,
                    nb_cultures: r.get(9)?,
                    operateur_prenom: r.get(10)?,
                    operateur_nom: r.get(11)?,
                    operateur_telephone: r.get(12)?,
                    operateur_id_national: r.get(13)?,
                    operateur_genre: r.get(14)?,
                    operateur_annee_nais: r.get(15)?,
                    proprietaire_prenom: r.get(16)?,
                    proprietaire_nom: r.get(17)?,
                    proprietaire_telephone: r.get(18)?,
                    proprietaire_id_national: r.get(19)?,
                    proprietaire_genre: r.get(20)?,
                    travailleurs_permanents: r.get(21)?,
                    travailleurs_temporaires: r.get(22)?,
                    inspecteur_nom: r.get(23)?,
                    inspecteur_annee: r.get(24)?,
                    inspecteur_mois: r.get(25)?,
                    inspecteur_jour: r.get(26)?,
                    cultures_count: r.get(27)?,
                    unites_count: r.get(28)?,
                    created_at: r.get(29)?,
                })
            })
            .map_err(|e| format!("Erreur de lecture des exploitations : {e}"))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Erreur de lecture des exploitations : {e}"))
    }

    pub fn export_data(&self, token: &str) -> Result<ExportData, String> {
        let conn = self.0.lock().map_err(|_| "Base de données verrouillée".to_string())?;
        let user_id = self.require_user(&conn, token)?;

        let exported_at: String = conn
            .query_row("SELECT datetime('now')", [], |r| r.get(0))
            .map_err(|e| format!("Erreur d'horodatage : {e}"))?;

        let mut stmt = conn
            .prepare("SELECT id FROM exploitations WHERE user_id = ?1 ORDER BY created_at DESC, id DESC")
            .map_err(|e| format!("Erreur de lecture : {e}"))?;
        let ids: Vec<i64> = stmt
            .query_map(params![user_id], |r| r.get(0))
            .map_err(|e| format!("Erreur de lecture : {e}"))?
            .collect::<Result<_, _>>()
            .map_err(|e| format!("Erreur de lecture : {e}"))?;

        let mut exploitations = Vec::with_capacity(ids.len());
        for id in ids {
            let exploitation = self
                .load_exploitation_by_id(&conn, user_id, id)?
                .ok_or_else(|| format!("Exploitation {id} introuvable"))?;
            let cultures = self.load_cultures(&conn, id)?;
            let unites = self.load_unites(&conn, id)?;
            exploitations.push(ExportExploitation {
                exploitation,
                cultures,
                unites,
            });
        }

        Ok(ExportData {
            version: 1,
            exported_at,
            exploitations,
        })
    }

    pub fn reset_data(&self, token: &str) -> Result<usize, String> {
        let conn = self.0.lock().map_err(|_| "Base de données verrouillée".to_string())?;
        let user_id = self.require_user(&conn, token)?;

        let scope = "exploitation_id IN (SELECT id FROM exploitations WHERE user_id = ?1)";
        let _ = conn.execute(
            &format!("DELETE FROM allocations WHERE {scope}"),
            params![user_id],
        );
        let _ = conn.execute("DELETE FROM fiches_tracabilite WHERE user_id = ?1", params![user_id]);
        let unites = conn
            .execute(&format!("DELETE FROM unites_agricoles WHERE {scope}"), params![user_id])
            .map_err(|e| format!("Erreur de réinitialisation : {e}"))?;
        let cultures = conn
            .execute(&format!("DELETE FROM cultures_certifiees WHERE {scope}"), params![user_id])
            .map_err(|e| format!("Erreur de réinitialisation : {e}"))?;
        let exploitations = conn
            .execute("DELETE FROM exploitations WHERE user_id = ?1", params![user_id])
            .map_err(|e| format!("Erreur de réinitialisation : {e}"))?;
        let _ = conn.execute("DELETE FROM fts_search WHERE user_id = ?1", params![user_id]);

        Ok(unites + cultures + exploitations)
    }

    pub fn stats_globales(&self, token: &str) -> Result<GlobalStats, String> {
        let conn = self.0.lock().map_err(|_| "Base de données verrouillée".to_string())?;
        let user_id = self.require_user(&conn, token)?;

        let nb_exploitations: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM exploitations WHERE user_id = ?1",
                params![user_id],
                |r| r.get(0),
            )
            .unwrap_or(0);
        let nb_unites: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM unites_agricoles u JOIN exploitations e ON u.exploitation_id = e.id WHERE e.user_id = ?1",
                params![user_id],
                |r| r.get(0),
            )
            .unwrap_or(0);
        let nb_cultures: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM cultures_certifiees c JOIN exploitations e ON c.exploitation_id = e.id WHERE e.user_id = ?1",
                params![user_id],
                |r| r.get(0),
            )
            .unwrap_or(0);
        let superficie_total_ha: f64 = conn
            .query_row(
                "SELECT COALESCE(SUM(superficie_total), 0) FROM exploitations WHERE user_id = ?1",
                params![user_id],
                |r| r.get(0),
            )
            .unwrap_or(0.0);
        let nb_producteurs: i64 = conn
            .query_row(
                "SELECT COUNT(DISTINCT id) FROM exploitations WHERE user_id = ?1",
                params![user_id],
                |r| r.get(0),
            )
            .unwrap_or(0);
        let nb_fiches: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM fiches_tracabilite WHERE user_id = ?1",
                params![user_id],
                |r| r.get(0),
            )
            .unwrap_or(0);
        let (nb_allocations, quantite_allouee_kg): (i64, f64) = conn
            .query_row(
                "SELECT COUNT(*), COALESCE(SUM(a.quantite_kg), 0)
                 FROM allocations a JOIN fiches_tracabilite f ON a.fiche_id = f.id
                 WHERE f.user_id = ?1",
                params![user_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(|e| format!("Erreur de statistiques : {e}"))?;
        let produit_plus_cultive: Option<String> = conn
            .query_row(
                "SELECT c.produit FROM cultures_certifiees c JOIN exploitations e ON c.exploitation_id = e.id
                 WHERE e.user_id = ?1 AND c.produit IS NOT NULL AND c.produit != ''
                 GROUP BY c.produit ORDER BY SUM(COALESCE(c.superficie_ha, 0)) DESC LIMIT 1",
                params![user_id],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| format!("Erreur de statistiques : {e}"))?;

        Ok(GlobalStats {
            nb_exploitations,
            nb_unites,
            nb_cultures,
            superficie_total_ha,
            nb_producteurs,
            nb_fiches,
            nb_allocations,
            quantite_allouee_kg,
            produit_plus_cultive,
        })
    }

    pub fn search_exploitations(
        &self,
        token: &str,
        query: &str,
        category: SearchCategory,
    ) -> Result<Vec<Exploitation>, String> {
        let fts_query = build_fts_query(query, category);
        if fts_query.is_empty() {
            return Ok(Vec::new());
        }

        let conn = self.0.lock().map_err(|_| "Base de données verrouillée".to_string())?;
        let user_id = self.require_user(&conn, token)?;
        self.ensure_search_index(&conn, user_id)?;

        let mut stmt = conn
            .prepare(
                "SELECT exploitation_id, bm25(fts_search, 1.2, 0.5, 0.9, 0.9, 1.0, 0.8) AS rank
                 FROM fts_search
                 WHERE fts_search MATCH ?1 AND user_id = ?2
                 ORDER BY rank
                 LIMIT 100",
            )
            .map_err(|e| format!("Erreur de recherche : {e}"))?;

        let ids: Vec<i64> = stmt
            .query_map(params![fts_query, user_id], |r| r.get(0))
            .map_err(|e| format!("Erreur de recherche : {e}"))?
            .collect::<Result<_, _>>()
            .map_err(|e| format!("Erreur de recherche : {e}"))?;

        let mut out = Vec::with_capacity(ids.len());
        for id in ids {
            if let Ok(Some(e)) = self.load_exploitation_by_id(&conn, user_id, id) {
                out.push(e);
            }
        }
        Ok(out)
    }

    fn ensure_search_index(&self, conn: &Connection, user_id: i64) -> Result<(), String> {
        let indexed: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM fts_search WHERE user_id = ?1)",
                params![user_id],
                |r| r.get(0),
            )
            .map_err(|e| format!("Erreur d'index de recherche : {e}"))?;
        if indexed {
            return Ok(());
        }

        let has_data: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM exploitations WHERE user_id = ?1)",
                params![user_id],
                |r| r.get(0),
            )
            .map_err(|e| format!("Erreur d'index de recherche : {e}"))?;
        if has_data {
            self.rebuild_search_index(conn, user_id)?;
        }
        Ok(())
    }

    fn rebuild_search_index(&self, conn: &Connection, user_id: i64) -> Result<(), String> {
        conn.execute("DELETE FROM fts_search WHERE user_id = ?1", params![user_id])
            .map_err(|e| format!("Erreur d'index de recherche : {e}"))?;

        let mut stmt = conn
            .prepare("SELECT id FROM exploitations WHERE user_id = ?1 ORDER BY id ASC")
            .map_err(|e| format!("Erreur d'index de recherche : {e}"))?;
        let ids: Vec<i64> = stmt
            .query_map(params![user_id], |r| r.get(0))
            .map_err(|e| format!("Erreur d'index de recherche : {e}"))?
            .collect::<Result<_, _>>()
            .map_err(|e| format!("Erreur d'index de recherche : {e}"))?;

        let mut insert = conn
            .prepare(
                "INSERT INTO fts_search
                     (user_id, exploitation_id, operateur, localisation, culture, unite, identifiant, inspecteur)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            )
            .map_err(|e| format!("Erreur d'index de recherche : {e}"))?;

        for id in ids {
            let e = self
                .load_exploitation_by_id(conn, user_id, id)?
                .ok_or_else(|| format!("Exploitation {id} introuvable"))?;
            let cultures = self.load_cultures(conn, id)?;
            let unites = self.load_unites(conn, id)?;
            let col = build_search_columns(&e, &cultures, &unites);
            insert
                .execute(params![
                    user_id,
                    id,
                    col.operateur,
                    col.localisation,
                    col.culture,
                    col.unite,
                    col.identifiant,
                    col.inspecteur
                ])
                .map_err(|err| format!("Erreur d'index de recherche : {err}"))?;
        }
        Ok(())
    }

    fn exploitation_id(&self, conn: &Connection, user_id: i64, id_interne: &str) -> Option<i64> {
        conn.query_row(
            "SELECT id FROM exploitations WHERE user_id = ?1 AND id_interne = ?2",
            params![user_id, id_interne],
            |r| r.get(0),
        )
        .optional()
        .ok()
        .flatten()
    }

    fn load_exploitation(
        &self,
        conn: &Connection,
        user_id: i64,
        id_interne: &str,
    ) -> Result<Option<Exploitation>, String> {
        let mut stmt = conn
            .prepare(
                "SELECT e.id, e.id_interne, e.id_national, e.village, e.district,
                        e.region_inspection, e.superficie_total, e.type_exploitation,
                        e.nb_unites, e.nb_cultures,
                        e.operateur_prenom, e.operateur_nom, e.operateur_telephone,
                        e.operateur_id_national, e.operateur_genre, e.operateur_annee_nais,
                        e.proprietaire_prenom, e.proprietaire_nom, e.proprietaire_telephone,
                        e.proprietaire_id_national, e.proprietaire_genre,
                        e.travailleurs_permanents, e.travailleurs_temporaires,
                        e.inspecteur_nom, e.inspecteur_annee, e.inspecteur_mois, e.inspecteur_jour,
                        (SELECT COUNT(*) FROM cultures_certifiees c WHERE c.exploitation_id = e.id),
                        (SELECT COUNT(*) FROM unites_agricoles u WHERE u.exploitation_id = e.id),
                        e.created_at
                 FROM exploitations e
                 WHERE e.user_id = ?1 AND e.id_interne = ?2",
            )
            .map_err(|e| format!("Erreur de lecture : {e}"))?;

        let row = stmt
            .query_row(params![user_id, id_interne], |r| {
                Ok(Exploitation {
                    id: r.get(0)?,
                    id_interne: r.get(1)?,
                    id_national: r.get(2)?,
                    village: r.get(3)?,
                    district: r.get(4)?,
                    region_inspection: r.get(5)?,
                    superficie_total: r.get(6)?,
                    type_exploitation: r.get(7)?,
                    nb_unites: r.get(8)?,
                    nb_cultures: r.get(9)?,
                    operateur_prenom: r.get(10)?,
                    operateur_nom: r.get(11)?,
                    operateur_telephone: r.get(12)?,
                    operateur_id_national: r.get(13)?,
                    operateur_genre: r.get(14)?,
                    operateur_annee_nais: r.get(15)?,
                    proprietaire_prenom: r.get(16)?,
                    proprietaire_nom: r.get(17)?,
                    proprietaire_telephone: r.get(18)?,
                    proprietaire_id_national: r.get(19)?,
                    proprietaire_genre: r.get(20)?,
                    travailleurs_permanents: r.get(21)?,
                    travailleurs_temporaires: r.get(22)?,
                    inspecteur_nom: r.get(23)?,
                    inspecteur_annee: r.get(24)?,
                    inspecteur_mois: r.get(25)?,
                    inspecteur_jour: r.get(26)?,
                    cultures_count: r.get(27)?,
                    unites_count: r.get(28)?,
                    created_at: r.get(29)?,
                })
            })
            .optional()
            .map_err(|e| format!("Erreur de lecture : {e}"))?;

        Ok(row)
    }

    pub fn get_exploitation(&self, token: &str, id: i64) -> Result<ExploitationDetail, String> {
        let conn = self.0.lock().map_err(|_| "Base de données verrouillée".to_string())?;
        let user_id = self.require_user(&conn, token)?;

        let exploitation = self
            .load_exploitation_by_id(&conn, user_id, id)?
            .ok_or_else(|| "Exploitation introuvable".to_string())?;

        let cultures = self.load_cultures(&conn, id)?;
        let unites = self.load_unites(&conn, id)?;

        Ok(ExploitationDetail {
            exploitation,
            cultures,
            unites,
        })
    }

    pub fn produits_disponibles(&self, token: &str) -> Result<Vec<ProduitStock>, String> {
        // Lecture des réglages AVANT verrouillage (Mutex non réentrant).
        let filtre_nouveaux = self.filtre_nouveaux_sql();
        let conn = self.0.lock().map_err(|_| "Base de données verrouillée".to_string())?;
        let user_id = self.require_user(&conn, token)?;

        let (y, m, _d) = today_utc();
        let period = period_of(y, m);
        let (saison_debut, semaines) = saison_courante();
        let two_weeks_ago = date_start_days_ago(14);

        let mut stmt = conn
            .prepare(&format!(
                "SELECT c.produit,
                        COALESCE(SUM(c.recolte_estimee), 0) AS est,
                        COUNT(DISTINCT e.id) AS nb
                 FROM cultures_certifiees c
                 JOIN exploitations e ON e.id = c.exploitation_id AND e.user_id = ?1
                 WHERE c.produit IS NOT NULL AND trim(c.produit) <> ''
                   AND NOT EXISTS (
                       SELECT 1 FROM allocations a2
                       JOIN fiches_tracabilite f2 ON f2.id = a2.fiche_id
                        AND f2.created_at >= ?2
                       WHERE a2.exploitation_id = e.id
                   ){filtre_nouveaux}
                 GROUP BY c.produit",
            ))
            .map_err(|e| format!("Erreur de lecture des stocks : {e}"))?;

        let estimees: Vec<(String, f64, u64)> = stmt
            .query_map(params![user_id, two_weeks_ago], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?))
            })
            .map_err(|e| format!("Erreur de lecture des stocks : {e}"))?
            .collect::<Result<_, _>>()
            .map_err(|e| format!("Erreur de lecture des stocks : {e}"))?;

        let mut alloues: Vec<(String, f64)> = Vec::new();
        if let Ok(mut stmt2) = conn.prepare(
            "SELECT c.produit, COALESCE(SUM(a.quantite_kg), 0)
             FROM allocations a
             JOIN cultures_certifiees c ON c.id = a.culture_id
             JOIN exploitations e ON e.id = c.exploitation_id AND e.user_id = ?1
             JOIN fiches_tracabilite f ON f.id = a.fiche_id AND f.created_at >= ?2
             WHERE NOT EXISTS (
                 SELECT 1 FROM allocations a2
                 JOIN fiches_tracabilite f2 ON f2.id = a2.fiche_id
                  AND f2.created_at >= ?3
                 WHERE a2.exploitation_id = e.id
             )
             GROUP BY c.produit",
        ) {
            alloues = stmt2
                .query_map(params![user_id, saison_debut, two_weeks_ago], |r| Ok((r.get(0)?, r.get(1)?)))
                .map_err(|e| format!("Erreur de lecture des stocks : {e}"))?
                .collect::<Result<_, _>>()
                .map_err(|e| format!("Erreur de lecture des stocks : {e}"))?;
        }

        let alloc_map: std::collections::HashMap<&str, f64> =
            alloues.iter().map(|(p, a)| (p.as_str(), *a)).collect();

        // Stock livrable : estimation cumulée semaine par semaine depuis le début
        // de saison, moins les quantités déjà allouées sur la saison.
        let mut out: Vec<ProduitStock> = estimees
            .into_iter()
            .map(|(produit, est, nb)| {
                let al = alloc_map.get(produit.as_str()).copied().unwrap_or(0.0);
                let estime_saison = weekly_estimate(est, period) * semaines;
                ProduitStock {
                    produit,
                    quantite_estimee_kg: estime_saison,
                    quantite_allouee_kg: al,
                    quantite_livrable_kg: estime_saison - al,
                    nb_exploitations: nb,
                }
            })
            .collect();
        out.sort_by(|a, b| b.quantite_livrable_kg.partial_cmp(&a.quantite_livrable_kg).unwrap_or(std::cmp::Ordering::Equal));
        Ok(out)
    }

    /// Lignes de reste par culture pour un semestre :
    /// (culture_id, exploitation_id, id_interne, operateur, produit, estime_kg, alloue_kg).
    /// L'estimation suit le semestre demandé (cumul hebdo × semaines), l'alloué
    /// cumule les fiches du semestre.
    fn lignes_reste_semestre(
        conn: &Connection,
        user_id: i64,
        debut: &str,
        fin: &str,
        ratio: f64,
        semaines: f64,
    ) -> Result<Vec<(i64, i64, Option<String>, String, String, f64, f64)>, String> {
        let mut stmt = conn
            .prepare(
                "SELECT c.id, c.exploitation_id, e.id_interne,
                        trim(coalesce(e.operateur_prenom, '') || ' ' || coalesce(e.operateur_nom, '')) AS operateur,
                        c.produit, c.recolte_estimee,
                        COALESCE((SELECT SUM(a.quantite_kg) FROM allocations a
                                  JOIN fiches_tracabilite f ON f.id = a.fiche_id
                                   AND f.created_at >= ?2 AND f.created_at < ?3
                                  WHERE a.culture_id = c.id), 0) AS alloue
                 FROM cultures_certifiees c
                 JOIN exploitations e ON e.id = c.exploitation_id AND e.user_id = ?1
                 WHERE c.produit IS NOT NULL AND trim(c.produit) <> ''
                   AND c.recolte_estimee > 0
                 ORDER BY c.produit, c.id",
            )
            .map_err(|e| format!("Erreur de lecture du semestre : {e}"))?;
        let rows = stmt
            .query_map(params![user_id, debut, fin], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, f64>(5)?,
                    r.get::<_, f64>(6)?,
                ))
            })
            .map_err(|e| format!("Erreur de lecture du semestre : {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Erreur de lecture du semestre : {e}"))?;
        Ok(rows
            .into_iter()
            .map(|(cid, eid, interne, operateur, produit, recolte, alloue)| {
                let operateur = if operateur.trim().is_empty() {
                    interne.clone().unwrap_or_else(|| "Non renseigné".into())
                } else {
                    operateur
                };
                // Estimation saisonnière cumulée : base hebdo × semaines.
                let estime = recolte * ratio / WEEKS_PER_SEMESTER * semaines;
                (cid, eid, interne, operateur, produit, estime, alloue)
            })
            .collect())
    }

    /// Rapport des restes d'un semestre : estimation − alloué, par produit et
    /// par producteur. Calculé en direct (toujours exact), avec la date de
    /// clôture si le semestre a été enregistré via `cloturer_semestre`.
    pub fn rapport_semestre(
        &self,
        token: &str,
        semestre: &str,
        annee: i64,
    ) -> Result<RapportSemestre, String> {
        let ratio = ratio_semestre(semestre)?;
        valider_annee(annee)?;
        let (debut, fin) = bornes_semestre(semestre, annee)?;
        let semaines = semaines_pour_semestre(&debut, &fin);
        let conn = self.0.lock().map_err(|_| "Base de données verrouillée".to_string())?;
        let user_id = self.require_user(&conn, token)?;

        let lignes = Self::lignes_reste_semestre(&conn, user_id, &debut, &fin, ratio, semaines)?;
        let mut par_produit_map: std::collections::HashMap<String, (f64, f64)> =
            std::collections::HashMap::new();
        let mut par_producteur = Vec::with_capacity(lignes.len());
        let (mut estime, mut alloue) = (0.0, 0.0);
        for (_, eid, interne, operateur, produit, est, al) in lignes {
            estime += est;
            alloue += al;
            let entry = par_produit_map.entry(produit.clone()).or_insert((0.0, 0.0));
            entry.0 += est;
            entry.1 += al;
            par_producteur.push(ResteProducteur {
                exploitation_id: eid,
                id_interne: interne,
                operateur,
                produit: produit.clone(),
                estime_kg: est,
                alloue_kg: al,
                reste_kg: est - al,
            });
        }
        let mut producteurs_vus: std::collections::HashMap<String, std::collections::HashSet<i64>> =
            std::collections::HashMap::new();
        for p in &par_producteur {
            producteurs_vus.entry(p.produit.clone()).or_default().insert(p.exploitation_id);
        }
        let mut par_produit: Vec<ResteProduit> = par_produit_map
            .into_iter()
            .map(|(produit, (est, al))| ResteProduit {
                nb_producteurs: producteurs_vus.get(&produit).map(|s| s.len()).unwrap_or(0),
                produit,
                estime_kg: est,
                alloue_kg: al,
                reste_kg: est - al,
            })
            .collect();
        par_produit.sort_by(|a, b| a.produit.cmp(&b.produit));

        let cloture_le: Option<String> = conn
            .query_row(
                "SELECT MAX(created_at) FROM clotures_semestre WHERE user_id = ?1 AND semestre = ?2 AND annee = ?3",
                params![user_id, semestre, annee],
                |r| r.get(0),
            )
            .map_err(|e| format!("Erreur de lecture des clôtures : {e}"))?;

        Ok(RapportSemestre {
            semestre: semestre.to_string(),
            annee,
            debut,
            fin,
            estime_kg: estime,
            alloue_kg: alloue,
            reste_kg: estime - alloue,
            par_produit,
            par_producteur,
            cloture_le,
        })
    }

    /// Enregistre la clôture d'un semestre (un enregistrement par culture :
    /// estimation, alloué, reste). Idempotent : une nouvelle clôture écrase
    /// la précédente. Retourne le nombre de cultures enregistrées.
    pub fn cloturer_semestre(
        &self,
        token: &str,
        semestre: &str,
        annee: i64,
    ) -> Result<usize, String> {
        let ratio = ratio_semestre(semestre)?;
        valider_annee(annee)?;
        let (debut, fin) = bornes_semestre(semestre, annee)?;
        let semaines = semaines_pour_semestre(&debut, &fin);
        let conn = self.0.lock().map_err(|_| "Base de données verrouillée".to_string())?;
        let user_id = self.require_user(&conn, token)?;

        let lignes = Self::lignes_reste_semestre(&conn, user_id, &debut, &fin, ratio, semaines)?;
        let n = lignes.len();
        for (culture_id, _, _, _, produit, estime, alloue) in &lignes {
            conn.execute(
                "INSERT INTO clotures_semestre
                     (user_id, semestre, annee, culture_id, produit, estime_kg, alloue_kg, reste_kg)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT(user_id, semestre, annee, culture_id) DO UPDATE SET
                     produit = excluded.produit,
                     estime_kg = excluded.estime_kg,
                     alloue_kg = excluded.alloue_kg,
                     reste_kg = excluded.reste_kg,
                     created_at = datetime('now')",
                params![user_id, semestre, annee, culture_id, produit, estime, alloue, estime - alloue],
            )
            .map_err(|e| format!("Enregistrement de la clôture impossible : {e}"))?;
        }
        Ok(n)
    }

    /// Vobox : fin de semaine, le surplus de commandes émis au-dessus du
    /// livrable estimé est absorbé dans les estimations de récolte, au prorata
    /// de l'estimation de base de chaque produit. Ne s'exécute qu'une fois
    /// par semaine civile (marqueur dans les réglages).
    /// Retourne la liste { produit, facial = nouvelle estimation / ancienne }.
    pub fn ajuster_estimations_hebdomadaires(&self, token: &str) -> Result<Vec<(String, f64)>, String> {
        let semaine_courante = current_week_start();
        let derniere = self
            .get_setting("derniere_semaine_ajustee")
            .ok()
            .flatten()
            .unwrap_or_default();
        if derniere == semaine_courante {
            return Ok(Vec::new());
        }

        let debut_semaine_derniere = date_start_days_ago(7);
        let (y, m, _d) = today_utc();
        let period = period_of(y, m);
        let ratio = period_ratio(period);

        let conn = self.0.lock().map_err(|_| "Base de données verrouillée".to_string())?;
        let user_id = self.require_user(&conn, token)?;

        // Allocated par produit sur la semaine écoulée (lundi→dimanche).
        let allocated: Vec<(String, f64)> = {
            let mut stmt = conn
                .prepare(
                    "SELECT f.produit, COALESCE(SUM(a.quantite_kg), 0)
                     FROM allocations a
                     JOIN fiches_tracabilite f ON f.id = a.fiche_id
                     WHERE f.user_id = ?1
                       AND f.created_at >= ?2
                       AND f.created_at < ?3
                     GROUP BY f.produit",
                )
                .map_err(|e| format!("Préparation impossible : {e}"))?;
            let alloc_iter = stmt
                .query_map(
                    params![user_id, debut_semaine_derniere, semaine_courante],
                    |r| Ok((r.get::<_, String>(0)?, r.get::<_, f64>(1)?)),
                )
                .map_err(|e| format!("Requête impossible : {e}"))?;
            alloc_iter.collect::<Result<_, _>>().map_err(|e| format!("Lecture impossible : {e}"))?
        };

        if allocated.is_empty() {
            conn.execute(
                "INSERT INTO settings (key, value) VALUES ('derniere_semaine_ajustee', ?1)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![semaine_courante],
            )
            .map_err(|e| format!("Marquage de semaine impossible : {e}"))?;
            return Ok(Vec::new());
        }

        let mut facteurs: Vec<(String, f64)> = Vec::new();

        for (produit, alloue_semaine) in allocated {
            // Livrable estimé du produit sur cette semaine (estimation hebdo sommée).
            let livrable_semaine: f64 = conn
                .query_row(
                    "SELECT COALESCE(SUM(c.recolte_estimee), 0) * ?2 / ?3
                     FROM cultures_certifiees c
                     JOIN exploitations e ON e.id = c.exploitation_id
                     WHERE e.user_id = ?1 AND trim(lower(c.produit)) = trim(lower(?4))",
                    params![user_id, ratio, WEEKS_PER_SEMESTER, &produit],
                    |r| r.get(0),
                )
                .map_err(|e| format!("Lecture de l'estimé impossible : {e}"))?;

            if livrable_semaine <= 0.000001 {
                continue;
            }
            let surplus = (alloue_semaine - livrable_semaine).max(0.0);
            if surplus <= 0.000001 {
                continue;
            }
            // Nouvelle estimation totale du produit couvrant ce surplus.
            let est_total: f64 = conn
                .query_row(
                    "SELECT COALESCE(SUM(c.recolte_estimee), 0)
                     FROM cultures_certifiees c
                     JOIN exploitations e ON e.id = c.exploitation_id
                     WHERE e.user_id = ?1 AND trim(lower(c.produit)) = trim(lower(?2))",
                    params![user_id, &produit],
                    |r| r.get(0),
                )
                .map_err(|e| format!("Lecture impossible : {e}"))?;
            if est_total <= 0.000001 {
                continue;
            }
            let facteur = (est_total + surplus) / est_total;

            conn.execute(
                "UPDATE cultures_certifiees
                 SET recolte_estimee = recolte_estimee * ?1
                 WHERE id IN (
                     SELECT c.id FROM cultures_certifiees c
                     JOIN exploitations e ON e.id = c.exploitation_id
                     WHERE e.user_id = ?2 AND trim(lower(c.produit)) = trim(lower(?3))
                 )",
                params![facteur, user_id, &produit],
            )
            .map_err(|e| format!("Mise à jour de l'estimation impossible : {e}"))?;

            facteurs.push((produit, facteur));
        }

        conn.execute(
            "INSERT INTO settings (key, value) VALUES ('derniere_semaine_ajustee', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![semaine_courante],
        )
        .map_err(|e| format!("Marquage de semaine impossible : {e}"))?;

        Ok(facteurs)
    }

    /// Génération automatique : les quantités par producteur suivent les
    /// disponibilités hebdomadaires avec une marge aléatoire de ±5 %
    /// (tolérance sur les estimations de récolte). Toutes les commandes
    /// sont honorées en intégralité; le surplus est compensé à l'ajustement
    /// hebdo. Le mode manuel (`generer_fiche_manuelle`) garde lui les
    /// quantités exactes saisies.
    pub fn generer_fiche(
        &self,
        token: &str,
        produit: &str,
        quantite_kg: f64,
        titre: Option<&str>,
    ) -> Result<Fiche, String> {
        let mut rng = MargeRng::from_entropy();
        self.generer_fiche_impl(token, produit, quantite_kg, titre, &mut rng)
    }

    #[cfg(test)]
    fn generer_fiche_seed(
        &self,
        token: &str,
        produit: &str,
        quantite_kg: f64,
        titre: Option<&str>,
        seed: u64,
    ) -> Result<Fiche, String> {
        let mut rng = MargeRng::seeded(seed);
        self.generer_fiche_impl(token, produit, quantite_kg, titre, &mut rng)
    }

    fn generer_fiche_impl(
        &self,
        token: &str,
        produit: &str,
        quantite_kg: f64,
        titre: Option<&str>,
        rng: &mut MargeRng,
    ) -> Result<Fiche, String> {
        let produit = produit.trim().to_string();
        if produit.is_empty() {
            return Err("Produit invalide".into());
        }
        if quantite_kg <= 0.0 {
            return Err("La quantité demandée doit être strictement positive".into());
        }
        let titre = titre
            .map(|t| t.trim())
            .filter(|t| !t.is_empty())
            .unwrap_or("Fiche de traçabilité produit")
            .to_string();

        // Réglage lu AVANT verrouillage (Mutex non réentrant).
        let filtre_nouveaux = self.filtre_nouveaux_sql();
        let mut conn = self.0.lock().map_err(|_| "Base de données verrouillée".to_string())?;
        let user_id = self.require_user(&conn, token)?;
        let tx = conn
            .transaction()
            .map_err(|e| format!("Erreur de transaction : {e}"))?;

        let (y, m, _d) = today_utc();
        let period = period_of(y, m);
        let ratio = period_ratio(period);
        // Stock livrable : cumul semaine par semaine depuis le début de saison.
        let (saison_debut, semaines) = saison_courante();
        let two_weeks_ago = date_start_days_ago(14);

        let rows: Vec<(i64, i64, f64, f64)> = {
            let mut stmt = tx
                .prepare(&format!(
                    "SELECT c.id, c.exploitation_id,
                            c.recolte_estimee * ?3 / ?4 * ?7 - COALESCE(
                                (SELECT SUM(a.quantite_kg) FROM allocations a
                                 JOIN fiches_tracabilite f ON f.id = a.fiche_id
                                  AND f.created_at >= ?5
                                 WHERE a.culture_id = c.id), 0
                            ) AS dispo,
                            c.recolte_estimee AS estimee
                     FROM cultures_certifiees c
                     JOIN exploitations e ON e.id = c.exploitation_id AND e.user_id = ?1
                     WHERE trim(lower(c.produit)) = trim(lower(?2))
                       AND c.recolte_estimee > 0
                       AND NOT EXISTS (
                           SELECT 1 FROM allocations a2
                           JOIN fiches_tracabilite f2 ON f2.id = a2.fiche_id
                            AND f2.created_at >= ?6
                           WHERE a2.exploitation_id = c.exploitation_id
                       ){filtre_nouveaux}
                     ORDER BY dispo DESC, c.id ASC",
                ))
                .map_err(|e| format!("Erreur de préparation : {e}"))?;

            let fetched = stmt
                .query_map(
                    params![user_id, produit, ratio, WEEKS_PER_SEMESTER, saison_debut, two_weeks_ago, semaines],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
                )
                .map_err(|e| format!("Erreur de lecture des candidats : {e}"))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("Erreur de lecture des candidats : {e}"))?;
            fetched
        };

        // Marge aléatoire ±5 % sur la part de chaque producteur (voir `repartir_quantites`).
        // Vobox : toutes les commandes sont honorées en intégralité, même au-delà
        // du livrable estimé — le surplus est compensé à l'ajustement hebdo.
        // Poids du surplus : estimations initiales (récolte estimée) de chaque
        // producteur, pas les disponibilités, pour ne pas concentrer les grosses
        // livraisons sur 1-2 producteurs.
        let mut poids: Vec<f64> = rows.iter().map(|(_, _, _, est)| *est).collect();
        let mut somme: f64 = poids.iter().sum();
        if somme <= 0.0 && !rows.is_empty() {
            // Aucune estimation pondérable : répartition égale entre éligibles.
            poids = vec![1.0; rows.len()];
            somme = rows.len() as f64;
        }
        let (allocations, allocated_total) = repartir_quantites_pleine(
            &rows,
            &poids,
            somme,
            quantite_kg,
            rng,
        );

        if allocations.is_empty() {
            return Err(format!(
                "Aucun producteur éligible trouvé pour « {produit} »"
            ));
        }

        let year: String = tx
            .query_row("SELECT strftime('%Y','now')", [], |r| r.get(0))
            .map_err(|e| format!("Erreur de référence : {e}"))?;
        let seq: i64 = tx
            .query_row("SELECT COALESCE(MAX(id), 0) + 1 FROM fiches_tracabilite", [], |r| r.get(0))
            .map_err(|e| format!("Erreur de référence : {e}"))?;
        let reference = format!("AGB-{year}-{seq:04}");
        let complete = (allocated_total - quantite_kg).abs() < 0.000001;
        let statut = if complete { "complete" } else { "partielle" };

        tx.execute(
            "INSERT INTO fiches_tracabilite
                 (user_id, reference, titre, produit, quantite_demandee_kg, quantite_attribuee_kg, statut)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![user_id, reference, titre, produit, quantite_kg, allocated_total, statut],
        )
        .map_err(|e| format!("Erreur d'enregistrement de la fiche : {e}"))?;
        let fiche_id = tx.last_insert_rowid();

        {
            let mut insert = tx
                .prepare(
                    "INSERT INTO allocations (fiche_id, culture_id, exploitation_id, quantite_kg)
                     VALUES (?1, ?2, ?3, ?4)",
                )
                .map_err(|e| format!("Erreur d'enregistrement des allocations : {e}"))?;
            for (culture_id, exploitation_id, qty) in &allocations {
                insert
                    .execute(params![fiche_id, culture_id, exploitation_id, qty])
                    .map_err(|e| format!("Erreur d'enregistrement des allocations : {e}"))?;
            }
        }

        tx.commit().map_err(|e| format!("Erreur de validation : {e}"))?;

        let mut items = Vec::with_capacity(allocations.len());
        for (culture_id, exploitation_id, qty) in allocations {
            let exploitation = self
                .load_exploitation_by_id(&conn, user_id, exploitation_id)?
                .ok_or_else(|| "Exploitation introuvable".to_string())?;
            let unites = self.load_unites(&conn, exploitation_id)?;
            items.push(FicheItem {
                culture_id,
                quantite_kg: qty,
                exploitation,
                unites,
            });
        }

        let created_at: String = conn
            .query_row(
                "SELECT created_at FROM fiches_tracabilite WHERE id = ?1",
                params![fiche_id],
                |r| r.get(0),
            )
            .map_err(|e| format!("Erreur de lecture de la fiche : {e}"))?;

        Ok(Fiche {
            id: fiche_id,
            reference,
            titre,
            produit,
            quantite_demandee_kg: quantite_kg,
            quantite_attribuee_kg: allocated_total,
            statut: statut.into(),
            created_at,
            items,
        })
    }

    pub fn producteurs_pour_fiche(&self, token: &str, produit: &str) -> Result<Vec<ProducteurFiche>, String> {
        // Réglage lu AVANT verrouillage (Mutex non réentrant).
        let filtre_nouveaux = self.filtre_nouveaux_sql();
        let conn = self.0.lock().map_err(|_| "Base de données verrouillée".to_string())?;
        let user_id = self.require_user(&conn, token)?;
        let produit = produit.trim().to_string();
        if produit.is_empty() {
            return Err("Produit invalide".into());
        }

        let (y, m, _d) = today_utc();
        let period = period_of(y, m);
        let ratio = period_ratio(period);
        // Stock livrable : cumul saisonnier ; indicateur de participation : semaine courante.
        let (saison_debut, semaines) = saison_courante();
        let week_start = current_week_start();

        let mut stmt = conn
            .prepare(&format!(
                "SELECT c.id, c.exploitation_id,
                        c.recolte_estimee * ?3 / ?4 * ?6 - COALESCE(
                            (SELECT SUM(a.quantite_kg) FROM allocations a
                             JOIN fiches_tracabilite f ON f.id = a.fiche_id
                              AND f.created_at >= ?5
                             WHERE a.culture_id = c.id), 0
                        ) AS dispo,
                        EXISTS(
                            SELECT 1 FROM allocations a2
                            JOIN fiches_tracabilite f2 ON f2.id = a2.fiche_id
                             AND f2.created_at >= ?7
                            WHERE a2.exploitation_id = c.exploitation_id
                        ) AS part_semaine
                 FROM cultures_certifiees c
                 JOIN exploitations e ON e.id = c.exploitation_id AND e.user_id = ?1
                 WHERE trim(lower(c.produit)) = trim(lower(?2))
                   AND c.recolte_estimee > 0
                   AND dispo > 0.000001{filtre_nouveaux}
                 ORDER BY dispo DESC, c.id ASC",
            ))
            .map_err(|e| format!("Erreur de préparation : {e}"))?;

        let rows = stmt
            .query_map(params![user_id, produit, ratio, WEEKS_PER_SEMESTER, saison_debut, semaines, week_start], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, f64>(2)?,
                    r.get::<_, i64>(3)?,
                ))
            })
            .map_err(|e| format!("Erreur de lecture des producteurs : {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Erreur de lecture des producteurs : {e}"))?;

        let mut out = Vec::with_capacity(rows.len());
        for (culture_id, exploitation_id, dispo, part) in rows {
            let exploitation = self
                .load_exploitation_by_id(&conn, user_id, exploitation_id)?
                .ok_or_else(|| "Exploitation introuvable".to_string())?;
            out.push(ProducteurFiche {
                culture_id,
                exploitation,
                quantite_livrable_kg: dispo,
                participe_semaine: part != 0,
            });
        }
        Ok(out)
    }

    pub fn generer_fiche_manuelle(
        &self,
        token: &str,
        produit: &str,
        titre: Option<&str>,
        allocations: Vec<(i64, f64)>,
    ) -> Result<Fiche, String> {
        let produit = produit.trim().to_string();
        if produit.is_empty() {
            return Err("Produit invalide".into());
        }
        if allocations.is_empty() {
            return Err("Sélectionnez au moins un producteur".into());
        }
        for (_, qty) in &allocations {
            if !qty.is_finite() || *qty <= 0.0 {
                return Err("Chaque quantité doit être strictement positive".into());
            }
        }
        let total: f64 = allocations.iter().map(|(_, q)| q).sum();
        let titre = titre
            .map(|t| t.trim())
            .filter(|t| !t.is_empty())
            .unwrap_or("Fiche de traçabilité produit")
            .to_string();

        // Réglages lus AVANT verrouillage (Mutex non réentrant).
        let inclure_nouveaux = self.inclure_nouveaux();
        let mut conn = self.0.lock().map_err(|_| "Base de données verrouillée".to_string())?;
        let user_id = self.require_user(&conn, token)?;
        let tx = conn
            .transaction()
            .map_err(|e| format!("Erreur de transaction : {e}"))?;

        let (y, m, _d) = today_utc();
        let period = period_of(y, m);
        let ratio = period_ratio(period);
        // Capacité livrable : cumul saisonnier.
        let (saison_debut, semaines) = saison_courante();

        let mut stmt = tx
            .prepare(
                "SELECT c.id, c.exploitation_id, e.id_interne,
                        c.recolte_estimee * ?3 / ?4 * ?7 - COALESCE(
                            (SELECT SUM(a.quantite_kg) FROM allocations a
                             JOIN fiches_tracabilite f ON f.id = a.fiche_id
                              AND f.created_at >= ?5
                             WHERE a.culture_id = c.id), 0
                        ) AS dispo,
                        EXISTS(
                            SELECT 1 FROM allocations a0
                            WHERE a0.exploitation_id = e.id
                        ) AS deja_participe
                 FROM cultures_certifiees c
                 JOIN exploitations e ON e.id = c.exploitation_id AND e.user_id = ?1
                 WHERE trim(lower(c.produit)) = trim(lower(?2))
                   AND c.id = ?6",
            )
            .map_err(|e| format!("Erreur de préparation : {e}"))?;

        let mut inserted: Vec<(i64, i64, f64)> = Vec::with_capacity(allocations.len());
        for (culture_id, qty) in &allocations {
            let row = stmt
                .query_row(
                    params![user_id, produit, ratio, WEEKS_PER_SEMESTER, saison_debut, culture_id, semaines],
                    |r| {
                    Ok((
                        r.get::<_, i64>(0)?,
                        r.get::<_, i64>(1)?,
                        r.get::<_, Option<String>>(2)?,
                        r.get::<_, f64>(3)?,
                        r.get::<_, i64>(4)?,
                    ))
                })
                .optional()
                .map_err(|e| format!("Erreur de lecture des capacités : {e}"))?;

            let Some((cid, exploitation_id, id_interne, _dispo, deja)) = row else {
                return Err(format!("Culture introuvable ou ne correspond pas à « {produit} »"));
            };
            let qui = id_interne.unwrap_or_else(|| exploitation_id.to_string());
            if !inclure_nouveaux && deja == 0 {
                return Err(format!(
                    "Producteur « {qui} » sans participation passée : activez « Inclure les \
                     producteurs n'ayant jamais participé » dans Paramètres → Fiches, ou \
                     choisissez un producteur ayant déjà participé"
                ));
            }
            inserted.push((cid, exploitation_id, *qty));
        }
        drop(stmt);

        let year: String = tx
            .query_row("SELECT strftime('%Y','now')", [], |r| r.get(0))
            .map_err(|e| format!("Erreur de référence : {e}"))?;
        let seq: i64 = tx
            .query_row("SELECT COALESCE(MAX(id), 0) + 1 FROM fiches_tracabilite", [], |r| r.get(0))
            .map_err(|e| format!("Erreur de référence : {e}"))?;
        let reference = format!("AGB-{year}-{seq:04}");

        tx.execute(
            "INSERT INTO fiches_tracabilite
                 (user_id, reference, titre, produit, quantite_demandee_kg, quantite_attribuee_kg, statut)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'complete')",
            params![user_id, reference, titre, produit, total, total],
        )
        .map_err(|e| format!("Erreur d'enregistrement de la fiche : {e}"))?;
        let fiche_id = tx.last_insert_rowid();

        {
            let mut insert = tx
                .prepare(
                    "INSERT INTO allocations (fiche_id, culture_id, exploitation_id, quantite_kg)
                     VALUES (?1, ?2, ?3, ?4)",
                )
                .map_err(|e| format!("Erreur d'enregistrement des allocations : {e}"))?;
            for (culture_id, exploitation_id, qty) in &inserted {
                insert
                    .execute(params![fiche_id, culture_id, exploitation_id, qty])
                    .map_err(|e| format!("Erreur d'enregistrement des allocations : {e}"))?;
            }
        }

        tx.commit().map_err(|e| format!("Erreur de validation : {e}"))?;

        let mut items = Vec::with_capacity(inserted.len());
        for (culture_id, exploitation_id, qty) in inserted {
            let exploitation = self
                .load_exploitation_by_id(&conn, user_id, exploitation_id)?
                .ok_or_else(|| "Exploitation introuvable".to_string())?;
            let unites = self.load_unites(&conn, exploitation_id)?;
            items.push(FicheItem {
                culture_id,
                quantite_kg: qty,
                exploitation,
                unites,
            });
        }

        let created_at: String = conn
            .query_row(
                "SELECT created_at FROM fiches_tracabilite WHERE id = ?1",
                params![fiche_id],
                |r| r.get(0),
            )
            .map_err(|e| format!("Erreur de lecture de la fiche : {e}"))?;

        Ok(Fiche {
            id: fiche_id,
            reference,
            titre,
            produit,
            quantite_demandee_kg: total,
            quantite_attribuee_kg: total,
            statut: "complete".into(),
            created_at,
            items,
        })
    }

    pub fn historique_producteur(&self, token: &str, exploitation_id: i64) -> Result<HistoriqueProducteur, String> {
        let conn = self.0.lock().map_err(|_| "Base de données verrouillée".to_string())?;
        let user_id = self.require_user(&conn, token)?;

        self.load_exploitation_by_id(&conn, user_id, exploitation_id)?
            .ok_or_else(|| "Exploitation introuvable".to_string())?;

        let mut stmt = conn
            .prepare(
                "SELECT f.id, f.reference, f.titre, f.produit, f.statut, f.created_at,
                        a.quantite_kg
                 FROM allocations a
                 JOIN fiches_tracabilite f ON f.id = a.fiche_id
                 WHERE a.exploitation_id = ?1 AND f.user_id = ?2
                 ORDER BY f.created_at DESC, f.id DESC",
            )
            .map_err(|e| format!("Erreur de préparation : {e}"))?;

        let rows = stmt
            .query_map(params![exploitation_id, user_id], |r| {
                Ok(ParticipationProducteur {
                    fiche_id: r.get(0)?,
                    reference: r.get(1)?,
                    titre: r.get(2)?,
                    produit: r.get(3)?,
                    statut: r.get(4)?,
                    created_at: r.get(5)?,
                    quantite_kg: r.get(6)?,
                })
            })
            .map_err(|e| format!("Erreur de lecture des participations : {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Erreur de lecture des participations : {e}"))?;

        let total_kg = rows.iter().map(|p| p.quantite_kg).sum();
        Ok(HistoriqueProducteur {
            total_kg,
            nb_participations: rows.len(),
            participations: rows,
        })
    }

    pub fn producteurs_participants(
        &self,
        token: &str,
    ) -> Result<Vec<ResumeParticipation>, String> {
        let conn = self.0.lock().map_err(|_| "Base de données verrouillée".to_string())?;
        let user_id = self.require_user(&conn, token)?;

        let mut stmt = conn
            .prepare(
                "SELECT a.exploitation_id, COUNT(*), COALESCE(SUM(a.quantite_kg), 0.0)
                 FROM allocations a
                 JOIN fiches_tracabilite f ON f.id = a.fiche_id
                 WHERE f.user_id = ?1
                 GROUP BY a.exploitation_id
                 ORDER BY SUM(a.quantite_kg) DESC",
            )
            .map_err(|e| format!("Erreur de préparation : {e}"))?;

        let rows = stmt
            .query_map(params![user_id], |r| {
                Ok(ResumeParticipation {
                    exploitation_id: r.get(0)?,
                    nb_participations: r.get(1)?,
                    total_kg: r.get(2)?,
                })
            })
            .map_err(|e| format!("Erreur de lecture des participants : {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Erreur de lecture des participants : {e}"))?;
        Ok(rows)
    }

    pub fn list_fiches(&self, token: &str) -> Result<Vec<FicheSummary>, String> {
        let conn = self.0.lock().map_err(|_| "Base de données verrouillée".to_string())?;
        let user_id = self.require_user(&conn, token)?;

        let mut stmt = conn
            .prepare(
                "SELECT f.id, f.reference, f.titre, f.produit, f.quantite_demandee_kg,
                        f.quantite_attribuee_kg, f.statut, f.created_at,
                        (SELECT COUNT(DISTINCT a.exploitation_id) FROM allocations a WHERE a.fiche_id = f.id)
                 FROM fiches_tracabilite f
                 WHERE f.user_id = ?1
                 ORDER BY f.created_at DESC, f.id DESC",
            )
            .map_err(|e| format!("Erreur de lecture des fiches : {e}"))?;

        let rows = stmt
            .query_map(params![user_id], |r| {
                Ok(FicheSummary {
                    id: r.get(0)?,
                    reference: r.get(1)?,
                    titre: r.get(2)?,
                    produit: r.get(3)?,
                    quantite_demandee_kg: r.get(4)?,
                    quantite_attribuee_kg: r.get(5)?,
                    statut: r.get(6)?,
                    created_at: r.get(7)?,
                    nb_producteurs: r.get(8)?,
                })
            })
            .map_err(|e| format!("Erreur de lecture des fiches : {e}"))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Erreur de lecture des fiches : {e}"))
    }

    pub fn get_fiche(&self, token: &str, id: i64) -> Result<Fiche, String> {
        let conn = self.0.lock().map_err(|_| "Base de données verrouillée".to_string())?;
        let user_id = self.require_user(&conn, token)?;

        let row = conn
            .query_row(
                "SELECT f.id, f.reference, f.titre, f.produit, f.quantite_demandee_kg,
                        f.quantite_attribuee_kg, f.statut, f.created_at
                 FROM fiches_tracabilite f
                 WHERE f.id = ?1 AND f.user_id = ?2",
                params![id, user_id],
                |r| {
                    Ok((
                        r.get::<_, i64>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, String>(3)?,
                        r.get::<_, f64>(4)?,
                        r.get::<_, f64>(5)?,
                        r.get::<_, String>(6)?,
                        r.get::<_, String>(7)?,
                    ))
                },
            )
            .map_err(|e| format!("Erreur de lecture de la fiche : {e}"))?;

        let (fiche_id, reference, titre, produit, demandee, attribuee, statut, created_at) = row;

        let mut stmt = conn
            .prepare(
                "SELECT a.culture_id, a.quantite_kg, a.exploitation_id
                 FROM allocations a
                 WHERE a.fiche_id = ?1
                 ORDER BY a.id ASC",
            )
            .map_err(|e| format!("Erreur de lecture de la fiche : {e}"))?;
        let allocs: Vec<(i64, f64, i64)> = stmt
            .query_map(params![fiche_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .map_err(|e| format!("Erreur de lecture de la fiche : {e}"))?
            .collect::<Result<_, _>>()
            .map_err(|e| format!("Erreur de lecture de la fiche : {e}"))?;

        let mut items = Vec::with_capacity(allocs.len());
        for (culture_id, qty, exploitation_id) in allocs {
            let exploitation = self
                .load_exploitation_by_id(&conn, user_id, exploitation_id)?
                .ok_or_else(|| "Exploitation introuvable".to_string())?;
            let unites = self.load_unites(&conn, exploitation_id)?;
            items.push(FicheItem {
                culture_id,
                quantite_kg: qty,
                exploitation,
                unites,
            });
        }

        Ok(Fiche {
            id: fiche_id,
            reference,
            titre,
            produit,
            quantite_demandee_kg: demandee,
            quantite_attribuee_kg: attribuee,
            statut,
            created_at,
            items,
        })
    }

    fn load_exploitation_by_id(
        &self,
        conn: &Connection,
        user_id: i64,
        id: i64,
    ) -> Result<Option<Exploitation>, String> {
        let mut stmt = conn
            .prepare(
                "SELECT e.id, e.id_interne, e.id_national, e.village, e.district,
                        e.region_inspection, e.superficie_total, e.type_exploitation,
                        e.nb_unites, e.nb_cultures,
                        e.operateur_prenom, e.operateur_nom, e.operateur_telephone,
                        e.operateur_id_national, e.operateur_genre, e.operateur_annee_nais,
                        e.proprietaire_prenom, e.proprietaire_nom, e.proprietaire_telephone,
                        e.proprietaire_id_national, e.proprietaire_genre,
                        e.travailleurs_permanents, e.travailleurs_temporaires,
                        e.inspecteur_nom, e.inspecteur_annee, e.inspecteur_mois, e.inspecteur_jour,
                        (SELECT COUNT(*) FROM cultures_certifiees c WHERE c.exploitation_id = e.id),
                        (SELECT COUNT(*) FROM unites_agricoles u WHERE u.exploitation_id = e.id),
                        e.created_at
                 FROM exploitations e
                 WHERE e.user_id = ?1 AND e.id = ?2",
            )
            .map_err(|e| format!("Erreur de lecture : {e}"))?;

        let row = stmt
            .query_row(params![user_id, id], |r| {
                Ok(Exploitation {
                    id: r.get(0)?,
                    id_interne: r.get(1)?,
                    id_national: r.get(2)?,
                    village: r.get(3)?,
                    district: r.get(4)?,
                    region_inspection: r.get(5)?,
                    superficie_total: r.get(6)?,
                    type_exploitation: r.get(7)?,
                    nb_unites: r.get(8)?,
                    nb_cultures: r.get(9)?,
                    operateur_prenom: r.get(10)?,
                    operateur_nom: r.get(11)?,
                    operateur_telephone: r.get(12)?,
                    operateur_id_national: r.get(13)?,
                    operateur_genre: r.get(14)?,
                    operateur_annee_nais: r.get(15)?,
                    proprietaire_prenom: r.get(16)?,
                    proprietaire_nom: r.get(17)?,
                    proprietaire_telephone: r.get(18)?,
                    proprietaire_id_national: r.get(19)?,
                    proprietaire_genre: r.get(20)?,
                    travailleurs_permanents: r.get(21)?,
                    travailleurs_temporaires: r.get(22)?,
                    inspecteur_nom: r.get(23)?,
                    inspecteur_annee: r.get(24)?,
                    inspecteur_mois: r.get(25)?,
                    inspecteur_jour: r.get(26)?,
                    cultures_count: r.get(27)?,
                    unites_count: r.get(28)?,
                    created_at: r.get(29)?,
                })
            })
            .optional()
            .map_err(|e| format!("Erreur de lecture : {e}"))?;

        Ok(row)
    }

    fn load_cultures(&self, conn: &Connection, exploitation_id: i64) -> Result<Vec<Culture>, String> {
        let mut stmt = conn
            .prepare(
                "SELECT id, id_interne, produit, variete, superficie_ha,
                        recolte_estimee, recolte_precedente, volume_vendu
                 FROM cultures_certifiees WHERE exploitation_id = ?1
                 ORDER BY id ASC",
            )
            .map_err(|e| format!("Erreur de lecture des cultures : {e}"))?;

        let rows = stmt
            .query_map(params![exploitation_id], |r| {
                Ok(Culture {
                    id: r.get(0)?,
                    id_interne: r.get(1)?,
                    produit: r.get(2)?,
                    variete: r.get(3)?,
                    superficie_ha: r.get(4)?,
                    recolte_estimee: r.get(5)?,
                    recolte_precedente: r.get(6)?,
                    volume_vendu: r.get(7)?,
                })
            })
            .map_err(|e| format!("Erreur de lecture des cultures : {e}"))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Erreur de lecture des cultures : {e}"))
    }

    fn load_unites(&self, conn: &Connection, exploitation_id: i64) -> Result<Vec<Unite>, String> {
        let mut stmt = conn
            .prepare(
                "SELECT id, id_interne, identifiant_unite, superficie_ha, latitude, longitude
                 FROM unites_agricoles WHERE exploitation_id = ?1
                 ORDER BY id ASC",
            )
            .map_err(|e| format!("Erreur de lecture des unités : {e}"))?;

        let rows = stmt
            .query_map(params![exploitation_id], |r| {
                Ok(Unite {
                    id: r.get(0)?,
                    id_interne: r.get(1)?,
                    identifiant_unite: r.get(2)?,
                    superficie_ha: r.get(3)?,
                    latitude: r.get(4)?,
                    longitude: r.get(5)?,
                })
            })
            .map_err(|e| format!("Erreur de lecture des unités : {e}"))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Erreur de lecture des unités : {e}"))
    }
}

fn extract_terms(input: &str) -> Vec<String> {
    let mut terms: Vec<String> = Vec::new();
    let mut cur = String::new();
    for ch in input.chars() {
        if ch.is_alphanumeric() {
            cur.push(ch);
        } else if !cur.is_empty() {
            terms.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        terms.push(cur);
    }
    terms.retain(|t| t.chars().count() >= 2);
    terms
}

fn build_fts_query(input: &str, category: SearchCategory) -> String {
    let terms = extract_terms(input);
    if terms.is_empty() {
        return String::new();
    }
    let prefixes: Vec<String> = terms.iter().map(|t| format!("{t}*")).collect();
    match category.column_filter() {
        None => prefixes.join(" "),
        Some(col) => prefixes
            .iter()
            .map(|p| format!("{col} : {p}"))
            .collect::<Vec<_>>()
            .join(" "),
    }
}

fn push_field(parts: &mut Vec<String>, field: Option<&String>) {
    if let Some(v) = field {
        let t = v.trim();
        if !t.is_empty() {
            parts.push(t.to_string());
        }
    }
}

fn join_fields(fields: &[Option<&String>]) -> String {
    let mut parts: Vec<String> = Vec::new();
    for &f in fields {
        push_field(&mut parts, f);
    }
    parts.join(" ")
}

struct SearchColumns {
    operateur: String,
    localisation: String,
    culture: String,
    unite: String,
    identifiant: String,
    inspecteur: String,
}

fn build_search_columns(e: &Exploitation, cultures: &[Culture], unites: &[Unite]) -> SearchColumns {
    let operateur = join_fields(&[
        e.operateur_prenom.as_ref(),
        e.operateur_nom.as_ref(),
        e.operateur_telephone.as_ref(),
        e.operateur_id_national.as_ref(),
        e.operateur_genre.as_ref(),
        e.proprietaire_prenom.as_ref(),
        e.proprietaire_nom.as_ref(),
        e.proprietaire_telephone.as_ref(),
        e.proprietaire_id_national.as_ref(),
        e.proprietaire_genre.as_ref(),
    ]);

    let localisation = join_fields(&[
        e.village.as_ref(),
        e.district.as_ref(),
        e.region_inspection.as_ref(),
    ]);

    let mut culture_parts: Vec<String> = Vec::new();
    for c in cultures {
        push_field(&mut culture_parts, c.produit.as_ref());
        push_field(&mut culture_parts, c.variete.as_ref());
    }
    let culture = culture_parts.join(" ");

    let mut unite_parts: Vec<String> = Vec::new();
    for u in unites {
        push_field(&mut unite_parts, u.identifiant_unite.as_ref());
        push_field(&mut unite_parts, u.id_interne.as_ref());
    }
    let unite = unite_parts.join(" ");

    let identifiant = join_fields(&[
        e.id_interne.as_ref(),
        e.id_national.as_ref(),
        e.type_exploitation.as_ref(),
    ]);

    let inspecteur = join_fields(&[e.inspecteur_nom.as_ref()]);

    SearchColumns {
        operateur,
        localisation,
        culture,
        unite,
        identifiant,
        inspecteur,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::excel::{CultureDraft, ExploitationDraft, ParsedWorkbook, UniteDraft};

    #[test]
    fn construire_requete_fts() {
        assert_eq!(build_fts_query("ALIOU", SearchCategory::All), "ALIOU*");
        assert_eq!(build_fts_query("Alou traore", SearchCategory::All), "Alou* traore*");
        assert_eq!(build_fts_query("CAPKO_001", SearchCategory::All), "CAPKO* 001*");
        assert_eq!(build_fts_query("B-2025-GMR", SearchCategory::All), "2025* GMR*");
        assert_eq!(build_fts_query("Kàkólópózó", SearchCategory::All), "Kàkólópózó*");
        assert_eq!(build_fts_query("###", SearchCategory::All), "");
        assert_eq!(build_fts_query("", SearchCategory::All), "");
        assert_eq!(build_fts_query("x y", SearchCategory::All), "");

        assert_eq!(build_fts_query("alio", SearchCategory::Operateur), "operateur : alio*");
        assert_eq!(build_fts_query("koko kaka", SearchCategory::Localisation), "localisation : koko* localisation : kaka*");
        assert_eq!(build_fts_query("cacao", SearchCategory::Culture), "culture : cacao*");
        assert_eq!(build_fts_query("u-kokolo", SearchCategory::Unite), "unite : kokolo*");
        assert_eq!(build_fts_query("capko", SearchCategory::Identifiant), "identifiant : capko*");
        assert_eq!(build_fts_query("soug", SearchCategory::Inspecteur), "inspecteur : soug*");
    }

    #[test]
    fn rechercher_dans_l_index_fts() {
        let db = Db::init(Path::new(":memory:")).expect("base");
        let session = db.register("Test B", "test@agro.app", "secret1").expect("compte");
        let token = &session.token;

        let mut data = ParsedWorkbook::default();
        data.exploitations.push(ExploitationDraft {
            id_interne: Some("CAPKO_001".into()),
            id_national: Some("CI-2025-0001".into()),
            village: Some("KOKOLOPOZO".into()),
            district: Some("SASANDRA".into()),
            region_inspection: Some("BAS SASANDRA".into()),
            type_exploitation: Some("Petite plantation".into()),
            operateur_prenom: Some("ALIOU".into()),
            operateur_nom: Some("TRAORE".into()),
            operateur_telephone: Some("0103315797".into()),
            inspecteur_nom: Some("SOUGUE DAOUDA".into()),
            inspecteur_annee: Some(2026),
            inspecteur_jour: Some(23),
            inspecteur_mois: Some(12),
            ..ExploitationDraft::empty()
        });
        data.cultures.push(CultureDraft {
            id_interne: Some("CAPKO_001".into()),
            produit: Some("CACAO".into()),
            variete: Some("Mercedes".into()),
            ..CultureDraft::empty()
        });
        data.unites.push(UniteDraft {
            id_interne: Some("CAPKO_001".into()),
            identifiant_unite: Some("U-KOKOLO-1".into()),
            ..UniteDraft::empty()
        });

        db.import_workbook(token, &data).expect("import");

        let all = SearchCategory::All;
        assert_eq!(db.search_exploitations(token, "alio", all).expect("search").len(), 1, "opérateur, préfixe");
        assert_eq!(db.search_exploitations(token, "ALIOU TRAO", all).expect("search").len(), 1, "tokens multiples");
        assert_eq!(db.search_exploitations(token, "KOKOLOPOZO", all).expect("search").len(), 1, "village");
        assert_eq!(db.search_exploitations(token, "cacao", all).expect("search").len(), 1, "produit de culture");
        assert_eq!(db.search_exploitations(token, "U-KOKOLO", all).expect("search").len(), 1, "identifiant d'unité");
        assert_eq!(db.search_exploitations(token, "CAPKO", all).expect("search").len(), 1, "ID interne");
        assert_eq!(db.search_exploitations(token, "0103315", all).expect("search").len(), 1, "téléphone");
        assert_eq!(db.search_exploitations(token, "soug", all).expect("search").len(), 1, "inspecteur");

        assert_eq!(db.search_exploitations(token, "cacao", SearchCategory::Culture).expect("search").len(), 1, "culture dans Culture");
        assert_eq!(db.search_exploitations(token, "cacao", SearchCategory::Localisation).expect("search").len(), 0, "culture absente de Localisation");
        assert_eq!(db.search_exploitations(token, "trao", SearchCategory::Operateur).expect("search").len(), 1, "opérateur dans Operateur");
        assert_eq!(db.search_exploitations(token, "trao", SearchCategory::Culture).expect("search").len(), 0, "prénom absent de Culture");
        assert_eq!(db.search_exploitations(token, "kokolo", SearchCategory::Unite).expect("search").len(), 1, "unité dans Unité");
        assert_eq!(db.search_exploitations(token, "kokolo", SearchCategory::Localisation).expect("search").len(), 1, "village dans Localisation");
        assert_eq!(db.search_exploitations(token, "kokolo", SearchCategory::Identifiant).expect("search").len(), 0, "unité absente d'Identifiant");
        assert_eq!(db.search_exploitations(token, "soug", SearchCategory::Inspecteur).expect("search").len(), 1, "inspecteur dans Inspecteur");
        assert_eq!(db.search_exploitations(token, "soug", SearchCategory::Operateur).expect("search").len(), 0, "inspecteur absent d'Operateur");

        assert!(db.search_exploitations(token, "zzz", all).expect("search").is_empty(), "aucune correspondance");
        assert!(db.search_exploitations(token, "###", all).expect("search").is_empty(), "requête non pertinente");

        db.reset_data(token).expect("reset");
        assert!(db.search_exploitations(token, "alio", all).expect("search").is_empty(), "index vidé après reset");
        assert!(db.list_exploitations(token).expect("liste").is_empty(), "données vidées");
    }

    fn db_avec_cacao() -> (Db, String) {
        let db = Db::init(Path::new(":memory:")).expect("base");
        let session = db.register("Test B", "test@agro.app", "secret1").expect("compte");
        let token = session.token.clone();

        let mut data = ParsedWorkbook::default();
        data.exploitations.push(ExploitationDraft {
            id_interne: Some("CAPKO_001".into()),
            operateur_prenom: Some("ALIOU".into()),
            operateur_nom: Some("TRAORE".into()),
            operateur_telephone: Some("0103315797".into()),
            village: Some("KOKOLOPOZO".into()),
            district: Some("SASANDRA".into()),
            ..ExploitationDraft::empty()
        });
        data.exploitations.push(ExploitationDraft {
            id_interne: Some("CAPKO_002".into()),
            operateur_prenom: Some("MARIAM".into()),
            operateur_nom: Some("KEBE".into()),
            operateur_telephone: Some("0103315899".into()),
            village: Some("KOKOLOPOZO".into()),
            district: Some("SASANDRA".into()),
            ..ExploitationDraft::empty()
        });
        data.cultures.push(CultureDraft {
            id_interne: Some("CAPKO_001".into()),
            produit: Some("cacao".into()),
            recolte_estimee: Some(2000.0),
            ..CultureDraft::empty()
        });
        data.cultures.push(CultureDraft {
            id_interne: Some("CAPKO_002".into()),
            produit: Some("cacao".into()),
            recolte_estimee: Some(800.0),
            ..CultureDraft::empty()
        });
        data.unites.push(UniteDraft {
            id_interne: Some("CAPKO_001".into()),
            identifiant_unite: Some("UA-1".into()),
            superficie_ha: Some(2.47),
            latitude: Some(5.143708),
            longitude: Some(-6.074116),
        });

        db.import_workbook(&token, &data).expect("import");
        (db, token)
    }

    fn hebdo_cacao() -> (f64, f64) {
        let (y, m, _d) = today_utc();
        let p = period_of(y, m);
        (weekly_estimate(2000.0, p), weekly_estimate(800.0, p))
    }

    /// Semaines cumulées de la saison en cours (factorise les attentes saisonnières).
    fn saison_semaines() -> f64 {
        saison_courante().1
    }

    #[test]
    fn marge_plus_moins_5_pour_cent() {
        // Stock suffisant : le total demandé est atteint exactement,
        // chaque part non finale reste dans ±5 % de sa disponibilité.
        let lignes = vec![(1, 10, 100.0), (2, 20, 50.0)];
        for seed in 0..50u64 {
            let mut rng = MargeRng::seeded(seed);
            let (allocs, total) = repartir_quantites(&lignes, 120.0, &mut rng);
            assert!((total - 120.0).abs() < 0.0001, "seed {seed} : total exact");
            assert!(!allocs.is_empty());
            for (i, &(_, _, qty)) in allocs.iter().enumerate() {
                let dispo = lignes[i].2;
                assert!(qty > 0.0, "seed {seed} : part positive");
                assert!(
                    qty <= dispo * MARGE_MAX + 0.0001,
                    "seed {seed} : part {qty} ≤ +5 % de {dispo}"
                );
                if i + 1 < allocs.len() {
                    // Part non finale : pas écrêtée par le reste, donc ≥ −5 %.
                    assert!(
                        qty >= dispo * MARGE_MIN - 0.0001,
                        "seed {seed} : part {qty} ≥ −5 % de {dispo}"
                    );
                }
            }
        }

        // Stock insuffisant : tout est attribué, chaque part dans ±5 % de sa dispo.
        for seed in 0..50u64 {
            let mut rng = MargeRng::seeded(seed);
            let (allocs, total) = repartir_quantites(&lignes, 200.0, &mut rng);
            assert_eq!(allocs.len(), 2);
            assert!((total - (allocs[0].2 + allocs[1].2)).abs() < 0.0001);
            for &(cid, _, qty) in &allocs {
                let dispo = if cid == 1 { 100.0 } else { 50.0 };
                assert!(
                    (qty - dispo).abs() <= dispo * 0.05 + 0.0001,
                    "seed {seed} : {qty} dans ±5 % de {dispo}"
                );
            }
        }

        // Le total ne dépasse jamais la demande.
        for seed in 0..50u64 {
            let mut rng = MargeRng::seeded(seed);
            let (_, total) = repartir_quantites(&lignes, 60.0, &mut rng);
            assert!(total <= 60.0 + 0.0001, "seed {seed} : pas de dépassement");
        }

        // Déterminisme : même graine, même répartition.
        let mut a = MargeRng::seeded(7);
        let mut b = MargeRng::seeded(7);
        assert_eq!(
            repartir_quantites(&lignes, 120.0, &mut a).0,
            repartir_quantites(&lignes, 120.0, &mut b).0
        );

        // Le facteur couvre bien les deux côtés de 1.0 (aléatoire effectif).
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;
        for seed in 0..200u64 {
            let mut rng = MargeRng::seeded(seed);
            let f = rng.facteur_marge();
            assert!((MARGE_MIN..=MARGE_MAX).contains(&f), "facteur {f} hors borne");
            min = min.min(f);
            max = max.max(f);
        }
        assert!(min < 1.0 && max > 1.0, "marge effective des deux côtés (min={min}, max={max})");
    }

    #[test]
    fn saison_courante_bornee() {
        let (debut, semaines) = saison_courante();
        assert!((1.0..=24.0).contains(&semaines), "semaines={semaines}");
        assert_eq!(debut.len(), 10, "format YYYY-MM-DD");
        let mois: u32 = debut[5..7].parse().expect("mois");
        // Cohérence avec la période : S1 ↔ septembre, S2 ↔ mars.
        let (_y, m, _d) = today_utc();
        let saison_attendue = if (3..=8).contains(&m) { 3 } else { 9 };
        assert_eq!(mois, saison_attendue);
    }

    #[test]
    fn cumul_saisonnier_semaine_par_semaine() {
        // Livrable = estimation hebdo × semaines écoulées, moins l'alloué saison.
        let (db, token) = db_avec_cacao();
        let (h1, h2) = hebdo_cacao();
        let s = saison_semaines();
        let stocks = db.produits_disponibles(&token).expect("stocks");
        assert!((stocks[0].quantite_livrable_kg - (h1 + h2) * s).abs() < 0.01);
        assert!(
            (stocks[0].quantite_estimee_kg
                - stocks[0].quantite_livrable_kg
                - stocks[0].quantite_allouee_kg)
                .abs()
                < 0.01,
            "estimation − livrable − alloué = 0"
        );
    }

    #[test]
    fn filtre_nouveaux_producteurs_optionnel() {
        let (db, token) = db_avec_cacao();
        // Défaut : inclus (comportement historique).
        assert!(db.inclure_nouveaux(), "inclus par défaut");
        let liste_avant = db.producteurs_pour_fiche(&token, "cacao").expect("liste");
        assert_eq!(liste_avant.len(), 2);
        let id2 = liste_avant[1].culture_id;

        // Exclus : sans participation passée → ni aperçu, ni génération auto, ni manuel.
        db.set_inclure_nouveaux(false).expect("réglage");
        assert!(!db.inclure_nouveaux());
        assert!(db.producteurs_pour_fiche(&token, "cacao").expect("liste").is_empty());
        let err = db.generer_fiche(&token, "cacao", 10.0, None).expect_err("aucun éligible");
        assert!(err.contains("Aucun producteur éligible"), "erreur: {err}");
        let err_manuel = db
            .generer_fiche_manuelle(&token, "cacao", None, vec![(id2, 5.0)])
            .expect_err("jamais participé");
        assert!(err_manuel.contains("sans participation"), "erreur: {err_manuel}");

        // Après une participation, le producteur redevient éligible.
        db.set_inclure_nouveaux(true).expect("réglage");
        let (h1, _h2) = hebdo_cacao();
        db.generer_fiche_seed(&token, "cacao", h1 * 0.5, None, 1).expect("fiche");
        db.set_inclure_nouveaux(false).expect("réglage");
        let liste = db.producteurs_pour_fiche(&token, "cacao").expect("liste");
        assert_eq!(liste.len(), 1, "seul l'ancien participant reste");
        assert_eq!(liste[0].exploitation.id_interne.as_deref(), Some("CAPKO_001"));
    }

    #[test]
    fn generer_fiche_selectionne_et_verrouille() {
        let (db, token) = db_avec_cacao();
        let (h1, h2) = hebdo_cacao();
        let total = h1 + h2;
        // Stock livrable : cumul saisonnier semaine par semaine.
        let total_saison = total * saison_semaines();

        let stocks = db.produits_disponibles(&token).expect("stocks");
        assert_eq!(stocks.len(), 1);
        assert_eq!(stocks[0].produit, "cacao");
        assert!((stocks[0].quantite_estimee_kg - total_saison).abs() < 0.01, "estimation saisonnière cumulée");
        assert!((stocks[0].quantite_livrable_kg - total_saison).abs() < 0.01);
        assert_eq!(stocks[0].nb_exploitations, 2);

        let fiche = db.generer_fiche_seed(&token, "cacao", h1, None, 1).expect("fiche");
        assert_eq!(fiche.statut, "complete");
        assert_eq!(fiche.titre, "Fiche de traçabilité produit", "titre par défaut");
        assert!((fiche.quantite_attribuee_kg - h1).abs() < 0.0001);
        assert_eq!(fiche.items.len(), 1, "un seul producteur couvre la demande");
        assert_eq!(fiche.items[0].exploitation.id_interne.as_deref(), Some("CAPKO_001"));
        assert!((fiche.items[0].quantite_kg - h1).abs() < 0.0001);
        assert!(fiche.reference.starts_with("AGB-"), "référence AGB");
        assert_eq!(fiche.items[0].unites.len(), 1, "parcelles incluses dans la fiche");

        let fiche2 = db.generer_fiche_seed(&token, "cacao", h2, Some("Client EXIM SA"), 1).expect("fiche2");
        assert_eq!(fiche2.statut, "complete");
        assert_eq!(fiche2.titre, "Client EXIM SA", "titre personnalisé");
        assert!((fiche2.quantite_attribuee_kg - h2).abs() < 0.0001);
        assert_eq!(fiche2.items.len(), 1);
        assert_eq!(fiche2.items[0].exploitation.id_interne.as_deref(), Some("CAPKO_002"));
        assert_ne!(fiche.reference, fiche2.reference, "références uniques");

        let err = db.generer_fiche_seed(&token, "cacao", 1.0, None, 1).expect_err("stock épuisé");
        assert!(err.contains("Aucun producteur éligible"), "erreur: {err}");

        let stocks2 = db.produits_disponibles(&token).expect("stocks");
        assert!(stocks2.is_empty(), "plus aucun producteur éligible cette semaine (budget + carence 2 semaines)");

        let fiches = db.list_fiches(&token).expect("liste");
        assert_eq!(fiches.len(), 2);
        assert_eq!(fiches[0].nb_producteurs, 1);
    }

    #[test]
    fn generer_fiche_complete_au_dela_du_stock() {
        let (db, token) = db_avec_cacao();
        let (h1, h2) = hebdo_cacao();
        // Demande au-delà du stock saisonnier même avec la marge +5 %.
        let total_saison = (h1 + h2) * saison_semaines();
        let demande = total_saison * 1.10 + 50.0;

        let fiche = db.generer_fiche_seed(&token, "cacao", demande, None, 5).expect("fiche intégrale");
        assert_eq!(fiche.statut, "complete", "toute commande est honorée intégralement");
        assert_eq!(fiche.items.len(), 2);
        // Le surplus est réparti : chaque part couvre au moins sa disponibilité (±5 %),
        // et le total attribué = demande intégrale.
        let s = saison_semaines();
        for item in &fiche.items {
            let dispo = if item.exploitation.id_interne.as_deref() == Some("CAPKO_001") {
                h1 * s
            } else {
                h2 * s
            };
            assert!(
                item.quantite_kg >= dispo * MARGE_MIN - 0.01,
                "part {} ≥ livrable {dispo} (marge -5 %), surplus ajouté",
                item.quantite_kg
            );
        }
        assert!(
            (fiche.quantite_attribuee_kg - demande).abs() < 0.0001,
            "attribué {} = demande {demande}",
            fiche.quantite_attribuee_kg
        );

        // Les deux producteurs ayant participé, plus personne n'est éligible.
        let err = db.generer_fiche_seed(&token, "cacao", 1.0, None, 1).expect_err("épuisé");
        assert!(err.contains("Aucun producteur éligible"), "erreur: {err}");

        let err2 = db.generer_fiche_seed(&token, "cafe", 1.0, None, 1).expect_err("produit inconnu");
        assert!(err2.contains("Aucun producteur éligible"), "erreur: {err2}");
    }

    #[test]
    fn liste_producteurs_manuels_par_capacite() {
        let (db, token) = db_avec_cacao();
        let (h1, h2) = hebdo_cacao();

        let liste = db.producteurs_pour_fiche(&token, "cacao").expect("liste");
        assert_eq!(liste.len(), 2);
        assert_eq!(liste[0].exploitation.id_interne.as_deref(), Some("CAPKO_001"));
        assert!((liste[0].quantite_livrable_kg - h1 * saison_semaines()).abs() < 0.01);
        assert!(!liste[0].participe_semaine, "aucune participation au départ");
        assert_eq!(liste[1].exploitation.id_interne.as_deref(), Some("CAPKO_002"));
        assert!((liste[1].quantite_livrable_kg - h2 * saison_semaines()).abs() < 0.01);

        let vide = db.producteurs_pour_fiche(&token, "cafe").expect("liste vide");
        assert!(vide.is_empty());
    }

    #[test]
    fn producteur_participe_deja_cette_semaine() {
        let (db, token) = db_avec_cacao();
        let (h1, _h2) = hebdo_cacao();

        // CAPKO_001 consomme la moitié d'une semaine de stock saisonnier.
        db.generer_fiche_seed(&token, "cacao", h1 * 0.5, None, 1).expect("fiche");

        let liste = db.producteurs_pour_fiche(&token, "cacao").expect("liste");
        assert_eq!(liste.len(), 2, "le producteur garde de la capacité");
        let capko1 = liste
            .iter()
            .find(|p| p.exploitation.id_interne.as_deref() == Some("CAPKO_001"))
            .expect("CAPKO_001");
        assert!(capko1.participe_semaine, "déjà affecté cette semaine");
        let reste_attendu = h1 * saison_semaines() - h1 * 0.5;
        assert!((capko1.quantite_livrable_kg - reste_attendu).abs() < 0.01, "livrable restant");

        let capko2 = liste
            .iter()
            .find(|p| p.exploitation.id_interne.as_deref() == Some("CAPKO_002"))
            .expect("CAPKO_002");
        assert!(!capko2.participe_semaine);
    }

    #[test]
    fn generer_fiche_manuelle_repartition() {
        let (db, token) = db_avec_cacao();
        let (h1, h2) = hebdo_cacao();
        let liste = db.producteurs_pour_fiche(&token, "cacao").expect("liste");
        let (id1, id2) = (liste[0].culture_id, liste[1].culture_id);

        let quota2 = h2 * 0.5;
        let total = h1 + quota2;
        let fiche = db
            .generer_fiche_manuelle(&token, "cacao", Some("Commande manuelle"), vec![(id1, h1), (id2, quota2)])
            .expect("fiche manuelle");
        assert_eq!(fiche.statut, "complete");
        assert_eq!(fiche.titre, "Commande manuelle");
        assert!((fiche.quantite_demandee_kg - total).abs() < 0.0001);
        assert!((fiche.quantite_attribuee_kg - total).abs() < 0.0001);
        assert_eq!(fiche.items.len(), 2);
        assert!((fiche.items[0].quantite_kg - h1).abs() < 0.0001);
        assert!((fiche.items[1].quantite_kg - quota2).abs() < 0.0001);
        assert!(fiche.reference.starts_with("AGB-"));
    }

    #[test]
    fn generer_fiche_manuelle_validation() {
        let (db, token) = db_avec_cacao();
        let (h1, _h2) = hebdo_cacao();
        let liste = db.producteurs_pour_fiche(&token, "cacao").expect("liste");
        let id1 = liste[0].culture_id;

        // Vobox : la fiche manuelle honore l'intégralité, même au-delà du livrable.
        let trop = db
            .generer_fiche_manuelle(&token, "cacao", None, vec![(id1, h1 * saison_semaines() + 1.0)])
            .expect("livrable dépassé mais commande honorée");
        assert_eq!(trop.statut, "complete");
        assert!((trop.quantite_attribuee_kg - (h1 * saison_semaines() + 1.0)).abs() < 0.0001);

        let inconnu = db
            .generer_fiche_manuelle(&token, "cafe", None, vec![(id1, 10.0)])
            .expect_err("culture hors produit");
        assert!(inconnu.contains("Culture introuvable"), "erreur: {inconnu}");

        let vide = db.generer_fiche_manuelle(&token, "cacao", None, vec![]).expect_err("vide");
        assert!(vide.contains("au moins un producteur"), "erreur: {vide}");

        let negatif = db
            .generer_fiche_manuelle(&token, "cacao", None, vec![(id1, -5.0)])
            .expect_err("quantité négative");
        assert!(negatif.contains("strictement positive"), "erreur: {negatif}");
    }

    #[test]
    fn historique_participation_producteur() {
        let (db, token) = db_avec_cacao();
        let (h1, h2) = hebdo_cacao();

        db.generer_fiche_seed(&token, "cacao", h1, None, 1).expect("fiche 1");
        db.generer_fiche_seed(&token, "cacao", h2, Some("Second contrat"), 1).expect("fiche 2");

        let exs = db.search_exploitations(&token, "capko", SearchCategory::All).expect("recherche");
        let capko1 = exs
            .iter()
            .find(|e| e.id_interne.as_deref() == Some("CAPKO_001"))
            .expect("CAPKO_001")
            .id;
        let capko2 = exs
            .iter()
            .find(|e| e.id_interne.as_deref() == Some("CAPKO_002"))
            .expect("CAPKO_002")
            .id;

        let hist1 = db.historique_producteur(&token, capko1).expect("historique 1");
        assert_eq!(hist1.nb_participations, 1);
        assert!((hist1.total_kg - h1).abs() < 0.0001);
        assert_eq!(hist1.participations[0].produit, "cacao");
        assert!(hist1.participations[0].reference.starts_with("AGB-"));
        assert!((hist1.participations[0].quantite_kg - h1).abs() < 0.0001);

        let hist2 = db.historique_producteur(&token, capko2).expect("historique 2");
        assert_eq!(hist2.nb_participations, 1);
        assert_eq!(hist2.participations[0].titre, "Second contrat");
        assert!((hist2.total_kg - h2).abs() < 0.0001);

        let absent = db
            .historique_producteur(&token, 999_999)
            .expect_err("exploitation inconnue");
        assert!(absent.contains("introuvable"), "erreur: {absent}");
    }

    #[test]
    fn liste_producteurs_participants() {
        let (db, token) = db_avec_cacao();
        let (h1, h2) = hebdo_cacao();

        let parts = db.producteurs_participants(&token).expect("participants");
        assert!(parts.is_empty(), "aucun participant avant fiche");

        db.generer_fiche_seed(&token, "cacao", h1, None, 1).expect("fiche 1");
        db.generer_fiche_seed(&token, "cacao", h2, Some("Second contrat"), 1).expect("fiche 2");

        let parts = db.producteurs_participants(&token).expect("participants");
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].nb_participations, 1);
        assert!((parts[0].total_kg - h1).abs() < 0.0001);
        assert_eq!(parts[1].nb_participations, 1);
        assert!((parts[1].total_kg - h2).abs() < 0.0001);
    }

    #[test]
    fn reimport_remplace_sans_toucher_fiches() {
        let (db, token) = db_avec_cacao();
        let (h1, _h2) = hebdo_cacao();

        // Fiche générée avant réimport : protège CAPKO_001 et sa culture cacao.
        let fiche = db.generer_fiche_seed(&token, "cacao", h1, None, 1).expect("fiche");
        assert_eq!(fiche.items.len(), 1);

        // Nouveau fichier : CAPKO_001 modifié, CAPKO_002 absent (sans allocation).
        let mut data = ParsedWorkbook::default();
        data.exploitations.push(ExploitationDraft {
            id_interne: Some("CAPKO_001".into()),
            village: Some("NOUVEAU_VILLAGE".into()),
            ..ExploitationDraft::empty()
        });
        data.cultures.push(CultureDraft {
            id_interne: Some("CAPKO_001".into()),
            produit: Some("cacao".into()),
            recolte_estimee: Some(3000.0),
            ..CultureDraft::empty()
        });
        data.unites.push(UniteDraft {
            id_interne: Some("CAPKO_001".into()),
            identifiant_unite: Some("UA-1".into()),
            superficie_ha: Some(3.0),
            latitude: Some(5.0),
            longitude: Some(-6.0),
        });

        let res = db.import_workbook(&token, &data).expect("reimport");
        assert_eq!(res.exploitations_added, 0, "aucun ajout");
        assert_eq!(res.cultures_added, 0, "pas de doublon culture");
        assert_eq!(res.unites_added, 0, "pas de doublon unité");
        assert_eq!(res.rows_updated, 3, "exploitation + culture + unité");
        assert_eq!(res.rows_deleted, 2, "CAPKO_002 et sa culture");

        // Données remplacées, pas dupliquées.
        let exs = db.list_exploitations(&token).expect("liste");
        assert_eq!(exs.len(), 1);
        assert_eq!(exs[0].id_interne.as_deref(), Some("CAPKO_001"));
        assert_eq!(exs[0].village.as_deref(), Some("NOUVEAU_VILLAGE"));

        // Culture mise à jour (3000) : livrable restant = cumul saisonnier − h1 alloué,
        // une seule culture cacao restante (pas de doublon, CAPKO_002 supprimé).
        let (y, m, _d) = today_utc();
        let attendu = weekly_estimate(3000.0, period_of(y, m)) * saison_semaines() - h1;
        let prods = db.producteurs_pour_fiche(&token, "cacao").expect("producteurs");
        assert_eq!(prods.len(), 1);
        assert!((prods[0].quantite_livrable_kg - attendu).abs() < 0.01);
        assert!(prods[0].participe_semaine);

        // Fiche générée intacte.
        let relue = db.get_fiche(&token, fiche.id).expect("relecture");
        assert_eq!(relue.items.len(), 1);
        assert!((relue.items[0].quantite_kg - h1).abs() < 0.0001);
    }

    #[test]
    fn relire_fiche_apres_generation() {
        let (db, token) = db_avec_cacao();
        let (h1, h2) = hebdo_cacao();
        // Demande = tout le livrable saisonnier : les deux producteurs contribuent.
        let total_saison = (h1 + h2) * saison_semaines();

        let fiche = db.generer_fiche_seed(&token, "cacao", total_saison, Some("Export KOKOLO"), 1).expect("fiche");
        assert_eq!(fiche.titre, "Export KOKOLO");
        assert_eq!(fiche.items.len(), 2);

        let list = db.list_fiches(&token).expect("liste");
        assert_eq!(list.len(), 1);
        let summary = &list[0];
        assert_eq!(summary.id, fiche.id);
        assert_eq!(summary.reference, fiche.reference);
        assert_eq!(summary.titre, "Export KOKOLO");
        assert_eq!(summary.statut, "complete");
        assert_eq!(summary.produit, "cacao");
        assert_eq!(summary.nb_producteurs, 2);

        let relue = db.get_fiche(&token, fiche.id).expect("relecture");
        assert_eq!(relue.reference, fiche.reference);
        assert_eq!(relue.titre, "Export KOKOLO", "titre conservé");
        assert_eq!(relue.statut, fiche.statut);
        assert!((relue.quantite_demandee_kg - total_saison).abs() < 0.01);
        assert!((relue.quantite_attribuee_kg - total_saison).abs() < 0.01);
        assert_eq!(relue.items.len(), 2, "producteurs restaurés");
        let sum: f64 = relue.items.iter().map(|i| i.quantite_kg).sum();
        assert!((sum - total_saison).abs() < 0.01, "quantités par producteur cohérentes");
        for item in &relue.items {
            assert!(
                (item.exploitation.id_interne.as_deref() == Some("CAPKO_001") && item.unites.len() == 1)
                    || (item.exploitation.id_interne.as_deref() == Some("CAPKO_002") && item.unites.is_empty()),
                "parcelles cohérentes pour {:?}",
                item.exploitation.id_interne
            );
        }
        assert_eq!(relue.items[0].exploitation.id_interne.as_deref(), Some("CAPKO_001"));

        let absent = db.get_fiche(&token, 9999).expect_err("fiche inconnue");
        assert!(absent.contains("Erreur de lecture"), "erreur: {absent}");
    }

    #[test]
    fn producteur_ineligible_deux_semaines() {
        let (db, token) = db_avec_cacao();
        let (_h1, _h2) = hebdo_cacao();

        let fiche1 = db.generer_fiche_seed(&token, "cacao", 1.0, None, 1).expect("fiche1");
        assert_eq!(fiche1.items[0].exploitation.id_interne.as_deref(), Some("CAPKO_001"));
        assert!((fiche1.quantite_attribuee_kg - 1.0).abs() < 0.0001);

        // CAPKO_001 garde du stock mais vient d'être utilisé : il est exclu 2 semaines.
        // Demande au-delà du livrable saisonnier de CAPKO_002 même avec la marge +5 % :
        // la commande reste honorée intégralement, le surplus allant au seul éligible.
        let h2s = hebdo_cacao().1 * saison_semaines();
        let fiche2 = db.generer_fiche_seed(&token, "cacao", h2s * 1.10 + 10.0, None, 1).expect("fiche2");
        assert_eq!(fiche2.statut, "complete", "commande honorée au-delà du livrable");
        assert_eq!(fiche2.items.len(), 1, "CAPKO_001 exclu par la période de carence");
        assert_eq!(fiche2.items[0].exploitation.id_interne.as_deref(), Some("CAPKO_002"));
        // Le surplus est ajouté à CAPKO_002 : l'intégralité de la demande est couverte.
        assert!(
            (fiche2.quantite_attribuee_kg - (h2s * 1.10 + 10.0)).abs() < 0.0001,
            "attribué {} = demande {}",
            fiche2.quantite_attribuee_kg,
            h2s * 1.10 + 10.0
        );
    }

    #[test]
    fn rapport_et_cloture_semestre() {
        let (db, token) = db_avec_cacao();
        let (h1, h2) = hebdo_cacao();
        let s = saison_semaines();
        let (y, m, _d) = today_utc();
        let (sem, annee) = semestre_pour_date(y, m);

        // Sans allocation : reste = estimation saisonnière, pas de clôture.
        let rap = db.rapport_semestre(&token, sem, annee).expect("rapport");
        assert!(rap.cloture_le.is_none());
        assert_eq!(rap.par_produit.len(), 1);
        assert_eq!(rap.par_producteur.len(), 2);
        assert!((rap.estime_kg - (h1 + h2) * s).abs() < 0.01);
        assert!(rap.alloue_kg.abs() < 0.0001);
        assert!((rap.reste_kg - rap.estime_kg).abs() < 0.01);

        // Après une fiche : l'alloué et le reste suivent.
        db.generer_fiche_seed(&token, "cacao", h1, None, 1).expect("fiche");
        let rap2 = db.rapport_semestre(&token, sem, annee).expect("rapport2");
        assert!((rap2.alloue_kg - h1).abs() < 0.0001);
        assert!((rap2.reste_kg - (rap2.estime_kg - h1)).abs() < 0.01);

        // Clôture : 2 cultures enregistrées, date figée, idempotent.
        let n = db.cloturer_semestre(&token, sem, annee).expect("clôture");
        assert_eq!(n, 2);
        let rap3 = db.rapport_semestre(&token, sem, annee).expect("rapport3");
        assert!(rap3.cloture_le.is_some());
        let n2 = db.cloturer_semestre(&token, sem, annee).expect("reclôture");
        assert_eq!(n2, 2);

        // Paramètres invalides rejetés.
        assert!(db.rapport_semestre(&token, "S3", annee).is_err());
        assert!(db.rapport_semestre(&token, sem, 1999).is_err());
    }

    #[test]
    fn estimation_hebdo_par_periode() {
        assert_eq!(period_of(2026, 9), 1, "septembre → S1");
        assert_eq!(period_of(2026, 2), 1, "février → S1");
        assert_eq!(period_of(2026, 3), 2, "mars → S2");
        assert_eq!(period_of(2026, 8), 2, "août → S2");

        let s1 = weekly_estimate(2000.0, 1);
        let s2 = weekly_estimate(2000.0, 2);
        assert!((s1 - 2000.0 * 0.7 / 24.0).abs() < 0.0001);
        assert!((s2 - 2000.0 * 0.3 / 24.0).abs() < 0.0001);
        assert!(s1 > s2, "S1 plus productive que S2");

        let (y, m, _d) = today_utc();
        let p = period_of(y, m);
        assert!((1..=2).contains(&p));
        let ws = current_week_start();
        let days = ws.len();
        assert_eq!(days, 10, "format YYYY-MM-DD");
        let wd = days_from_civil(y, m, 1);
        assert!(wd > 0, "date cohérente");
    }
}
