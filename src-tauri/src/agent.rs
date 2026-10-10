use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::db::{AiSession, Db, SearchCategory};
use crate::AI_API_BASE;

const DEFAULT_MODEL: &str = "deepseek-chat";
const MAX_STEPS: usize = 6;

const SETTING_MODEL: &str = "ai_model";

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChatMsg {
    pub role: String,
    pub content: String,
}

#[derive(Serialize, Clone, Debug)]
pub struct ProposedAction {
    pub tool: String,
    pub args: serde_json::Value,
    pub label: String,
}

#[derive(Serialize, Clone, Debug)]
pub struct AgentResponse {
    pub reply: String,
    pub actions: Vec<ProposedAction>,
    pub used_tools: Vec<String>,
    pub model: String,
}

#[derive(Serialize)]
struct ApiRequest {
    model: String,
    messages: Vec<serde_json::Value>,
    tools: Vec<ToolSpec>,
    temperature: f64,
    max_tokens: u32,
}

#[derive(Serialize)]
struct ToolSpec {
    r#type: String,
    function: ToolFunction,
}

#[derive(Serialize)]
struct ToolFunction {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

#[derive(Deserialize)]
struct ApiResponse {
    choices: Vec<ApiChoice>,
}

#[derive(Deserialize)]
struct RefreshedAiSession {
    token: String,
    refresh_token: String,
}

#[derive(Deserialize)]
struct ApiChoice {
    message: ApiMessage,
}

#[derive(Deserialize)]
struct ApiMessage {
    content: Option<String>,
    tool_calls: Option<Vec<ApiToolCall>>,
}

#[derive(Deserialize, Clone, Serialize)]
struct ApiToolCall {
    id: String,
    function: ApiFunction,
}

#[derive(Deserialize, Clone, Serialize)]
struct ApiFunction {
    name: String,
    arguments: String,
}

fn system_prompt() -> serde_json::Value {
    let today = chrono_ish_today();
    json!({
        "role": "system",
        "content": format!(
            "Tu es l'assistant agro-agentic de Vobox, un outil de gestion agricole et de traçabilité (exportateurs de produits certifiés).
Date du jour : {today}.
Tu t'exprimes en français, de façon concise, précise et structurée (courtes sections, listes à puces si utile, nombres arrondis avec une virgule et l'unité : kg, ha).

Tu disposes d'outils en lecture seule exécutés automatiquement côté serveur :
- stats_globales : chiffres clés de la base (exploitations, cultures, superficie, fiches, allocations).
- rechercher_producteurs (query) : recherche de producteurs/exploitations par opérateur, localisation, culture, unité, identifiant ou inspecteur.
- lister_fiches : liste des fiches de traçabilité.
- stocks_produits : disponibilité des produits (estimée, allouée, disponible).
- producteurs_pour_fiche (produit) : producteurs éligibles et quantités hebdomadaires pour un produit.

Règles :
- Utilise les outils dès que la réponse repose sur des données réelles. Ne jamais inventer de chiffres : si une donnée manque, dis-le et suggère de l'importer.
- N'appelle un outil qu'avec des arguments corrects et complets (extrais le produit/la requête du message).
- Pour créer une fiche de traçabilité, tu DISPOSES des outils generer_fiche et generer_fiche_manuelle : ils ne sont jamais exécutés automatiquement, ils sont proposés à l'utilisateur qui doit confirmer. Quand tu veux créer une fiche, appelle l'outil avec les bons arguments PUIS explique dans ta réponse en quoi consiste l'action proposée (produit, quantité) et demande la confirmation. Utilise generer_fiche_manuelle uniquement si l'utilisateur donne des quantités précises par producteur, et divisions par exploitation si connues.
- Si l'utilisateur pose une question générale (conseils, réglementation), réponds directement sans outil."
        )
    })
}

fn chrono_ish_today() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = secs / 86_400;
    let (y, m, d) = civil_from_days(days as i64);
    format!("{y:04}-{m:02}-{d:02}")
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

