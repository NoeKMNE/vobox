mod agent;
mod db;
mod excel;

use tauri::Emitter;
use tauri::Manager;
use tauri_plugin_updater::UpdaterExt;
use url::Url;

use base64::Engine as _;
use db::{
    Db, Exploitation, ExploitationDetail, Fiche, FicheSummary, GlobalStats, HistoriqueProducteur,
    ImportResult, ProduitStock, ProducteurFiche, RapportSemestre, ResumeParticipation, SearchCategory, Session, User,
};

pub(crate) const AI_API_BASE: &str = "https://logixio-code.click/api/v1";

#[derive(serde::Deserialize)]
struct AiAuthResponse {
    token: String,
    refresh_token: String,
}

struct AiAuthError {
    status: reqwest::StatusCode,
    message: String,
}

async fn request_ai_auth(
    path: &str,
    payload: serde_json::Value,
) -> Result<AiAuthResponse, AiAuthError> {
    let endpoint = format!("{AI_API_BASE}/{path}");
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(12))
        .build()
        .map_err(|_| AiAuthError {
            status: reqwest::StatusCode::INTERNAL_SERVER_ERROR,
            message: "Service IA indisponible.".into(),
        })?;
    let response = client
        .post(endpoint)
        .json(&payload)
        .send()
        .await
        .map_err(|_| AiAuthError {
            status: reqwest::StatusCode::SERVICE_UNAVAILABLE,
            message: "Impossible de joindre le service IA.".into(),
        })?;
    let status = response.status();
    if status.is_success() {
        return response.json().await.map_err(|_| AiAuthError {
            status: reqwest::StatusCode::BAD_GATEWAY,
            message: "Réponse invalide du service IA.".into(),
        });
    }

    let body = response.json::<serde_json::Value>().await.unwrap_or_default();
    let message = body
        .get("error")
        .and_then(|error| error.get("message"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("Service IA temporairement indisponible.")
        .to_string();
    Err(AiAuthError { status, message })
}

async fn save_remote_ai_session(
    db: &Db,
    user_id: i64,
    email: &str,
    password: &str,
    name: &str,
    register: bool,
) -> Result<(), String> {
    let app_version = env!("CARGO_PKG_VERSION");
    let (path, payload) = if register {
        (
            "auth/register",
            serde_json::json!({
                "name": name,
                "email": email,
                "password": password,
                "app": "vobox-desktop",
                "app_version": app_version,
                "device_id": ""
            }),
        )
    } else {
        (
            "auth/chat",
            serde_json::json!({
                "email": email,
                "password": password,
                "app": "vobox-desktop",
                "app_version": app_version,
                "device_id": ""
            }),
        )
    };

    let credentials = match request_ai_auth(path, payload).await {
        Ok(credentials) => credentials,
        Err(error) if !register && error.status == reqwest::StatusCode::UNAUTHORIZED => {
            request_ai_auth(
                "auth/register",
                serde_json::json!({
                    "name": name,
                    "email": email,
                    "password": password,
                    "app": "vobox-desktop",
                    "app_version": app_version,
                    "device_id": ""
                }),
            )
            .await
            .map_err(|error| error.message)?
        }
        Err(error) => return Err(error.message),
    };
    db.set_ai_session(user_id, &credentials.token, &credentials.refresh_token)
}

#[tauri::command]
async fn register(
    app: tauri::AppHandle,
    state: tauri::State<'_, Db>,
    name: String,
    email: String,
    password: String,
    telemetry_consent: bool,
) -> Result<Session, String> {
    let session = state.register(&name, &email, &password)?;
    // Le consentement ne doit jamais faire échouer l'inscription.
    let _ = state.set_telemetry_consent(telemetry_consent);
    if save_remote_ai_session(
        &state,
        session.user.id,
        &session.user.email,
        &password,
        &name,
        true,
    )
    .await
    .is_err()
    {
        log::warn!("Initialisation de l'assistant après inscription impossible.");
    }
    track_and_flush(
        &app,
        "register",
        serde_json::json!({ "telemetry_consent": telemetry_consent }),
    );
    Ok(session)
}

/// Endpoint de collecte télémétrie : réglage `telemetry_endpoint` (modifiable),
/// sinon `VOBOX_TELEMETRY_URL` (ou `AGROBOX_TELEMETRY_URL` pour compatibilité).
/// Vide / absent = conservation locale uniquement, aucun envoi réseau.
fn telemetry_endpoint(db: &Db) -> Option<String> {
    if let Ok(Some(url)) = db.get_setting("telemetry_endpoint") {
        let url = url.trim().trim_end_matches('/').to_string();
        if !url.is_empty() {
            return Some(url);
        }
    }
    std::env::var("VOBOX_TELEMETRY_URL")
        .or_else(|_| std::env::var("AGROBOX_TELEMETRY_URL"))
        .ok()
        .filter(|u| !u.trim().is_empty())
        .map(|u| u.trim().trim_end_matches('/').to_string())
}

/// Enregistre un événement d'usage puis tente l'envoi du tampon en tâche de fond.
/// Toujours best-effort : sans consentement rien n'est enregistré, et un échec
/// réseau ne remonte jamais d'erreur (les événements restent en local).
fn track_and_flush(app: &tauri::AppHandle, event: &str, props: serde_json::Value) {
    {
        let db = app.state::<Db>();
        db.track_event(event, &props);
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        flush_telemetry(&app).await;
    });
}

