# Veloce — Spec de conception : salons fidèles (arborescence + permissions)

**Date :** 2026-06-29
**Statut :** Validé (design approuvé)
**Périmètre :** afficher la liste des salons comme Discord — catégories, ordre, icônes par type, et **masquage des salons non visibles** via calcul des permissions (`VIEW_CHANNEL`). Règle deux bugs : salons dans le désordre, et erreur 403 « Accès manquant » (50001).

---

## 1. Problème

Aujourd'hui `text_channels_sorted` ne garde que les salons texte (kind 0) et les
trie à plat par `position` → ordre faux (les catégories sont ignorées, les
positions sont relatives), et cliquer un salon sans droit déclenche un 403
affiché en bannière rouge. Un client fidèle groupe par **catégories**, ordonne
correctement, montre les types de salons, et **masque** ce que l'utilisateur ne
peut pas voir.

## 2. Modèles (`veloce-discord/src/models.rs`)

- `Channel` : ajouter `parent_id: Option<Snowflake>` et
  `permission_overwrites: Vec<Overwrite>` (tous deux `#[serde(default)]`).
- `Overwrite { id: Snowflake, kind: u8 (#[serde(rename = "type")], 0=rôle 1=membre),
  allow: String, deny: String }` (bitfields en chaîne, format Discord v10).
- `Role { id: Snowflake, permissions: String, position: i64 }`.

Tous tolérants (`#[serde(default)]` sur l'optionnel), `Deserialize`.

## 3. Permissions — module pur (`veloce-discord/src/perms.rs`)

Constantes : `ADMINISTRATOR = 1 << 3`, `VIEW_CHANNEL = 1 << 10`.

Algorithme officiel Discord, **pur et testable** (entrées = données déjà
récupérées) :

- `base_permissions(everyone_perms: u64, member_role_perms: &[u64], is_owner: bool) -> u64`
  - `is_owner` → tous les bits (`u64::MAX`).
  - sinon `perms = everyone_perms | OR(member_role_perms)` ; si `ADMINISTRATOR`
    → tous les bits.
- `channel_permissions(base, overwrites, everyone_id, member_role_ids, me_id) -> u64`
  - si `base` a `ADMINISTRATOR` → renvoyer `base` (bypass).
  - appliquer l'overwrite `@everyone` (id == `everyone_id`, kind 0) :
    `perms = (perms & !deny) | allow`.
  - agréger les overwrites des rôles du membre (kind 0, id ∈ member_role_ids) :
    `allow |= o.allow; deny |= o.deny` puis `perms = (perms & !deny) | allow`.
  - appliquer l'overwrite membre (kind 1, id == me_id) :
    `perms = (perms & !deny) | allow`.
- `can_view_channel(...) -> bool` : `VIEW_CHANNEL` présent dans le résultat.
- `everyone_id` = l'id du rôle `@everyone` = l'id de la guilde.

Les bitfields string → `u64` via `str::parse().unwrap_or(0)` (tolérant).

L'héritage de catégorie est assuré par Discord (synchro des overwrites dans le
salon) → on calcule sur les `permission_overwrites` du salon, conforme à la doc.

## 4. REST (`veloce-discord/src/rest.rs`)

Nouvelles méthodes :
- `guild(&self, guild_id) -> Result<GuildDetail>` via `GET /guilds/{id}` ;
  `GuildDetail { owner_id: Snowflake, roles: Vec<Role> }`.
- `current_member(&self, guild_id) -> Result<MemberRoles>` via
  `GET /users/@me/guilds/{id}/member` ; `MemberRoles { roles: Vec<Snowflake> }`.

(`guild_channels` existant renvoie déjà les `permission_overwrites` ; le modèle
`Channel` enrichi les capture.)

## 5. Flux de données (`veloce-app/src/net.rs`)

`Command::SelectGuild(guild_id)` (inchangé côté UI) déclenche, en parallèle,
`guild_channels` + `guild` + `current_member`. Quand les trois réussissent, on
émet un nouvel event :