fn tools_specs() -> Vec<ToolSpec> {
    let specs = [
        ("stats_globales", "Chiffres clés de la base de données de l'utilisateur : nombre d'exploitations, d'unités, de cultures, superficie totale, fiches de traçabilité, quantités allouées, produit le plus cultivé.", json!({ "type": "object", "properties": {} })),
        ("rechercher_producteurs", "Rechercher des producteurs/exploitations agricoles à partir d'un texte libre (opérateur, village, district, culture, unité, identifiant, inspecteur). Retourne jusqu'à 100 résultats avec leurs caractéristiques.", json!({ "type": "object", "properties": { "query": { "type": "string", "description": "Terme de recherche (nom, village, culture, ID...)" } }, "required": ["query"] })),
        ("lister_fiches", "Lister toutes les fiches de traçabilité existantes (référence, produit, quantités, statut).", json!({ "type": "object", "properties": {} })),
        ("stocks_produits", "Disponibilité des produits : quantité estimée, allouée et disponible par produit, et nombre d'exploitations concernées.", json!({ "type": "object", "properties": {} })),
        ("producteurs_pour_fiche", "Lister les producteurs éligibles et leur quantité hebdomadaire pour un produit donné.", json!({ "type": "object", "properties": { "produit": { "type": "string", "description": "Nom exact du produit" } }, "required": ["produit"] })),
        ("generer_fiche", "Proposer la création d'une fiche de traçabilité pour un produit et une quantité. NON exécuté : l'utilisateur confirme avant.", json!({ "type": "object", "properties": { "produit": { "type": "string" }, "quantite_kg": { "type": "number" }, "titre": { "type": "string" } }, "required": ["produit", "quantite_kg"] })),
        ("generer_fiche_manuelle", "Proposer la création d'une fiche de traçabilité avec des répartitions précises [(culture_id, quantite_kg), ...]. NON exécuté : l'utilisateur confirme avant.", json!({ "type": "object", "properties": { "produit": { "type": "string" }, "allocations": { "type": "array", "items": { "type": "array", "prefixItems": [{ "type": "integer" }, { "type": "number" }], "minItems": 2, "maxItems": 2 } }, "titre": { "type": "string" } }, "required": ["produit", "allocations"] })),
    ];
    specs
        .into_iter()
        .map(|(name, description, parameters)| ToolSpec {
            r#type: "function".into(),
            function: ToolFunction {
                name: name.to_string(),
                description: description.to_string(),
                parameters,
            },
        })
        .collect()
}

fn exec_tool(db: &Db, token: &str, name: &str, args: &serde_json::Value) -> Result<String, String> {
    let value = match name {
        "stats_globales" => serde_json::to_value(db.stats_globales(token)?)
            .map_err(|e| e.to_string())?,
        "rechercher_producteurs" => {
            let query = args.get("query").and_then(|q| q.as_str()).unwrap_or("");
            if query.trim().is_empty() {
                json!({ "erreur": "requête vide" })
            } else {
                let result = db.search_exploitations(token, query, SearchCategory::All)?;
                serde_json::to_value(result).map_err(|e| e.to_string())?
            }
        }
        "lister_fiches" => serde_json::to_value(db.list_fiches(token)?).map_err(|e| e.to_string())?,
        "stocks_produits" => {
            serde_json::to_value(db.produits_disponibles(token)?).map_err(|e| e.to_string())?
        }
        "producteurs_pour_fiche" => {
            let produit = args.get("produit").and_then(|p| p.as_str()).unwrap_or("");
            if produit.trim().is_empty() {
                json!({ "erreur": "produit requis" })
            } else {
                serde_json::to_value(db.producteurs_pour_fiche(token, produit)?)
                    .map_err(|e| e.to_string())?
            }
        }
        _ => json!({ "erreur": format!("Outil inconnu : {name}") }),
    };
    serde_json::to_string(&value).map_err(|e| e.to_string())
}

fn build_request(model: &str, messages: Vec<serde_json::Value>) -> ApiRequest {
    ApiRequest {
        model: model.to_string(),
        messages,
        tools: tools_specs(),
        temperature: 0.4,
        max_tokens: 1600,
    }
}

