use serde::Deserialize;

pub type Snowflake = String;

#[derive(Debug, Clone, Deserialize)]
pub struct User {
    pub id: Snowflake,
    pub username: String,
    #[serde(default)]
    pub global_name: Option<String>,
    #[serde(default)]
    pub discriminator: Option<String>,
    #[serde(default)]
    pub avatar: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Guild {
    pub id: Snowflake,
    pub name: String,
    #[serde(default)]
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Channel {
    pub id: Snowflake,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub kind: u8,
    #[serde(default)]
    pub guild_id: Option<Snowflake>,
    #[serde(default)]
    pub position: Option<i32>,
    #[serde(default)]
    pub parent_id: Option<Snowflake>,
    #[serde(default)]
    pub permission_overwrites: Vec<Overwrite>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Overwrite {
    pub id: Snowflake,
    #[serde(rename = "type")]
    pub kind: u8, // 0 = rôle, 1 = membre
    #[serde(default)]
    pub allow: String,
    #[serde(default)]
    pub deny: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Role {
    pub id: Snowflake,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub permissions: String,
    #[serde(default)]
    pub position: i64,
    #[serde(default)]
    pub color: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Message {
    pub id: Snowflake,
    pub channel_id: Snowflake,
    pub content: String,
    pub author: User,
    #[serde(default)]
    pub timestamp: Option<String>,
    #[serde(default)]
    pub edited_timestamp: Option<String>,
    #[serde(default)]
    pub mentions: Vec<User>,
    #[serde(default)]
    pub mention_roles: Vec<Snowflake>,
    #[serde(default)]
    pub attachments: Vec<Attachment>,
    #[serde(default)]
    pub embeds: Vec<Embed>,
    #[serde(default)]
    pub referenced_message: Option<Box<Message>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Attachment {
    pub id: Snowflake,
    #[serde(default)]
    pub filename: String,
    #[serde(default)]
    pub content_type: Option<String>,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub proxy_url: String,
    #[serde(default)]
    pub size: u64,
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
}

impl Attachment {
    pub fn is_image(&self) -> bool {
        if let Some(ct) = &self.content_type {
            if ct.starts_with("image/") {
                return true;
            }
        }
        let lower = self.filename.to_ascii_lowercase();
        [".png", ".jpg", ".jpeg", ".gif", ".webp", ".bmp"]
            .iter()
            .any(|ext| lower.ends_with(ext))
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Embed {
    #[serde(rename = "type", default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub color: Option<u32>,
    #[serde(default)]
    pub author: Option<EmbedAuthor>,
    #[serde(default)]
    pub fields: Vec<EmbedField>,
    #[serde(default)]
    pub image: Option<EmbedMedia>,
    #[serde(default)]
    pub thumbnail: Option<EmbedMedia>,
    #[serde(default)]
    pub footer: Option<EmbedFooter>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EmbedAuthor {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub icon_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EmbedField {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub value: String,
    #[serde(default)]
    pub inline: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EmbedMedia {
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub proxy_url: Option<String>,
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EmbedFooter {
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub icon_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GatewayPayload {
    pub op: u8,
    #[serde(default)]
    pub d: serde_json::Value,
    #[serde(default)]
    pub s: Option<u64>,
    #[serde(default)]
    pub t: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialise_message_avec_champs_extra() {
        let json = include_str!("../../../tests/fixtures/message_create.json");
        let m: Message = serde_json::from_str(json).unwrap();
        assert_eq!(m.id, "111");
        assert_eq!(m.channel_id, "222");
        assert_eq!(m.content, "salut **monde**");
        assert_eq!(m.author.username, "alice");
        assert_eq!(m.author.global_name.as_deref(), Some("Alice"));
    }

    #[test]
    fn deserialise_channel_type_renomme() {
        let json = include_str!("../../../tests/fixtures/channel.json");
        let c: Channel = serde_json::from_str(json).unwrap();
        assert_eq!(c.kind, 0);
        assert_eq!(c.name.as_deref(), Some("général"));
        assert_eq!(c.guild_id.as_deref(), Some("10"));
    }

    #[test]
    fn deserialise_guild_ignore_owner_id() {
        let json = include_str!("../../../tests/fixtures/guild.json");
        let g: Guild = serde_json::from_str(json).unwrap();
        assert_eq!(g.name, "Mon Serveur");
        assert_eq!(g.icon.as_deref(), Some("abc123"));
    }

    #[test]
    fn deserialise_channel_avec_parent_et_overwrites() {
        let json = r#"{ "id":"5","type":0,"name":"général","guild_id":"10","position":2,
            "parent_id":"99",
            "permission_overwrites":[ {"id":"10","type":0,"allow":"0","deny":"1024"} ] }"#;
        let c: Channel = serde_json::from_str(json).unwrap();
        assert_eq!(c.parent_id.as_deref(), Some("99"));
        assert_eq!(c.permission_overwrites.len(), 1);
        let o = &c.permission_overwrites[0];
        assert_eq!(o.id, "10");
        assert_eq!(o.kind, 0);
        assert_eq!(o.deny, "1024");
    }

    #[test]
    fn channel_sans_parent_ni_overwrites() {
        // rétro-compat : un salon sans ces champs reste valide.
        let json = r#"{ "id":"1","type":0,"name":"x" }"#;
        let c: Channel = serde_json::from_str(json).unwrap();
        assert!(c.parent_id.is_none());
        assert!(c.permission_overwrites.is_empty());
    }

    #[test]
    fn deserialise_role() {
        let json = r#"{ "id":"10","name":"@everyone","permissions":"1024","position":0 }"#;
        let r: Role = serde_json::from_str(json).unwrap();
        assert_eq!(r.id, "10");
        assert_eq!(r.permissions, "1024");
    }

    #[test]
    fn deserialise_message_riche() {
        let json = include_str!("../../../tests/fixtures/message_rich.json");
        let m: Message = serde_json::from_str(json).unwrap();
        assert_eq!(m.author.avatar.as_deref(), Some("abc123"));
        assert_eq!(m.mentions.len(), 1);
        assert_eq!(m.mention_roles, vec!["77".to_string()]);
        assert_eq!(m.attachments.len(), 1);
        assert!(m.attachments[0].is_image());
        assert_eq!(m.attachments[0].width, Some(800));
        assert_eq!(m.embeds.len(), 1);
        assert_eq!(m.embeds[0].title.as_deref(), Some("Titre"));
        assert_eq!(m.embeds[0].color, Some(5793266));
        assert_eq!(m.embeds[0].fields[0].inline, true);
        assert_eq!(
            m.referenced_message.as_ref().unwrap().content,
            "message parent"
        );
    }

    #[test]
    fn is_image_par_extension_sans_content_type() {
        let a = Attachment {
            id: "1".into(),
            filename: "x.JPG".into(),
            content_type: None,
            url: "u".into(),
            proxy_url: "u".into(),
            size: 0,
            width: None,
            height: None,
        };
        assert!(a.is_image());
        let b = Attachment {
            filename: "x.zip".into(),
            ..a.clone()
        };
        assert!(!b.is_image());
    }

    #[test]
    fn is_image_par_content_type_sans_extension() {
        let a = Attachment {
            id: "1".into(),
            filename: "blob".into(),
            content_type: Some("image/png".into()),
            url: "u".into(),
            proxy_url: "u".into(),
            size: 0,
            width: None,
            height: None,
        };
        assert!(a.is_image());
    }

    #[test]
    fn non_image_content_type_retourne_faux() {
        let a = Attachment {
            id: "1".into(),
            filename: "blob".into(),
            content_type: Some("application/octet-stream".into()),
            url: "u".into(),
            proxy_url: "u".into(),
            size: 0,
            width: None,
            height: None,
        };
        assert!(!a.is_image());
    }

    #[test]
    fn message_minimal_reste_valide() {
        // Rétro-compat : un message sans les nouveaux champs se désérialise.
        let json = r#"{ "id":"1","channel_id":"2","content":"hi",
            "author": { "id":"3","username":"u" } }"#;
        let m: Message = serde_json::from_str(json).unwrap();
        assert!(m.attachments.is_empty());
        assert!(m.embeds.is_empty());
        assert!(m.referenced_message.is_none());
    }
}
