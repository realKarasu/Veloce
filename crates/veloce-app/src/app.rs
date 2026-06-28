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

/// True si le message doit être ajouté (pas déjà présent par id).
pub fn should_append_message(existing: &[Message], incoming: &Message) -> bool {
    !existing.iter().any(|m| m.id == incoming.id)
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
    last_error: Option<String>,
}

enum Screen {
    Token {
        input: String,
        error: Option<String>,
    },
    Chat {
        net: NetHandle,
        state: Box<ChatState>,
    },
}

pub struct VeloceApp {
    screen: Screen,
    /// Token lu au démarrage depuis le trousseau ; déclenche la connexion auto au 1er `update`.
    pending_token: Option<String>,
}

impl VeloceApp {
    pub fn new() -> Self {
        Self {
            screen: Screen::Token {
                input: String::new(),
                error: None,
            },
            pending_token: keyring_get(),
        }
    }

    fn connect(&mut self, token: String, ctx: &egui::Context) {
        keyring_set(&token);
        let net = spawn_net(token, ctx.clone());
        self.screen = Screen::Chat {
            net,
            state: Box::new(ChatState::default()),
        };
    }
}

fn keyring_get() -> Option<String> {
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
        .ok()?
        .get_password()
        .ok()
}

fn keyring_set(token: &str) {
    if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER) {
        let _ = entry.set_password(token);
    }
}

fn keyring_clear() {
    if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER) {
        let _ = entry.delete_credential();
    }
}

fn spans_to_job(spans: &[Span]) -> LayoutJob {
    let mut job = LayoutJob::default();
    for s in spans {
        let mut fmt = TextFormat {
            font_id: FontId::proportional(14.0),
            ..Default::default()
        };
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

        // Capturé en dehors du match pour éviter l'emprunt multiple de self.
        let mut auth_failed: Option<String> = None;

        match &mut self.screen {
            Screen::Token { input, error } => {
                let mut submit: Option<String> = None;
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.heading("Veloce");
                    ui.label("Colle ton token Discord :");
                    ui.add(
                        egui::TextEdit::singleline(input)
                            .password(true)
                            .desired_width(400.0),
                    );
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
                // Drain des events — intercepter AuthFailed avant apply_event.
                while let Ok(ev) = net.events.try_recv() {
                    if let Event::AuthFailed(msg) = ev {
                        auth_failed = Some(msg);
                        continue;
                    }
                    apply_event(state, ev);
                }
                draw_chat(ctx, net, state.as_mut());
            }
        }

        // Hors du match : réinitialiser l'écran si l'auth a échoué.
        if let Some(msg) = auth_failed {
            keyring_clear();
            self.screen = Screen::Token {
                input: String::new(),
                error: Some(msg),
            };
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
                state.last_error = None;
            }
        }
        Event::MessagesLoaded {
            channel_id,
            messages,
        } => {
            if Some(&channel_id) == state.selected_channel.as_ref() {
                state.messages = messages;
                state.last_error = None;
            }
        }
        Event::MessageCreated(m) => {
            if Some(&m.channel_id) == state.selected_channel.as_ref()
                && should_append_message(&state.messages, &m)
            {
                state.messages.push(m);
            }
        }
        Event::MessageUpdated(m) => {
            if let Some(existing) = state.messages.iter_mut().find(|x| x.id == m.id) {
                *existing = m;
            }
        }
        Event::MessageDeleted { id, .. } => state.messages.retain(|m| m.id != id),
        Event::Error(e) => {
            tracing::warn!("erreur réseau: {e}");
            state.last_error = Some(e);
        }
        // AuthFailed est intercepté dans `update` avant d'atteindre cette fonction.
        Event::AuthFailed(_) => {}
    }
}

fn draw_chat(ctx: &egui::Context, net: &NetHandle, state: &mut ChatState) {
    egui::SidePanel::left("guilds")
        .exact_width(180.0)
        .show(ctx, |ui| {
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
                    if ui
                        .selectable_label(state.selected_guild.as_ref() == Some(&g.id), &g.name)
                        .clicked()
                    {
                        state.selected_guild = Some(g.id.clone());
                        state.channels.clear();
                        net.send(Command::SelectGuild(g.id));
                    }
                }
            });
        });

    egui::SidePanel::left("channels")
        .exact_width(200.0)
        .show(ctx, |ui| {
            ui.heading("Salons");
            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| {
                for c in state.channels.clone() {
                    let name = c.name.clone().unwrap_or_else(|| c.id.clone());
                    if ui
                        .selectable_label(
                            state.selected_channel.as_ref() == Some(&c.id),
                            format!("# {name}"),
                        )
                        .clicked()
                    {
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
            let resp = ui.add(
                egui::TextEdit::singleline(&mut state.draft)
                    .desired_width(f32::INFINITY)
                    .hint_text("Message…"),
            );
            let send = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if send && !state.draft.trim().is_empty() {
                if let Some(cid) = state.selected_channel.clone() {
                    net.send(Command::SendMessage {
                        channel_id: cid,
                        content: state.draft.trim().to_string(),
                    });
                    state.draft.clear();
                }
                resp.request_focus();
            }
        });
    });

    egui::CentralPanel::default().show(ctx, |ui| {
        if let Some(err) = &state.last_error {
            ui.colored_label(Color32::LIGHT_RED, err.as_str());
            ui.separator();
        }
        egui::ScrollArea::vertical()
            .stick_to_bottom(true)
            .show(ui, |ui| {
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
                        ui.label(spans_to_job(&parse_markdown(&m.content)));
                    });
                }
            });
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use veloce_discord::{Channel, User};

    fn ch(id: &str, kind: u8, pos: i32) -> Channel {
        Channel {
            id: id.into(),
            name: Some(id.into()),
            kind,
            guild_id: None,
            position: Some(pos),
        }
    }

    fn make_msg(id: &str) -> Message {
        Message {
            id: id.into(),
            channel_id: "ch1".into(),
            content: String::new(),
            author: User {
                id: "u1".into(),
                username: "user".into(),
                global_name: None,
                discriminator: None,
            },
            timestamp: None,
        }
    }

    #[test]
    fn ne_garde_que_les_salons_texte_tries_par_position() {
        let input = vec![ch("b", 0, 2), ch("voc", 2, 0), ch("a", 0, 1)];
        let out = text_channels_sorted(input);
        let ids: Vec<_> = out.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, vec!["a", "b"]);
    }

    #[test]
    fn should_append_message_dedupe_par_id() {
        let existing = vec![make_msg("1")];
        // Même id → ne doit pas être ajouté.
        assert!(!should_append_message(&existing, &make_msg("1")));
        // Id différent → doit être ajouté.
        assert!(should_append_message(&existing, &make_msg("2")));
    }
}
