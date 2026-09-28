use actix_web::{web, HttpResponse};
use serde_json::{json, Value};
use tracing::info;
use uuid::Uuid;

use crate::{
    app::AppState,
    domain::{
        Category, CategoryWithChannels, Channel, Claims, CreateCategory, CreateChannel, Server,
        ServerMember, ServerStructure,
    },
    error::AppError,
    routes::channels as legacy,
    services::permissions::can_manage_channels,
};

fn map_legacy_error(err: actix_web::Error) -> AppError {
    use actix_web::http::StatusCode;

    let status = err.as_response_error().status_code();
    let message = err.to_string();

    match status {
        StatusCode::BAD_REQUEST => AppError::BadRequest(message),
        StatusCode::UNAUTHORIZED => AppError::Unauthorized(message),
        StatusCode::FORBIDDEN => AppError::Forbidden(message),
        StatusCode::NOT_FOUND => AppError::NotFound(message),
        StatusCode::CONFLICT => AppError::Conflict(message),
        _ => AppError::Internal(message),
    }
}

fn pool_data(state: &web::Data<AppState>) -> web::Data<sqlx::SqlitePool> {
    web::Data::new(state.pool().clone())
}

pub async fn create_channel(
    state: web::Data<AppState>,
    claims: Claims,
    path: web::Path<String>,
    payload: web::Json<CreateChannel>,
) -> Result<HttpResponse, AppError> {
    let server_id = path.into_inner();
    let pool = state.pool();
    let data = payload.into_inner();

    if !can_manage_channels(pool, &claims.user_id, &server_id).await? {
        return Err(AppError::Forbidden(
            "Permissions insuffisantes pour créer un canal".into(),
        ));
    }

    if data.r#type != "text" && data.r#type != "voice" {
        return Err(AppError::bad_request("Type de canal invalide"));
    }

    if let Some(ref category_id) = data.category_id {
        let exists: i64 =
            sqlx::query_scalar("SELECT COUNT(1) FROM categories WHERE id = ? AND server_id = ?")
                .bind(category_id)
                .bind(&server_id)
                .fetch_one(pool)
                .await?;

        if exists == 0 {
            return Err(AppError::NotFound("Catégorie introuvable".into()));
        }
    }

    let position: i64 = if let Some(ref category_id) = data.category_id {
        sqlx::query_scalar(
            "SELECT COALESCE(MAX(position), 0) + 1 FROM channels WHERE server_id = ? AND category_id = ?",
        )
        .bind(&server_id)
        .bind(category_id)
        .fetch_one(pool)
        .await?
    } else {
        sqlx::query_scalar(
            "SELECT COALESCE(MAX(position), 0) + 1 FROM channels WHERE server_id = ? AND category_id IS NULL",
        )
        .bind(&server_id)
        .fetch_one(pool)
        .await?
    };

    let channel_id = Uuid::new_v4().to_string();

    sqlx::query(
        "INSERT INTO channels (id, server_id, name, type, category_id, position, description, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, datetime('now'))",
    )
    .bind(&channel_id)
    .bind(&server_id)
    .bind(&data.name)
    .bind(&data.r#type)
    .bind(&data.category_id)
    .bind(position as i32)
    .bind(&data.description)
    .execute(pool)
    .await?;

    let channel = sqlx::query_as::<_, Channel>(
        "SELECT id, server_id, name, type, category_id, position, description, created_at FROM channels WHERE id = ?",
    )
    .bind(&channel_id)
    .fetch_one(pool)
    .await?;

    info!(
        target: "http.channels",
        %server_id,
        %channel_id,
        channel_name = %channel.name,
        "Canal créé via API"
    );

    Ok(HttpResponse::Created().json(channel))
}

