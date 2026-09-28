use dispeak_shared::events::ClientEvent;
use dispeak_shared::types::{Category as SharedCategory, Channel as SharedChannel, Server as SharedServer, ServerMember as SharedServerMember};
use config::{load_runtime_config, parse_cli_args, CliCommand, RuntimeConfig, CONFIG_EXAMPLE};
use dotenv::dotenv;
use futures_util::{stream::SplitSink, SinkExt, StreamExt};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use sqlx::{Row, Sqlite, SqlitePool};
use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio_tungstenite::{
    accept_hdr_async,
    tungstenite::{
        handshake::server::{ErrorResponse, Request, Response},
        http::StatusCode,
        Message as WsMessage,
    },
};
use uuid::Uuid;

mod events;
mod config;
mod http_api;
mod models;
mod permissions;
mod services;
mod voip;

use models::*;
use permissions::*;
use voip::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketMessage {
    pub action: String,
    pub data: serde_json::Value,
}

pub type WebSocketSender =
    Arc<Mutex<SplitSink<tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>, WsMessage>>>;

#[derive(Debug, Clone)]
pub struct Client {
    pub id: String,
    pub authenticated_user_id: String,
    pub user_id: Option<String>,
    pub username: Option<String>,
    pub current_channel: Option<String>,
    pub current_server: Option<String>,
    pub sender: WebSocketSender,
}

#[derive(Debug, Clone)]
pub struct ServerState {
    pub clients: Arc<RwLock<HashMap<String, Client>>>,
    pub pool: SqlitePool,
    pub sfu: SFU,
    pub presence_connections: Arc<RwLock<HashMap<String, usize>>>,
    pub presence_seq: Arc<RwLock<HashMap<String, u64>>>,
    pub jwt_validation: Arc<JwtValidationConfig>,
}

#[derive(Debug, Clone, Deserialize)]
struct JwtClaims {
    user_id: String,
    exp: usize,
}

#[derive(Debug, Clone)]
pub enum JwtValidationConfig {
    Hs256 { secret: String, allow_insecure_defaults: bool, enforce_secret: bool },
    Rs256 { modulus_b64url: String, exponent_b64url: String },
}

#[derive(Debug, Clone, Deserialize)]
struct HostingBootstrapJwtVerification {
    mode: String,
    alg: String,
    jwks_url: String,
    #[serde(default)]
    kid: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct HostingBootstrapResponse {
    server_id: String,
    owner_user_id: String,
    owner_username: String,
    server: SharedServer,
    runtime_token: String,
    runtime_token_expires_at: String,
    heartbeat_interval_sec: u64,
    jwt_verification: HostingBootstrapJwtVerification,
}

#[derive(Debug, Clone, Deserialize)]
struct SnapshotServerRoleRow {
    id: String,
    server_id: String,
    name: String,
    color: String,
    position: i32,
    permissions_bitfield: i32,
    is_default: bool,
    created_at: String,
}

#[derive(Debug, Clone, Deserialize)]
struct SnapshotMemberRoleRow {
    id: String,
    member_id: String,
    role_id: String,
    assigned_at: String,
}

#[derive(Debug, Clone, Deserialize)]
struct SnapshotOverrideRow {
    id: String,
    #[serde(default)]
    channel_id: Option<String>,
    #[serde(default)]
    category_id: Option<String>,
    #[serde(default)]
    role_id: Option<String>,
    #[serde(default)]
    member_id: Option<String>,
    allow_bitfield: i32,
    deny_bitfield: i32,
    created_at: String,
}

#[derive(Debug, Clone, Deserialize)]
struct HostingSnapshotResponse {
    server: SharedServer,
    members: Vec<SharedServerMember>,
    categories: Vec<SharedCategory>,
    channels: Vec<SharedChannel>,
    roles: Vec<SnapshotServerRoleRow>,
    member_roles: Vec<SnapshotMemberRoleRow>,
    channel_role_overrides: Vec<SnapshotOverrideRow>,
    category_role_overrides: Vec<SnapshotOverrideRow>,
    member_channel_overrides: Vec<SnapshotOverrideRow>,
    member_category_overrides: Vec<SnapshotOverrideRow>,
    #[allow(dead_code)]
    synced_at: String,
}

fn presence_key(server_id: &str, user_id: &str) -> String {
    format!("{server_id}::{user_id}")
}

async fn next_presence_seq(state: &ServerState, server_id: &str) -> u64 {
    let mut seq_map = state.presence_seq.write().await;
    let entry = seq_map.entry(server_id.to_string()).or_insert(0);
    *entry += 1;
    *entry
}

async fn mark_presence_join(state: &ServerState, server_id: &str, user_id: &str) -> (bool, u64) {
    let key = presence_key(server_id, user_id);
    let transitioned_online = {
        let mut connections = state.presence_connections.write().await;
        let count = connections.entry(key).or_insert(0);
        *count += 1;
        *count == 1
    };

    let seq = next_presence_seq(state, server_id).await;
    (transitioned_online, seq)
}

async fn mark_presence_leave(state: &ServerState, server_id: &str, user_id: &str) -> (bool, u64) {
    let key = presence_key(server_id, user_id);
    let transitioned_offline = {
        let mut connections = state.presence_connections.write().await;
        match connections.get_mut(&key) {
            Some(count) if *count > 1 => {
                *count -= 1;
                false
            }
            Some(_) => {
                connections.remove(&key);
                true
            }
            None => true,
        }
    };

    let seq = next_presence_seq(state, server_id).await;
    (transitioned_offline, seq)
}

async fn send_presence_snapshot(
    client_id: &str,
    server_id: &str,
    state: &ServerState,
) -> Result<(), String> {
    let members = services::fetch_members_by_server(&state.pool, server_id)
        .await
        .map_err(|e| e.to_string())?;

    let current_seq = {
        let seq_map = state.presence_seq.read().await;
        *seq_map.get(server_id).unwrap_or(&0)
    };

    let snapshot_msg = WebSocketMessage {
        action: "presence_snapshot".to_string(),
        data: serde_json::json!({
            "server_id": server_id,
            "seq": current_seq,
            "members": members
        }),
    };

    send_to_client(client_id, &snapshot_msg, &state.clients).await
}

async fn broadcast_presence_delta(
    server_id: &str,
    user_id: &str,
    username: &str,
    is_online: bool,
    state: &ServerState,
    seq: u64,
) {
    let delta_msg = WebSocketMessage {
        action: "presence_delta".to_string(),
        data: serde_json::json!({
            "server_id": server_id,
            "seq": seq,
            "changes": [{
                "user_id": user_id,
                "username": username,
                "is_online": is_online,
                "last_seen": chrono::Utc::now().to_rfc3339(),
            }]
        }),
    };
    broadcast_to_server(server_id, &delta_msg, &state.clients).await;
}

async fn normalize_channel_positions_tx(
    tx: &mut sqlx::Transaction<'_, Sqlite>,
    server_id: &str,
    category_id: Option<&str>,
) -> Result<(), sqlx::Error> {
    let rows: Vec<(String,)> = if let Some(category_id) = category_id {
        sqlx::query_as(
            "SELECT id FROM channels WHERE server_id = ? AND category_id = ? ORDER BY position ASC, created_at ASC, id ASC",
        )
        .bind(server_id)
        .bind(category_id)
        .fetch_all(&mut **tx)
        .await?
    } else {
        sqlx::query_as(
            "SELECT id FROM channels WHERE server_id = ? AND category_id IS NULL ORDER BY position ASC, created_at ASC, id ASC",
        )
        .bind(server_id)
        .fetch_all(&mut **tx)
        .await?
    };

    for (index, (channel_id,)) in rows.into_iter().enumerate() {
        sqlx::query("UPDATE channels SET position = ? WHERE id = ?")
            .bind(index as i32)
            .bind(channel_id)
            .execute(&mut **tx)
            .await?;
    }

    Ok(())
}

async fn normalize_category_positions_tx(
    tx: &mut sqlx::Transaction<'_, Sqlite>,
    server_id: &str,
) -> Result<(), sqlx::Error> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT id FROM categories WHERE server_id = ? ORDER BY position ASC, created_at ASC, id ASC",
    )
    .bind(server_id)
    .fetch_all(&mut **tx)
    .await?;

    for (index, (category_id,)) in rows.into_iter().enumerate() {
        sqlx::query("UPDATE categories SET position = ? WHERE id = ?")
            .bind(index as i32)
            .bind(category_id)
            .execute(&mut **tx)
            .await?;
    }

    Ok(())
}

fn extract_query_param<'a>(query: &'a str, key: &str) -> Option<&'a str> {
    query.split('&').find_map(|segment| {
        let (param_key, param_value) = segment.split_once('=')?;
        if param_key == key {
            Some(param_value)
        } else {
            None
        }
    })
}

fn ws_auth_error(message: &str) -> ErrorResponse {
    let mut response = ErrorResponse::new(Some(message.to_string()));
    *response.status_mut() = StatusCode::UNAUTHORIZED;
    response
}

fn validate_ws_token(token: &str, jwt_config: &JwtValidationConfig) -> Result<JwtClaims, String> {
    match jwt_config {
        JwtValidationConfig::Hs256 {
            secret,
            allow_insecure_defaults,
            enforce_secret,
        } => {
            let resolved_secret = if !secret.trim().is_empty() {
                secret.clone()
            } else if *allow_insecure_defaults {
                eprintln!("⚠️ JWT_SECRET absent: fallback de dev utilisé pour WebSocket");
                "secret".to_string()
            } else if *enforce_secret {
                return Err("Missing JWT_SECRET (legacy HS256 mode)".to_string());
            } else {
                "secret".to_string()
            };

            let key = DecodingKey::from_secret(resolved_secret.as_ref());
            let validation = Validation::new(Algorithm::HS256);
            decode::<JwtClaims>(token, &key, &validation)
                .map(|data| data.claims)
                .map_err(|err| format!("Invalid WebSocket token (HS256): {err}"))
        }
        JwtValidationConfig::Rs256 {
            modulus_b64url,
            exponent_b64url,
        } => {
            let key = DecodingKey::from_rsa_components(modulus_b64url, exponent_b64url)
                .map_err(|err| format!("Invalid RSA public key components for WS auth: {err}"))?;
            let validation = Validation::new(Algorithm::RS256);
            decode::<JwtClaims>(token, &key, &validation)
                .map(|data| data.claims)
                .map_err(|err| format!("Invalid WebSocket token (RS256): {err}"))
        }
    }
}

fn extract_request_id_from_data(data: &serde_json::Value) -> Option<String> {
    data.get("request_id")
        .and_then(|v| v.as_str())
        .map(|v| v.to_string())
}

fn attach_request_id(
    mut payload: serde_json::Value,
    request_id: Option<&str>,
) -> serde_json::Value {
    if let (Some(id), Some(object)) = (request_id, payload.as_object_mut()) {
        object.insert(
            "request_id".to_string(),
            serde_json::Value::String(id.to_string()),
        );
    }

    payload
}

fn parse_string_array(data: &serde_json::Value, key: &str) -> Option<Vec<String>> {
    let values = data.get(key)?.as_array()?;
    let mut result = Vec::with_capacity(values.len());
    for value in values {
        result.push(value.as_str()?.to_string());
    }
    Some(result)
}

async fn ensure_client_identity_matches(
    msg: &WebSocketMessage,
    client_id: &str,
    state: &ServerState,
) -> Result<bool, String> {
    let (authenticated_user_id, known_username) = {
        let clients = state.clients.read().await;
        let client = clients
            .get(client_id)
            .ok_or_else(|| "Client not found".to_string())?;
        (
            client.authenticated_user_id.clone(),
            client.username.clone(),
        )
    };

    if let Some(user_id) = msg.data.get("user_id").and_then(|v| v.as_str()) {
        if user_id != authenticated_user_id {
            let error_msg = WebSocketMessage {
                action: "error".to_string(),
                data: serde_json::json!({
                    "message": "Identity mismatch: invalid user_id for this connection"
                }),
            };
            send_to_client(client_id, &error_msg, &state.clients).await?;
            return Ok(false);
        }
    }

    if let (Some(payload_username), Some(current_username)) = (
        msg.data.get("username").and_then(|v| v.as_str()),
        known_username.as_deref(),
    ) {
        if payload_username != current_username {
            let error_msg = WebSocketMessage {
                action: "error".to_string(),
                data: serde_json::json!({
                    "message": "Identity mismatch: invalid username for this connection"
                }),
            };
            send_to_client(client_id, &error_msg, &state.clients).await?;
            return Ok(false);
        }
    }

    Ok(true)
}

async fn get_authenticated_user_id(
    client_id: &str,
    clients: &Arc<RwLock<HashMap<String, Client>>>,
) -> Result<String, String> {
    let clients = clients.read().await;
    let client = clients
        .get(client_id)
        .ok_or_else(|| "Client not found".to_string())?;
    Ok(client.authenticated_user_id.clone())
}

