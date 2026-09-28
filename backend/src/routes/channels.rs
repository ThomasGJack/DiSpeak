use actix_web::{web, HttpResponse, Result};
use reqwest::Client;
use serde_json::json;
use sqlx::{Row, SqlitePool};
use std::collections::HashMap;
use std::env;
use tracing::info;
use uuid::Uuid;

use crate::domain::{
    Category, CategoryWithChannels, Channel, Claims, CreateCategory, CreateChannel, Server,
    ServerMember, ServerStructure,
};
use crate::services::permissions::{
    can_manage_channels, get_effective_channel_permissions, remove_channel_role_overrides,
    set_channel_role_overrides, set_member_channel_overrides,
};

pub async fn create_channel(
    pool: web::Data<SqlitePool>,
    claims: Claims,
    path: web::Path<String>,
    channel_data: web::Json<CreateChannel>,
) -> Result<HttpResponse> {
    let server_id = path.into_inner();

    // Vérifier que l'utilisateur peut gérer les canaux
    if !can_manage_channels(pool.get_ref(), &claims.user_id, &server_id)
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?
    {
        return Ok(HttpResponse::Forbidden().json(json!({
            "error": "You don't have permission to create channels"
        })));
    }

    let payload = channel_data.into_inner();

    // Valider le type de canal
    if !["text", "voice"].contains(&payload.r#type.as_str()) {
        return Ok(HttpResponse::BadRequest().json(json!({
            "error": "Invalid channel type"
        })));
    }

    // Vérifier la catégorie si fournie
    if let Some(ref category_id) = payload.category_id {
        let category_check = sqlx::query("SELECT 1 FROM categories WHERE id = ? AND server_id = ?")
            .bind(category_id)
            .bind(&server_id)
            .fetch_optional(pool.get_ref())
            .await
            .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

        if category_check.is_none() {
            return Ok(HttpResponse::BadRequest().json(json!({
                "error": "Category not found for this server"
            })));
        }
    }

    let CreateChannel {
        name,
        r#type,
        category_id,
        description,
    } = payload;

    info!(target: "http.channels", %server_id, user_id = %claims.user_id, channel_name = %name, channel_type = %r#type, "Créer un canal via API");

    // Obtenir la position suivante
    let position: i64 = if let Some(ref category_id) = category_id {
        sqlx::query_scalar(
            "SELECT COALESCE(MAX(position), 0) + 1 FROM channels WHERE server_id = ? AND category_id = ?"
        )
        .bind(&server_id)
        .bind(category_id)
        .fetch_one(pool.get_ref())
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?
    } else {
        sqlx::query_scalar(
            "SELECT COALESCE(MAX(position), 0) + 1 FROM channels WHERE server_id = ? AND category_id IS NULL"
        )
        .bind(&server_id)
        .fetch_one(pool.get_ref())
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?
    };

    // Créer le canal
    let channel_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO channels (id, server_id, name, type, category_id, position, description, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, datetime('now'))"
    )
    .bind(&channel_id)
    .bind(&server_id)
    .bind(&name)
    .bind(&r#type)
    .bind(&category_id)
    .bind(position as i32)
    .bind(&description)
    .execute(pool.get_ref())
    .await
    .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    let channel = sqlx::query_as::<_, Channel>(
        "SELECT id, server_id, name, type, category_id, position, description, created_at FROM channels WHERE id = ?"
    )
    .bind(&channel_id)
    .fetch_one(pool.get_ref())
    .await
    .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    info!(target: "http.channels", %server_id, channel_id = %channel_id, "Canal créé");
    Ok(HttpResponse::Created().json(channel))
}

pub async fn create_category(
    pool: web::Data<SqlitePool>,
    claims: Claims,
    path: web::Path<String>,
    category_data: web::Json<CreateCategory>,
) -> Result<HttpResponse> {
    let server_id = path.into_inner();

    // Vérifier que l'utilisateur peut gérer les canaux
    if !can_manage_channels(pool.get_ref(), &claims.user_id, &server_id)
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?
    {
        return Ok(HttpResponse::Forbidden().json(json!({
            "error": "You don't have permission to create categories"
        })));
    }

    let payload = category_data.into_inner();

    // Obtenir la position suivante
    let position: i64 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(position), 0) + 1 FROM categories WHERE server_id = ?",
    )
    .bind(&server_id)
    .fetch_one(pool.get_ref())
    .await
    .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    // Créer la catégorie
    let category_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO categories (id, server_id, name, position, created_at) VALUES (?, ?, ?, ?, datetime('now'))"
    )
    .bind(&category_id)
    .bind(&server_id)
    .bind(&payload.name)
    .bind(position as i32)
    .execute(pool.get_ref())
    .await
    .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    let category = sqlx::query_as::<_, Category>(
        "SELECT id, server_id, name, position, created_at FROM categories WHERE id = ?",
    )
    .bind(&category_id)
    .fetch_one(pool.get_ref())
    .await
    .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    info!(target: "http.categories", %server_id, category_id = %category_id, "Catégorie créée");
    Ok(HttpResponse::Created().json(category))
}

