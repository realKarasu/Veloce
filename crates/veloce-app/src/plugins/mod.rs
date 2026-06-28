pub mod loud;
pub mod message_counter;
pub mod text_replace;

pub use loud::Loud;
pub use message_counter::MessageCounter;
pub use text_replace::TextReplace;

use std::collections::HashMap;
use std::path::PathBuf;
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
            if self.is_enabled(p.name()) {
                p.on_render_content(content);
            }
        }
    }
}

/// Désérialise l'état activé ; JSON invalide → map vide.
fn parse_enabled(s: &str) -> HashMap<String, bool> {
    serde_json::from_str(s).unwrap_or_default()
}

fn config_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "veloce").map(|d| d.config_dir().join("plugins.json"))
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

    /// Manager prêt à l'emploi : état persistant chargé puis plugins intégrés enregistrés.
    pub fn builtin() -> Self {
        let mut m = Self::new();
        m.load_persisted();
        m.register(Box::new(TextReplace::default()));
        m.register(Box::new(MessageCounter::default()));
        m.register(Box::new(Loud));
        m
    }

    /// UI de la fenêtre Plugins : pour chaque plugin, une case activé + ses réglages.
    pub fn settings_ui(&mut self, ui: &mut eframe::egui::Ui) {
        use eframe::egui;
        let mut toggles: Vec<(String, bool)> = Vec::new();
        for p in &mut self.plugins {
            let name = p.name().to_string();
            let mut on = self.enabled.get(&name).copied().unwrap_or(false);
            ui.horizontal(|ui| {
                if ui.checkbox(&mut on, &name).changed() {
                    toggles.push((name.clone(), on));
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
        let changed = !toggles.is_empty();
        for (name, on) in toggles {
            self.set_enabled(&name, on);
        }
        if changed {
            self.save();
        }
    }
}

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
            Self {
                nm: nm.into(),
                on,
                tag: tag.into(),
                events,
            }
        }
    }
    impl Plugin for TestPlugin {
        fn name(&self) -> &str {
            &self.nm
        }
        fn description(&self) -> &str {
            "t"
        }
        fn default_enabled(&self) -> bool {
            self.on
        }
        fn on_event(&mut self, _e: &Event) {
            self.events.set(self.events.get() + 1);
        }
        fn on_outgoing_message(&mut self, c: &mut String) {
            c.push_str(&self.tag);
        }
        fn on_render_content(&self, c: &mut String) {
            c.push_str(&self.tag);
        }
    }

    #[test]
    fn register_applique_default_enabled() {
        let mut m = PluginManager::new();
        m.register(Box::new(TestPlugin::new(
            "A",
            true,
            "",
            Rc::new(Cell::new(0)),
        )));
        m.register(Box::new(TestPlugin::new(
            "B",
            false,
            "",
            Rc::new(Cell::new(0)),
        )));
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
        m.register(Box::new(TestPlugin::new(
            "A",
            true,
            "a",
            Rc::new(Cell::new(0)),
        )));
        m.register(Box::new(TestPlugin::new(
            "B",
            true,
            "b",
            Rc::new(Cell::new(0)),
        )));
        let mut s = String::new();
        m.apply_outgoing(&mut s);
        assert_eq!(s, "ab");
        m.set_enabled("A", false);
        let mut s2 = String::new();
        m.apply_outgoing(&mut s2);
        assert_eq!(s2, "b");
    }

    #[test]
    fn parse_enabled_valide_et_invalide() {
        let m = parse_enabled(r#"{"A":true,"B":false}"#);
        assert_eq!(m.get("A"), Some(&true));
        assert_eq!(m.get("B"), Some(&false));
        // JSON invalide → map vide (pas de panique).
        assert!(parse_enabled("pas du json").is_empty());
    }

    #[test]
    fn apply_render_chaine_les_actives() {
        let mut m = PluginManager::new();
        m.register(Box::new(TestPlugin::new(
            "A",
            true,
            "!",
            Rc::new(Cell::new(0)),
        )));
        let mut s = "hi".to_string();
        m.apply_render(&mut s);
        assert_eq!(s, "hi!");
    }
}
