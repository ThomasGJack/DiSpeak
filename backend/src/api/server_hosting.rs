use crate::app::AppState;
use crate::domain::Server;
use crate::error::AppError;
use crate::services::auth::{jwt_kid, jwt_signing_mode, JwtSigningMode};
use crate::services::owner_keys::verify_owner_key;
use actix_web::{web, HttpRequest, HttpResponse};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{FromRow, Row};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct BootstrapRequest {
    pub server_id: String,
    pub owner_key: String,
    pub instance_name: Option<String>,
    pub advertised_host: Option<String>,
    pub advertised_port: Option<i32>,
    pub server_version: Option<String>,
    pub platform: Option<String>,
}

#[derive(Debug, Serialize)]
struct JwtVerificationInfo {
    mode: String,
    alg: String,
    jwks_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    kid: Option<String>,
}

#[derive(Debug, Serialize)]
struct BootstrapResponse {
    server_id: String,
    owner_user_id: String,
    owner_username: String,
    server: Server,
    runtime_token: String,
    runtime_token_expires_at: String,
    heartbeat_interval_sec: u64,
    jwt_verification: JwtVerificationInfo,
}

#[derive(Debug, Serialize, FromRow)]
struct MemberRoleRow {
    id: String,
    member_id: String,
    role_id: String,
    assigned_at: String,
}

#[derive(Debug, Serialize, FromRow)]
struct ChannelRoleOverrideRow {
    id: String,
    channel_id: String,
    role_id: String,
    allow_bitfield: i32,
    deny_bitfield: i32,
    created_at: String,
}

#[derive(Debug, Serialize, FromRow)]
struct CategoryRoleOverrideRow {
    id: String,
    category_id: String,
    role_id: String,
    allow_bitfield: i32,
    deny_bitfield: i32,
    created_at: String,
}

#[derive(Debug, Serialize, FromRow)]
struct MemberChannelOverrideRow {
    id: String,
    channel_id: String,
    member_id: String,
    allow_bitfield: i32,
    deny_bitfield: i32,
    created_at: String,
}

#[derive(Debug, Serialize, FromRow)]
struct MemberCategoryOverrideRow {
    id: String,
    category_id: String,
    member_id: String,
    allow_bitfield: i32,
    deny_bitfield: i32,
    created_at: String,
}

#[derive(Debug, Serialize, FromRow)]
struct ServerRoleRow {
    id: String,
    server_id: String,
    name: String,
    color: String,
    position: i32,
    permissions_bitfield: i32,
    is_default: bool,
    created_at: String,
}

#[derive(Debug, Serialize)]
struct HostingSnapshot {
    server: Server,
    members: Vec<dispeak_shared::types::ServerMember>,
    categories: Vec<dispeak_shared::types::Category>,
    channels: Vec<dispeak_shared::types::Channel>,
    roles: Vec<ServerRoleRow>,
    member_roles: Vec<MemberRoleRow>,
    channel_role_overrides: Vec<ChannelRoleOverrideRow>,
    category_role_overrides: Vec<CategoryRoleOverrideRow>,
    member_channel_overrides: Vec<MemberChannelOverrideRow>,
    member_category_overrides: Vec<MemberCategoryOverrideRow>,
    synced_at: String,
}

#[derive(Debug, Deserialize)]
pub struct HostingHeartbeatRequest {
    pub connected_clients: usize,
    pub status: String,
    pub actual_host: String,
    pub actual_port: i32,
    pub timestamp: String,
}

#[derive(Debug)]
struct RuntimeSession {
    id: String,
    server_id: String,
    owner_user_id: String,
}

fn hash_runtime_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

fn generate_runtime_token() -> String {
    let raw = Uuid::new_v4().to_string().replace('-', "");
    format!("dsrt_{}_{}", &raw[..12], Uuid::new_v4().to_string().replace('-', ""))
}

fn extract_bearer(req: &HttpRequest) -> Result<String, AppError> {
    let header = req
        .headers()
        .get("Authorization")
        .ok_or_else(|| AppError::Unauthorized("Authorization header manquant".to_string()))?;
    let header = header
        .to_str()
        .map_err(|_| AppError::Unauthorized("Authorization header invalide".to_string()))?;
    let token = header
        .strip_prefix("Bearer ")
        .ok_or_else(|| AppError::Unauthorized("Authorization header invalide".to_string()))?;
    if token.trim().is_empty() {
        return Err(AppError::Unauthorized("Token runtime manquant".to_string()));
    }
    Ok(token.to_string())
}

