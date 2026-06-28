# Veloce — Plan d'implémentation : salons fidèles (arborescence + permissions)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Afficher les salons comme Discord — groupés par catégories, ordonnés correctement, avec icônes par type, et en masquant les salons non visibles via le calcul des permissions (VIEW_CHANNEL). Règle l'ordre des salons et l'erreur 403 « Accès manquant ».

**Architecture:** La logique lourde (permissions, arbre) vit en **fonctions pures** dans `veloce-discord` (testées sans réseau ni egui) : modèles enrichis, module `perms`, module `channel_tree`, méthodes REST. L'app récupère channels+rôles+owner+membre au clic serveur, calcule l'ensemble visible et construit l'arbre via ces fonctions pures, puis le rend.

**Tech Stack:** Rust 2021, serde, tokio, reqwest, eframe/egui.

## Global Constraints

- Algorithme de permissions **officiel Discord** : base = `@everyone` (rôle id == guild_id) OR rôles du membre ; bypass owner / `ADMINISTRATOR` (1<<3) ; overwrites du salon dans l'ordre @everyone → agrégat rôles → membre (`perms = (perms & !deny) | allow`) ; visible = bit `VIEW_CHANNEL` (1<<10).
- Bitfields de permissions = **chaînes** Discord v10 → `u64` via `parse().unwrap_or(0)`.
- Modèles **tolérants** (`#[serde(default)]` sur l'optionnel) ; `veloce-discord` ne dépend jamais d'egui.
- Ordre d'un niveau : salons racine (sans `parent_id`) avant les catégories ; dans une catégorie, tri par **(groupe de type, position)** où groupe = 1 pour vocal/stage (kinds 2,13) et 0 sinon ; catégories par `position`.
- Tasks 1-4 sont dans `veloce-discord` et n'ajoutent que des **éléments publics de lib** (+ MAJ d'un helper de test app en Task 1) → le workspace reste **vert** après chacune. Task 5 fait le changement couplé event+app.
- Édition 2021. Commits en français, style `type: description`.

---

## Structure des fichiers

```
crates/veloce-discord/src/
├─ models.rs        # Task 1 : Channel.parent_id/permission_overwrites, Overwrite, Role
├─ perms.rs         # Task 2 : base_permissions, channel_permissions, can_view_channel, visible_channel_ids
├─ channel_tree.rs  # Task 3 : TreeRow, build_channel_tree
├─ rest.rs          # Task 4 : guild(), current_member(), GuildDetail, MemberRoles
├─ events.rs        # Task 5 : GuildChannels remplace ChannelsLoaded
└─ lib.rs           # exports (Tasks 1-5)
crates/veloce-app/src/
├─ net.rs           # Task 5 : fetch parallèle + émet GuildChannels ; stocke me_id
└─ app.rs           # Task 1 (helper test) ; Task 5 : calcul visible + arbre + rendu UI
```

---

### Task 1 : Modèles (Channel enrichi, Overwrite, Role)

**Files:**
- Modify: `crates/veloce-discord/src/models.rs`, `crates/veloce-discord/src/lib.rs`
- Modify: `crates/veloce-app/src/app.rs` (le helper de test `ch()` — pour garder le workspace vert)

**Interfaces:**
- Produces : `Channel` gagne `parent_id: Option<Snowflake>`, `permission_overwrites: Vec<Overwrite>` ; `Overwrite { id: Snowflake, kind: u8 (#[serde(rename="type")]), allow: String, deny: String }` ; `Role { id: Snowflake, permissions: String, position: i64 }`. Tous `#[derive(Debug, Clone, Deserialize)]`.

- [ ] **Step 1 : Écrire les tests (échouent)** — ajouter dans le `mod tests` de `models.rs` :

