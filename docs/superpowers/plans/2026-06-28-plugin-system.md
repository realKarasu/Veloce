# Veloce — Plan d'implémentation : système de plugins v1

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Doter Veloce d'un système de plugins statiques (trait Rust, compilés dans le binaire, activables à l'exécution), avec persistance, fenêtre de réglages egui, et 3 plugins d'exemple.

**Architecture:** Un module `plugins/` dans `veloce-app` : un trait `Plugin` object-safe (hooks à impl par défaut), un `PluginManager` qui oriente les hooks vers les plugins activés et persiste l'état activé en JSON. L'app appelle le manager aux trois points clés : drain d'events, envoi de message, rendu de message. `veloce-discord` reste inchangé.

**Tech Stack:** Rust 2021, egui/eframe, serde_json, `directories` (config dir), `veloce_discord::Event`.

## Global Constraints

- `veloce-discord` **inchangé** — il ne doit jamais dépendre d'egui/eframe. Tout le code plugins vit dans `veloce-app`.
- Le trait `Plugin` est **object-safe** (`Box<dyn Plugin>`) : méthodes `&self`/`&mut self`, pas de génériques, hooks avec impl par défaut.
- **Règle d'or :** un plugin **désactivé** ne reçoit **aucun** hook.
- `set_enabled` est **pur** (met à jour la map en mémoire, **pas d'I/O**) ; la persistance se fait via `save()` (explicite) appelé par l'app après un toggle.
- `PluginManager::new()` est **vide et sans I/O** (hermétique pour les tests) ; `PluginManager::builtin()` charge l'état persistant puis enregistre les plugins intégrés.
- Ordre d'application : les hooks `apply_outgoing`/`apply_render` chaînent les plugins activés **dans l'ordre d'enregistrement**.
- Dépendance nouvelle (plancher, épingler la dernière compatible) : `directories = "5"` (veloce-app uniquement).
- **Tasks 1-3** produisent du code consommé seulement en Task 4 : `cargo build` + les **tests unitaires** sont la barrière ; les warnings `dead_code` sont **attendus** et tolérés ; ne PAS lancer `clippy -D warnings` comme gate avant la Task 4. La Task 4 rend **tout le workspace** clippy-clean.
- Édition 2021. Messages de commit en français, style `type: description`.

---

## Structure des fichiers

```
crates/veloce-app/src/
├─ plugins/
│  ├─ mod.rs              # Task 1 (trait + manager core), Task 2 (persistance), Task 4 (builtin + settings_ui)
│  ├─ text_replace.rs     # Task 3
│  ├─ message_counter.rs  # Task 3
│  └─ loud.rs             # Task 3
├─ app.rs                 # Task 4 (câblage)
├─ main.rs                # Task 1 (`mod plugins;`)
└─ Cargo.toml             # Task 2 (`directories`)
```

---

### Task 1 : Trait `Plugin` + `PluginManager` (cœur, sans persistance)

**Files:**
- Create: `crates/veloce-app/src/plugins/mod.rs`
- Modify: `crates/veloce-app/src/main.rs` (ajouter `mod plugins;`)

**Interfaces:**
- Consumes: `veloce_discord::Event`, `eframe::egui`.
- Produces:
  - `trait Plugin { fn name(&self) -> &str; fn description(&self) -> &str; fn default_enabled(&self) -> bool {false}; fn on_event(&mut self, &Event) {}; fn on_outgoing_message(&mut self, &mut String) {}; fn on_render_content(&self, &mut String) {}; fn settings_ui(&mut self, &mut egui::Ui) {} }`
  - `struct PluginManager { plugins: Vec<Box<dyn Plugin>>, enabled: HashMap<String, bool> }` (derive `Default`).
  - `PluginManager::new() -> Self` (vide) ; `register(&mut self, Box<dyn Plugin>)` ; `is_enabled(&self, &str) -> bool` ; `set_enabled(&mut self, &str, bool)` (pur) ; `dispatch_event(&mut self, &Event)` ; `apply_outgoing(&mut self, &mut String)` ; `apply_render(&self, &mut String)`.

**Note (dead_code) :** ce module est consommé en Task 4 → `cargo build` aura des warnings `dead_code` ; c'est attendu. Gate = `cargo test -p veloce-app plugins` + `cargo build`. Ne pas lancer `clippy -D warnings`.

- [ ] **Step 1 : Écrire les tests (échouent)** — dans `crates/veloce-app/src/plugins/mod.rs` :

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;
    use veloce_discord::Event;

    struct TestPlugin {
        nm: String,
        on: bool,
        tag: String,
        events: Rc<Cell<u32>>,
    }
    impl TestPlugin {
        fn new(nm: &str, on: bool, tag: &str, events: Rc<Cell<u32>>) -> Self {
            Self { nm: nm.into(), on, tag: tag.into(), events }
        }
    }
    impl Plugin for TestPlugin {
        fn name(&self) -> &str { &self.nm }
        fn description(&self) -> &str { "t" }
        fn default_enabled(&self) -> bool { self.on }
        fn on_event(&mut self, _e: &Event) { self.events.set(self.events.get() + 1); }
        fn on_outgoing_message(&mut self, c: &mut String) { c.push_str(&self.tag); }
        fn on_render_content(&self, c: &mut String) { c.push_str(&self.tag); }
    }

    #[test]
    fn register_applique_default_enabled() {
        let mut m = PluginManager::new();
        m.register(Box::new(TestPlugin::new("A", true, "", Rc::new(Cell::new(0)))));
        m.register(Box::new(TestPlugin::new("B", false, "", Rc::new(Cell::new(0)))));
        assert!(m.is_enabled("A"));
        assert!(!m.is_enabled("B"));
    }

    #[test]
    fn dispatch_event_seulement_si_active() {
        let c = Rc::new(Cell::new(0));
        let mut m = PluginManager::new();
        m.register(Box::new(TestPlugin::new("A", true, "", c.clone())));
        m.dispatch_event(&Event::Error("x".into()));
        assert_eq!(c.get(), 1);
        m.set_enabled("A", false);
        m.dispatch_event(&Event::Error("x".into()));
        assert_eq!(c.get(), 1); // désactivé → pas d'appel
    }

    #[test]
    fn apply_outgoing_chaine_dans_l_ordre_et_ignore_desactives() {
        let mut m = PluginManager::new();
        m.register(Box::new(TestPlugin::new("A", true, "a", Rc::new(Cell::new(0)))));
        m.register(Box::new(TestPlugin::new("B", true, "b", Rc::new(Cell::new(0)))));
        let mut s = String::new();
        m.apply_outgoing(&mut s);
        assert_eq!(s, "ab");
        m.set_enabled("A", false);
        let mut s2 = String::new();
        m.apply_outgoing(&mut s2);
        assert_eq!(s2, "b");
    }

    #[test]
    fn apply_render_chaine_les_actives() {
        let mut m = PluginManager::new();
        m.register(Box::new(TestPlugin::new("A", true, "!", Rc::new(Cell::new(0)))));
        let mut s = "hi".to_string();
        m.apply_render(&mut s);
        assert_eq!(s, "hi!");
    }
}
```

- [ ] **Step 2 : Vérifier l'échec**

Run: `cargo test -p veloce-app plugins`
Expected: FAIL (`Plugin`/`PluginManager` introuvables).

- [ ] **Step 3 : Implémenter le haut de `mod.rs`**

```rust
use std::collections::HashMap;
use veloce_discord::Event;

