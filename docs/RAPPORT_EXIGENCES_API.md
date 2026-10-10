# Rapport des exigences API AgroBox — Fonctionnalité agentique de Vobox

> **Destinataire** : équipe qui développe l'API AgroBox (relais IA).
> **Objet** : tout ce dont l'application desktop Vobox a besoin côté API AgroBox pour faire fonctionner l'assistant agentique : **la clé du fournisseur LLM (DeepSeek) est stockée sur l'API, jamais dans l'application**.
> **Sources** : implémentation réelle du client (`src-tauri/src/agent.rs`, `src-tauri/src/lib.rs`, `src/composables/useAssistant.ts`, `src/views/Assistant.vue`).
> Ce rapport complète (et parfois remplace) le contrat `docs/API_CONTRAT.md`. En cas de divergence, **ce document fait foi**.

---

## 1. Décision d'architecture et conséquences

| Avant (à abandonner) | Après (cible) |
|---|---|
| L'utilisateur saisit une clé DeepSeek dans l'app | Aucune clé côté client |
| L'app appelle `https://api.deepseek.com` directement | L'app appelle **votre API** uniquement |
| Panneau « Connexion de l'assistant » (URL / clé / modèle) | Panneau **supprimé**, invisible pour l'utilisateur |
| L'utilisateur choisit le modèle | L'API **décide** du modèle ; l'app affiche celui renvoyé |

**Conséquence directe :** l'application ne peut plus être fonctionnelle tant que votre API n'expose pas les deux endpoints du §3. Il n'y a plus de mode dégradé hors-ligne pour l'assistant : sans token, l'assistant est désactivé avec un message explicite (les autres fonctions de l'app restent 100 % locales).

**Ce que l'API devient, concrètement :** un *proxy d'authentification + un proxy de complétion* vers DeepSeek. L'API détient la clé, l'app détient le compte utilisateur.

---

## 2. Chaîne d'appels attendue (vue d'ensemble)

```
┌─ App Vobox (desktop, local) ────────────────────────────────┐
│ 1. POST {API}/auth/register ou /auth/chat                  │
│      { name?, email, password, app_version, ... }          │
│      → { token, refresh_token, expires_in } (local)        │
│ 2. POST {API}/chat/completions                               │
│      Bearer <token>  + messages + tools (OpenAI)            │
│      → { choices[0].message }                               │
│ 3. l'app exécute les outils demandés **en local (SQLite)**  │
│ 4. POST {API}/chat/completions   (message role:"tool")      │
│      → réponse finale en français                           │
│    (boucle plafonnée à 6 requêtes par message utilisateur) │
└──────────────────────────────────────────────────────────────┘
```

Le point clé : **les outils ne s'exécutent pas sur l'API**. L'API relaie des `tool_calls` vers DeepSeek et l'app répond avec les résultats de ses requêtes SQLite. Votre API n'a donc **jamais** accès aux données agricoles de l'utilisateur.

---

## 3. Endpoints requis

### 3.1 `POST {base}/auth/chat` — obtention du token IA

Au premier `register`, l'app crée aussi le compte API et reçoit ses jetons IA.
Au `login`, l'app échange email/mot de passe contre des jetons IA. Si le compte local
préexiste sur une ancienne version et n'existe pas encore sur l'API, l'app tente la
création API avant de déclarer l'assistant indisponible. Une panne API ne bloque pas
les fonctions locales.

**Requête de création / migration du compte**

```http
POST {base_url}/auth/register
Content-Type: application/json
```

```json
{
  "name": "Nom utilisateur",
  "email": "user@example.com",
  "password": "••••••••",
  "app_version": "0.1.0",
  "device_id": ""
}
```

Cet endpoint crée le compte API absent. Pour un email déjà enregistré, il ne délivre
des jetons que si le mot de passe fourni est correct ; il ne doit jamais remplacer
le mot de passe d'un compte existant.

**Réponse attendue**

Même format que `/auth/chat` : `token`, `refresh_token`, `expires_in` et
`refresh_expires_in`.

