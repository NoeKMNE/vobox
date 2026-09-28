use std::path::Path;
use std::sync::Mutex;

use argon2::password_hash::rand_core::{OsRng, RngCore};
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

use crate::excel::{ParsedWorkbook, SheetDump};

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
    pub quantite_disponible_kg: f64,
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
             DROP TABLE IF EXISTS fts_search;
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
        let mut loaded_ids = Vec::new();

        for d in &data.exploitations {
            let Some(id_interne) = d.id_interne.clone() else {
                continue;
            };
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
            loaded_ids.push(id_interne);
        }

        for c in &data.cultures {
            let Some(id_interne) = c.id_interne.clone() else {
                continue;
            };
            let explosion_id = self.exploitation_id(&conn, user_id, &id_interne);
            let Some(exploitation_id) = explosion_id else {
                continue;
            };
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
            cultures_added += inserted as usize;
        }

        for u in &data.unites {
            let Some(id_interne) = u.id_interne.clone() else {
                continue;
            };
            let Some(exploitation_id) = self.exploitation_id(&conn, user_id, &id_interne) else {
                continue;
            };
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
            unites_added += inserted as usize;
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
        let conn = self.0.lock().map_err(|_| "Base de données verrouillée".to_string())?;
        let user_id = self.require_user(&conn, token)?;

        let mut stmt = conn
            .prepare(
                "SELECT c.produit,
                        COALESCE(SUM(c.recolte_estimee), 0) AS est,
                        COUNT(DISTINCT e.id) AS nb
                 FROM cultures_certifiees c
                 JOIN exploitations e ON e.id = c.exploitation_id AND e.user_id = ?1
                 WHERE c.produit IS NOT NULL AND trim(c.produit) <> ''
                 GROUP BY c.produit",
            )
            .map_err(|e| format!("Erreur de lecture des stocks : {e}"))?;

        let estimees: Vec<(String, f64, u64)> = stmt
            .query_map(params![user_id], |r| {
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
             GROUP BY c.produit",
        ) {
            alloues = stmt2
                .query_map(params![user_id], |r| Ok((r.get(0)?, r.get(1)?)))
                .map_err(|e| format!("Erreur de lecture des stocks : {e}"))?
                .collect::<Result<_, _>>()
                .map_err(|e| format!("Erreur de lecture des stocks : {e}"))?;
        }

        let alloc_map: std::collections::HashMap<&str, f64> =
            alloues.iter().map(|(p, a)| (p.as_str(), *a)).collect();

        let mut out: Vec<ProduitStock> = estimees
            .into_iter()
            .map(|(produit, est, nb)| {
                let al = alloc_map.get(produit.as_str()).copied().unwrap_or(0.0);
                ProduitStock {
                    produit,
                    quantite_estimee_kg: est,
                    quantite_allouee_kg: al,
                    quantite_disponible_kg: est - al,
                    nb_exploitations: nb,
                }
            })
            .collect();
        out.sort_by(|a, b| b.quantite_disponible_kg.partial_cmp(&a.quantite_disponible_kg).unwrap_or(std::cmp::Ordering::Equal));
        Ok(out)
    }

    pub fn generer_fiche(
        &self,
        token: &str,
        produit: &str,
        quantite_kg: f64,
        titre: Option<&str>,
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

        let mut conn = self.0.lock().map_err(|_| "Base de données verrouillée".to_string())?;
        let user_id = self.require_user(&conn, token)?;
        let tx = conn
            .transaction()
            .map_err(|e| format!("Erreur de transaction : {e}"))?;

        let rows: Vec<(i64, i64, f64)> = {
            let mut stmt = tx
                .prepare(
                    "SELECT c.id, c.exploitation_id,
                            c.recolte_estimee - COALESCE(
                                (SELECT SUM(a.quantite_kg) FROM allocations a WHERE a.culture_id = c.id), 0
                            ) AS dispo
                     FROM cultures_certifiees c
                     JOIN exploitations e ON e.id = c.exploitation_id AND e.user_id = ?1
                     WHERE trim(lower(c.produit)) = trim(lower(?2))
                       AND c.recolte_estimee > 0
                       AND dispo > 0.000001
                     ORDER BY dispo DESC, c.id ASC",
                )
                .map_err(|e| format!("Erreur de préparation : {e}"))?;

            let fetched = stmt
                .query_map(params![user_id, produit], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
                .map_err(|e| format!("Erreur de lecture des candidats : {e}"))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("Erreur de lecture des candidats : {e}"))?;
            fetched
        };

        let mut remaining = quantite_kg;
        let mut allocated_total = 0.0;
        let mut allocations: Vec<(i64, i64, f64)> = Vec::new();
        for (culture_id, exploitation_id, dispo) in rows {
            if remaining <= 0.0 {
                break;
            }
            let take = dispo.min(remaining);
            remaining -= take;
            allocated_total += take;
            allocations.push((culture_id, exploitation_id, take));
        }

        if allocations.is_empty() {
            return Err(format!("Stock insuffisant pour « {produit} »"));
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
            ..UniteDraft::empty()
        });

        db.import_workbook(&token, &data).expect("import");
        (db, token)
    }

    #[test]
    fn generer_fiche_selectionne_et_verrouille() {
        let (db, token) = db_avec_cacao();

        let stocks = db.produits_disponibles(&token).expect("stocks");
        assert_eq!(stocks.len(), 1);
        assert_eq!(stocks[0].produit, "cacao");
        assert_eq!(stocks[0].quantite_estimee_kg, 2800.0);
        assert_eq!(stocks[0].quantite_disponible_kg, 2800.0);
        assert_eq!(stocks[0].nb_exploitations, 2);

        let fiche = db.generer_fiche(&token, "cacao", 1500.0, None).expect("fiche");
        assert_eq!(fiche.statut, "complete");
        assert_eq!(fiche.titre, "Fiche de traçabilité produit", "titre par défaut");
        assert_eq!(fiche.quantite_attribuee_kg, 1500.0);
        assert_eq!(fiche.items.len(), 1, "un seul producteur couvre la demande");
        assert_eq!(fiche.items[0].exploitation.id_interne.as_deref(), Some("CAPKO_001"));
        assert_eq!(fiche.items[0].quantite_kg, 1500.0);
        assert!(fiche.reference.starts_with("AGB-"), "référence AGB");
        assert_eq!(fiche.items[0].unites.len(), 1, "parcelles incluses dans la fiche");

        let fiche2 = db.generer_fiche(&token, "cacao", 1300.0, Some("Client EXIM SA")).expect("fiche2");
        assert_eq!(fiche2.statut, "complete");
        assert_eq!(fiche2.titre, "Client EXIM SA", "titre personnalisé");
        assert_eq!(fiche2.quantite_attribuee_kg, 1300.0);
        assert_eq!(fiche2.items.len(), 2);
        assert_ne!(fiche.reference, fiche2.reference, "références uniques");

        let err = db.generer_fiche(&token, "cacao", 1.0, None).expect_err("stock épuisé");
        assert!(err.contains("Stock insuffisant"), "erreur: {err}");

        let stocks2 = db.produits_disponibles(&token).expect("stocks");
        assert_eq!(stocks2[0].quantite_disponible_kg, 0.0, "production verrouillée après génération");

        let fiches = db.list_fiches(&token).expect("liste");
        assert_eq!(fiches.len(), 2);
        assert_eq!(fiches[0].nb_producteurs, 2);
    }

    #[test]
    fn generer_fiche_partielle_si_stock_insuffisant() {
        let (db, token) = db_avec_cacao();

        let fiche = db.generer_fiche(&token, "cacao", 3000.0, None).expect("fiche partielle");
        assert_eq!(fiche.statut, "partielle");
        assert_eq!(fiche.quantite_attribuee_kg, 2800.0, "toute l'estimation est affectée");
        assert_eq!(fiche.items.len(), 2);

        let err = db.generer_fiche(&token, "cacao", 1.0, None).expect_err("épuisé");
        assert!(err.contains("Stock insuffisant"));

        let err2 = db.generer_fiche(&token, "cafe", 1.0, None).expect_err("produit inconnu");
        assert!(err2.contains("Stock insuffisant"), "erreur: {err2}");
    }

    #[test]
    fn relire_fiche_apres_generation() {
        let (db, token) = db_avec_cacao();

        let fiche = db.generer_fiche(&token, "cacao", 2200.0, Some("Export KOKOLO")).expect("fiche");
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
        assert_eq!(relue.quantite_demandee_kg, 2200.0);
        assert_eq!(relue.quantite_attribuee_kg, 2200.0);
        assert_eq!(relue.items.len(), 2, "producteurs restaurés");
        let total: f64 = relue.items.iter().map(|i| i.quantite_kg).sum();
        assert_eq!(total, 2200.0, "quantités par producteur cohérentes");
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
}