pub trait Plugin {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn default_enabled(&self) -> bool {
        false
    }
    /// Observe les events gateway/REST (lecture seule).
    fn on_event(&mut self, _event: &Event) {}
    /// Transforme un message AVANT envoi.
    fn on_outgoing_message(&mut self, _content: &mut String) {}
    /// Transforme le contenu AFFICHÉ avant le rendu markdown.
    fn on_render_content(&self, _content: &mut String) {}
    /// Réglages du plugin (egui). Optionnel.
    fn settings_ui(&mut self, _ui: &mut eframe::egui::Ui) {}
}

#[derive(Default)]
pub struct PluginManager {
    plugins: Vec<Box<dyn Plugin>>,
    enabled: HashMap<String, bool>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Enregistre un plugin. Si son état activé n'est pas déjà connu, applique `default_enabled()`.
    pub fn register(&mut self, plugin: Box<dyn Plugin>) {
        let name = plugin.name().to_string();
        let default = plugin.default_enabled();
        self.enabled.entry(name).or_insert(default);
        self.plugins.push(plugin);
    }

    pub fn is_enabled(&self, name: &str) -> bool {
        self.enabled.get(name).copied().unwrap_or(false)
    }

    /// Pur : met à jour la map en mémoire (pas d'I/O ; la persistance se fait via `save`).
    pub fn set_enabled(&mut self, name: &str, enabled: bool) {
        self.enabled.insert(name.to_string(), enabled);
    }