// Fonctions utilitaires pour l'envoi de messages
async fn send_to_client(
    client_id: &str,
    message: &WebSocketMessage,
    clients: &Arc<RwLock<HashMap<String, Client>>>,
) -> Result<(), String> {
    let clients = clients.read().await;
    if let Some(client) = clients.get(client_id) {
        let mut sender = client.sender.lock().await;
        let msg_text = serde_json::to_string(message).map_err(|e| e.to_string())?;
        sender
            .send(WsMessage::Text(msg_text))
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

async fn broadcast_to_channel(
    channel_id: &str,
    message: &WebSocketMessage,
    clients: &Arc<RwLock<HashMap<String, Client>>>,
) {
    let clients = clients.read().await;
    for client in clients.values() {
        if client.current_channel.as_deref() == Some(channel_id) {
            let mut sender = client.sender.lock().await;
            if let Ok(msg_text) = serde_json::to_string(message) {
                let _ = sender.send(WsMessage::Text(msg_text)).await;
            }
        }
    }
}

async fn broadcast_to_server(
    server_id: &str,
    message: &WebSocketMessage,
    clients: &Arc<RwLock<HashMap<String, Client>>>,
) {
    let clients = clients.read().await;
    for client in clients.values() {
        if client.current_server.as_deref() == Some(server_id) {
            let mut sender = client.sender.lock().await;
            if let Ok(msg_text) = serde_json::to_string(message) {
                let _ = sender.send(WsMessage::Text(msg_text)).await;
            }
        }
    }
}

// Gestion des messages WebSocket
async fn handle_message(
    msg: WebSocketMessage,
    client_id: &str,
    state: &ServerState,
) -> Result<(), String> {
    if !ensure_client_identity_matches(&msg, client_id, state).await? {
        return Ok(());
    }

    match events::parse_client_event(&msg.action) {
        ClientEvent::JoinServer => {
            if let Some(server_id) = msg.data.get("server_id").and_then(|v| v.as_str()) {
                let user_id = get_authenticated_user_id(client_id, &state.clients).await?;

                // Vérifier que le serveur existe
                let server_check = sqlx::query("SELECT 1 FROM servers WHERE id = ?")
                    .bind(server_id)
                    .fetch_optional(&state.pool)
                    .await
                    .map_err(|e| e.to_string())?;

                if server_check.is_none() {
                    let error_msg = WebSocketMessage {
                        action: "error".to_string(),
                        data: serde_json::json!({ "message": "Server not found" }),
                    };
                    return send_to_client(client_id, &error_msg, &state.clients).await;
                }

                // Vérifier si l'utilisateur est déjà membre
                let member_check = sqlx::query(
                    "SELECT role, username FROM server_members WHERE server_id = ? AND user_id = ?",
                )
                .bind(server_id)
                .bind(&user_id)
                .fetch_optional(&state.pool)
                .await
                .map_err(|e| e.to_string())?;

                let Some(member_row) = member_check else {
                    let error_msg = WebSocketMessage {
                        action: "error".to_string(),
                        data: serde_json::json!({ "message": "User is not a member of this server" }),
                    };
                    return send_to_client(client_id, &error_msg, &state.clients).await;
                };

                let username: String = member_row.get("username");

                // Mettre à jour l'état du client
                {
                    let mut clients = state.clients.write().await;
                    if let Some(client) = clients.get_mut(client_id) {
                        client.current_server = Some(server_id.to_string());
                        client.user_id = Some(user_id.clone());
                        client.username = Some(username.to_string());
                    }
                }

                let (became_online, presence_seq) = mark_presence_join(state, server_id, &user_id).await;

                if became_online {
                    sqlx::query("UPDATE server_members SET is_online = 1, last_seen = datetime('now') WHERE server_id = ? AND user_id = ?")
                        .bind(server_id)
                        .bind(&user_id)
                        .execute(&state.pool)
                        .await
                        .map_err(|e| e.to_string())?;
                } else {
                    sqlx::query("UPDATE server_members SET last_seen = datetime('now') WHERE server_id = ? AND user_id = ?")
                        .bind(server_id)
                        .bind(&user_id)
                        .execute(&state.pool)
                        .await
                        .map_err(|e| e.to_string())?;
                }

                let join_ack = WebSocketMessage {
                    action: "join_server_ack".to_string(),
                    data: serde_json::json!({
                        "server_id": server_id,
                        "structure_version": chrono::Utc::now().timestamp_millis(),
                        "presence_seq": presence_seq,
                        "voice_seq": 0u64
                    }),
                };
                send_to_client(client_id, &join_ack, &state.clients).await?;

                send_presence_snapshot(client_id, server_id, state).await?;

                // Envoyer la structure du serveur
                match services::fetch_server_structure(&state.pool, server_id).await {
                    Ok(structure) => {
                        let structure_msg = WebSocketMessage {
                            action: "server_structure".to_string(),
                            data: serde_json::to_value(&structure).map_err(|e| e.to_string())?,
                        };
                        send_to_client(client_id, &structure_msg, &state.clients).await?;
                    }
                    Err(e) => {
                        println!("❌ Erreur chargement structure: {}", e);
                    }
                }

                // Notifier les autres membres de la connexion
                if became_online {
                    let join_msg = WebSocketMessage {
                        action: "member_joined".to_string(),
                        data: serde_json::json!({
                            "user_id": user_id,
                            "username": username.clone(),
                            "server_id": server_id
                        }),
                    };
                    broadcast_to_server(server_id, &join_msg, &state.clients).await;
                }

                if became_online {
                    broadcast_presence_delta(
                        server_id,
                        &user_id,
                        &username,
                        true,
                        state,
                        presence_seq,
                    )
                    .await;
                }

                println!("👤 {} a rejoint le serveur {}", username, server_id);
            }
        }
        ClientEvent::LeaveServer => {
            let (current_server, current_user_id, current_username) = {
                let clients = state.clients.read().await;
                let Some(client) = clients.get(client_id) else {
                    return Ok(());
                };
                (
                    client.current_server.clone(),
                    client.user_id
                        .clone()
                        .or_else(|| Some(client.authenticated_user_id.clone())),
                    client.username.clone().unwrap_or_else(|| "Utilisateur".to_string()),
                )
            };

            if let (Some(server_id), Some(user_id)) = (current_server, current_user_id) {
                let (became_offline, presence_seq) =
                    mark_presence_leave(state, &server_id, &user_id).await;

                if became_offline {
                    sqlx::query(
                        "UPDATE server_members SET is_online = 0, last_seen = datetime('now') WHERE server_id = ? AND user_id = ?",
                    )
                    .bind(&server_id)
                    .bind(&user_id)
                    .execute(&state.pool)
                    .await
                    .map_err(|e| e.to_string())?;
                }

                // Mettre à jour l'état du client
                {
                    let mut clients = state.clients.write().await;
                    if let Some(client) = clients.get_mut(client_id) {
                        client.current_server = None;
                        client.current_channel = None;
                    }
                }

                if became_offline {
                    let leave_msg = WebSocketMessage {
                        action: "member_left".to_string(),
                        data: serde_json::json!({
                            "user_id": user_id,
                            "username": current_username,
                            "server_id": server_id
                        }),
                    };
                    broadcast_to_server(&server_id, &leave_msg, &state.clients).await;
                }

                if became_offline {
                    broadcast_presence_delta(
                        &server_id,
                        &user_id,
                        &current_username,
                        false,
                        state,
                        presence_seq,
                    )
                    .await;
                }

                println!("👋 {} a quitté le serveur {}", current_username, server_id);
            }
        }
        ClientEvent::JoinChannel | ClientEvent::JoinTextChannel => {
            if let (Some(channel_id), Some(user_id), Some(_username)) = (
                msg.data.get("channel_id").and_then(|v| v.as_str()),
                msg.data.get("user_id").and_then(|v| v.as_str()),
                msg.data.get("username").and_then(|v| v.as_str()),
            ) {
                // Vérifier les permissions
                if !check_channel_permission(&state.pool, user_id, channel_id, "view")
                    .await
                    .map_err(|e| e.to_string())?
                {
                    let error_msg = WebSocketMessage {
                        action: "error".to_string(),
                        data: serde_json::json!({ "message": "You don't have permission to view this channel" }),
                    };
                    return send_to_client(client_id, &error_msg, &state.clients).await;
                }

                println!("Client {} rejoint le canal texte {}", client_id, channel_id);

                // Mettre à jour l'état du client
                {
                    let mut clients = state.clients.write().await;
                    if let Some(client) = clients.get_mut(client_id) {
                        client.current_channel = Some(channel_id.to_string());
                    }
                }

                // Récupérer l'historique complet des messages avec pagination
                let limit = msg
                    .data
                    .get("limit")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(100)
                    .min(200) as i64;
                let before_message_id = msg.data.get("before_message_id").and_then(|v| v.as_str());

                let history = services::fetch_channel_history(
                    &state.pool,
                    channel_id,
                    limit,
                    before_message_id,
                )
                .await
                .map_err(|e| e.to_string())?;

                let services::MessageHistory {
                    messages,
                    total_count,
                    has_more,
                    next_cursor,
                } = history;

                let history_msg = WebSocketMessage {
                    action: "message_history".to_string(),
                    data: serde_json::json!({
                        "channel_id": channel_id,
                        "messages": messages,
                        "total_count": total_count,
                        "has_more": has_more,
                        "next_cursor": next_cursor,
                    }),
                };
                send_to_client(client_id, &history_msg, &state.clients).await?;
            }
        }
        ClientEvent::SendMessage => {
            if let (Some(channel_id), Some(user_id), Some(username), Some(content)) = (
                msg.data.get("channel_id").and_then(|v| v.as_str()),
                msg.data.get("user_id").and_then(|v| v.as_str()),
                msg.data.get("username").and_then(|v| v.as_str()),
                msg.data.get("content").and_then(|v| v.as_str()),
            ) {
                // Vérifier les permissions
                if !check_channel_permission(&state.pool, user_id, channel_id, "send_messages")
                    .await
                    .map_err(|e| e.to_string())?
                {
                    let error_msg = WebSocketMessage {
                        action: "error".to_string(),
                        data: serde_json::json!({ "message": "You don't have permission to send messages in this channel" }),
                    };
                    return send_to_client(client_id, &error_msg, &state.clients).await;
                }

                let message_id = Uuid::new_v4().to_string();
                let now = chrono::Utc::now().to_rfc3339();

                // Extraire les données optionnelles
                let reply_to_message_id =
                    msg.data.get("reply_to_message_id").and_then(|v| v.as_str());
                let attachments = msg
                    .data
                    .get("attachments")
                    .and_then(|v| serde_json::to_string(v).ok());
                let is_system = msg
                    .data
                    .get("is_system")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                // Insérer le message avec les nouvelles colonnes
                sqlx::query(
                    "INSERT INTO messages (id, channel_id, user_id, username, content, created_at, reply_to_message_id, attachments, is_system) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
                )
                    .bind(&message_id)
                    .bind(channel_id)
                    .bind(user_id)
                    .bind(username)
                    .bind(content)
                    .bind(&now)
                    .bind(reply_to_message_id)
                    .bind(&attachments)
                    .bind(is_system)
                    .execute(&state.pool)
                    .await
                    .map_err(|e| e.to_string())?;

                // Traiter les mentions dans le message
                if let Some(mentions) = msg.data.get("mentions").and_then(|v| v.as_array()) {
                    for mention in mentions {
                        if let Some(mentioned_user_id) = mention.as_str() {
                            let mention_id = Uuid::new_v4().to_string();
                            let _ = sqlx::query(
                                "INSERT INTO message_mentions (id, message_id, mentioned_user_id, created_at) VALUES (?, ?, ?, ?)"
                            )
                                .bind(&mention_id)
                                .bind(&message_id)
                                .bind(mentioned_user_id)
                                .bind(&now)
                                .execute(&state.pool)
                                .await;
                        }
                    }
                }

                let message = ChatMessage {
                    id: message_id,
                    channel_id: channel_id.to_string(),
                    user_id: user_id.to_string(),
                    username: username.to_string(),
                    content: content.to_string(),
                    created_at: now,
                    edited_at: None,
                    reply_to_message_id: reply_to_message_id.map(|s| s.to_string()),
                    attachments,
                    reactions: None,
                    is_pinned: false,
                    is_system,
                };

                println!(
                    "📡 Diffusion du message à tous les utilisateurs du canal {}",
                    channel_id
                );

                // Diffuser le nouveau message à TOUS les utilisateurs du canal
                let broadcast_msg = WebSocketMessage {
                    action: "new_message".to_string(),
                    data: serde_json::to_value(&message).map_err(|e| e.to_string())?,
                };

                broadcast_to_channel(channel_id, &broadcast_msg, &state.clients).await;

                let ack_msg = WebSocketMessage {
                    action: "message_ack".to_string(),
                    data: serde_json::json!({ "message_id": message.id }),
                };
                send_to_client(client_id, &ack_msg, &state.clients).await?;
            } else {
                let error_msg = WebSocketMessage {
                    action: "error".to_string(),
                    data: serde_json::json!({ "message": "Invalid message format" }),
                };
                send_to_client(client_id, &error_msg, &state.clients).await?;
            }
        }
        ClientEvent::JoinVoiceChannel => {
            if let (Some(channel_id), Some(user_id), Some(username)) = (
                msg.data.get("channel_id").and_then(|v| v.as_str()),
                msg.data.get("user_id").and_then(|v| v.as_str()),
                msg.data.get("username").and_then(|v| v.as_str()),
            ) {
                // Vérifier les permissions
                if !check_channel_permission(&state.pool, user_id, channel_id, "connect_voice")
                    .await
                    .map_err(|e| e.to_string())?
                {
                    let error_msg = WebSocketMessage {
                        action: "error".to_string(),
                        data: serde_json::json!({ "message": "You don't have permission to connect to this voice channel" }),
                    };
                    return send_to_client(client_id, &error_msg, &state.clients).await;
                }

                let peer_id = Uuid::new_v4().to_string();
                match state
                    .sfu
                    .join_voice_channel(
                        channel_id.to_string(),
                        user_id.to_string(),
                        username.to_string(),
                        peer_id.clone(),
                    )
                    .await
                {
                    Ok(participant) => {
                        // Créer une session vocale dans la base de données
                        let session_id = Uuid::new_v4().to_string();
                        let now = chrono::Utc::now().to_rfc3339();

                        let _ = sqlx::query(
                            "INSERT INTO voice_sessions (id, channel_id, user_id, username, peer_id, joined_at) VALUES (?, ?, ?, ?, ?, ?)"
                        )
                        .bind(&session_id)
                        .bind(channel_id)
                        .bind(user_id)
                        .bind(username)
                        .bind(&peer_id)
                        .bind(&now)
                        .execute(&state.pool)
                        .await;

                        let joined_msg = WebSocketMessage {
                            action: "voice_joined".to_string(),
                            data: serde_json::json!({
                                "channel_id": channel_id,
                                "user_id": user_id,
                                "username": username,
                                "peer_id": peer_id,
                                "session_id": session_id,
                            }),
                        };
                        send_to_client(client_id, &joined_msg, &state.clients).await?;

                        // Notifier les autres participants
                        let notify_msg = WebSocketMessage {
                            action: "voice_participant_joined".to_string(),
                            data: serde_json::json!({
                                "channel_id": channel_id,
                                "user_id": user_id,
                                "username": username,
                                "peer_id": peer_id,
                                "is_muted": participant.is_muted,
                                "is_deafened": participant.is_deafened,
                            }),
                        };
                        broadcast_to_channel(channel_id, &notify_msg, &state.clients).await;
                    }
                    Err(e) => {
                        let error_msg = WebSocketMessage {
                            action: "error".to_string(),
                            data: serde_json::json!({
                                "message": format!("Failed to join voice channel: {}", e),
                                "channel_id": channel_id,
                            }),
                        };
                        send_to_client(client_id, &error_msg, &state.clients).await?;
                    }
                }
            }
        }
        ClientEvent::LeaveVoiceChannel => {
            if let (Some(channel_id), Some(user_id)) = (
                msg.data.get("channel_id").and_then(|v| v.as_str()),
                msg.data.get("user_id").and_then(|v| v.as_str()),
            ) {
                match state
                    .sfu
                    .leave_voice_channel(channel_id.to_string(), user_id.to_string())
                    .await
                {
                    Ok(_) => {
                        // Supprimer la session vocale de la base de données
                        let _ = sqlx::query(
                            "DELETE FROM voice_sessions WHERE channel_id = ? AND user_id = ?",
                        )
                        .bind(channel_id)
                        .bind(user_id)
                        .execute(&state.pool)
                        .await;

                        println!("Client {} a quitté le canal vocal {}", user_id, channel_id);

                        // Notifier les autres participants
                        let notify_msg = WebSocketMessage {
                            action: "voice_participant_left".to_string(),
                            data: serde_json::json!({
                                "channel_id": channel_id,
                                "user_id": user_id,
                            }),
                        };
                        broadcast_to_channel(channel_id, &notify_msg, &state.clients).await;
                    }
                    Err(e) => {
                        eprintln!("Erreur lors du départ du canal vocal: {}", e);
                    }
                }
            }
        }
        ClientEvent::WebrtcOffer => {
            if let (Some(channel_id), Some(user_id), Some(offer_sdp)) = (
                msg.data.get("channel_id").and_then(|v| v.as_str()),
                msg.data.get("user_id").and_then(|v| v.as_str()),
                msg.data.get("sdp").and_then(|v| v.as_str()),
            ) {
                match state
                    .sfu
                    .handle_webrtc_offer(
                        channel_id.to_string(),
                        user_id.to_string(),
                        offer_sdp.to_string(),
                    )
                    .await
                {
                    Ok(answer_sdp) => {
                        let answer_msg = WebSocketMessage {
                            action: "webrtc_answer".to_string(),
                            data: serde_json::json!({
                                "channel_id": channel_id,
                                "sdp": answer_sdp,
                            }),
                        };
                        send_to_client(client_id, &answer_msg, &state.clients).await?;
                    }
                    Err(e) => {
                        eprintln!("Erreur traitement offre WebRTC: {}", e);
                        let error_msg = WebSocketMessage {
                            action: "error".to_string(),
                            data: serde_json::json!({ "message": format!("WebRTC offer failed: {}", e) }),
                        };
                        send_to_client(client_id, &error_msg, &state.clients).await?;
                    }
                }
            }
        }
        ClientEvent::WebrtcAnswer => {
            if let (Some(channel_id), Some(user_id), Some(answer_sdp)) = (
                msg.data.get("channel_id").and_then(|v| v.as_str()),
                msg.data.get("user_id").and_then(|v| v.as_str()),
                msg.data.get("sdp").and_then(|v| v.as_str()),
            ) {
                match state
                    .sfu
                    .handle_webrtc_answer(
                        channel_id.to_string(),
                        user_id.to_string(),
                        answer_sdp.to_string(),
                    )
                    .await
                {
                    Ok(_) => { /* Answer processed */ }
                    Err(e) => {
                        eprintln!("Erreur traitement réponse WebRTC: {}", e);
                        let error_msg = WebSocketMessage {
                            action: "error".to_string(),
                            data: serde_json::json!({ "message": format!("WebRTC answer failed: {}", e) }),
                        };
                        send_to_client(client_id, &error_msg, &state.clients).await?;
                    }
                }
            }
        }
        ClientEvent::IceCandidate => {
            if let (Some(channel_id), Some(user_id), Some(candidate_sdp)) = (
                msg.data.get("channel_id").and_then(|v| v.as_str()),
                msg.data.get("user_id").and_then(|v| v.as_str()),
                msg.data.get("candidate").and_then(|v| v.as_str()),
            ) {
                match state
                    .sfu
                    .handle_ice_candidate(
                        channel_id.to_string(),
                        user_id.to_string(),
                        candidate_sdp.to_string(),
                    )
                    .await
                {
                    Ok(_) => { /* Candidate processed */ }
                    Err(e) => {
                        eprintln!("Erreur traitement ICE candidate: {}", e);
                        let error_msg = WebSocketMessage {
                            action: "error".to_string(),
                            data: serde_json::json!({ "message": format!("ICE candidate failed: {}", e) }),
                        };
                        send_to_client(client_id, &error_msg, &state.clients).await?;
                    }
                }
            }
        }
        ClientEvent::ToggleAudio => {
            if let (Some(channel_id), Some(user_id), Some(enabled)) = (
                msg.data.get("channel_id").and_then(|v| v.as_str()),
                msg.data.get("user_id").and_then(|v| v.as_str()),
                msg.data.get("enabled").and_then(|v| v.as_bool()),
            ) {
                match state
                    .sfu
                    .update_participant_status(
                        channel_id.to_string(),
                        user_id.to_string(),
                        Some(!enabled), // muted = !enabled
                        None,
                        None,
                        None,
                        None,
                        None,
                    )
                    .await
                {
                    Ok(_) => {
                        // Notifier tous les participants du changement
                        let notify_msg = WebSocketMessage {
                            action: "voice_participant_updated".to_string(),
                            data: serde_json::json!({
                                "channel_id": channel_id,
                                "user_id": user_id,
                                "is_muted": !enabled,
                            }),
                        };
                        broadcast_to_channel(channel_id, &notify_msg, &state.clients).await;
                    }
                    Err(e) => {
                        let error_msg = WebSocketMessage {
                            action: "error".to_string(),
                            data: serde_json::json!({ "message": format!("Failed to toggle audio: {}", e) }),
                        };
                        send_to_client(client_id, &error_msg, &state.clients).await?;
                    }
                }
            }
        }
        ClientEvent::ToggleVideo => {
            if let (Some(channel_id), Some(user_id), Some(enabled)) = (
                msg.data.get("channel_id").and_then(|v| v.as_str()),
                msg.data.get("user_id").and_then(|v| v.as_str()),
                msg.data.get("enabled").and_then(|v| v.as_bool()),
            ) {
                match state
                    .sfu
                    .update_participant_status(
                        channel_id.to_string(),
                        user_id.to_string(),
                        None,
                        None,
                        Some(enabled),
                        None,
                        None,
                        None,
                    )
                    .await
                {
                    Ok(_) => {
                        // Notifier tous les participants du changement
                        let notify_msg = WebSocketMessage {
                            action: "voice_participant_updated".to_string(),
                            data: serde_json::json!({
                                "channel_id": channel_id,
                                "user_id": user_id,
                                "is_video_enabled": enabled,
                            }),
                        };
                        broadcast_to_channel(channel_id, &notify_msg, &state.clients).await;
                    }
                    Err(e) => {
                        let error_msg = WebSocketMessage {
                            action: "error".to_string(),
                            data: serde_json::json!({ "message": format!("Failed to toggle video: {}", e) }),
                        };
                        send_to_client(client_id, &error_msg, &state.clients).await?;
                    }
                }
            }
        }
        ClientEvent::ChangeAudioQuality => {
            if let (Some(channel_id), Some(user_id), Some(quality)) = (
                msg.data.get("channel_id").and_then(|v| v.as_str()),
                msg.data.get("user_id").and_then(|v| v.as_str()),
                msg.data.get("quality").and_then(|v| v.as_u64()),
            ) {
                match state
                    .sfu
                    .update_participant_status(
                        channel_id.to_string(),
                        user_id.to_string(),
                        None,
                        None,
                        None,
                        Some(quality as u32),
                        None,
                        None,
                    )
                    .await
                {
                    Ok(_) => {
                        println!(
                            "🎵 Qualité audio changée à {} kbps pour {} dans le canal {}",
                            quality, user_id, channel_id
                        );
                    }
                    Err(e) => {
                        let error_msg = WebSocketMessage {
                            action: "error".to_string(),
                            data: serde_json::json!({ "message": format!("Failed to change audio quality: {}", e) }),
                        };
                        send_to_client(client_id, &error_msg, &state.clients).await?;
                    }
                }
            }
        }
        ClientEvent::ChangeVideoQuality => {
            if let (Some(channel_id), Some(user_id), Some(quality)) = (
                msg.data.get("channel_id").and_then(|v| v.as_str()),
                msg.data.get("user_id").and_then(|v| v.as_str()),
                msg.data.get("quality").and_then(|v| v.as_u64()),
            ) {
                match state
                    .sfu
                    .update_participant_status(
                        channel_id.to_string(),
                        user_id.to_string(),
                        None,
                        None,
                        None,
                        None,
                        Some(quality as u32),
                        None,
                    )
                    .await
                {
                    Ok(_) => {
                        println!(
                            "🎥 Qualité vidéo changée à {}p pour {} dans le canal {}",
                            quality, user_id, channel_id
                        );
                    }
                    Err(e) => {
                        let error_msg = WebSocketMessage {
                            action: "error".to_string(),
                            data: serde_json::json!({ "message": format!("Failed to change video quality: {}", e) }),
                        };
                        send_to_client(client_id, &error_msg, &state.clients).await?;
                    }
                }
            }
        }
        ClientEvent::AddMediaTrack => {
            if let (Some(channel_id), Some(user_id), Some(track_id), Some(kind)) = (
                msg.data.get("channel_id").and_then(|v| v.as_str()),
                msg.data.get("user_id").and_then(|v| v.as_str()),
                msg.data.get("track_id").and_then(|v| v.as_str()),
                msg.data.get("kind").and_then(|v| v.as_str()),
            ) {
                let ssrc = msg
                    .data
                    .get("ssrc")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32);
                let payload_type = msg
                    .data
                    .get("payload_type")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u8);
                let codec = msg
                    .data
                    .get("codec")
                    .and_then(|v| v.as_str())
                    .map(|v| v.to_string());
                let bitrate = msg
                    .data
                    .get("bitrate")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32);

                match state
                    .sfu
                    .handle_media_track_added(
                        channel_id.to_string(),
                        user_id.to_string(),
                        track_id.to_string(),
                        kind.to_string(),
                        ssrc,
                        payload_type,
                        codec,
                        bitrate,
                    )
                    .await
                {
                    Ok(_) => {
                        println!(
                            "🎵 Track {} ajouté pour {} dans le canal {}",
                            kind, user_id, channel_id
                        );
                    }
                    Err(e) => {
                        let error_msg = WebSocketMessage {
                            action: "error".to_string(),
                            data: serde_json::json!({ "message": format!("Failed to add media track: {}", e) }),
                        };
                        send_to_client(client_id, &error_msg, &state.clients).await?;
                    }
                }
            }
        }
        ClientEvent::RemoveMediaTrack => {
            if let (Some(channel_id), Some(user_id), Some(track_id)) = (
                msg.data.get("channel_id").and_then(|v| v.as_str()),
                msg.data.get("user_id").and_then(|v| v.as_str()),
                msg.data.get("track_id").and_then(|v| v.as_str()),
            ) {
                match state
                    .sfu
                    .handle_media_track_removed(
                        channel_id.to_string(),
                        user_id.to_string(),
                        track_id.to_string(),
                    )
                    .await
                {
                    Ok(_) => {
                        println!(
                            "🎵 Track {} retiré pour {} dans le canal {}",
                            track_id, user_id, channel_id
                        );
                    }
                    Err(e) => {
                        let error_msg = WebSocketMessage {
                            action: "error".to_string(),
                            data: serde_json::json!({ "message": format!("Failed to remove media track: {}", e) }),
                        };
                        send_to_client(client_id, &error_msg, &state.clients).await?;
                    }
                }
            }
        }
        ClientEvent::GetVoiceStats => {
            if let Some(channel_id) = msg.data.get("channel_id").and_then(|v| v.as_str()) {
                match state.sfu.get_connection_stats(channel_id.to_string()).await {
                    Some(stats) => {
                        let stats_msg = WebSocketMessage {
                            action: "voice_stats".to_string(),
                            data: stats,
                        };
                        send_to_client(client_id, &stats_msg, &state.clients).await?;
                    }
                    None => {
                        let error_msg = WebSocketMessage {
                            action: "error".to_string(),
                            data: serde_json::json!({ "message": "No voice channel found" }),
                        };
                        send_to_client(client_id, &error_msg, &state.clients).await?;
                    }
                }
            }
        }
        ClientEvent::LoadMessageHistory => {
            if let (Some(channel_id), Some(user_id)) = (
                msg.data.get("channel_id").and_then(|v| v.as_str()),
                msg.data.get("user_id").and_then(|v| v.as_str()),
            ) {
                // Vérifier les permissions
                if !check_channel_permission(&state.pool, user_id, channel_id, "view")
                    .await
                    .map_err(|e| e.to_string())?
                {
                    let error_msg = WebSocketMessage {
                        action: "error".to_string(),
                        data: serde_json::json!({ "message": "You don't have permission to view this channel" }),
                    };
                    return send_to_client(client_id, &error_msg, &state.clients).await;
                }

                let limit = msg.data.get("limit").and_then(|v| v.as_u64()).unwrap_or(50) as i64;
                let before_message_id = msg.data.get("before_message_id").and_then(|v| v.as_str());

                let rows = if let Some(before_id) = before_message_id {
                    sqlx::query(
                        "SELECT id, channel_id, user_id, username, content, created_at, edited_at, reply_to_message_id, attachments, reactions, is_pinned, is_system FROM messages WHERE channel_id = ? AND created_at < (SELECT created_at FROM messages WHERE id = ?) ORDER BY created_at DESC LIMIT ?"
                    )
                        .bind(channel_id)
                        .bind(before_id)
                        .bind(limit)
                        .fetch_all(&state.pool)
                        .await
                } else {
                    sqlx::query(
                        "SELECT id, channel_id, user_id, username, content, created_at, edited_at, reply_to_message_id, attachments, reactions, is_pinned, is_system FROM messages WHERE channel_id = ? ORDER BY created_at DESC LIMIT ?"
                    )
                        .bind(channel_id)
                        .bind(limit)
                        .fetch_all(&state.pool)
                        .await
                }.map_err(|e| e.to_string())?;

                let mut messages = Vec::new();
                for row in rows {
                    messages.push(ChatMessage {
                        id: row.get("id"),
                        channel_id: row.get("channel_id"),
                        user_id: row.get("user_id"),
                        username: row.get("username"),
                        content: row.get("content"),
                        created_at: row.get("created_at"),
                        edited_at: row.get("edited_at"),
                        reply_to_message_id: row.get("reply_to_message_id"),
                        attachments: row.get("attachments"),
                        reactions: row.get("reactions"),
                        is_pinned: row.get("is_pinned"),
                        is_system: row.get("is_system"),
                    });
                }

                let has_more = messages.len() as i64 == limit;
                let next_cursor = if has_more && !messages.is_empty() {
                    Some(messages.last().unwrap().id.clone())
                } else {
                    None
                };

                let history_msg = WebSocketMessage {
                    action: "message_history_page".to_string(),
                    data: serde_json::json!({
                        "channel_id": channel_id,
                        "messages": messages,
                        "has_more": has_more,
                        "next_cursor": next_cursor,
                    }),
                };
                send_to_client(client_id, &history_msg, &state.clients).await?;
            }
        }
        ClientEvent::EditMessage => {
            if let (Some(message_id), Some(user_id), Some(new_content)) = (
                msg.data.get("message_id").and_then(|v| v.as_str()),
                msg.data.get("user_id").and_then(|v| v.as_str()),
                msg.data.get("content").and_then(|v| v.as_str()),
            ) {
                // Vérifier que l'utilisateur est l'auteur du message ou a les permissions
                let message_row =
                    sqlx::query("SELECT channel_id, user_id FROM messages WHERE id = ?")
                        .bind(message_id)
                        .fetch_optional(&state.pool)
                        .await
                        .map_err(|e| e.to_string())?;

                if let Some(row) = message_row {
                    let channel_id: String = row.get("channel_id");
                    let author_id: String = row.get("user_id");

                    // Vérifier les permissions (auteur ou permission manage_messages)
                    if author_id != user_id
                        && !check_channel_permission(
                            &state.pool,
                            user_id,
                            &channel_id,
                            "manage_messages",
                        )
                        .await
                        .map_err(|e| e.to_string())?
                    {
                        let error_msg = WebSocketMessage {
                            action: "error".to_string(),
                            data: serde_json::json!({ "message": "You don't have permission to edit this message" }),
                        };
                        return send_to_client(client_id, &error_msg, &state.clients).await;
                    }

                    let now = chrono::Utc::now().to_rfc3339();

                    // Mettre à jour le message
                    sqlx::query("UPDATE messages SET content = ?, edited_at = ? WHERE id = ?")
                        .bind(new_content)
                        .bind(&now)
                        .bind(message_id)
                        .execute(&state.pool)
                        .await
                        .map_err(|e| e.to_string())?;

                    // Récupérer le message mis à jour
                    let updated_row = sqlx::query(
                        "SELECT id, channel_id, user_id, username, content, created_at, edited_at, reply_to_message_id, attachments, reactions, is_pinned, is_system FROM messages WHERE id = ?"
                    )
                        .bind(message_id)
                        .fetch_one(&state.pool)
                        .await
                        .map_err(|e| e.to_string())?;

                    let updated_message = ChatMessage {
                        id: updated_row.get("id"),
                        channel_id: updated_row.get("channel_id"),
                        user_id: updated_row.get("user_id"),
                        username: updated_row.get("username"),
                        content: updated_row.get("content"),
                        created_at: updated_row.get("created_at"),
                        edited_at: updated_row.get("edited_at"),
                        reply_to_message_id: updated_row.get("reply_to_message_id"),
                        attachments: updated_row.get("attachments"),
                        reactions: updated_row.get("reactions"),
                        is_pinned: updated_row.get("is_pinned"),
                        is_system: updated_row.get("is_system"),
                    };

                    // Diffuser la mise à jour à tous les utilisateurs du canal
                    let broadcast_msg = WebSocketMessage {
                        action: "message_edited".to_string(),
                        data: serde_json::to_value(&updated_message).map_err(|e| e.to_string())?,
                    };
                    broadcast_to_channel(&channel_id, &broadcast_msg, &state.clients).await;

                    let ack_msg = WebSocketMessage {
                        action: "message_edit_ack".to_string(),
                        data: serde_json::json!({ "message_id": message_id }),
                    };
                    send_to_client(client_id, &ack_msg, &state.clients).await?;
                } else {
                    let error_msg = WebSocketMessage {
                        action: "error".to_string(),
                        data: serde_json::json!({ "message": "Message not found" }),
                    };
                    send_to_client(client_id, &error_msg, &state.clients).await?;
                }
            }
        }
        ClientEvent::DeleteMessage => {
            if let (Some(message_id), Some(user_id)) = (
                msg.data.get("message_id").and_then(|v| v.as_str()),
                msg.data.get("user_id").and_then(|v| v.as_str()),
            ) {
                // Vérifier que l'utilisateur est l'auteur du message ou a les permissions
                let message_row =
                    sqlx::query("SELECT channel_id, user_id FROM messages WHERE id = ?")
                        .bind(message_id)
                        .fetch_optional(&state.pool)
                        .await
                        .map_err(|e| e.to_string())?;

                if let Some(row) = message_row {
                    let channel_id: String = row.get("channel_id");
                    let author_id: String = row.get("user_id");

                    // Vérifier les permissions (auteur ou permission manage_messages)
                    if author_id != user_id
                        && !check_channel_permission(
                            &state.pool,
                            user_id,
                            &channel_id,
                            "manage_messages",
                        )
                        .await
                        .map_err(|e| e.to_string())?
                    {
                        let error_msg = WebSocketMessage {
                            action: "error".to_string(),
                            data: serde_json::json!({ "message": "You don't have permission to delete this message" }),
                        };
                        return send_to_client(client_id, &error_msg, &state.clients).await;
                    }

                    // Supprimer le message (cascade supprimera les mentions et réactions)
                    sqlx::query("DELETE FROM messages WHERE id = ?")
                        .bind(message_id)
                        .execute(&state.pool)
                        .await
                        .map_err(|e| e.to_string())?;

                    // Diffuser la suppression à tous les utilisateurs du canal
                    let broadcast_msg = WebSocketMessage {
                        action: "message_deleted".to_string(),
                        data: serde_json::json!({
                            "message_id": message_id,
                            "channel_id": channel_id,
                        }),
                    };
                    broadcast_to_channel(&channel_id, &broadcast_msg, &state.clients).await;

                    let ack_msg = WebSocketMessage {
                        action: "message_delete_ack".to_string(),
                        data: serde_json::json!({ "message_id": message_id }),
                    };
                    send_to_client(client_id, &ack_msg, &state.clients).await?;
                } else {
                    let error_msg = WebSocketMessage {
                        action: "error".to_string(),
                        data: serde_json::json!({ "message": "Message not found" }),
                    };
                    send_to_client(client_id, &error_msg, &state.clients).await?;
                }
            }
        }
        ClientEvent::AddReaction => {
            if let (Some(message_id), Some(user_id), Some(emoji)) = (
                msg.data.get("message_id").and_then(|v| v.as_str()),
                msg.data.get("user_id").and_then(|v| v.as_str()),
                msg.data.get("emoji").and_then(|v| v.as_str()),
            ) {
                // Vérifier que le message existe et que l'utilisateur peut voir le canal
                let message_row = sqlx::query("SELECT channel_id FROM messages WHERE id = ?")
                    .bind(message_id)
                    .fetch_optional(&state.pool)
                    .await
                    .map_err(|e| e.to_string())?;

                if let Some(row) = message_row {
                    let channel_id: String = row.get("channel_id");

                    if !check_channel_permission(&state.pool, user_id, &channel_id, "view")
                        .await
                        .map_err(|e| e.to_string())?
                    {
                        let error_msg = WebSocketMessage {
                            action: "error".to_string(),
                            data: serde_json::json!({ "message": "You don't have permission to view this channel" }),
                        };
                        return send_to_client(client_id, &error_msg, &state.clients).await;
                    }

                    let reaction_id = Uuid::new_v4().to_string();
                    let now = chrono::Utc::now().to_rfc3339();

                    // Ajouter la réaction (ou ignorer si elle existe déjà)
                    let _ = sqlx::query(
                        "INSERT OR IGNORE INTO message_reactions (id, message_id, user_id, emoji, created_at) VALUES (?, ?, ?, ?, ?)"
                    )
                        .bind(&reaction_id)
                        .bind(message_id)
                        .bind(user_id)
                        .bind(emoji)
                        .bind(&now)
                        .execute(&state.pool)
                        .await;

                    // Diffuser la réaction à tous les utilisateurs du canal
                    let broadcast_msg = WebSocketMessage {
                        action: "reaction_added".to_string(),
                        data: serde_json::json!({
                            "message_id": message_id,
                            "user_id": user_id,
                            "emoji": emoji,
                        }),
                    };
                    broadcast_to_channel(&channel_id, &broadcast_msg, &state.clients).await;
                } else {
                    let error_msg = WebSocketMessage {
                        action: "error".to_string(),
                        data: serde_json::json!({ "message": "Message not found" }),
                    };
                    send_to_client(client_id, &error_msg, &state.clients).await?;
                }
            }
        }
        ClientEvent::RemoveReaction => {
            if let (Some(message_id), Some(user_id), Some(emoji)) = (
                msg.data.get("message_id").and_then(|v| v.as_str()),
                msg.data.get("user_id").and_then(|v| v.as_str()),
                msg.data.get("emoji").and_then(|v| v.as_str()),
            ) {
                // Vérifier que le message existe et que l'utilisateur peut voir le canal
                let message_row = sqlx::query("SELECT channel_id FROM messages WHERE id = ?")
                    .bind(message_id)
                    .fetch_optional(&state.pool)
                    .await
                    .map_err(|e| e.to_string())?;

                if let Some(row) = message_row {
                    let channel_id: String = row.get("channel_id");

                    if !check_channel_permission(&state.pool, user_id, &channel_id, "view")
                        .await
                        .map_err(|e| e.to_string())?
                    {
                        let error_msg = WebSocketMessage {
                            action: "error".to_string(),
                            data: serde_json::json!({ "message": "You don't have permission to view this channel" }),
                        };
                        return send_to_client(client_id, &error_msg, &state.clients).await;
                    }

                    // Supprimer la réaction
                    sqlx::query(
                        "DELETE FROM message_reactions WHERE message_id = ? AND user_id = ? AND emoji = ?"
                    )
                        .bind(message_id)
                        .bind(user_id)
                        .bind(emoji)
                        .execute(&state.pool)
                        .await
                        .map_err(|e| e.to_string())?;

                    // Diffuser la suppression de réaction à tous les utilisateurs du canal
                    let broadcast_msg = WebSocketMessage {
                        action: "reaction_removed".to_string(),
                        data: serde_json::json!({
                            "message_id": message_id,
                            "user_id": user_id,
                            "emoji": emoji,
                        }),
                    };
                    broadcast_to_channel(&channel_id, &broadcast_msg, &state.clients).await;
                } else {
                    let error_msg = WebSocketMessage {
                        action: "error".to_string(),
                        data: serde_json::json!({ "message": "Message not found" }),
                    };
                    send_to_client(client_id, &error_msg, &state.clients).await?;
                }
            }
        }
        ClientEvent::GetServerMembers => {
            if let Some(server_id) = msg.data.get("server_id").and_then(|v| v.as_str()) {
                let rows = sqlx::query_as::<_, ServerMember>(
                    "SELECT id, server_id, user_id, username, avatar, role, joined_at, last_seen, is_online FROM server_members WHERE server_id = ? ORDER BY is_online DESC, role ASC, username ASC"
                )
                    .bind(server_id)
                    .fetch_all(&state.pool)
                    .await
                    .map_err(|e| e.to_string())?;

                let members_msg = WebSocketMessage {
                    action: "server_members".to_string(),
                    data: serde_json::json!({
                        "server_id": server_id,
                        "members": rows.clone(),
                    }),
                };
                send_to_client(client_id, &members_msg, &state.clients).await?;

                let current_seq = {
                    let seq_map = state.presence_seq.read().await;
                    *seq_map.get(server_id).unwrap_or(&0)
                };
                let presence_snapshot = WebSocketMessage {
                    action: "presence_snapshot".to_string(),
                    data: serde_json::json!({
                        "server_id": server_id,
                        "seq": current_seq,
                        "members": rows,
                    }),
                };
                send_to_client(client_id, &presence_snapshot, &state.clients).await?;
            } else {
                let error_msg = WebSocketMessage {
                    action: "error".to_string(),
                    data: serde_json::json!({ "message": "Server ID missing" }),
                };
                send_to_client(client_id, &error_msg, &state.clients).await?;
            }
        }
        ClientEvent::GetServerStructure => {
            if let Some(server_id) = msg.data.get("server_id").and_then(|v| v.as_str()) {
                match services::fetch_server_structure(&state.pool, server_id).await {
                    Ok(structure) => {
                        let structure_msg = WebSocketMessage {
                            action: "server_structure".to_string(),
                            data: serde_json::to_value(&structure).map_err(|e| e.to_string())?,
                        };
                        send_to_client(client_id, &structure_msg, &state.clients).await?;
                    }
                    Err(e) => {
                        println!("❌ Erreur chargement structure: {}", e);
                        let error_msg = WebSocketMessage {
                            action: "error".to_string(),
                            data: serde_json::json!({
                                "message": "Unable to load server structure",
                                "server_id": server_id
                            }),
                        };
                        send_to_client(client_id, &error_msg, &state.clients).await?;
                    }
                }
            }
        }
        ClientEvent::GetUserRole => {
            if let (Some(user_id), Some(server_id)) = (
                msg.data.get("user_id").and_then(|v| v.as_str()),
                msg.data.get("server_id").and_then(|v| v.as_str()),
            ) {
                match get_user_role(&state.pool, user_id, server_id).await {
                    Ok(role) => {
                        let role_msg = WebSocketMessage {
                            action: "user_role".to_string(),
                            data: serde_json::json!({
                                "user_id": user_id,
                                "server_id": server_id,
                                "role": role,
                            }),
                        };
                        send_to_client(client_id, &role_msg, &state.clients).await?;
                    }
                    Err(e) => {
                        let error_msg = WebSocketMessage {
                            action: "error".to_string(),
                            data: serde_json::json!({
                                "message": format!("Failed to get user role: {}", e),
                            }),
                        };
                        send_to_client(client_id, &error_msg, &state.clients).await?;
                    }
                }
            }
        }
        ClientEvent::CheckServerPermissions => {
            if let (Some(user_id), Some(server_id)) = (
                msg.data.get("user_id").and_then(|v| v.as_str()),
                msg.data.get("server_id").and_then(|v| v.as_str()),
            ) {
                let can_manage = can_manage_server(&state.pool, user_id, server_id)
                    .await
                    .unwrap_or(false);
                let is_owner = is_server_owner(&state.pool, user_id, server_id)
                    .await
                    .unwrap_or(false);

                let permissions_msg = WebSocketMessage {
                    action: "server_permissions".to_string(),
                    data: serde_json::json!({
                        "user_id": user_id,
                        "server_id": server_id,
                        "can_manage_server": can_manage,
                        "is_server_owner": is_owner,
                    }),
                };
                send_to_client(client_id, &permissions_msg, &state.clients).await?;
            }
        }
        ClientEvent::GetVoiceRoomInfo => {
            if let Some(channel_id) = msg.data.get("channel_id").and_then(|v| v.as_str()) {
                match state.sfu.get_room_info(channel_id.to_string()).await {
                    Some(room_info) => {
                        let room_msg = WebSocketMessage {
                            action: "voice_room_info".to_string(),
                            data: room_info,
                        };
                        send_to_client(client_id, &room_msg, &state.clients).await?;
                        let snapshot_msg = WebSocketMessage {
                            action: "voice_room_snapshot".to_string(),
                            data: room_msg.data.clone(),
                        };
                        send_to_client(client_id, &snapshot_msg, &state.clients).await?;
                    }
                    None => {
                        let empty_msg = WebSocketMessage {
                            action: "voice_room_info".to_string(),
                            data: serde_json::json!({
                                "channel_id": channel_id,
                                "participants": [],
                                "participant_count": 0,
                            }),
                        };
                        send_to_client(client_id, &empty_msg, &state.clients).await?;
                        let snapshot_msg = WebSocketMessage {
                            action: "voice_room_snapshot".to_string(),
                            data: empty_msg.data.clone(),
                        };
                        send_to_client(client_id, &snapshot_msg, &state.clients).await?;
                    }
                }
            }
        }
        // Gestion des canaux
        ClientEvent::CreateChannel => {
            let request_id = extract_request_id_from_data(&msg.data);
            if let (Some(server_id), Some(name), Some(channel_type), Some(user_id)) = (
                msg.data.get("server_id").and_then(|v| v.as_str()),
                msg.data.get("name").and_then(|v| v.as_str()),
                msg.data.get("channel_type").and_then(|v| v.as_str()),
                msg.data.get("user_id").and_then(|v| v.as_str()),
            ) {
                // Vérifier les permissions
                if !can_manage_channels(&state.pool, user_id, &server_id)
                    .await
                    .map_err(|e| e.to_string())?
                {
                    let error_msg = WebSocketMessage {
                        action: "error".to_string(),
                        data: attach_request_id(
                            serde_json::json!({ "message": "You don't have permission to manage channels" }),
                            request_id.as_deref(),
                        ),
                    };
                    return send_to_client(client_id, &error_msg, &state.clients).await;
                }

                let channel_id = uuid::Uuid::new_v4().to_string();
                let category_id = msg.data.get("category_id").and_then(|v| v.as_str());
                let description = msg.data.get("description").and_then(|v| v.as_str());
                let mut tx = state.pool.begin().await.map_err(|e| e.to_string())?;

                let next_position: i64 = if let Some(category_id) = category_id {
                    sqlx::query_scalar(
                        "SELECT COALESCE(MAX(position), -1) + 1 FROM channels WHERE server_id = ? AND category_id = ?",
                    )
                    .bind(server_id)
                    .bind(category_id)
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(|e| e.to_string())?
                } else {
                    sqlx::query_scalar(
                        "SELECT COALESCE(MAX(position), -1) + 1 FROM channels WHERE server_id = ? AND category_id IS NULL",
                    )
                    .bind(server_id)
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(|e| e.to_string())?
                };

                // Créer le canal
                sqlx::query(
                    "INSERT INTO channels (id, server_id, name, type, category_id, position, description, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, datetime('now'))"
                )
                .bind(&channel_id)
                .bind(server_id)
                .bind(name)
                .bind(channel_type)
                .bind(category_id)
                .bind(next_position as i32)
                .bind(description)
                .execute(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;

                tx.commit().await.map_err(|e| e.to_string())?;

                // Broadcast de la création
                let create_msg = WebSocketMessage {
                    action: "channel_created".to_string(),
                    data: serde_json::json!({
                        "channel_id": channel_id.clone(),
                        "server_id": server_id,
                        "name": name,
                        "type": channel_type,
                        "category_id": category_id,
                        "description": description,
                        "position": next_position
                    }),
                };
                broadcast_to_server(server_id, &create_msg, &state.clients).await;

                let ack_msg = WebSocketMessage {
                    action: "channel_create_ack".to_string(),
                    data: attach_request_id(
                        serde_json::json!({
                            "channel_id": channel_id,
                            "server_id": server_id,
                            "name": name,
                            "type": channel_type,
                            "category_id": category_id,
                            "description": description,
                            "position": next_position
                        }),
                        request_id.as_deref(),
                    ),
                };
                send_to_client(client_id, &ack_msg, &state.clients).await?;

                println!("📝 Canal {} créé dans le serveur {}", name, server_id);
            }
        }
        ClientEvent::UpdateChannel => {
            let request_id = extract_request_id_from_data(&msg.data);
            if let (Some(channel_id), Some(user_id)) = (
                msg.data.get("channel_id").and_then(|v| v.as_str()),
                msg.data.get("user_id").and_then(|v| v.as_str()),
            ) {
                // Récupérer le serveur et la catégorie du canal
                let channel_row =
                    sqlx::query("SELECT server_id, category_id FROM channels WHERE id = ?")
                        .bind(channel_id)
                        .fetch_one(&state.pool)
                        .await
                        .map_err(|e| e.to_string())?;
                let server_id: String = channel_row.get("server_id");
                // Vérifier les permissions
                if !can_manage_channels(&state.pool, user_id, &server_id)
                    .await
                    .map_err(|e| e.to_string())?
                {
                    let error_msg = WebSocketMessage {
                        action: "error".to_string(),
                        data: attach_request_id(
                            serde_json::json!({ "message": "You don't have permission to manage channels" }),
                            request_id.as_deref(),
                        ),
                    };
                    return send_to_client(client_id, &error_msg, &state.clients).await;
                }

                // Mettre à jour le canal
                let mut update_query = "UPDATE channels SET ".to_string();
                let mut params = Vec::new();
                let mut set_clauses = Vec::new();

                if let Some(name) = msg.data.get("name").and_then(|v| v.as_str()) {
                    set_clauses.push("name = ?");
                    params.push(name.to_string());
                }
                if let Some(description) = msg.data.get("description").and_then(|v| v.as_str()) {
                    set_clauses.push("description = ?");
                    params.push(description.to_string());
                }

                if !set_clauses.is_empty() {
                    update_query.push_str(&set_clauses.join(", "));
                    update_query.push_str(" WHERE id = ?");
                    params.push(channel_id.to_string());

                    let mut query = sqlx::query(&update_query);
                    for param in params {
                        query = query.bind(param);
                    }
                    query
                        .execute(&state.pool)
                        .await
                        .map_err(|e| e.to_string())?;

                    // Broadcast de la mise à jour
                    let update_msg = WebSocketMessage {
                        action: "channel_updated".to_string(),
                        data: serde_json::json!({
                            "channel_id": channel_id,
                            "server_id": server_id,
                            "name": msg.data.get("name"),
                            "description": msg.data.get("description")
                        }),
                    };
                    broadcast_to_server(&server_id, &update_msg, &state.clients).await;

                    let ack_msg = WebSocketMessage {
                        action: "channel_update_ack".to_string(),
                        data: attach_request_id(
                            serde_json::json!({
                                "channel_id": channel_id,
                                "server_id": server_id,
                                "name": msg.data.get("name"),
                                "description": msg.data.get("description")
                            }),
                            request_id.as_deref(),
                        ),
                    };
                    send_to_client(client_id, &ack_msg, &state.clients).await?;

                    println!("✏️ Canal {} mis à jour", channel_id);
                }
            }
        }
        ClientEvent::DeleteChannel => {
            let request_id = extract_request_id_from_data(&msg.data);
            if let (Some(channel_id), Some(user_id)) = (
                msg.data.get("channel_id").and_then(|v| v.as_str()),
                msg.data.get("user_id").and_then(|v| v.as_str()),
            ) {
                // Récupérer le serveur et la catégorie du canal
                let channel_row =
                    sqlx::query("SELECT server_id, category_id FROM channels WHERE id = ?")
                        .bind(channel_id)
                        .fetch_one(&state.pool)
                        .await
                        .map_err(|e| e.to_string())?;
                let server_id: String = channel_row.get("server_id");
                let current_category_id: Option<String> = channel_row.get("category_id");

                // Vérifier les permissions
                if !can_manage_channels(&state.pool, user_id, &server_id)
                    .await
                    .map_err(|e| e.to_string())?
                {
                    let error_msg = WebSocketMessage {
                        action: "error".to_string(),
                        data: attach_request_id(
                            serde_json::json!({ "message": "You don't have permission to manage channels" }),
                            request_id.as_deref(),
                        ),
                    };
                    return send_to_client(client_id, &error_msg, &state.clients).await;
                }

                let mut tx = state.pool.begin().await.map_err(|e| e.to_string())?;

                // Supprimer le canal
                sqlx::query("DELETE FROM channels WHERE id = ?")
                    .bind(channel_id)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| e.to_string())?;

                normalize_channel_positions_tx(&mut tx, &server_id, current_category_id.as_deref())
                    .await
                    .map_err(|e| e.to_string())?;
                tx.commit().await.map_err(|e| e.to_string())?;

                // Broadcast de la suppression
                let delete_msg = WebSocketMessage {
                    action: "channel_deleted".to_string(),
                    data: serde_json::json!({
                        "channel_id": channel_id,
                        "server_id": server_id
                    }),
                };
                broadcast_to_server(&server_id, &delete_msg, &state.clients).await;

                let ack_msg = WebSocketMessage {
                    action: "channel_delete_ack".to_string(),
                    data: attach_request_id(
                        serde_json::json!({
                            "channel_id": channel_id,
                            "server_id": server_id
                        }),
                        request_id.as_deref(),
                    ),
                };
                send_to_client(client_id, &ack_msg, &state.clients).await?;

                println!("🗑️ Canal {} supprimé", channel_id);
            }
        }
        ClientEvent::MoveChannel => {
            let request_id = extract_request_id_from_data(&msg.data);
            if let (Some(channel_id), Some(position), Some(user_id)) = (
                msg.data.get("channel_id").and_then(|v| v.as_str()),
                msg.data.get("position").and_then(|v| v.as_i64()),
                msg.data.get("user_id").and_then(|v| v.as_str()),
            ) {
                // Récupérer le serveur et la catégorie actuelle du canal
                let channel_row =
                    sqlx::query("SELECT server_id, category_id FROM channels WHERE id = ?")
                        .bind(channel_id)
                        .fetch_one(&state.pool)
                        .await
                        .map_err(|e| e.to_string())?;
                let server_id: String = channel_row.get("server_id");
                let old_category_id: Option<String> = channel_row.get("category_id");

                // Vérifier les permissions
                if !can_manage_channels(&state.pool, user_id, &server_id)
                    .await
                    .map_err(|e| e.to_string())?
                {
                    let error_msg = WebSocketMessage {
                        action: "error".to_string(),
                        data: attach_request_id(
                            serde_json::json!({ "message": "You don't have permission to manage channels" }),
                            request_id.as_deref(),
                        ),
                    };
                    return send_to_client(client_id, &error_msg, &state.clients).await;
                }

                let category_field_present = msg.data.get("category_id").is_some();
                let new_category_id = if category_field_present {
                    msg.data
                        .get("category_id")
                        .and_then(|v| v.as_str())
                        .map(|v| v.to_string())
                } else {
                    old_category_id.clone()
                };

                let mut tx = state.pool.begin().await.map_err(|e| e.to_string())?;

                if category_field_present {
                    sqlx::query("UPDATE channels SET category_id = ?, position = ? WHERE id = ?")
                        .bind(new_category_id.as_deref())
                        .bind(position as i32)
                        .bind(channel_id)
                        .execute(&mut *tx)
                        .await
                        .map_err(|e| e.to_string())?;
                } else {
                    sqlx::query("UPDATE channels SET position = ? WHERE id = ?")
                        .bind(position as i32)
                        .bind(channel_id)
                        .execute(&mut *tx)
                        .await
                        .map_err(|e| e.to_string())?;
                }

                normalize_channel_positions_tx(&mut tx, &server_id, new_category_id.as_deref())
                    .await
                    .map_err(|e| e.to_string())?;

                if old_category_id != new_category_id {
                    normalize_channel_positions_tx(&mut tx, &server_id, old_category_id.as_deref())
                        .await
                        .map_err(|e| e.to_string())?;
                }

                tx.commit().await.map_err(|e| e.to_string())?;

                // Broadcast du déplacement
                let move_msg = WebSocketMessage {
                    action: "channel_moved".to_string(),
                    data: serde_json::json!({
                        "channel_id": channel_id,
                        "server_id": server_id,
                        "position": position,
                        "category_id": new_category_id.clone()
                    }),
                };
                broadcast_to_server(&server_id, &move_msg, &state.clients).await;

                let ack_msg = WebSocketMessage {
                    action: "channel_move_ack".to_string(),
                    data: attach_request_id(
                        serde_json::json!({
                            "channel_id": channel_id,
                            "server_id": server_id,
                            "position": position,
                            "category_id": new_category_id
                        }),
                        request_id.as_deref(),
                    ),
                };
                send_to_client(client_id, &ack_msg, &state.clients).await?;

                println!("📦 Canal {} déplacé à la position {}", channel_id, position);
            }
        }
        ClientEvent::ReorderChannels => {
            let request_id = extract_request_id_from_data(&msg.data);
            if let (Some(server_id), Some(user_id), Some(mut ordered_channel_ids)) = (
                msg.data.get("server_id").and_then(|v| v.as_str()),
                msg.data.get("user_id").and_then(|v| v.as_str()),
                parse_string_array(&msg.data, "ordered_channel_ids"),
            ) {
                if ordered_channel_ids.is_empty() {
                    let error_msg = WebSocketMessage {
                        action: "error".to_string(),
                        data: attach_request_id(
                            serde_json::json!({ "message": "ordered_channel_ids cannot be empty" }),
                            request_id.as_deref(),
                        ),
                    };
                    return send_to_client(client_id, &error_msg, &state.clients).await;
                }

                if !can_manage_channels(&state.pool, user_id, server_id)
                    .await
                    .map_err(|e| e.to_string())?
                {
                    let error_msg = WebSocketMessage {
                        action: "error".to_string(),
                        data: attach_request_id(
                            serde_json::json!({ "message": "You don't have permission to manage channels" }),
                            request_id.as_deref(),
                        ),
                    };
                    return send_to_client(client_id, &error_msg, &state.clients).await;
                }

                let category_id = msg
                    .data
                    .get("category_id")
                    .and_then(|v| v.as_str())
                    .map(|v| v.to_string());

                let current_rows = if let Some(category_id) = category_id.as_deref() {
                    sqlx::query(
                        "SELECT id FROM channels WHERE server_id = ? AND category_id = ? ORDER BY position ASC, id ASC",
                    )
                    .bind(server_id)
                    .bind(category_id)
                    .fetch_all(&state.pool)
                    .await
                    .map_err(|e| e.to_string())?
                } else {
                    sqlx::query(
                        "SELECT id FROM channels WHERE server_id = ? AND category_id IS NULL ORDER BY position ASC, id ASC",
                    )
                    .bind(server_id)
                    .fetch_all(&state.pool)
                    .await
                    .map_err(|e| e.to_string())?
                };

                let mut current_ids: Vec<String> = current_rows
                    .into_iter()
                    .map(|row| row.get::<String, _>("id"))
                    .collect();

                let mut expected_ids = current_ids.clone();
                expected_ids.sort();
                ordered_channel_ids.sort();

                if expected_ids != ordered_channel_ids {
                    let error_msg = WebSocketMessage {
                        action: "error".to_string(),
                        data: attach_request_id(
                            serde_json::json!({
                                "message": "Invalid channel reorder payload for this scope",
                                "code": "invalid_reorder_payload"
                            }),
                            request_id.as_deref(),
                        ),
                    };
                    return send_to_client(client_id, &error_msg, &state.clients).await;
                }

                // Rebuild canonical order from client payload while preserving original unsorted variable.
                current_ids.clear();
                current_ids.extend(
                    parse_string_array(&msg.data, "ordered_channel_ids")
                        .unwrap_or_default()
                        .into_iter(),
                );

                let mut tx = state.pool.begin().await.map_err(|e| e.to_string())?;
                for (position, channel_id) in current_ids.iter().enumerate() {
                    sqlx::query("UPDATE channels SET position = ? WHERE id = ? AND server_id = ?")
                        .bind(position as i32)
                        .bind(channel_id)
                        .bind(server_id)
                        .execute(&mut *tx)
                        .await
                        .map_err(|e| e.to_string())?;
                }

                normalize_channel_positions_tx(&mut tx, server_id, category_id.as_deref())
                    .await
                    .map_err(|e| e.to_string())?;
                tx.commit().await.map_err(|e| e.to_string())?;

                let broadcast_msg = WebSocketMessage {
                    action: "channels_reordered".to_string(),
                    data: serde_json::json!({
                        "server_id": server_id,
                        "category_id": category_id.clone(),
                        "ordered_channel_ids": current_ids.clone()
                    }),
                };
                broadcast_to_server(server_id, &broadcast_msg, &state.clients).await;

                let ack_msg = WebSocketMessage {
                    action: "channel_reorder_ack".to_string(),
                    data: attach_request_id(
                        serde_json::json!({
                            "server_id": server_id,
                            "category_id": category_id,
                            "ordered_channel_ids": current_ids
                        }),
                        request_id.as_deref(),
                    ),
                };
                send_to_client(client_id, &ack_msg, &state.clients).await?;
            }
        }
        // Gestion des catégories
        ClientEvent::CreateCategory => {
            let request_id = extract_request_id_from_data(&msg.data);
            if let (Some(server_id), Some(name), Some(user_id)) = (
                msg.data.get("server_id").and_then(|v| v.as_str()),
                msg.data.get("name").and_then(|v| v.as_str()),
                msg.data.get("user_id").and_then(|v| v.as_str()),
            ) {
                // Vérifier les permissions
                if !can_manage_channels(&state.pool, user_id, server_id)
                    .await
                    .map_err(|e| e.to_string())?
                {
                    let error_msg = WebSocketMessage {
                        action: "error".to_string(),
                        data: attach_request_id(
                            serde_json::json!({ "message": "You don't have permission to manage channels" }),
                            request_id.as_deref(),
                        ),
                    };
                    return send_to_client(client_id, &error_msg, &state.clients).await;
                }

                let category_id = uuid::Uuid::new_v4().to_string();
                let mut tx = state.pool.begin().await.map_err(|e| e.to_string())?;
                let next_position: i64 = sqlx::query_scalar(
                    "SELECT COALESCE(MAX(position), -1) + 1 FROM categories WHERE server_id = ?",
                )
                .bind(server_id)
                .fetch_one(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;

                // Créer la catégorie
                sqlx::query(
                    "INSERT INTO categories (id, server_id, name, position, created_at) VALUES (?, ?, ?, ?, datetime('now'))"
                )
                .bind(&category_id)
                .bind(server_id)
                .bind(name)
                .bind(next_position as i32)
                .execute(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;

                tx.commit().await.map_err(|e| e.to_string())?;

                // Broadcast de la création
                let create_msg = WebSocketMessage {
                    action: "category_created".to_string(),
                    data: serde_json::json!({
                        "category_id": category_id.clone(),
                        "server_id": server_id,
                        "name": name,
                        "position": next_position
                    }),
                };
                broadcast_to_server(server_id, &create_msg, &state.clients).await;

                let ack_msg = WebSocketMessage {
                    action: "category_create_ack".to_string(),
                    data: attach_request_id(
                        serde_json::json!({
                            "category_id": category_id,
                            "server_id": server_id,
                            "name": name,
                            "position": next_position
                        }),
                        request_id.as_deref(),
                    ),
                };
                send_to_client(client_id, &ack_msg, &state.clients).await?;

                println!("📁 Catégorie {} créée dans le serveur {}", name, server_id);
            }
        }
        ClientEvent::UpdateCategory => {
            let request_id = extract_request_id_from_data(&msg.data);
            if let (Some(category_id), Some(user_id)) = (
                msg.data.get("category_id").and_then(|v| v.as_str()),
                msg.data.get("user_id").and_then(|v| v.as_str()),
            ) {
                // Récupérer le serveur de la catégorie
                let server_id = sqlx::query_scalar::<_, String>(
                    "SELECT server_id FROM categories WHERE id = ?",
                )
                .bind(category_id)
                .fetch_one(&state.pool)
                .await
                .map_err(|e| e.to_string())?;

                // Vérifier les permissions
                if !can_manage_channels(&state.pool, user_id, &server_id)
                    .await
                    .map_err(|e| e.to_string())?
                {
                    let error_msg = WebSocketMessage {
                        action: "error".to_string(),
                        data: attach_request_id(
                            serde_json::json!({ "message": "You don't have permission to manage channels" }),
                            request_id.as_deref(),
                        ),
                    };
                    return send_to_client(client_id, &error_msg, &state.clients).await;
                }

                // Mettre à jour la catégorie
                if let Some(name) = msg.data.get("name").and_then(|v| v.as_str()) {
                    sqlx::query("UPDATE categories SET name = ? WHERE id = ?")
                        .bind(name)
                        .bind(category_id)
                        .execute(&state.pool)
                        .await
                        .map_err(|e| e.to_string())?;

                    // Broadcast de la mise à jour
                    let update_msg = WebSocketMessage {
                        action: "category_updated".to_string(),
                        data: serde_json::json!({
                            "category_id": category_id,
                            "server_id": server_id,
                            "name": name
                        }),
                    };
                    broadcast_to_server(&server_id, &update_msg, &state.clients).await;

                    let ack_msg = WebSocketMessage {
                        action: "category_update_ack".to_string(),
                        data: attach_request_id(
                            serde_json::json!({
                                "category_id": category_id,
                                "server_id": server_id,
                                "name": name
                            }),
                            request_id.as_deref(),
                        ),
                    };
                    send_to_client(client_id, &ack_msg, &state.clients).await?;

                    println!("✏️ Catégorie {} mise à jour", category_id);
                }
            }
        }
        ClientEvent::DeleteCategory => {
            let request_id = extract_request_id_from_data(&msg.data);
            if let (Some(category_id), Some(user_id)) = (
                msg.data.get("category_id").and_then(|v| v.as_str()),
                msg.data.get("user_id").and_then(|v| v.as_str()),
            ) {
                // Récupérer le serveur de la catégorie
                let server_id = sqlx::query_scalar::<_, String>(
                    "SELECT server_id FROM categories WHERE id = ?",
                )
                .bind(category_id)
                .fetch_one(&state.pool)
                .await
                .map_err(|e| e.to_string())?;

                // Vérifier les permissions
                if !can_manage_channels(&state.pool, user_id, &server_id)
                    .await
                    .map_err(|e| e.to_string())?
                {
                    let error_msg = WebSocketMessage {
                        action: "error".to_string(),
                        data: attach_request_id(
                            serde_json::json!({ "message": "You don't have permission to manage channels" }),
                            request_id.as_deref(),
                        ),
                    };
                    return send_to_client(client_id, &error_msg, &state.clients).await;
                }

                let mut tx = state.pool.begin().await.map_err(|e| e.to_string())?;

                // Rebasculer les canaux dans "Sans catégorie" puis normaliser
                sqlx::query("UPDATE channels SET category_id = NULL WHERE category_id = ?")
                    .bind(category_id)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| e.to_string())?;

                normalize_channel_positions_tx(&mut tx, &server_id, None)
                    .await
                    .map_err(|e| e.to_string())?;

                // Supprimer la catégorie puis normaliser les positions des catégories
                sqlx::query("DELETE FROM categories WHERE id = ?")
                    .bind(category_id)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| e.to_string())?;

                normalize_category_positions_tx(&mut tx, &server_id)
                    .await
                    .map_err(|e| e.to_string())?;

                tx.commit().await.map_err(|e| e.to_string())?;

                // Broadcast de la suppression
                let delete_msg = WebSocketMessage {
                    action: "category_deleted".to_string(),
                    data: serde_json::json!({
                        "category_id": category_id,
                        "server_id": server_id
                    }),
                };
                broadcast_to_server(&server_id, &delete_msg, &state.clients).await;

                let ack_msg = WebSocketMessage {
                    action: "category_delete_ack".to_string(),
                    data: attach_request_id(
                        serde_json::json!({
                            "category_id": category_id,
                            "server_id": server_id
                        }),
                        request_id.as_deref(),
                    ),
                };
                send_to_client(client_id, &ack_msg, &state.clients).await?;

                println!("🗑️ Catégorie {} supprimée", category_id);
            }
        }
        ClientEvent::MoveCategory => {
            let request_id = extract_request_id_from_data(&msg.data);
            if let (Some(category_id), Some(position), Some(user_id)) = (
                msg.data.get("category_id").and_then(|v| v.as_str()),
                msg.data.get("position").and_then(|v| v.as_i64()),
                msg.data.get("user_id").and_then(|v| v.as_str()),
            ) {
                // Récupérer le serveur de la catégorie
                let server_id = sqlx::query_scalar::<_, String>(
                    "SELECT server_id FROM categories WHERE id = ?",
                )
                .bind(category_id)
                .fetch_one(&state.pool)
                .await
                .map_err(|e| e.to_string())?;

                // Vérifier les permissions
                if !can_manage_channels(&state.pool, user_id, &server_id)
                    .await
                    .map_err(|e| e.to_string())?
                {
                    let error_msg = WebSocketMessage {
                        action: "error".to_string(),
                        data: attach_request_id(
                            serde_json::json!({ "message": "You don't have permission to manage channels" }),
                            request_id.as_deref(),
                        ),
                    };
                    return send_to_client(client_id, &error_msg, &state.clients).await;
                }

                let mut tx = state.pool.begin().await.map_err(|e| e.to_string())?;

                sqlx::query("UPDATE categories SET position = ? WHERE id = ?")
                    .bind(position as i32)
                    .bind(category_id)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| e.to_string())?;

                normalize_category_positions_tx(&mut tx, &server_id)
                    .await
                    .map_err(|e| e.to_string())?;

                tx.commit().await.map_err(|e| e.to_string())?;

                // Broadcast du déplacement
                let move_msg = WebSocketMessage {
                    action: "category_moved".to_string(),
                    data: serde_json::json!({
                        "category_id": category_id,
                        "server_id": server_id,
                        "position": position
                    }),
                };
                broadcast_to_server(&server_id, &move_msg, &state.clients).await;

                let ack_msg = WebSocketMessage {
                    action: "category_move_ack".to_string(),
                    data: attach_request_id(
                        serde_json::json!({
                            "category_id": category_id,
                            "server_id": server_id,
                            "position": position
                        }),
                        request_id.as_deref(),
                    ),
                };
                send_to_client(client_id, &ack_msg, &state.clients).await?;

                println!(
                    "📦 Catégorie {} déplacée à la position {}",
                    category_id, position
                );
            }
        }
        ClientEvent::ReorderCategories => {
            let request_id = extract_request_id_from_data(&msg.data);
            if let (Some(server_id), Some(user_id), Some(mut ordered_category_ids)) = (
                msg.data.get("server_id").and_then(|v| v.as_str()),
                msg.data.get("user_id").and_then(|v| v.as_str()),
                parse_string_array(&msg.data, "ordered_category_ids"),
            ) {
                if ordered_category_ids.is_empty() {
                    let error_msg = WebSocketMessage {
                        action: "error".to_string(),
                        data: attach_request_id(
                            serde_json::json!({ "message": "ordered_category_ids cannot be empty" }),
                            request_id.as_deref(),
                        ),
                    };
                    return send_to_client(client_id, &error_msg, &state.clients).await;
                }

                if !can_manage_channels(&state.pool, user_id, server_id)
                    .await
                    .map_err(|e| e.to_string())?
                {
                    let error_msg = WebSocketMessage {
                        action: "error".to_string(),
                        data: attach_request_id(
                            serde_json::json!({ "message": "You don't have permission to manage channels" }),
                            request_id.as_deref(),
                        ),
                    };
                    return send_to_client(client_id, &error_msg, &state.clients).await;
                }

                let current_rows = sqlx::query(
                    "SELECT id FROM categories WHERE server_id = ? ORDER BY position ASC, id ASC",
                )
                .bind(server_id)
                .fetch_all(&state.pool)
                .await
                .map_err(|e| e.to_string())?;

                let mut current_ids: Vec<String> = current_rows
                    .into_iter()
                    .map(|row| row.get::<String, _>("id"))
                    .collect();

                let mut expected_ids = current_ids.clone();
                expected_ids.sort();
                ordered_category_ids.sort();
                if expected_ids != ordered_category_ids {
                    let error_msg = WebSocketMessage {
                        action: "error".to_string(),
                        data: attach_request_id(
                            serde_json::json!({
                                "message": "Invalid category reorder payload for this server",
                                "code": "invalid_reorder_payload"
                            }),
                            request_id.as_deref(),
                        ),
                    };
                    return send_to_client(client_id, &error_msg, &state.clients).await;
                }

                current_ids.clear();
                current_ids.extend(
                    parse_string_array(&msg.data, "ordered_category_ids")
                        .unwrap_or_default()
                        .into_iter(),
                );

                let mut tx = state.pool.begin().await.map_err(|e| e.to_string())?;
                for (position, category_id) in current_ids.iter().enumerate() {
                    sqlx::query("UPDATE categories SET position = ? WHERE id = ? AND server_id = ?")
                        .bind(position as i32)
                        .bind(category_id)
                        .bind(server_id)
                        .execute(&mut *tx)
                        .await
                        .map_err(|e| e.to_string())?;
                }
                normalize_category_positions_tx(&mut tx, server_id)
                    .await
                    .map_err(|e| e.to_string())?;
                tx.commit().await.map_err(|e| e.to_string())?;

                let broadcast_msg = WebSocketMessage {
                    action: "categories_reordered".to_string(),
                    data: serde_json::json!({
                        "server_id": server_id,
                        "ordered_category_ids": current_ids.clone()
                    }),
                };
                broadcast_to_server(server_id, &broadcast_msg, &state.clients).await;

                let ack_msg = WebSocketMessage {
                    action: "category_reorder_ack".to_string(),
                    data: attach_request_id(
                        serde_json::json!({
                            "server_id": server_id,
                            "ordered_category_ids": current_ids
                        }),
                        request_id.as_deref(),
                    ),
                };
                send_to_client(client_id, &ack_msg, &state.clients).await?;
            }
        }
        // Gestion des rôles
        ClientEvent::CreateRole => {
            let request_id = extract_request_id_from_data(&msg.data);
            if let (Some(server_id), Some(name), Some(user_id)) = (
                msg.data.get("server_id").and_then(|v| v.as_str()),
                msg.data.get("name").and_then(|v| v.as_str()),
                msg.data.get("user_id").and_then(|v| v.as_str()),
            ) {
                // Vérifier les permissions
                if !can_manage_roles(&state.pool, user_id, server_id)
                    .await
                    .map_err(|e| e.to_string())?
                {
                    let error_msg = WebSocketMessage {
                        action: "error".to_string(),
                        data: attach_request_id(
                            serde_json::json!({ "message": "You don't have permission to manage roles" }),
                            request_id.as_deref(),
                        ),
                    };
                    return send_to_client(client_id, &error_msg, &state.clients).await;
                }

                let role_id = uuid::Uuid::new_v4().to_string();
                let color = msg
                    .data
                    .get("color")
                    .and_then(|v| v.as_str())
                    .unwrap_or("#5865F2");
                let permissions_bitfield = msg
                    .data
                    .get("permissions_bitfield")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0) as i32;

                // Créer le rôle
                sqlx::query(
                    "INSERT INTO server_roles (id, server_id, name, color, position, permissions_bitfield, is_default, created_at) VALUES (?, ?, ?, ?, 0, ?, 0, datetime('now'))"
                )
                .bind(&role_id)
                .bind(server_id)
                .bind(name)
                .bind(color)
                .bind(permissions_bitfield)
                .execute(&state.pool)
                .await
                .map_err(|e| e.to_string())?;

                // Broadcast de la création
                let create_msg = WebSocketMessage {
                    action: "role_created".to_string(),
                    data: serde_json::json!({
                        "role_id": role_id.clone(),
                        "server_id": server_id,
                        "name": name,
                        "color": color,
                        "permissions_bitfield": permissions_bitfield
                    }),
                };
                broadcast_to_server(server_id, &create_msg, &state.clients).await;

                let ack_msg = WebSocketMessage {
                    action: "role_create_ack".to_string(),
                    data: attach_request_id(
                        serde_json::json!({
                            "role_id": role_id,
                            "server_id": server_id,
                            "name": name,
                            "color": color,
                            "permissions_bitfield": permissions_bitfield
                        }),
                        request_id.as_deref(),
                    ),
                };
                send_to_client(client_id, &ack_msg, &state.clients).await?;

                println!("🎭 Rôle {} créé dans le serveur {}", name, server_id);
            }
        }
        ClientEvent::UpdateRole => {
            let request_id = extract_request_id_from_data(&msg.data);
            if let (Some(role_id), Some(user_id)) = (
                msg.data.get("role_id").and_then(|v| v.as_str()),
                msg.data.get("user_id").and_then(|v| v.as_str()),
            ) {
                // Récupérer le serveur du rôle
                let server_id = sqlx::query_scalar::<_, String>(
                    "SELECT server_id FROM server_roles WHERE id = ?",
                )
                .bind(role_id)
                .fetch_one(&state.pool)
                .await
                .map_err(|e| e.to_string())?;

                // Vérifier les permissions
                if !can_manage_roles(&state.pool, user_id, &server_id)
                    .await
                    .map_err(|e| e.to_string())?
                {
                    let error_msg = WebSocketMessage {
                        action: "error".to_string(),
                        data: attach_request_id(
                            serde_json::json!({ "message": "You don't have permission to manage roles" }),
                            request_id.as_deref(),
                        ),
                    };
                    return send_to_client(client_id, &error_msg, &state.clients).await;
                }

                // Vérifier que ce n'est pas un rôle par défaut
                let is_default: bool =
                    sqlx::query_scalar("SELECT is_default FROM server_roles WHERE id = ?")
                        .bind(role_id)
                        .fetch_one(&state.pool)
                        .await
                        .map_err(|e| e.to_string())?;

                if is_default {
                    let error_msg = WebSocketMessage {
                        action: "error".to_string(),
                        data: attach_request_id(
                            serde_json::json!({ "message": "Cannot modify default roles" }),
                            request_id.as_deref(),
                        ),
                    };
                    return send_to_client(client_id, &error_msg, &state.clients).await;
                }

                // Mettre à jour le rôle
                let mut update_query = "UPDATE server_roles SET ".to_string();
                let mut params = Vec::new();
                let mut set_clauses = Vec::new();

                if let Some(name) = msg.data.get("name").and_then(|v| v.as_str()) {
                    set_clauses.push("name = ?");
                    params.push(name.to_string());
                }
                if let Some(color) = msg.data.get("color").and_then(|v| v.as_str()) {
                    set_clauses.push("color = ?");
                    params.push(color.to_string());
                }
                if let Some(permissions_bitfield) = msg
                    .data
                    .get("permissions_bitfield")
                    .and_then(|v| v.as_i64())
                {
                    set_clauses.push("permissions_bitfield = ?");
                    params.push(permissions_bitfield.to_string());
                }

                if !set_clauses.is_empty() {
                    update_query.push_str(&set_clauses.join(", "));
                    update_query.push_str(" WHERE id = ?");
                    params.push(role_id.to_string());

                    let mut query = sqlx::query(&update_query);
                    for param in params {
                        query = query.bind(param);
                    }
                    query
                        .execute(&state.pool)
                        .await
                        .map_err(|e| e.to_string())?;

                    // Broadcast de la mise à jour
                    let update_msg = WebSocketMessage {
                        action: "role_updated".to_string(),
                        data: serde_json::json!({
                            "role_id": role_id,
                            "server_id": server_id,
                            "name": msg.data.get("name"),
                            "color": msg.data.get("color"),
                            "permissions_bitfield": msg.data.get("permissions_bitfield")
                        }),
                    };
                    broadcast_to_server(&server_id, &update_msg, &state.clients).await;

                    let ack_msg = WebSocketMessage {
                        action: "role_update_ack".to_string(),
                        data: attach_request_id(
                            serde_json::json!({
                                "role_id": role_id,
                                "server_id": server_id,
                                "name": msg.data.get("name"),
                                "color": msg.data.get("color"),
                                "permissions_bitfield": msg.data.get("permissions_bitfield")
                            }),
                            request_id.as_deref(),
                        ),
                    };
                    send_to_client(client_id, &ack_msg, &state.clients).await?;

                    println!("✏️ Rôle {} mis à jour", role_id);
                }
            }
        }
        ClientEvent::DeleteRole => {
            let request_id = extract_request_id_from_data(&msg.data);
            if let (Some(role_id), Some(user_id)) = (
                msg.data.get("role_id").and_then(|v| v.as_str()),
                msg.data.get("user_id").and_then(|v| v.as_str()),
            ) {
                // Récupérer le serveur du rôle
                let server_id = sqlx::query_scalar::<_, String>(
                    "SELECT server_id FROM server_roles WHERE id = ?",
                )
                .bind(role_id)
                .fetch_one(&state.pool)
                .await
                .map_err(|e| e.to_string())?;

                // Vérifier les permissions
                if !can_manage_roles(&state.pool, user_id, &server_id)
                    .await
                    .map_err(|e| e.to_string())?
                {
                    let error_msg = WebSocketMessage {
                        action: "error".to_string(),
                        data: attach_request_id(
                            serde_json::json!({ "message": "You don't have permission to manage roles" }),
                            request_id.as_deref(),
                        ),
                    };
                    return send_to_client(client_id, &error_msg, &state.clients).await;
                }

                // Vérifier que ce n'est pas un rôle par défaut
                let is_default: bool =
                    sqlx::query_scalar("SELECT is_default FROM server_roles WHERE id = ?")
                        .bind(role_id)
                        .fetch_one(&state.pool)
                        .await
                        .map_err(|e| e.to_string())?;

                if is_default {
                    let error_msg = WebSocketMessage {
                        action: "error".to_string(),
                        data: attach_request_id(
                            serde_json::json!({ "message": "Cannot delete default roles" }),
                            request_id.as_deref(),
                        ),
                    };
                    return send_to_client(client_id, &error_msg, &state.clients).await;
                }

                // Supprimer le rôle
                sqlx::query("DELETE FROM server_roles WHERE id = ?")
                    .bind(role_id)
                    .execute(&state.pool)
                    .await
                    .map_err(|e| e.to_string())?;

                // Broadcast de la suppression
                let delete_msg = WebSocketMessage {
                    action: "role_deleted".to_string(),
                    data: serde_json::json!({
                        "role_id": role_id,
                        "server_id": server_id
                    }),
                };
                broadcast_to_server(&server_id, &delete_msg, &state.clients).await;

                let ack_msg = WebSocketMessage {
                    action: "role_delete_ack".to_string(),
                    data: attach_request_id(
                        serde_json::json!({
                            "role_id": role_id,
                            "server_id": server_id
                        }),
                        request_id.as_deref(),
                    ),
                };
                send_to_client(client_id, &ack_msg, &state.clients).await?;

                println!("🗑️ Rôle {} supprimé", role_id);
            }
        }
        ClientEvent::AssignRole => {
            let request_id = extract_request_id_from_data(&msg.data);
            if let (Some(member_id), Some(role_id), Some(user_id)) = (
                msg.data.get("member_id").and_then(|v| v.as_str()),
                msg.data.get("role_id").and_then(|v| v.as_str()),
                msg.data.get("user_id").and_then(|v| v.as_str()),
            ) {
                // Récupérer le serveur du membre
                let server_id = sqlx::query_scalar::<_, String>(
                    "SELECT server_id FROM server_members WHERE id = ?",
                )
                .bind(member_id)
                .fetch_one(&state.pool)
                .await
                .map_err(|e| e.to_string())?;

                // Vérifier les permissions
                if !can_manage_roles(&state.pool, user_id, &server_id)
                    .await
                    .map_err(|e| e.to_string())?
                {
                    let error_msg = WebSocketMessage {
                        action: "error".to_string(),
                        data: attach_request_id(
                            serde_json::json!({ "message": "You don't have permission to manage roles" }),
                            request_id.as_deref(),
                        ),
                    };
                    return send_to_client(client_id, &error_msg, &state.clients).await;
                }

                // Vérifier que le rôle appartient au serveur
                let role_check =
                    sqlx::query("SELECT 1 FROM server_roles WHERE id = ? AND server_id = ?")
                        .bind(role_id)
                        .bind(&server_id)
                        .fetch_optional(&state.pool)
                        .await
                        .map_err(|e| e.to_string())?;

                if role_check.is_none() {
                    let error_msg = WebSocketMessage {
                        action: "error".to_string(),
                        data: attach_request_id(
                            serde_json::json!({ "message": "Role not found for this server" }),
                            request_id.as_deref(),
                        ),
                    };
                    return send_to_client(client_id, &error_msg, &state.clients).await;
                }

                // Assigner le rôle
                let assignment_id = uuid::Uuid::new_v4().to_string();
                sqlx::query(
                    "INSERT OR IGNORE INTO member_roles (id, member_id, role_id, assigned_at) VALUES (?, ?, ?, datetime('now'))"
                )
                .bind(&assignment_id)
                .bind(member_id)
                .bind(role_id)
                .execute(&state.pool)
                .await
                .map_err(|e| e.to_string())?;

                // Broadcast de l'assignation
                let assign_msg = WebSocketMessage {
                    action: "member_role_assigned".to_string(),
                    data: serde_json::json!({
                        "member_id": member_id,
                        "role_id": role_id,
                        "server_id": server_id
                    }),
                };
                broadcast_to_server(&server_id, &assign_msg, &state.clients).await;

                let ack_msg = WebSocketMessage {
                    action: "role_assign_ack".to_string(),
                    data: attach_request_id(
                        serde_json::json!({
                            "member_id": member_id,
                            "role_id": role_id,
                            "server_id": server_id
                        }),
                        request_id.as_deref(),
                    ),
                };
                send_to_client(client_id, &ack_msg, &state.clients).await?;

                println!("🎭 Rôle {} assigné au membre {}", role_id, member_id);
            }
        }
        ClientEvent::RemoveRole => {
            let request_id = extract_request_id_from_data(&msg.data);
            if let (Some(member_id), Some(role_id), Some(user_id)) = (
                msg.data.get("member_id").and_then(|v| v.as_str()),
                msg.data.get("role_id").and_then(|v| v.as_str()),
                msg.data.get("user_id").and_then(|v| v.as_str()),
            ) {
                // Récupérer le serveur du membre
                let server_id = sqlx::query_scalar::<_, String>(
                    "SELECT server_id FROM server_members WHERE id = ?",
                )
                .bind(member_id)
                .fetch_one(&state.pool)
                .await
                .map_err(|e| e.to_string())?;

                // Vérifier les permissions
                if !can_manage_roles(&state.pool, user_id, &server_id)
                    .await
                    .map_err(|e| e.to_string())?
                {
                    let error_msg = WebSocketMessage {
                        action: "error".to_string(),
                        data: attach_request_id(
                            serde_json::json!({ "message": "You don't have permission to manage roles" }),
                            request_id.as_deref(),
                        ),
                    };
                    return send_to_client(client_id, &error_msg, &state.clients).await;
                }

                // Retirer le rôle
                sqlx::query("DELETE FROM member_roles WHERE member_id = ? AND role_id = ?")
                    .bind(member_id)
                    .bind(role_id)
                    .execute(&state.pool)
                    .await
                    .map_err(|e| e.to_string())?;

                // Broadcast du retrait
                let remove_msg = WebSocketMessage {
                    action: "member_role_removed".to_string(),
                    data: serde_json::json!({
                        "member_id": member_id,
                        "role_id": role_id,
                        "server_id": server_id
                    }),
                };
                broadcast_to_server(&server_id, &remove_msg, &state.clients).await;

                let ack_msg = WebSocketMessage {
                    action: "role_remove_ack".to_string(),
                    data: attach_request_id(
                        serde_json::json!({
                            "member_id": member_id,
                            "role_id": role_id,
                            "server_id": server_id
                        }),
                        request_id.as_deref(),
                    ),
                };
                send_to_client(client_id, &ack_msg, &state.clients).await?;

                println!("🎭 Rôle {} retiré du membre {}", role_id, member_id);
            }
        }
        // Gestion des permissions
        ClientEvent::SetChannelPermissions => {
            let request_id = extract_request_id_from_data(&msg.data);
            if let (
                Some(channel_id),
                Some(role_id),
                Some(allow_bitfield),
                Some(deny_bitfield),
                Some(user_id),
            ) = (
                msg.data.get("channel_id").and_then(|v| v.as_str()),
                msg.data.get("role_id").and_then(|v| v.as_str()),
                msg.data.get("allow_bitfield").and_then(|v| v.as_i64()),
                msg.data.get("deny_bitfield").and_then(|v| v.as_i64()),
                msg.data.get("user_id").and_then(|v| v.as_str()),
            ) {
                // Récupérer le serveur du canal
                let server_id =
                    sqlx::query_scalar::<_, String>("SELECT server_id FROM channels WHERE id = ?")
                        .bind(channel_id)
                        .fetch_one(&state.pool)
                        .await
                        .map_err(|e| e.to_string())?;

                // Vérifier les permissions
                if !can_manage_channels(&state.pool, user_id, &server_id)
                    .await
                    .map_err(|e| e.to_string())?
                {
                    let error_msg = WebSocketMessage {
                        action: "error".to_string(),
                        data: attach_request_id(
                            serde_json::json!({ "message": "You don't have permission to manage channels" }),
                            request_id.as_deref(),
                        ),
                    };
                    return send_to_client(client_id, &error_msg, &state.clients).await;
                }

                // Définir les overrides
                let override_id = uuid::Uuid::new_v4().to_string();
                sqlx::query(
                    "INSERT OR REPLACE INTO channel_role_overrides (id, channel_id, role_id, allow_bitfield, deny_bitfield, created_at, updated_at) VALUES (?, ?, ?, ?, ?, datetime('now'), datetime('now'))"
                )
                .bind(&override_id)
                .bind(channel_id)
                .bind(role_id)
                .bind(allow_bitfield as i32)
                .bind(deny_bitfield as i32)
                .execute(&state.pool)
                .await
                .map_err(|e| e.to_string())?;

                // Broadcast de la mise à jour des permissions
                let perm_msg = WebSocketMessage {
                    action: "permissions_updated".to_string(),
                    data: serde_json::json!({
                        "channel_id": channel_id,
                        "role_id": role_id,
                        "allow_bitfield": allow_bitfield,
                        "deny_bitfield": deny_bitfield,
                        "type": "channel_role"
                    }),
                };
                broadcast_to_server(&server_id, &perm_msg, &state.clients).await;

                let ack_msg = WebSocketMessage {
                    action: "set_channel_permissions_ack".to_string(),
                    data: attach_request_id(
                        serde_json::json!({
                            "channel_id": channel_id,
                            "role_id": role_id,
                            "allow_bitfield": allow_bitfield,
                            "deny_bitfield": deny_bitfield,
                            "server_id": server_id,
                            "type": "channel_role"
                        }),
                        request_id.as_deref(),
                    ),
                };
                send_to_client(client_id, &ack_msg, &state.clients).await?;

                println!(
                    "🔐 Permissions du canal {} mises à jour pour le rôle {}",
                    channel_id, role_id
                );
            }
        }
        ClientEvent::SetCategoryPermissions => {
            let request_id = extract_request_id_from_data(&msg.data);
            if let (
                Some(category_id),
                Some(role_id),
                Some(allow_bitfield),
                Some(deny_bitfield),
                Some(user_id),
            ) = (
                msg.data.get("category_id").and_then(|v| v.as_str()),
                msg.data.get("role_id").and_then(|v| v.as_str()),
                msg.data.get("allow_bitfield").and_then(|v| v.as_i64()),
                msg.data.get("deny_bitfield").and_then(|v| v.as_i64()),
                msg.data.get("user_id").and_then(|v| v.as_str()),
            ) {
                // Récupérer le serveur de la catégorie
                let server_id = sqlx::query_scalar::<_, String>(
                    "SELECT server_id FROM categories WHERE id = ?",
                )
                .bind(category_id)
                .fetch_one(&state.pool)
                .await
                .map_err(|e| e.to_string())?;

                // Vérifier les permissions
                if !can_manage_roles(&state.pool, user_id, &server_id)
                    .await
                    .map_err(|e| e.to_string())?
                {
                    let error_msg = WebSocketMessage {
                        action: "error".to_string(),
                        data: attach_request_id(
                            serde_json::json!({ "message": "You don't have permission to manage roles" }),
                            request_id.as_deref(),
                        ),
                    };
                    return send_to_client(client_id, &error_msg, &state.clients).await;
                }

                // Définir les overrides
                let override_id = uuid::Uuid::new_v4().to_string();
                sqlx::query(
                    "INSERT OR REPLACE INTO category_role_overrides (id, category_id, role_id, allow_bitfield, deny_bitfield, created_at, updated_at) VALUES (?, ?, ?, ?, ?, datetime('now'), datetime('now'))"
                )
                .bind(&override_id)
                .bind(category_id)
                .bind(role_id)
                .bind(allow_bitfield as i32)
                .bind(deny_bitfield as i32)
                .execute(&state.pool)
                .await
                .map_err(|e| e.to_string())?;

                // Broadcast de la mise à jour des permissions
                let perm_msg = WebSocketMessage {
                    action: "permissions_updated".to_string(),
                    data: serde_json::json!({
                        "category_id": category_id,
                        "role_id": role_id,
                        "allow_bitfield": allow_bitfield,
                        "deny_bitfield": deny_bitfield,
                        "type": "category_role"
                    }),
                };
                broadcast_to_server(&server_id, &perm_msg, &state.clients).await;

                let ack_msg = WebSocketMessage {
                    action: "set_category_permissions_ack".to_string(),
                    data: attach_request_id(
                        serde_json::json!({
                            "category_id": category_id,
                            "role_id": role_id,
                            "allow_bitfield": allow_bitfield,
                            "deny_bitfield": deny_bitfield,
                            "server_id": server_id,
                            "type": "category_role"
                        }),
                        request_id.as_deref(),
                    ),
                };
                send_to_client(client_id, &ack_msg, &state.clients).await?;

                println!(
                    "🔐 Permissions de la catégorie {} mises à jour pour le rôle {}",
                    category_id, role_id
                );
            }
        }
        ClientEvent::SetMemberChannelPermissions => {
            let request_id = extract_request_id_from_data(&msg.data);
            if let (
                Some(channel_id),
                Some(member_id),
                Some(allow_bitfield),
                Some(deny_bitfield),
                Some(user_id),
            ) = (
                msg.data.get("channel_id").and_then(|v| v.as_str()),
                msg.data.get("member_id").and_then(|v| v.as_str()),
                msg.data.get("allow_bitfield").and_then(|v| v.as_i64()),
                msg.data.get("deny_bitfield").and_then(|v| v.as_i64()),
                msg.data.get("user_id").and_then(|v| v.as_str()),
            ) {
                let server_id =
                    sqlx::query_scalar::<_, String>("SELECT server_id FROM channels WHERE id = ?")
                        .bind(channel_id)
                        .fetch_one(&state.pool)
                        .await
                        .map_err(|e| e.to_string())?;

                if !can_manage_channels(&state.pool, user_id, &server_id)
                    .await
                    .map_err(|e| e.to_string())?
                {
                    let error_msg = WebSocketMessage {
                        action: "error".to_string(),
                        data: attach_request_id(
                            serde_json::json!({ "message": "You don't have permission to manage channels" }),
                            request_id.as_deref(),
                        ),
                    };
                    return send_to_client(client_id, &error_msg, &state.clients).await;
                }

                let member_check = sqlx::query(
                    "SELECT 1 FROM server_members WHERE id = ? AND server_id = ?",
                )
                .bind(member_id)
                .bind(&server_id)
                .fetch_optional(&state.pool)
                .await
                .map_err(|e| e.to_string())?;

                if member_check.is_none() {
                    let error_msg = WebSocketMessage {
                        action: "error".to_string(),
                        data: attach_request_id(
                            serde_json::json!({ "message": "Member not found for this server" }),
                            request_id.as_deref(),
                        ),
                    };
                    return send_to_client(client_id, &error_msg, &state.clients).await;
                }

                let override_id = uuid::Uuid::new_v4().to_string();
                sqlx::query(
                    "INSERT OR REPLACE INTO member_channel_overrides (id, channel_id, member_id, allow_bitfield, deny_bitfield, created_at, updated_at) VALUES (?, ?, ?, ?, ?, datetime('now'), datetime('now'))"
                )
                .bind(&override_id)
                .bind(channel_id)
                .bind(member_id)
                .bind(allow_bitfield as i32)
                .bind(deny_bitfield as i32)
                .execute(&state.pool)
                .await
                .map_err(|e| e.to_string())?;

                let perm_msg = WebSocketMessage {
                    action: "permissions_updated".to_string(),
                    data: serde_json::json!({
                        "channel_id": channel_id,
                        "member_id": member_id,
                        "allow_bitfield": allow_bitfield,
                        "deny_bitfield": deny_bitfield,
                        "type": "channel_member"
                    }),
                };
                broadcast_to_server(&server_id, &perm_msg, &state.clients).await;

                let ack_msg = WebSocketMessage {
                    action: "set_member_channel_permissions_ack".to_string(),
                    data: attach_request_id(
                        serde_json::json!({
                            "channel_id": channel_id,
                            "member_id": member_id,
                            "allow_bitfield": allow_bitfield,
                            "deny_bitfield": deny_bitfield,
                            "server_id": server_id,
                            "type": "channel_member"
                        }),
                        request_id.as_deref(),
                    ),
                };
                send_to_client(client_id, &ack_msg, &state.clients).await?;

                println!(
                    "🔐 Permissions du canal {} mises à jour pour le membre {}",
                    channel_id, member_id
                );
            }
        }
        ClientEvent::SetMemberCategoryPermissions => {
            let request_id = extract_request_id_from_data(&msg.data);
            if let (
                Some(category_id),
                Some(member_id),
                Some(allow_bitfield),
                Some(deny_bitfield),
                Some(user_id),
            ) = (
                msg.data.get("category_id").and_then(|v| v.as_str()),
                msg.data.get("member_id").and_then(|v| v.as_str()),
                msg.data.get("allow_bitfield").and_then(|v| v.as_i64()),
                msg.data.get("deny_bitfield").and_then(|v| v.as_i64()),
                msg.data.get("user_id").and_then(|v| v.as_str()),
            ) {
                let server_id = sqlx::query_scalar::<_, String>(
                    "SELECT server_id FROM categories WHERE id = ?",
                )
                .bind(category_id)
                .fetch_one(&state.pool)
                .await
                .map_err(|e| e.to_string())?;

                if !can_manage_roles(&state.pool, user_id, &server_id)
                    .await
                    .map_err(|e| e.to_string())?
                {
                    let error_msg = WebSocketMessage {
                        action: "error".to_string(),
                        data: attach_request_id(
                            serde_json::json!({ "message": "You don't have permission to manage roles" }),
                            request_id.as_deref(),
                        ),
                    };
                    return send_to_client(client_id, &error_msg, &state.clients).await;
                }

                let member_check = sqlx::query(
                    "SELECT 1 FROM server_members WHERE id = ? AND server_id = ?",
                )
                .bind(member_id)
                .bind(&server_id)
                .fetch_optional(&state.pool)
                .await
                .map_err(|e| e.to_string())?;

                if member_check.is_none() {
                    let error_msg = WebSocketMessage {
                        action: "error".to_string(),
                        data: attach_request_id(
                            serde_json::json!({ "message": "Member not found for this server" }),
                            request_id.as_deref(),
                        ),
                    };
                    return send_to_client(client_id, &error_msg, &state.clients).await;
                }

                let override_id = uuid::Uuid::new_v4().to_string();
                sqlx::query(
                    "INSERT OR REPLACE INTO member_category_overrides (id, category_id, member_id, allow_bitfield, deny_bitfield, created_at, updated_at) VALUES (?, ?, ?, ?, ?, datetime('now'), datetime('now'))"
                )
                .bind(&override_id)
                .bind(category_id)
                .bind(member_id)
                .bind(allow_bitfield as i32)
                .bind(deny_bitfield as i32)
                .execute(&state.pool)
                .await
                .map_err(|e| e.to_string())?;

                let perm_msg = WebSocketMessage {
                    action: "permissions_updated".to_string(),
                    data: serde_json::json!({
                        "category_id": category_id,
                        "member_id": member_id,
                        "allow_bitfield": allow_bitfield,
                        "deny_bitfield": deny_bitfield,
                        "type": "category_member"
                    }),
                };
                broadcast_to_server(&server_id, &perm_msg, &state.clients).await;

                let ack_msg = WebSocketMessage {
                    action: "set_member_category_permissions_ack".to_string(),
                    data: attach_request_id(
                        serde_json::json!({
                            "category_id": category_id,
                            "member_id": member_id,
                            "allow_bitfield": allow_bitfield,
                            "deny_bitfield": deny_bitfield,
                            "server_id": server_id,
                            "type": "category_member"
                        }),
                        request_id.as_deref(),
                    ),
                };
                send_to_client(client_id, &ack_msg, &state.clients).await?;

                println!(
                    "🔐 Permissions de la catégorie {} mises à jour pour le membre {}",
                    category_id, member_id
                );
            }
        }
        ClientEvent::GetEffectivePermissions => {
            let request_id = msg
                .data
                .get("request_id")
                .and_then(|v| v.as_str())
                .map(|v| v.to_string());

            if let (Some(user_id), channel_id, category_id) = (
                msg.data.get("user_id").and_then(|v| v.as_str()),
                msg.data.get("channel_id").and_then(|v| v.as_str()),
                msg.data.get("category_id").and_then(|v| v.as_str()),
            ) {
                let permissions = if let Some(ch_id) = channel_id {
                    // Récupérer les permissions effectives du canal
                    get_effective_channel_permissions(&state.pool, user_id, ch_id)
                        .await
                        .map_err(|e| e.to_string())?
                } else if let Some(cat_id) = category_id {
                    // Récupérer les permissions effectives de la catégorie
                    get_effective_category_permissions(&state.pool, user_id, cat_id)
                        .await
                        .map_err(|e| e.to_string())?
                } else {
                    let error_data = match &request_id {
                        Some(id) => serde_json::json!({
                            "message": "Either channel_id or category_id must be provided",
                            "request_id": id
                        }),
                        None => serde_json::json!({
                            "message": "Either channel_id or category_id must be provided"
                        }),
                    };

                    let error_msg = WebSocketMessage {
                        action: "error".to_string(),
                        data: error_data,
                    };
                    return send_to_client(client_id, &error_msg, &state.clients).await;
                };

                let mut response_data = serde_json::json!({
                    "channel_id": channel_id,
                    "category_id": category_id,
                    "allow_bitfield": permissions.allow_bitfield,
                    "deny_bitfield": permissions.deny_bitfield,
                    "final_permissions": permissions.final_permissions
                });

                if let Some(request_id) = request_id {
                    if let Some(object) = response_data.as_object_mut() {
                        object.insert(
                            "request_id".to_string(),
                            serde_json::Value::String(request_id),
                        );
                    }
                }

                let response = WebSocketMessage {
                    action: "effective_permissions".to_string(),
                    data: response_data,
                };
                send_to_client(client_id, &response, &state.clients).await?;
            }
        }
        ClientEvent::Unknown => {
            println!("Message inconnu: {:?}", msg);
        }
    }
    Ok(())
}