pub async fn create_category(
    state: web::Data<AppState>,
    claims: Claims,
    path: web::Path<String>,
    payload: web::Json<CreateCategory>,
) -> Result<HttpResponse, AppError> {
    let server_id = path.into_inner();
    let pool = state.pool();
    let data = payload.into_inner();

    if !can_manage_channels(pool, &claims.user_id, &server_id).await? {
        return Err(AppError::Forbidden(
            "Permissions insuffisantes pour créer une catégorie".into(),
        ));
    }

    let position: i64 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(position), 0) + 1 FROM categories WHERE server_id = ?",
    )
    .bind(&server_id)
    .fetch_one(pool)
    .await?;

    let category_id = Uuid::new_v4().to_string();

    sqlx::query(
        "INSERT INTO categories (id, server_id, name, position, created_at)
         VALUES (?, ?, ?, ?, datetime('now'))",
    )
    .bind(&category_id)
    .bind(&server_id)
    .bind(&data.name)
    .bind(position as i32)
    .execute(pool)
    .await?;

    let category = sqlx::query_as::<_, Category>(
        "SELECT id, server_id, name, position, created_at FROM categories WHERE id = ?",
    )
    .bind(&category_id)
    .fetch_one(pool)
    .await?;

    info!(
        target: "http.categories",
        %server_id,
        %category_id,
        category_name = %category.name,
        "Catégorie créée via API"
    );

    Ok(HttpResponse::Created().json(category))
}

pub async fn delete_channel(
    state: web::Data<AppState>,
    claims: Claims,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let channel_id = path.into_inner();
    let pool = state.pool();

    let server_id = sqlx::query_scalar::<_, String>("SELECT server_id FROM channels WHERE id = ?")
        .bind(&channel_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Canal introuvable".into()))?;

    if !can_manage_channels(pool, &claims.user_id, &server_id).await? {
        return Err(AppError::Forbidden(
            "Permissions insuffisantes pour supprimer un canal".into(),
        ));
    }

    sqlx::query("DELETE FROM channels WHERE id = ?")
        .bind(&channel_id)
        .execute(pool)
        .await?;

    info!(target: "http.channels", %server_id, %channel_id, "Canal supprimé via API");

    Ok(HttpResponse::Ok().json(json!({
        "message": "Channel deleted successfully",
        "channel_id": channel_id,
    })))
}

pub async fn delete_category(
    state: web::Data<AppState>,
    claims: Claims,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let category_id = path.into_inner();
    let pool = state.pool();

    let server_id =
        sqlx::query_scalar::<_, String>("SELECT server_id FROM categories WHERE id = ?")
            .bind(&category_id)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| AppError::NotFound("Catégorie introuvable".into()))?;

    if !can_manage_channels(pool, &claims.user_id, &server_id).await? {
        return Err(AppError::Forbidden(
            "Permissions insuffisantes pour supprimer une catégorie".into(),
        ));
    }

    sqlx::query("UPDATE channels SET category_id = NULL WHERE category_id = ?")
        .bind(&category_id)
        .execute(pool)
        .await?;

    sqlx::query("DELETE FROM categories WHERE id = ?")
        .bind(&category_id)
        .execute(pool)
        .await?;

    info!(target: "http.categories", %server_id, %category_id, "Catégorie supprimée via API");

    Ok(HttpResponse::Ok().json(json!({
        "message": "Category deleted successfully",
        "category_id": category_id,
    })))
}

pub async fn get_server_channels(
    state: web::Data<AppState>,
    claims: Claims,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let server_id = path.into_inner();
    let pool = state.pool();

    let member_check =
        sqlx::query("SELECT 1 FROM server_members WHERE server_id = ? AND user_id = ?")
            .bind(&server_id)
            .bind(&claims.user_id)
            .fetch_optional(pool)
            .await?;

    if member_check.is_none() {
        return Err(AppError::Forbidden(
            "Vous n'êtes pas membre de ce serveur".into(),
        ));
    }

    let channels = sqlx::query_as::<_, Channel>(
        "SELECT id, server_id, name, type, category_id, position, description, created_at
         FROM channels WHERE server_id = ? ORDER BY position ASC, created_at ASC",
    )
    .bind(&server_id)
    .fetch_all(pool)
    .await?;

    Ok(HttpResponse::Ok().json(channels))
}

pub async fn get_server_categories(
    state: web::Data<AppState>,
    claims: Claims,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let server_id = path.into_inner();
    let pool = state.pool();

    let member_check =
        sqlx::query("SELECT 1 FROM server_members WHERE server_id = ? AND user_id = ?")
            .bind(&server_id)
            .bind(&claims.user_id)
            .fetch_optional(pool)
            .await?;

    if member_check.is_none() {
        return Err(AppError::Forbidden(
            "Vous n'êtes pas membre de ce serveur".into(),
        ));
    }

    let categories = sqlx::query_as::<_, Category>(
        "SELECT id, server_id, name, position, created_at
         FROM categories WHERE server_id = ? ORDER BY position ASC, created_at ASC",
    )
    .bind(&server_id)
    .fetch_all(pool)
    .await?;

    Ok(HttpResponse::Ok().json(categories))
}

