# Veloce — Plan d'implémentation : fondation v0.1

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Construire un client Discord natif minimal en Rust (egui) qui se connecte avec un token utilisateur, affiche serveurs/salons, lit les messages d'un salon en temps réel et envoie des messages texte.

**Architecture:** Workspace Cargo à deux crates. `veloce-discord` (lib, UI-agnostique) parle à Discord (REST + Gateway WebSocket) et expose des `Event`/`Command`/modèles. `veloce-app` (bin) est l'UI egui : un thread tokio fait le réseau, le thread principal fait l'UI, les deux communiquent par canaux. Les unités à logique pure (machine à états gateway, parseur markdown, identité, parsing REST) sont isolées pour être testables sans réseau.

**Tech Stack:** Rust 2021, tokio, tokio-tungstenite (rustls), reqwest (rustls), serde/serde_json, eframe/egui, keyring, tracing, thiserror/anyhow, base64.

## Global Constraints

- Édition Rust **2021**, MSRV **1.75+**.
- Pas d'OpenSSL : TLS via **rustls** partout (`reqwest` et `tokio-tungstenite`).
- IDs Discord = **`String`** (type alias `Snowflake`), jamais `u64` (overflow JSON/precision).
- Les modèles serde sont **tolérants** : tout champ non requis en v0.1 est `Option<_>` avec `#[serde(default)]`.
- `veloce-discord` **ne dépend jamais** d'egui/eframe.
- Versions = **planchers** ; épingler la dernière `0.x` compatible au moment de l'implémentation : `tokio "1"`, `tokio-tungstenite "0.26"`, `futures-util "0.3"`, `reqwest "0.12"`, `serde "1"`, `serde_json "1"`, `eframe "0.30"`, `egui "0.30"`, `keyring "3"`, `tracing "0.1"`, `tracing-subscriber "0.3"`, `thiserror "2"`, `anyhow "1"`, `base64 "0.22"`, `url "2"`.
- Licence **GPL-3.0**. En-tête de licence non requis par fichier, mais `LICENSE` à la racine.
- API Discord **v10**, `Authorization: <token>` (token user, **sans** préfixe `Bot`).
- Chaque tâche se termine par un commit. Messages de commit en français, style `type: description`.

---

## Structure des fichiers

```
Veloce/
├─ Cargo.toml                              # workspace (Task 1)
├─ LICENSE  README.md  .gitignore          # (Task 1, .gitignore déjà créé)
├─ .github/workflows/ci.yml                # (Task 1)
├─ crates/
│  ├─ veloce-discord/
│  │  ├─ Cargo.toml
│  │  └─ src/
│  │     ├─ lib.rs                         # ré-exports (Task 1, complété au fil de l'eau)
│  │     ├─ models.rs                      # Task 2
│  │     ├─ events.rs                      # Task 3
│  │     ├─ commands.rs                    # Task 3
│  │     ├─ identity.rs                    # Task 4
│  │     ├─ gateway_state.rs               # Task 5
│  │     ├─ error.rs                       # Task 6
│  │     ├─ rest.rs                        # Task 6
│  │     └─ gateway.rs                     # Task 7
│  └─ veloce-app/
│     ├─ Cargo.toml
│     └─ src/
│        ├─ main.rs                        # Task 10
│        ├─ app.rs                         # Task 10
│        ├─ net.rs                         # Task 8
│        └─ markdown.rs                    # Task 9
└─ tests/fixtures/                         # JSON capturés (Task 2)
```

---

### Task 1 : Scaffold du workspace, CI, licence, README

**Files:**
- Create: `Cargo.toml` (workspace)
- Create: `crates/veloce-discord/Cargo.toml`, `crates/veloce-discord/src/lib.rs`
- Create: `crates/veloce-app/Cargo.toml`, `crates/veloce-app/src/main.rs`
- Create: `LICENSE`, `README.md`, `.github/workflows/ci.yml`

**Interfaces:**
- Consumes: rien.
- Produces: workspace compilable ; crate lib `veloce_discord` ; binaire `veloce`.

- [ ] **Step 1 : Cargo.toml du workspace**

```toml
[workspace]
resolver = "2"
members = ["crates/veloce-discord", "crates/veloce-app"]

[workspace.package]
edition = "2021"
rust-version = "1.75"
license = "GPL-3.0"
repository = "https://github.com/<user>/Veloce"

[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
tracing = "0.1"
```

- [ ] **Step 2 : Cargo.toml de veloce-discord**

```toml
[package]
name = "veloce-discord"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true

[dependencies]
serde.workspace = true
serde_json.workspace = true
tokio.workspace = true
tracing.workspace = true
tokio-tungstenite = { version = "0.26", default-features = false, features = ["connect", "rustls-tls-webpki-roots"] }
futures-util = "0.3"
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }
thiserror = "2"
base64 = "0.22"
url = "2"
```

- [ ] **Step 3 : lib.rs initial**

```rust
//! veloce-discord — client Discord (REST + Gateway), UI-agnostique.
```

- [ ] **Step 4 : Cargo.toml de veloce-app**

```toml
[package]
name = "veloce-app"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true

[[bin]]
name = "veloce"
path = "src/main.rs"

[dependencies]
veloce-discord = { path = "../veloce-discord" }
serde.workspace = true
serde_json.workspace = true
tokio.workspace = true
tracing.workspace = true
eframe = "0.30"
egui = "0.30"
keyring = "3"
anyhow = "1"
tracing-subscriber = "0.3"
```

- [ ] **Step 5 : main.rs minimal**

```rust
fn main() {
    println!("Veloce — fondation v0.1");
}
```

- [ ] **Step 6 : LICENSE et README**

