#![allow(dead_code)]

use veloce_discord::User;

const CDN: &str = "https://cdn.discordapp.com";

pub fn avatar_url(user_id: &str, hash: &str, size: u32) -> String {
    let ext = if hash.starts_with("a_") { "gif" } else { "png" };
    format!("{CDN}/avatars/{user_id}/{hash}.{ext}?size={size}")
}

pub fn default_avatar_url(user_id: &str, discriminator: Option<&str>) -> String {
    let index = match discriminator {
        Some("0") | None => {
            let id: u64 = user_id.parse().unwrap_or(0);
            (id >> 22) % 6
        }
        Some(d) => d.parse::<u64>().unwrap_or(0) % 5,
    };
    format!("{CDN}/embed/avatars/{index}.png")
}

pub fn avatar_for(user: &User) -> String {
    match &user.avatar {
        Some(hash) if !hash.is_empty() => avatar_url(&user.id, hash, 80),
        _ => default_avatar_url(&user.id, user.discriminator.as_deref()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn avatar_png_et_gif() {
        assert_eq!(
            avatar_url("42", "abc", 80),
            "https://cdn.discordapp.com/avatars/42/abc.png?size=80"
        );
        assert_eq!(
            avatar_url("42", "a_xyz", 80),
            "https://cdn.discordapp.com/avatars/42/a_xyz.gif?size=80"
        );
    }

    #[test]
    fn defaut_systeme_pseudo_et_legacy() {
        // discriminator "0" (nouveau système) → (id >> 22) % 6
        let url = default_avatar_url("80351110224678912", Some("0"));
        assert!(url.starts_with("https://cdn.discordapp.com/embed/avatars/"));
        assert!(url.ends_with(".png"));
        // legacy : discriminator % 5
        assert_eq!(
            default_avatar_url("1", Some("1337")),
            "https://cdn.discordapp.com/embed/avatars/2.png"
        );
    }
}