async fn flush_telemetry(app: &tauri::AppHandle) {
    let (endpoint, pending) = {
        let db = app.state::<Db>();
        if !db.telemetry_consent() {
            return;
        }
        let Some(endpoint) = telemetry_endpoint(&db) else {
            return;
        };
        match db.take_pending_telemetry(100) {
            Ok(p) if !p.is_empty() => (endpoint, p),
            _ => return,
        }
    };
    let ids: Vec<i64> = pending.iter().map(|(id, ..)| *id).collect();
    let payload: Vec<serde_json::Value> = pending
        .iter()
        .map(|(_, event, props, created_at)| {
            let props: serde_json::Value =
                serde_json::from_str(props).unwrap_or(serde_json::Value::Null);
            serde_json::json!({ "event": event, "props": props, "created_at": created_at })
        })
        .collect();
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            log::warn!("Télémétrie : client HTTP impossible ({e})");
            return;
        }
    };
    match client.post(&endpoint).json(&payload).send().await {
        Ok(resp) if resp.status().is_success() => {
            let db = app.state::<Db>();
            if let Err(e) = db.mark_telemetry_synced(&ids) {
                log::warn!("Télémétrie : marquage d'envoi impossible ({e})");
            }
        }
        Ok(resp) => log::warn!("Télémétrie : rejet {} par {endpoint}", resp.status()),
        Err(e) => log::warn!("Télémétrie : envoi vers {endpoint} impossible ({e})"),
    }
}

#[tauri::command]
async fn login(
    app: tauri::AppHandle,
    state: tauri::State<'_, Db>,
    email: String,
    password: String,
) -> Result<Session, String> {
    let session = state.login(&email, &password)?;
    if save_remote_ai_session(
        &state,
        session.user.id,
        &session.user.email,
        &password,
        &session.user.name,
        false,
    )
    .await
    .is_err()
    {
        log::warn!("Initialisation de l'assistant après connexion impossible.");
    }
    track_and_flush(&app, "login", serde_json::json!({}));
    Ok(session)
}

#[tauri::command]
fn get_telemetry_consent(state: tauri::State<'_, Db>) -> bool {
    state.telemetry_consent()
}

/// Producteurs sans participation passée éligibles aux fiches ?
/// Réglage global (Paramètres → Fiches), défaut : inclus.
#[tauri::command]
fn get_inclure_nouveaux(state: tauri::State<'_, Db>) -> bool {
    state.inclure_nouveaux()
}

#[tauri::command]
fn set_inclure_nouveaux(state: tauri::State<'_, Db>, enabled: bool) -> Result<(), String> {
    state.set_inclure_nouveaux(enabled)
}

/// Rapport des restes d'un semestre (estimation − alloué), par produit et producteur.
#[tauri::command]
fn rapport_semestre(
    state: tauri::State<'_, Db>,
    token: String,
    semestre: String,
    annee: i64,
) -> Result<RapportSemestre, String> {
    state.rapport_semestre(&token, &semestre, annee)
}

/// Enregistre la clôture d'un semestre (idempotent : réinscriptible).
#[tauri::command]
fn cloturer_semestre(
    state: tauri::State<'_, Db>,
    token: String,
    semestre: String,
    annee: i64,
) -> Result<usize, String> {
    state.cloturer_semestre(&token, &semestre, annee)
}
#[tauri::command]
fn set_telemetry_consent(
    app: tauri::AppHandle,
    state: tauri::State<'_, Db>,
    enabled: bool,
) -> Result<(), String> {
    state.set_telemetry_consent(enabled)?;
    if enabled {
        track_and_flush(&app, "telemetry_opt_in", serde_json::json!({}));
    }
    Ok(())
}