async fn handle_connection(
    stream: tokio::net::TcpStream,
    addr: std::net::SocketAddr,
    state: ServerState,
) -> Result<(), String> {
    let authenticated_user_id_slot = Arc::new(std::sync::Mutex::new(None::<String>));
    let authenticated_user_id_slot_cb = authenticated_user_id_slot.clone();
    let jwt_validation = state.jwt_validation.clone();

    let ws_stream = accept_hdr_async(stream, move |request: &Request, response: Response| {
        let token = request
            .uri()
            .query()
            .and_then(|query| extract_query_param(query, "token"))
            .ok_or_else(|| ws_auth_error("Missing WebSocket token"))?;

        let claims = validate_ws_token(token, jwt_validation.as_ref())
            .map_err(|err| ws_auth_error(&err))?;

        let mut slot = authenticated_user_id_slot_cb
            .lock()
            .map_err(|_| ws_auth_error("Auth state lock failure"))?;
        *slot = Some(claims.user_id);

        Ok(response)
    })
    .await
    .map_err(|e| e.to_string())?;

    let authenticated_user_id = authenticated_user_id_slot
        .lock()
        .map_err(|_| "Auth state lock failure".to_string())?
        .clone()
        .ok_or_else(|| "Missing authenticated user".to_string())?;

    let client_id = Uuid::new_v4().to_string();

    println!("Nouvelle connexion WebSocket: {} ({})", client_id, addr);

    let (ws_sender, mut ws_receiver) = ws_stream.split();

    let client = Client {
        id: client_id.clone(),
        authenticated_user_id: authenticated_user_id.clone(),
        user_id: None,
        username: None,
        current_channel: None,
        current_server: None,
        sender: Arc::new(Mutex::new(ws_sender)),
    };

    {
        let mut clients = state.clients.write().await;
        clients.insert(client_id.clone(), client);
    }

    let welcome_msg = WebSocketMessage {
        action: "connected".to_string(),
        data: serde_json::json!({ "client_id": client_id }),
    };

    {
        let clients = state.clients.read().await;
        if let Some(client) = clients.get(&client_id) {
            let mut sender = client.sender.lock().await;
            sender
                .send(WsMessage::Text(
                    serde_json::to_string(&welcome_msg).map_err(|e| e.to_string())?,
                ))
                .await
                .map_err(|e| e.to_string())?;
        }
    }

    // Écouter les messages
    while let Some(msg) = ws_receiver.next().await {
        match msg.map_err(|e| e.to_string())? {
            WsMessage::Text(text) => {
                if let Ok(ws_msg) = serde_json::from_str::<WebSocketMessage>(&text) {
                    if let Err(e) = handle_message(ws_msg, &client_id, &state).await {
                        eprintln!("Erreur traitement message: {}", e);
                    }
                }
            }
            WsMessage::Close(_) => {
                break;
            }
            _ => {}
        }
    }

    // Nettoyage à la déconnexion
    let removed_client = {
        let mut clients = state.clients.write().await;
        clients.remove(&client_id)
    };

    if let Some(client) = removed_client {
        if let (Some(server_id), Some(user_id)) = (client.current_server, client.user_id) {
            let username = client.username.unwrap_or_else(|| "Utilisateur".to_string());
            let (became_offline, presence_seq) = mark_presence_leave(&state, &server_id, &user_id).await;

            if became_offline {
                let _ = sqlx::query(
                    "UPDATE server_members SET is_online = 0, last_seen = datetime('now') WHERE server_id = ? AND user_id = ?",
                )
                .bind(&server_id)
                .bind(&user_id)
                .execute(&state.pool)
                .await;

                let leave_msg = WebSocketMessage {
                    action: "member_left".to_string(),
                    data: serde_json::json!({
                        "user_id": user_id,
                        "server_id": server_id
                    }),
                };
                broadcast_to_server(&server_id, &leave_msg, &state.clients).await;
            }

            if became_offline {
                broadcast_presence_delta(
                    &server_id,
                    &user_id,
                    &username,
                    false,
                    &state,
                    presence_seq,
                )
                .await;
            }
        }
    }
    println!("Client {} déconnecté", client_id);

    Ok(())
}