    pub fn dispatch_event(&mut self, event: &Event) {
        for p in &mut self.plugins {
            if self.enabled.get(p.name()).copied().unwrap_or(false) {
                p.on_event(event);
            }
        }
    }

    pub fn apply_outgoing(&mut self, content: &mut String) {
        for p in &mut self.plugins {
            if self.enabled.get(p.name()).copied().unwrap_or(false) {
                p.on_outgoing_message(content);
            }
        }
    }

    pub fn apply_render(&self, content: &mut String) {
        for p in &self.plugins {
            if self.enabled.get(p.name()).copied().unwrap_or(false) {
                p.on_render_content(content);
            }
        }
    }
}
```

*(Les boucles lisent `self.enabled` tout en itérant `&mut self.plugins` — emprunts de champs disjoints, accepté par le borrow checker.)*

- [ ] **Step 4 : Déclarer le module** — dans `crates/veloce-app/src/main.rs`, ajouter `mod plugins;` (avec les autres `mod`).

- [ ] **Step 5 : Vérifier le succès**

Run: `cargo test -p veloce-app plugins`
Expected: PASS (4 tests). `cargo build -p veloce-app` compile (warnings dead_code tolérés).

- [ ] **Step 6 : Commit**

```bash
git add -A
git commit -m "feat(plugins): trait Plugin + PluginManager (cœur, hooks activables)"
```

---

### Task 2 : Persistance de l'état activé

**Files:**
- Modify: `crates/veloce-app/src/plugins/mod.rs`
- Modify: `crates/veloce-app/Cargo.toml` (ajouter `directories`)

**Interfaces:**
- Consumes: `PluginManager` (Task 1), `serde_json`, `directories`.
- Produces: `PluginManager::load_persisted(&mut self)` (remplace `self.enabled` par l'état lu sur disque) ; `PluginManager::save(&self)` (écrit le JSON) ; helper pur `parse_enabled(&str) -> HashMap<String, bool>`.

**Note (dead_code) :** toujours consommé en Task 4. Gate = `cargo test -p veloce-app plugins` + `cargo build`.

- [ ] **Step 1 : Ajouter la dépendance** — dans `crates/veloce-app/Cargo.toml`, section `[dependencies]`, ajouter :

```toml
directories = "5"
```

- [ ] **Step 2 : Écrire le test du parseur (échoue)** — ajouter dans le `mod tests` de `mod.rs` :

```rust
    #[test]
    fn parse_enabled_valide_et_invalide() {
        let m = parse_enabled(r#"{"A":true,"B":false}"#);
        assert_eq!(m.get("A"), Some(&true));
        assert_eq!(m.get("B"), Some(&false));
        // JSON invalide → map vide (pas de panique).
        assert!(parse_enabled("pas du json").is_empty());
    }
```

- [ ] **Step 3 : Vérifier l'échec**

Run: `cargo test -p veloce-app plugins::tests::parse_enabled_valide_et_invalide`
Expected: FAIL (`parse_enabled` introuvable).

- [ ] **Step 4 : Implémenter la persistance** — ajouter dans `mod.rs` (hors du bloc `impl` ou dans un nouveau `impl PluginManager`) :

```rust
use std::path::PathBuf;

/// Désérialise l'état activé ; JSON invalide → map vide.
fn parse_enabled(s: &str) -> HashMap<String, bool> {
    serde_json::from_str(s).unwrap_or_default()
}

fn config_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "veloce")
        .map(|d| d.config_dir().join("plugins.json"))
}