**Requête émise par l'app**

```http
POST /auth/chat HTTP/1.1
Host: api.agrobox.com
Content-Type: application/json
```

```json
{ "email": "user@example.com", "password": "••••••••",
  "app": "vobox-desktop", "app_version": "0.1.0", "device_id": "" }
```

| Champ | Type | Requis | Remarque |
|---|---|---|---|
| `email` | string | oui | identifiant de connexion local de l'app |
| `password` | string | oui | en clair **sur TLS uniquement**, jamais loggé |
| `app` | string | oui | identité du client (`vobox-desktop` ou `agrobox-desktop`) |
| `app_version` | string | oui | version de l'app (mise à jour / stats) |
| `device_id` | string (UUID) | recommandé | allows révocation ciblée du token |

**Réponse attendue (200)**

```json
{ "token": "eyJhbGciOi…", "expires_in": 2592000 }
```

`expires_in` est en **secondes**. L'app conserve en local le jeton d'accès et le
refresh token, et les utilise pour un renouvellement automatique après un `401`.
La clé fournisseur DeepSeek n'est jamais envoyée à l'app.

**Variantes tolérées** (au choix, mais une seule forme doit être implémentée) :

```json
{ "token": "…", "expires_at": 1893456000 }
{ "access_token": "…", "expires_in": 2592000, "token_type": "Bearer" }
```

Si vous utilisez des JWT expirants : renvoyer `expires_in = min(30 j, durée de vie du JWT)`, ou **mieux**, un JWT long (90 j) + refresh (cf. §3.4) — l'app n'aime pas les tokens courts car elle n'a pas toujours le mot de passe en mémoire.

**Erreurs attendues**

| Code | Cas | Message attendu (affiché tel quel à l'utilisateur) |
|---|---|---|
| `400` | corps invalide | « Requête invalide » |
| `401` | email / mot de passe inconnus | « Identifiants non reconnus par le service IA » |
| `403` | compte non vérifié, suspendu, quota nul | « Service IA non disponible pour ce compte » |
| `429` | trop de tentatives | « Trop de tentatives, réessayez dans quelques minutes » |
| `5xx` | panne | « Service IA momentanément indisponible » |

### 3.2 `POST {base}/chat/completions` — relais de complétion

Contrat **strictement OpenAI-compatible**. Aucune logique agentique, aucun filtrage, aucune réécriture de `messages`/`tools`.

**En-têtes envoyés par l'app**

```http
POST /chat/completions HTTP/1.1
Authorization: Bearer <token-ia>
Content-Type: application/json
```

**Corps réellement émis** (extrait fidèle de `agent.rs`)

```json
{
  "model": "deepseek-chat",
  "messages": [
    { "role": "system", "content": "Tu es l'assistant agro-agentic de Vobox…" },
    { "role": "user", "content": "Quel est l'état de mes exploitations ?" },
    {
      "role": "assistant",
      "content": null,
      "tool_calls": [
        { "id": "call_01", "function": { "name": "stats_globales", "arguments": "{}" } }
      ]
    },
    { "role": "tool", "tool_call_id": "call_01", "content": "{\"nb_exploitations\":12,…}" }
  ],
  "tools": [
    {
      "type": "function",
      "function": {
        "name": "stats_globales",
        "description": "Chiffres clés de la base…",
        "parameters": { "type": "object", "properties": {} }
      }
    }
  ],
  "temperature": 0.4,
  "max_tokens": 1600
}
```

À noter : `tools` (7 fonctions) est renvoyé **à chaque requête**, y compris aux tours suivants. `stream` n'est **jamais** envoyé →impérativement ne pas activer le streaming par défaut.

**Réponse attendue (200)**

```json
{
  "choices": [
    { "message": { "role": "assistant", "content": "Vous avez 12 exploitations…" } }
  ]
}
```

ou, appel d'outil :

```json
{
  "choices": [
    {
      "message": {
        "role": "assistant",
        "content": null,
        "tool_calls": [
          {
            "id": "call_07",
            "type": "function",
            "function": { "name": "rechercher_producteurs", "arguments": "{\"query\":\"Kouamé\"}" }
          }
        ]
      }
    }
  ]
}
```

L'app ne lit que `choices[0].message.{content, tool_calls}`. Les champs `id`, `created`, `usage`, `model`, `finish_reason` sont ignorés (mais `model` est **affiché** à l'utilisateur : renvoyez le nom réel du modèle utilisé).