async fn heartbeat_to_backend(
    backend_url: String,
    server_id: String,
    auth_token: String,
    state: ServerState,
) {
    let client = reqwest::Client::new();
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
    let mut is_registered = false;

    println!("💓 Démarrage du heartbeat vers le backend: {}", backend_url);

    loop {
        interval.tick().await;

        // Compter les clients connectés
        let client_count = {
            let clients = state.clients.read().await;
            clients.len()
        };

        let heartbeat_data = serde_json::json!({
            "server_id": server_id,
            "status": "online",
            "connected_clients": client_count,
            "timestamp": chrono::Utc::now().to_rfc3339()
        });

        // Essayer d'abord l'enregistrement automatique si pas encore enregistré
        let endpoint = if !is_registered {
            format!("{}/api/servers/{}/register-auto", backend_url, server_id)
        } else {
            format!("{}/api/servers/{}/heartbeat-token", backend_url, server_id)
        };

        let mut request = client.post(&endpoint).json(&heartbeat_data);

        if !auth_token.is_empty() {
            request = request.header("Authorization", format!("Bearer {}", auth_token));
        }

        match request.send().await {
            Ok(response) => {
                if response.status().is_success() {
                    if !is_registered {
                        println!("✅ Serveur enregistré automatiquement auprès du backend");
                        is_registered = true;
                    } else {
                        println!(
                            "💓 Heartbeat envoyé au backend: {} clients connectés",
                            client_count
                        );
                    }
                } else {
                    println!(
                        "❌ Erreur heartbeat: {} - {}",
                        response.status(),
                        response.status()
                    );
                    if response.status() == 404 && !is_registered {
                        println!("🔄 Tentative d'enregistrement automatique...");
                        is_registered = false; // Réessayer l'enregistrement
                    }
                }
            }
            Err(e) => {
                println!("❌ Erreur connexion backend: {}", e);
                println!("💡 Vérifiez que le backend est démarré sur {}", backend_url);
                is_registered = false; // Réessayer l'enregistrement à la prochaine tentative
            }
        }
    }
}

