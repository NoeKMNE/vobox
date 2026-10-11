# Contrat API AgroBox — Exigences du client Vobox

> ⚠️ Ce document est **périodé** sur le point clé : la clé DeepSeek n'est **plus saisie dans l'app**, elle est stockée sur l'API AgroBox (qui la détient et relaie les appels).
>👉 **Document de référence à jour : [`docs/RAPPORT_EXIGENCES_API.md`](./RAPPORT_EXIGENCES_API.md)** (rapport complet, exigences non négociables, plan de recette).
> Ce fichier est conservé comme historique du contrat initial.

> Document à destination de l'équipe qui développe l'API publique d'AgroBox.
> Le client desktop décrit dans ce contrat est désormais Vobox. Pour compatibilité avec
> le champ `app` identifie Vobox comme `vobox-desktop`; les anciens clients AgroBox
> restent identifiés comme `agrobox-desktop`.
> Le logiciel (desktop Tauri, local) est le **seul client** de cette API, qu'il utilise exclusivement pour **l'assistant intelligent**. Toutes les autres fonctions (données, fiches, archivage, comptes locaux) restent 100 % locales et ne dépendent pas de l'API.

## 1. Vue d'ensemble

Vobox utilise l'API AgroBox à l'**URL de base** `https://api.agrobox.com` pour :

1. **S'authentifier** auprès de l'API et obtenir un **token d'accès IA** (vérification du compte de l'utilisateur).
2. **Relayer les conversations** de l'assistant vers le LLM, au format **OpenAI-compatible** (`/chat/completions`), en y incluant un **appel d'outils (function calling)**.
3. **Analyser les fiches historiques à la demande de l'utilisateur** : Vobox transmet au même endpoint le texte extrait d'un classeur ou une photo en entrée multimodale. Cette analyse sert uniquement à préparer un aperçu ; l'enregistrement local n'a lieu qu'après validation explicite.

L'API **ne doit jamais** :
- exposer, retourner ou loguer la clé du fournisseur LLM tierce ;
- recevoir ou stocker les mots de passe en clair (uniquement le hash côté serveur) ;
- transmettre un token IA au-delà de l'app qui l'a demandé.

---

## 2. Endpoint d'authentification (obtention du token IA)

Objectif : après la connexion locale réussie, l'app appelle l'API avec l'email + mot de passe de l'utilisateur pour obtenir le token utilisé ensuite en tant que `Authorization: Bearer` sur `/chat/completions`.

**Requête proposée**

```
POST {base_url}/auth/chat
Content-Type: application/json
```

```json
{
  "email": "user@example.com",
  "password": "••••••••",
  "app": "vobox-desktop",
  "app_version": "0.1.0"
}
```

**Réponse attendue (succès, 200)**

```json
{
  "token": "jwt-or-opaque-token",
  "expires_in": 2592000
}
```

**Erreurs attendues**

| Code | Signification attendue |
|---|---|
| `401` | Email/mot de passe invalides |
| `403` | Compte vérifié requis / suspendu |
| `429` | Trop de tentatives |
| `5xx` | Erreur interne |

Format d'erreur uniforme (utilisé partout) :

```json
{ "error": { "message": "Clair et exploitable par l'UI" } }
```

L'app doit ensuite pouvoir se reconnecter automatiquement lorsqu'un ancien signalement du token IA est détecté par un `401` sur `/chat/completions` (voir §3).

---

## 3. Endpoint de conversation

### 3.1 Contrat fonctionnel

```
POST {base_url}/chat/completions
Authorization: Bearer <token-ia>
```

Le corps est **exactement conforme** au standard OpenAI/DeepSeek `chat/completions`. Le relais doit :

- **accepter les `tool_calls` en entrée** (messages `assistant` contenant `tool_calls`) ;
- **retourner des `tool_calls`** quand le modèle veut utiliser un outil ;
- **accepter les messages `tool`** (`role: "tool"`, `tool_call_id`) pour boucler ;
- **NBT : renvoyer la réponse finale avec `role: "assistant"` et `content` texte** en français.

### 3.2 Corps de requête réellement émis par l'app

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
        {
          "id": "call_01",
          "type": "function",
          "function": { "name": "stats_globales", "arguments": "{}" }
        }
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

### 3.3 Réponse attendue

```json
{
  "choices": [
    {
      "message": {
        "role": "assistant",
        "content": "Vous avez 12 exploitations…",
        "tool_calls": null
      }
    }
  ]
}
```

Ou, en cas d'appel d'outil, `message.tool_calls` :

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

- `choices` : tableau, l'app lit **uniquement `choices[0].message`**.
- Champs `usage`, `id`, `created`, `model` : libres.
- Le relais **peut réécrire** le nom du modèle (ne pas exposer le modèle interne) ; l'app reflète simplement la valeur envoyée.

