use chrono::Utc;
use dispeak_shared::types::License;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct CreateLicense {
    pub user_id: String,
    pub server_slots: i32,
    pub expires_at: Option<String>,
}

pub fn build_license(user_id: String, server_slots: i32) -> License {
    License {
        id: Uuid::new_v4().to_string(),
        user_id,
        server_slots,
        active: true,
        expires_at: None,
        created_at: Utc::now().to_rfc3339(),
    }
}