fn detect_platform_tag() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}

#[derive(Debug, Deserialize)]
struct JwksKey {
    #[serde(default)]
    kid: Option<String>,
    kty: String,
    #[serde(default)]
    alg: Option<String>,
    #[serde(rename = "use", default)]
    key_use: Option<String>,
    #[serde(default)]
    n: Option<String>,
    #[serde(default)]
    e: Option<String>,
}

#[derive(Debug, Deserialize)]
struct JwksResponse {
    keys: Vec<JwksKey>,
}

async fn fetch_rs256_key_components_from_jwks(
    http_client: &reqwest::Client,
    jwks_url: &str,
    preferred_kid: Option<&str>,
) -> Result<(String, String), String> {
    let response = http_client
        .get(jwks_url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch JWKS: {e}"))?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("JWKS endpoint returned {}", status));
    }
    let body: JwksResponse = response
        .json()
        .await
        .map_err(|e| format!("Invalid JWKS response JSON: {e}"))?;

    let select_key = |key: &JwksKey| {
        key.kty.eq_ignore_ascii_case("RSA")
            && key.n.as_ref().map(|v| !v.is_empty()).unwrap_or(false)
            && key.e.as_ref().map(|v| !v.is_empty()).unwrap_or(false)
            && key
                .alg
                .as_ref()
                .map(|alg| alg.eq_ignore_ascii_case("RS256"))
                .unwrap_or(true)
            && key
                .key_use
                .as_ref()
                .map(|u| u.eq_ignore_ascii_case("sig"))
                .unwrap_or(true)
    };

    let key = if let Some(kid) = preferred_kid {
        body.keys
            .iter()
            .find(|k| k.kid.as_deref() == Some(kid) && select_key(k))
            .or_else(|| body.keys.iter().find(|k| select_key(k)))
    } else {
        body.keys.iter().find(|k| select_key(k))
    }
    .ok_or_else(|| "JWKS response missing compatible RSA (RS256) key with n/e".to_string())?;

    Ok((
        key.n.clone().unwrap_or_default(),
        key.e.clone().unwrap_or_default(),
    ))
}

