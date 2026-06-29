use crate::models::{Channel, Overwrite, Role, Snowflake};
use std::collections::HashSet;

pub const ADMINISTRATOR: u64 = 1 << 3;
pub const VIEW_CHANNEL: u64 = 1 << 10;

fn parse(s: &str) -> u64 {
    s.parse::<u64>().unwrap_or(0)
}

/// Permissions de base : @everyone OR rôles du membre ; owner/ADMIN → tous les bits.
pub fn base_permissions(everyone_perms: u64, member_role_perms: &[u64], is_owner: bool) -> u64 {
    if is_owner {
        return u64::MAX;
    }
    let mut perms = everyone_perms;
    for r in member_role_perms {
        perms |= r;
    }
    if perms & ADMINISTRATOR != 0 {
        return u64::MAX;
    }
    perms
}

/// Applique les overwrites d'un salon : @everyone → agrégat rôles → membre.
pub fn channel_permissions(
    base: u64,
    overwrites: &[Overwrite],
    everyone_id: &str,
    member_role_ids: &[Snowflake],
    me_id: &str,
) -> u64 {
    if base & ADMINISTRATOR != 0 {
        return base;
    }
    let mut perms = base;
    // @everyone
    if let Some(o) = overwrites
        .iter()
        .find(|o| o.kind == 0 && o.id == everyone_id)
    {
        perms = (perms & !parse(&o.deny)) | parse(&o.allow);
    }
    // agrégat des rôles du membre
    let mut allow = 0u64;
    let mut deny = 0u64;
    for o in overwrites
        .iter()
        .filter(|o| o.kind == 0 && member_role_ids.iter().any(|r| r == &o.id))
    {
        allow |= parse(&o.allow);
        deny |= parse(&o.deny);
    }
    perms = (perms & !deny) | allow;
    // membre
    if let Some(o) = overwrites.iter().find(|o| o.kind == 1 && o.id == me_id) {
        perms = (perms & !parse(&o.deny)) | parse(&o.allow);
    }
    perms
}

pub fn can_view_channel(
    base: u64,
    overwrites: &[Overwrite],
    everyone_id: &str,
    member_role_ids: &[Snowflake],
    me_id: &str,
) -> bool {
    channel_permissions(base, overwrites, everyone_id, member_role_ids, me_id) & VIEW_CHANNEL != 0
}

/// Ensemble des ids de salons visibles par le membre.
pub fn visible_channel_ids(
    channels: &[Channel],
    roles: &[Role],
    owner_id: &str,
    member_roles: &[Snowflake],
    me_id: &str,
    guild_id: &str,
) -> HashSet<Snowflake> {
    let everyone_perms = roles
        .iter()
        .find(|r| r.id == guild_id)
        .map(|r| parse(&r.permissions))
        .unwrap_or(0);
    let member_role_perms: Vec<u64> = roles
        .iter()
        .filter(|r| member_roles.iter().any(|id| id == &r.id))
        .map(|r| parse(&r.permissions))
        .collect();
    let base = base_permissions(everyone_perms, &member_role_perms, me_id == owner_id);

    channels
        .iter()
        .filter(|c| {
            can_view_channel(
                base,
                &c.permission_overwrites,
                guild_id,
                member_roles,
                me_id,
            )
        })
        .map(|c| c.id.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Channel, Overwrite, Role};

    fn ow(id: &str, kind: u8, allow: &str, deny: &str) -> Overwrite {
        Overwrite {
            id: id.into(),
            kind,
            allow: allow.into(),
            deny: deny.into(),
        }
    }
    fn chan(id: &str, parent: Option<&str>, kind: u8, ows: Vec<Overwrite>) -> Channel {
        Channel {
            id: id.into(),
            name: Some(id.into()),
            kind,
            guild_id: Some("10".into()),
            position: Some(0),
            parent_id: parent.map(|s| s.into()),
            permission_overwrites: ows,
        }
    }
    const V: u64 = VIEW_CHANNEL;

    #[test]
    fn owner_a_tout() {
        assert_eq!(base_permissions(0, &[], true), u64::MAX);
    }

    #[test]
    fn admin_a_tout() {
        let b = base_permissions(ADMINISTRATOR, &[], false);
        assert_eq!(b, u64::MAX);
    }

    #[test]
    fn everyone_deny_view_cache() {
        // base a VIEW, mais @everyone overwrite deny VIEW.
        let b = V;
        let ows = vec![ow("10", 0, "0", &V.to_string())]; // everyone (id==guild) deny VIEW
        assert!(!can_view_channel(b, &ows, "10", &[], "me"));
    }

    #[test]
    fn role_allow_view_par_dessus_everyone_deny() {
        let b = V;
        let ows = vec![
            ow("10", 0, "0", &V.to_string()), // @everyone deny VIEW
            ow("42", 0, &V.to_string(), "0"), // rôle 42 allow VIEW
        ];
        assert!(can_view_channel(b, &ows, "10", &["42".to_string()], "me"));
    }

    #[test]
    fn member_overwrite_prioritaire() {
        let b = V;
        let ows = vec![
            ow("42", 0, "0", &V.to_string()), // rôle 42 deny VIEW
            ow("me", 1, &V.to_string(), "0"), // membre allow VIEW
        ];
        assert!(can_view_channel(b, &ows, "10", &["42".to_string()], "me"));
    }

    #[test]
    fn visible_channel_ids_filtre() {
        let everyone = Role {
            id: "10".into(),
            name: String::new(),
            permissions: V.to_string(),
            position: 0,
            color: 0,
        };
        let chans = vec![
            chan("a", None, 0, vec![]), // visible (hérite base VIEW)
            chan("b", None, 0, vec![ow("10", 0, "0", &V.to_string())]), // caché (everyone deny)
        ];
        let vis = visible_channel_ids(&chans, &[everyone], "owner", &[], "me", "10");
        assert!(vis.contains("a"));
        assert!(!vis.contains("b"));
    }
}
