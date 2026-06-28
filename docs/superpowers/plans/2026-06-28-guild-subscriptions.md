# Veloce — Plan d'implémentation : abonnements de guilde (op 14)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Permettre le temps réel sur les gros serveurs en envoyant une frame gateway op 14 (« lazy guild request ») quand l'utilisateur sélectionne un serveur, avec re-souscription après reconnexion.

**Architecture:** Ajouter une **entrée** vers la tâche gateway : un canal `GatewayCommand` (app → `run_gateway`) qui contourne la boucle REST. La gateway maintient l'ensemble des guilds abonnés, envoie op 14 à la demande (si connectée) et ré-émet op 14 à chaque READY. La construction de la frame est isolée dans une fonction pure.

**Tech Stack:** Rust 2021, tokio (mpsc/watch), tokio-tungstenite, serde_json, egui (côté app).

## Global Constraints

- API Discord **v10**. La frame op 14 est de l'**API user non documentée** : isolée dans `build_guild_subscribe` (unique point de maintenance) ; son efficacité réelle est un **critère de vérification manuelle** (gros serveur), pas un test CI.
- Frame op 14 (format plat) : `{ "op": 14, "d": { "guild_id": <gid>, "typing": true, "activities": true, "threads": false, "channels": {} } }`.
- N'émettre op 14 que si la session est établie (`hb_started`) ; sinon mémoriser dans le set et émettre au READY. **Re-souscrire tous les guilds du set à chaque READY** (Discord oublie à chaque nouvelle session).
- L'enum `Command` REST et `handle_command` restent **inchangés** (op 14 ne passe pas par le REST).
- `veloce-discord` ne dépend jamais d'egui.
- **Task 1** change la signature de `run_gateway`, ce qui casse l'appel dans `veloce-app` jusqu'à la Task 2. Donc Task 1 se vérifie **au niveau du package** `-p veloce-discord` (lib auto-cohérente, clippy-clean) ; le build/clippy **du workspace entier** est rétabli en Task 2.
- Édition 2021. Messages de commit en français, style `type: description`.

---

## Structure des fichiers

```
crates/veloce-discord/src/
├─ gateway.rs   # Task 1 : GatewayCommand, build_guild_subscribe, run_gateway/connect_once
└─ lib.rs       # Task 1 : export GatewayCommand
crates/veloce-app/src/
├─ net.rs       # Task 2 : canal gateway, NetHandle.subscribe_guild
└─ app.rs       # Task 2 : appel subscribe_guild au clic serveur
```

---

### Task 1 : `veloce-discord` — GatewayCommand, frame op 14, intégration gateway

**Files:**
- Modify: `crates/veloce-discord/src/gateway.rs`
- Modify: `crates/veloce-discord/src/lib.rs`

**Interfaces:**
- Consumes: `crate::models::Snowflake`, `crate::gateway_state` (inchangé), tokio mpsc.
- Produces:
  - `enum GatewayCommand { SubscribeGuild(Snowflake) }` (derive `Debug, Clone`).
  - `pub fn build_guild_subscribe(guild_id: &str) -> serde_json::Value`.
  - `run_gateway(token: String, event_tx: UnboundedSender<Event>, shutdown: watch::Receiver<bool>, gw_cmd_rx: UnboundedReceiver<GatewayCommand>)` — **nouveau 4e paramètre**.

**Note (build) :** la signature de `run_gateway` change → `veloce-app` ne compilera plus tant que la Task 2 n'a pas mis à jour l'appel. Vérifier UNIQUEMENT `-p veloce-discord` ici.

- [ ] **Step 1 : Écrire le test (échoue)** — dans le `mod tests` de `crates/veloce-discord/src/gateway.rs`, ajouter :

```rust
    #[test]
    fn guild_subscribe_op14() {
        let v = build_guild_subscribe("123");
        assert_eq!(v["op"], 14);
        assert_eq!(v["d"]["guild_id"], "123");
        assert_eq!(v["d"]["typing"], true);
        assert!(v["d"]["activities"].is_boolean());
        assert!(v["d"]["threads"].is_boolean());
        assert!(v["d"]["channels"].is_object());
    }
```