pub async fn get_server_structure(
    state: web::Data<AppState>,
    claims: Claims,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let server_id = path.into_inner();
    let pool = state.pool();

    let member_check =
        sqlx::query("SELECT 1 FROM server_members WHERE server_id = ? AND user_id = ?")
            .bind(&server_id)
            .bind(&claims.user_id)
            .fetch_optional(pool)
            .await?;

    if member_check.is_none() {
        let owner_check = sqlx::query("SELECT 1 FROM servers WHERE id = ? AND owner_id = ?")
            .bind(&server_id)
            .bind(&claims.user_id)
            .fetch_optional(pool)
            .await?;

        if owner_check.is_none() {
            return Err(AppError::Forbidden(
                "Vous n'êtes pas autorisé à consulter ce serveur".into(),
            ));
        }
    }

    let server = sqlx::query_as::<_, Server>(
        "SELECT id, owner_id, name, host, port, max_users, is_online, last_heartbeat,
                actual_host, actual_port, icon, color, description, created_at
         FROM servers WHERE id = ?",
    )
    .bind(&server_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Serveur introuvable".into()))?;

    let categories = sqlx::query_as::<_, Category>(
        "SELECT id, server_id, name, position, created_at
         FROM categories WHERE server_id = ? ORDER BY position ASC, created_at ASC",
    )
    .bind(&server_id)
    .fetch_all(pool)
    .await?;

    let channels = sqlx::query_as::<_, Channel>(
        "SELECT id, server_id, name, type, category_id, position, description, created_at
         FROM channels WHERE server_id = ? ORDER BY position ASC, created_at ASC",
    )
    .bind(&server_id)
    .fetch_all(pool)
    .await?;

    let members = sqlx::query_as::<_, ServerMember>(
        "SELECT id, server_id, user_id, username, avatar, role, joined_at, last_seen, is_online
         FROM server_members WHERE server_id = ?
         ORDER BY (role = 'owner') DESC, (role = 'moderator') DESC, is_online DESC, username COLLATE NOCASE ASC",
    )
    .bind(&server_id)
    .fetch_all(pool)
    .await?;

    let mut categories_with_channels = Vec::with_capacity(categories.len());
    for category in categories.iter().cloned() {
        let cat_channels: Vec<Channel> = channels
            .iter()
            .filter(|channel| channel.category_id.as_deref() == Some(&category.id))
            .cloned()
            .collect();

        categories_with_channels.push(CategoryWithChannels {
            category,
            channels: cat_channels,
        });
    }

    let uncategorized_channels: Vec<Channel> = channels
        .into_iter()
        .filter(|channel| channel.category_id.is_none())
        .collect();

    Ok(HttpResponse::Ok().json(ServerStructure {
        server,
        categories: categories_with_channels,
        uncategorized_channels,
        members,
    }))
}

pub async fn update_channel(
    state: web::Data<AppState>,
    claims: Claims,
    path: web::Path<String>,
    payload: web::Json<Value>,
) -> Result<HttpResponse, AppError> {
    let channel_id = path.into_inner();
    let pool = state.pool();

    let server_id = sqlx::query_scalar::<_, String>("SELECT server_id FROM channels WHERE id = ?")
        .bind(&channel_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Canal introuvable".into()))?;

    if !can_manage_channels(pool, &claims.user_id, &server_id).await? {
        return Err(AppError::Forbidden(
            "Permissions insuffisantes pour modifier ce canal".into(),
        ));
    }

    let body = payload.into_inner();
    let mut fields = Vec::new();
    let mut bindings: Vec<Value> = Vec::new();

    if let Some(name) = body.get("name").and_then(|v| v.as_str()) {
        fields.push("name = ?".to_string());
        bindings.push(Value::String(name.to_string()));
    }

    if let Some(description) = body.get("description").and_then(|v| v.as_str()) {
        fields.push("description = ?".to_string());
        bindings.push(Value::String(description.to_string()));
    }

    if fields.is_empty() {
        return Err(AppError::bad_request("Aucun champ valide à mettre à jour"));
    }

    let mut query = format!("UPDATE channels SET {} WHERE id = ?", fields.join(", "));
    let mut sql = sqlx::query(&query);

    for binding in bindings {
        match binding {
            Value::String(value) => {
                sql = sql.bind(value);
            }
            _ => {}
        }
    }

    sql = sql.bind(&channel_id);
    sql.execute(pool).await?;

    Ok(HttpResponse::Ok().json(json!({
        "message": "Channel updated successfully",
        "channel_id": channel_id,
    })))
}

pub async fn update_channel_position(
    state: web::Data<AppState>,
    claims: Claims,
    path: web::Path<String>,
    payload: web::Json<Value>,
) -> Result<HttpResponse, AppError> {
    let channel_id = path.into_inner();
    let pool = state.pool();

    let server_id = sqlx::query_scalar::<_, String>("SELECT server_id FROM channels WHERE id = ?")
        .bind(&channel_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Canal introuvable".into()))?;

    if !can_manage_channels(pool, &claims.user_id, &server_id).await? {
        return Err(AppError::Forbidden(
            "Permissions insuffisantes pour réordonner les canaux".into(),
        ));
    }

    let position = payload
        .get("position")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| AppError::bad_request("Position manquante"))?;

    sqlx::query("UPDATE channels SET position = ? WHERE id = ?")
        .bind(position as i32)
        .bind(&channel_id)
        .execute(pool)
        .await?;

    Ok(HttpResponse::Ok().json(json!({
        "message": "Channel position updated successfully",
        "channel_id": channel_id,
    })))
}

pub async fn update_category_position(
    state: web::Data<AppState>,
    claims: Claims,
    path: web::Path<String>,
    payload: web::Json<Value>,
) -> Result<HttpResponse, AppError> {
    let category_id = path.into_inner();
    let pool = state.pool();

    let server_id =
        sqlx::query_scalar::<_, String>("SELECT server_id FROM categories WHERE id = ?")
            .bind(&category_id)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| AppError::NotFound("Catégorie introuvable".into()))?;

    if !can_manage_channels(pool, &claims.user_id, &server_id).await? {
        return Err(AppError::Forbidden(
            "Permissions insuffisantes pour réordonner les catégories".into(),
        ));
    }

    let position = payload
        .get("position")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| AppError::bad_request("Position manquante"))?;

    sqlx::query("UPDATE categories SET position = ? WHERE id = ?")
        .bind(position as i32)
        .bind(&category_id)
        .execute(pool)
        .await?;

    Ok(HttpResponse::Ok().json(json!({
        "message": "Category position updated successfully",
        "category_id": category_id,
    })))
}