impl PluginManager {
    /// Charge l'état activé persistant depuis le disque (remplace la map en mémoire).
    /// À appeler AVANT `register` pour que l'état persistant l'emporte sur `default_enabled`.
    pub fn load_persisted(&mut self) {
        if let Some(s) = config_path().and_then(|p| std::fs::read_to_string(p).ok()) {
            self.enabled = parse_enabled(&s);
        }
    }

    /// Écrit l'état activé sur disque (échecs d'I/O tolérés silencieusement).
    pub fn save(&self) {
        let Some(path) = config_path() else { return };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(&self.enabled) {
            let _ = std::fs::write(path, json);
        }
    }
}
```

- [ ] **Step 5 : Vérifier le succès**

Run: `cargo test -p veloce-app plugins`
Expected: PASS (5 tests). `cargo build -p veloce-app` compile (warnings dead_code tolérés).

- [ ] **Step 6 : Commit**

```bash
git add -A
git commit -m "feat(plugins): persistance JSON de l'état activé (directories)"
```

---

### Task 3 : Plugins d'exemple (TextReplace, MessageCounter, Loud)

**Files:**
- Create: `crates/veloce-app/src/plugins/text_replace.rs`
- Create: `crates/veloce-app/src/plugins/message_counter.rs`
- Create: `crates/veloce-app/src/plugins/loud.rs`
- Modify: `crates/veloce-app/src/plugins/mod.rs` (déclarer les sous-modules)

**Interfaces:**
- Consumes: `crate::plugins::Plugin`, `veloce_discord::Event`, `veloce_discord::Message`, `eframe::egui`.
- Produces: `TextReplace` (+ `pub fn apply_rules(&str, &[(String,String)]) -> String`), `MessageCounter` (champ `count: u64`), `Loud` — tous `impl Plugin`, tous `Default`.

**Note (dead_code) :** enregistrés en Task 4. Gate = `cargo test -p veloce-app plugins` + `cargo build`.

- [ ] **Step 1 : Déclarer les sous-modules** — dans `mod.rs`, en haut, ajouter :

```rust
pub mod loud;
pub mod message_counter;
pub mod text_replace;
```

- [ ] **Step 2 : Écrire les tests (échouent)** — créer les 3 fichiers avec leur module de test d'abord, OU écrire chaque fichier complet puis vérifier. Tests à inclure :

Dans `text_replace.rs` :

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_rules_remplace_plusieurs_et_ignore_vide() {
        let rules = vec![
            ("foo".to_string(), "bar".to_string()),
            (String::new(), "X".to_string()), // règle vide → ignorée
        ];
        assert_eq!(apply_rules("foo foo baz", &rules), "bar bar baz");
        assert_eq!(apply_rules("rien", &rules), "rien");
    }
}
```

Dans `message_counter.rs` :

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use veloce_discord::{Message, User};

    fn msg() -> Message {
        Message {
            id: "1".into(),
            channel_id: "c".into(),
            content: String::new(),
            author: User { id: "u".into(), username: "u".into(), global_name: None, discriminator: None },
            timestamp: None,
        }
    }

    #[test]
    fn compte_seulement_les_message_created() {
        let mut p = MessageCounter::default();
        p.on_event(&veloce_discord::Event::Error("x".into()));
        assert_eq!(p.count, 0);
        p.on_event(&veloce_discord::Event::MessageCreated(msg()));
        assert_eq!(p.count, 1);
    }
}
```

Dans `loud.rs` :

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn met_en_majuscules() {
        let p = Loud;
        let mut s = "hello".to_string();
        p.on_render_content(&mut s);
        assert_eq!(s, "HELLO");
    }
}
```

- [ ] **Step 3 : Vérifier l'échec**

Run: `cargo test -p veloce-app plugins`
Expected: FAIL (types des plugins introuvables).

- [ ] **Step 4 : Implémenter `text_replace.rs`**

