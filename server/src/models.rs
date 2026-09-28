use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// Constantes pour les permissions (bitfield)
pub const VIEW_CHANNELS: i32 = 1;
pub const SEND_MESSAGES: i32 = 2;
pub const MANAGE_MESSAGES: i32 = 4;
pub const MANAGE_CHANNELS: i32 = 8;
pub const MANAGE_ROLES: i32 = 32;
pub const CONNECT_VOICE: i32 = 64;
pub const SPEAK_VOICE: i32 = 128;
pub const ADMINISTRATOR: i32 = 2147483647; // Toutes permissions

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Channel {
    pub id: String,
    pub server_id: String,
    pub name: String,
    pub r#type: String,
    pub category_id: Option<String>,
    pub position: i32,
    pub description: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Category {
    pub id: String,
    pub server_id: String,
    pub name: String,
    pub position: i32,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ServerMember {
    pub id: String,
    pub server_id: String,
    pub user_id: String,
    pub username: String,
    pub avatar: Option<String>,
    pub role: String,
    pub joined_at: String,
    pub last_seen: String,
    pub is_online: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[allow(dead_code)]
pub struct VoiceSession {
    pub id: String,
    pub channel_id: String,
    pub user_id: String,
    pub username: String,
    pub peer_id: String,
    pub joined_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ChatMessage {
    pub id: String,
    pub channel_id: String,
    pub user_id: String,
    pub username: String,
    pub content: String,
    pub created_at: String,
    pub edited_at: Option<String>,
    pub reply_to_message_id: Option<String>,
    pub attachments: Option<String>, // JSON array
    pub reactions: Option<String>,   // JSON object
    pub is_pinned: bool,
    pub is_system: bool,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageAttachment {
    pub id: String,
    pub message_id: String,
    pub filename: String,
    pub file_size: i64,
    pub mime_type: String,
    pub file_url: String,
    pub thumbnail_url: Option<String>,
    pub created_at: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageReaction {
    pub id: String,
    pub message_id: String,
    pub user_id: String,
    pub emoji: String,
    pub created_at: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageMention {
    pub id: String,
    pub message_id: String,
    pub mentioned_user_id: String,
    pub created_at: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessageWithDetails {
    pub message: ChatMessage,
    pub attachments: Vec<MessageAttachment>,
    pub reactions: Vec<MessageReaction>,
    pub mentions: Vec<MessageMention>,
    pub reply_to_message: Option<ChatMessage>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageHistoryResponse {
    pub messages: Vec<ChatMessageWithDetails>,
    pub total_count: i64,
    pub has_more: bool,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryWithChannels {
    pub category: Category,
    pub channels: Vec<Channel>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Server {
    pub id: String,
    pub owner_id: String,
    pub name: String,
    pub host: String,
    pub port: i32,
    pub max_users: i32,
    pub is_online: bool,
    pub last_heartbeat: Option<String>,
    pub actual_host: Option<String>,
    pub actual_port: Option<i32>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub description: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerStructure {
    pub server: Server,
    pub categories: Vec<CategoryWithChannels>,
    pub uncategorized_channels: Vec<Channel>,
    pub members: Vec<ServerMember>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct VoiceParticipant {
    pub user_id: String,
    pub username: String,
    pub peer_id: String,
    pub is_muted: bool,
    pub is_deafened: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct VoiceRoom {
    pub channel_id: String,
    pub participants: Vec<VoiceParticipant>,
}
