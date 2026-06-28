//! veloce-discord — client Discord (REST + Gateway), UI-agnostique.

pub mod models;
pub use models::{Channel, GatewayPayload, Guild, Message, Snowflake, User};

pub mod commands;
pub mod events;
pub mod gateway_state;
pub mod identity;
pub use commands::Command;
pub use events::{ConnectionState, Event};
pub use gateway_state::{GatewayAction, GatewayState};