pub async fn delete_category(
    pool: web::Data<SqlitePool>,
    claims: Claims,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let category_id = path.into_inner();

    let category_row = sqlx::query("SELECT server_id FROM categories WHERE id = ?")
        .bind(&category_id)
        .fetch_optional(pool.get_ref())
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    let server_id = match category_row {
        Some(row) => row.get::<String, _>("server_id"),
        None => {
            return Ok(HttpResponse::NotFound().json(json!({
                "error": "Category not found"
            })));
        }
    };

    if !can_manage_channels(pool.get_ref(), &claims.user_id, &server_id)
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?
    {
        return Ok(HttpResponse::Forbidden().json(json!({
            "error": "You don't have permission to manage channels"
        })));
    }

    // Détacher les canaux de la catégorie avant suppression
    sqlx::query("UPDATE channels SET category_id = NULL WHERE category_id = ?")
        .bind(&category_id)
        .execute(pool.get_ref())
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    sqlx::query("DELETE FROM categories WHERE id = ?")
        .bind(&category_id)
        .execute(pool.get_ref())
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    info!(target: "http.categories", %server_id, category_id = %category_id, "Catégorie supprimée");

    Ok(HttpResponse::Ok().json(json!({
        "message": "Category deleted successfully",
        "category_id": category_id,
    })))
}

pub async fn update_channel(
    pool: web::Data<SqlitePool>,
    claims: Claims,
    path: web::Path<String>,
    channel_data: web::Json<serde_json::Value>,
) -> Result<HttpResponse> {
    let channel_id = path.into_inner();

    // Récupérer le serveur du canal
    let server_row = sqlx::query("SELECT server_id FROM channels WHERE id = ?")
        .bind(&channel_id)
        .fetch_optional(pool.get_ref())
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    let server_id = match server_row {
        Some(row) => row.get::<String, _>("server_id"),
        None => {
            return Ok(HttpResponse::NotFound().json(json!({
                "error": "Channel not found"
            })))
        }
    };

    // Vérifier que l'utilisateur peut gérer les canaux
    if !can_manage_channels(pool.get_ref(), &claims.user_id, &server_id)
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?
    {
        return Ok(HttpResponse::Forbidden().json(json!({
            "error": "You don't have permission to update channels"
        })));
    }

    // Construire la requête de mise à jour dynamiquement
    let mut update_fields = Vec::new();
    let mut bind_values: Vec<String> = Vec::new();

    if let Some(name) = channel_data.get("name").and_then(|v| v.as_str()) {
        update_fields.push("name = ?");
        bind_values.push(name.to_string());
    }
    if let Some(description) = channel_data.get("description").and_then(|v| v.as_str()) {
        update_fields.push("description = ?");
        bind_values.push(description.to_string());
    }

    if update_fields.is_empty() {
        return Ok(HttpResponse::BadRequest().json(json!({
            "error": "No fields to update"
        })));
    }

    let query = format!(
        "UPDATE channels SET {} WHERE id = ?",
        update_fields.join(", ")
    );
    let mut query_builder = sqlx::query(&query);

    for value in bind_values {
        query_builder = query_builder.bind(value);
    }

    query_builder
        .bind(&channel_id)
        .execute(pool.get_ref())
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    Ok(HttpResponse::Ok().json(json!({
        "message": "Channel updated successfully"
    })))
}

