use crate::types::{
    ChatMessage, Identifier, MessageHistory, ServerMember, ServerRole, ServerStructure, Timestamp,
    VoiceParticipant, VoiceRoom,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ClientEvent {
    JoinServer,
    LeaveServer,
    JoinChannel,
    JoinTextChannel,
    SendMessage,
    JoinVoiceChannel,
    LeaveVoiceChannel,
    WebrtcOffer,
    WebrtcAnswer,
    IceCandidate,
    ToggleAudio,
    ToggleVideo,
    ChangeAudioQuality,
    ChangeVideoQuality,
    AddMediaTrack,
    RemoveMediaTrack,
    GetVoiceStats,
    LoadMessageHistory,
    EditMessage,
    DeleteMessage,
    AddReaction,
    RemoveReaction,
    GetServerMembers,
    GetServerStructure,
    GetUserRole,
    CheckServerPermissions,
    GetVoiceRoomInfo,
    CreateChannel,
    UpdateChannel,
    DeleteChannel,
    MoveChannel,
    ReorderChannels,
    CreateCategory,
    UpdateCategory,
    DeleteCategory,
    MoveCategory,
    ReorderCategories,
    CreateRole,
    UpdateRole,
    DeleteRole,
    AssignRole,
    RemoveRole,
    SetChannelPermissions,
    SetCategoryPermissions,
    SetMemberChannelPermissions,
    SetMemberCategoryPermissions,
    GetEffectivePermissions,
    #[serde(other)]
    Unknown,
}

impl ClientEvent {
    pub fn as_str(&self) -> &'static str {
        match self {
            ClientEvent::JoinServer => "join_server",
            ClientEvent::LeaveServer => "leave_server",
            ClientEvent::JoinChannel => "join_channel",
            ClientEvent::JoinTextChannel => "join_text_channel",
            ClientEvent::SendMessage => "send_message",
            ClientEvent::JoinVoiceChannel => "join_voice_channel",
            ClientEvent::LeaveVoiceChannel => "leave_voice_channel",
            ClientEvent::WebrtcOffer => "webrtc_offer",
            ClientEvent::WebrtcAnswer => "webrtc_answer",
            ClientEvent::IceCandidate => "ice_candidate",
            ClientEvent::ToggleAudio => "toggle_audio",
            ClientEvent::ToggleVideo => "toggle_video",
            ClientEvent::ChangeAudioQuality => "change_audio_quality",
            ClientEvent::ChangeVideoQuality => "change_video_quality",
            ClientEvent::AddMediaTrack => "add_media_track",
            ClientEvent::RemoveMediaTrack => "remove_media_track",
            ClientEvent::GetVoiceStats => "get_voice_stats",
            ClientEvent::LoadMessageHistory => "load_message_history",
            ClientEvent::EditMessage => "edit_message",
            ClientEvent::DeleteMessage => "delete_message",
            ClientEvent::AddReaction => "add_reaction",
            ClientEvent::RemoveReaction => "remove_reaction",
            ClientEvent::GetServerMembers => "get_server_members",
            ClientEvent::GetServerStructure => "get_server_structure",
            ClientEvent::GetUserRole => "get_user_role",
            ClientEvent::CheckServerPermissions => "check_server_permissions",
            ClientEvent::GetVoiceRoomInfo => "get_voice_room_info",
            ClientEvent::CreateChannel => "create_channel",
            ClientEvent::UpdateChannel => "update_channel",
            ClientEvent::DeleteChannel => "delete_channel",
            ClientEvent::MoveChannel => "move_channel",
            ClientEvent::ReorderChannels => "reorder_channels",
            ClientEvent::CreateCategory => "create_category",
            ClientEvent::UpdateCategory => "update_category",
            ClientEvent::DeleteCategory => "delete_category",
            ClientEvent::MoveCategory => "move_category",
            ClientEvent::ReorderCategories => "reorder_categories",
            ClientEvent::CreateRole => "create_role",
            ClientEvent::UpdateRole => "update_role",
            ClientEvent::DeleteRole => "delete_role",
            ClientEvent::AssignRole => "assign_role",
            ClientEvent::RemoveRole => "remove_role",
            ClientEvent::SetChannelPermissions => "set_channel_permissions",
            ClientEvent::SetCategoryPermissions => "set_category_permissions",
            ClientEvent::SetMemberChannelPermissions => "set_member_channel_permissions",
            ClientEvent::SetMemberCategoryPermissions => "set_member_category_permissions",
            ClientEvent::GetEffectivePermissions => "get_effective_permissions",
            ClientEvent::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ServerEvent {
    Connected,
    Error,
    JoinServerAck,
    ServerStructure,
    PresenceSnapshot,
    PresenceDelta,
    MemberJoined,
    MemberLeft,
    MessageHistory,
    MessageHistoryPage,
    NewMessage,
    MessageAck,
    MessageEdited,
    MessageEditAck,
    MessageDeleted,
    MessageDeleteAck,
    ReactionAdded,
    ReactionRemoved,
    ServerMembers,
    ServerPermissions,
    UserRole,
    VoiceRoomInfo,
    VoiceRoomSnapshot,
    VoiceJoined,
    VoiceParticipantJoined,
    VoiceParticipantLeft,
    VoiceParticipantUpdated,
    VoiceStats,
    ChannelCreated,
    ChannelCreateAck,
    ChannelUpdated,
    ChannelUpdateAck,
    ChannelDeleted,
    ChannelDeleteAck,
    ChannelMoved,
    ChannelMoveAck,
    ChannelsReordered,
    ChannelReorderAck,
    CategoryCreated,
    CategoryCreateAck,
    CategoryUpdated,
    CategoryUpdateAck,
    CategoryDeleted,
    CategoryDeleteAck,
    CategoryMoved,
    CategoryMoveAck,
    CategoriesReordered,
    CategoryReorderAck,
    RoleCreated,
    RoleCreateAck,
    RoleUpdated,
    RoleUpdateAck,
    RoleDeleted,
    RoleDeleteAck,
    MemberRoleAssigned,
    RoleAssignAck,
    MemberRoleRemoved,
    RoleRemoveAck,
    PermissionsUpdated,
    SetChannelPermissionsAck,
    SetCategoryPermissionsAck,
    SetMemberChannelPermissionsAck,
    SetMemberCategoryPermissionsAck,
    EffectivePermissions,
    WebrtcAnswer,
    #[serde(other)]
    Unknown,
}

impl ServerEvent {
    pub fn as_str(&self) -> &'static str {
        match self {
            ServerEvent::Connected => "connected",
            ServerEvent::Error => "error",
            ServerEvent::JoinServerAck => "join_server_ack",
            ServerEvent::ServerStructure => "server_structure",
            ServerEvent::PresenceSnapshot => "presence_snapshot",
            ServerEvent::PresenceDelta => "presence_delta",
            ServerEvent::MemberJoined => "member_joined",
            ServerEvent::MemberLeft => "member_left",
            ServerEvent::MessageHistory => "message_history",
            ServerEvent::MessageHistoryPage => "message_history_page",
            ServerEvent::NewMessage => "new_message",
            ServerEvent::MessageAck => "message_ack",
            ServerEvent::MessageEdited => "message_edited",
            ServerEvent::MessageEditAck => "message_edit_ack",
            ServerEvent::MessageDeleted => "message_deleted",
            ServerEvent::MessageDeleteAck => "message_delete_ack",
            ServerEvent::ReactionAdded => "reaction_added",
            ServerEvent::ReactionRemoved => "reaction_removed",
            ServerEvent::ServerMembers => "server_members",
            ServerEvent::ServerPermissions => "server_permissions",
            ServerEvent::UserRole => "user_role",
            ServerEvent::VoiceRoomInfo => "voice_room_info",
            ServerEvent::VoiceRoomSnapshot => "voice_room_snapshot",
            ServerEvent::VoiceJoined => "voice_joined",
            ServerEvent::VoiceParticipantJoined => "voice_participant_joined",
            ServerEvent::VoiceParticipantLeft => "voice_participant_left",
            ServerEvent::VoiceParticipantUpdated => "voice_participant_updated",
            ServerEvent::VoiceStats => "voice_stats",
            ServerEvent::ChannelCreated => "channel_created",
            ServerEvent::ChannelCreateAck => "channel_create_ack",
            ServerEvent::ChannelUpdated => "channel_updated",
            ServerEvent::ChannelUpdateAck => "channel_update_ack",
            ServerEvent::ChannelDeleted => "channel_deleted",
            ServerEvent::ChannelDeleteAck => "channel_delete_ack",
            ServerEvent::ChannelMoved => "channel_moved",
            ServerEvent::ChannelMoveAck => "channel_move_ack",
            ServerEvent::ChannelsReordered => "channels_reordered",
            ServerEvent::ChannelReorderAck => "channel_reorder_ack",
            ServerEvent::CategoryCreated => "category_created",
            ServerEvent::CategoryCreateAck => "category_create_ack",
            ServerEvent::CategoryUpdated => "category_updated",
            ServerEvent::CategoryUpdateAck => "category_update_ack",
            ServerEvent::CategoryDeleted => "category_deleted",
            ServerEvent::CategoryDeleteAck => "category_delete_ack",
            ServerEvent::CategoryMoved => "category_moved",
            ServerEvent::CategoryMoveAck => "category_move_ack",
            ServerEvent::CategoriesReordered => "categories_reordered",
            ServerEvent::CategoryReorderAck => "category_reorder_ack",
            ServerEvent::RoleCreated => "role_created",
            ServerEvent::RoleCreateAck => "role_create_ack",
            ServerEvent::RoleUpdated => "role_updated",
            ServerEvent::RoleUpdateAck => "role_update_ack",
            ServerEvent::RoleDeleted => "role_deleted",
            ServerEvent::RoleDeleteAck => "role_delete_ack",
            ServerEvent::MemberRoleAssigned => "member_role_assigned",
            ServerEvent::RoleAssignAck => "role_assign_ack",
            ServerEvent::MemberRoleRemoved => "member_role_removed",
            ServerEvent::RoleRemoveAck => "role_remove_ack",
            ServerEvent::PermissionsUpdated => "permissions_updated",
            ServerEvent::SetChannelPermissionsAck => "set_channel_permissions_ack",
            ServerEvent::SetCategoryPermissionsAck => "set_category_permissions_ack",
            ServerEvent::SetMemberChannelPermissionsAck => "set_member_channel_permissions_ack",
            ServerEvent::SetMemberCategoryPermissionsAck => "set_member_category_permissions_ack",
            ServerEvent::EffectivePermissions => "effective_permissions",
            ServerEvent::WebrtcAnswer => "webrtc_answer",
            ServerEvent::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EventEnvelope<A> {
    pub action: A,
    #[serde(default)]
    pub data: Value,
}

impl<A> EventEnvelope<A> {
    pub fn new(action: A, data: Value) -> Self {
        Self { action, data }
    }
}

pub type ClientEventEnvelope = EventEnvelope<ClientEvent>;
pub type ServerEventEnvelope = EventEnvelope<ServerEvent>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct JoinServerPayload {
    pub server_id: Identifier,
    pub user_id: Identifier,
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LeaveServerPayload {
    pub server_id: Identifier,
    pub user_id: Identifier,
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct JoinChannelPayload {
    pub channel_id: Identifier,
    pub user_id: Identifier,
    pub username: String,
    pub limit: Option<u32>,
    pub before_message_id: Option<Identifier>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SendMessagePayload {
    pub channel_id: Identifier,
    pub user_id: Identifier,
    pub username: String,
    pub content: String,
    pub reply_to_message_id: Option<Identifier>,
    pub attachments: Option<Value>,
    pub is_system: bool,
    pub mentions: Option<Vec<Identifier>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EditMessagePayload {
    pub message_id: Identifier,
    pub content: String,
    pub edited_at: Timestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeleteMessagePayload {
    pub message_id: Identifier,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MessageHistoryRequestPayload {
    pub channel_id: Identifier,
    pub limit: Option<u32>,
    pub before_message_id: Option<Identifier>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MessageAckPayload {
    pub message_id: Identifier,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ErrorPayload {
    pub message: String,
    #[serde(default)]
    pub code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServerStructurePayload {
    pub server: ServerStructure,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MemberEventPayload {
    pub server_id: Identifier,
    pub user_id: Identifier,
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServerMembersPayload {
    pub members: Vec<ServerMember>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServerPermissionsPayload {
    pub server_id: Identifier,
    pub permissions: Vec<ServerRole>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UserRolePayload {
    pub user_id: Identifier,
    pub roles: Vec<ServerRole>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VoiceJoinPayload {
    pub channel_id: Identifier,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VoiceParticipantPayload {
    pub channel_id: Identifier,
    pub participant: VoiceParticipant,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VoiceRoomInfoPayload {
    pub room: VoiceRoom,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReactionPayload {
    pub message_id: Identifier,
    pub emoji: String,
    pub user_id: Identifier,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PermissionsUpdatedPayload {
    pub server_id: Identifier,
    pub role_id: Option<Identifier>,
    pub channel_id: Option<Identifier>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServerStructureResponse {
    pub server: ServerStructure,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MessageHistoryPayload {
    pub channel_id: Identifier,
    pub messages: Vec<ChatMessage>,
    pub total_count: i64,
    pub has_more: bool,
    pub next_cursor: Option<Identifier>,
}

impl MessageHistoryPayload {
    pub fn new(channel_id: Identifier, history: MessageHistory) -> Self {
        Self {
            channel_id,
            messages: history.messages,
            total_count: history.total_count,
            has_more: history.has_more,
            next_cursor: history.next_cursor,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NewMessagePayload {
    pub message: ChatMessage,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MessageEditPayload {
    pub message: ChatMessage,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChannelEventPayload {
    pub server_id: Identifier,
    pub channel_id: Identifier,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CategoryEventPayload {
    pub server_id: Identifier,
    pub category_id: Identifier,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RoleEventPayload {
    pub server_id: Identifier,
    pub role_id: Identifier,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MemberRolePayload {
    pub server_id: Identifier,
    pub user_id: Identifier,
    pub roles: Vec<ServerRole>,
}
