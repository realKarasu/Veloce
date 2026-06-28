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
