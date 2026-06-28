use crate::models::Snowflake;

#[derive(Debug, Clone)]
pub enum Command {
    SelectGuild(Snowflake),
    FetchHistory(Snowflake),
    SendMessage {
        channel_id: Snowflake,
        content: String,
    },
}