pub async fn delete_channel(
    pool: web::Data<SqlitePool>,
    claims: Claims,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let channel_id = path.into_inner();

    // Récupérer le serveur du canal
    let server_row = sqlx::query("SELECT server_id FROM channels WHERE id = ?")
        .bind(&channel_id)
        .fetch_optional(pool.get_ref())
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    let server_id = match server_row {
        Some(row) => row.get::<String, _>("server_id"),
        None => {
            return Ok(HttpResponse::NotFound().json(json!({
                "error": "Channel not found"
            })))
        }
    };

    // Vérifier que l'utilisateur peut gérer les canaux
    if !can_manage_channels(pool.get_ref(), &claims.user_id, &server_id)
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?
    {
        return Ok(HttpResponse::Forbidden().json(json!({
            "error": "You don't have permission to delete channels"
        })));
    }

    // Supprimer le canal
    sqlx::query("DELETE FROM channels WHERE id = ?")
        .bind(&channel_id)
        .execute(pool.get_ref())
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    Ok(HttpResponse::Ok().json(json!({
        "message": "Channel deleted successfully"
    })))
}

pub async fn get_server_channels(
    pool: web::Data<SqlitePool>,
    claims: Claims,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let server_id = path.into_inner();

    // Vérifier que l'utilisateur est membre du serveur
    let member_check =
        sqlx::query("SELECT 1 FROM server_members WHERE server_id = ? AND user_id = ?")
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

    let channels = sqlx::query_as::<_, Channel>(
        "SELECT id, server_id, name, type, category_id, position, description, created_at FROM channels WHERE server_id = ? ORDER BY position ASC, created_at ASC"
    )
    .bind(&server_id)
    .fetch_all(pool.get_ref())
    .await
    .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    Ok(HttpResponse::Ok().json(channels))
}

pub async fn get_server_categories(
    pool: web::Data<SqlitePool>,
    claims: Claims,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let server_id = path.into_inner();

    let member_check =
        sqlx::query("SELECT 1 FROM server_members WHERE server_id = ? AND user_id = ?")
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

    let categories = sqlx::query_as::<_, Category>(
        "SELECT id, server_id, name, position, created_at FROM categories WHERE server_id = ? ORDER BY position ASC, created_at ASC"
    )
    .bind(&server_id)
    .fetch_all(pool.get_ref())
    .await
    .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    Ok(HttpResponse::Ok().json(categories))
}

pub async fn get_server_structure(
    pool: web::Data<SqlitePool>,
    http_client: web::Data<Client>,
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

    // Si pas membre, vérifier si c'est le propriétaire
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

    let server = match server {
        Some(server) => server,
        None => {
            return Ok(HttpResponse::NotFound().json(json!({
                "error": "Server not found"
            })));
        }
    };

    if let Some(remote_structure) =
        fetch_remote_server_structure(http_client.get_ref(), &server, &server_id).await
    {
        return Ok(HttpResponse::Ok().json(remote_structure));
    }

    let categories = sqlx::query_as::<_, Category>(
        "SELECT id, server_id, name, position, created_at FROM categories WHERE server_id = ? ORDER BY position ASC, created_at ASC"
    )
    .bind(&server_id)
    .fetch_all(pool.get_ref())
    .await
    .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    let channels = sqlx::query_as::<_, Channel>(
        "SELECT id, server_id, name, type, category_id, position, description, created_at FROM channels WHERE server_id = ? ORDER BY position ASC, created_at ASC"
    )
    .bind(&server_id)
    .fetch_all(pool.get_ref())
    .await
    .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    let members = sqlx::query_as::<_, ServerMember>(
        "SELECT id, server_id, user_id, username, avatar, role, joined_at, last_seen, is_online FROM server_members WHERE server_id = ? ORDER BY (role = 'owner') DESC, (role = 'moderator') DESC, is_online DESC, username COLLATE NOCASE ASC"
    )
    .bind(&server_id)
    .fetch_all(pool.get_ref())
    .await
    .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    let mut grouped: HashMap<String, Vec<Channel>> = HashMap::new();
    let mut uncategorized_channels: Vec<Channel> = Vec::new();

    for channel in channels {
        if let Some(ref category_id) = channel.category_id {
            grouped
                .entry(category_id.clone())
                .or_default()
                .push(channel);
        } else {
            uncategorized_channels.push(channel);
        }
    }

    let mut categories_with_channels = Vec::with_capacity(categories.len());
    for category in categories {
        let channels_for_category = grouped.remove(&category.id).unwrap_or_default();
        categories_with_channels.push(CategoryWithChannels {
            category,
            channels: channels_for_category,
        });
    }

    let structure = ServerStructure {
        server,
        categories: categories_with_channels,
        uncategorized_channels,
        members,
    };

    Ok(HttpResponse::Ok().json(structure))
}