### 3.3 `GET {base}/health` — diagnostic

```http
GET /health  →  200  {"status":"ok"}
```

Utilisé pour distinguer « app hors-ligne / API injoignable » de « clé IA manquante » dans les messages d'erreur. Bonus : `GET /health` peut vérifier la joignabilité de DeepSeek et renvoyer `{"status":"degraded"}` avec `503`.

### 3.4 Optionnel mais recommandé

| Endpoint | Usage |
|---|---|
| `POST {base}/auth/chat/refresh` `{ refresh_token }` | évite de redemander le mot de passe ; l'app le stocke si vous le renvoyez (`refresh_token` + `refresh_expires_in`) |
| `GET {base}/auth/chat/check` `Authorization: Bearer` | valide un token au démarrage, évite d'attendre un 401 sur la première conversation |
| `GET {base}/models` | alimente le sélecteur de modèle (si vous voulez l'exposer ; sinon l'app masque le sélecteur) |

---

## 4. Exigences techniques non négociables (points de rupture)

Chaque ligne correspond à un mode de défaillance observé ou possible côté client.

| # | Exigence | Si non respectée |
|---|---|---|
| **R1** | `tool_calls[].function.arguments` doit être une **chaîne JSON** (pas un objet) | l'app parse `{}` → l'outil est appelé **sans arguments**, la réponse est faussée silencieusement |
| **R2** | `tool_calls[].id` est **obligatoire** et unique | erreur de désérialisation → conversation cassée au 2ᵉ tour |
| **R3** | `message.content` peut valoir `null` (souvent le cas quand il y a des `tool_calls`) | erreur de parsing JSON |
| **R4** | Réponse = **JSON en une seule pièce**, non streamée | « Réponse IA illisible » |
| **R5** | Tous les messages (`system`, `user`, `assistant`+`tool_calls`, `tool`) sont **relayés à l'identique** | l'agent perd la boucle d'outils |
| **R6** | Les erreurs HTTP renvoient `{ "error": { "message": "…" } }` | l'app affiche un corps brut / vide |
| **R7** | `message` est **en français et directement affichable** (une phrase, pas de trace technique, pas de JSON) | l'utilisateur voit une stack trace |
| **R8** | HTTPS + TLS 1.2 minimum, redirection HTTPS→HTTPS | reqwest refuse la connexion |
| **R9** | Pas d'HTTP 100-continue / keep-alive cassé, `Content-Type: application/json` | refus de connexion ou parse incorrecte |
| **R10** | `401` réservé à un vrai problème de token (jamais sur erreur métier) | l'app tente un refresh et boucle |

### 4.1 Point d'attention connu (écart côté application)

Quand l'app renvoie l'historique au tour suivant, elle **re-sérialise** les `tool_calls` reçus sans le champ `type` :

```json
{ "id": "call_07", "function": { "name": "…", "arguments": "…" } }
```

Le schéma OpenAI strict exige `"type": "function"`. DeepSeek est généralement tolérant, mais ce n'est pas garanti.

**Deux remèdes possibles (au choix) :**

- **côté API (recommandé, robuste)** : normaliser les `tool_calls` entrants avant d'envoyer à DeepSeek → ajouter `"type": "function"` si absent ;
- **côté app** : ajouter le champ dans `agent.rs` (une ligne, correction que je peux faire dès validation).

---

## 5. Modèles et quotas

**Modèles envoyés par l'app** : `deepseek-chat`, `deepseek-reasoner` (sélecteur). Si votre API ne veut pas exposer DeepSeek, elle est libre de **remplacer** `model` par son propre identifiant interne et de renvoyer le modèle réellement utilisé dans la réponse.

