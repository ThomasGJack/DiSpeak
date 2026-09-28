use chrono::Utc;
use dispeak_shared::types::Server;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct CreateServer {
    pub name: String,
    pub host: String,
    pub port: i32,
    pub max_users: i32,
}

#[derive(Debug, Deserialize, Default)]
pub struct UpdateServer {
    pub name: Option<String>,
    pub max_users: Option<i32>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AddMember {
    pub user_id: String,
    pub role: Option<String>,
}

pub fn build_server(
    owner_id: String,
    name: String,
    host: String,
    port: i32,
    max_users: i32,
) -> Server {
    Server {
        id: Uuid::new_v4().to_string(),
        owner_id,
        name,
        host,
        port,
        max_users,
        is_online: false,
        last_heartbeat: None,
        actual_host: None,
        actual_port: None,
        icon: None,
        color: Some("#5865F2".to_string()),
        description: None,
        created_at: Utc::now().to_rfc3339(),
    }
}

pub fn build_auto_server(server_id: String, owner_id: String, name: String) -> Server {
    Server {
        id: server_id,
        owner_id,
        name,
        host: "127.0.0.1".to_string(),
        port: 8001,
        max_users: 100,
        is_online: true,
        last_heartbeat: Some(Utc::now().to_rfc3339()),
        actual_host: Some("127.0.0.1".to_string()),
        actual_port: Some(8001),
        icon: None,
        color: Some("#5865F2".to_string()),
        description: Some("Serveur créé automatiquement".to_string()),
        created_at: Utc::now().to_rfc3339(),
    }
}
