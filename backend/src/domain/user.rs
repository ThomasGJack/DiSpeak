use chrono::Utc;
use dispeak_shared::types::User;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct UpdateMemberRole {
    pub role: String,
}

pub fn build_local_user(username: String, email: String, user_number: i32) -> User {
    User {
        id: Uuid::new_v4().to_string(),
        user_number,
        username,
        email,
        provider: "local".to_string(),
        provider_id: None,
        avatar: None,
        status: "offline".to_string(),
        role: "user".to_string(),
        bio: None,
        is_active: true,
        deleted_at: None,
        created_at: Utc::now().to_rfc3339(),
    }
}

pub fn build_oauth_user(
    username: String,
    email: String,
    provider: String,
    provider_id: String,
    avatar: Option<String>,
    user_number: i32,
) -> User {
    User {
        id: Uuid::new_v4().to_string(),
        user_number,
        username,
        email,
        provider,
        provider_id: Some(provider_id),
        avatar,
        status: "offline".to_string(),
        role: "user".to_string(),
        bio: None,
        is_active: true,
        deleted_at: None,
        created_at: Utc::now().to_rfc3339(),
    }
}
