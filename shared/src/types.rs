use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;

pub type Identifier = String;
pub type Timestamp = String;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, FromRow)]
pub struct Server {
    pub id: Identifier,
    pub owner_id: Identifier,
    pub name: String,
    pub host: String,
    pub port: i32,
    pub max_users: i32,
    pub is_online: bool,
    pub last_heartbeat: Option<Timestamp>,
    pub actual_host: Option<String>,
    pub actual_port: Option<i32>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub description: Option<String>,
    pub created_at: Timestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, FromRow)]
pub struct Category {
    pub id: Identifier,
    pub server_id: Identifier,
    pub name: String,
    pub position: i32,
    pub created_at: Timestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, FromRow)]
pub struct Channel {
    pub id: Identifier,
    pub server_id: Identifier,
    pub name: String,
    #[serde(rename = "type")]
    #[sqlx(rename = "type")]
    pub r#type: String,
    pub category_id: Option<Identifier>,
    pub position: i32,
    pub description: Option<String>,
    pub created_at: Timestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CategoryWithChannels {
    pub category: Category,
    pub channels: Vec<Channel>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, FromRow)]
pub struct ServerMember {
    pub id: Identifier,
    pub server_id: Identifier,
    pub user_id: Identifier,
    pub username: String,
    pub avatar: Option<String>,
    pub role: String,
    pub joined_at: Timestamp,
    pub last_seen: Timestamp,
    pub is_online: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServerStructure {
    pub server: Server,
    pub categories: Vec<CategoryWithChannels>,
    pub uncategorized_channels: Vec<Channel>,
    pub members: Vec<ServerMember>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, FromRow)]
pub struct ChatMessage {
    pub id: Identifier,
    pub channel_id: Identifier,
    pub user_id: Identifier,
    pub username: String,
    pub content: String,
    pub created_at: Timestamp,
    pub edited_at: Option<Timestamp>,
    pub reply_to_message_id: Option<Identifier>,
    pub attachments: Option<Value>,
    pub reactions: Option<Value>,
    pub is_pinned: bool,
    pub is_system: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MessageHistory {
    pub messages: Vec<ChatMessage>,
    pub total_count: i64,
    pub has_more: bool,
    pub next_cursor: Option<Identifier>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VoiceParticipant {
    pub user_id: Identifier,
    pub username: String,
    pub peer_id: Identifier,
    pub is_muted: bool,
    pub is_deafened: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VoiceRoom {
    pub channel_id: Identifier,
    pub participants: Vec<VoiceParticipant>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, FromRow)]
pub struct ServerRole {
    pub id: Identifier,
    pub server_id: Identifier,
    pub name: String,
    pub color: String,
    pub position: i32,
    pub permissions_bitfield: i32,
    pub is_default: bool,
    pub created_at: Timestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServerRoleWithMembers {
    pub role: ServerRole,
    pub members_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServerMemberWithRoles {
    pub member: ServerMember,
    pub roles: Vec<ServerRole>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, FromRow)]
pub struct License {
    pub id: Identifier,
    pub user_id: Identifier,
    pub server_slots: i32,
    pub active: bool,
    pub expires_at: Option<Timestamp>,
    pub created_at: Timestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, FromRow)]
pub struct User {
    pub id: Identifier,
    pub user_number: i32,
    pub username: String,
    pub email: String,
    pub provider: String,
    pub provider_id: Option<String>,
    pub avatar: Option<String>,
    pub status: String,
    pub role: String,
    pub bio: Option<String>,
    pub is_active: bool,
    pub deleted_at: Option<Timestamp>,
    pub created_at: Timestamp,
}

pub mod permissions {
    pub const VIEW_CHANNELS: i32 = 1;
    pub const SEND_MESSAGES: i32 = 2;
    pub const MANAGE_MESSAGES: i32 = 4;
    pub const MANAGE_CHANNELS: i32 = 8;
    pub const MANAGE_ROLES: i32 = 32;
    pub const CONNECT_VOICE: i32 = 64;
    pub const SPEAK_VOICE: i32 = 128;
    pub const ADMINISTRATOR: i32 = 2_147_483_647;
}

