use thiserror::Error;

#[derive(Debug, Error)]
pub enum DiscordError {
    #[error("erreur HTTP: {0}")]
    Http(#[from] reqwest::Error),
    #[error("token invalide (401)")]
    Unauthorized,
    #[error("rate limited, réessai dans {retry_after_ms} ms")]
    RateLimited { retry_after_ms: u64 },
    #[error("erreur API {status}: {body}")]
    Api { status: u16, body: String },
    #[error("erreur de décodage: {0}")]
    Decode(String),
}

pub type Result<T> = std::result::Result<T, DiscordError>;