```rust
use crate::plugins::Plugin;
use eframe::egui;

/// Applique les règles find→replace dans l'ordre (règles à `from` vide ignorées).
pub fn apply_rules(content: &str, rules: &[(String, String)]) -> String {
    let mut out = content.to_string();
    for (from, to) in rules {
        if !from.is_empty() {
            out = out.replace(from.as_str(), to);
        }
    }
    out
}

pub struct TextReplace {
    rules: Vec<(String, String)>,
}

impl Default for TextReplace {
    fn default() -> Self {
        Self {
            rules: vec![("(shrug)".to_string(), "¯\\_(ツ)_/¯".to_string())],
        }
    }
}

impl Plugin for TextReplace {
    fn name(&self) -> &str {
        "TextReplace"
    }
    fn description(&self) -> &str {
        "Remplace du texte dans les messages envoyés."
    }
    fn on_outgoing_message(&mut self, content: &mut String) {
        *content = apply_rules(content, &self.rules);
    }
    fn settings_ui(&mut self, ui: &mut egui::Ui) {
        ui.label("Règles (texte → remplacement) :");
        let mut to_remove = None;
        for (i, (from, to)) in self.rules.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                ui.text_edit_singleline(from);
                ui.label("→");
                ui.text_edit_singleline(to);
                if ui.button("✕").clicked() {
                    to_remove = Some(i);
                }
            });
        }
        if let Some(i) = to_remove {
            self.rules.remove(i);
        }
        if ui.button("+ Règle").clicked() {
            self.rules.push((String::new(), String::new()));
        }
    }
}
```

- [ ] **Step 5 : Implémenter `message_counter.rs`**

```rust
use crate::plugins::Plugin;
use eframe::egui;
use veloce_discord::Event;

#[derive(Default)]
pub struct MessageCounter {
    count: u64,
}

impl Plugin for MessageCounter {
    fn name(&self) -> &str {
        "MessageCounter"
    }
    fn description(&self) -> &str {
        "Compte les messages reçus pendant la session."
    }
    fn on_event(&mut self, event: &Event) {
        if let Event::MessageCreated(_) = event {
            self.count += 1;
        }
    }
    fn settings_ui(&mut self, ui: &mut egui::Ui) {
        ui.label(format!("Messages vus : {}", self.count));
    }
}
```

- [ ] **Step 6 : Implémenter `loud.rs`**

```rust
use crate::plugins::Plugin;

#[derive(Default)]
pub struct Loud;

impl Plugin for Loud {
    fn name(&self) -> &str {
        "Loud"
    }
    fn description(&self) -> &str {
        "MET LES MESSAGES AFFICHÉS EN MAJUSCULES."
    }
    fn on_render_content(&self, content: &mut String) {
        *content = content.to_uppercase();
    }
}
```

- [ ] **Step 7 : Vérifier le succès**

Run: `cargo test -p veloce-app plugins`
Expected: PASS (8 tests au total). `cargo build -p veloce-app` compile (warnings dead_code tolérés).

- [ ] **Step 8 : Commit**

```bash
git add -A
git commit -m "feat(plugins): plugins d'exemple TextReplace, MessageCounter, Loud"
```

---

### Task 4 : Intégration — `builtin()`, câblage, fenêtre Plugins (workspace clippy-clean)

**Files:**
- Modify: `crates/veloce-app/src/plugins/mod.rs` (ajouter `builtin()` + `settings_ui()`)
- Modify: `crates/veloce-app/src/app.rs` (champs, hooks, fenêtre)

**Interfaces:**
- Consumes: `PluginManager`, `TextReplace`, `MessageCounter`, `Loud` (Tasks 1-3).
- Produces: `PluginManager::builtin() -> Self` ; `PluginManager::settings_ui(&mut self, &mut egui::Ui)`. `VeloceApp` gagne `plugins: PluginManager` et `show_plugins: bool`.

