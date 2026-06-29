# Rendu fidèle Discord — Phase 1 (modèle + layout + images) — Plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Capturer les champs riches d'un message Discord (avatar, pièces jointes, embeds, mentions, réponse) dans le modèle, et afficher les messages façon Discord — gouttière avatar, en-tête `nom · heure`, regroupement des messages consécutifs, et images inline.

**Architecture:** Le crate `veloce-discord` reçoit déjà les messages via serde (REST + gateway `from_value::<Message>`). On étend les structs avec des champs `#[serde(default)]` (rétro-compatibles, inertes tant que non rendus). Côté `veloce-app`, trois modules purs et testables (`cdn`, `timestamp`, `grouping`) alimentent une refonte de la boucle de rendu dans `app.rs`.

**Tech Stack:** Rust 2021, serde, eframe/egui 0.30, egui_extras (loader d'images URL déjà installé).

## Global Constraints

- Rust édition 2021, rust-version 1.75 (workspace).
- Nouveaux champs de désérialisation TOUJOURS `#[serde(default)]` (rétro-compat REST + gateway).
- Aucune nouvelle dépendance externe (parsing de date fait à la main).
- Les modules de logique pure (`cdn`, `timestamp`, `grouping`) sont testés unitairement, sans egui.
- `cargo build`, `cargo clippy`, `cargo fmt --check`, `cargo test` doivent rester verts.
- Messages de commit SANS trailer `Co-Authored-By` ni `Claude-Session`.

---

### Task 1: Modèle de données enrichi (`veloce-discord`)

**Files:**
- Modify: `crates/veloce-discord/src/models.rs`
- Modify: `crates/veloce-discord/src/lib.rs`
- Create: `tests/fixtures/message_rich.json`
- Test: `crates/veloce-discord/src/models.rs` (module `tests`)

**Interfaces:**
- Produces: `User.avatar: Option<String>` ; `Role.name: String`, `Role.color: u32` ;
  `Message.attachments: Vec<Attachment>`, `Message.embeds: Vec<Embed>`,
  `Message.mentions: Vec<User>`, `Message.mention_roles: Vec<Snowflake>`,
  `Message.referenced_message: Option<Box<Message>>`, `Message.edited_timestamp: Option<String>` ;
  `Attachment { id, filename, content_type: Option<String>, url: String, proxy_url: String, size: u64, width: Option<u32>, height: Option<u32> }` avec `fn is_image(&self) -> bool` ;
  `Embed { kind: Option<String>, title, description, url: Option<String>, color: Option<u32>, author: Option<EmbedAuthor>, fields: Vec<EmbedField>, image: Option<EmbedMedia>, thumbnail: Option<EmbedMedia>, footer: Option<EmbedFooter> }` et sous-structs.

- [ ] **Step 1: Écrire le test de désérialisation (échoue)**

Créer la fixture `tests/fixtures/message_rich.json` :

```json
{
  "id": "555",
  "channel_id": "222",
  "content": "regarde <#222> et <@1> ||secret||",
  "author": { "id": "1", "username": "alice", "global_name": "Alice", "avatar": "abc123" },
  "timestamp": "2026-06-29T14:23:45.000000+00:00",
  "edited_timestamp": null,
  "mentions": [ { "id": "1", "username": "alice", "global_name": "Alice" } ],
  "mention_roles": ["77"],
  "attachments": [
    { "id": "9", "filename": "photo.png", "content_type": "image/png",
      "url": "https://cdn.example/photo.png", "proxy_url": "https://cdn.example/photo.png",
      "size": 1024, "width": 800, "height": 600 }
  ],
  "embeds": [
    { "type": "rich", "title": "Titre", "description": "Desc", "color": 5793266,
      "fields": [ { "name": "F1", "value": "V1", "inline": true } ],
      "footer": { "text": "pied" } }
  ],
  "referenced_message": {
    "id": "100", "channel_id": "222", "content": "message parent",
    "author": { "id": "2", "username": "bob" }
  }
}
```

Ajouter dans le module `tests` de `models.rs` :

```rust
#[test]
fn deserialise_message_riche() {
    let json = include_str!("../../../tests/fixtures/message_rich.json");
    let m: Message = serde_json::from_str(json).unwrap();
    assert_eq!(m.author.avatar.as_deref(), Some("abc123"));
    assert_eq!(m.mentions.len(), 1);
    assert_eq!(m.mention_roles, vec!["77".to_string()]);
    assert_eq!(m.attachments.len(), 1);
    assert!(m.attachments[0].is_image());
    assert_eq!(m.attachments[0].width, Some(800));
    assert_eq!(m.embeds.len(), 1);
    assert_eq!(m.embeds[0].title.as_deref(), Some("Titre"));
    assert_eq!(m.embeds[0].color, Some(5793266));
    assert_eq!(m.embeds[0].fields[0].inline, true);
    assert_eq!(m.referenced_message.as_ref().unwrap().content, "message parent");
}

#[test]
fn is_image_par_extension_sans_content_type() {
    let a = Attachment {
        id: "1".into(), filename: "x.JPG".into(), content_type: None,
        url: "u".into(), proxy_url: "u".into(), size: 0, width: None, height: None,
    };
    assert!(a.is_image());
    let b = Attachment { filename: "x.zip".into(), ..a.clone() };
    assert!(!b.is_image());
}

#[test]
fn message_minimal_reste_valide() {
    // Rétro-compat : un message sans les nouveaux champs se désérialise.
    let json = r#"{ "id":"1","channel_id":"2","content":"hi",
        "author": { "id":"3","username":"u" } }"#;
    let m: Message = serde_json::from_str(json).unwrap();
    assert!(m.attachments.is_empty());
    assert!(m.embeds.is_empty());
    assert!(m.referenced_message.is_none());
}
```

- [ ] **Step 2: Lancer le test pour vérifier l'échec**

Run: `cargo test -p veloce-discord deserialise_message_riche`
Expected: FAIL — `no field 'avatar'` / `cannot find type 'Attachment'`.

- [ ] **Step 3: Implémenter les structs**

Dans `models.rs`, ajouter `avatar` à `User` :

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct User {
    pub id: Snowflake,
    pub username: String,
    #[serde(default)]
    pub global_name: Option<String>,
    #[serde(default)]
    pub discriminator: Option<String>,
    #[serde(default)]
    pub avatar: Option<String>,
}
```

Ajouter `name`/`color` à `Role` :

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct Role {
    pub id: Snowflake,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub permissions: String,
    #[serde(default)]
    pub position: i64,
    #[serde(default)]
    pub color: u32,
}
```

Étendre `Message` :

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct Message {
    pub id: Snowflake,
    pub channel_id: Snowflake,
    pub content: String,
    pub author: User,
    #[serde(default)]
    pub timestamp: Option<String>,
    #[serde(default)]
    pub edited_timestamp: Option<String>,
    #[serde(default)]
    pub mentions: Vec<User>,
    #[serde(default)]
    pub mention_roles: Vec<Snowflake>,
    #[serde(default)]
    pub attachments: Vec<Attachment>,
    #[serde(default)]
    pub embeds: Vec<Embed>,
    #[serde(default)]
    pub referenced_message: Option<Box<Message>>,
}
```

Ajouter les nouveaux types (en bas, avant `#[cfg(test)]`) :

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct Attachment {
    pub id: Snowflake,
    #[serde(default)]
    pub filename: String,
    #[serde(default)]
    pub content_type: Option<String>,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub proxy_url: String,
    #[serde(default)]
    pub size: u64,
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
}

impl Attachment {
    pub fn is_image(&self) -> bool {
        if let Some(ct) = &self.content_type {
            if ct.starts_with("image/") {
                return true;
            }
        }
        let lower = self.filename.to_ascii_lowercase();
        [".png", ".jpg", ".jpeg", ".gif", ".webp", ".bmp"]
            .iter()
            .any(|ext| lower.ends_with(ext))
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Embed {
    #[serde(rename = "type", default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub color: Option<u32>,
    #[serde(default)]
    pub author: Option<EmbedAuthor>,
    #[serde(default)]
    pub fields: Vec<EmbedField>,
    #[serde(default)]
    pub image: Option<EmbedMedia>,
    #[serde(default)]
    pub thumbnail: Option<EmbedMedia>,
    #[serde(default)]
    pub footer: Option<EmbedFooter>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EmbedAuthor {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub icon_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EmbedField {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub value: String,
    #[serde(default)]
    pub inline: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EmbedMedia {
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub proxy_url: Option<String>,
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EmbedFooter {
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub icon_url: Option<String>,
}
```

Exporter dans `lib.rs` (ligne `pub use models::{...}`) :

```rust
pub use models::{
    Attachment, Channel, Embed, EmbedAuthor, EmbedField, EmbedFooter, EmbedMedia,
    GatewayPayload, Guild, Message, Overwrite, Role, Snowflake, User,
};
```

- [ ] **Step 4: Lancer les tests pour vérifier le succès**

Run: `cargo test -p veloce-discord`
Expected: PASS (les 3 nouveaux tests + les anciens, notamment ceux qui construisent `User`/`Role` — vérifier qu'aucun littéral de struct existant ne casse ; si un test construit `User {...}` sans `avatar`, ajouter `avatar: None`, et `Role {...}` sans `name`/`color`, ajouter `name: String::new(), color: 0`).

- [ ] **Step 5: Corriger les littéraux de struct cassés**

Les tests existants de `perms.rs` / `models.rs` qui construisent `Role { id, permissions, position }` doivent recevoir `name: String::new(), color: 0`. Idem tout `User { ... }` littéral → `avatar: None`. Relancer `cargo test -p veloce-discord` jusqu'au vert.

- [ ] **Step 6: Commit**

```bash
git add crates/veloce-discord/src/models.rs crates/veloce-discord/src/lib.rs tests/fixtures/message_rich.json
git commit -m "feat(discord): modele de message enrichi (attachments, embeds, mentions, avatar)"
```

---

### Task 2: Module `cdn` — URLs d'avatar (`veloce-app`)

**Files:**
- Create: `crates/veloce-app/src/cdn.rs`
- Modify: `crates/veloce-app/src/main.rs:1-6` (ajouter `mod cdn;`)
- Test: `crates/veloce-app/src/cdn.rs` (module `tests`)

**Interfaces:**
- Consumes: `veloce_discord::User` (champs `id`, `discriminator`, `avatar`).
- Produces: `pub fn avatar_for(user: &User) -> String`.

- [ ] **Step 1: Écrire les tests (échouent)**

Créer `crates/veloce-app/src/cdn.rs` :

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn avatar_png_et_gif() {
        assert_eq!(
            avatar_url("42", "abc", 80),
            "https://cdn.discordapp.com/avatars/42/abc.png?size=80"
        );
        assert_eq!(
            avatar_url("42", "a_xyz", 80),
            "https://cdn.discordapp.com/avatars/42/a_xyz.gif?size=80"
        );
    }

    #[test]
    fn defaut_systeme_pseudo_et_legacy() {
        // discriminator "0" (nouveau système) → (id >> 22) % 6
        let url = default_avatar_url("80351110224678912", Some("0"));
        assert!(url.starts_with("https://cdn.discordapp.com/embed/avatars/"));
        assert!(url.ends_with(".png"));
        // legacy : discriminator % 5
        assert_eq!(
            default_avatar_url("1", Some("1337")),
            "https://cdn.discordapp.com/embed/avatars/2.png"
        );
    }
}
```

- [ ] **Step 2: Lancer pour vérifier l'échec**

Run: `cargo test -p veloce-app --lib cdn`
Expected: FAIL — fonctions inexistantes.

- [ ] **Step 3: Implémenter `cdn.rs`**

En haut du fichier (avant le module `tests`) :

```rust
use veloce_discord::User;

const CDN: &str = "https://cdn.discordapp.com";

pub fn avatar_url(user_id: &str, hash: &str, size: u32) -> String {
    let ext = if hash.starts_with("a_") { "gif" } else { "png" };
    format!("{CDN}/avatars/{user_id}/{hash}.{ext}?size={size}")
}

pub fn default_avatar_url(user_id: &str, discriminator: Option<&str>) -> String {
    let index = match discriminator {
        Some("0") | None => {
            let id: u64 = user_id.parse().unwrap_or(0);
            (id >> 22) % 6
        }
        Some(d) => (d.parse::<u64>().unwrap_or(0) % 5),
    };
    format!("{CDN}/embed/avatars/{index}.png")
}

pub fn avatar_for(user: &User) -> String {
    match &user.avatar {
        Some(hash) if !hash.is_empty() => avatar_url(&user.id, hash, 80),
        _ => default_avatar_url(&user.id, user.discriminator.as_deref()),
    }
}
```

Dans `main.rs`, ajouter la déclaration de module (ordre alpha) :

```rust
mod app;
mod cdn;
mod emoji;
mod fonts;
mod markdown;
mod net;
mod plugins;
```

- [ ] **Step 4: Lancer pour vérifier le succès**

Run: `cargo test -p veloce-app --lib cdn`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/veloce-app/src/cdn.rs crates/veloce-app/src/main.rs
git commit -m "feat(app): module cdn (urls avatar Discord + defaut)"
```

---

### Task 3: Module `timestamp` — parsing/format ISO 8601 sans dépendance

**Files:**
- Create: `crates/veloce-app/src/timestamp.rs`
- Modify: `crates/veloce-app/src/main.rs` (ajouter `mod timestamp;`)
- Test: `crates/veloce-app/src/timestamp.rs` (module `tests`)

**Interfaces:**
- Produces: `pub fn parse_epoch(iso: &str) -> Option<i64>` (secondes Unix UTC) ;
  `pub fn format_timestamp(iso: &str) -> String` (`"DD/MM/YYYY à HH:MM"`, ou chaîne vide si invalide).

- [ ] **Step 1: Écrire les tests (échouent)**

Créer `crates/veloce-app/src/timestamp.rs` :

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_connu() {
        // 1970-01-01T00:00:00 = 0
        assert_eq!(parse_epoch("1970-01-01T00:00:00.000000+00:00"), Some(0));
        // 2021-01-01T00:00:00Z = 1609459200
        assert_eq!(parse_epoch("2021-01-01T00:00:00+00:00"), Some(1_609_459_200));
    }

    #[test]
    fn epoch_invalide_donne_none() {
        assert_eq!(parse_epoch(""), None);
        assert_eq!(parse_epoch("pas-une-date"), None);
    }

    #[test]
    fn format_lisible() {
        assert_eq!(format_timestamp("2026-06-29T14:23:45.000000+00:00"), "29/06/2026 à 14:23");
    }

    #[test]
    fn format_invalide_donne_vide() {
        assert_eq!(format_timestamp("xxx"), "");
    }
}
```

- [ ] **Step 2: Lancer pour vérifier l'échec**

Run: `cargo test -p veloce-app --lib timestamp`
Expected: FAIL — fonctions inexistantes.

- [ ] **Step 3: Implémenter `timestamp.rs`**

```rust
/// Convertit une date civile (UTC) en jours depuis 1970-01-01 (algorithme de
/// Howard Hinnant, pur entier).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + if m > 2 { -3 } else { 9 }) as i64;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// Parse les composants `YYYY-MM-DDTHH:MM:SS` au début de `iso` (la fraction et
/// le fuseau sont ignorés ; Discord renvoie de l'UTC). Renvoie (y,mo,d,h,mi,s).
fn parts(iso: &str) -> Option<(i64, i64, i64, i64, i64, i64)> {
    let b = iso.as_bytes();
    if b.len() < 19 || b[4] != b'-' || b[7] != b'-' || b[10] != b'T'
        || b[13] != b':' || b[16] != b':'
    {
        return None;
    }
    let num = |a: usize, z: usize| iso.get(a..z)?.parse::<i64>().ok();
    Some((
        num(0, 4)?, num(5, 7)?, num(8, 10)?,
        num(11, 13)?, num(14, 16)?, num(17, 19)?,
    ))
}

pub fn parse_epoch(iso: &str) -> Option<i64> {
    let (y, mo, d, h, mi, s) = parts(iso)?;
    Some(days_from_civil(y, mo, d) * 86400 + h * 3600 + mi * 60 + s)
}

pub fn format_timestamp(iso: &str) -> String {
    match parts(iso) {
        Some((y, mo, d, h, mi, _)) => format!("{d:02}/{mo:02}/{y:04} à {h:02}:{mi:02}"),
        None => String::new(),
    }
}
```

Dans `main.rs`, ajouter `mod timestamp;` (après `mod plugins;` ou en ordre alpha — placer après `mod plugins;`).

- [ ] **Step 4: Lancer pour vérifier le succès**

Run: `cargo test -p veloce-app --lib timestamp`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/veloce-app/src/timestamp.rs crates/veloce-app/src/main.rs
git commit -m "feat(app): parsing/format de timestamp ISO 8601 sans dependance"
```

---

### Task 4: Module `grouping` — regroupement des messages consécutifs

**Files:**
- Create: `crates/veloce-app/src/grouping.rs`
- Modify: `crates/veloce-app/src/main.rs` (ajouter `mod grouping;`)
- Test: `crates/veloce-app/src/grouping.rs` (module `tests`)

**Interfaces:**
- Consumes: `veloce_discord::Message`, `crate::timestamp::parse_epoch`.
- Produces: `pub fn group_flags(messages: &[Message]) -> Vec<bool>` — `flags[i] == true`
  si le message `i` commence un nouveau groupe (afficher avatar + en-tête).

- [ ] **Step 1: Écrire les tests (échouent)**

Créer `crates/veloce-app/src/grouping.rs` :

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use veloce_discord::{Message, User};

    fn msg(id: &str, author: &str, ts: &str) -> Message {
        Message {
            id: id.into(),
            channel_id: "c".into(),
            content: String::new(),
            author: User {
                id: author.into(),
                username: author.into(),
                global_name: None,
                discriminator: None,
                avatar: None,
            },
            timestamp: Some(ts.into()),
            edited_timestamp: None,
            mentions: vec![],
            mention_roles: vec![],
            attachments: vec![],
            embeds: vec![],
            referenced_message: None,
        }
    }

    #[test]
    fn premier_est_toujours_entete() {
        let m = vec![msg("1", "alice", "2026-06-29T14:00:00+00:00")];
        assert_eq!(group_flags(&m), vec![true]);
    }

    #[test]
    fn meme_auteur_proche_est_continuation() {
        let m = vec![
            msg("1", "alice", "2026-06-29T14:00:00+00:00"),
            msg("2", "alice", "2026-06-29T14:03:00+00:00"),
        ];
        assert_eq!(group_flags(&m), vec![true, false]);
    }

    #[test]
    fn auteur_different_coupe() {
        let m = vec![
            msg("1", "alice", "2026-06-29T14:00:00+00:00"),
            msg("2", "bob", "2026-06-29T14:01:00+00:00"),
        ];
        assert_eq!(group_flags(&m), vec![true, true]);
    }

    #[test]
    fn ecart_superieur_a_7min_coupe() {
        let m = vec![
            msg("1", "alice", "2026-06-29T14:00:00+00:00"),
            msg("2", "alice", "2026-06-29T14:08:00+00:00"),
        ];
        assert_eq!(group_flags(&m), vec![true, true]);
    }
}
```

- [ ] **Step 2: Lancer pour vérifier l'échec**

Run: `cargo test -p veloce-app --lib grouping`
Expected: FAIL — `group_flags` inexistant.

- [ ] **Step 3: Implémenter `grouping.rs`**

En haut du fichier :

```rust
use crate::timestamp::parse_epoch;
use veloce_discord::Message;

const GROUP_WINDOW_SECS: i64 = 7 * 60;

pub fn group_flags(messages: &[Message]) -> Vec<bool> {
    let mut flags = Vec::with_capacity(messages.len());
    for (i, m) in messages.iter().enumerate() {
        let new_group = if i == 0 {
            true
        } else {
            let prev = &messages[i - 1];
            if prev.author.id != m.author.id {
                true
            } else {
                match (
                    prev.timestamp.as_deref().and_then(parse_epoch),
                    m.timestamp.as_deref().and_then(parse_epoch),
                ) {
                    (Some(a), Some(b)) => (b - a).abs() > GROUP_WINDOW_SECS,
                    _ => true, // timestamp manquant/illisible → on coupe (en-tête)
                }
            }
        };
        flags.push(new_group);
    }
    flags
}
```

Dans `main.rs`, ajouter `mod grouping;`.

- [ ] **Step 4: Lancer pour vérifier le succès**

Run: `cargo test -p veloce-app --lib grouping`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/veloce-app/src/grouping.rs crates/veloce-app/src/main.rs
git commit -m "feat(app): regroupement des messages consecutifs (meme auteur < 7 min)"
```

---

### Task 5: Refonte du rendu — avatar, en-tête, regroupement, images (`app.rs`)

**Files:**
- Modify: `crates/veloce-app/src/app.rs` (la boucle de messages dans `draw_chat`, ~ lignes 446-465 ; ajouter un champ `viewer` à `ChatState` ; helpers de rendu)
- Test: manuel (rendu egui non testable unitairement) + non-régression `cargo test`.

**Interfaces:**
- Consumes: `crate::cdn::avatar_for`, `crate::timestamp::format_timestamp`,
  `crate::grouping::group_flags`, `veloce_discord::{Message, Attachment}`,
  `crate::render_message` (existant, inchangé pour le corps texte).
- Produces: rendu visuel ; champ `ChatState.viewer: Option<String>`.

- [ ] **Step 1: Ajouter l'état visionneuse à `ChatState`**

Dans la déclaration `struct ChatState` (≈ ligne 31), ajouter le champ :

```rust
    /// URL de l'image ouverte en grand (visionneuse), `None` = fermée.
    viewer: Option<String>,
```

(`#[derive(Default)]` couvre `None` automatiquement.)

- [ ] **Step 2: Ajouter les imports**

En tête de `app.rs`, compléter l'import du crate et des modules :

```rust
use crate::cdn::avatar_for;
use crate::grouping::group_flags;
use crate::timestamp::format_timestamp;
```

et ajouter `Attachment` à la liste `use veloce_discord::{...}`.

- [ ] **Step 3: Écrire le helper de rendu d'une image attachée**

Ajouter dans `app.rs` (près de `render_message`) :

```rust
const IMAGE_MAX_W: f32 = 400.0;

/// Affiche une image attachée, taille bornée en conservant le ratio. Clic →
/// ouvre la visionneuse (renvoie `Some(url)` si cliquée).
fn render_attachment_image(ui: &mut egui::Ui, att: &Attachment) -> Option<String> {
    let (w, h) = (att.width.unwrap_or(0) as f32, att.height.unwrap_or(0) as f32);
    let size = if w > 0.0 && h > 0.0 && w > IMAGE_MAX_W {
        egui::vec2(IMAGE_MAX_W, IMAGE_MAX_W * h / w)
    } else if w > 0.0 && h > 0.0 {
        egui::vec2(w, h)
    } else {
        egui::vec2(IMAGE_MAX_W, IMAGE_MAX_W * 0.6)
    };
    let resp = ui
        .add(
            egui::Image::new(&att.url)
                .fit_to_exact_size(size)
                .sense(egui::Sense::click())
                .corner_radius(6.0),
        )
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    resp.clicked().then(|| att.url.clone())
}
```

- [ ] **Step 4: Réécrire la boucle de messages**

Remplacer le bloc `for m in &state.messages { ... }` (dans le `ScrollArea` du `CentralPanel`, ≈ lignes 449-463) par :

```rust
let flags = group_flags(&state.messages);
for (i, m) in state.messages.iter().enumerate() {
    let header = flags[i];
    ui.horizontal(|ui| {
        // Gouttière avatar 40px.
        ui.allocate_ui(egui::vec2(48.0, 0.0), |ui| {
            ui.set_width(48.0);
            if header {
                ui.add(
                    egui::Image::new(avatar_for(&m.author))
                        .fit_to_exact_size(egui::vec2(40.0, 40.0))
                        .corner_radius(20.0),
                );
            }
        });
        // Colonne contenu.
        ui.vertical(|ui| {
            if header {
                ui.horizontal(|ui| {
                    let name = m
                        .author
                        .global_name
                        .clone()
                        .unwrap_or_else(|| m.author.username.clone());
                    ui.label(RichText::new(name).strong().color(Color32::WHITE));
                    if let Some(ts) = &m.timestamp {
                        let t = format_timestamp(ts);
                        if !t.is_empty() {
                            ui.label(RichText::new(t).small().color(Color32::GRAY));
                        }
                    }
                });
            }
            if !m.content.is_empty() {
                render_message(ui, &m.content, plugins);
            }
            for att in &m.attachments {
                if att.is_image() {
                    if let Some(url) = render_attachment_image(ui, att) {
                        clicked_image = Some(url);
                    }
                } else {
                    let label = format!("📎 {} ({} o)", att.filename, att.size);
                    if ui.link(label).clicked() {
                        ui.ctx().open_url(egui::OpenUrl::new_tab(&att.url));
                    }
                }
            }
        });
    });
    ui.add_space(if header { 6.0 } else { 1.0 });
}
```

Juste avant le `ScrollArea` (ou avant la boucle), déclarer l'accumulateur de clic :

```rust
let mut clicked_image: Option<String> = None;
```

et juste après le `ScrollArea`, appliquer le clic :

```rust
if let Some(url) = clicked_image {
    state.viewer = Some(url);
}
```

- [ ] **Step 5: Ajouter l'overlay visionneuse**

À la fin de `draw_chat` (avant la fermeture de la fonction, après le bloc `if *show_plugins`), ajouter :

```rust
if let Some(url) = state.viewer.clone() {
    let mut close = false;
    egui::Area::new(egui::Id::new("image_viewer"))
        .fixed_pos(egui::pos2(0.0, 0.0))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            let screen = ctx.screen_rect();
            ui.painter()
                .rect_filled(screen, 0.0, Color32::from_black_alpha(220));
            let resp = ui.allocate_rect(screen, egui::Sense::click());
            ui.put(
                screen.shrink(40.0),
                egui::Image::new(&url).max_size(screen.size() * 0.9),
            );
            if resp.clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                close = true;
            }
        });
    if close {
        state.viewer = None;
    }
}
```

- [ ] **Step 6: Build + clippy + fmt**

Run: `cargo build -p veloce-app && cargo clippy -p veloce-app -- -D warnings && cargo fmt --check`
Expected: compile sans erreur ni warning ; fmt propre. Corriger les noms d'API egui 0.30 si besoin (`corner_radius` vs `rounding` : sur egui 0.30 c'est `corner_radius`; si l'API diffère, adapter et re-vérifier).

- [ ] **Step 7: Non-régression des tests**

Run: `cargo test`
Expected: PASS (workspace entier).

- [ ] **Step 8: Vérification manuelle**

Run: `cargo run -p veloce-app`
Se connecter, ouvrir un salon avec des images et plusieurs messages d'affilée. Vérifier :
- avatars affichés à gauche, en-tête `nom · heure` ;
- messages consécutifs du même auteur regroupés (pas d'avatar/en-tête répété) ;
- images affichées inline, clic → visionneuse plein écran, clic/Échap → ferme ;
- pièces jointes non-image en lien `📎`.

- [ ] **Step 9: Commit**

```bash
git add crates/veloce-app/src/app.rs
git commit -m "feat(app): layout Discord (avatar, en-tete, regroupement) + images inline"
```

---

## Notes d'exécution

- **Ordre** : Tâches 1→5 strictement (chaque module pur avant son usage dans `app.rs`).
- **egui 0.30** : les noms `corner_radius`, `OpenUrl::new_tab`, `Image::sense` sont ceux de la 0.30 ; si un nom diffère à la compilation, adapter localement (l'app utilise déjà `egui::Image::new(url)` et `fit_to_exact_size`).
- **Hors périmètre Phase 1** (plans suivants) : tokenizer mentions/spoilers/liens (Phase 2), cartes d'embed + réponses + popup profil (Phase 3). Les données sont déjà capturées par la Tâche 1.