```rust
    #[test]
    fn deserialise_channel_avec_parent_et_overwrites() {
        let json = r#"{ "id":"5","type":0,"name":"général","guild_id":"10","position":2,
            "parent_id":"99",
            "permission_overwrites":[ {"id":"10","type":0,"allow":"0","deny":"1024"} ] }"#;
        let c: Channel = serde_json::from_str(json).unwrap();
        assert_eq!(c.parent_id.as_deref(), Some("99"));
        assert_eq!(c.permission_overwrites.len(), 1);
        let o = &c.permission_overwrites[0];
        assert_eq!(o.id, "10");
        assert_eq!(o.kind, 0);
        assert_eq!(o.deny, "1024");
    }

    #[test]
    fn channel_sans_parent_ni_overwrites() {
        // rétro-compat : un salon sans ces champs reste valide.
        let json = r#"{ "id":"1","type":0,"name":"x" }"#;
        let c: Channel = serde_json::from_str(json).unwrap();
        assert!(c.parent_id.is_none());
        assert!(c.permission_overwrites.is_empty());
    }

    #[test]
    fn deserialise_role() {
        let json = r#"{ "id":"10","name":"@everyone","permissions":"1024","position":0 }"#;
        let r: Role = serde_json::from_str(json).unwrap();
        assert_eq!(r.id, "10");
        assert_eq!(r.permissions, "1024");
    }
```

- [ ] **Step 2 : Vérifier l'échec**

Run: `cargo test -p veloce-discord models`
Expected: FAIL (champs/`Role` inexistants).

- [ ] **Step 3 : Modifier `Channel` + ajouter `Overwrite` et `Role`** dans `models.rs`

Dans `struct Channel`, ajouter (après `position`) :

```rust
    #[serde(default)]
    pub parent_id: Option<Snowflake>,
    #[serde(default)]
    pub permission_overwrites: Vec<Overwrite>,
```