**Note :** c'est la tâche d'intégration ; elle consomme tout le code des Tasks 1-3 (plus de dead_code). Gate complet : `cargo test --all`, `cargo build`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --all` — **tous propres**. Le code egui ci-dessous peut nécessiter de petits ajustements d'API/emprunts ; les faire en préservant le comportement (toggle + persistance + réglages par plugin + les 3 hooks câblés). Ne PAS lancer la GUI (vérif manuelle = étape utilisateur).

- [ ] **Step 1 : Ajouter `builtin()` et `settings_ui()`** — dans `crates/veloce-app/src/plugins/mod.rs`, ajouter les ré-exports en haut et les méthodes :

```rust
pub use loud::Loud;
pub use message_counter::MessageCounter;
pub use text_replace::TextReplace;
```

```rust
impl PluginManager {
    /// Manager prêt à l'emploi : état persistant chargé puis plugins intégrés enregistrés.
    pub fn builtin() -> Self {
        let mut m = Self::new();
        m.load_persisted();
        m.register(Box::new(TextReplace::default()));
        m.register(Box::new(MessageCounter::default()));
        m.register(Box::new(Loud::default()));
        m
    }

    /// UI de la fenêtre Plugins : pour chaque plugin, une case activé + ses réglages.
    pub fn settings_ui(&mut self, ui: &mut eframe::egui::Ui) {
        use eframe::egui;
        let mut changed = false;
        for p in &mut self.plugins {
            let name = p.name().to_string();
            let mut on = self.enabled.get(&name).copied().unwrap_or(false);
            ui.horizontal(|ui| {
                if ui.checkbox(&mut on, &name).changed() {
                    self.enabled.insert(name.clone(), on);
                    changed = true;
                }
                ui.weak(p.description());
            });
            if on {
                egui::CollapsingHeader::new(format!("Réglages — {name}"))
                    .id_salt(&name)
                    .show(ui, |ui| p.settings_ui(ui));
            }
            ui.separator();
        }
        if changed {
            self.save();
        }
    }
}
```

*(Emprunts disjoints : `&mut self.plugins` (boucle) et `self.enabled` (closure) sont des champs distincts → accepté en édition 2021. Si le borrow checker résiste, collecter les `(name, on)` à basculer dans un `Vec` puis les appliquer après la boucle.)*

- [ ] **Step 2 : Câbler dans `app.rs` — champs et constructeur**

Ajouter `use crate::plugins::PluginManager;` en haut. Modifier la struct et `new` :

```rust
pub struct VeloceApp {
    screen: Screen,
    pending_token: Option<String>,
    plugins: PluginManager,
    show_plugins: bool,
}

impl VeloceApp {
    pub fn new() -> Self {
        Self {
            screen: Screen::Token {
                input: String::new(),
                error: None,
            },
            pending_token: keyring_get(),
            plugins: PluginManager::builtin(),
            show_plugins: false,
        }
    }
    // `connect` inchangé
}
```

- [ ] **Step 3 : Câbler le drain d'events** — dans `update`, l'arm `Screen::Chat`, appeler `dispatch_event` avant `apply_event`, et passer `plugins`/`show_plugins` à `draw_chat` :

```rust
            Screen::Chat { net, state } => {
                while let Ok(ev) = net.events.try_recv() {
                    self.plugins.dispatch_event(&ev);
                    if let Event::AuthFailed(msg) = ev {
                        auth_failed = Some(msg);
                        continue;
                    }
                    apply_event(state, ev);
                }
                draw_chat(
                    ctx,
                    net,
                    state.as_mut(),
                    &mut self.plugins,
                    &mut self.show_plugins,
                );
            }