async fn validate_runtime_session(
    pool: &sqlx::SqlitePool,
    req: &HttpRequest,
    expected_server_id: &str,
) -> Result<RuntimeSession, AppError> {
    let token = extract_bearer(req)?;
    let token_hash = hash_runtime_token(&token);

    let row = sqlx::query(
        "SELECT id, server_id, owner_user_id, expires_at, is_active FROM server_runtime_sessions WHERE token_hash = ?"
    )
    .bind(token_hash)
    .fetch_optional(pool)
    .await?;

    let row = row.ok_or_else(|| AppError::Unauthorized("Session runtime invalide".to_string()))?;
    let server_id: String = row.get("server_id");
    let owner_user_id: String = row.get("owner_user_id");
    let expires_at: String = row.get("expires_at");
    let is_active: bool = row.get("is_active");
    let session_id: String = row.get("id");

    if !is_active {
        return Err(AppError::Unauthorized("Session runtime désactivée".to_string()));
    }

    if server_id != expected_server_id {
        return Err(AppError::Forbidden(
            "Session runtime non autorisée pour ce serveur".to_string(),
        ));
    }

    let expires_at_dt = chrono::DateTime::parse_from_rfc3339(&expires_at)
        .map_err(|_| AppError::internal("expires_at session runtime invalide"))?
        .with_timezone(&Utc);
    if expires_at_dt <= Utc::now() {
        return Err(AppError::Unauthorized("Session runtime expirée".to_string()));
    }

    let ip = req.connection_info().realip_remote_addr().map(|v| v.to_string());
    sqlx::query(
        "UPDATE server_runtime_sessions SET last_seen_at = datetime('now'), last_seen_ip = ? WHERE id = ?"
    )
    .bind(ip)
    .bind(&session_id)
    .execute(pool)
    .await?;

    Ok(RuntimeSession {
        id: session_id,
        server_id,
        owner_user_id,
    })
}

fn build_jwks_url(req: &HttpRequest) -> String {
    if let Ok(public_url) = std::env::var("BACKEND_PUBLIC_URL") {
        return format!("{}/api/auth/jwks", public_url.trim_end_matches('/'));
    }
    let info = req.connection_info();
    format!("{}://{}/api/auth/jwks", info.scheme(), info.host())
}