Et ajouter les structs :

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct Overwrite {
    pub id: Snowflake,
    #[serde(rename = "type")]
    pub kind: u8, // 0 = rôle, 1 = membre
    #[serde(default)]
    pub allow: String,
    #[serde(default)]
    pub deny: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Role {
    pub id: Snowflake,
    #[serde(default)]
    pub permissions: String,
    #[serde(default)]
    pub position: i64,
}
```

- [ ] **Step 4 : Exporter** — dans `lib.rs`, étendre l'export models :

```rust
pub use models::{Channel, GatewayPayload, Guild, Message, Overwrite, Role, Snowflake, User};
```

- [ ] **Step 5 : Mettre à jour le helper de test de l'app** — dans `crates/veloce-app/src/app.rs`, le helper `ch()` du `mod tests` construit un `Channel` littéral ; ajouter les deux nouveaux champs pour que ça compile :

```rust
    fn ch(id: &str, kind: u8, pos: i32) -> Channel {
        Channel {
            id: id.into(),
            name: Some(id.into()),
            kind,
            guild_id: None,
            position: Some(pos),
            parent_id: None,
            permission_overwrites: Vec::new(),
        }
    }
```

- [ ] **Step 6 : Vérifier le succès (workspace entier)**

Run:
```bash
cargo test -p veloce-discord models
cargo test --all
cargo clippy --all-targets -- -D warnings
cargo fmt --all
```
Expected: tests PASS, workspace clippy-clean (les nouveaux champs sont consommés en Task 5 ; comme ce sont des champs `pub` de lib, pas de `dead_code`).

- [ ] **Step 7 : Commit**

```bash
git add -A
git commit -m "feat(discord): Channel.parent_id/permission_overwrites + Overwrite + Role"
```

---

### Task 2 : Module permissions (pur)

**Files:**
- Create: `crates/veloce-discord/src/perms.rs`
- Modify: `crates/veloce-discord/src/lib.rs`

**Interfaces:**
- Consumes: `crate::models::{Channel, Overwrite, Role, Snowflake}`.
- Produces:
  - `pub const ADMINISTRATOR: u64`, `pub const VIEW_CHANNEL: u64`.
  - `pub fn base_permissions(everyone_perms: u64, member_role_perms: &[u64], is_owner: bool) -> u64`
  - `pub fn channel_permissions(base: u64, overwrites: &[Overwrite], everyone_id: &str, member_role_ids: &[Snowflake], me_id: &str) -> u64`
  - `pub fn can_view_channel(base, overwrites, everyone_id, member_role_ids, me_id) -> bool`
  - `pub fn visible_channel_ids(channels: &[Channel], roles: &[Role], owner_id: &str, member_roles: &[Snowflake], me_id: &str, guild_id: &str) -> std::collections::HashSet<Snowflake>`

- [ ] **Step 1 : Écrire les tests (échouent)** — `perms.rs` :

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Channel, Overwrite, Role};

    fn ow(id: &str, kind: u8, allow: &str, deny: &str) -> Overwrite {
        Overwrite { id: id.into(), kind, allow: allow.into(), deny: deny.into() }
    }
    fn chan(id: &str, parent: Option<&str>, kind: u8, ows: Vec<Overwrite>) -> Channel {
        Channel { id: id.into(), name: Some(id.into()), kind, guild_id: Some("10".into()),
            position: Some(0), parent_id: parent.map(|s| s.into()), permission_overwrites: ows }
    }
    const V: u64 = VIEW_CHANNEL;

    #[test]
    fn owner_a_tout() {
        assert_eq!(base_permissions(0, &[], true), u64::MAX);
    }

    #[test]
    fn admin_a_tout() {
        let b = base_permissions(ADMINISTRATOR, &[], false);
        assert_eq!(b, u64::MAX);
    }

    #[test]
    fn everyone_deny_view_cache() {
        // base a VIEW, mais @everyone overwrite deny VIEW.
        let b = V;
        let ows = vec![ow("10", 0, "0", &V.to_string())]; // everyone (id==guild) deny VIEW
        assert!(!can_view_channel(b, &ows, "10", &[], "me"));
    }

    #[test]
    fn role_allow_view_par_dessus_everyone_deny() {
        let b = V;
        let ows = vec![
            ow("10", 0, "0", &V.to_string()),          // @everyone deny VIEW
            ow("42", 0, &V.to_string(), "0"),          // rôle 42 allow VIEW
        ];
        assert!(can_view_channel(b, &ows, "10", &["42".to_string()], "me"));
    }

    #[test]
    fn member_overwrite_prioritaire() {
        let b = V;
        let ows = vec![
            ow("42", 0, "0", &V.to_string()),          // rôle 42 deny VIEW
            ow("me", 1, &V.to_string(), "0"),          // membre allow VIEW
        ];
        assert!(can_view_channel(b, &ows, "10", &["42".to_string()], "me"));
    }

    #[test]
    fn visible_channel_ids_filtre() {
        let everyone = Role { id: "10".into(), permissions: V.to_string(), position: 0 };
        let chans = vec![
            chan("a", None, 0, vec![]),                                   // visible (hérite base VIEW)
            chan("b", None, 0, vec![ow("10", 0, "0", &V.to_string())]),   // caché (everyone deny)
        ];
        let vis = visible_channel_ids(&chans, &[everyone], "owner", &[], "me", "10");
        assert!(vis.contains("a"));
        assert!(!vis.contains("b"));
    }
}
```

- [ ] **Step 2 : Vérifier l'échec**

Run: `cargo test -p veloce-discord perms`
Expected: FAIL (module/fonctions inexistants).

- [ ] **Step 3 : Implémenter `perms.rs`** (au-dessus du `mod tests`)

```rust
use crate::models::{Channel, Overwrite, Role, Snowflake};
use std::collections::HashSet;

pub const ADMINISTRATOR: u64 = 1 << 3;
pub const VIEW_CHANNEL: u64 = 1 << 10;

fn parse(s: &str) -> u64 {
    s.parse::<u64>().unwrap_or(0)
}

/// Permissions de base : @everyone OR rôles du membre ; owner/ADMIN → tous les bits.
pub fn base_permissions(everyone_perms: u64, member_role_perms: &[u64], is_owner: bool) -> u64 {
    if is_owner {
        return u64::MAX;
    }
    let mut perms = everyone_perms;
    for r in member_role_perms {
        perms |= r;
    }
    if perms & ADMINISTRATOR != 0 {
        return u64::MAX;
    }
    perms
}

/// Applique les overwrites d'un salon : @everyone → agrégat rôles → membre.
pub fn channel_permissions(
    base: u64,
    overwrites: &[Overwrite],
    everyone_id: &str,
    member_role_ids: &[Snowflake],
    me_id: &str,
) -> u64 {
    if base & ADMINISTRATOR != 0 {
        return base;
    }
    let mut perms = base;
    // @everyone
    if let Some(o) = overwrites.iter().find(|o| o.kind == 0 && o.id == everyone_id) {
        perms = (perms & !parse(&o.deny)) | parse(&o.allow);
    }
    // agrégat des rôles du membre
    let mut allow = 0u64;
    let mut deny = 0u64;
    for o in overwrites
        .iter()
        .filter(|o| o.kind == 0 && member_role_ids.iter().any(|r| r == &o.id))
    {
        allow |= parse(&o.allow);
        deny |= parse(&o.deny);
    }
    perms = (perms & !deny) | allow;
    // membre
    if let Some(o) = overwrites.iter().find(|o| o.kind == 1 && o.id == me_id) {
        perms = (perms & !parse(&o.deny)) | parse(&o.allow);
    }
    perms
}

pub fn can_view_channel(
    base: u64,
    overwrites: &[Overwrite],
    everyone_id: &str,
    member_role_ids: &[Snowflake],
    me_id: &str,
) -> bool {
    channel_permissions(base, overwrites, everyone_id, member_role_ids, me_id) & VIEW_CHANNEL != 0
}

/// Ensemble des ids de salons visibles par le membre.
pub fn visible_channel_ids(
    channels: &[Channel],
    roles: &[Role],
    owner_id: &str,
    member_roles: &[Snowflake],
    me_id: &str,
    guild_id: &str,
) -> HashSet<Snowflake> {
    let everyone_perms = roles
        .iter()
        .find(|r| r.id == guild_id)
        .map(|r| parse(&r.permissions))
        .unwrap_or(0);
    let member_role_perms: Vec<u64> = roles
        .iter()
        .filter(|r| member_roles.iter().any(|id| id == &r.id))
        .map(|r| parse(&r.permissions))
        .collect();
    let base = base_permissions(everyone_perms, &member_role_perms, me_id == owner_id);

    channels
        .iter()
        .filter(|c| can_view_channel(base, &c.permission_overwrites, guild_id, member_roles, me_id))
        .map(|c| c.id.clone())
        .collect()
}
```

- [ ] **Step 4 : Exporter** — dans `lib.rs` :

```rust
pub mod perms;
pub use perms::{can_view_channel, visible_channel_ids};
```

- [ ] **Step 5 : Vérifier**

Run: `cargo test -p veloce-discord perms && cargo clippy -p veloce-discord --all-targets -- -D warnings`
Expected: 6 tests PASS, clippy clean.

- [ ] **Step 6 : Commit**

```bash
git add -A
git commit -m "feat(discord): module permissions pur (VIEW_CHANNEL) + tests"
```

---

### Task 3 : Arbre des salons (pur)

**Files:**
- Create: `crates/veloce-discord/src/channel_tree.rs`
- Modify: `crates/veloce-discord/src/lib.rs`

**Interfaces:**
- Consumes: `crate::models::{Channel, Snowflake}`.
- Produces: `pub enum TreeRow { Category { id: Snowflake, name: String }, Channel(Channel) }` (derive Debug, Clone) ; `pub fn build_channel_tree(channels: &[Channel], visible: &std::collections::HashSet<Snowflake>) -> Vec<TreeRow>`.

- [ ] **Step 1 : Écrire les tests (échouent)** — `channel_tree.rs` :

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Channel;
    use std::collections::HashSet;

    fn c(id: &str, kind: u8, parent: Option<&str>, pos: i32) -> Channel {
        Channel { id: id.into(), name: Some(id.into()), kind, guild_id: Some("10".into()),
            position: Some(pos), parent_id: parent.map(|s| s.into()), permission_overwrites: vec![] }
    }

    #[test]
    fn groupe_par_categorie_et_ordonne() {
        let chans = vec![
            c("cat1", 4, None, 1),
            c("txt", 0, Some("cat1"), 1),
            c("voc", 2, Some("cat1"), 0),   // vocal -> après le texte malgré position plus basse
            c("root", 0, None, 0),          // salon racine sans catégorie
        ];
        let visible: HashSet<_> = ["cat1", "txt", "voc", "root"].iter().map(|s| s.to_string()).collect();
        let rows = build_channel_tree(&chans, &visible);
        // racine d'abord, puis catégorie, puis ses enfants (texte avant vocal)
        match &rows[0] { TreeRow::Channel(ch) => assert_eq!(ch.id, "root"), _ => panic!() }
        match &rows[1] { TreeRow::Category { id, .. } => assert_eq!(id, "cat1"), _ => panic!() }
        match &rows[2] { TreeRow::Channel(ch) => assert_eq!(ch.id, "txt"), _ => panic!() }
        match &rows[3] { TreeRow::Channel(ch) => assert_eq!(ch.id, "voc"), _ => panic!() }
    }

    #[test]
    fn filtre_invisibles_et_categorie_vide_masquee() {
        let chans = vec![
            c("cat1", 4, None, 0),
            c("secret", 0, Some("cat1"), 0), // pas dans visible
        ];
        let visible: HashSet<_> = HashSet::new(); // rien de visible
        let rows = build_channel_tree(&chans, &visible);
        assert!(rows.is_empty()); // catégorie sans enfant visible -> masquée
    }
}
```

- [ ] **Step 2 : Vérifier l'échec**

Run: `cargo test -p veloce-discord channel_tree`
Expected: FAIL.

- [ ] **Step 3 : Implémenter `channel_tree.rs`**

```rust
use crate::models::{Channel, Snowflake};
use std::collections::HashSet;