async fn fetch_remote_server_structure(
    client: &Client,
    server: &Server,
    server_id: &str,
) -> Option<ServerStructure> {
    let host = server
        .actual_host
        .clone()
        .unwrap_or_else(|| server.host.clone());
    let ws_port = server.actual_port.unwrap_or(server.port);
    let api_port = env::var("SERVER_INTERNAL_API_PORT")
        .ok()
        .and_then(|value| value.parse::<i32>().ok())
        .unwrap_or_else(|| ws_port.saturating_add(1));
    let api_url = format!(
        "http://{}:{}/api/servers/{}/structure",
        host, api_port, server_id
    );

    let internal_api_token = env::var("SERVER_INTERNAL_API_TOKEN")
        .ok()
        .filter(|t| !t.trim().is_empty())
        .or_else(|| Some("dispeak-internal-dev-token".to_string()));
    let mut request = client.get(&api_url);
    if let Some(token) = internal_api_token.as_deref() {
        request = request.header("Authorization", format!("Bearer {}", token));
    }

    match request.send().await {
        Ok(response) if response.status().is_success() => {
            match response.json::<ServerStructure>().await {
                Ok(structure) => Some(structure),
                Err(err) => {
                    println!(
                        "❌ Impossible de parser la structure distante depuis {}: {}",
                        api_url, err
                    );
                    None
                }
            }
        }
        Ok(response) => {
            println!(
                "❌ Le serveur {} a répondu avec un statut inattendu: {}",
                api_url,
                response.status()
            );
            None
        }
        Err(err) => {
            println!(
                "❌ Impossible de joindre l'API du serveur {}: {}",
                api_url, err
            );
            None
        }
    }
}

/// Mettre à jour la position d'un canal
pub async fn update_channel_position(
    pool: web::Data<SqlitePool>,
    claims: Claims,
    path: web::Path<String>,
    position_data: web::Json<serde_json::Value>,
) -> Result<HttpResponse> {
    let channel_id = path.into_inner();

    // Récupérer le serveur du canal
    let server_row = sqlx::query("SELECT server_id FROM channels WHERE id = ?")
        .bind(&channel_id)
        .fetch_optional(pool.get_ref())
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    let server_id = match server_row {
        Some(row) => row.get::<String, _>("server_id"),
        None => {
            return Ok(HttpResponse::NotFound().json(json!({
                "error": "Channel not found"
            })))
        }
    };

    // Vérifier que l'utilisateur peut gérer les canaux
    if !can_manage_channels(pool.get_ref(), &claims.user_id, &server_id)
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?
    {
        return Ok(HttpResponse::Forbidden().json(json!({
            "error": "You don't have permission to manage channels"
        })));
    }

    let new_position = position_data
        .get("position")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| actix_web::error::ErrorBadRequest("Missing position"))?;

    // Mettre à jour la position
    sqlx::query("UPDATE channels SET position = ? WHERE id = ?")
        .bind(new_position as i32)
        .bind(&channel_id)
        .execute(pool.get_ref())
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    Ok(HttpResponse::Ok().json(json!({
        "message": "Channel position updated successfully"
    })))
}

