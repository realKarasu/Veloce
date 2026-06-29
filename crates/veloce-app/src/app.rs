use crate::emoji::{split_emojis, EmojiSeg};
use crate::markdown::{parse_markdown, Span};
use crate::net::{spawn_net, NetHandle};
use crate::plugins::PluginManager;
use eframe::egui;
use egui::{Color32, RichText};
use std::collections::HashSet;
use veloce_discord::{
    build_channel_tree, visible_channel_ids, Channel, Command, ConnectionState, Event, Guild,
    Message, TreeRow, User,
};

const KEYRING_SERVICE: &str = "veloce";
const KEYRING_USER: &str = "token";

fn channel_icon(kind: u8) -> &'static str {
    match kind {
        2 => "🔊",  // vocal
        5 => "📢",  // annonce
        13 => "🎙",  // stage
        15 => "💬", // forum
        _ => "#",   // texte et autres
    }
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
    channel_tree: Vec<TreeRow>,
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

const EMOJI_SIZE: f32 = 20.0;

/// Ligne cliquable pleine largeur de la sidebar : icône (glyphe) optionnelle +
/// nom avec emojis couleur (images). Le rectangle est réservé d'abord AVEC le
/// sense de clic (clic fiable sur toute la ligne, fond borné au panneau), puis
/// le contenu est rendu dans ce rect via un `new_child`.
fn rich_label_row(
    ui: &mut egui::Ui,
    icon: Option<&str>,
    name: &str,
    selected: bool,
    enabled: bool,
) -> egui::Response {
    const ROW_H: f32 = 24.0;
    let width = ui.available_width();
    let sense = if enabled {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, ROW_H), sense);

    if selected || (enabled && resp.hovered()) {
        let bg = if selected {
            ui.visuals().selection.bg_fill
        } else {
            ui.visuals().widgets.hovered.bg_fill
        };
        ui.painter().rect_filled(rect, 4.0, bg.gamma_multiply(0.5));
    }

    let color = if enabled {
        ui.visuals().text_color()
    } else {
        ui.visuals().weak_text_color()
    };
    let mut content = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect.shrink2(egui::vec2(6.0, 0.0)))
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    content.spacing_mut().item_spacing.x = 3.0;
    if let Some(ic) = icon {
        content.label(RichText::new(ic).color(color));
    }
    for seg in split_emojis(name) {
        match seg {
            EmojiSeg::Text(t) => {
                content.label(RichText::new(t).size(14.0).color(color));
            }
            EmojiSeg::Emoji { url } => {
                content.add(egui::Image::new(url).fit_to_exact_size(egui::vec2(18.0, 18.0)));
            }
        }
    }
    resp
}

/// RichText stylé selon un span markdown.
fn span_rich(text: &str, span: &Span) -> RichText {
    let mut rt = RichText::new(text).size(14.0);
    if span.code {
        rt = rt.monospace().background_color(Color32::from_gray(40));
    }
    if span.bold {
        rt = rt.strong().color(Color32::WHITE);
    }
    if span.italic {
        rt = rt.italics();
    }
    if span.strike {
        rt = rt.strikethrough();
    }
    rt
}

/// Rend un message : markdown (via plugins) + emojis couleur inline.
fn render_message(ui: &mut egui::Ui, content: &str, plugins: &mut PluginManager) {
    let mut c = content.to_string();
    plugins.apply_render(&mut c);
    let spans = parse_markdown(&c);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        for span in &spans {
            for seg in split_emojis(&span.text) {
                match seg {
                    EmojiSeg::Text(t) => {
                        ui.label(span_rich(&t, span));
                    }
                    EmojiSeg::Emoji { url } => {
                        ui.add(
                            egui::Image::new(url)
                                .fit_to_exact_size(egui::vec2(EMOJI_SIZE, EMOJI_SIZE)),
                        );
                    }
                }
            }
        }
    });
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
        Event::GuildChannels {
            guild_id,
            channels,
            roles,
            owner_id,
            member_roles,
            me_id,
        } => {
            if Some(&guild_id) == state.selected_guild.as_ref() {
                // Données de permissions absentes (rôles vides, ex. endpoint
                // refusé) → afficher tous les salons plutôt que rien.
                let visible: HashSet<String> = if roles.is_empty() {
                    channels.iter().map(|c| c.id.clone()).collect()
                } else {
                    visible_channel_ids(
                        &channels,
                        &roles,
                        &owner_id,
                        &member_roles,
                        &me_id,
                        &guild_id,
                    )
                };
                state.channel_tree = build_channel_tree(&channels, &visible);
                state.channels = channels;
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

fn draw_chat(
    ctx: &egui::Context,
    net: &NetHandle,
    state: &mut ChatState,
    plugins: &mut crate::plugins::PluginManager,
    show_plugins: &mut bool,
) {
    egui::SidePanel::left("guilds")
        .exact_width(180.0)
        .resizable(false)
        .show(ctx, |ui| {
            ui.heading("Serveurs");
            if ui.button("⚙ Plugins").clicked() {
                *show_plugins = !*show_plugins;
            }
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
                    let selected = state.selected_guild.as_ref() == Some(&g.id);
                    if rich_label_row(ui, None, &g.name, selected, true).clicked() {
                        state.selected_guild = Some(g.id.clone());
                        state.channels.clear();
                        state.channel_tree.clear();
                        net.subscribe_guild(g.id.clone());
                        net.send(Command::SelectGuild(g.id));
                    }
                }
            });
        });

    egui::SidePanel::left("channels")
        .exact_width(200.0)
        .resizable(false)
        .show(ctx, |ui| {
            ui.heading("Salons");
            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| {
                for row in state.channel_tree.clone() {
                    match row {
                        TreeRow::Category { name, .. } => {
                            ui.add_space(4.0);
                            ui.label(egui::RichText::new(name.to_uppercase()).small().strong());
                        }
                        TreeRow::Channel(c) => {
                            let selectable = matches!(c.kind, 0 | 5 | 15);
                            let selected = state.selected_channel.as_ref() == Some(&c.id);
                            let name = c.name.clone().unwrap_or_else(|| c.id.clone());
                            let resp = rich_label_row(
                                ui,
                                Some(channel_icon(c.kind)),
                                &name,
                                selected,
                                selectable,
                            );
                            if resp.clicked() {
                                state.selected_channel = Some(c.id.clone());
                                state.messages.clear();
                                net.send(Command::FetchHistory(c.id.clone()));
                            }
                        }
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
                    let mut content = state.draft.trim().to_string();
                    plugins.apply_outgoing(&mut content);
                    if !content.trim().is_empty() {
                        net.send(Command::SendMessage {
                            channel_id: cid,
                            content,
                        });
                    }
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
                        render_message(ui, &m.content, plugins);
                    });
                }
            });
    });

    if *show_plugins {
        let mut open = true;
        egui::Window::new("Plugins")
            .open(&mut open)
            .show(ctx, |ui| plugins.settings_ui(ui));
        if !open {
            *show_plugins = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use veloce_discord::User;

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
    fn should_append_message_dedupe_par_id() {
        let existing = vec![make_msg("1")];
        // Même id → ne doit pas être ajouté.
        assert!(!should_append_message(&existing, &make_msg("1")));
        // Id différent → doit être ajouté.
        assert!(should_append_message(&existing, &make_msg("2")));
    }
}