```

*(`self.plugins` et `self.show_plugins` sont des champs disjoints de `self.screen` (emprunté via `net`/`state`) → accepté.)*

- [ ] **Step 4 : Mettre à jour la signature et le corps de `draw_chat`**

Nouvelle signature :

```rust
fn draw_chat(
    ctx: &egui::Context,
    net: &NetHandle,
    state: &mut ChatState,
    plugins: &mut crate::plugins::PluginManager,
    show_plugins: &mut bool,
) {
```

Dans le panneau « guilds », après `ui.heading("Serveurs");`, ajouter le bouton d'ouverture :

```rust
            if ui.button("⚙ Plugins").clicked() {
                *show_plugins = !*show_plugins;
            }
```

Dans le composer, appliquer `apply_outgoing` avant l'envoi :

```rust
            if send && !state.draft.trim().is_empty() {
                if let Some(cid) = state.selected_channel.clone() {
                    let mut content = state.draft.trim().to_string();
                    plugins.apply_outgoing(&mut content);
                    net.send(Command::SendMessage {
                        channel_id: cid,
                        content,
                    });
                    state.draft.clear();
                }
                resp.request_focus();
            }
```

Dans le `CentralPanel` de rendu des messages, appliquer `apply_render` avant `parse_markdown` :

```rust
                for m in &state.messages {
                    ui.horizontal_wrapped(|ui| {
                        let name = m
                            .author
                            .global_name
                            .clone()
                            .unwrap_or_else(|| m.author.username.clone());
                        ui.label(
                            RichText::new(format!("{name}: "))
                                .strong()
                                .color(Color32::LIGHT_BLUE),
                        );
                        let mut content = m.content.clone();
                        plugins.apply_render(&mut content);
                        ui.label(spans_to_job(&parse_markdown(&content)));
                    });
                }
```

À la fin de `draw_chat`, afficher la fenêtre Plugins. `egui::Window::open` prend
un `&mut bool` pour gérer la croix de fermeture ; on utilise un booléen local
puis on répercute la fermeture dans `*show_plugins` :

```rust
    if *show_plugins {
        let mut open = true;
        egui::Window::new("Plugins")
            .open(&mut open)
            .show(ctx, |ui| plugins.settings_ui(ui));
        if !open {
            *show_plugins = false;
        }
    }
```

- [ ] **Step 5 : Gates complets**

Run:
```bash
cargo test --all
cargo build
cargo clippy --all-targets -- -D warnings
cargo fmt --all
```
Expected: tests PASS (anciens + plugins), build OK, **clippy 0 warning sur tout le workspace**, fmt propre.

- [ ] **Step 6 : Vérification manuelle (utilisateur)**

Run: `cargo run --bin veloce`
Expected : bouton « ⚙ Plugins » ouvre la fenêtre ; activer **Loud** met les messages affichés en MAJUSCULES ; **TextReplace** activé remplace `(shrug)` à l'envoi ; **MessageCounter** incrémente ; l'état activé persiste après redémarrage.

- [ ] **Step 7 : Commit**

```bash
git add -A
git commit -m "feat(plugins): builtin(), fenêtre Plugins et câblage (events/envoi/rendu)"
```

---

## Self-Review (effectuée)

**1. Couverture de la spec :**
- Trait `Plugin` (hooks + défaut + object-safe) → Task 1. ✅
- `PluginManager` (registre, dispatch/apply, is/set_enabled) → Task 1. ✅
- Règle d'or « désactivé = aucun hook » → testée Task 1. ✅
- Persistance JSON (directories, load/save, parse) → Task 2. ✅
- 3 plugins d'exemple (TextReplace/MessageCounter/Loud) → Task 3. ✅
- `builtin()` + câblage (dispatch_event / apply_outgoing / apply_render) → Task 4. ✅
- Fenêtre Plugins (toggle + settings_ui + persist on toggle) → Task 4. ✅
- `veloce-discord` inchangé → aucune tâche n'y touche. ✅
- Tests pure (manager, apply_rules, counter, loud, parse_enabled) → Tasks 1/2/3. ✅

**2. Placeholders :** aucun « TBD/TODO ». La fenêtre Plugins utilise le motif `let mut open = true;` + `.open(&mut open)` (egui exige un `&mut bool` pour la croix de fermeture).

**3. Cohérence des types :** `Plugin`/`PluginManager` identiques Tasks 1-4 ; `apply_render` en `&self`, `dispatch_event`/`apply_outgoing` en `&mut self` cohérents ; `set_enabled` pur + `save()` explicite cohérents Tasks 1/2/4 ; `builtin()` enregistre exactement les 3 plugins de Task 3 ; signatures `draw_chat` mises à jour de façon cohérente (Task 4 Steps 3-4).
