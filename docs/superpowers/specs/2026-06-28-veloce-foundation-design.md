# Veloce — Spec de conception : fondation v0.1

**Date :** 2026-06-28
**Statut :** Validé (en attente de revue finale utilisateur)
**Périmètre de cette spec :** v0.1 « fondation minimale » uniquement.

---

## 1. Vision

Veloce est un **client Discord natif écrit 100% en Rust**, dans l'esprit de
[Vencord](https://github.com/Vendicated/Vencord) : extensible (plugins + thèmes),
mais natif au lieu d'être une modification d'un client Electron. Objectifs
prioritaires : **rapidité** et **consommation mémoire/CPU minimale** — on remplace
tout le runtime Electron de Discord par du Rust natif.

L'extensibilité (plugins/thèmes) est l'identité du projet, mais ne peut exister
qu'une fois qu'un client fonctionnel est en place. La v0.1 construit donc la
**fondation**, en préservant dès maintenant les coutures (« seams ») nécessaires
aux plugins.

### Avertissement (CGU)

Un client tiers est dans une **zone grise des CGU Discord** et peut exposer un
compte à un risque de bannissement. Des projets natifs comparables existent
(Abaddon, discordo, purple-discord). Recommandation : tester avec un **compte
secondaire**.

---

## 2. Périmètre v0.1

### Inclus
- Authentification par **token utilisateur** (collé par l'utilisateur).
- Connexion **Gateway** (WebSocket) + **REST** Discord.
- Liste des serveurs (guilds) et des salons (channels).
- Lecture des messages d'un salon, avec mise à jour **temps réel**.
- Envoi de messages texte.
- Rendu **markdown basique** (gras, italique, code inline, blocs de code, barré,
  liens, mentions résolues en noms).
- Reconnexion automatique, gestion des rate limits.
- Stockage sécurisé du token (trousseau OS).

### Hors périmètre (pour plus tard, mais l'archi le prépare)
Voix / WebRTC · images, embeds et pièces jointes riches · réactions · threads ·
notifications système · recherche · **système de plugins/thèmes complet**.

### Critères de succès v0.1
1. Au lancement, après saisie du token, l'app affiche serveurs + salons.
2. En sélectionnant un salon, l'historique récent (≈50 derniers messages)
   s'affiche.
3. Un message envoyé depuis un autre client apparaît en temps réel dans Veloce.
4. Un message tapé dans Veloce arrive sur Discord et s'affiche.
5. Au repos (aucune activité), la consommation CPU est ≈0 (pas de polling).

---

## 3. Stack technique

| Domaine | Choix | Raison |
|---|---|---|
| Langage | Rust (édition 2021) | 100% Rust, perf, faible conso |
| Async | `tokio` | Standard de fait |
| WebSocket | `tokio-tungstenite` + `rustls` | Pas d'OpenSSL → plus léger/portable |
| REST | `reqwest` (backend rustls) | Ergonomique, async |
| Sérialisation | `serde` / `serde_json` | Modèles Discord |
| GUI | `egui` / `eframe` (backend `glow`) | Léger, itération rapide, cross-platform |
| Token | `keyring` | Trousseau OS, jamais en clair |
| Logs | `tracing` / `tracing-subscriber` | Diagnostic |
| Erreurs | `thiserror` (lib), `anyhow` (app) | Idiomatique |

Cross-platform : **macOS, Linux, Windows** dès le départ.

---

## 4. Architecture

### Organisation du workspace

```
Veloce/
├─ Cargo.toml                 # workspace
├─ crates/
│  ├─ veloce-discord/         # LIB : client Discord, UI-agnostique
│  │  └─ src/
│  │     ├─ lib.rs
│  │     ├─ rest.rs           # client REST + buckets de rate limit
│  │     ├─ gateway.rs        # connexion WS + boucle d'events
│  │     ├─ gateway_state.rs  # machine à états PURE (heartbeat/resume) — testable
│  │     ├─ identity.rs       # super properties / User-Agent (à maintenir)
│  │     ├─ models.rs         # structs serde (User, Guild, Channel, Message…)
│  │     ├─ events.rs         # enum Event (réseau → consommateur)
│  │     └─ commands.rs       # enum Command (consommateur → réseau)
│  └─ veloce-app/             # BIN : application egui
│     └─ src/
│        ├─ main.rs
│        ├─ app.rs            # état UI + boucle eframe
│        ├─ net.rs            # pont thread tokio <-> UI (channels)
│        ├─ markdown.rs       # parseur markdown -> egui::LayoutJob (PUR, testable)
│        └─ views/            # panneaux : guilds, channels, messages, composer
├─ docs/
├─ .github/workflows/ci.yml
├─ README.md  LICENSE  .gitignore
```

`veloce-discord` ne dépend **jamais** d'egui. C'est le cœur réutilisable et la
base des futurs plugins.

### Frontières des unités

- **`veloce-discord` (lib)** — *Quoi :* parle à Discord (REST + Gateway), expose
  des modèles, émet des `Event`, consomme des `Command`. *Dépend de :* tokio,
  reqwest, tungstenite, serde. *Ne connaît pas :* l'UI.
- **`gateway_state` (module pur)** — *Quoi :* décide des transitions (quand
  heartbeat, quand RESUME vs IDENTIFY, suivi de la séquence) à partir d'events
  entrants. *Pas d'I/O* → testable unitairement.
- **`veloce-app` (bin)** — *Quoi :* présente l'état, capture les entrées,
  traduit en `Command`, applique les `Event`. *Dépend de :* veloce-discord,
  eframe/egui.
- **`markdown` (module pur)** — *Quoi :* `&str` → `egui::LayoutJob`. *Pas
  d'I/O* → testable unitairement.

---

## 5. Connectivité Discord

### Gateway (WebSocket)

URL : `wss://gateway.discord.gg/?v=10&encoding=json`. Machine à états :

1. Réception `HELLO` (op 10) → `heartbeat_interval`.
2. Envoi `IDENTIFY` (op 2) : token + **super properties** (cf. `identity`).
3. Heartbeat (op 1) toutes les `heartbeat_interval` ms, avec dernier `seq`.
4. Réception `READY` / `READY_SUPPLEMENTAL` → user, guilds, channels.
5. Dispatch temps réel : `MESSAGE_CREATE`, `MESSAGE_UPDATE`, `MESSAGE_DELETE`
   (autres events ignorés en v0.1).
6. Reconnexion : `RESUME` (op 6, avec session_id + seq) ; sur `INVALID_SESSION`
   (op 9) ou `RECONNECT` (op 7) → repli ré-IDENTIFY ; backoff exponentiel.

### REST

Base : `https://discord.com/api/v10`. En-tête `Authorization: <token>` (token
**user**, sans préfixe `Bot`). Endpoints v0.1 :

- `GET /users/@me` — valider le token, récupérer l'utilisateur courant.
- `GET /users/@me/guilds` — liste des serveurs.
- `GET /guilds/{id}/channels` — salons d'un serveur.
- `GET /channels/{id}/messages?limit=50` — historique récent.
- `POST /channels/{id}/messages` — envoyer un message.

### Identité client (`identity`)

Pour ne pas être flaggé immédiatement, les requêtes (REST + IDENTIFY) doivent
inclure des **super properties** réalistes (`X-Super-Properties` base64 : build
number, OS, version client, locale…) et un `User-Agent` cohérent. **Dette de
maintenance connue** : le build number évolue côté Discord. Tout est isolé dans
`identity.rs` pour une mise à jour en un seul endroit.

---

## 6. Flux de données UI ⇄ réseau

Deux threads, deux canaux :

```
[ Thread UI : eframe/egui ]  <--Event--  [ Thread tokio : gateway + REST ]
          |                                          ^
          +----------------- Command ----------------+
```

- **`Command`** (UI → réseau, `tokio::sync::mpsc`) : `SelectChannel(id)`,
  `FetchHistory(channel_id)`, `SendMessage{channel_id, content}`.
- **`Event`** (réseau → UI, canal + `Context::request_repaint`) : `Connecting`,
  `Ready{user, guilds}`, `ChannelsLoaded{guild_id, channels}`,
  `MessagesLoaded{channel_id, messages}`, `MessageCreated(message)`,
  `MessageUpdated`, `MessageDeleted`, `ConnectionState(state)`, `Error(msg)`.
- Le réseau appelle `egui::Context::request_repaint()` à chaque `Event` →
  **aucun polling**, CPU ≈0 au repos (critère de succès #5).

Le runtime tokio tourne sur un thread dédié (`std::thread::spawn` +
`tokio::runtime`). eframe garde le thread principal.

---

## 7. Interface (egui)

Disposition **3 panneaux** type Discord :

- **Gauche** — liste des serveurs (icône/nom).
- **Milieu** — salons du serveur sélectionné.
- **Droite** — messages du salon (scroll, auteur + contenu rendu markdown) +
  **zone de saisie** (Entrée pour envoyer).

Markdown basique via `egui::LayoutJob` : `**gras**`, `*italique*`, `` `code` ``,
blocs ```` ``` ````, `~~barré~~`, liens cliquables, mentions `<@id>` → nom.
Spoilers et markdown avancé : plus tard.

---

## 8. Robustesse

- **Reconnexion** : RESUME prioritaire, backoff exponentiel plafonné, repli
  ré-IDENTIFY.
- **Rate limits REST** : respect du `Retry-After` sur HTTP 429 ; file d'attente
  par bucket (par route).
- **Token** : `keyring` (trousseau OS). Saisie au premier lancement, validé via
  `GET /users/@me` avant connexion gateway. Jamais écrit en clair sur disque.
- **État de connexion** toujours visible dans l'UI (connecté / reconnexion /
  erreur).
- **Erreurs** : `thiserror` côté lib (types d'erreur explicites), `anyhow` côté
  app.

---

## 9. Stratégie de test (TDD)

- **Modèles** : tests de désérialisation `serde` sur **fixtures JSON** capturées
  (READY, MESSAGE_CREATE, guild, channel…).
- **`gateway_state`** : tests unitaires des transitions (heartbeat dû, RESUME vs
  IDENTIFY, suivi de séquence, INVALID_SESSION) — fonctions pures, sans réseau.
- **`markdown`** : tests unitaires `&str` → `LayoutJob` (chaque syntaxe + cas
  imbriqués/dégénérés).
- **REST** : tests des helpers de parsing/buckets ; les appels live nécessitent
  un token et restent hors CI.
- Approche TDD : test rouge → implémentation → vert, pour chaque unité ci-dessus.

---

## 10. Couture pour les plugins (préparée, non implémentée)

La v0.1 **ne construit pas** le système de plugins, mais préserve les seams :

- `Event` et `Command` sont des **types publics stables** dans `veloce-discord`.
- Les modèles (`Message`, `Guild`, `Channel`, `User`) sont publics et stables.
- Point d'extension futur : un `Plugin` trait pourra **observer** le flux
  d'`Event` et **injecter** des `Command`, et l'UI exposera des hooks de rendu.
- Objectif : ajouter les plugins plus tard **sans casser** la fondation.

---

## 11. Repo & qualité

- Nom **Veloce**, repo **public**, branche `main`.
- **Licence : GPL-3.0** (esprit Vencord) — décidée, copyleft pour garder le
  projet et ses dérivés ouverts.
- **CI GitHub Actions** (macOS/Linux/Windows) : `cargo fmt --check`,
  `cargo clippy -- -D warnings`, `cargo test`.
- `README.md` (présentation, avertissement CGU, build), `.gitignore` Rust.

---

## 12. Risques connus

| Risque | Mitigation |
|---|---|
| Flag/ban du compte (client tiers) | Avertissement explicite ; conseiller compte secondaire ; super properties réalistes |
| Build number Discord obsolète | Isolé dans `identity.rs`, mise à jour en un point |
| API user non documentée / changeante | S'appuyer sur les comportements connus ; modèles tolérants (champs optionnels) |
| Captcha/2FA (login complet) | Évité en v0.1 via token utilisateur |
