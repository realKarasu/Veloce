use crate::models::{Channel, Guild, Message, Snowflake, User};

#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
    Connecting,
    Connected,
    Reconnecting,
    Disconnected,
}

#[derive(Debug, Clone)]
pub enum Event {
    Connection(ConnectionState),
    Ready {
        user: User,
        guilds: Vec<Guild>,
    },
    ChannelsLoaded {
        guild_id: Snowflake,
        channels: Vec<Channel>,
    },
    MessagesLoaded {
        channel_id: Snowflake,
        messages: Vec<Message>,
    },
    MessageCreated(Message),
    MessageUpdated(Message),
    MessageDeleted {
        id: Snowflake,
        channel_id: Snowflake,
    },
    Error(String),
    /// Échec d'authentification (token invalide/expiré) — distinct d'une erreur réseau transitoire.
    AuthFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_deleted_porte_id_et_channel() {
        let e = Event::MessageDeleted {
            id: "1".into(),
            channel_id: "2".into(),
        };
        match e.clone() {
            Event::MessageDeleted { id, channel_id } => {
                assert_eq!(id, "1");
                assert_eq!(channel_id, "2");
            }
            other => panic!("variant inattendu: {other:?}"),
        }
    }

    #[test]
    fn connection_state_compare() {
        assert_ne!(ConnectionState::Connected, ConnectionState::Disconnected);
    }
}