const CATEGORY: u8 = 4;

#[derive(Debug, Clone)]
pub enum TreeRow {
    Category { id: Snowflake, name: String },
    Channel(Channel),
}

/// 1 pour vocal/stage (affichés après), 0 sinon.
fn type_group(kind: u8) -> u8 {
    match kind {
        2 | 13 => 1,
        _ => 0,
    }
}

fn sort_key(c: &Channel) -> (u8, i32, String) {
    (type_group(c.kind), c.position.unwrap_or(0), c.id.clone())
}

/// Construit l'arbre fidèle Discord : salons racine, puis catégories + enfants.
pub fn build_channel_tree(channels: &[Channel], visible: &HashSet<Snowflake>) -> Vec<TreeRow> {
    let is_visible = |c: &Channel| visible.contains(&c.id);
    let mut rows = Vec::new();

    // Salons racine (pas de parent, pas une catégorie), visibles.
    let mut roots: Vec<&Channel> = channels
        .iter()
        .filter(|c| c.kind != CATEGORY && c.parent_id.is_none() && is_visible(c))
        .collect();
    roots.sort_by_key(|c| sort_key(c));
    for c in roots {
        rows.push(TreeRow::Channel(c.clone()));
    }

    // Catégories triées par position ; incluses seulement si ≥1 enfant visible.
    let mut cats: Vec<&Channel> = channels.iter().filter(|c| c.kind == CATEGORY).collect();
    cats.sort_by_key(|c| (c.position.unwrap_or(0), c.id.clone()));
    for cat in cats {
        let mut children: Vec<&Channel> = channels
            .iter()
            .filter(|c| {
                c.kind != CATEGORY && c.parent_id.as_deref() == Some(cat.id.as_str()) && is_visible(c)
            })
            .collect();
        if children.is_empty() {
            continue;
        }
        children.sort_by_key(|c| sort_key(c));
        rows.push(TreeRow::Category {
            id: cat.id.clone(),
            name: cat.name.clone().unwrap_or_default(),
        });
        for c in children {
            rows.push(TreeRow::Channel(c.clone()));
        }
    }

    rows
}
```

- [ ] **Step 4 : Exporter** — dans `lib.rs` :

```rust
pub mod channel_tree;
pub use channel_tree::{build_channel_tree, TreeRow};
```

- [ ] **Step 5 : Vérifier**

Run: `cargo test -p veloce-discord channel_tree && cargo clippy -p veloce-discord --all-targets -- -D warnings`
Expected: 2 tests PASS, clippy clean.

- [ ] **Step 6 : Commit**

```bash
git add -A
git commit -m "feat(discord): arbre des salons pur (catégories + ordre Discord) + tests"
```

---

### Task 4 : REST — guild() et current_member()

**Files:**
- Modify: `crates/veloce-discord/src/rest.rs`, `crates/veloce-discord/src/lib.rs`

**Interfaces:**
- Consumes: `crate::models::Role`, `RestClient`.
- Produces: `pub struct GuildDetail { owner_id: Snowflake, roles: Vec<Role> }` ; `pub struct MemberRoles { roles: Vec<Snowflake> }` ; `RestClient::guild(&self, &str) -> Result<GuildDetail>` ; `RestClient::current_member(&self, &str) -> Result<MemberRoles>`.

**Note :** appels live (token requis) → non testés unitairement ; gate = build + clippy.

- [ ] **Step 1 : Implémenter** dans `rest.rs` — ajouter les types (en haut, après les `use`) :

```rust
use crate::models::{Role, Snowflake};

