use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct CreateChannel {
    pub name: String,
    pub r#type: String,
    pub category_id: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct UpdateChannel {
    pub name: Option<String>,
    pub description: Option<String>,
    pub position: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ChannelPermission {
    pub id: String,
    pub channel_id: String,
    pub role: String,
    pub can_view: bool,
    pub can_send_messages: bool,
    pub can_manage_messages: bool,
    pub can_connect_voice: bool,
    pub can_speak: bool,
}

