use crate::events::{ConnectionState, Event};
use crate::gateway_state::{GatewayAction, GatewayState};
use crate::identity::super_properties_json;
use crate::models::{GatewayPayload, Guild, Message, User};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::time::Duration;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::sync::watch;
use tokio_tungstenite::tungstenite::Message as WsMessage;

pub const GATEWAY_URL: &str = "wss://gateway.discord.gg/?v=10&encoding=json";

pub fn build_identify(token: &str) -> Value {
    json!({
        "op": 2,
        "d": {
            "token": token,
            "capabilities": 16381,
            "properties": super_properties_json(),
            "presence": { "status": "online", "since": 0, "activities": [], "afk": false },
            "compress": false
        }
    })
}

fn build_resume(token: &str, session_id: &str, seq: Option<u64>) -> Value {
    json!({ "op": 6, "d": { "token": token, "session_id": session_id, "seq": seq.unwrap_or(0) } })
}

fn build_heartbeat(seq: Option<u64>) -> Value {
    json!({ "op": 1, "d": seq })
}

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

/// Pure backoff helper: doubles current_ms, capped at 30 s.
pub fn next_backoff(current_ms: u64) -> u64 {
    (current_ms * 2).min(30_000)
}

/// Boucle principale : (re)connexion jusqu'au shutdown, avec backoff.
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

/// Une session : handshake + boucle de réception. Err(()) = besoin de reconnecter.
async fn connect_once(
    token: &str,
    state: &mut GatewayState,
    event_tx: &UnboundedSender<Event>,
    shutdown: &mut watch::Receiver<bool>,
    backoff: &mut u64,
    gw_cmd_rx: &mut UnboundedReceiver<GatewayCommand>,
    subscribed: &mut HashSet<crate::models::Snowflake>,
) -> std::result::Result<(), ()> {
    let (ws, _) = tokio_tungstenite::connect_async(GATEWAY_URL)
        .await
        .map_err(|e| {
            let _ = event_tx.send(Event::Error(format!("connexion gateway: {e}")));
        })?;
    let (mut write, mut read) = ws.split();
    let mut hb = tokio::time::interval(Duration::from_secs(45));
    hb.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut hb_started = false;
    let mut gw_cmd_open = true;

    loop {
        tokio::select! {
            _ = shutdown.changed() => return Ok(()),
            _ = hb.tick(), if hb_started => {
                if write
                    .send(WsMessage::Text(
                        build_heartbeat(state.seq).to_string().into(),
                    ))
                    .await
                    .is_err()
                {
                    return Err(());
                }
            }
            msg = read.next() => {
                let txt = match msg {
                    Some(Ok(WsMessage::Close(_))) | Some(Err(_)) | None => return Err(()),
                    Some(Ok(WsMessage::Text(txt))) => txt,
                    _ => continue,
                };
                let payload: GatewayPayload = match serde_json::from_str(&txt) {
                    Ok(p) => p,
                    Err(_) => continue,
                };
                // HELLO : démarre heartbeat à l'intervalle reçu, puis handshake
                if payload.op == 10 {
                    if let Some(ms) = payload
                        .d
                        .get("heartbeat_interval")
                        .and_then(Value::as_u64)
                    {
                        state.on_hello(ms);
                        hb = tokio::time::interval(Duration::from_millis(ms));
                        hb.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
                        hb.reset();
                        *backoff = 1000;
                        hb_started = true;
                    }
                    let hs = state.handshake_action();
                    let frame = match &hs {
                        GatewayAction::Resume { session_id, seq } => {
                            build_resume(token, session_id, *seq)
                        }
                        _ => build_identify(token),
                    };
                    if write
                        .send(WsMessage::Text(frame.to_string().into()))
                        .await
                        .is_err()
                    {
                        return Err(());
                    }
                    continue;
                }
                let resumable = if payload.op == 9 {
                    payload.d.as_bool()
                } else {
                    None
                };
                let action =
                    state.on_payload(payload.op, payload.t.as_deref(), payload.s, resumable);
                match action {
                    GatewayAction::SendHeartbeat => {
                        if write
                            .send(WsMessage::Text(
                                build_heartbeat(state.seq).to_string().into(),
                            ))
                            .await
                            .is_err()
                        {
                            return Err(());
                        }
                    }
                    GatewayAction::ReconnectResumable | GatewayAction::ReconnectFull => {
                        return Err(())
                    }
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
                    _ => {}
                }
            }
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
        }
    }
}

fn dispatch_event(t: &str, d: &Value, state: &mut GatewayState, tx: &UnboundedSender<Event>) {
    match t {
        "READY" => {
            if let Some(sid) = d.get("session_id").and_then(Value::as_str) {
                state.set_session(sid.to_string());
            }
            let user: Option<User> = d
                .get("user")
                .and_then(|u| serde_json::from_value(u.clone()).ok());
            let guilds: Vec<Guild> = d
                .get("guilds")
                .and_then(|g| serde_json::from_value(g.clone()).ok())
                .unwrap_or_default();
            let _ = tx.send(Event::Connection(ConnectionState::Connected));
            if let Some(user) = user {
                let _ = tx.send(Event::Ready { user, guilds });
            } else {
                let _ = tx.send(Event::Error("READY sans user déchiffrable".into()));
            }
        }
        "MESSAGE_CREATE" => {
            if let Ok(m) = serde_json::from_value::<Message>(d.clone()) {
                let _ = tx.send(Event::MessageCreated(m));
            }
        }
        "MESSAGE_UPDATE" => {
            if let Ok(m) = serde_json::from_value::<Message>(d.clone()) {
                let _ = tx.send(Event::MessageUpdated(m));
            }
        }
        "MESSAGE_DELETE" => {
            if let (Some(id), Some(cid)) = (
                d.get("id").and_then(Value::as_str),
                d.get("channel_id").and_then(Value::as_str),
            ) {
                let _ = tx.send(Event::MessageDeleted {
                    id: id.into(),
                    channel_id: cid.into(),
                });
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn identify_contient_token_et_properties() {
        let v = build_identify("mon-token");
        assert_eq!(v["op"], 2);
        assert_eq!(v["d"]["token"], "mon-token");
        assert!(v["d"]["properties"]["os"].is_string());
    }

    #[test]
    fn next_backoff_double_et_plafonne() {
        assert_eq!(next_backoff(1000), 2000);
        assert_eq!(next_backoff(20_000), 30_000);
        assert_eq!(next_backoff(30_000), 30_000);
    }

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
}
