pub mod auth;
pub mod category;
pub mod channel;
pub mod license;
pub mod role;
pub mod server;
pub mod user;

pub use dispeak_shared::types::{
    Category, CategoryWithChannels, Channel, ChatMessage, License, MessageHistory, Server,
    ServerMember, ServerMemberWithRoles, ServerRole, ServerRoleWithMembers, ServerStructure, User,
    VoiceParticipant, VoiceRoom,
};

pub use auth::{AuthResponse, Claims, CreateUser, LoginUser};
pub use category::{CategoryPermission, CreateCategory, UpdateCategory};
pub use channel::{ChannelPermission, CreateChannel, UpdateChannel};
pub use license::{build_license, CreateLicense};
pub use role::{AssignRoles, CreateRole, UpdateRole};
pub use server::{build_auto_server, build_server, AddMember, CreateServer, UpdateServer};
pub use user::{build_local_user, build_oauth_user, UpdateMemberRole};

pub mod permissions {
    pub use dispeak_shared::types::permissions::*;
}