/// Mettre à jour la position d'une catégorie
pub async fn update_category_position(
    pool: web::Data<SqlitePool>,
    claims: Claims,
    path: web::Path<String>,
    position_data: web::Json<serde_json::Value>,
) -> Result<HttpResponse> {
    let category_id = path.into_inner();

    // Récupérer le serveur de la catégorie
    let server_row = sqlx::query("SELECT server_id FROM categories WHERE id = ?")
        .bind(&category_id)
        .fetch_optional(pool.get_ref())
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    let server_id = match server_row {
        Some(row) => row.get::<String, _>("server_id"),
        None => {
            return Ok(HttpResponse::NotFound().json(json!({
                "error": "Category not found"
            })))
        }
    };

    // Vérifier que l'utilisateur peut gérer les canaux
    if !can_manage_channels(pool.get_ref(), &claims.user_id, &server_id)
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?
    {
        return Ok(HttpResponse::Forbidden().json(json!({
            "error": "You don't have permission to manage channels"
        })));
    }

    let new_position = position_data
        .get("position")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| actix_web::error::ErrorBadRequest("Missing position"))?;

    // Mettre à jour la position
    sqlx::query("UPDATE categories SET position = ? WHERE id = ?")
        .bind(new_position as i32)
        .bind(&category_id)
        .execute(pool.get_ref())
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    Ok(HttpResponse::Ok().json(json!({
        "message": "Category position updated successfully"
    })))
}

