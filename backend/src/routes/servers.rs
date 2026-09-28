use actix_web::{web, HttpResponse, Result};
use serde::Deserialize;
use serde_json::json;
use sqlx::{QueryBuilder, Row, SqlitePool};
use std::env;
use uuid::Uuid;

use crate::domain::{
    build_server, Claims, CreateServer, Server, ServerMember, UpdateMemberRole, UpdateServer,
};
use crate::repositories::log_audit;
// Permissions functions are imported but not used yet - keeping for future features
use crate::api::channels;

#[derive(Debug, Deserialize)]
pub struct ServerHeartbeat {
    pub actual_host: String,
    pub actual_port: i32,
}

fn is_dev_like_backend_env() -> bool {
    matches!(
        env::var("BACKEND_ENV")
            .or_else(|_| env::var("APP_ENV"))
            .unwrap_or_else(|_| "development".to_string())
            .to_lowercase()
            .as_str(),
        "dev" | "development" | "local" | "test"
    )
}

fn allow_register_auto_system_server() -> bool {
    env::var("BACKEND_ALLOW_REGISTER_AUTO_SYSTEM_SERVER")
        .ok()
        .map(|v| v.eq_ignore_ascii_case("true"))
        .unwrap_or_else(is_dev_like_backend_env)
}

fn allow_legacy_heartbeat_token() -> bool {
    env::var("BACKEND_ALLOW_LEGACY_HEARTBEAT_TOKEN")
        .ok()
        .map(|v| v.eq_ignore_ascii_case("true"))
        .unwrap_or_else(is_dev_like_backend_env)
}

