# Veloce — Spec de conception : système de plugins v1

**Date :** 2026-06-28
**Statut :** Validé (design approuvé)
**Périmètre :** système de plugins statiques (trait Rust) pour `veloce-app`.

---

## 1. Vision

Donner à Veloce son identité « Vencord » : l'**extensibilité**. Les plugins
sont des modules Rust implémentant un trait `Plugin`, **compilés dans le
binaire** et **activables/désactivables à l'exécution** par l'utilisateur — le
modèle réel de Vencord (les plugins font partie du code, l'utilisateur choisit
ceux qu'il active). Mécanisme statique = zéro overhead, 100% Rust, type-safe,
fidèle à l'objectif « rapide et léger ».

Le seam est déjà posé par la v0.1 : les types publics `Event`/`Command` de
`veloce-discord`. Cette spec construit la couche plugins au-dessus, dans
`veloce-app`.

## 2. Périmètre

### Inclus
- Trait `Plugin` (object-safe, hooks à impl par défaut).
- `PluginManager` : registre, orchestration des hooks, activation/désactivation.
- Persistance de l'ensemble activé (fichier JSON dans le dossier config OS).
- Fenêtre egui « Plugins » : liste, toggle, réglages par plugin.
- Câblage dans l'app : events, transformation à l'envoi, transformation à
  l'affichage.
- 3 plugins d'exemple : `TextReplace`, `MessageCounter`, `Loud`.

### Hors périmètre (plus tard)
- Injection de `Command` par les plugins ; filtrage/suppression d'events.
- Thèmes (CSS-like) ; chargement dynamique (WASM/.so) — le trait en est la base.
- Abonnements de guilde gateway `op 14` → **sous-projet suivant** (spec/plan
  séparés).

### Critères de succès
1. Au moins 3 plugins listés dans la fenêtre Plugins, chacun activable.
2. `TextReplace` activé : un message envoyé voit ses règles find→replace
   appliquées.
3. `Loud` activé : les messages affichés passent en MAJUSCULES ; désactivé :
   affichage normal.
4. `MessageCounter` activé : le compteur augmente à chaque message reçu.
5. L'état activé/désactivé **persiste** après redémarrage.
6. Un plugin désactivé n'a **aucun** effet (aucun hook appelé).

## 3. Architecture

```
crates/veloce-app/src/
├─ plugins/
│  ├─ mod.rs              # trait Plugin + PluginManager + persistance
│  ├─ text_replace.rs     # exemple : on_outgoing_message + settings_ui
│  ├─ message_counter.rs  # exemple : on_event + settings_ui
│  └─ loud.rs             # exemple : on_render_content
├─ app.rs                 # câblage (manager, fenêtre, hooks)
└─ main.rs                # `mod plugins;`
```

`veloce-discord` **inchangé** (reste UI-agnostique).

### Frontières des unités
- **`Plugin` (trait)** — *Quoi :* contrat d'extension. *Dépend de :*
  `veloce_discord::Event`, `egui::Ui` (réglages). *Object-safe.*
- **`PluginManager`** — *Quoi :* détient les plugins + l'état activé, oriente
  les hooks vers les plugins activés, persiste l'état. *Dépend de :* le trait,
  `serde_json`, `directories`.
- **Plugins d'exemple** — *Quoi :* une fonctionnalité chacun. *Dépendent de :*
  le trait (+ `egui` pour les réglages).
- **`app.rs`** — *Quoi :* appelle le manager aux bons points (drain d'events,
  envoi, rendu) et affiche la fenêtre Plugins.

## 4. Le trait `Plugin`

```rust
use veloce_discord::Event;

pub trait Plugin {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn default_enabled(&self) -> bool { false }

    /// Observe les events gateway/REST (lecture seule).
    fn on_event(&mut self, _event: &Event) {}
    /// Transforme un message AVANT envoi (find/replace, etc.).
    fn on_outgoing_message(&mut self, _content: &mut String) {}
    /// Transforme le contenu AFFICHÉ avant le rendu markdown.
    fn on_render_content(&self, _content: &mut String) {}
    /// Réglages du plugin (egui). Optionnel.
    fn settings_ui(&mut self, _ui: &mut egui::Ui) {}
}
```

Hooks : observer / transformer-sortie / transformer-affichage + réglages.
Couvre les patterns Vencord usuels. `on_render_content` est `&self`
(lecture) pour être appelable pendant le rendu.

## 5. `PluginManager`

```rust
pub struct PluginManager {
    plugins: Vec<Box<dyn Plugin>>,
    enabled: std::collections::HashMap<String, bool>, // name -> activé
}
```

- `new() -> Self` : enregistre les plugins intégrés, charge l'état persistant
  (à défaut, `default_enabled()` de chaque plugin).
- `dispatch_event(&mut self, event: &Event)` : `on_event` pour chaque plugin
  activé.
- `apply_outgoing(&mut self, content: &mut String)` : chaîne
  `on_outgoing_message` des activés, dans l'ordre d'enregistrement.
- `apply_render(&self, content: &mut String)` : chaîne `on_render_content` des
  activés.
- `is_enabled(&self, name) -> bool`, `set_enabled(&mut self, name, bool)` (puis
  `save()`).
- `settings_ui(&mut self, ui)` : pour chaque plugin, une case « activé » + un
  repli (`CollapsingHeader`) appelant `settings_ui`.
- `save()` / `load()` : JSON dans `directories::ProjectDirs` (`veloce`,
  fichier `plugins.json`).

**Règle d'or :** un plugin désactivé ne reçoit aucun hook (vérifié par test).

## 6. Câblage dans `app.rs`

- `VeloceApp` gagne `plugins: PluginManager` (global, persiste entre connexions)
  et `show_plugins: bool` (fenêtre ouverte ?).
- `update()` : lors du drain des events de `Screen::Chat`, appeler
  `self.plugins.dispatch_event(&ev)` (en plus de `apply_event`). *(Destructurer
  `self` pour emprunter `plugins` et `screen` séparément.)*
- `draw_chat(..., plugins: &mut PluginManager)` :
  - **Envoi** : avant `net.send(SendMessage { content })`, faire
    `plugins.apply_outgoing(&mut content)`.
  - **Rendu** : `let mut c = m.content.clone(); plugins.apply_render(&mut c);`
    puis `parse_markdown(&c)`.
- **Fenêtre Plugins** : bouton « ⚙ Plugins » dans le panneau serveurs →
  bascule `show_plugins` → `egui::Window::new("Plugins")` affichant
  `plugins.settings_ui(ui)`.

## 7. Plugins d'exemple

1. **TextReplace** (`on_outgoing_message` + `settings_ui`)
   - État : `Vec<(String, String)>` de règles find→replace (au moins une règle
     éditable dans les réglages).
   - `on_outgoing_message` : applique chaque règle au contenu sortant.
   - Logique pure testable : `apply_rules(content, rules) -> String`.

2. **MessageCounter** (`on_event` + `settings_ui`)
   - État : `count: u64`.
   - `on_event` : `if let Event::MessageCreated(_) = event { self.count += 1 }`.
   - `settings_ui` : affiche « Messages vus : N ».

3. **Loud** (`on_render_content`)
   - `on_render_content` : `*content = content.to_uppercase()`.
   - Démo cosmétique (esprit Vencord), prouve le hook d'affichage.

## 8. Persistance

`directories::ProjectDirs::from("", "", "veloce")` → `config_dir()` →
`plugins.json` : un objet `{ "<name>": <bool> }`. Échecs d'I/O tolérés
silencieusement (le plugin system marche sans persistance, on retombe sur
`default_enabled`). Nouvelle dépendance : `directories = "5"` (veloce-app).

## 9. Tests (TDD)

Parties pures (sans egui) :
- `PluginManager` : `dispatch_event`/`apply_outgoing`/`apply_render` ne touchent
  que les plugins **activés** ; `apply_outgoing` chaîne dans l'ordre ;
  `set_enabled` bascule ; (dé)sérialisation de l'enabled set. Testé via un
  plugin de test (settings_ui no-op).
- `TextReplace::apply_rules` (find/replace, plusieurs règles, règle vide).
- `Loud` : `on_render_content` met en majuscules.
- `MessageCounter` : `on_event(MessageCreated)` incrémente ; les autres events
  non.

Les `settings_ui` (egui) et la fenêtre : vérifiés par build + clippy + run
manuel.

## 10. Risques

| Risque | Mitigation |
|---|---|
| Emprunts egui (manager + state simultanés) | Destructurer `self` ; `apply_render` en `&self` |
| `settings_ui` non testable (egui Ui) | Isoler la logique pure (apply_rules, count) et la tester ; UI au build |
| Coût du clone de contenu au rendu | Borné par le repaint-à-la-demande ; contenu = une String par message visible |
| Persistance indisponible | Échecs tolérés, repli sur `default_enabled` |
