use egui::Context;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio::sync::watch;
use veloce_discord::{run_gateway, Command, Event, GatewayCommand, RestClient};

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
        let _ = self
            .gw_cmd_tx
            .send(GatewayCommand::SubscribeGuild(guild_id));
    }
}

pub fn spawn_net(token: String, ctx: Context) -> NetHandle {
    let (event_out, events): (Sender<Event>, Receiver<Event>) = channel();
    let (cmd_tx, mut cmd_rx) = unbounded_channel::<Command>();
    let (gw_cmd_tx, gw_cmd_rx) = unbounded_channel::<GatewayCommand>();
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime tokio");
        rt.block_on(async move {
            // canal interne gateway -> relais
            let (gw_tx, mut gw_rx) = unbounded_channel::<Event>();

            // Valider le token avant de lancer la gateway.
            let rest = match RestClient::new(token.clone()) {
                Ok(r) => r,
                Err(e) => {
                    let _ = event_out.send(Event::AuthFailed(format!("Token invalide : {e}")));
                    ctx.request_repaint();
                    return;
                }
            };
            let me = match rest.current_user().await {
                Ok(u) => u,
                Err(e) => {
                    let _ = event_out
                        .send(Event::AuthFailed(format!("Authentification échouée : {e}")));
                    ctx.request_repaint();
                    return;
                }
            };
            let me_id = me.id.clone();

            // tâche gateway
            let gw_token = token.clone();
            let gw_shutdown = shutdown_rx.clone();
            tokio::spawn(async move { run_gateway(gw_token, gw_tx, gw_shutdown, gw_cmd_rx).await });

            loop {
                tokio::select! {
                    Some(ev) = gw_rx.recv() => {
                        if event_out.send(ev).is_err() { break; }
                        ctx.request_repaint();
                    }
                    Some(cmd) = cmd_rx.recv() => {
                        handle_command(&rest, cmd, &event_out, &ctx, &me_id).await;
                    }
                    else => break,
                }
            }
        });
    });

    NetHandle {
        events,
        cmd_tx,
        gw_cmd_tx,
        _shutdown: shutdown_tx,
    }
}

async fn handle_command(
    rest: &RestClient,
    cmd: Command,
    out: &Sender<Event>,
    ctx: &Context,
    me_id: &str,
) {
    let result: Result<Event, String> = match cmd {
        Command::SelectGuild(guild_id) => {
            let (channels, detail, member) = tokio::join!(
                rest.guild_channels(&guild_id),
                rest.guild(&guild_id),
                rest.current_member(&guild_id),
            );
            match (channels, detail, member) {
                (Ok(channels), Ok(detail), Ok(member)) => Ok(Event::GuildChannels {
                    guild_id,
                    channels,
                    roles: detail.roles,
                    owner_id: detail.owner_id,
                    member_roles: member.roles,
                    me_id: me_id.to_string(),
                }),
                _ => Err("Impossible de charger les salons du serveur".to_string()),
            }
        }
        Command::FetchHistory(channel_id) => rest
            .channel_messages(&channel_id, 50)
            .await
            .map(|mut messages| {
                messages.reverse();
                Event::MessagesLoaded {
                    channel_id,
                    messages,
                }
            })
            .map_err(|e| e.to_string()),
        Command::SendMessage {
            channel_id,
            content,
        } => rest
            .send_message(&channel_id, &content)
            .await
            .map(Event::MessageCreated)
            .map_err(|e| e.to_string()),
    };
    let ev = result.unwrap_or_else(Event::Error);
    let _ = out.send(ev);
    ctx.request_repaint();
}