async fn bootstrap_hosting(
    config: &RuntimeConfig,
    http_client: &reqwest::Client,
) -> Result<(HostingBootstrapResponse, HostingSnapshotResponse, JwtValidationConfig), String> {
    let endpoint = format!(
        "{}/api/server-hosting/bootstrap",
        config.backend_url.trim_end_matches('/')
    );

    let bootstrap_payload = serde_json::json!({
        "server_id": config.server_id,
        "owner_key": config.owner_key.clone().unwrap_or_default(),
        "instance_name": config.instance_name.clone().unwrap_or_else(|| "DiSpeak Server".to_string()),
        "advertised_host": config.ws_host,
        "advertised_port": config.ws_port as i32,
        "server_version": env!("CARGO_PKG_VERSION"),
        "platform": detect_platform_tag(),
    });

    let bootstrap_response = http_client
        .post(&endpoint)
        .timeout(std::time::Duration::from_millis(config.bootstrap_timeout_ms))
        .json(&bootstrap_payload)
        .send()
        .await
        .map_err(|e| format!("Bootstrap backend unreachable: {e}"))?;

    if !bootstrap_response.status().is_success() {
        let status = bootstrap_response.status();
        let body = bootstrap_response
            .text()
            .await
            .unwrap_or_else(|_| "<body unavailable>".to_string());
        return Err(format!("Bootstrap refused ({}): {}", status, body));
    }

    let bootstrap: HostingBootstrapResponse = bootstrap_response
        .json()
        .await
        .map_err(|e| format!("Invalid bootstrap response: {e}"))?;

    let jwt_validation = if bootstrap.jwt_verification.alg.eq_ignore_ascii_case("RS256") {
        let (modulus_b64url, exponent_b64url) = fetch_rs256_key_components_from_jwks(
            http_client,
            &bootstrap.jwt_verification.jwks_url,
            bootstrap.jwt_verification.kid.as_deref(),
        )
        .await?;
        JwtValidationConfig::Rs256 {
            modulus_b64url,
            exponent_b64url,
        }
    } else {
        if !config.allow_legacy_hs256 {
            return Err("Backend returned legacy HS256 JWT mode but security.allow_legacy_hs256=false".to_string());
        }
        let secret = config.jwt_secret.clone().ok_or_else(|| {
            "Legacy HS256 mode requires JWT_SECRET in environment (SERVER_ALLOW_LEGACY_HS256=true)"
                .to_string()
        })?;
        JwtValidationConfig::Hs256 {
            secret,
            allow_insecure_defaults: false,
            enforce_secret: true,
        }
    };

    let snapshot_endpoint = format!(
        "{}/api/server-hosting/{}/snapshot",
        config.backend_url.trim_end_matches('/'),
        bootstrap.server_id
    );
    let snapshot_response = http_client
        .get(&snapshot_endpoint)
        .bearer_auth(&bootstrap.runtime_token)
        .timeout(std::time::Duration::from_millis(config.bootstrap_timeout_ms))
        .send()
        .await
        .map_err(|e| format!("Snapshot request failed: {e}"))?;

    if !snapshot_response.status().is_success() {
        let status = snapshot_response.status();
        let body = snapshot_response
            .text()
            .await
            .unwrap_or_else(|_| "<body unavailable>".to_string());
        return Err(format!("Snapshot refused ({}): {}", status, body));
    }

    let snapshot: HostingSnapshotResponse = snapshot_response
        .json()
        .await
        .map_err(|e| format!("Invalid snapshot response: {e}"))?;

    Ok((bootstrap, snapshot, jwt_validation))
}

