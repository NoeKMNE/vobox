mod db;
mod excel;

use tauri::Manager;

use base64::Engine as _;
use db::{
    Db, Exploitation, ExploitationDetail, Fiche, FicheSummary, ImportResult, ProduitStock,
    SearchCategory, Session, User,
};

#[tauri::command]
fn register(state: tauri::State<'_, Db>, name: String, email: String, password: String) -> Result<Session, String> {
    state.register(&name, &email, &password)
}

#[tauri::command]
fn login(state: tauri::State<'_, Db>, email: String, password: String) -> Result<Session, String> {
    state.login(&email, &password)
}

#[tauri::command]
fn logout(state: tauri::State<'_, Db>, token: String) {
    state.logout(&token);
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
    Ok(result)
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
    let path = dir.join(format!("agrobox-backup-{now}.json"));
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
fn generer_fiche(
    state: tauri::State<'_, Db>,
    token: String,
    produit: String,
    quantite_kg: f64,
    titre: Option<String>,
) -> Result<Fiche, String> {
    state.generer_fiche(&token, &produit, quantite_kg, titre.as_deref())
}

#[tauri::command]
fn list_fiches(state: tauri::State<'_, Db>, token: String) -> Result<Vec<FicheSummary>, String> {
    state.list_fiches(&token)
}

#[tauri::command]
fn get_fiche(state: tauri::State<'_, Db>, token: String, id: i64) -> Result<Fiche, String> {
    state.get_fiche(&token, id)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
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
            let db_path = app_data_dir.join("agrobox.db");
            let db = Db::init(&db_path).map_err(|e| format!("Initialisation de la base : {e}"))?;
            app.manage(db);

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
            list_exploitations,
            get_exploitation,
            search_exploitations,
            export_backup,
            reset_data,
            produits_disponibles,
            generer_fiche,
            list_fiches,
            get_fiche
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}