Écrire le texte intégral **GPL-3.0** dans `LICENSE` (copier depuis https://www.gnu.org/licenses/gpl-3.0.txt). `README.md` :

```markdown
# Veloce

Client Discord natif, **100% Rust**, dans l'esprit de [Vencord](https://github.com/Vendicated/Vencord) : rapide et léger.

> ⚠️ **Avertissement** : Veloce est un client tiers. Les clients tiers sont dans une zone grise des CGU Discord et peuvent exposer un compte à un bannissement. Utilisez un **compte secondaire**.

## Build
```
cargo run --bin veloce
```

## Licence
GPL-3.0
```

- [ ] **Step 7 : CI GitHub Actions** — `.github/workflows/ci.yml`

```yaml
name: CI
on: [push, pull_request]
jobs:
  check:
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
      - run: cargo fmt --all -- --check
      - run: cargo clippy --all-targets -- -D warnings
      - run: cargo test --all
```

- [ ] **Step 8 : Vérifier le build**

Run: `cargo build && cargo run --bin veloce`
Expected: compile, affiche « Veloce — fondation v0.1 ».

- [ ] **Step 9 : Commit**

```bash
git add -A
git commit -m "chore: scaffold du workspace, CI, licence GPL-3.0 et README"
```

---

### Task 2 : Modèles Discord + désérialisation

**Files:**
- Create: `crates/veloce-discord/src/models.rs`
- Create: `tests/fixtures/message_create.json`, `tests/fixtures/guild.json`, `tests/fixtures/channel.json`
- Modify: `crates/veloce-discord/src/lib.rs`

**Interfaces:**
- Consumes: rien.
- Produces: `Snowflake` (= `String`) ; structs `User { id, username, global_name: Option<String>, discriminator: Option<String> }`, `Guild { id, name, icon: Option<String> }`, `Channel { id, name: Option<String>, kind: u8 (serde rename "type"), guild_id: Option<Snowflake>, position: Option<i32> }`, `Message { id, channel_id, content, author: User, timestamp: Option<String> }`, `GatewayPayload { op: u8, d: serde_json::Value, s: Option<u64>, t: Option<String> }`. Tous `#[derive(Debug, Clone, serde::Deserialize)]` (sauf `GatewayPayload` : Debug + Deserialize).

- [ ] **Step 1 : Fixtures JSON** — créer `tests/fixtures/message_create.json` :

```json
{ "id": "111", "channel_id": "222", "content": "salut **monde**",
  "timestamp": "2026-06-28T10:00:00.000000+00:00",
  "author": { "id": "333", "username": "alice", "global_name": "Alice", "discriminator": "0" } }
```

`tests/fixtures/guild.json` :

```json
{ "id": "10", "name": "Mon Serveur", "icon": "abc123", "owner_id": "333" }
```

`tests/fixtures/channel.json` :

```json
{ "id": "222", "type": 0, "name": "général", "guild_id": "10", "position": 1, "topic": null }
```

- [ ] **Step 2 : Écrire les tests (échouent)** — dans `crates/veloce-discord/src/models.rs` :

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialise_message_avec_champs_extra() {
        let json = include_str!("../../../tests/fixtures/message_create.json");
        let m: Message = serde_json::from_str(json).unwrap();
        assert_eq!(m.id, "111");
        assert_eq!(m.channel_id, "222");
        assert_eq!(m.content, "salut **monde**");
        assert_eq!(m.author.username, "alice");
        assert_eq!(m.author.global_name.as_deref(), Some("Alice"));
    }

    #[test]
    fn deserialise_channel_type_renomme() {
        let json = include_str!("../../../tests/fixtures/channel.json");
        let c: Channel = serde_json::from_str(json).unwrap();
        assert_eq!(c.kind, 0);
        assert_eq!(c.name.as_deref(), Some("général"));
        assert_eq!(c.guild_id.as_deref(), Some("10"));
    }

    #[test]
    fn deserialise_guild_ignore_owner_id() {
        let json = include_str!("../../../tests/fixtures/guild.json");
        let g: Guild = serde_json::from_str(json).unwrap();
        assert_eq!(g.name, "Mon Serveur");
        assert_eq!(g.icon.as_deref(), Some("abc123"));
    }
}
```

- [ ] **Step 3 : Vérifier l'échec**

Run: `cargo test -p veloce-discord models`
Expected: FAIL (types `Message`/`Channel`/`Guild` introuvables).

- [ ] **Step 4 : Implémenter les modèles** — en haut de `models.rs` :

```rust
use serde::Deserialize;

pub type Snowflake = String;