/// Définir les overrides de permissions pour un rôle sur un canal
pub async fn set_channel_permissions(
    pool: web::Data<SqlitePool>,
    claims: Claims,
    path: web::Path<String>,
    permissions_data: web::Json<serde_json::Value>,
) -> Result<HttpResponse> {
    let channel_id = path.into_inner();

    // Récupérer le serveur du canal
    let server_row = sqlx::query("SELECT server_id FROM channels WHERE id = ?")
        .bind(&channel_id)
        .fetch_optional(pool.get_ref())
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    let server_id = match server_row {
        Some(row) => row.get::<String, _>("server_id"),
        None => {
            return Ok(HttpResponse::NotFound().json(json!({
                "error": "Channel not found"
            })))
        }
    };

    // Vérifier que l'utilisateur peut gérer les canaux
    if !can_manage_channels(pool.get_ref(), &claims.user_id, &server_id)
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?
    {
        return Ok(HttpResponse::Forbidden().json(json!({
            "error": "You don't have permission to manage channels"
        })));
    }

    let role_id = permissions_data
        .get("role_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| actix_web::error::ErrorBadRequest("Missing role_id"))?;

    let allow_bitfield = permissions_data
        .get("allow_bitfield")
        .and_then(|v| v.as_i64())
        .unwrap_or(0) as i32;

    let deny_bitfield = permissions_data
        .get("deny_bitfield")
        .and_then(|v| v.as_i64())
        .unwrap_or(0) as i32;

    // Vérifier que le rôle appartient au serveur
    let role_check = sqlx::query("SELECT 1 FROM server_roles WHERE id = ? AND server_id = ?")
        .bind(role_id)
        .bind(&server_id)
        .fetch_optional(pool.get_ref())
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    if role_check.is_none() {
        return Ok(HttpResponse::BadRequest().json(json!({
            "error": "Role not found for this server"
        })));
    }

    // Définir les overrides
    set_channel_role_overrides(
        pool.get_ref(),
        &channel_id,
        role_id,
        allow_bitfield,
        deny_bitfield,
    )
    .await
    .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    Ok(HttpResponse::Ok().json(json!({
        "message": "Channel permissions updated successfully"
    })))
}

/// Définir les overrides de permissions pour un membre sur un canal
pub async fn set_member_channel_permissions(
    pool: web::Data<SqlitePool>,
    claims: Claims,
    path: web::Path<(String, String)>,
    permissions_data: web::Json<serde_json::Value>,
) -> Result<HttpResponse> {
    let (channel_id, member_id) = path.into_inner();

    // Récupérer le serveur du canal
    let server_row = sqlx::query("SELECT server_id FROM channels WHERE id = ?")
        .bind(&channel_id)
        .fetch_optional(pool.get_ref())
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    let server_id = match server_row {
        Some(row) => row.get::<String, _>("server_id"),
        None => {
            return Ok(HttpResponse::NotFound().json(json!({
                "error": "Channel not found"
            })))
        }
    };

    // Vérifier que l'utilisateur peut gérer les canaux
    if !can_manage_channels(pool.get_ref(), &claims.user_id, &server_id)
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?
    {
        return Ok(HttpResponse::Forbidden().json(json!({
            "error": "You don't have permission to manage channels"
        })));
    }

    // Vérifier que le membre appartient au serveur
    let member_check = sqlx::query("SELECT id FROM server_members WHERE id = ? AND server_id = ?")
        .bind(&member_id)
        .bind(&server_id)
        .fetch_optional(pool.get_ref())
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    if member_check.is_none() {
        return Ok(HttpResponse::BadRequest().json(json!({
            "error": "Member not found for this server"
        })));
    }

    let allow_bitfield = permissions_data
        .get("allow_bitfield")
        .and_then(|v| v.as_i64())
        .unwrap_or(0) as i32;

    let deny_bitfield = permissions_data
        .get("deny_bitfield")
        .and_then(|v| v.as_i64())
        .unwrap_or(0) as i32;

    // Définir les overrides
    set_member_channel_overrides(
        pool.get_ref(),
        &channel_id,
        &member_id,
        allow_bitfield,
        deny_bitfield,
    )
    .await
    .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    Ok(HttpResponse::Ok().json(json!({
        "message": "Member channel permissions updated successfully"
    })))
}

/// Récupérer les permissions effectives d'un utilisateur pour un canal
pub async fn get_channel_permissions(
    pool: web::Data<SqlitePool>,
    claims: Claims,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let channel_id = path.into_inner();

    // Récupérer le serveur du canal
    let server_row = sqlx::query("SELECT server_id FROM channels WHERE id = ?")
        .bind(&channel_id)
        .fetch_optional(pool.get_ref())
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    let server_id = match server_row {
        Some(row) => row.get::<String, _>("server_id"),
        None => {
            return Ok(HttpResponse::NotFound().json(json!({
                "error": "Channel not found"
            })))
        }
    };

    // Vérifier que l'utilisateur est membre du serveur
    let member_check =
        sqlx::query("SELECT 1 FROM server_members WHERE server_id = ? AND user_id = ?")
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

    // Récupérer les permissions effectives
    let effective_permissions =
        get_effective_channel_permissions(pool.get_ref(), &claims.user_id, &channel_id)
            .await
            .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    Ok(HttpResponse::Ok().json(json!({
        "channel_id": channel_id,
        "allow_bitfield": effective_permissions.allow_bitfield,
        "deny_bitfield": effective_permissions.deny_bitfield,
        "final_permissions": effective_permissions.final_permissions,
    })))
}

/// Supprimer les overrides de permissions pour un rôle sur un canal
pub async fn remove_channel_permissions(
    pool: web::Data<SqlitePool>,
    claims: Claims,
    path: web::Path<(String, String)>,
) -> Result<HttpResponse> {
    let (channel_id, role_id) = path.into_inner();

    // Récupérer le serveur du canal
    let server_row = sqlx::query("SELECT server_id FROM channels WHERE id = ?")
        .bind(&channel_id)
        .fetch_optional(pool.get_ref())
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    let server_id = match server_row {
        Some(row) => row.get::<String, _>("server_id"),
        None => {
            return Ok(HttpResponse::NotFound().json(json!({
                "error": "Channel not found"
            })))
        }
    };

    // Vérifier que l'utilisateur peut gérer les canaux
    if !can_manage_channels(pool.get_ref(), &claims.user_id, &server_id)
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?
    {
        return Ok(HttpResponse::Forbidden().json(json!({
            "error": "You don't have permission to manage channels"
        })));
    }

    // Supprimer les overrides
    remove_channel_role_overrides(pool.get_ref(), &channel_id, &role_id)
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    Ok(HttpResponse::Ok().json(json!({
        "message": "Channel permissions removed successfully"
    })))
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/channels")
            .route("/{channel_id}", web::put().to(update_channel))
            .route("/{channel_id}", web::delete().to(delete_channel))
            .route(
                "/{channel_id}/position",
                web::put().to(update_channel_position),
            )
            .route(
                "/{channel_id}/permissions",
                web::post().to(set_channel_permissions),
            )
            .route(
                "/{channel_id}/permissions",
                web::get().to(get_channel_permissions),
            )
            .route(
                "/{channel_id}/permissions/{role_id}",
                web::delete().to(remove_channel_permissions),
            )
            .route(
                "/{channel_id}/members/{member_id}/permissions",
                web::post().to(set_member_channel_permissions),
            ),
    );
    cfg.service(
        web::scope("/categories").route("/{category_id}", web::delete().to(delete_category)),
    );
}