### 3.4 Outils déclarés par l'app (tableau `tools`)

L'app déclare **7 fonctions** ; l'API doit les laisser passer telles quelles (elle n'a pas à les exécuter) :

| name | Paramètres (`parameters`) |
|---|---|
| `stats_globales` | `{}` |
| `rechercher_producteurs` | `{ "query": string }` (requis) |
| `lister_fiches` | `{}` |
| `stocks_produits` | `{}` |
| `producteurs_pour_fiche` | `{ "produit": string }` (requis) |
| `generer_fiche` | `{ "produit": string, "quantite_kg": number }` (+ `titre` optionnel) |
| `generer_fiche_manuelle` | `{ "produit": string, "allocations": [[int, number], ...] }` (+ `titre` optionnel) |

L'API **déclare et relaie** ces descriptions sans les interpréter : c'est le client qui les exécute localement.

### 3.5 Erreurs / codes HTTP

| Code | Comportement attendu côté app |
|---|---|
| `401` | Token IA invalide → l'app tente une nouvelle authentification (`/auth/chat`), sinon affiche « Connexion requise ». |
| `402` | Crédit insuffisant → message explicite. |
| `404` | Endpoint introuvable → indique une URL/version erronée. |
| `429` | Quota dépassé → message + suggestion de réessayer plus tard. |
| `5xx` / timeout | « Service temporairement indisponible ». |

Toutes les erreurs doivent renvoyer `{ "error": { "message": string } }` — l'app affiche ce message **tel quel** à l'utilisateur.

---

## 4. Exigences transversales

### 4.1 Réseau & temps
- **HTTPS obligatoire**, TLS 1.2+.
- **Premier octet de réponse < 4 s** en nominal ; l'app tolère jusqu'à ~60 s pour une boucle complète.
- L'app envoie systématiquement `Content-Type: application/json` et `Authorization: Bearer <token>`.
- Pas de mécanisme de streaming requis aujourd'hui (l'app faits des requêtes unitaires) — le streaming est une **option future**.

### 4.2 Sécurité
- Le token IA est stocké localement par l'app (base SQLite locale) ; durée de vie recommandée ≥ 30 jours.
- **Renouvellement silencieux** : si `/chat/completions` répond `401`, l'app se ré-authentifie une fois puis réessaie la conversation.
- **Rate limiting** côté API sur l'IP + le token pour limiter l'abus.
- Ne **jamais** renvoyer la clé du fournisseur LLM dans une réponse ou un header.
- Ne **jamais** loguer : le contenu des messages, les emails complets ni les mots de passe.

### 4.3 Confidentialité des données
- L'API reçoit uniquement : email + mot de passe (auth), puis texte de conversation + **résultats d'outils** (données agricoles agrégées produites localement).
- Hors import volontaire d'une fiche historique, aucun fichier Excel, aucun avatar ni aucune donnée brute de la base ne transitent par l'API. Pour cette analyse, le contenu du fichier (texte du classeur ou photo) et la liste locale des producteurs/cultures sont envoyés au service IA ; l'API doit traiter ces données comme confidentielles, ne pas les journaliser et ne les conserver que le temps nécessaire à la requête.
- Politique de rétention des logs : **0** par défaut, ou anonymisée.

### 4.4 Versioning & compatibilité
- Chemin stable recommandé : **en-tête de version** (`X-AgroBox-Version: 1`) ou chemin `/v1/…`.
- Le contrat `/chat/completions` est figé : toute évolution doit rester **additive** (champs optionnels) pour ne pas casser les clients existants.
- Un endpoint **health** simple (ex. `GET {base_url}/health` → `200 {"status":"ok"}`) utile pour le diagnostic.

### 4.5 Intégration dans l'app (rappel du point de configuration unique)
Tout est résumé dans des constantes côté client :
- `AI_BASE_URL` (constant Rust unique) ← l'URL de ton backend ;
- endpoint auth → `POST {base}/auth/chat` ;
- endpoint chat → `POST {base}/chat/completions`.

L'app **n'expose aucune configuration réseau à l'utilisateur final** : mise en route transparente dès la connexion du compte.

---

## 5. Checklist pour l'équipe API

- [ ] Implémenter `POST /auth/chat` (retour `{ token, expires_in }`) avec vérification du compte.
- [ ] Implémenter `POST /chat/completions` = relais OpenAI-compatible, avec **passage intégral** des `tools`, `tool_calls` et messages `tool`.
- [ ] Formater toutes les erreurs en `{ error: { message } }`.
- [ ] Ajouter le rate limiting et la rétention zéro des prompts.
- [ ] Fournir `GET /health`.
- [ ] Tester la boucle complète : une question utilisateur → 1 ou 2 appels d'outil → réponse finale, avec le JSON d'exemple du §3.