#[derive(Debug, Clone, Deserialize)]
pub struct User {
    pub id: Snowflake,
    pub username: String,
    #[serde(default)]
    pub global_name: Option<String>,
    #[serde(default)]
    pub discriminator: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Guild {
    pub id: Snowflake,
    pub name: String,
    #[serde(default)]
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Channel {
    pub id: Snowflake,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub kind: u8,
    #[serde(default)]
    pub guild_id: Option<Snowflake>,
    #[serde(default)]
    pub position: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Message {
    pub id: Snowflake,
    pub channel_id: Snowflake,
    pub content: String,
    pub author: User,
    #[serde(default)]
    pub timestamp: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GatewayPayload {
    pub op: u8,
    #[serde(default)]
    pub d: serde_json::Value,
    #[serde(default)]
    pub s: Option<u64>,
    #[serde(default)]
    pub t: Option<String>,
}
```

- [ ] **Step 5 : Exporter dans lib.rs** — ajouter :

```rust
pub mod models;
pub use models::{Channel, GatewayPayload, Guild, Message, Snowflake, User};
```

- [ ] **Step 6 : Vérifier le succès**

Run: `cargo test -p veloce-discord models`
Expected: PASS (3 tests).

- [ ] **Step 7 : Commit**

```bash
git add -A
git commit -m "feat(discord): modèles serde tolérants + tests de désérialisation"
```

---

### Task 3 : Enums Event et Command

**Files:**
- Create: `crates/veloce-discord/src/events.rs`, `crates/veloce-discord/src/commands.rs`
- Modify: `crates/veloce-discord/src/lib.rs`

**Interfaces:**
- Consumes: `models::{User, Guild, Channel, Message, Snowflake}`.
- Produces:
  - `enum ConnectionState { Connecting, Connected, Reconnecting, Disconnected }` (derive Debug, Clone, PartialEq).
  - `enum Event { Connection(ConnectionState), Ready { user: User, guilds: Vec<Guild> }, ChannelsLoaded { guild_id: Snowflake, channels: Vec<Channel> }, MessagesLoaded { channel_id: Snowflake, messages: Vec<Message> }, MessageCreated(Message), MessageUpdated(Message), MessageDeleted { id: Snowflake, channel_id: Snowflake }, Error(String) }` (derive Debug, Clone).
  - `enum Command { SelectGuild(Snowflake), FetchHistory(Snowflake), SendMessage { channel_id: Snowflake, content: String } }` (derive Debug, Clone).

- [ ] **Step 1 : Écrire le test (échoue)** — dans `events.rs` :

```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn event_est_clonable_et_debug() {
        let e = Event::Connection(ConnectionState::Connected);
        let _ = format!("{:?}", e.clone());
        assert_eq!(ConnectionState::Connected, ConnectionState::Connected);
    }
}
```

- [ ] **Step 2 : Vérifier l'échec**

Run: `cargo test -p veloce-discord events`
Expected: FAIL (`Event` introuvable).

- [ ] **Step 3 : Implémenter events.rs**

```rust
use crate::models::{Channel, Guild, Message, Snowflake, User};

#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
    Connecting,
    Connected,
    Reconnecting,
    Disconnected,
}

#[derive(Debug, Clone)]
pub enum Event {
    Connection(ConnectionState),
    Ready { user: User, guilds: Vec<Guild> },
    ChannelsLoaded { guild_id: Snowflake, channels: Vec<Channel> },
    MessagesLoaded { channel_id: Snowflake, messages: Vec<Message> },
    MessageCreated(Message),
    MessageUpdated(Message),
    MessageDeleted { id: Snowflake, channel_id: Snowflake },
    Error(String),
}
```

- [ ] **Step 4 : Implémenter commands.rs**

```rust
use crate::models::Snowflake;

#[derive(Debug, Clone)]
pub enum Command {
    SelectGuild(Snowflake),
    FetchHistory(Snowflake),
    SendMessage { channel_id: Snowflake, content: String },
}
```

- [ ] **Step 5 : Exporter dans lib.rs**

```rust
pub mod commands;
pub mod events;
pub use commands::Command;
pub use events::{ConnectionState, Event};
```

- [ ] **Step 6 : Vérifier le succès**

Run: `cargo test -p veloce-discord events`
Expected: PASS.

- [ ] **Step 7 : Commit**

```bash
git add -A
git commit -m "feat(discord): enums Event et Command (couture plugins)"
```

---

### Task 4 : Identité client (super properties)

**Files:**
- Create: `crates/veloce-discord/src/identity.rs`
- Modify: `crates/veloce-discord/src/lib.rs`

**Interfaces:**
- Consumes: rien (crate `base64`, `serde_json`).
- Produces: `pub fn super_properties_json() -> serde_json::Value` ; `pub fn super_properties_b64() -> String` (base64 standard du JSON compact) ; `pub const USER_AGENT: &str`.

- [ ] **Step 1 : Écrire les tests (échouent)** — `identity.rs` :

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;

    #[test]
    fn super_properties_b64_se_decode_en_json_valide() {
        let b64 = super_properties_b64();
        let bytes = base64::engine::general_purpose::STANDARD.decode(b64).unwrap();
        let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert!(v.get("os").is_some());
        assert!(v.get("browser").is_some());
        assert!(v.get("client_build_number").is_some());
    }

    #[test]
    fn user_agent_non_vide() {
        assert!(USER_AGENT.contains("Mozilla"));
    }
}
```

- [ ] **Step 2 : Vérifier l'échec**

Run: `cargo test -p veloce-discord identity`
Expected: FAIL.

- [ ] **Step 3 : Implémenter identity.rs**

```rust
use base64::Engine;
use serde_json::json;

/// User-Agent mimant un navigateur récent. À maintenir si Discord durcit ses contrôles.
pub const USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
     (KHTML, like Gecko) discord/1.0.9028 Chrome/120.0.0.0 Electron/28.0.0 Safari/537.36";

/// Propriétés client envoyées via X-Super-Properties et dans IDENTIFY.
/// `client_build_number` évolue côté Discord ; le mettre à jour ici uniquement.
pub fn super_properties_json() -> serde_json::Value {
    json!({
        "os": "Windows",
        "browser": "Discord Client",
        "release_channel": "stable",
        "client_version": "1.0.9028",
        "os_version": "10.0.19045",
        "system_locale": "fr",
        "client_build_number": 9999,
        "native_build_number": 9999
    })
}

pub fn super_properties_b64() -> String {
    let s = serde_json::to_string(&super_properties_json()).expect("json valide");
    base64::engine::general_purpose::STANDARD.encode(s)
}
```

- [ ] **Step 4 : Exporter dans lib.rs**

```rust
pub mod identity;
```

- [ ] **Step 5 : Vérifier le succès**

Run: `cargo test -p veloce-discord identity`
Expected: PASS (2 tests).

- [ ] **Step 6 : Commit**

```bash
git add -A
git commit -m "feat(discord): identité client (super properties, user-agent)"
```

---

### Task 5 : Machine à états Gateway (pure)

**Files:**
- Create: `crates/veloce-discord/src/gateway_state.rs`
- Modify: `crates/veloce-discord/src/lib.rs`

**Interfaces:**
- Consumes: rien.
- Produces:
  - `enum GatewayAction { StartHeartbeat { interval_ms: u64 }, SendHeartbeat, Identify, Resume { session_id: String, seq: Option<u64> }, ReconnectResumable, ReconnectFull, Dispatch(String), Ignore }` (derive Debug, Clone, PartialEq).
  - `struct GatewayState { seq: Option<u64>, session_id: Option<String>, heartbeat_interval_ms: Option<u64>, last_ack: bool }` (derive Debug, Clone, Default).
  - Méthodes : `fn on_payload(&mut self, op: u8, t: Option<&str>, s: Option<u64>, invalid_session_resumable: Option<bool>) -> GatewayAction` ; `fn handshake_action(&self) -> GatewayAction` ; `fn set_session(&mut self, id: String)`.

**Sémantique attendue :** op 10 (HELLO, `d.heartbeat_interval` passé via paramètre n'existe pas ici — voir note) → on enregistre l'intervalle et on renvoie `StartHeartbeat`. Pour passer l'intervalle, `on_payload` lit l'op mais l'intervalle vient d'un appel dédié `on_hello(interval_ms)`. Décision : ajouter `fn on_hello(&mut self, interval_ms: u64) -> GatewayAction` séparé pour rester pur et explicite. op 0 → enregistre `s` dans `seq`, renvoie `Dispatch(t)`. op 1 → `SendHeartbeat`. op 7 → `ReconnectResumable`. op 9 → `ReconnectResumable` si `Some(true)`, sinon vide la session et `ReconnectFull`. op 11 → `last_ack = true`, `Ignore`.

- [ ] **Step 1 : Écrire les tests (échouent)** — `gateway_state.rs` :

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hello_demarre_heartbeat() {
        let mut s = GatewayState::default();
        assert_eq!(s.on_hello(41250), GatewayAction::StartHeartbeat { interval_ms: 41250 });
        assert_eq!(s.heartbeat_interval_ms, Some(41250));
    }

    #[test]
    fn handshake_identify_sans_session_puis_resume_avec() {
        let mut s = GatewayState::default();
        assert_eq!(s.handshake_action(), GatewayAction::Identify);
        s.set_session("sess-1".into());
        s.on_payload(0, Some("MESSAGE_CREATE"), Some(7), None);
        assert_eq!(
            s.handshake_action(),
            GatewayAction::Resume { session_id: "sess-1".into(), seq: Some(7) }
        );
    }

    #[test]
    fn dispatch_enregistre_la_sequence() {
        let mut s = GatewayState::default();
        let a = s.on_payload(0, Some("READY"), Some(1), None);
        assert_eq!(a, GatewayAction::Dispatch("READY".into()));
        assert_eq!(s.seq, Some(1));
    }

    #[test]
    fn op1_demande_heartbeat() {
        let mut s = GatewayState::default();
        assert_eq!(s.on_payload(1, None, None, None), GatewayAction::SendHeartbeat);
    }

    #[test]
    fn invalid_session_non_resumable_vide_la_session() {
        let mut s = GatewayState::default();
        s.set_session("x".into());
        let a = s.on_payload(9, None, None, Some(false));
        assert_eq!(a, GatewayAction::ReconnectFull);
        assert!(s.session_id.is_none());
    }

    #[test]
    fn invalid_session_resumable_garde_la_session() {
        let mut s = GatewayState::default();
        s.set_session("x".into());
        let a = s.on_payload(9, None, None, Some(true));
        assert_eq!(a, GatewayAction::ReconnectResumable);
        assert_eq!(s.session_id.as_deref(), Some("x"));
    }

    #[test]
    fn ack_met_a_jour_last_ack() {
        let mut s = GatewayState::default();
        s.last_ack = false;
        assert_eq!(s.on_payload(11, None, None, None), GatewayAction::Ignore);
        assert!(s.last_ack);
    }
}
```

- [ ] **Step 2 : Vérifier l'échec**

Run: `cargo test -p veloce-discord gateway_state`
Expected: FAIL.

- [ ] **Step 3 : Implémenter gateway_state.rs**

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum GatewayAction {
    StartHeartbeat { interval_ms: u64 },
    SendHeartbeat,
    Identify,
    Resume { session_id: String, seq: Option<u64> },
    ReconnectResumable,
    ReconnectFull,
    Dispatch(String),
    Ignore,
}

#[derive(Debug, Clone, Default)]
pub struct GatewayState {
    pub seq: Option<u64>,
    pub session_id: Option<String>,
    pub heartbeat_interval_ms: Option<u64>,
    pub last_ack: bool,
}

impl GatewayState {
    pub fn on_hello(&mut self, interval_ms: u64) -> GatewayAction {
        self.heartbeat_interval_ms = Some(interval_ms);
        self.last_ack = true;
        GatewayAction::StartHeartbeat { interval_ms }
    }

    pub fn handshake_action(&self) -> GatewayAction {
        match &self.session_id {
            Some(id) => GatewayAction::Resume { session_id: id.clone(), seq: self.seq },
            None => GatewayAction::Identify,
        }
    }

    pub fn set_session(&mut self, id: String) {
        self.session_id = Some(id);
    }

    pub fn on_payload(
        &mut self,
        op: u8,
        t: Option<&str>,
        s: Option<u64>,
        invalid_session_resumable: Option<bool>,
    ) -> GatewayAction {
        match op {
            0 => {
                if let Some(seq) = s {
                    self.seq = Some(seq);
                }
                GatewayAction::Dispatch(t.unwrap_or_default().to_string())
            }
            1 => GatewayAction::SendHeartbeat,
            7 => GatewayAction::ReconnectResumable,
            9 => {
                if invalid_session_resumable == Some(true) {
                    GatewayAction::ReconnectResumable
                } else {
                    self.session_id = None;
                    self.seq = None;
                    GatewayAction::ReconnectFull
                }
            }
            11 => {
                self.last_ack = true;
                GatewayAction::Ignore
            }
            _ => GatewayAction::Ignore,
        }
    }
}
```

- [ ] **Step 4 : Exporter dans lib.rs**

```rust
pub mod gateway_state;
pub use gateway_state::{GatewayAction, GatewayState};
```

- [ ] **Step 5 : Vérifier le succès**

Run: `cargo test -p veloce-discord gateway_state`
Expected: PASS (7 tests).

- [ ] **Step 6 : Commit**

```bash
git add -A
git commit -m "feat(discord): machine à états gateway pure + tests"
```

---

### Task 6 : Erreurs + client REST

**Files:**
- Create: `crates/veloce-discord/src/error.rs`, `crates/veloce-discord/src/rest.rs`
- Modify: `crates/veloce-discord/src/lib.rs`

**Interfaces:**
- Consumes: `models::*`, `identity::*`.
- Produces:
  - `enum DiscordError { Http(reqwest::Error), Unauthorized, RateLimited { retry_after_ms: u64 }, Api { status: u16, body: String }, Decode(String) }` + `impl std::error::Error` via `thiserror` ; alias `type Result<T> = std::result::Result<T, DiscordError>`.
  - `fn parse_retry_after_ms(header_value: Option<&str>) -> u64` (pur, testable ; secondes flottantes → ms ; défaut 1000).
  - `struct RestClient { http: reqwest::Client, token: String }` ; `RestClient::new(token: String) -> Result<Self>` (configure les en-têtes par défaut : `Authorization`, `User-Agent`, `X-Super-Properties`, `Content-Type: application/json`) ; méthodes async `current_user(&self) -> Result<User>`, `current_user_guilds(&self) -> Result<Vec<Guild>>`, `guild_channels(&self, guild_id: &str) -> Result<Vec<Channel>>`, `channel_messages(&self, channel_id: &str, limit: u8) -> Result<Vec<Message>>`, `send_message(&self, channel_id: &str, content: &str) -> Result<Message>`.

- [ ] **Step 1 : Écrire le test pur (échoue)** — `rest.rs` :

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_after_secondes_vers_ms() {
        assert_eq!(parse_retry_after_ms(Some("1.5")), 1500);
        assert_eq!(parse_retry_after_ms(Some("0.2")), 200);
    }

    #[test]
    fn retry_after_absent_ou_invalide_donne_defaut() {
        assert_eq!(parse_retry_after_ms(None), 1000);
        assert_eq!(parse_retry_after_ms(Some("abc")), 1000);
    }
}
```

- [ ] **Step 2 : Vérifier l'échec**

Run: `cargo test -p veloce-discord rest`
Expected: FAIL (`parse_retry_after_ms` introuvable).

- [ ] **Step 3 : Implémenter error.rs**

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DiscordError {
    #[error("erreur HTTP: {0}")]
    Http(#[from] reqwest::Error),
    #[error("token invalide (401)")]
    Unauthorized,
    #[error("rate limited, réessai dans {retry_after_ms} ms")]
    RateLimited { retry_after_ms: u64 },
    #[error("erreur API {status}: {body}")]
    Api { status: u16, body: String },
    #[error("erreur de décodage: {0}")]
    Decode(String),
}

pub type Result<T> = std::result::Result<T, DiscordError>;
```

- [ ] **Step 4 : Implémenter rest.rs**

```rust
use crate::error::{DiscordError, Result};
use crate::identity::{super_properties_b64, USER_AGENT};
use crate::models::{Channel, Guild, Message, User};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE, USER_AGENT as UA};

const API_BASE: &str = "https://discord.com/api/v10";

pub fn parse_retry_after_ms(header_value: Option<&str>) -> u64 {
    header_value
        .and_then(|v| v.parse::<f64>().ok())
        .map(|secs| (secs * 1000.0) as u64)
        .unwrap_or(1000)
}

pub struct RestClient {
    http: reqwest::Client,
    #[allow(dead_code)]
    token: String,
}

impl RestClient {
    pub fn new(token: String) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, HeaderValue::from_str(&token).map_err(|e| DiscordError::Decode(e.to_string()))?);
        headers.insert(UA, HeaderValue::from_static(USER_AGENT));
        headers.insert("X-Super-Properties", HeaderValue::from_str(&super_properties_b64()).map_err(|e| DiscordError::Decode(e.to_string()))?);
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        let http = reqwest::Client::builder().default_headers(headers).build()?;
        Ok(Self { http, token })
    }

    async fn get_json<T: serde::de::DeserializeOwned>(&self, url: String) -> Result<T> {
        let resp = self.http.get(&url).send().await?;
        Self::handle(resp).await
    }

    async fn handle<T: serde::de::DeserializeOwned>(resp: reqwest::Response) -> Result<T> {
        let status = resp.status();
        if status.as_u16() == 401 {
            return Err(DiscordError::Unauthorized);
        }
        if status.as_u16() == 429 {
            let ra = resp.headers().get("retry-after").and_then(|v| v.to_str().ok()).map(|s| s.to_string());
            return Err(DiscordError::RateLimited { retry_after_ms: parse_retry_after_ms(ra.as_deref()) });
        }
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(DiscordError::Api { status: status.as_u16(), body });
        }
        resp.json::<T>().await.map_err(|e| DiscordError::Decode(e.to_string()))
    }

    pub async fn current_user(&self) -> Result<User> {
        self.get_json(format!("{API_BASE}/users/@me")).await
    }

    pub async fn current_user_guilds(&self) -> Result<Vec<Guild>> {
        self.get_json(format!("{API_BASE}/users/@me/guilds")).await
    }

    pub async fn guild_channels(&self, guild_id: &str) -> Result<Vec<Channel>> {
        self.get_json(format!("{API_BASE}/guilds/{guild_id}/channels")).await
    }

    pub async fn channel_messages(&self, channel_id: &str, limit: u8) -> Result<Vec<Message>> {
        self.get_json(format!("{API_BASE}/channels/{channel_id}/messages?limit={limit}")).await
    }

    pub async fn send_message(&self, channel_id: &str, content: &str) -> Result<Message> {
        let body = serde_json::json!({ "content": content });
        let resp = self.http.post(format!("{API_BASE}/channels/{channel_id}/messages")).json(&body).send().await?;
        Self::handle(resp).await
    }
}
```

- [ ] **Step 5 : Exporter dans lib.rs**

```rust
pub mod error;
pub mod rest;
pub use error::{DiscordError, Result};
pub use rest::RestClient;
```

- [ ] **Step 6 : Vérifier le succès**

Run: `cargo test -p veloce-discord rest && cargo clippy -p veloce-discord -- -D warnings`
Expected: PASS (2 tests), clippy clean.

- [ ] **Step 7 : Commit**

```bash
git add -A
git commit -m "feat(discord): client REST + erreurs + parsing retry-after"
```

---

### Task 7 : Connexion Gateway (WebSocket)

**Files:**
- Create: `crates/veloce-discord/src/gateway.rs`
- Modify: `crates/veloce-discord/src/lib.rs`

**Interfaces:**
- Consumes: `GatewayState`/`GatewayAction`, `GatewayPayload`, `identity`, `events::{Event, ConnectionState}`, `models::*`, `error::Result`.
- Produces: `async fn run_gateway(token: String, event_tx: tokio::sync::mpsc::UnboundedSender<Event>, shutdown: tokio::sync::watch::Receiver<bool>)` — boucle de connexion auto-reconnectante qui émet des `Event` (Ready, MessageCreated, Connection, Error). Pas de valeur de retour (tourne jusqu'au shutdown). Constante `GATEWAY_URL = "wss://gateway.discord.gg/?v=10&encoding=json"`.

**Note :** cette tâche est de l'I/O ; sa logique pure (transitions) est déjà testée en Task 5. Vérification = compilation + clippy + (manuel) connexion réelle. Pas de test unitaire ajouté ici hormis un test de construction du payload IDENTIFY.

- [ ] **Step 1 : Test pur du payload IDENTIFY (échoue)** — `gateway.rs` :

```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn identify_contient_token_et_properties() {
        let v = build_identify("mon-token");
        assert_eq!(v["op"], 2);
        assert_eq!(v["d"]["token"], "mon-token");
        assert!(v["d"]["properties"]["os"].is_string());
    }
}
```

- [ ] **Step 2 : Vérifier l'échec**

Run: `cargo test -p veloce-discord gateway::tests`
Expected: FAIL (`build_identify` introuvable).

- [ ] **Step 3 : Implémenter gateway.rs** (helpers purs + boucle I/O)

```rust
use crate::events::{ConnectionState, Event};
use crate::gateway_state::{GatewayAction, GatewayState};
use crate::identity::super_properties_json;
use crate::models::{GatewayPayload, Guild, Message, User};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::watch;
use tokio_tungstenite::tungstenite::Message as WsMessage;

pub const GATEWAY_URL: &str = "wss://gateway.discord.gg/?v=10&encoding=json";

pub fn build_identify(token: &str) -> Value {
    json!({
        "op": 2,
        "d": {
            "token": token,
            "capabilities": 16381,
            "properties": super_properties_json(),
            "presence": { "status": "online", "since": 0, "activities": [], "afk": false },
            "compress": false
        }
    })
}

fn build_resume(token: &str, session_id: &str, seq: Option<u64>) -> Value {
    json!({ "op": 6, "d": { "token": token, "session_id": session_id, "seq": seq.unwrap_or(0) } })
}

fn build_heartbeat(seq: Option<u64>) -> Value {
    json!({ "op": 1, "d": seq })
}

/// Boucle principale : (re)connexion jusqu'au shutdown, avec backoff.
pub async fn run_gateway(
    token: String,
    event_tx: UnboundedSender<Event>,
    mut shutdown: watch::Receiver<bool>,
) {
    let mut state = GatewayState::default();
    let mut backoff_ms = 1000u64;
    loop {
        if *shutdown.borrow() {
            return;
        }
        let _ = event_tx.send(Event::Connection(ConnectionState::Connecting));
        match connect_once(&token, &mut state, &event_tx, &mut shutdown).await {
            Ok(()) => return, // shutdown propre
            Err(()) => {
                let _ = event_tx.send(Event::Connection(ConnectionState::Reconnecting));
                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_millis(backoff_ms)) => {}
                    _ = shutdown.changed() => return,
                }
                backoff_ms = (backoff_ms * 2).min(30_000);
            }
        }
    }
}

/// Une session : handshake + boucle de réception. Err(()) = besoin de reconnecter.
async fn connect_once(
    token: &str,
    state: &mut GatewayState,
    event_tx: &UnboundedSender<Event>,
    shutdown: &mut watch::Receiver<bool>,
) -> std::result::Result<(), ()> {
    let (ws, _) = tokio_tungstenite::connect_async(GATEWAY_URL).await.map_err(|e| {
        let _ = event_tx.send(Event::Error(format!("connexion gateway: {e}")));
    })?;
    let (mut write, mut read) = ws.split();
    let mut hb = tokio::time::interval(Duration::from_secs(45));
    hb.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut handshaken = false;

    loop {
        tokio::select! {
            _ = shutdown.changed() => return Ok(()),
            _ = hb.tick(), if state.heartbeat_interval_ms.is_some() => {
                if write.send(WsMessage::Text(build_heartbeat(state.seq).to_string().into())).await.is_err() {
                    return Err(());
                }
            }
            msg = read.next() => {
                let Some(Ok(WsMessage::Text(txt))) = msg else {
                    if matches!(msg, Some(Ok(WsMessage::Close(_))) | None) { return Err(()); }
                    continue;
                };
                let payload: GatewayPayload = match serde_json::from_str(&txt) {
                    Ok(p) => p,
                    Err(_) => continue,
                };
                // HELLO : démarre heartbeat à l'intervalle reçu, puis handshake
                if payload.op == 10 {
                    if let Some(ms) = payload.d.get("heartbeat_interval").and_then(Value::as_u64) {
                        state.on_hello(ms);
                        hb = tokio::time::interval(Duration::from_millis(ms));
                        hb.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
                        hb.reset();
                    }
                    let hs = state.handshake_action();
                    let frame = match &hs {
                        GatewayAction::Resume { session_id, seq } => build_resume(token, session_id, *seq),
                        _ => build_identify(token),
                    };
                    if write.send(WsMessage::Text(frame.to_string().into())).await.is_err() {
                        return Err(());
                    }
                    handshaken = true;
                    continue;
                }
                let resumable = if payload.op == 9 { payload.d.as_bool() } else { None };
                let action = state.on_payload(payload.op, payload.t.as_deref(), payload.s, resumable);
                match action {
                    GatewayAction::SendHeartbeat => {
                        if write.send(WsMessage::Text(build_heartbeat(state.seq).to_string().into())).await.is_err() {
                            return Err(());
                        }
                    }
                    GatewayAction::ReconnectResumable | GatewayAction::ReconnectFull => return Err(()),
                    GatewayAction::Dispatch(t) => dispatch_event(&t, &payload.d, state, event_tx),
                    _ => {}
                }
                let _ = handshaken; // marqueur de progression
            }
        }
    }
}

fn dispatch_event(t: &str, d: &Value, state: &mut GatewayState, tx: &UnboundedSender<Event>) {
    match t {
        "READY" => {
            if let Some(sid) = d.get("session_id").and_then(Value::as_str) {
                state.set_session(sid.to_string());
            }
            let user: Option<User> = d.get("user").and_then(|u| serde_json::from_value(u.clone()).ok());
            let guilds: Vec<Guild> = d.get("guilds")
                .and_then(|g| serde_json::from_value(g.clone()).ok())
                .unwrap_or_default();
            if let Some(user) = user {
                let _ = tx.send(Event::Connection(ConnectionState::Connected));
                let _ = tx.send(Event::Ready { user, guilds });
            }
        }
        "MESSAGE_CREATE" => {
            if let Ok(m) = serde_json::from_value::<Message>(d.clone()) {
                let _ = tx.send(Event::MessageCreated(m));
            }
        }
        "MESSAGE_UPDATE" => {
            if let Ok(m) = serde_json::from_value::<Message>(d.clone()) {
                let _ = tx.send(Event::MessageUpdated(m));
            }
        }
        "MESSAGE_DELETE" => {
            if let (Some(id), Some(cid)) = (
                d.get("id").and_then(Value::as_str),
                d.get("channel_id").and_then(Value::as_str),
            ) {
                let _ = tx.send(Event::MessageDeleted { id: id.into(), channel_id: cid.into() });
            }
        }
        _ => {}
    }
}
```

- [ ] **Step 4 : Exporter dans lib.rs**

```rust
pub mod gateway;
pub use gateway::run_gateway;
```

- [ ] **Step 5 : Vérifier**

Run: `cargo test -p veloce-discord gateway::tests && cargo clippy -p veloce-discord -- -D warnings`
Expected: PASS (1 test), clippy clean.

- [ ] **Step 6 : Commit**

```bash
git add -A
git commit -m "feat(discord): connexion gateway WebSocket auto-reconnectante"
```

---

### Task 8 : Pont réseau (thread tokio ⇄ UI)

**Files:**
- Create: `crates/veloce-app/src/net.rs`

**Interfaces:**
- Consumes: `veloce_discord::{run_gateway, RestClient, Command, Event}`.
- Produces:
  - `struct NetHandle { pub events: std::sync::mpsc::Receiver<Event>, cmd_tx: tokio::sync::mpsc::UnboundedSender<Command>, _shutdown: tokio::sync::watch::Sender<bool> }`.
  - `fn spawn_net(token: String, ctx: egui::Context) -> NetHandle` : démarre un runtime tokio sur un thread dédié, lance `run_gateway` + une boucle de traitement des `Command` via REST, et transfère tous les `Event` vers un `std::sync::mpsc` (lisible par l'UI), en appelant `ctx.request_repaint()` à chaque event.
  - `impl NetHandle { pub fn send(&self, cmd: Command) }`.

**Note :** I/O et threading — pas de test unitaire ; vérification par compilation puis run manuel en Task 10.

- [ ] **Step 1 : Implémenter net.rs**

```rust
use egui::Context;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio::sync::watch;
use veloce_discord::{run_gateway, Command, Event, RestClient};

pub struct NetHandle {
    pub events: Receiver<Event>,
    cmd_tx: UnboundedSender<Command>,
    _shutdown: watch::Sender<bool>,
}

impl NetHandle {
    pub fn send(&self, cmd: Command) {
        let _ = self.cmd_tx.send(cmd);
    }
}

pub fn spawn_net(token: String, ctx: Context) -> NetHandle {
    let (event_out, events): (Sender<Event>, Receiver<Event>) = channel();
    let (cmd_tx, mut cmd_rx) = unbounded_channel::<Command>();
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime tokio");
        rt.block_on(async move {
            // canal interne gateway -> relais
            let (gw_tx, mut gw_rx) = unbounded_channel::<Event>();

            // tâche gateway
            let gw_token = token.clone();
            let gw_shutdown = shutdown_rx.clone();
            tokio::spawn(async move { run_gateway(gw_token, gw_tx, gw_shutdown).await });

            // client REST pour les commandes
            let rest = match RestClient::new(token.clone()) {
                Ok(r) => r,
                Err(e) => {
                    let _ = event_out.send(Event::Error(format!("REST: {e}")));
                    return;
                }
            };

            loop {
                tokio::select! {
                    Some(ev) = gw_rx.recv() => {
                        if event_out.send(ev).is_err() { break; }
                        ctx.request_repaint();
                    }
                    Some(cmd) = cmd_rx.recv() => {
                        handle_command(&rest, cmd, &event_out, &ctx).await;
                    }
                    else => break,
                }
            }
        });
    });

    NetHandle { events, cmd_tx, _shutdown: shutdown_tx }
}

async fn handle_command(rest: &RestClient, cmd: Command, out: &Sender<Event>, ctx: &Context) {
    let result: Result<Event, String> = match cmd {
        Command::SelectGuild(guild_id) => rest
            .guild_channels(&guild_id)
            .await
            .map(|channels| Event::ChannelsLoaded { guild_id, channels })
            .map_err(|e| e.to_string()),
        Command::FetchHistory(channel_id) => rest
            .channel_messages(&channel_id, 50)
            .await
            .map(|mut messages| {
                messages.reverse(); // l'API renvoie du plus récent au plus ancien
                Event::MessagesLoaded { channel_id, messages }
            })
            .map_err(|e| e.to_string()),
        Command::SendMessage { channel_id, content } => rest
            .send_message(&channel_id, &content)
            .await
            .map(Event::MessageCreated)
            .map_err(|e| e.to_string()),
    };
    let ev = result.unwrap_or_else(|e| Event::Error(e));
    let _ = out.send(ev);
    ctx.request_repaint();
}
```

- [ ] **Step 2 : Déclarer le module** — dans `crates/veloce-app/src/main.rs`, ajouter `mod net;` (l'usage viendra en Task 10).

- [ ] **Step 3 : Vérifier**

Run: `cargo build -p veloce-app`
Expected: compile (warnings d'items inutilisés tolérés jusqu'à la Task 10).

- [ ] **Step 4 : Commit**

```bash
git add -A
git commit -m "feat(app): pont réseau thread tokio <-> UI via canaux"
```

---

### Task 9 : Parseur markdown (pur)

**Files:**
- Create: `crates/veloce-app/src/markdown.rs`

**Interfaces:**
- Consumes: rien (la conversion vers `LayoutJob` viendra en Task 10).
- Produces:
  - `struct Span { pub text: String, pub bold: bool, pub italic: bool, pub strike: bool, pub code: bool }` (derive Debug, Clone, PartialEq).
  - `fn parse_markdown(input: &str) -> Vec<Span>` : gère `**gras**`, `*italique*`, `~~barré~~`, `` `code` ``. (Blocs de code, liens, mentions : itération ultérieure.) Le texte sans marqueur produit un `Span` neutre.

- [ ] **Step 1 : Écrire les tests (échouent)** — `markdown.rs` :

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn neutre(t: &str) -> Span {
        Span { text: t.into(), bold: false, italic: false, strike: false, code: false }
    }

    #[test]
    fn texte_simple_un_span_neutre() {
        assert_eq!(parse_markdown("bonjour"), vec![neutre("bonjour")]);
    }

    #[test]
    fn gras_detecte() {
        let r = parse_markdown("a **b** c");
        assert_eq!(r[0], neutre("a "));
        assert_eq!(r[1], Span { text: "b".into(), bold: true, italic: false, strike: false, code: false });
        assert_eq!(r[2], neutre(" c"));
    }

    #[test]
    fn italique_et_code() {
        let r = parse_markdown("*i* `c`");
        assert!(r.iter().any(|s| s.text == "i" && s.italic));
        assert!(r.iter().any(|s| s.text == "c" && s.code));
    }

    #[test]
    fn barre_detecte() {
        let r = parse_markdown("~~x~~");
        assert_eq!(r, vec![Span { text: "x".into(), bold: false, italic: false, strike: true, code: false }]);
    }

    #[test]
    fn marqueur_non_ferme_reste_litteral() {
        assert_eq!(parse_markdown("**oops"), vec![neutre("**oops")]);
    }
}
```

- [ ] **Step 2 : Vérifier l'échec**

Run: `cargo test -p veloce-app markdown`
Expected: FAIL (`parse_markdown` introuvable).

- [ ] **Step 3 : Implémenter markdown.rs**

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub strike: bool,
    pub code: bool,
}

impl Span {
    fn neutral(text: String) -> Self {
        Span { text, bold: false, italic: false, strike: false, code: false }
    }
}

/// Marqueurs reconnus (ordre = priorité de détection).
const MARKERS: &[(&str, fn(&mut Span))] = &[
    ("**", |s| s.bold = true),
    ("~~", |s| s.strike = true),
    ("`", |s| s.code = true),
    ("*", |s| s.italic = true),
];

pub fn parse_markdown(input: &str) -> Vec<Span> {
    let mut spans = Vec::new();
    let mut buf = String::new();
    let mut rest = input;

    'outer: while !rest.is_empty() {
        for (marker, apply) in MARKERS {
            if let Some(after_open) = rest.strip_prefix(marker) {
                if let Some(end) = after_open.find(marker) {
                    let content = &after_open[..end];
                    if !content.is_empty() {
                        if !buf.is_empty() {
                            spans.push(Span::neutral(std::mem::take(&mut buf)));
                        }
                        let mut span = Span::neutral(content.to_string());
                        apply(&mut span);
                        spans.push(span);
                        rest = &after_open[end + marker.len()..];
                        continue 'outer;
                    }
                }
            }
        }
        // aucun marqueur : consommer un caractère
        let ch = rest.chars().next().unwrap();
        buf.push(ch);
        rest = &rest[ch.len_utf8()..];
    }
    if !buf.is_empty() {
        spans.push(Span::neutral(buf));
    }
    spans
}
```

- [ ] **Step 4 : Déclarer le module** — dans `main.rs`, ajouter `mod markdown;`.

- [ ] **Step 5 : Vérifier le succès**

Run: `cargo test -p veloce-app markdown`
Expected: PASS (5 tests).

- [ ] **Step 6 : Commit**

```bash
git add -A
git commit -m "feat(app): parseur markdown basique (gras/italique/barré/code)"
```

---

### Task 10 : Application egui (UI + token + câblage)

**Files:**
- Create: `crates/veloce-app/src/app.rs`
- Modify: `crates/veloce-app/src/main.rs`

**Interfaces:**
- Consumes: `net::{spawn_net, NetHandle}`, `markdown::{parse_markdown, Span}`, `veloce_discord::{Command, Event, ConnectionState}` et modèles, `keyring`, `eframe`/`egui`.
- Produces: binaire `veloce` exécutable (UI 3 panneaux fonctionnelle).

**Comportement :** au lancement, lire le token depuis le trousseau (`keyring`, service `"veloce"`, user `"token"`). Absent → écran de saisie ; à la validation, stocker dans le trousseau et appeler `spawn_net`. Connecté → 3 panneaux. Drain des `Event` à chaque frame (`try_recv` en boucle).

- [ ] **Step 1 : Écrire un test pur de tri des salons** — dans `app.rs` (logique extraite pour rester testable) :

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use veloce_discord::Channel;

    fn ch(id: &str, kind: u8, pos: i32) -> Channel {
        Channel { id: id.into(), name: Some(id.into()), kind, guild_id: None, position: Some(pos) }
    }

    #[test]
    fn ne_garde_que_les_salons_texte_tries_par_position() {
        let input = vec![ch("b", 0, 2), ch("voc", 2, 0), ch("a", 0, 1)];
        let out = text_channels_sorted(input);
        let ids: Vec<_> = out.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, vec!["a", "b"]);
    }
}
```

- [ ] **Step 2 : Vérifier l'échec**

Run: `cargo test -p veloce-app app`
Expected: FAIL (`text_channels_sorted` introuvable).

- [ ] **Step 3 : Implémenter app.rs** (état + helper testé + UI)

```rust
use crate::markdown::{parse_markdown, Span};
use crate::net::{spawn_net, NetHandle};
use eframe::egui;
use egui::{text::LayoutJob, Color32, FontId, RichText, TextFormat};
use veloce_discord::{Channel, Command, ConnectionState, Event, Guild, Message, User};

const KEYRING_SERVICE: &str = "veloce";
const KEYRING_USER: &str = "token";

/// Ne conserve que les salons texte (type 0), triés par position.
pub fn text_channels_sorted(mut channels: Vec<Channel>) -> Vec<Channel> {
    channels.retain(|c| c.kind == 0);
    channels.sort_by_key(|c| c.position.unwrap_or(0));
    channels
}

#[derive(Default)]
struct ChatState {
    user: Option<User>,
    guilds: Vec<Guild>,
    channels: Vec<Channel>,
    messages: Vec<Message>,
    selected_guild: Option<String>,
    selected_channel: Option<String>,
    connection: Option<ConnectionState>,
    draft: String,
}

enum Screen {
    Token { input: String, error: Option<String> },
    Chat { net: NetHandle, state: ChatState },
}

pub struct VeloceApp {
    screen: Screen,
    /// Token lu au démarrage depuis le trousseau ; déclenche la connexion auto au 1er `update`.
    pending_token: Option<String>,
}

impl VeloceApp {
    pub fn new() -> Self {
        Self {
            screen: Screen::Token { input: String::new(), error: None },
            pending_token: keyring_get(),
        }
    }

    fn connect(&mut self, token: String, ctx: &egui::Context) {
        keyring_set(&token);
        let net = spawn_net(token, ctx.clone());
        self.screen = Screen::Chat { net, state: ChatState::default() };
    }
}

fn keyring_get() -> Option<String> {
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER).ok()?.get_password().ok()
}

fn keyring_set(token: &str) {
    if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER) {
        let _ = entry.set_password(token);
    }
}

fn spans_to_job(spans: &[Span]) -> LayoutJob {
    let mut job = LayoutJob::default();
    for s in spans {
        let mut fmt = TextFormat { font_id: FontId::proportional(14.0), ..Default::default() };
        if s.code {
            fmt.font_id = FontId::monospace(13.0);
            fmt.background = Color32::from_gray(40);
        }
        if s.bold {
            fmt.color = Color32::WHITE;
        }
        if s.italic {
            fmt.italics = true;
        }
        if s.strike {
            fmt.strikethrough = egui::Stroke::new(1.0, Color32::GRAY);
        }
        job.append(&s.text, 0.0, fmt);
    }
    job
}

impl eframe::App for VeloceApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Connexion auto si un token est en attente.
        if let Some(token) = self.pending_token.take() {
            self.connect(token, ctx);
        }

        match &mut self.screen {
            Screen::Token { input, error } => {
                let mut submit: Option<String> = None;
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.heading("Veloce");
                    ui.label("Colle ton token Discord :");
                    ui.add(egui::TextEdit::singleline(input).password(true).desired_width(400.0));
                    if let Some(e) = error {
                        ui.colored_label(Color32::LIGHT_RED, e.as_str());
                    }
                    if ui.button("Se connecter").clicked() && !input.trim().is_empty() {
                        submit = Some(input.trim().to_string());
                    }
                });
                if let Some(token) = submit {
                    self.connect(token, ctx);
                }
            }
            Screen::Chat { net, state } => {
                // Drain des events.
                while let Ok(ev) = net.events.try_recv() {
                    apply_event(state, ev);
                }
                draw_chat(ctx, net, state);
            }
        }
    }
}

fn apply_event(state: &mut ChatState, ev: Event) {
    match ev {
        Event::Connection(c) => state.connection = Some(c),
        Event::Ready { user, guilds } => {
            state.user = Some(user);
            state.guilds = guilds;
        }
        Event::ChannelsLoaded { guild_id, channels } => {
            if Some(&guild_id) == state.selected_guild.as_ref() {
                state.channels = text_channels_sorted(channels);
            }
        }
        Event::MessagesLoaded { channel_id, messages } => {
            if Some(&channel_id) == state.selected_channel.as_ref() {
                state.messages = messages;
            }
        }
        Event::MessageCreated(m) => {
            if Some(&m.channel_id) == state.selected_channel.as_ref() {
                state.messages.push(m);
            }
        }
        Event::MessageUpdated(m) => {
            if let Some(existing) = state.messages.iter_mut().find(|x| x.id == m.id) {
                *existing = m;
            }
        }
        Event::MessageDeleted { id, .. } => state.messages.retain(|m| m.id != id),
        Event::Error(e) => tracing::warn!("erreur réseau: {e}"),
    }
}

fn draw_chat(ctx: &egui::Context, net: &NetHandle, state: &mut ChatState) {
    egui::SidePanel::left("guilds").exact_width(180.0).show(ctx, |ui| {
        ui.heading("Serveurs");
        let status = match &state.connection {
            Some(ConnectionState::Connected) => "● connecté",
            Some(ConnectionState::Reconnecting) => "○ reconnexion…",
            Some(ConnectionState::Connecting) => "○ connexion…",
            _ => "○ hors ligne",
        };
        ui.label(status);
        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| {
            for g in state.guilds.clone() {
                if ui.selectable_label(state.selected_guild.as_ref() == Some(&g.id), &g.name).clicked() {
                    state.selected_guild = Some(g.id.clone());
                    state.channels.clear();
                    net.send(Command::SelectGuild(g.id));
                }
            }
        });
    });

    egui::SidePanel::left("channels").exact_width(200.0).show(ctx, |ui| {
        ui.heading("Salons");
        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| {
            for c in state.channels.clone() {
                let name = c.name.clone().unwrap_or_else(|| c.id.clone());
                if ui.selectable_label(state.selected_channel.as_ref() == Some(&c.id), format!("# {name}")).clicked() {
                    state.selected_channel = Some(c.id.clone());
                    state.messages.clear();
                    net.send(Command::FetchHistory(c.id));
                }
            }
        });
    });

    egui::TopBottomPanel::bottom("composer").show(ctx, |ui| {
        let enabled = state.selected_channel.is_some();
        ui.add_enabled_ui(enabled, |ui| {
            let resp = ui.add(egui::TextEdit::singleline(&mut state.draft).desired_width(f32::INFINITY).hint_text("Message…"));
            let send = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if send && !state.draft.trim().is_empty() {
                if let Some(cid) = state.selected_channel.clone() {
                    net.send(Command::SendMessage { channel_id: cid, content: state.draft.trim().to_string() });
                    state.draft.clear();
                }
                resp.request_focus();
            }
        });
    });

    egui::CentralPanel::default().show(ctx, |ui| {
        egui::ScrollArea::vertical().stick_to_bottom(true).show(ui, |ui| {
            for m in &state.messages {
                ui.horizontal_wrapped(|ui| {
                    let name = m.author.global_name.clone().unwrap_or_else(|| m.author.username.clone());
                    ui.label(RichText::new(format!("{name}: ")).strong().color(Color32::LIGHT_BLUE));
                    ui.label(spans_to_job(&parse_markdown(&m.content)));
                });
            }
        });
    });
}
```

- [ ] **Step 4 : Réécrire main.rs**

```rust
mod app;
mod markdown;
mod net;

use app::VeloceApp;

fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt().with_env_filter("veloce=info,warn").init();
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Veloce",
        options,
        Box::new(|_cc| Ok(Box::new(VeloceApp::new()))),
    )
}
```

- [ ] **Step 5 : Vérifier la compilation et les tests**

Run: `cargo build -p veloce-app && cargo test -p veloce-app && cargo clippy --all-targets -- -D warnings`
Expected: build OK, tests PASS (markdown + app), clippy clean.

- [ ] **Step 6 : Vérification manuelle (avec compte secondaire)**

Run: `cargo run --bin veloce`
Expected : écran de saisie du token → après validation, liste des serveurs apparaît → clic sur un serveur affiche les salons → clic sur un salon texte affiche l'historique → un message envoyé depuis un autre client apparaît en temps réel → un message tapé dans Veloce part et s'affiche. Vérifier que l'app au repos ne consomme ~aucun CPU.

- [ ] **Step 7 : Commit**

```bash
git add -A
git commit -m "feat(app): UI egui 3 panneaux, auth token (keyring), câblage complet"
```

---

## Self-Review (effectuée)

**1. Couverture de la spec :**
- Auth token → Tasks 6 (REST headers) + 10 (saisie/keyring). ✅
- Gateway (HELLO/IDENTIFY/heartbeat/READY/RESUME/INVALID_SESSION) → Tasks 5 (états) + 7 (I/O). ✅
- REST (5 endpoints) → Task 6. ✅
- Liste serveurs/salons → Tasks 7 (READY guilds) + 6/8/10. ✅
- Lecture temps réel + envoi → Tasks 7/8/10. ✅
- Markdown basique → Task 9 (parse) + 10 (rendu LayoutJob). ✅
- Reconnexion/backoff → Task 7. ✅
- Rate limit retry-after → Task 6. ✅
- Token sécurisé (keyring) → Task 10. ✅
- État de connexion visible → Task 10 (panneau gauche). ✅
- Couture plugins (Event/Command publics) → Task 3. ✅
- Repo/CI/licence → Task 1. ✅
- Tests (modèles, états, markdown, retry-after) → Tasks 2/5/9/6. ✅

**2. Placeholders :** aucun « TBD/TODO » fonctionnel. Le LICENSE renvoie au texte officiel GPL-3.0 (contenu connu, pas un placeholder de code).

**3. Cohérence des types :** `Snowflake = String` partout ; `Channel.kind` (pas `type`) cohérent Tasks 2/10 ; `Event`/`Command` identiques Tasks 3/7/8/10 ; `Span` identique Tasks 9/10 ; `text_channels_sorted` défini et utilisé Task 10 ; `parse_retry_after_ms` défini/testé Task 6.

**Note de granularité :** Task 10 est la plus grosse (UI + câblage). Son seul fragment testé unitairement est `text_channels_sorted` (Step 1) ; le reste (rendu egui, keyring, boucle d'events) est vérifié par compilation + run manuel (Steps 5-6), ce qui est attendu pour de l'I/O et de l'UI.
