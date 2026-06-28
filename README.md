# Veloce

Client Discord **natif**, écrit **100% en Rust**, dans l'esprit de
[Vencord](https://github.com/Vendicated/Vencord) : rapide et léger. Là où Vencord
modifie le client Electron de Discord, Veloce le remplace par du natif (GUI
[egui](https://github.com/emilk/egui)) — pour une empreinte mémoire et CPU
minimale, et une architecture pensée pour les plugins.

> ⚠️ **Avertissement.** Veloce est un **client tiers**. Les clients tiers sont
> dans une zone grise des CGU de Discord et peuvent exposer un compte à un
> bannissement. Utilisez un **compte secondaire**. Vous utilisez Veloce à vos
> propres risques.

## État — v0.1 (fondation)

Première version fonctionnelle de bout en bout :

- Connexion par **token utilisateur** (validé avant connexion, stocké dans le
  **trousseau du système** — jamais en clair sur disque).
- **Gateway** temps réel (WebSocket) + **REST**, avec reconnexion automatique
  (backoff, RESUME/IDENTIFY) et respect des rate limits.
- Liste des **serveurs** et **salons texte**, lecture de l'historique, réception
  des messages en **temps réel**, **envoi** de messages.
- Rendu **markdown** basique (gras, italique, barré, code).
- Interface **3 panneaux** (serveurs · salons · messages) ; au repos, ~0 % CPU
  (aucun polling, repaint à la demande).

## Build & lancement

Prérequis : Rust ≥ 1.75.

```sh
cargo run --bin veloce
```

Au premier lancement, collez votre token Discord ; il est mémorisé dans le
trousseau de l'OS pour les lancements suivants. Un token invalide ramène à
l'écran de saisie (et purge le token mémorisé).

## Limitations connues

- **Temps réel sur gros serveurs :** pour les comptes utilisateur, Discord peut
  exiger une trame d'abonnement de guilde (gateway `op 14`) pour recevoir les
  `MESSAGE_CREATE` des grands serveurs. Non implémenté en v0.1 → sur un très
  gros serveur, les nouveaux messages peuvent ne pas arriver en direct.
- **Éditions de messages :** les `MESSAGE_UPDATE` partiels (embed seul, etc.)
  peuvent ne pas se mettre à jour.
- **Identité client :** le `client_build_number` des super-properties doit être
  maintenu à jour (un seul endroit : `crates/veloce-discord/src/identity.rs`).

## Hors périmètre v0.1 (à venir)

Voix/WebRTC · images, embeds et pièces jointes riches · réactions · threads ·
notifications système · recherche · **système de plugins/thèmes complet** (la
v0.1 en pose déjà la couture via les types `Event`/`Command` publics).

## Architecture

Workspace Cargo en deux crates :

- **`veloce-discord`** — le client Discord (modèles, Gateway, REST, machine à
  états du protocole), **100% UI-agnostique** et réutilisable. C'est la base des
  futurs plugins.
- **`veloce-app`** — l'application egui : un thread tokio fait le réseau, le
  thread principal fait l'UI, les deux communiquent par canaux.

Conception, plan d'implémentation et décisions : voir `docs/superpowers/`.

## Licence

[GPL-3.0](LICENSE).
