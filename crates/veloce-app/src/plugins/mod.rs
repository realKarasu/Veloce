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