pub async fn get_channel_permissions(
    state: web::Data<AppState>,
    claims: Claims,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    legacy::get_channel_permissions(pool_data(&state), claims, path)
        .await
        .map_err(map_legacy_error)
}

pub async fn set_channel_permissions(
    state: web::Data<AppState>,
    claims: Claims,
    path: web::Path<String>,
    payload: web::Json<Value>,
) -> Result<HttpResponse, AppError> {
    legacy::set_channel_permissions(pool_data(&state), claims, path, payload)
        .await
        .map_err(map_legacy_error)
}

pub async fn remove_channel_permissions(
    state: web::Data<AppState>,
    claims: Claims,
    path: web::Path<(String, String)>,
) -> Result<HttpResponse, AppError> {
    legacy::remove_channel_permissions(pool_data(&state), claims, path)
        .await
        .map_err(map_legacy_error)
}

pub async fn set_member_channel_permissions(
    state: web::Data<AppState>,
    claims: Claims,
    path: web::Path<(String, String)>,
    payload: web::Json<Value>,
) -> Result<HttpResponse, AppError> {
    legacy::set_member_channel_permissions(pool_data(&state), claims, path, payload)
        .await
        .map_err(map_legacy_error)
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
                web::get().to(get_channel_permissions),
            )
            .route(
                "/{channel_id}/permissions",
                web::post().to(set_channel_permissions),
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
        web::scope("/categories")
            .route("/{category_id}", web::delete().to(delete_category))
            .route(
                "/{category_id}/position",
                web::put().to(update_category_position),
            ),
    );
}
