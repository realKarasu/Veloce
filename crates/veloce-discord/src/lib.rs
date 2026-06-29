//! veloce-discord — client Discord (REST + Gateway), UI-agnostique.

pub mod models;
pub use models::{
    Attachment, Channel, Embed, EmbedAuthor, EmbedField, EmbedFooter, EmbedMedia, GatewayPayload,
    Guild, Message, Overwrite, Role, Snowflake, User,
};

pub mod commands;
pub mod events;
pub mod gateway_state;
pub mod identity;
pub use commands::Command;
pub use events::{ConnectionState, Event};
pub use gateway_state::{GatewayAction, GatewayState};

pub mod perms;
pub use perms::{can_view_channel, visible_channel_ids};

pub mod channel_tree;
pub use channel_tree::{build_channel_tree, TreeRow};

pub mod error;
pub mod gateway;
pub mod rest;
pub use error::{DiscordError, Result};
pub use gateway::{run_gateway, GatewayCommand};
pub use rest::{GuildDetail, MemberRoles, RestClient};