pub async fn chat(db: &Db, token: &str, history: Vec<ChatMsg>) -> Result<AgentResponse, String> {
    let user = db.validate(token)?;
    let mut ai_session = db
        .get_ai_session(user.id)?
        .ok_or_else(|| "Connexion au service IA indisponible. Déconnectez-vous puis reconnectez-vous.".to_string())?;
    let configured_model = db
        .get_setting(SETTING_MODEL)?
        .filter(|m| !m.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_MODEL.to_string());
    let model = if ["deepseek-chat", "deepseek-reasoner"].contains(&configured_model.as_str()) {
        configured_model
    } else {
        DEFAULT_MODEL.to_string()
    };
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(90))
        .build()
        .map_err(|_| "Impossible d'initialiser le service IA.".to_string())?;
    let endpoint = format!("{AI_API_BASE}/chat/completions");

    let mut messages: Vec<serde_json::Value> = vec![system_prompt()];
    for m in history {
        let role = if m.role == "assistant" { "assistant" } else { "user" };
        messages.push(json!({ "role": role, "content": m.content }));
    }

    let mut used_tools: Vec<String> = Vec::new();
    let mut actions: Vec<ProposedAction> = Vec::new();

    for _step in 0..MAX_STEPS {
        let req = build_request(&model, messages.clone());
        let resp = send_completion(
            &client,
            &endpoint,
            db,
            user.id,
            &mut ai_session,
            &req,
        )
        .await?;

        let message = resp
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| "Réponse IA vide".to_string())?
            .message;

        let Some(calls) = message.tool_calls else {
            return Ok(AgentResponse {
                reply: message.content.unwrap_or_default(),
                actions,
                used_tools,
                model,
            });
        };

        let mut assistant_msg = json!({ "role": "assistant" });
        if let Some(content) = &message.content {
            assistant_msg["content"] = json!(content);
        }
        assistant_msg["tool_calls"] = serde_json::to_value(&calls).map_err(|e| e.to_string())?;
        messages.push(assistant_msg);

        for call in calls {
            let name = call.function.name.clone();
            let args: serde_json::Value = serde_json::from_str(&call.function.arguments)
                .unwrap_or_else(|_| json!({}));

            if name == "generer_fiche" || name == "generer_fiche_manuelle" {
                let label = if name == "generer_fiche_manuelle" {
                    "Créer une fiche avec répartition manuelle"
                } else {
                    "Créer une fiche de traçabilité"
                };
                actions.push(ProposedAction {
                    tool: name.clone(),
                    args: args.clone(),
                    label: label.to_string(),
                });
                messages.push(json!({
                    "role": "tool",
                    "tool_call_id": call.id,
                    "content": "Action planifiée mais NON exécutée : elle attend la confirmation explicite de l'utilisateur. Résumez-la et demandez la confirmation."
                }));
            } else {
                used_tools.push(name.clone());
                let result = exec_tool(db, token, &name, &args)?;
                messages.push(json!({
                    "role": "tool",
                    "tool_call_id": call.id,
                    "content": result
                }));
            }
        }
    }

    Err("L'assistant n'a pas pu finaliser sa réponse. Réessayez.".to_string())
}

async fn send_completion(
    client: &reqwest::Client,
    endpoint: &str,
    db: &Db,
    user_id: i64,
    ai_session: &mut AiSession,
    req: &ApiRequest,
) -> Result<ApiResponse, String> {
    for attempt in 0..2 {
        let response = client
            .post(endpoint)
            .bearer_auth(&ai_session.access_token)
            .header("Content-Type", "application/json")
            .json(req)
            .send()
            .await
            .map_err(|_| "Impossible de joindre le service IA.".to_string())?;

        let status = response.status();
        if status == reqwest::StatusCode::UNAUTHORIZED && attempt == 0 {
            let refreshed = client
                .post(format!("{AI_API_BASE}/auth/chat/refresh"))
                .json(&serde_json::json!({ "refresh_token": ai_session.refresh_token }))
                .send()
                .await
                .map_err(|_| "Session IA expirée. Reconnectez-vous.".to_string())?;
            if !refreshed.status().is_success() {
                return Err("Session IA expirée. Reconnectez-vous.".into());
            }
            let refreshed = refreshed
                .json::<RefreshedAiSession>()
                .await
                .map_err(|_| "Réponse invalide du service IA.".to_string())?;
            ai_session.access_token = refreshed.token;
            ai_session.refresh_token = refreshed.refresh_token;
            db.set_ai_session(
                user_id,
                &ai_session.access_token,
                &ai_session.refresh_token,
            )?;
            continue;
        }
        if !status.is_success() {
            let body = response.json::<serde_json::Value>().await.unwrap_or_default();
            let message = body
                .get("error")
                .and_then(|error| error.get("message"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or("Service IA temporairement indisponible.");
            return Err(match status.as_u16() {
                401 => format!("Authentification IA refusée ({status}) : {message}"),
                402 => format!("Crédit IA insuffisant ({status}) : {message}"),
                429 => format!("Trop de requêtes vers l'IA ({status}) : {message}"),
                404 => format!("Endpoint IA introuvable ({endpoint})"),
                _ => format!("Erreur IA ({status}) : {message}"),
            });
        }

        return response
            .json::<ApiResponse>()
            .await
            .map_err(|_| "Réponse IA illisible.".to_string());
    }

    Err("Authentification IA refusée. Reconnectez-vous.".into())
}