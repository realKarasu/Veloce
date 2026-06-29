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
- Liste des **serveurs** et des **salons** fidèle à Discord : arborescence par
  **catégories**, icônes par type (#, 🔊, 📢…), et **masquage par permissions**
  (les salons sans accès n'apparaissent pas). Lecture de l'historique, réception
  des messages en **temps réel**, **envoi** de messages.
- Rendu **markdown** basique (gras, italique, barré, code).
- **Polices larges** (latin étendu, cyrillique, grec, symboles, CJK) et
  **emojis en couleur** partout — Unicode (twemoji) et custom Discord
  `<:nom:id>` — dans les messages comme dans les noms de salons/serveurs.
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

## Plugins

L'extensibilité dans l'esprit de Vencord : des plugins **écrits en Rust**,
compilés dans le binaire et **activables/désactivables à l'exécution**. Le
bouton **« ⚙ Plugins »** ouvre la fenêtre de gestion (toggle + réglages par
plugin) ; l'ensemble activé **persiste** entre les sessions.

Plugins intégrés en v1 :

- **TextReplace** — règles *texte → remplacement* appliquées aux messages
  envoyés (réglables).
- **MessageCounter** — compte les messages reçus pendant la session.
- **Loud** — met les messages affichés en MAJUSCULES (démo cosmétique).

Ajouter un plugin = implémenter le trait `Plugin`
(`crates/veloce-app/src/plugins/`) avec les hooks `on_event` /
`on_outgoing_message` / `on_render_content` / `settings_ui`, puis l'enregistrer
dans `PluginManager::builtin()`.

## Limitations connues

- **Temps réel sur gros serveurs :** Veloce envoie désormais une trame
  d'abonnement de guilde (gateway `op 14`) au clic sur un serveur, et la
  ré-émet après reconnexion — ce qui débloque les `MESSAGE_CREATE` des grands
  serveurs pour les comptes utilisateur. C'est de l'**API non documentée** :
  format « best-effort » isolé dans `build_guild_subscribe`, à **vérifier sur un
  vrai gros serveur** (ajustable en un seul endroit si Discord change).
- **Éditions de messages :** les `MESSAGE_UPDATE` partiels (embed seul, etc.)
  peuvent ne pas se mettre à jour.
- **Identité client :** le `client_build_number` des super-properties doit être
  maintenu à jour (un seul endroit : `crates/veloce-discord/src/identity.rs`).
- **Plugins :** seul l'**ensemble activé** persiste ; les réglages internes d'un
  plugin (ex. les règles de TextReplace) reviennent à leur défaut au
  redémarrage.
- **Emojis :** rendus en images couleur téléchargées depuis un CDN (1 requête
  par emoji unique, cache en session). Les emojis **animés** sont affichés en
  **statique**.

## Hors périmètre v0.1 (à venir)

Voix/WebRTC · images, embeds et pièces jointes riches · réactions · threads ·
notifications système · recherche · **thèmes** (CSS-like) · **chargement
dynamique** de plugins (WASM/.so) · injection de `Command` par les plugins. (Le
système de plugins statiques, lui, est déjà là — voir ci-dessus.)

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