#[tauri::command]
async fn logout(state: tauri::State<'_, Db>, token: String) -> Result<(), String> {
    let ai_token = state
        .validate(&token)
        .ok()
        .and_then(|user| state.get_ai_session(user.id).ok().flatten())
        .map(|session| session.access_token);
    state.logout(&token);
    if let Some(ai_token) = ai_token {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .map_err(|error| format!("Client de révocation IA indisponible : {error}"))?;
        match client
            .post(format!("{AI_API_BASE}/auth/chat/logout"))
            .bearer_auth(ai_token)
            .send()
            .await
        {
            Ok(response) if response.status().is_success() => {}
            Ok(response) => {
                log::warn!("Révocation de la session IA refusée: {}", response.status());
            }
            Err(error) => log::warn!("Révocation de la session IA impossible: {error}"),
        }
    }
    Ok(())
}

#[tauri::command]
fn get_session(state: tauri::State<'_, Db>, token: String) -> Result<User, String> {
    state.validate(&token)
}

#[tauri::command]
fn complete_onboarding(state: tauri::State<'_, Db>, token: String) -> Result<(), String> {
    state.complete_onboarding(&token)
}

#[tauri::command]
fn update_profile(
    state: tauri::State<'_, Db>,
    token: String,
    name: String,
    email: String,
) -> Result<User, String> {
    state.update_profile(&token, &name, &email)
}

#[tauri::command]
fn update_avatar(
    state: tauri::State<'_, Db>,
    token: String,
    avatar: Option<String>,
) -> Result<User, String> {
    state.update_avatar(&token, avatar)
}

#[tauri::command]
fn import_excel(
    app: tauri::AppHandle,
    state: tauri::State<'_, Db>,
    token: String,
    file_b64: String,
    _file_name: String,
) -> Result<ImportResult, String> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&file_b64)
        .map_err(|e| format!("Fichier illisible : {e}"))?;
    let parsed = excel::parse_excel(&bytes)?;
    let mut result = state.import_workbook(&token, &parsed)?;
    result.sheets = excel::inspect_excel(&bytes).unwrap_or_default();
    // Copie de travail éditée automatiquement chaque semaine (estimations).
    if let Ok(dir) = app.path().app_data_dir() {
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(dir.join("vobox-working-copy.xlsx"), &bytes);
    }
    track_and_flush(
        &app,
        "import_excel",
        serde_json::json!({
            "exploitations_added": result.exploitations_added,
            "cultures_added": result.cultures_added,
            "unites_added": result.unites_added,
        }),
    );
    Ok(result)
}

#[tauri::command]
async fn analyser_fiche_import(
    state: tauri::State<'_, Db>,
    token: String,
    file_b64: String,
    file_name: String,
    mime_type: String,
) -> Result<agent::FicheAnalysis, String> {
    if file_b64.len() > 16 * 1024 * 1024 {
        return Err("Le fichier ne doit pas dépasser 12 Mo.".into());
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&file_b64)
        .map_err(|e| format!("Fichier illisible : {e}"))?;
    if bytes.is_empty() || bytes.len() > 12 * 1024 * 1024 {
        return Err("Le fichier doit peser entre 1 octet et 12 Mo.".into());
    }
    let extension = std::path::Path::new(&file_name)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let (excel_text, image_data_url) = match extension.as_str() {
        "xlsx" | "xls" => (Some(excel::traceability_text(&bytes)?), None),
        "jpg" | "jpeg" | "png" | "webp" => {
            let expected_mime = match extension.as_str() {
                "jpg" | "jpeg" => "image/jpeg",
                "png" => "image/png",
                _ => "image/webp",
            };
            if mime_type != expected_mime {
                return Err("Le format de l’image n’est pas reconnu.".into());
            }
            (None, Some(format!("data:{expected_mime};base64,{file_b64}")))
        }
        _ => return Err("Formats acceptés : XLSX, XLS, JPG, PNG et WebP.".into()),
    };
    agent::analyser_fiche(&state, &token, &file_name, excel_text, image_data_url).await
}

