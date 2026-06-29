use crate::timestamp::parse_epoch;
use veloce_discord::Message;

const GROUP_WINDOW_SECS: i64 = 7 * 60;

pub fn group_flags(messages: &[Message]) -> Vec<bool> {
    let mut flags = Vec::with_capacity(messages.len());
    for (i, m) in messages.iter().enumerate() {
        let new_group = if i == 0 {
            true
        } else {
            let prev = &messages[i - 1];
            if prev.author.id != m.author.id {
                true
            } else {
                match (
                    prev.timestamp.as_deref().and_then(parse_epoch),
                    m.timestamp.as_deref().and_then(parse_epoch),
                ) {
                    (Some(a), Some(b)) => (b - a).abs() > GROUP_WINDOW_SECS,
                    _ => true, // timestamp manquant/illisible → on coupe (en-tête)
                }
            }
        };
        flags.push(new_group);
    }
    flags
}

#[cfg(test)]
mod tests {
    use super::*;
    use veloce_discord::{Message, User};

    fn msg(id: &str, author: &str, ts: &str) -> Message {
        Message {
            id: id.into(),
            channel_id: "c".into(),
            content: String::new(),
            author: User {
                id: author.into(),
                username: author.into(),
                global_name: None,
                discriminator: None,
                avatar: None,
            },
            timestamp: Some(ts.into()),
            edited_timestamp: None,
            mentions: vec![],
            mention_roles: vec![],
            attachments: vec![],
            embeds: vec![],
            referenced_message: None,
        }
    }

    #[test]
    fn premier_est_toujours_entete() {
        let m = vec![msg("1", "alice", "2026-06-29T14:00:00+00:00")];
        assert_eq!(group_flags(&m), vec![true]);
    }

    #[test]
    fn meme_auteur_proche_est_continuation() {
        let m = vec![
            msg("1", "alice", "2026-06-29T14:00:00+00:00"),
            msg("2", "alice", "2026-06-29T14:03:00+00:00"),
        ];
        assert_eq!(group_flags(&m), vec![true, false]);
    }

    #[test]
    fn auteur_different_coupe() {
        let m = vec![
            msg("1", "alice", "2026-06-29T14:00:00+00:00"),
            msg("2", "bob", "2026-06-29T14:01:00+00:00"),
        ];
        assert_eq!(group_flags(&m), vec![true, true]);
    }

    #[test]
    fn ecart_superieur_a_7min_coupe() {
        let m = vec![
            msg("1", "alice", "2026-06-29T14:00:00+00:00"),
            msg("2", "alice", "2026-06-29T14:08:00+00:00"),
        ];
        assert_eq!(group_flags(&m), vec![true, true]);
    }

    #[test]
    fn timestamp_absent_coupe_groupe() {
        let m1 = msg("1", "alice", "2026-06-29T14:00:00+00:00");
        let m2 = Message {
            id: "2".into(),
            channel_id: "c".into(),
            content: String::new(),
            author: User {
                id: "alice".into(),
                username: "alice".into(),
                global_name: None,
                discriminator: None,
                avatar: None,
            },
            timestamp: None,
            edited_timestamp: None,
            mentions: vec![],
            mention_roles: vec![],
            attachments: vec![],
            embeds: vec![],
            referenced_message: None,
        };
        let m = vec![m1, m2];
        assert_eq!(group_flags(&m), vec![true, true]);
    }
}