`Event::GuildChannels { guild_id, channels: Vec<Channel>, roles: Vec<Role>,
owner_id: Snowflake, member_roles: Vec<Snowflake>, me_id: Snowflake }`

(`me_id` vient du `User` courant déjà connu côté app/net.) En cas d'échec d'un
des appels → `Event::Error` (filet `last_error`), pas de crash.

L'ancien `Event::ChannelsLoaded` est **remplacé** par `GuildChannels` (un seul
consommateur, l'app).

## 6. Arbre des salons — pur (`veloce-app/src/app.rs` ou `channels.rs`)

`build_channel_tree(channels: &[Channel], visible: &HashSet<Snowflake>) -> Vec<TreeRow>`
où `TreeRow` = `Category { id, name }` ou `Channel(Channel)`.

- ne garder que les salons dont l'id ∈ `visible` (catégorie visible si ≥1 enfant
  visible) ;
- ordre : catégories (kind 4) et salons sans `parent_id` au niveau racine, triés
  par `position` ; sous chaque catégorie, ses enfants triés par
  **(groupe de type, position)** où groupe = 0 pour texte/annonce/forum
  (0,5,15) et 1 pour vocal/stage (2,13) ;
- salons racine sans catégorie listés avant les catégories (comme Discord).

## 7. UI (`veloce-app/src/app.rs`)

Panneau Salons : rendre les `TreeRow` — catégories en en-tête (gras, repliable
via `CollapsingHeader`), salons imbriqués avec **icône par type** : `#` (texte
0), `🔊` (vocal 2), `📢` (annonce 5), `🎙` (stage 13), `💬` (forum 15). Seuls
les salons visibles. Clic sur un salon texte/annonce → `FetchHistory` (inchangé) ;
clic sur vocal/stage → no-op. Plus de bannière 403 (l'inaccessible n'est pas
listé ; `last_error` conservé pour les vraies erreurs).

L'état `ChatState` stocke désormais `roles`/`owner_id`/`member_roles`/`me_id`
nécessaires au calcul, et les `channels` complets (pas seulement texte).

## 8. Tests

- **`perms`** (cœur) : owner → tout ; ADMINISTRATOR → tout ; `@everyone` deny
  VIEW → non visible ; rôle allow VIEW (sur deny @everyone) → visible ;
  overwrite membre allow prioritaire ; agrégat de plusieurs rôles.
- **`build_channel_tree`** : groupement par catégorie, ordre (type puis
  position), filtrage par `visible`, salons racine avant catégories, catégorie
  sans enfant visible masquée.
- Modèles : désérialisation d'un `Channel` avec `parent_id` + overwrites, d'un
  `Role`.

## 9. Critères de succès

1. Les salons s'affichent **groupés par catégorie**, dans l'ordre Discord, avec
   icônes par type. (manuel)
2. Les salons que le compte ne peut pas voir **n'apparaissent pas** ; plus de
   403 en usage normal. (manuel)
3. `perms` et `build_channel_tree` couverts par tests unitaires (verts).
4. Aucune régression : sélection serveur → salons → messages → envoi ; plugins ;
   build/clippy/fmt propres.

## 10. Hors périmètre

Rejoindre la voix, contenu forums/threads, salons annonces suivis, édition de
permissions. **Emojis couleur** = sous-projet suivant (SP2).

## 11. Risques

| Risque | Mitigation |
|---|---|
| Endpoints user (`GET /guilds/{id}`, `/users/@me/guilds/{id}/member`) refusés | Vérif live ; échec → `Error` (filet), pas de crash ; ajustement endpoint si besoin |
| Algorithme de permissions subtil | Module **pur** couvert par tests exhaustifs (cas owner/admin/deny/allow/membre) |
| Catégorie non synchronisée (héritage) | On suit l'algo officiel (overwrites du salon) ; cas limites rares documentés |
| 3 appels REST par sélection de serveur | Acceptable (au clic, pas en boucle) ; en parallèle |