pub async fn create_server(
    pool: web::Data<SqlitePool>,
    claims: Claims,
    server_data: web::Json<CreateServer>,
) -> Result<HttpResponse> {
    // Vérifier que l'utilisateur a une licence valide
    let license_check = sqlx::query(
        "SELECT id FROM licenses WHERE user_id = ? AND active = 1 AND (expires_at IS NULL OR expires_at > datetime('now'))"
    )
    .bind(&claims.user_id)
    .fetch_optional(pool.get_ref())
    .await
    .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    if license_check.is_none() {
        return Ok(HttpResponse::Forbidden().json(serde_json::json!({
            "error": "Vous devez avoir une licence valide pour créer un serveur"
        })));
    }

    let payload = server_data.into_inner();

    let mut tx = pool
        .begin()
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    let server = build_server(
        claims.user_id.clone(),
        payload.name,
        payload.host,
        payload.port,
        payload.max_users,
    );

    sqlx::query(
        "INSERT INTO servers (id, owner_id, name, host, port, max_users, is_online, last_heartbeat, actual_host, actual_port, icon, color, description, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&server.id)
    .bind(&server.owner_id)
    .bind(&server.name)
    .bind(&server.host)
    .bind(&server.port)
    .bind(&server.max_users)
    .bind(&server.is_online)
    .bind(&server.last_heartbeat)
    .bind(&server.actual_host)
    .bind(&server.actual_port)
    .bind(&server.icon)
    .bind(&server.color)
    .bind(&server.description)
    .bind(&server.created_at)
    .execute(&mut *tx)
    .await
    .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    let owner_row = sqlx::query("SELECT username, avatar FROM users WHERE id = ?")
        .bind(&server.owner_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    let owner_username: String = owner_row.get("username");
    let owner_avatar: Option<String> = owner_row.get("avatar");

    let member_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO server_members (id, server_id, user_id, username, avatar, role, joined_at, last_seen, is_online) VALUES (?, ?, ?, ?, ?, 'owner', datetime('now'), datetime('now'), 0)"
    )
    .bind(&member_id)
    .bind(&server.id)
    .bind(&server.owner_id)
    .bind(&owner_username)
    .bind(&owner_avatar)
    .execute(&mut *tx)
    .await
    .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    tx.commit()
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    let _ = log_audit(
        pool.get_ref(),
        &server.owner_id,
        "server_created",
        None,
        Some(&format!("Server {} created", server.name)),
        None,
    )
    .await;

    Ok(HttpResponse::Created().json(server))
}

pub async fn get_servers(pool: web::Data<SqlitePool>) -> Result<HttpResponse> {
    let servers = sqlx::query_as::<_, Server>(
        "SELECT id, owner_id, name, host, port, max_users, is_online, last_heartbeat, actual_host, actual_port, icon, color, description, created_at FROM servers"
    )
    .fetch_all(pool.get_ref())
    .await
    .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    Ok(HttpResponse::Ok().json(servers))
}

pub async fn get_server(
    pool: web::Data<SqlitePool>,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let server_id = path.into_inner();

    let server = sqlx::query_as::<_, Server>(
        "SELECT id, owner_id, name, host, port, max_users, is_online, last_heartbeat, actual_host, actual_port, icon, color, description, created_at FROM servers WHERE id = ?"
    )
    .bind(&server_id)
    .fetch_optional(pool.get_ref())
    .await
    .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    match server {
        Some(server) => Ok(HttpResponse::Ok().json(server)),
        None => Ok(HttpResponse::NotFound().json(json!({
            "error": "Server not found"
        }))),
    }
}

pub async fn delete_server(
    pool: web::Data<SqlitePool>,
    claims: Claims,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let server_id = path.into_inner();

    let result = sqlx::query("DELETE FROM servers WHERE id = ? AND owner_id = ?")
        .bind(&server_id)
        .bind(&claims.user_id)
        .execute(pool.get_ref())
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    if result.rows_affected() > 0 {
        Ok(HttpResponse::Ok().json(serde_json::json!({
            "message": "Server deleted successfully"
        })))
    } else {
        Ok(HttpResponse::NotFound().json(serde_json::json!({
            "error": "Server not found or not owned by user"
        })))
    }
}

pub async fn get_my_servers(pool: web::Data<SqlitePool>, claims: Claims) -> Result<HttpResponse> {
    let servers = sqlx::query_as::<_, Server>(
        "SELECT id, owner_id, name, host, port, max_users, is_online, last_heartbeat, actual_host, actual_port, icon, color, description, created_at FROM servers WHERE owner_id = ?"
    )
    .bind(&claims.user_id)
    .fetch_all(pool.get_ref())
    .await
    .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    Ok(HttpResponse::Ok().json(servers))
}

pub async fn join_server(
    pool: web::Data<SqlitePool>,
    _claims: Claims,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let server_id = path.into_inner();

    // Vérifier que le serveur existe
    let server_row =
        sqlx::query("SELECT id, is_online, actual_host, actual_port FROM servers WHERE id = ?")
            .bind(&server_id)
            .fetch_optional(pool.get_ref())
            .await
            .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    match server_row {
        Some(row) => {
            let is_online: bool = row.get("is_online");
            let actual_host: Option<String> = row.get("actual_host");
            let actual_port: Option<i32> = row.get("actual_port");

            if !is_online {
                return Ok(HttpResponse::ServiceUnavailable().json(json!({
                    "error": "Le serveur est actuellement hors ligne"
                })));
            }

            Ok(HttpResponse::Ok().json(json!({
                "message": "You can now connect to this server",
                "server_id": server_id,
                "host": actual_host.unwrap_or_else(|| "127.0.0.1".to_string()),
                "port": actual_port.unwrap_or(8001)
            })))
        }
        None => Ok(HttpResponse::NotFound().json(json!({
            "error": "Server not found"
        }))),
    }
}

#[derive(Debug, Deserialize)]
pub struct ServerHeartbeatData {
    #[allow(dead_code)]
    pub server_id: String,
    #[allow(dead_code)]
    pub status: String,
    pub connected_clients: usize,
    #[allow(dead_code)]
    pub timestamp: String,
}

pub async fn register_server_auto(
    pool: web::Data<SqlitePool>,
    path: web::Path<String>,
    server_data: web::Json<ServerHeartbeatData>,
) -> Result<HttpResponse> {
    if !allow_register_auto_system_server() {
        eprintln!("⚠️ register-auto refusé: endpoint legacy désactivé (utiliser /api/server-hosting/bootstrap)");
        return Ok(HttpResponse::Gone().json(json!({
            "error": "Legacy endpoint register-auto désactivé. Utilisez le bootstrap /api/server-hosting/bootstrap."
        })));
    }

    let server_id = path.into_inner();

    eprintln!("⚠️ register-auto legacy utilisé (déprécié)");
    println!(
        "🔄 Tentative d'enregistrement automatique du serveur: {}",
        server_id
    );

    // Vérifier si le serveur existe déjà
    let existing_server = sqlx::query("SELECT id FROM servers WHERE id = ?")
        .bind(&server_id)
        .fetch_optional(pool.get_ref())
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    if existing_server.is_none() {
        // Créer le serveur automatiquement
        let server_name = format!("Serveur Auto-{}", &server_id[..8]);

        // Créer d'abord un utilisateur système s'il n'existe pas
        let system_user_id = "system-user";
        let system_user_exists = sqlx::query("SELECT id FROM users WHERE id = ?")
            .bind(system_user_id)
            .fetch_optional(pool.get_ref())
            .await
            .map_err(|e| {
                println!("❌ Erreur vérification utilisateur système: {}", e);
                actix_web::error::ErrorInternalServerError("Database error")
            })?;

        if system_user_exists.is_none() {
            // Créer l'utilisateur système
            let now = chrono::Utc::now().to_rfc3339();

            // Obtenir un user_number valide pour le système
            let system_user_number =
                sqlx::query_scalar::<_, i32>("SELECT COALESCE(MAX(user_number), 0) + 1 FROM users")
                    .fetch_one(pool.get_ref())
                    .await
                    .map_err(|e| {
                        println!("❌ Erreur obtention user_number: {}", e);
                        actix_web::error::ErrorInternalServerError("Database error")
                    })?;

            sqlx::query(
                "INSERT INTO users (id, user_number, username, email, provider, password_hash, status, role, is_active, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(system_user_id)
            .bind(system_user_number)
            .bind("Système")
            .bind("system@dispeak.local")
            .bind("local")
            .bind("system") // Hash simple pour le système
            .bind("offline")
            .bind("admin")
            .bind(true)
            .bind(&now)
            .execute(pool.get_ref())
            .await
            .map_err(|e| {
                println!("❌ Erreur création utilisateur système: {}", e);
                actix_web::error::ErrorInternalServerError("Database error")
            })?;

            println!(
                "✅ Utilisateur système créé avec user_number: {}",
                system_user_number
            );
        }

        sqlx::query(
            "INSERT INTO servers (id, owner_id, name, host, port, max_users, is_online, last_heartbeat, actual_host, actual_port, icon, color, description, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&server_id)
        .bind(system_user_id) // Utiliser l'ID de l'utilisateur système
        .bind(&server_name)
        .bind("127.0.0.1")
        .bind(8001)
        .bind(100)
        .bind(true)
        .bind(chrono::Utc::now().to_rfc3339())
        .bind("127.0.0.1")
        .bind(8001)
        .bind(None::<String>)
        .bind(Some("#5865F2".to_string()))
        .bind(Some("Serveur créé automatiquement".to_string()))
        .bind(chrono::Utc::now().to_rfc3339())
        .execute(pool.get_ref())
        .await
        .map_err(|e| {
            println!("❌ Erreur création serveur: {}", e);
            actix_web::error::ErrorInternalServerError("Database error")
        })?;

        println!(
            "✅ Serveur auto-enregistré: {} ({})",
            server_name, server_id
        );
    }

    // Mettre à jour le statut du serveur
    sqlx::query("UPDATE servers SET is_online = 1, last_heartbeat = datetime('now') WHERE id = ?")
        .bind(&server_id)
        .execute(pool.get_ref())
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    println!(
        "✅ Heartbeat traité avec succès pour le serveur: {}",
        server_id
    );

    Ok(HttpResponse::Ok().json(json!({
        "message": "Heartbeat received",
        "server_id": server_id,
        "is_online": true,
        "connected_clients": server_data.connected_clients
    })))
}

pub async fn server_heartbeat_token(
    pool: web::Data<SqlitePool>,
    path: web::Path<String>,
    heartbeat_data: web::Json<ServerHeartbeatData>,
) -> Result<HttpResponse> {
    if !allow_legacy_heartbeat_token() {
        eprintln!("⚠️ heartbeat-token refusé: endpoint legacy désactivé (utiliser /api/server-hosting/*)");
        return Ok(HttpResponse::Gone().json(json!({
            "error": "Legacy endpoint heartbeat-token désactivé. Utilisez /api/server-hosting/{server_id}/heartbeat avec runtime_token."
        })));
    }

    eprintln!("⚠️ heartbeat-token legacy utilisé (déprécié)");
    let server_id = path.into_inner();

    // Vérifier que le serveur existe
    let server_check = sqlx::query("SELECT id FROM servers WHERE id = ?")
        .bind(&server_id)
        .fetch_optional(pool.get_ref())
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    if server_check.is_none() {
        return Ok(HttpResponse::NotFound().json(json!({
            "error": "Server not found"
        })));
    }

    // Mettre à jour le serveur comme étant en ligne
    sqlx::query("UPDATE servers SET is_online = 1, last_heartbeat = datetime('now') WHERE id = ?")
        .bind(&server_id)
        .execute(pool.get_ref())
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    Ok(HttpResponse::Ok().json(json!({
        "message": "Heartbeat received",
        "server_id": server_id,
        "is_online": true,
        "connected_clients": heartbeat_data.connected_clients
    })))
}

pub async fn server_heartbeat(
    pool: web::Data<SqlitePool>,
    claims: Claims,
    path: web::Path<String>,
    heartbeat_data: web::Json<ServerHeartbeat>,
) -> Result<HttpResponse> {
    let server_id = path.into_inner();

    // Vérifier que l'utilisateur est propriétaire du serveur
    let server_check = sqlx::query("SELECT owner_id FROM servers WHERE id = ?")
        .bind(&server_id)
        .fetch_optional(pool.get_ref())
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    match server_check {
        Some(row) => {
            let owner_id: String = row.get("owner_id");
            if owner_id != claims.user_id {
                return Ok(HttpResponse::Forbidden().json(json!({
                    "error": "Only server owner can send heartbeat"
                })));
            }
        }
        None => {
            return Ok(HttpResponse::NotFound().json(json!({
                "error": "Server not found"
            })));
        }
    }

    // Mettre à jour le serveur comme étant en ligne
    sqlx::query(
        "UPDATE servers SET is_online = 1, last_heartbeat = datetime('now'), actual_host = ?, actual_port = ? WHERE id = ?"
    )
    .bind(&heartbeat_data.actual_host)
    .bind(&heartbeat_data.actual_port)
    .bind(&server_id)
    .execute(pool.get_ref())
    .await
    .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    Ok(HttpResponse::Ok().json(json!({
        "message": "Heartbeat received",
        "is_online": true
    })))
}

// Gestion des membres du serveur
pub async fn get_server_members(
    pool: web::Data<SqlitePool>,
    claims: Claims,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let server_id = path.into_inner();

    // Vérifier que l'utilisateur est membre du serveur
    let member_check =
        sqlx::query("SELECT role FROM server_members WHERE server_id = ? AND user_id = ?")
            .bind(&server_id)
            .bind(&claims.user_id)
            .fetch_optional(pool.get_ref())
            .await
            .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    if member_check.is_none() {
        return Ok(HttpResponse::Forbidden().json(json!({
            "error": "Not a member of this server"
        })));
    }

    let members = sqlx::query_as::<_, ServerMember>(
        "SELECT id, server_id, user_id, username, avatar, role, joined_at, last_seen, is_online FROM server_members WHERE server_id = ? ORDER BY (role = 'owner') DESC, (role = 'moderator') DESC, is_online DESC, joined_at ASC"
    )
    .bind(&server_id)
    .fetch_all(pool.get_ref())
    .await
    .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    Ok(HttpResponse::Ok().json(members))
}

pub async fn add_server_member(
    pool: web::Data<SqlitePool>,
    claims: Claims,
    path: web::Path<String>,
    member_data: web::Json<serde_json::Value>,
) -> Result<HttpResponse> {
    let server_id = path.into_inner();

    // Vérifier que l'utilisateur est propriétaire du serveur
    let owner_check = sqlx::query(
        "SELECT role FROM server_members WHERE server_id = ? AND user_id = ? AND role = 'owner'",
    )
    .bind(&server_id)
    .bind(&claims.user_id)
    .fetch_optional(pool.get_ref())
    .await
    .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    if owner_check.is_none() {
        return Ok(HttpResponse::Forbidden().json(json!({
            "error": "Only server owner can add members"
        })));
    }

    let user_id = member_data
        .get("user_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| actix_web::error::ErrorBadRequest("Missing user_id"))?;

    // Vérifier que l'utilisateur existe
    let user = sqlx::query(
        "SELECT username, avatar FROM users WHERE id = ? AND is_active = 1 AND deleted_at IS NULL",
    )
    .bind(user_id)
    .fetch_optional(pool.get_ref())
    .await
    .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    let user = match user {
        Some(u) => u,
        None => {
            return Ok(HttpResponse::NotFound().json(json!({
                "error": "User not found"
            })))
        }
    };

    // Vérifier que le membre n'est pas déjà présent
    let existing_member =
        sqlx::query("SELECT 1 FROM server_members WHERE server_id = ? AND user_id = ?")
            .bind(&server_id)
            .bind(user_id)
            .fetch_optional(pool.get_ref())
            .await
            .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    if existing_member.is_some() {
        return Ok(HttpResponse::Conflict().json(json!({
            "error": "User is already a member of this server"
        })));
    }

    // Ajouter le membre
    let member_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO server_members (id, server_id, user_id, username, avatar, role, joined_at, last_seen, is_online) VALUES (?, ?, ?, ?, ?, 'user', datetime('now'), datetime('now'), 0)"
    )
    .bind(&member_id)
    .bind(&server_id)
    .bind(user_id)
    .bind(user.get::<String, _>("username"))
    .bind(user.get::<Option<String>, _>("avatar"))
    .execute(pool.get_ref())
    .await
    .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    let _ = log_audit(
        pool.get_ref(),
        user_id,
        "member_added",
        Some(&claims.user_id),
        Some(&format!("Added to server {}", server_id)),
        None,
    )
    .await;

    Ok(HttpResponse::Ok().json(json!({
        "message": "Member added successfully"
    })))
}

pub async fn update_member_role(
    pool: web::Data<SqlitePool>,
    claims: Claims,
    path: web::Path<(String, String)>,
    role_data: web::Json<UpdateMemberRole>,
) -> Result<HttpResponse> {
    let (server_id, user_id) = path.into_inner();

    // Vérifier que l'utilisateur est propriétaire du serveur
    let owner_check = sqlx::query(
        "SELECT role FROM server_members WHERE server_id = ? AND user_id = ? AND role = 'owner'",
    )
    .bind(&server_id)
    .bind(&claims.user_id)
    .fetch_optional(pool.get_ref())
    .await
    .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    if owner_check.is_none() {
        return Ok(HttpResponse::Forbidden().json(json!({
            "error": "Only server owner can change roles"
        })));
    }

    // Valider le rôle
    if !["owner", "moderator", "user"].contains(&role_data.role.as_str()) {
        return Ok(HttpResponse::BadRequest().json(json!({
            "error": "Invalid role"
        })));
    }

    // Mettre à jour le rôle
    sqlx::query("UPDATE server_members SET role = ? WHERE server_id = ? AND user_id = ?")
        .bind(&role_data.role)
        .bind(&server_id)
        .bind(&user_id)
        .execute(pool.get_ref())
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    let _ = log_audit(
        pool.get_ref(),
        &user_id,
        "role_changed",
        Some(&claims.user_id),
        Some(&format!(
            "Role changed to {} in server {}",
            role_data.role, server_id
        )),
        None,
    )
    .await;

    Ok(HttpResponse::Ok().json(json!({
        "message": "Role updated successfully"
    })))
}

pub async fn remove_server_member(
    pool: web::Data<SqlitePool>,
    claims: Claims,
    path: web::Path<(String, String)>,
) -> Result<HttpResponse> {
    let (server_id, user_id) = path.into_inner();

    // Vérifier que l'utilisateur est propriétaire ou modérateur
    let member_check = sqlx::query(
        "SELECT role FROM server_members WHERE server_id = ? AND user_id = ? AND role IN ('owner', 'moderator')"
    )
    .bind(&server_id)
    .bind(&claims.user_id)
    .fetch_optional(pool.get_ref())
    .await
    .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    if member_check.is_none() {
        return Ok(HttpResponse::Forbidden().json(json!({
            "error": "Insufficient permissions"
        })));
    }

    // Supprimer le membre
    sqlx::query("DELETE FROM server_members WHERE server_id = ? AND user_id = ?")
        .bind(&server_id)
        .bind(&user_id)
        .execute(pool.get_ref())
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    let _ = log_audit(
        pool.get_ref(),
        &user_id,
        "member_removed",
        Some(&claims.user_id),
        Some(&format!("Removed from server {}", server_id)),
        None,
    )
    .await;

    Ok(HttpResponse::Ok().json(json!({
        "message": "Member removed successfully"
    })))
}

pub async fn update_server_settings(
    pool: web::Data<SqlitePool>,
    claims: Claims,
    path: web::Path<String>,
    settings_data: web::Json<UpdateServer>,
) -> Result<HttpResponse> {
    let server_id = path.into_inner();

    // Vérifier que l'utilisateur est propriétaire du serveur
    let owner_check = sqlx::query(
        "SELECT role FROM server_members WHERE server_id = ? AND user_id = ? AND role = 'owner'",
    )
    .bind(&server_id)
    .bind(&claims.user_id)
    .fetch_optional(pool.get_ref())
    .await
    .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    if owner_check.is_none() {
        return Ok(HttpResponse::Forbidden().json(json!({
            "error": "Only server owner can update settings"
        })));
    }

    // Construire la requête de mise à jour dynamiquement
    let mut query_builder = QueryBuilder::new("UPDATE servers SET ");
    let mut has_updates = false;

    {
        let mut separated = query_builder.separated(", ");

        if let Some(name) = &settings_data.name {
            separated.push("name = ");
            separated.push_bind(name);
            has_updates = true;
        }
        if let Some(max_users) = settings_data.max_users {
            if max_users < 1 || max_users > 10_000 {
                return Ok(HttpResponse::BadRequest().json(json!({
                    "error": "max_users doit être compris entre 1 et 10000"
                })));
            }
            separated.push("max_users = ");
            separated.push_bind(max_users);
            has_updates = true;
        }
        if let Some(icon) = &settings_data.icon {
            separated.push("icon = ");
            separated.push_bind(icon);
            has_updates = true;
        }
        if let Some(color) = &settings_data.color {
            separated.push("color = ");
            separated.push_bind(color);
            has_updates = true;
        }
        if let Some(description) = &settings_data.description {
            separated.push("description = ");
            separated.push_bind(description);
            has_updates = true;
        }
    }

    if !has_updates {
        return Ok(HttpResponse::BadRequest().json(json!({
            "error": "No fields to update"
        })));
    }

    query_builder.push(" WHERE id = ");
    query_builder.push_bind(&server_id);

    query_builder
        .build()
        .execute(pool.get_ref())
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    let _ = log_audit(
        pool.get_ref(),
        &claims.user_id,
        "server_updated",
        None,
        Some(&format!("Updated server {} settings", server_id)),
        None,
    )
    .await;

    Ok(HttpResponse::Ok().json(json!({
        "message": "Server settings updated successfully"
    })))
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/servers")
            .route("", web::post().to(create_server))
            .route("", web::get().to(get_servers))
            .route("/my", web::get().to(get_my_servers))
            .route("/{server_id}", web::get().to(get_server))
            .route("/{server_id}", web::delete().to(delete_server))
            .route("/{server_id}/join", web::post().to(join_server))
            .route("/{server_id}/heartbeat", web::post().to(server_heartbeat))
            .route(
                "/{server_id}/heartbeat-token",
                web::post().to(server_heartbeat_token),
            )
            .route(
                "/{server_id}/register-auto",
                web::post().to(register_server_auto),
            )
            .route("/{server_id}/members", web::get().to(get_server_members))
            .route("/{server_id}/members", web::post().to(add_server_member))
            .route(
                "/{server_id}/members/{user_id}/role",
                web::put().to(update_member_role),
            )
            .route(
                "/{server_id}/members/{user_id}",
                web::delete().to(remove_server_member),
            )
            .route(
                "/{server_id}/settings",
                web::put().to(update_server_settings),
            )
            .route(
                "/{server_id}/channels",
                web::get().to(channels::get_server_channels),
            )
            .route(
                "/{server_id}/channels",
                web::post().to(channels::create_channel),
            )
            .route(
                "/{server_id}/categories",
                web::get().to(channels::get_server_categories),
            )
            .route(
                "/{server_id}/categories",
                web::post().to(channels::create_category),
            )
            .route(
                "/{server_id}/structure",
                web::get().to(channels::get_server_structure),
            )
            .route(
                "/{server_id}/categories/{category_id}/position",
                web::put().to(channels::update_category_position),
            )
            .route("/{server_id}/members", web::get().to(get_server_members))
            .route(
                "/{server_id}/members-with-roles",
                web::get().to(get_server_members_with_roles),
            )
            .route("/{server_id}/roles", web::get().to(get_server_roles))
            .route("/{server_id}/settings", web::get().to(get_server_settings)),
    );
}

// Fonctions pour les endpoints manquants
pub async fn get_server_members_with_roles(
    pool: web::Data<SqlitePool>,
    claims: Claims,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let server_id = path.into_inner();

    // Vérifier que l'utilisateur est membre du serveur OU propriétaire
    let member_check =
        sqlx::query("SELECT 1 FROM server_members WHERE server_id = ? AND user_id = ?")
            .bind(&server_id)
            .bind(&claims.user_id)
            .fetch_optional(pool.get_ref())
            .await
            .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    if member_check.is_none() {
        let owner_check = sqlx::query("SELECT 1 FROM servers WHERE id = ? AND owner_id = ?")
            .bind(&server_id)
            .bind(&claims.user_id)
            .fetch_optional(pool.get_ref())
            .await
            .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

        if owner_check.is_none() {
            return Ok(HttpResponse::Forbidden().json(json!({
                "error": "Not a member of this server"
            })));
        }
    }

    // Récupérer les membres avec leurs rôles
    let members_with_roles = sqlx::query_as::<_, ServerMember>(
        "SELECT id, server_id, user_id, username, avatar, role, joined_at, last_seen, is_online FROM server_members WHERE server_id = ? ORDER BY joined_at ASC"
    )
    .bind(&server_id)
    .fetch_all(pool.get_ref())
    .await
    .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    Ok(HttpResponse::Ok().json(members_with_roles))
}

pub async fn get_server_roles(
    pool: web::Data<SqlitePool>,
    claims: Claims,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let server_id = path.into_inner();

    // Vérifier que l'utilisateur est membre du serveur OU propriétaire
    let member_check =
        sqlx::query("SELECT 1 FROM server_members WHERE server_id = ? AND user_id = ?")
            .bind(&server_id)
            .bind(&claims.user_id)
            .fetch_optional(pool.get_ref())
            .await
            .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    if member_check.is_none() {
        let owner_check = sqlx::query("SELECT 1 FROM servers WHERE id = ? AND owner_id = ?")
            .bind(&server_id)
            .bind(&claims.user_id)
            .fetch_optional(pool.get_ref())
            .await
            .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

        if owner_check.is_none() {
            return Ok(HttpResponse::Forbidden().json(json!({
                "error": "Not a member of this server"
            })));
        }
    }

    // Pour l'instant, retourner un tableau vide car les rôles ne sont pas encore implémentés
    Ok(HttpResponse::Ok().json(json!([])))
}

pub async fn get_server_settings(
    pool: web::Data<SqlitePool>,
    claims: Claims,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let server_id = path.into_inner();

    // Vérifier que l'utilisateur est membre du serveur OU propriétaire
    let member_check =
        sqlx::query("SELECT 1 FROM server_members WHERE server_id = ? AND user_id = ?")
            .bind(&server_id)
            .bind(&claims.user_id)
            .fetch_optional(pool.get_ref())
            .await
            .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    if member_check.is_none() {
        let owner_check = sqlx::query("SELECT 1 FROM servers WHERE id = ? AND owner_id = ?")
            .bind(&server_id)
            .bind(&claims.user_id)
            .fetch_optional(pool.get_ref())
            .await
            .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

        if owner_check.is_none() {
            return Ok(HttpResponse::Forbidden().json(json!({
                "error": "Not a member of this server"
            })));
        }
    }

    let server = sqlx::query_as::<_, Server>(
        "SELECT id, owner_id, name, host, port, max_users, is_online, last_heartbeat, actual_host, actual_port, icon, color, description, created_at FROM servers WHERE id = ?"
    )
    .bind(&server_id)
    .fetch_optional(pool.get_ref())
    .await
    .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    match server {
        Some(server) => Ok(HttpResponse::Ok().json(server)),
        None => Ok(HttpResponse::NotFound().json(json!({
            "error": "Server not found"
        }))),
    }
}