#[tauri::command]
fn importer_fiche_historique(
    app: tauri::AppHandle,
    state: tauri::State<'_, Db>,
    token: String,
    fiche: db::FicheImportRequest,
) -> Result<Fiche, String> {
    let imported = state.importer_fiche_historique(&token, fiche)?;
    track_and_flush(
        &app,
        "importer_fiche_historique",
        serde_json::json!({
            "nb_producteurs": imported.items.len(),
            "quantite_kg": imported.quantite_attribuee_kg,
        }),
    );
    Ok(imported)
}

#[derive(serde::Serialize)]
struct WorkingExcel {
    file_b64: String,
    sheets: Vec<excel::SheetDump>,
}

#[tauri::command]
fn working_excel_exists(
    app: tauri::AppHandle,
    state: tauri::State<'_, Db>,
    token: String,
) -> Result<bool, String> {
    state.validate(&token)?;
    let path = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Dossier des données inaccessible : {e}"))?
        .join("vobox-working-copy.xlsx");
    Ok(path.is_file())
}

#[tauri::command]
fn get_working_excel(
    app: tauri::AppHandle,
    state: tauri::State<'_, Db>,
    token: String,
) -> Result<WorkingExcel, String> {
    state.validate(&token)?;
    let path = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Dossier des données inaccessible : {e}"))?
        .join("vobox-working-copy.xlsx");
    let bytes = std::fs::read(path)
        .map_err(|e| format!("Fichier Excel de travail introuvable ou illisible : {e}"))?;
    let sheets = excel::inspect_excel(&bytes)?;
    Ok(WorkingExcel {
        file_b64: base64::engine::general_purpose::STANDARD.encode(bytes),
        sheets,
    })
}

#[tauri::command]
fn list_exploitations(state: tauri::State<'_, Db>, token: String) -> Result<Vec<Exploitation>, String> {
    state.list_exploitations(&token)
}

#[tauri::command]
fn get_exploitation(
    state: tauri::State<'_, Db>,
    token: String,
    id: i64,
) -> Result<ExploitationDetail, String> {
    state.get_exploitation(&token, id)
}

#[tauri::command]
fn search_exploitations(
    state: tauri::State<'_, Db>,
    token: String,
    query: String,
    category: SearchCategory,
) -> Result<Vec<Exploitation>, String> {
    state.search_exploitations(&token, &query, category)
}

#[tauri::command]
fn export_backup(
    app: tauri::AppHandle,
    state: tauri::State<'_, Db>,
    token: String,
) -> Result<String, String> {
    let data = state.export_data(&token)?;
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("backups");
    std::fs::create_dir_all(&dir).map_err(|e| format!("Dossier de sauvegarde impossible : {e}"))?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let json = serde_json::to_string_pretty(&data)
        .map_err(|e| format!("Sérialisation de la sauvegarde : {e}"))?;
    let path = dir.join(format!("vobox-backup-{now}.json"));
    std::fs::write(&path, json).map_err(|e| format!("Écriture de la sauvegarde : {e}"))?;
    Ok(path.to_string_lossy().into_owned())
}

#[tauri::command]
fn reset_data(state: tauri::State<'_, Db>, token: String) -> Result<usize, String> {
    state.reset_data(&token)
}

#[tauri::command]
fn produits_disponibles(state: tauri::State<'_, Db>, token: String) -> Result<Vec<ProduitStock>, String> {
    state.produits_disponibles(&token)
}

#[tauri::command]
fn producteurs_pour_fiche(
    state: tauri::State<'_, Db>,
    token: String,
    produit: String,
) -> Result<Vec<ProducteurFiche>, String> {
    state.producteurs_pour_fiche(&token, &produit)
}

#[tauri::command]
fn generer_fiche_manuelle(
    app: tauri::AppHandle,
    state: tauri::State<'_, Db>,
    token: String,
    produit: String,
    titre: Option<String>,
    allocations: Vec<(i64, f64)>,
) -> Result<Fiche, String> {
    let fiche = state.generer_fiche_manuelle(&token, &produit, titre.as_deref(), allocations)?;
    track_and_flush(
        &app,
        "generer_fiche",
        serde_json::json!({ "mode": "manuel", "statut": fiche.statut }),
    );
    Ok(fiche)
}

#[tauri::command]
fn historique_producteur(
    state: tauri::State<'_, Db>,
    token: String,
    exploitation_id: i64,
) -> Result<HistoriqueProducteur, String> {
    state.historique_producteur(&token, exploitation_id)
}

