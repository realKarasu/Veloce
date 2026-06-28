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
    pub permissions: String,
    #[serde(default)]
    pub position: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Message {
    pub id: Snowflake,
    pub channel_id: Snowflake,
    pub content: String,
    pub author: User,
    #[serde(default)]
    pub timestamp: Option<String>,
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
}
