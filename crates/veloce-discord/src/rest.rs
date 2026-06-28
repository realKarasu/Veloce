use crate::error::{DiscordError, Result};
use crate::identity::{super_properties_b64, USER_AGENT};
use crate::models::{Channel, Guild, Message, User};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE, USER_AGENT as UA};

const API_BASE: &str = "https://discord.com/api/v10";

pub fn parse_retry_after_ms(header_value: Option<&str>) -> u64 {
    header_value
        .and_then(|v| v.parse::<f64>().ok())
        .map(|secs| (secs * 1000.0) as u64)
        .unwrap_or(1000)
}

pub struct RestClient {
    http: reqwest::Client,
}

impl RestClient {
    pub fn new(token: String) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&token).map_err(|e| DiscordError::Decode(e.to_string()))?,
        );
        headers.insert(UA, HeaderValue::from_static(USER_AGENT));
        headers.insert(
            "X-Super-Properties",
            HeaderValue::from_str(&super_properties_b64())
                .map_err(|e| DiscordError::Decode(e.to_string()))?,
        );
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        let http = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;
        Ok(Self { http })
    }

    async fn get_json<T: serde::de::DeserializeOwned>(&self, url: String) -> Result<T> {
        let resp = self.http.get(&url).send().await?;
        Self::handle(resp).await
    }

    async fn handle<T: serde::de::DeserializeOwned>(resp: reqwest::Response) -> Result<T> {
        let status = resp.status();
        if status.as_u16() == 401 {
            return Err(DiscordError::Unauthorized);
        }
        if status.as_u16() == 429 {
            let ra = resp
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string());
            return Err(DiscordError::RateLimited {
                retry_after_ms: parse_retry_after_ms(ra.as_deref()),
            });
        }
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(DiscordError::Api {
                status: status.as_u16(),
                body,
            });
        }
        resp.json::<T>()
            .await
            .map_err(|e| DiscordError::Decode(e.to_string()))
    }

    pub async fn current_user(&self) -> Result<User> {
        self.get_json(format!("{API_BASE}/users/@me")).await
    }

    pub async fn current_user_guilds(&self) -> Result<Vec<Guild>> {
        self.get_json(format!("{API_BASE}/users/@me/guilds")).await
    }

    pub async fn guild_channels(&self, guild_id: &str) -> Result<Vec<Channel>> {
        self.get_json(format!("{API_BASE}/guilds/{guild_id}/channels"))
            .await
    }

    pub async fn channel_messages(&self, channel_id: &str, limit: u8) -> Result<Vec<Message>> {
        self.get_json(format!(
            "{API_BASE}/channels/{channel_id}/messages?limit={limit}"
        ))
        .await
    }

    pub async fn send_message(&self, channel_id: &str, content: &str) -> Result<Message> {
        let body = serde_json::json!({ "content": content });
        let resp = self
            .http
            .post(format!("{API_BASE}/channels/{channel_id}/messages"))
            .json(&body)
            .send()
            .await?;
        Self::handle(resp).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_after_secondes_vers_ms() {
        assert_eq!(parse_retry_after_ms(Some("1.5")), 1500);
        assert_eq!(parse_retry_after_ms(Some("0.2")), 200);
    }

    #[test]
    fn retry_after_absent_ou_invalide_donne_defaut() {
        assert_eq!(parse_retry_after_ms(None), 1000);
        assert_eq!(parse_retry_after_ms(Some("abc")), 1000);
    }
}