#[tauri::command]
fn producteurs_participants(
    state: tauri::State<'_, Db>,
    token: String,
) -> Result<Vec<ResumeParticipation>, String> {
    state.producteurs_participants(&token)
}

#[tauri::command]
fn generer_fiche(
    app: tauri::AppHandle,
    state: tauri::State<'_, Db>,
    token: String,
    produit: String,
    quantite_kg: f64,
    titre: Option<String>,
) -> Result<Fiche, String> {
    let fiche = state.generer_fiche(&token, &produit, quantite_kg, titre.as_deref())?;
    track_and_flush(
        &app,
        "generer_fiche",
        serde_json::json!({ "mode": "auto", "statut": fiche.statut }),
    );
    Ok(fiche)
}

#[tauri::command]
fn list_fiches(state: tauri::State<'_, Db>, token: String) -> Result<Vec<FicheSummary>, String> {
    state.list_fiches(&token)
}

#[tauri::command]
fn get_fiche(state: tauri::State<'_, Db>, token: String, id: i64) -> Result<Fiche, String> {
    state.get_fiche(&token, id)
}

#[tauri::command]
fn supprimer_fiche(state: tauri::State<'_, Db>, token: String, id: i64) -> Result<(), String> {
    state.supprimer_fiche(&token, id)
}

#[tauri::command]
fn set_fiche_nb_sacs(
    state: tauri::State<'_, Db>,
    token: String,
    id: i64,
    nb_sacs: i64,
) -> Result<Fiche, String> {
    state.set_fiche_nb_sacs(&token, id, nb_sacs)
}

#[tauri::command]
fn stats_globales(state: tauri::State<'_, Db>, token: String) -> Result<GlobalStats, String> {
    state.stats_globales(&token)
}

#[tauri::command]
fn get_ai_settings(state: tauri::State<'_, Db>, token: String) -> Result<serde_json::Value, String> {
    let user = state.validate(&token)?;
    let stored_model = state
        .get_setting("ai_model")?
        .filter(|model| !model.trim().is_empty())
        .unwrap_or_else(|| "deepseek-chat".to_string());
    let model = if ["deepseek-chat", "deepseek-reasoner"].contains(&stored_model.as_str()) {
        stored_model
    } else {
        "deepseek-chat".to_string()
    };
    Ok(serde_json::json!({
        "ready": state.get_ai_session(user.id)?.is_some(),
        "model": model,
    }))
}

#[tauri::command]
fn save_ai_settings(
    state: tauri::State<'_, Db>,
    token: String,
    model: Option<String>,
) -> Result<(), String> {
    state.validate(&token)?;
    if let Some(m) = model {
        let model = m.trim();
        if !["deepseek-chat", "deepseek-reasoner"].contains(&model) {
            return Err("Modèle invalide".into());
        }
        state.set_setting("ai_model", model)?;
    }
    Ok(())
}

#[tauri::command]
async fn assistant_chat(
    state: tauri::State<'_, Db>,
    token: String,
    messages: Vec<agent::ChatMsg>,
) -> Result<agent::AgentResponse, String> {
    agent::chat(&state, &token, messages).await
}

/// Manifeste public des versions Vobox publiées sur GitHub Releases.

fn update_endpoint() -> Option<Url> {
    Url::parse("https://github.com/NoeKMNE/vobox/releases/latest/download/stable.json").ok()
}

fn update_token() -> Option<String> {
    // Les releases Vobox sont publiques et ne nécessitent pas de jeton.
    None
}

#[derive(serde::Serialize, Clone)]
struct UpdateInfo {
    available: bool,
    current_version: String,
    version: Option<String>,
    notes: Option<String>,
    pub_date: Option<String>,
}

async fn find_update(app: &tauri::AppHandle) -> Result<Option<tauri_plugin_updater::Update>, String> {
    // Le manifeste et les archives sont publiés sur le dépôt public Vobox.
    let Some(endpoint) = update_endpoint() else {
        return Ok(None);
    };

    let builder = app
        .updater_builder()
        .endpoints(vec![endpoint])
        .map_err(|e| e.to_string())?;
    let builder = match update_token() {
        Some(token) => builder
            .header("Authorization", format!("Bearer {token}"))
            .map_err(|e| e.to_string())?,
        None => builder,
    };
    let updater = builder.build().map_err(|e| e.to_string())?;
    updater.check().await.map_err(|e| e.to_string())
}

