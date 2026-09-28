use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct CreateCategory {
    pub name: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct UpdateCategory {
    pub name: Option<String>,
    pub position: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CategoryPermission {
    pub id: String,
    pub category_id: String,
    pub role: String,
    pub can_view: bool,
    pub can_send_messages: bool,
    pub can_manage_messages: bool,
}