#[derive(Debug, Clone, serde::Deserialize)]
pub struct GuildDetail {
    pub owner_id: Snowflake,
    #[serde(default)]
    pub roles: Vec<Role>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct MemberRoles {
    #[serde(default)]
    pub roles: Vec<Snowflake>,
}
```

*(Si `Snowflake`/`Role`/`Message`/etc. sont déjà importés dans `rest.rs`, fusionner l'import au lieu de le dupliquer.)*

Et dans `impl RestClient`, ajouter :

```rust
    pub async fn guild(&self, guild_id: &str) -> Result<GuildDetail> {
        self.get_json(format!("{API_BASE}/guilds/{guild_id}")).await
    }

    pub async fn current_member(&self, guild_id: &str) -> Result<MemberRoles> {
        self.get_json(format!("{API_BASE}/users/@me/guilds/{guild_id}/member"))
            .await
    }
```

- [ ] **Step 2 : Exporter** — dans `lib.rs` :

```rust
pub use rest::{GuildDetail, MemberRoles, RestClient};
```

- [ ] **Step 3 : Vérifier**

Run: `cargo build -p veloce-discord && cargo clippy -p veloce-discord --all-targets -- -D warnings`
Expected: compile, clippy clean.

- [ ] **Step 4 : Commit**

```bash
git add -A
git commit -m "feat(discord): REST guild() + current_member() (owner, rôles, rôles membre)"
```

---

### Task 5 : Intégration — event GuildChannels, fetch parallèle, calcul visibilité, rendu arbre

**Files:**
- Modify: `crates/veloce-discord/src/events.rs` (+ `lib.rs` si besoin)
- Modify: `crates/veloce-app/src/net.rs`
- Modify: `crates/veloce-app/src/app.rs`

**Interfaces:**
- Consumes: `veloce_discord::{visible_channel_ids, build_channel_tree, TreeRow, GuildDetail, MemberRoles, Channel, Role}`.
- Produces: `Event::GuildChannels { guild_id, channels: Vec<Channel>, roles: Vec<Role>, owner_id: Snowflake, member_roles: Vec<Snowflake>, me_id: Snowflake }` (remplace `ChannelsLoaded`).

**Note :** tâche d'intégration (event + réseau + UI). Le seul test unitaire neuf est facultatif ; gate = `cargo test --all`, `cargo build`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --all` — tous propres. Le code egui peut nécessiter de petits ajustements d'API (préserver le comportement). Vérif réelle (ordre + masquage) = **manuelle** (compte requis).

- [ ] **Step 1 : Remplacer l'event** — dans `crates/veloce-discord/src/events.rs`, supprimer la variante `ChannelsLoaded { guild_id, channels }` et ajouter :

```rust
    GuildChannels {
        guild_id: Snowflake,
        channels: Vec<Channel>,
        roles: Vec<Role>,
        owner_id: Snowflake,
        member_roles: Vec<Snowflake>,
        me_id: Snowflake,
    },
```

Ajouter `Role` à l'import en tête de `events.rs` :

```rust
use crate::models::{Channel, Guild, Message, Role, Snowflake, User};
```

- [ ] **Step 2 : Réseau — fetch parallèle + me_id** — dans `crates/veloce-app/src/net.rs` :

Importer ce qu'il faut : `use veloce_discord::{run_gateway, Command, Event, GatewayCommand, RestClient};` (déjà le cas) — pas de nouveau type requis ici hormis l'usage via `rest`.

Capturer `me_id` à la validation du token (la valeur de `current_user()` est aujourd'hui ignorée) :

```rust
            let me = match rest.current_user().await {
                Ok(u) => u,
                Err(e) => {
                    let _ = event_out
                        .send(Event::AuthFailed(format!("Authentification échouée : {e}")));
                    ctx.request_repaint();
                    return;
                }
            };
            let me_id = me.id.clone();
```

(Supprimer l'ancien bloc `if let Err(e) = rest.current_user().await { ... }`.)

Passer `me_id` à `handle_command` :

```rust
                    Some(cmd) = cmd_rx.recv() => {
                        handle_command(&rest, cmd, &event_out, &ctx, &me_id).await;
                    }
```

Réécrire la signature et l'arm `SelectGuild` de `handle_command` :

```rust
async fn handle_command(
    rest: &RestClient,
    cmd: Command,
    out: &Sender<Event>,
    ctx: &Context,
    me_id: &str,
) {
    let result: Result<Event, String> = match cmd {
        Command::SelectGuild(guild_id) => {
            let (channels, detail, member) = tokio::join!(
                rest.guild_channels(&guild_id),
                rest.guild(&guild_id),
                rest.current_member(&guild_id),
            );
            match (channels, detail, member) {
                (Ok(channels), Ok(detail), Ok(member)) => Ok(Event::GuildChannels {
                    guild_id,
                    channels,
                    roles: detail.roles,
                    owner_id: detail.owner_id,
                    member_roles: member.roles,
                    me_id: me_id.to_string(),
                }),
                _ => Err("Impossible de charger les salons du serveur".to_string()),
            }
        }
        Command::FetchHistory(channel_id) => rest
            .channel_messages(&channel_id, 50)
            .await
            .map(|mut messages| {
                messages.reverse();
                Event::MessagesLoaded { channel_id, messages }
            })
            .map_err(|e| e.to_string()),
        Command::SendMessage { channel_id, content } => rest
            .send_message(&channel_id, &content)
            .await
            .map(Event::MessageCreated)
            .map_err(|e| e.to_string()),
    };
    let ev = result.unwrap_or_else(Event::Error);
    let _ = out.send(ev);
    ctx.request_repaint();
}
```

- [ ] **Step 3 : App — état, calcul, rendu** — dans `crates/veloce-app/src/app.rs` :

Imports : ajouter `use veloce_discord::{build_channel_tree, visible_channel_ids, TreeRow};` aux imports `veloce_discord::{...}` existants.

`ChatState` : remplacer `channels: Vec<Channel>` par les deux champs (garder `channels` pour retrouver le nom du salon sélectionné, + l'arbre) :

```rust
    channels: Vec<Channel>,
    channel_tree: Vec<TreeRow>,
```

Supprimer `text_channels_sorted` **et son test** `ne_garde_que_les_salons_texte_tries_par_position` (remplacés par `build_channel_tree`, testé dans `veloce-discord`).

`apply_event` : remplacer l'arm `ChannelsLoaded` par :

```rust
        Event::GuildChannels {
            guild_id,
            channels,
            roles,
            owner_id,
            member_roles,
            me_id,
        } => {
            if Some(&guild_id) == state.selected_guild.as_ref() {
                let visible =
                    visible_channel_ids(&channels, &roles, &owner_id, &member_roles, &me_id, &guild_id);
                state.channel_tree = build_channel_tree(&channels, &visible);
                state.channels = channels;
                state.last_error = None;
            }
        }
```

Au clic serveur (`draw_chat`, panneau guilds) : remplacer `state.channels.clear();` par `state.channel_tree.clear();` (et garder `state.channels.clear();` aussi si tu veux ; au minimum vider l'arbre). Le `net.subscribe_guild` + `net.send(Command::SelectGuild(..))` restent.

Panneau « channels » : remplacer la boucle `for c in state.channels.clone()` par le rendu de l'arbre :

```rust
    egui::SidePanel::left("channels")
        .exact_width(200.0)
        .show(ctx, |ui| {
            ui.heading("Salons");
            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| {
                for row in state.channel_tree.clone() {
                    match row {
                        TreeRow::Category { name, .. } => {
                            ui.add_space(4.0);
                            ui.label(egui::RichText::new(name.to_uppercase()).small().strong());
                        }
                        TreeRow::Channel(c) => {
                            let icon = channel_icon(c.kind);
                            let label = format!("{icon} {}", c.name.clone().unwrap_or_else(|| c.id.clone()));
                            let selectable = matches!(c.kind, 0 | 5 | 15); // texte/annonce/forum
                            let selected = state.selected_channel.as_ref() == Some(&c.id);
                            let resp = ui.add_enabled(selectable, egui::SelectableLabel::new(selected, label));
                            if resp.clicked() {
                                state.selected_channel = Some(c.id.clone());
                                state.messages.clear();
                                net.send(Command::FetchHistory(c.id.clone()));
                            }
                        }
                    }
                }
            });
        });
```

Ajouter le helper d'icône (près de `text_channels_sorted` supprimé) :

```rust
fn channel_icon(kind: u8) -> &'static str {
    match kind {
        2 => "🔊",  // vocal
        5 => "📢",  // annonce
        13 => "🎙", // stage
        15 => "💬", // forum
        _ => "#",   // texte et autres
    }
}
```

- [ ] **Step 4 : Gates complets**

Run:
```bash
cargo test --all
cargo build
cargo clippy --all-targets -- -D warnings
cargo fmt --all
```
Expected: tests PASS (perms + channel_tree + modèles + existants ; `text_channels_sorted` retiré), build OK, **clippy 0 warning sur tout le workspace**, fmt propre.

- [ ] **Step 5 : Vérification manuelle (utilisateur)**

Run: `cargo run --bin veloce`
Expected : salons **groupés par catégories**, dans l'ordre Discord, avec icônes par type ; les salons sans accès **n'apparaissent plus** ; plus d'erreur 403 en navigation normale.

- [ ] **Step 6 : Commit**

```bash
git add -A
git commit -m "feat(app): salons fidèles — arborescence, icônes, masquage par permissions"
```

---

## Self-Review (effectuée)

**1. Couverture de la spec :**
- Channel.parent_id/overwrites + Overwrite + Role → Task 1. ✅
- Permissions (base/channel/can_view/visible_channel_ids, ADMIN/owner/VIEW) → Task 2 + tests. ✅
- REST guild()/current_member() → Task 4. ✅
- Flux GuildChannels (remplace ChannelsLoaded) + fetch parallèle → Task 5 (events + net). ✅
- Arbre (catégories, ordre type+position, racine avant catégories, filtrage) → Task 3 + tests. ✅
- UI (catégories, icônes par type, visibles seulement, clic vocal no-op) → Task 5. ✅
- 403 réglé (inaccessibles masqués ; last_error en filet) → Task 5. ✅
- Tests perms + arbre + modèles → Tasks 1/2/3. ✅

**2. Placeholders :** aucun. Les notes (fusion d'imports rest.rs ; ajustements egui en Task 5) sont des précisions d'intégration.

**3. Cohérence des types :** `Overwrite.kind`/`Role.permissions` (String) cohérents Tasks 1/2 ; `visible_channel_ids`/`build_channel_tree` signatures identiques Tasks 2/3/5 ; `Event::GuildChannels` champs identiques events↔net↔app (Task 5) ; `me_id: Snowflake` (=String) cohérent ; `channel_icon` kinds alignés avec `type_group`.