/// Vérifie si une mise à jour est disponible (sans l'installer).
/// Appelé au démarrage et depuis Paramètres : affiche un popup si `available`.
#[tauri::command]
async fn check_update(app: tauri::AppHandle) -> Result<UpdateInfo, String> {
    let current_version = app.package_info().version.to_string();
    let none = || UpdateInfo {
        available: false,
        current_version: current_version.clone(),
        version: None,
        notes: None,
        pub_date: None,
    };
    match find_update(&app).await? {
        Some(update) => Ok(UpdateInfo {
            available: true,
            current_version,
            version: Some(update.version.clone()),
            notes: update.body.clone(),
            pub_date: Some(update.date.map(|d| d.date().to_string()).unwrap_or_default()),
        }),
        None => Ok(none()),
    }
}

/// Télécharge et installe la mise à jour (action explicite de l'utilisateur).
/// Émet `Vobox-update-progress` ({downloaded, total}) puis `Vobox-update-installed`.
/// Sur Windows, l'installeur se lance et ferme l'app (redémarrage auto) ;
/// sur macOS/Linux, l'app doit être relancée (bouton « Redémarrer »).
#[tauri::command]
async fn install_update(app: tauri::AppHandle) -> Result<(), String> {
    let update = find_update(&app)
        .await?
        .ok_or("Aucune mise à jour disponible")?;
    let progress_app = app.clone();
    let mut downloaded: u64 = 0;
    update
        .download_and_install(
            move |chunk_len, total| {
                downloaded += chunk_len as u64;
                let _ = progress_app.emit(
                    "Vobox-update-progress",
                    serde_json::json!({ "downloaded": downloaded, "total": total }),
                );
            },
            || {},
        )
        .await
        .map_err(|e| e.to_string())?;
    let _ = app.emit("Vobox-update-installed", ());
    Ok(())
}

#[tauri::command]
fn restart_app(app: tauri::AppHandle) {
    app.restart();
}

#[tauri::command]
async fn auto_update(app: tauri::AppHandle) -> Result<(), String> {
    // Conservé pour compatibilité : vérifie et installe silencieusement
    // (utilisé uniquement si le frontend ne gère pas le popup).
    if find_update(&app).await?.is_none() {
        return Ok(());
    }
    install_update(app).await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            let app_data_dir = app.handle().path().app_data_dir()?;
            std::fs::create_dir_all(&app_data_dir)?;
            let db_path = app_data_dir.join("vobox.db");
            let db = Db::init(&db_path).map_err(|e| format!("Initialisation de la base : {e}"))?;
            db.remove_setting("ai_api_key")?;
            db.remove_setting("ai_base_url")?;

            // Vobox : ajustement des estimations en début de semaine, appliqué
            // aussi à la copie de travail du classeur importé.
            if let Ok(token) = db.first_token() {
                if let Ok(facteurs) = db.ajuster_estimations_hebdomadaires(&token) {
                    let working = app_data_dir.join("vobox-working-copy.xlsx");
                    if let Ok(bytes) = std::fs::read(&working) {
                        if let Ok(new_bytes) = excel::maj_estimations_xlsx(&bytes, &facteurs) {
                            let _ = std::fs::write(&working, new_bytes);
                        }
                    }
                }
            }

            app.manage(db);

            // Télémétrie : événement de démarrage (seulement si opt-in actif).
            track_and_flush(app.handle(), "app_start", serde_json::json!({}));

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            register,
            login,
            logout,
            get_session,
            complete_onboarding,
            update_profile,
            update_avatar,
            import_excel,
            analyser_fiche_import,
            importer_fiche_historique,
            working_excel_exists,
            get_working_excel,
            list_exploitations,
            get_exploitation,
            search_exploitations,
            export_backup,
            reset_data,
            produits_disponibles,
            producteurs_pour_fiche,
            generer_fiche,
            generer_fiche_manuelle,
            historique_producteur,
            producteurs_participants,
            list_fiches,
            get_fiche,
            supprimer_fiche,
            set_fiche_nb_sacs,
            stats_globales,
            get_ai_settings,
            save_ai_settings,
            assistant_chat,
            auto_update,
            check_update,
            install_update,
            restart_app,
            get_telemetry_consent,
            set_telemetry_consent,
            get_inclure_nouveaux,
            set_inclure_nouveaux,
            rapport_semestre,
            cloturer_semestre
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}