- [ ] **Step 2 : Vérifier l'échec**

Run: `cargo test -p veloce-discord gateway::tests::guild_subscribe_op14`
Expected: FAIL (`build_guild_subscribe` introuvable).

- [ ] **Step 3 : Ajouter l'enum + le builder** — dans `gateway.rs`, après les `use` et `build_heartbeat`, ajouter :

```rust
use std::collections::HashSet;
use tokio::sync::mpsc::UnboundedReceiver;

/// Commande adressée à la tâche gateway (canal app → gateway, distinct du REST).
#[derive(Debug, Clone)]
pub enum GatewayCommand {
    SubscribeGuild(crate::models::Snowflake),
}

/// Frame op 14 (« lazy guild request ») pour s'abonner aux events d'une guilde.
/// API user non documentée — unique point de maintenance si le format change.
pub fn build_guild_subscribe(guild_id: &str) -> Value {
    json!({
        "op": 14,
        "d": {
            "guild_id": guild_id,
            "typing": true,
            "activities": true,
            "threads": false,
            "channels": {}
        }
    })
}
```

*(L'import existant de `UnboundedSender` reste ; ajouter `UnboundedReceiver`. Si l'`use` actuel est `use tokio::sync::mpsc::UnboundedSender;`, le remplacer par `use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};` et supprimer la ligne d'import dupliquée que ce Step 3 propose pour `UnboundedReceiver`.)*

- [ ] **Step 4 : Modifier `run_gateway`** — nouvelle signature + set d'abonnements passé à `connect_once` :

```rust
pub async fn run_gateway(
    token: String,
    event_tx: UnboundedSender<Event>,
    mut shutdown: watch::Receiver<bool>,
    mut gw_cmd_rx: UnboundedReceiver<GatewayCommand>,
) {
    let mut state = GatewayState::default();
    let mut backoff_ms = 1000u64;
    let mut subscribed: HashSet<crate::models::Snowflake> = HashSet::new();
    loop {
        if *shutdown.borrow() {
            return;
        }
        let _ = event_tx.send(Event::Connection(ConnectionState::Connecting));
        match connect_once(
            &token,
            &mut state,
            &event_tx,
            &mut shutdown,
            &mut backoff_ms,
            &mut gw_cmd_rx,
            &mut subscribed,
        )
        .await
        {
            Ok(()) => return,
            Err(()) => {
                let _ = event_tx.send(Event::Connection(ConnectionState::Reconnecting));
                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_millis(backoff_ms)) => {}
                    _ = shutdown.changed() => return,
                }
                backoff_ms = next_backoff(backoff_ms);
            }
        }
    }
}
```

- [ ] **Step 5 : Modifier `connect_once`** — nouvelle signature, garde anti-busy-loop, branche `select!`, re-souscription au READY.

Signature :

```rust
async fn connect_once(
    token: &str,
    state: &mut GatewayState,
    event_tx: &UnboundedSender<Event>,
    shutdown: &mut watch::Receiver<bool>,
    backoff: &mut u64,
    gw_cmd_rx: &mut UnboundedReceiver<GatewayCommand>,
    subscribed: &mut HashSet<crate::models::Snowflake>,
) -> std::result::Result<(), ()> {
```

Après `let mut hb_started = false;`, ajouter la garde locale :

```rust
    let mut gw_cmd_open = true;
```

Dans le `tokio::select! { ... }`, ajouter cette branche (à côté des branches `shutdown`, `hb.tick`, `read.next`) :

```rust
            cmd = gw_cmd_rx.recv(), if gw_cmd_open => {
                match cmd {
                    Some(GatewayCommand::SubscribeGuild(gid)) => {
                        subscribed.insert(gid.clone());
                        if hb_started
                            && write
                                .send(WsMessage::Text(
                                    build_guild_subscribe(&gid).to_string().into(),
                                ))
                                .await
                                .is_err()
                        {
                            return Err(());
                        }
                    }
                    None => gw_cmd_open = false, // émetteur lâché : ne plus interroger ce canal
                }
            }
```

Dans le bras `GatewayAction::Dispatch(t)`, ré-émettre op 14 pour tous les guilds abonnés après un READY :

```rust
                    GatewayAction::Dispatch(t) => {
                        dispatch_event(&t, &payload.d, state, event_tx);
                        if t == "READY" {
                            for gid in subscribed.iter() {
                                if write
                                    .send(WsMessage::Text(
                                        build_guild_subscribe(gid).to_string().into(),
                                    ))
                                    .await
                                    .is_err()
                                {
                                    return Err(());
                                }
                            }
                        }
                    }
```

- [ ] **Step 6 : Exporter dans `lib.rs`** — modifier la ligne d'export de `gateway` pour inclure `GatewayCommand` :

```rust
pub use gateway::{run_gateway, GatewayCommand};
```

*(Si `build_guild_subscribe` n'est pas déjà visible des tests, il l'est : le `mod tests` est dans `gateway.rs`. Pas besoin de l'exporter du crate.)*

- [ ] **Step 7 : Vérifier (package veloce-discord uniquement)**

Run:
```bash
cargo test -p veloce-discord
cargo build -p veloce-discord
cargo clippy -p veloce-discord --all-targets -- -D warnings
cargo fmt --all
```
Expected: tests PASS (dont `guild_subscribe_op14` + les anciens du gateway), build du lib OK, clippy 0 warning sur `veloce-discord`, fmt propre. **NE PAS** lancer `cargo build`/`clippy` sur tout le workspace (veloce-app est cassé jusqu'à la Task 2).

- [ ] **Step 8 : Commit**

```bash
git add -A
git commit -m "feat(discord): GatewayCommand + frame op 14 (abonnement de guilde)"
```

---

### Task 2 : `veloce-app` — plomberie du canal gateway + câblage UI

**Files:**
- Modify: `crates/veloce-app/src/net.rs`
- Modify: `crates/veloce-app/src/app.rs`

**Interfaces:**
- Consumes: `veloce_discord::{run_gateway, GatewayCommand, Command, Event, RestClient}`.
- Produces: `NetHandle::subscribe_guild(&self, guild_id: String)`.

**Note :** cette tâche rétablit le build/clippy du **workspace entier**. Gate complet : `cargo test --all`, `cargo build`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --all` — tous propres.

- [ ] **Step 1 : Mettre à jour `net.rs`** — importer `GatewayCommand`, créer le canal, le passer à `run_gateway`, l'exposer via `NetHandle`.

Modifier l'import :

```rust
use veloce_discord::{run_gateway, Command, Event, GatewayCommand, RestClient};
```

Ajouter le champ au `NetHandle` et la méthode :

```rust
pub struct NetHandle {
    pub events: Receiver<Event>,
    cmd_tx: UnboundedSender<Command>,
    gw_cmd_tx: UnboundedSender<GatewayCommand>,
    _shutdown: watch::Sender<bool>,
}

impl NetHandle {
    pub fn send(&self, cmd: Command) {
        let _ = self.cmd_tx.send(cmd);
    }

    /// Demande au gateway de s'abonner aux events d'une guilde (frame op 14).
    pub fn subscribe_guild(&self, guild_id: String) {
        let _ = self.gw_cmd_tx.send(GatewayCommand::SubscribeGuild(guild_id));
    }
}
```

Dans `spawn_net`, créer le canal gateway, le passer à `run_gateway`, et stocker l'émetteur dans `NetHandle` :

```rust
    let (cmd_tx, mut cmd_rx) = unbounded_channel::<Command>();
    let (gw_cmd_tx, gw_cmd_rx) = unbounded_channel::<GatewayCommand>();
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
```

```rust
            // tâche gateway
            let gw_token = token.clone();
            let gw_shutdown = shutdown_rx.clone();
            tokio::spawn(async move { run_gateway(gw_token, gw_tx, gw_shutdown, gw_cmd_rx).await });
```

```rust
    NetHandle {
        events,
        cmd_tx,
        gw_cmd_tx,
        _shutdown: shutdown_tx,
    }
```

*(`gw_cmd_rx` est déplacé dans la tâche `run_gateway` ; `gw_cmd_tx` est stocké dans `NetHandle`. Comme `gw_cmd_rx` est créé hors du `thread::spawn` puis `move` dans la closure, vérifier qu'il est bien capturé par la closure du thread — il l'est, comme `cmd_rx`/`shutdown_rx`.)*

- [ ] **Step 2 : Câbler l'UI dans `app.rs`** — au clic sur un serveur, s'abonner en plus de sélectionner.

Dans `draw_chat`, le bloc de sélection de guilde devient :

```rust
                    if ui
                        .selectable_label(state.selected_guild.as_ref() == Some(&g.id), &g.name)
                        .clicked()
                    {
                        state.selected_guild = Some(g.id.clone());
                        state.channels.clear();
                        net.subscribe_guild(g.id.clone());
                        net.send(Command::SelectGuild(g.id));
                    }
```

- [ ] **Step 3 : Gates complets (workspace entier)**

Run:
```bash
cargo test --all
cargo build
cargo clippy --all-targets -- -D warnings
cargo fmt --all
```
Expected: tous les tests PASS (inchangés + `guild_subscribe_op14`), build OK, **clippy 0 warning sur tout le workspace**, fmt propre.

- [ ] **Step 4 : Vérification manuelle (utilisateur, gros serveur conseillé)**

Run: `cargo run --bin veloce`
Expected : sélectionner un serveur n'a pas de régression sur les petits serveurs ; sur un **gros** serveur, les nouveaux messages arrivent désormais en temps réel. (L'envoi de la frame op 14 est observable en activant les logs réseau / un proxy si besoin.)

- [ ] **Step 5 : Commit**

```bash
git add -A
git commit -m "feat(app): canal gateway + abonnement de guilde au clic (op 14)"
```

---

## Self-Review (effectuée)

**1. Couverture de la spec :**
- Canal app→gateway + `GatewayCommand::SubscribeGuild` → Task 1 (enum, run_gateway param) + Task 2 (net). ✅
- `build_guild_subscribe` (pur, isolé) → Task 1, testé. ✅
- Branche `select!` + envoi si `hb_started` → Task 1 Step 5. ✅
- Re-souscription au READY → Task 1 Step 5 (bras Dispatch). ✅
- `NetHandle.subscribe_guild` → Task 2 Step 1. ✅
- Câblage UI au clic serveur → Task 2 Step 2. ✅
- `Command` REST / `handle_command` inchangés → aucune tâche ne les touche. ✅
- Anti-busy-loop sur émetteur lâché → garde `gw_cmd_open` (Task 1 Step 5). ✅
- Tests : `build_guild_subscribe` (Task 1) ; effet réel = critère manuel (Task 2 Step 4). ✅

**2. Placeholders :** aucun « TBD/TODO ». Les notes sur les imports (Step 3) et la capture de `gw_cmd_rx` (Task 2 Step 1) sont des précisions d'intégration, pas des placeholders de code.

**3. Cohérence des types :** `GatewayCommand::SubscribeGuild(Snowflake)` identique Tasks 1/2 ; `run_gateway` 4 paramètres cohérent (Task 1 def, Task 2 appel) ; `subscribe_guild(&self, String)` ↔ `Snowflake` (= `String`) cohérent ; `build_guild_subscribe(&str)` utilisé en Task 1 (branche + READY) ; `gw_cmd_rx`/`gw_cmd_tx` types `UnboundedReceiver/Sender<GatewayCommand>` cohérents.