**Volume par message utilisateur** (important pour dimensionner et facturer) :

| Métrique | Valeur |
|---|---|
| Requêtes HTTP vers l'API | **1 à 6** (boucle d'outils, plafond `MAX_STEPS = 6`) |
| `tools` déclarés par requête | 7 (retransmis intégralement) |
| `max_tokens` par requête | 1600 |
| `temperature` | 0.4 |
| Historique | complet, sans troncature côté app (la fenêtre de conversation peut grossir) |

**Quotas / rate limiting recommandés**

- Par utilisateur (token) : ~30 requêtes/min, ~500 requêtes/jour ; par IP en secours.
- En-têtes utiles (l'app les ignore aujourd'hui, ils serviront à une future UI de quota) : `X-RateLimit-Limit`, `X-RateLimit-Remaining`, `X-RateLimit-Reset`, `Retry-After` sur `429`.
- **Budget LLM** : prévoir un plafond par utilisateur ; renvoyer `402` quand il est atteint (l'app sait déjà afficher « Crédit IA insuffisant »).
- Si vous utilisez un modèle raisonneur (`deepseek-reasoner`), `max_tokens: 1600` peut être juste : autorisez le relais à **augmenter `max_tokens`** pour ce modèle, ou limitez-vous à `deepseek-chat` par défaut.

---

## 6. Sécurité et confidentialité

**Données qui quittent le poste de l'utilisateur, et uniquement celles-là :**

1. `email` + `password` (une fois, à `/auth/chat`, en TLS) ;
2. le **texte de la conversation** (questions de l'utilisateur, réponses du modèle) ;
3. les **résultats d'outils**, sérialisés en JSON : statistiques agrégées, listes de producteurs/exploitations, stocks, fiches de traçabilité.

**Exigences :**

- ❌ Ne jamais **logger** : corps de requête, contenu des messages, `tool results`, mots de passe, tokens.
- ❌ Ne jamais **exposer** la clé DeepSeek (ni réponse, ni header, ni message d'erreur du fournisseur).
- ❌ Ne **jamais exécuter** les outils, ni supposer l'existence de la base de données de l'utilisateur.
- ✅ Rétention des logs : **zéro**, ou anonymisée + purge < 30 jours.
- ✅ Hachage des mots de passe côté serveur si l'API gère elle-même des comptes (Argon2id / bcrypt).
- ✅ `Authorization` comparé en temps constant ; token lié au compte, révocable.
- ✅ Ne jamais mettre de données(user) dans une URL ou un query string (tout en POST/header).
- ✅ Le contenu utilisateur peut contenir des données personnelles d'agriculteurs : ne pas l'utiliser pour entraîner un modèle, ne pas le transmettre à un tiers autre que DeepSeek.

---

## 7. Erreurs : table de correspondance (à tester)

| Situation | HTTP | Corps minimal | Affichage app |
|---|---|---|---|
| Token absent / expiré | `401` | `{"error":{"message":"Session expirée"}}` | « Authentification IA refusée (401) … » puis refresh + 1 retry |
| Crédit/quota épuisé | `402` | `{"error":{"message":"Quota épuisé"}}` | « Crédit IA insuffisant (402) … » |
| Base `/v1` absente | `404` | `{"error":{"message":"Not found"}}` | « Endpoint IA introuvable … » (sert à détecter une mauvaise URL) |
| Rate limit | `429` | + `Retry-After` | « Trop de requêtes vers l'IA (429) … » |
| Panne amont (DeepSeek) | `502` / `503` | `{"error":{"message":"Amont indisponible"}}` | « Erreur IA (503) … » |
| Timeout amont | `504` | idem | idem |
| Corps non-JSON | `200` mais HTML | — | « Réponse IA illisible » |

---

## 8. Conséquences directes côté application (travail que j'engage)

| # | Modification app | Dépend de l'API |
|---|---|---|
| A1 | Constante unique `AI_BASE_URL` côté Rust (URL de votre API, plus de réglage utilisateur) | votre URL de prod |
| A2 | Suppression du panneau « Connexion de l'assistant » (URL / clé / modèle) et du sélecteur de modèle si vous n'exposez pas `/models` | — |
| A3 | Acquisition automatique du token après `login` et `register` (best-effort, non bloquant, + à l'ouverture de session) | `POST /auth/chat` |
| A4 | Stockage du token en local (`settings`, clé `ai_api_key` → `ai_token` + `ai_token_expires_at`) | — |
| A5 | Sur `401` : 1 refresh + 1 retry automatique, sinon message « reconnectez-vous » | `POST /auth/chat` (ou refresh) |
| A6 | Hors-ligne / API injoignable : assistant désactivé + message explicite ; **le reste de l'app reste utilisable** | `GET /health` (bonus) |
| A7 | Correctif `type: "function"` dans les `tool_calls` renvoyés (cf. §4.1) | — |
| A8 | `deepseek-reasoner` : `max_tokens` relevé si nécessaire | votre arbitrage quota |

Rien de tout cela ne peut être validé avant que `POST /auth/chat` et `POST /chat/completions` ne répondent au format du §3.

---

## 9. Plan de test / recette

**9.1 Jeu de données local (pour tester la boucle d'outils)**
L'app doit disposer de données réelles (import Excel) : ~10 exploitations, cultures, unités, fiches, allocations. Sans données, l'agent ne peut utiliser que `stats_globales` et répondra « base vide ».

**9.2 Scenarios de recette**

| # | Scénario | Attendu |
|---|---|---|
| T1 | `POST /auth/chat` identifiants valides | `200` + token, `expires_in` présent |
| T2 | mauvais mot de passe | `401` + message FR lisible |
| T3 | question simple, aucun outil (« bonjour ») | 1 requête, réponse texte FR |
| T4 | « Combien d'exploitations ? » | 2 requêtes : `stats_globales` puis réponse chiffrée |
| T5 | « Cherche les producteurs de Kouamé » | `rechercher_producteurs` avec `arguments` = chaîne JSON correcte |
| T6 | « Crée une fiche ananas 500 kg » | `generer_fiche` **proposé**, **jamais exécuté** sans confirmation UI |
| T7 | token expiré | `401` → refresh automatique → réponse OK |
| T8 | API down (`/health` en échec) | message clair, app hors assistant utilisable |
| T9 | requête > 30 s | message lisible, pas de crash |
| T10 | conversation longue (20 messages) | pas de troncature ni erreur de taille |

**9.3 Commandes prêtes à jouer (à exécuter côté API)**

```bash
# Auth
curl -i -X POST https://api.agrobox.com/auth/chat \
  -H 'Content-Type: application/json' \
  -d '{"email":"a@b.c","password":"secret","app":"vobox-desktop","app_version":"0.1.0"}'

# Chat sans outil
curl -i -X POST https://api.agrobox.com/chat/completions \
  -H 'Authorization: Bearer <TOKEN>' -H 'Content-Type: application/json' \
  -d '{"model":"deepseek-chat","messages":[{"role":"user","content":"ping"}],"max_tokens":50}'

# Chat avec boucle d'outils (2e tour)
curl -i -X POST https://api.agrobox.com/chat/completions \
  -H 'Authorization: Bearer <TOKEN>' -H 'Content-Type: application/json' \
  -d '{"model":"deepseek-chat","messages":[
        {"role":"user","content":"Combien d exploitations ?"},
        {"role":"assistant","content":null,"tool_calls":[{"id":"call_01","type":"function","function":{"name":"stats_globales","arguments":"{}"}}]},
        {"role":"tool","tool_call_id":"call_01","content":"{\"nb_exploitations\":12}"}],
      "tools":[{"type":"function","function":{"name":"stats_globales","description":"stats","parameters":{"type":"object","properties":{}}}}],
      "temperature":0.4,"max_tokens":1600}'
```

**9.4 Endpoint de test (recommandé pour découpler nos developments)**

Tant que votre API n'est pas prête, un mock compatible suffit à développer l'app :

```
POST /auth/chat            → { "token": "mock", "expires_in": 2592000 }
POST /chat/completions     → { "choices": [ { "message": { "role": "assistant", "content": "…" } } ] }
                            (ou un tool_call de test au 1er tour pour valider la boucle)
GET  /health               → { "status": "ok" }
```

Un mock « con » (qui renvoie volontairement des `arguments` en objet, un `id` manquant, un `content: null`) est très utile : il vérifie que l'app ne casse pas et remonte une erreur propre. Je peux l'écrire dans le dépôt si vous le souhaitez.

---

## 10. Questions ouvertes (à trancher avant l'implémentation)

1. **URL de production** exacte, et faut-il un préfixe `/v1` ?
2. **Comptes** : l'API gère-t-elle sa propre base d'utilisateurs (mêmes emails que l'app) ou vérifie-t-elle un compte distant existant ? L'app envoie email + mot de passe : cela suppose que l'API possède (ou peut vérifier) ces identifiants.
3. Durée de vie du token : **30 j** (simple) ou **JWT court + refresh** (robuste) ?
4. **Modèle** : l'app doit-elle exposer `deepseek-chat` / `deepseek-reasoner`, ou votre API masque-t-elle DeepSeek derrière un nom interne ?
5. **Crédits/quota** : géré par compte ? par abonnement ? `402` en cas d'épuisement ?
6. **Révoquation** : faut-il un `device_id` pour pouvoir révoquer une session précise ?
7. Un endpoint de **telemetry** (usage tokens par utilisateur pour l'affichage « consommation ») est-il souhaité plus tard ?
8. Environnement de **test** : staging dispo pour valider la boucle d'outils avant la prod ?
9. **CORS / proxy** : l'app Tauri appelle directement votre API en HTTPS (pas de navigateur) → pas de CORS requis, confirmé ?
10. Existe-t-il une **politique de conservation** à respecter sur les conversations (RGPD) ? Par défaut je recommande : ne rien stocker.

---

## 11. Checklist API

- [ ] `POST /auth/chat` → `{ token, expires_in }`, erreurs FR, rate limiting, aucun log sensible
- [ ] `POST /chat/completions` → relais OpenAI **transparent** (`messages`, `tools`, `tool_calls`, `tool`) + normalisation `"type":"function"` (§4.1)
- [ ] Support `content: null` et `tool_calls` incomplets sans erreur 5xx
- [ ] Erreurs uniformes `{ "error": { "message": … } }` + mapping `401/402/404/429/502/503/504`
- [ ] HTTPS, non-streaming par défaut, `Content-Type: application/json`
- [ ] Clé DeepSeek jamais exposée, jamais loggée
- [ ] Quotas par utilisateur + en-têtes de rate limit
- [ ] `GET /health` (au minimum)
- [ ] Validation des 10 scénarios de recette (§9.2) sur un jeu de données réel
- [ ] Réponse aux 10 questions ouvertes (§10)

---

## 12. Annexe — outils déclarés par l'app (référencés par le modèle, exécutés en local)

| Outil | Paramètres | Exécution |
|---|---|---|
| `stats_globales` | `{}` | automatique |
| `rechercher_producteurs` | `{ query: string }` | automatique |
| `lister_fiches` | `{}` | automatique |
| `stocks_produits` | `{}` | automatique |
| `producteurs_pour_fiche` | `{ produit: string }` | automatique |
| `generer_fiche` | `{ produit: string, quantite_kg: number, titre?: string }` | **confirmation utilisateur obligatoire** |
| `generer_fiche_manuelle` | `{ produit: string, allocations: [[culture_id, quantite_kg], …], titre?: string }` | **confirmation utilisateur obligatoire** |

Votre API n'a rien à savoir de leur implémentation : elle ne fait que transporter les declarations et les résultats.