async fn apply_hosting_snapshot_to_local_db(
    pool: &SqlitePool,
    snapshot: &HostingSnapshotResponse,
) -> Result<(), sqlx::Error> {
    let server_id = snapshot.server.id.clone();
    let mut tx = pool.begin().await?;

    // Nettoyage ciblé du serveur (ordre dépendances -> parents)
    sqlx::query(
        "DELETE FROM member_roles WHERE member_id IN (SELECT id FROM server_members WHERE server_id = ?)"
    )
    .bind(&server_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "DELETE FROM member_channel_overrides WHERE member_id IN (SELECT id FROM server_members WHERE server_id = ?)"
    )
    .bind(&server_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "DELETE FROM member_category_overrides WHERE member_id IN (SELECT id FROM server_members WHERE server_id = ?)"
    )
    .bind(&server_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "DELETE FROM channel_role_overrides WHERE channel_id IN (SELECT id FROM channels WHERE server_id = ?)"
    )
    .bind(&server_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "DELETE FROM category_role_overrides WHERE category_id IN (SELECT id FROM categories WHERE server_id = ?)"
    )
    .bind(&server_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query("DELETE FROM server_roles WHERE server_id = ?")
        .bind(&server_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM voice_sessions WHERE channel_id IN (SELECT id FROM channels WHERE server_id = ?)")
        .bind(&server_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM channels WHERE server_id = ?")
        .bind(&server_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM categories WHERE server_id = ?")
        .bind(&server_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM server_members WHERE server_id = ?")
        .bind(&server_id)
        .execute(&mut *tx)
        .await?;

    let s = &snapshot.server;
    sqlx::query(
        "INSERT OR REPLACE INTO servers (id, owner_id, name, host, port, max_users, is_online, last_heartbeat, actual_host, actual_port, icon, color, description, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&s.id)
    .bind(&s.owner_id)
    .bind(&s.name)
    .bind(&s.host)
    .bind(s.port)
    .bind(s.max_users)
    .bind(s.is_online)
    .bind(&s.last_heartbeat)
    .bind(&s.actual_host)
    .bind(&s.actual_port)
    .bind(&s.icon)
    .bind(&s.color)
    .bind(&s.description)
    .bind(&s.created_at)
    .execute(&mut *tx)
    .await?;

    for member in &snapshot.members {
        sqlx::query(
            "INSERT OR REPLACE INTO server_members (id, server_id, user_id, username, avatar, role, joined_at, last_seen, is_online) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&member.id)
        .bind(&member.server_id)
        .bind(&member.user_id)
        .bind(&member.username)
        .bind(&member.avatar)
        .bind(&member.role)
        .bind(&member.joined_at)
        .bind(&member.last_seen)
        .bind(member.is_online)
        .execute(&mut *tx)
        .await?;
    }

    for category in &snapshot.categories {
        sqlx::query(
            "INSERT OR REPLACE INTO categories (id, server_id, name, position, created_at) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(&category.id)
        .bind(&category.server_id)
        .bind(&category.name)
        .bind(category.position)
        .bind(&category.created_at)
        .execute(&mut *tx)
        .await?;
    }

    for channel in &snapshot.channels {
        sqlx::query(
            "INSERT OR REPLACE INTO channels (id, server_id, name, type, category_id, position, description, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&channel.id)
        .bind(&channel.server_id)
        .bind(&channel.name)
        .bind(&channel.r#type)
        .bind(&channel.category_id)
        .bind(channel.position)
        .bind(&channel.description)
        .bind(&channel.created_at)
        .execute(&mut *tx)
        .await?;
    }

    for role in &snapshot.roles {
        sqlx::query(
            "INSERT OR REPLACE INTO server_roles (id, server_id, name, color, position, permissions_bitfield, is_default, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&role.id)
        .bind(&role.server_id)
        .bind(&role.name)
        .bind(&role.color)
        .bind(role.position)
        .bind(role.permissions_bitfield)
        .bind(role.is_default)
        .bind(&role.created_at)
        .execute(&mut *tx)
        .await?;
    }

    for mr in &snapshot.member_roles {
        sqlx::query(
            "INSERT OR REPLACE INTO member_roles (id, member_id, role_id, assigned_at) VALUES (?, ?, ?, ?)"
        )
        .bind(&mr.id)
        .bind(&mr.member_id)
        .bind(&mr.role_id)
        .bind(&mr.assigned_at)
        .execute(&mut *tx)
        .await?;
    }

    for ov in &snapshot.channel_role_overrides {
        sqlx::query(
            "INSERT OR REPLACE INTO channel_role_overrides (id, channel_id, role_id, allow_bitfield, deny_bitfield, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&ov.id)
        .bind(ov.channel_id.as_deref().unwrap_or_default())
        .bind(ov.role_id.as_deref().unwrap_or_default())
        .bind(ov.allow_bitfield)
        .bind(ov.deny_bitfield)
        .bind(&ov.created_at)
        .bind(&ov.created_at)
        .execute(&mut *tx)
        .await?;
    }

    for ov in &snapshot.category_role_overrides {
        sqlx::query(
            "INSERT OR REPLACE INTO category_role_overrides (id, category_id, role_id, allow_bitfield, deny_bitfield, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&ov.id)
        .bind(ov.category_id.as_deref().unwrap_or_default())
        .bind(ov.role_id.as_deref().unwrap_or_default())
        .bind(ov.allow_bitfield)
        .bind(ov.deny_bitfield)
        .bind(&ov.created_at)
        .bind(&ov.created_at)
        .execute(&mut *tx)
        .await?;
    }

    for ov in &snapshot.member_channel_overrides {
        sqlx::query(
            "INSERT OR REPLACE INTO member_channel_overrides (id, channel_id, member_id, allow_bitfield, deny_bitfield, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&ov.id)
        .bind(ov.channel_id.as_deref().unwrap_or_default())
        .bind(ov.member_id.as_deref().unwrap_or_default())
        .bind(ov.allow_bitfield)
        .bind(ov.deny_bitfield)
        .bind(&ov.created_at)
        .bind(&ov.created_at)
        .execute(&mut *tx)
        .await?;
    }

    for ov in &snapshot.member_category_overrides {
        sqlx::query(
            "INSERT OR REPLACE INTO member_category_overrides (id, category_id, member_id, allow_bitfield, deny_bitfield, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&ov.id)
        .bind(ov.category_id.as_deref().unwrap_or_default())
        .bind(ov.member_id.as_deref().unwrap_or_default())
        .bind(ov.allow_bitfield)
        .bind(ov.deny_bitfield)
        .bind(&ov.created_at)
        .bind(&ov.created_at)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

async fn heartbeat_to_backend_runtime(
    backend_url: String,
    server_id: String,
    runtime_token: String,
    actual_host: String,
    actual_port: u16,
    interval_sec: u64,
    state: ServerState,
) {
    let client = reqwest::Client::new();
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(interval_sec.max(5)));
    println!("💓 Démarrage du heartbeat runtime sécurisé vers le backend: {}", backend_url);

    loop {
        interval.tick().await;
        let client_count = {
            let clients = state.clients.read().await;
            clients.len()
        };

        let endpoint = format!(
            "{}/api/server-hosting/{}/heartbeat",
            backend_url.trim_end_matches('/'),
            server_id
        );

        let payload = serde_json::json!({
            "connected_clients": client_count,
            "status": "online",
            "actual_host": actual_host,
            "actual_port": actual_port as i32,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        match client
            .post(&endpoint)
            .bearer_auth(&runtime_token)
            .json(&payload)
            .send()
            .await
        {
            Ok(response) if response.status().is_success() => {
                println!("💓 Heartbeat runtime envoyé: {} clients", client_count);
            }
            Ok(response) => {
                let status = response.status();
                let body = response.text().await.unwrap_or_else(|_| "<body unavailable>".to_string());
                eprintln!("❌ Heartbeat runtime rejeté ({}): {}", status, body);
            }
            Err(err) => {
                eprintln!("❌ Heartbeat runtime erreur backend: {}", err);
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Démarrage du serveur DiSpeak...");

    let cli_command = parse_cli_args()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
    if matches!(cli_command, CliCommand::PrintConfigExample) {
        println!("{CONFIG_EXAMPLE}");
        return Ok(());
    }

    // Charger les variables d'environnement (fallback legacy).
    dotenv().ok();

    let config = match &cli_command {
        CliCommand::Run { config_path } | CliCommand::ValidateConfig { config_path } => {
            load_runtime_config(config_path.clone())
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?
        }
        CliCommand::PrintConfigExample => unreachable!(),
    };

    if matches!(cli_command, CliCommand::ValidateConfig { .. }) {
        println!("✅ Configuration valide");
        if let Some(path) = &config.config_path_used {
            println!("   Source config: {}", path.display());
        } else {
            println!("   Source config: .env / variables d'environnement (fallback legacy)");
        }
        println!("   Server ID: {}", config.server_id);
        println!("   Backend: {}", config.backend_url);
        println!(
            "   Mode bootstrap propriétaire: {}",
            if config.should_use_bootstrap() { "activé" } else { "désactivé (legacy)" }
        );
        return Ok(());
    }

    // Initialiser la base de données
    let pool = SqlitePool::connect(&config.database_url).await?;

    // Exécuter les migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("Erreur migration: {e}")))?;
    println!("✅ Base de données initialisée");

    // Évite les statuts fantômes après crash/redémarrage.
    if let Err(err) = sqlx::query("UPDATE server_members SET is_online = 0")
        .execute(&pool)
        .await
    {
        eprintln!("⚠️ Impossible de reset les présences au démarrage: {}", err);
    }

    let allow_insecure_defaults = env::var("SERVER_ALLOW_INSECURE_DEFAULTS")
        .unwrap_or_else(|_| "false".to_string())
        .eq_ignore_ascii_case("true");
    let enforce_jwt_secret = env::var("SERVER_ENFORCE_JWT_SECRET")
        .unwrap_or_else(|_| "true".to_string())
        .eq_ignore_ascii_case("true");
    let is_dev_like_env = matches!(
        env::var("SERVER_ENV")
            .or_else(|_| env::var("APP_ENV"))
            .unwrap_or_else(|_| "development".to_string())
            .to_lowercase()
            .as_str(),
        "dev" | "development" | "local" | "test"
    );
    let allow_legacy_runtime_flow = env::var("SERVER_ALLOW_LEGACY_RUNTIME_FLOW")
        .ok()
        .map(|v| v.eq_ignore_ascii_case("true"))
        .unwrap_or(is_dev_like_env);

    let http_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(config.bootstrap_timeout_ms))
        .build()?;

    let mut bootstrap_runtime: Option<HostingBootstrapResponse> = None;
    let jwt_validation = if config.should_use_bootstrap() {
        println!("🔐 Validation propriétaire activée: bootstrap backend requis");
        let (bootstrap, snapshot, jwt_validation) = bootstrap_hosting(&config, &http_client)
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::PermissionDenied, format!("Bootstrap serveur refusé: {e}")))?;
        apply_hosting_snapshot_to_local_db(&pool, &snapshot)
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("Sync snapshot backend->serveur échouée: {e}")))?;
        println!(
            "✅ Bootstrap validé pour le serveur {} (owner: {} / {})",
            bootstrap.server_id, bootstrap.owner_username, bootstrap.owner_user_id
        );
        println!("ℹ️ Runtime token expire à {}", bootstrap.runtime_token_expires_at);
        println!(
            "ℹ️ Vérification JWT WS: mode={} alg={}",
            bootstrap.jwt_verification.mode, bootstrap.jwt_verification.alg
        );
        bootstrap_runtime = Some(bootstrap.clone());
        jwt_validation
    } else {
        if !allow_legacy_runtime_flow {
            return Err("Mode runtime legacy (register-auto/heartbeat-token) désactivé: configurez OWNER_KEY + bootstrap backend, ou activez SERVER_ALLOW_LEGACY_RUNTIME_FLOW=true pour le dev".into());
        }
        let jwt_secret_present = config
            .jwt_secret
            .as_ref()
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false);
        if enforce_jwt_secret && !allow_insecure_defaults && !jwt_secret_present {
            return Err("JWT_SECRET requis (mode legacy HS256, désactivez SERVER_ENFORCE_JWT_SECRET ou activez SERVER_ALLOW_INSECURE_DEFAULTS pour le dev)".into());
        }
        JwtValidationConfig::Hs256 {
            secret: config.jwt_secret.clone().unwrap_or_default(),
            allow_insecure_defaults,
            enforce_secret: enforce_jwt_secret,
        }
    };

    // Créer l'état du serveur
    let state = ServerState {
        clients: Arc::new(RwLock::new(HashMap::new())),
        pool,
        sfu: SFU::new(),
        presence_connections: Arc::new(RwLock::new(HashMap::new())),
        presence_seq: Arc::new(RwLock::new(HashMap::new())),
        jwt_validation: Arc::new(jwt_validation),
    };

    // Démarrer le système de heartbeat vers le backend
    if let Some(bootstrap) = bootstrap_runtime {
        let heartbeat_state = state.clone();
        let backend_url = config.backend_url.clone();
        let server_id = bootstrap.server_id.clone();
        let runtime_token = bootstrap.runtime_token.clone();
        let heartbeat_interval = bootstrap.heartbeat_interval_sec.max(config.heartbeat_interval_sec);
        let actual_host = config.ws_host.clone();
        let actual_port = config.ws_port;
        tokio::spawn(async move {
            heartbeat_to_backend_runtime(
                backend_url,
                server_id,
                runtime_token,
                actual_host,
                actual_port,
                heartbeat_interval,
                heartbeat_state,
            )
            .await;
        });
    } else if !config.backend_url.is_empty() && !config.server_id.is_empty() {
        if config.legacy_auth_token.is_some() {
            eprintln!("⚠️ AUTH_TOKEN legacy détecté: flow register-auto/heartbeat-token déprécié");
        } else {
            eprintln!("⚠️ Démarrage en mode legacy sans OWNER_KEY (register-auto/heartbeat-token déprécié)");
        }
        let heartbeat_state = state.clone();
        let backend_url = config.backend_url.clone();
        let server_id = config.server_id.clone();
        let auth_token = config.legacy_auth_token.clone().unwrap_or_default();
        tokio::spawn(async move {
            heartbeat_to_backend(backend_url, server_id, auth_token, heartbeat_state).await;
        });
    }

    // Démarrer le serveur WebSocket
    let host = config.ws_host.clone();
    let port = config.ws_port;
    let addr = format!("{}:{}", host, port);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    println!("🌐 Serveur WebSocket démarré sur {}", addr);

    // Démarrer l'API HTTP interne (incluant l'endpoint de santé)
    let internal_api_host = config.internal_api_host.clone();
    let api_port: u16 = config.resolved_internal_api_port();
    let api_addr = format!("{}:{}", internal_api_host, api_port);
    let api_listener = tokio::net::TcpListener::bind(&api_addr).await?;
    println!("🛠️ API HTTP interne (incluant /health) démarrée sur {}", api_addr);

    let internal_api_token = config
        .internal_api_token
        .clone()
        .or_else(|| {
            eprintln!("⚠️ SERVER_INTERNAL_API_TOKEN absent: utilisation du token de dev par défaut (à remplacer en production)");
            Some("dispeak-internal-dev-token".to_string())
        });

    let http_router = http_api::build_router(state.pool.clone(), internal_api_token);

    tokio::spawn(async move {
        if let Err(err) = axum::serve(api_listener, http_router.into_make_service()).await {
            eprintln!("❌ Erreur serveur HTTP: {}", err);
        }
    });

    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                let state = state.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(stream, addr, state).await {
                        eprintln!("Erreur de connexion: {}", e);
                    }
                });
            }
            Err(err) => {
                eprintln!("❌ Erreur accept WebSocket: {}", err);
            }
        }
    }
}
