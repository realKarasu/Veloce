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

#[cfg(test)]
mod tests {
    use super::*;
    use veloce_discord::{Message, User};

    fn msg() -> Message {
        Message {
            id: "1".into(),
            channel_id: "c".into(),
            content: String::new(),
            author: User {
                id: "u".into(),
                username: "u".into(),
                global_name: None,
                discriminator: None,
            },
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