pub async fn bootstrap(
    state: web::Data<AppState>,
    req: HttpRequest,
    payload: web::Json<BootstrapRequest>,
) -> Result<HttpResponse, AppError> {
    let pool = state.pool();
    let payload = payload.into_inner();

    if payload.server_id.trim().is_empty() {
        return Err(AppError::BadRequest("server_id manquant".into()));
    }
    if payload.owner_key.trim().is_empty() {
        return Err(AppError::BadRequest("owner_key manquant".into()));
    }

    let peer_ip = req.connection_info().realip_remote_addr().map(|v| v.to_string());
    let verified = verify_owner_key(
        pool,
        &payload.owner_key,
        Some(&payload.server_id),
        peer_ip.as_deref(),
    )
    .await?;

    let user_row = sqlx::query("SELECT id, username, is_active, deleted_at FROM users WHERE id = ?")
        .bind(&verified.user_id)
        .fetch_optional(pool)
        .await?;
    let user_row = user_row.ok_or_else(|| AppError::Unauthorized("Utilisateur introuvable".into()))?;
    let is_active: bool = user_row.get("is_active");
    let deleted_at: Option<String> = user_row.get("deleted_at");
    if !is_active || deleted_at.is_some() {
        return Err(AppError::Forbidden("Compte utilisateur inactif".into()));
    }
    let owner_username: String = user_row.get("username");

    let server = sqlx::query_as::<_, Server>(
        "SELECT id, owner_id, name, host, port, max_users, is_online, last_heartbeat, actual_host, actual_port, icon, color, description, created_at FROM servers WHERE id = ?"
    )
    .bind(&payload.server_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Serveur introuvable".into()))?;

    if server.owner_id != verified.user_id {
        return Err(AppError::Forbidden(
            "La clé propriétaire ne correspond pas au propriétaire de ce serveur".into(),
        ));
    }

    // Désactiver les sessions actives précédentes pour ce serveur (1 runtime à la fois).
    sqlx::query(
        "UPDATE server_runtime_sessions SET is_active = 0 WHERE server_id = ? AND is_active = 1"
    )
    .bind(&payload.server_id)
    .execute(pool)
    .await?;

    let runtime_token = generate_runtime_token();
    let token_hash = hash_runtime_token(&runtime_token);
    let expires_at = (Utc::now() + Duration::hours(12)).to_rfc3339();
    let session_id = Uuid::new_v4().to_string();

    sqlx::query(
        "INSERT INTO server_runtime_sessions (id, server_id, owner_user_id, token_hash, expires_at, created_at, last_seen_at, last_seen_ip, instance_name, server_version, platform, is_active) VALUES (?, ?, ?, ?, ?, datetime('now'), datetime('now'), ?, ?, ?, ?, 1)"
    )
    .bind(&session_id)
    .bind(&payload.server_id)
    .bind(&verified.user_id)
    .bind(token_hash)
    .bind(&expires_at)
    .bind(peer_ip)
    .bind(payload.instance_name.clone())
    .bind(payload.server_version.clone())
    .bind(payload.platform.clone())
    .execute(pool)
    .await?;

    let heartbeat_interval_sec = std::env::var("SERVER_HOSTING_HEARTBEAT_INTERVAL_SEC")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(30);

    // Optional: persist actual host/port when provided.
    if let (Some(host), Some(port)) = (payload.advertised_host.as_ref(), payload.advertised_port) {
        let _ = sqlx::query(
            "UPDATE servers SET actual_host = ?, actual_port = ?, is_online = 0 WHERE id = ?"
        )
        .bind(host)
        .bind(port)
        .bind(&payload.server_id)
        .execute(pool)
        .await;
    }

    let signing_mode = jwt_signing_mode();
    let (mode, alg) = match signing_mode {
        JwtSigningMode::Rs256 => ("jwks".to_string(), "RS256".to_string()),
        JwtSigningMode::Hs256 => ("legacy_hs256".to_string(), "HS256".to_string()),
    };

    Ok(HttpResponse::Ok().json(BootstrapResponse {
        server_id: payload.server_id,
        owner_user_id: verified.user_id,
        owner_username,
        server,
        runtime_token,
        runtime_token_expires_at: expires_at,
        heartbeat_interval_sec,
        jwt_verification: JwtVerificationInfo {
            mode,
            alg,
            jwks_url: build_jwks_url(&req),
            kid: jwt_kid(),
        },
    }))
}

pub async fn snapshot(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let pool = state.pool();
    let server_id = path.into_inner();
    let _session = validate_runtime_session(pool, &req, &server_id).await?;

    let server = sqlx::query_as::<_, Server>(
        "SELECT id, owner_id, name, host, port, max_users, is_online, last_heartbeat, actual_host, actual_port, icon, color, description, created_at FROM servers WHERE id = ?"
    )
    .bind(&server_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Serveur introuvable".into()))?;

    let members = sqlx::query_as::<_, dispeak_shared::types::ServerMember>(
        "SELECT id, server_id, user_id, username, avatar, role, joined_at, last_seen, is_online FROM server_members WHERE server_id = ? ORDER BY joined_at ASC"
    )
    .bind(&server_id)
    .fetch_all(pool)
    .await?;

    let categories = sqlx::query_as::<_, dispeak_shared::types::Category>(
        "SELECT id, server_id, name, position, created_at FROM categories WHERE server_id = ? ORDER BY position ASC, created_at ASC"
    )
    .bind(&server_id)
    .fetch_all(pool)
    .await?;

    let channels = sqlx::query_as::<_, dispeak_shared::types::Channel>(
        "SELECT id, server_id, name, type, category_id, position, description, created_at FROM channels WHERE server_id = ? ORDER BY category_id ASC, position ASC, created_at ASC"
    )
    .bind(&server_id)
    .fetch_all(pool)
    .await?;

    let roles = sqlx::query_as::<_, ServerRoleRow>(
        "SELECT id, server_id, name, color, position, permissions_bitfield, is_default, created_at FROM server_roles WHERE server_id = ? ORDER BY position DESC, created_at ASC"
    )
    .bind(&server_id)
    .fetch_all(pool)
    .await?;

    let member_roles = sqlx::query_as::<_, MemberRoleRow>(
        "SELECT mr.id, mr.member_id, mr.role_id, mr.assigned_at FROM member_roles mr INNER JOIN server_members sm ON sm.id = mr.member_id WHERE sm.server_id = ?"
    )
    .bind(&server_id)
    .fetch_all(pool)
    .await?;

    let channel_role_overrides = sqlx::query_as::<_, ChannelRoleOverrideRow>(
        "SELECT cro.id, cro.channel_id, cro.role_id, cro.allow_bitfield, cro.deny_bitfield, cro.created_at FROM channel_role_overrides cro INNER JOIN channels c ON c.id = cro.channel_id WHERE c.server_id = ?"
    )
    .bind(&server_id)
    .fetch_all(pool)
    .await?;

    let category_role_overrides = sqlx::query_as::<_, CategoryRoleOverrideRow>(
        "SELECT cro.id, cro.category_id, cro.role_id, cro.allow_bitfield, cro.deny_bitfield, cro.created_at FROM category_role_overrides cro INNER JOIN categories c ON c.id = cro.category_id WHERE c.server_id = ?"
    )
    .bind(&server_id)
    .fetch_all(pool)
    .await?;

    let member_channel_overrides = sqlx::query_as::<_, MemberChannelOverrideRow>(
        "SELECT mco.id, mco.channel_id, mco.member_id, mco.allow_bitfield, mco.deny_bitfield, mco.created_at FROM member_channel_overrides mco INNER JOIN channels c ON c.id = mco.channel_id WHERE c.server_id = ?"
    )
    .bind(&server_id)
    .fetch_all(pool)
    .await?;

    let member_category_overrides = sqlx::query_as::<_, MemberCategoryOverrideRow>(
        "SELECT mco.id, mco.category_id, mco.member_id, mco.allow_bitfield, mco.deny_bitfield, mco.created_at FROM member_category_overrides mco INNER JOIN categories c ON c.id = mco.category_id WHERE c.server_id = ?"
    )
    .bind(&server_id)
    .fetch_all(pool)
    .await?;

    Ok(HttpResponse::Ok().json(HostingSnapshot {
        server,
        members,
        categories,
        channels,
        roles,
        member_roles,
        channel_role_overrides,
        category_role_overrides,
        member_channel_overrides,
        member_category_overrides,
        synced_at: Utc::now().to_rfc3339(),
    }))
}

pub async fn heartbeat(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    payload: web::Json<HostingHeartbeatRequest>,
) -> Result<HttpResponse, AppError> {
    let pool = state.pool();
    let server_id = path.into_inner();
    let session = validate_runtime_session(pool, &req, &server_id).await?;
    let payload = payload.into_inner();

    if payload.status.trim().is_empty() {
        return Err(AppError::BadRequest("status manquant".into()));
    }

    sqlx::query(
        "UPDATE servers SET is_online = 1, last_heartbeat = datetime('now'), actual_host = ?, actual_port = ? WHERE id = ?"
    )
    .bind(&payload.actual_host)
    .bind(payload.actual_port)
    .bind(&server_id)
    .execute(pool)
    .await?;

    sqlx::query(
        "UPDATE server_runtime_sessions SET last_seen_at = datetime('now') WHERE id = ?"
    )
    .bind(&session.id)
    .execute(pool)
    .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "Hosting heartbeat received",
        "server_id": server_id,
        "connected_clients": payload.connected_clients,
        "status": payload.status,
        "timestamp": payload.timestamp
    })))
}

pub async fn shutdown(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let pool = state.pool();
    let server_id = path.into_inner();
    let session = validate_runtime_session(pool, &req, &server_id).await?;

    sqlx::query("UPDATE servers SET is_online = 0 WHERE id = ?")
        .bind(&server_id)
        .execute(pool)
        .await?;

    sqlx::query("UPDATE server_runtime_sessions SET is_active = 0, last_seen_at = datetime('now') WHERE id = ?")
        .bind(&session.id)
        .execute(pool)
        .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "Hosting runtime session closed",
        "server_id": server_id,
        "owner_user_id": session.owner_user_id
    })))
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/server-hosting")
            .route("/bootstrap", web::post().to(bootstrap))
            .route("/{server_id}/snapshot", web::get().to(snapshot))
            .route("/{server_id}/heartbeat", web::post().to(heartbeat))
            .route("/{server_id}/shutdown", web::post().to(shutdown)),
    );
}
