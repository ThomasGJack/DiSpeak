use crate::app::AppState;
use crate::domain::{Claims, User};
use crate::error::AppError;
use crate::repositories::log_audit;
use crate::services::owner_keys::{ensure_user_owner_key, rotate_user_owner_key};
use actix_web::{web, HttpResponse};
use serde_json::json;
use sqlx::{QueryBuilder, Row, Sqlite, SqlitePool};

pub async fn get_user(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let pool = state.pool();
    let user_id = path.into_inner();

    let row = sqlx::query(
        "SELECT id, user_number, username, email, provider, provider_id, avatar, status, role, bio, is_active, deleted_at, created_at FROM users WHERE id = ? AND deleted_at IS NULL",
    )
    .bind(&user_id)
    .fetch_optional(pool)
    .await?;

    let row = match row {
        Some(row) => row,
        None => return Err(AppError::NotFound("Utilisateur introuvable".into())),
    };

    let user = User {
        id: row.get("id"),
        user_number: row.get("user_number"),
        username: row.get("username"),
        email: row.get("email"),
        provider: row.get("provider"),
        provider_id: row.get("provider_id"),
        avatar: row.get("avatar"),
        status: row.get("status"),
        role: row.get("role"),
        bio: row.get("bio"),
        is_active: row.get("is_active"),
        deleted_at: row.get("deleted_at"),
        created_at: row.get("created_at"),
    };

    Ok(HttpResponse::Ok().json(user))
}

pub async fn update_user(
    state: web::Data<AppState>,
    claims: Claims,
    user_data: web::Json<serde_json::Value>,
) -> Result<HttpResponse, AppError> {
    let pool: &SqlitePool = state.pool();
    let mut query_builder = QueryBuilder::<Sqlite>::new("UPDATE users SET ");
    let mut columns = query_builder.separated(", ");
    let mut has_updates = false;

    if let Some(username) = user_data.get("username").and_then(|v| v.as_str()) {
        if username.len() < 3 || username.len() > 32 {
            return Err(AppError::BadRequest(
                "Le nom d'utilisateur doit contenir entre 3 et 32 caractères".into(),
            ));
        }

        let existing = sqlx::query(
            "SELECT id FROM users WHERE username = ? AND id != ? AND deleted_at IS NULL",
        )
        .bind(username)
        .bind(&claims.user_id)
        .fetch_optional(pool)
        .await?;

        if existing.is_some() {
            return Err(AppError::Conflict(
                "Ce nom d'utilisateur est déjà pris".into(),
            ));
        }

        has_updates = true;
        columns.push("username = ").push_bind(username);
    }

    if let Some(status) = user_data.get("status").and_then(|v| v.as_str()) {
        if !["online", "away", "busy", "offline"].contains(&status) {
            return Err(AppError::BadRequest("Statut invalide".into()));
        }
        has_updates = true;
        columns.push("status = ").push_bind(status);
    }

    if let Some(avatar) = user_data.get("avatar").and_then(|v| v.as_str()) {
        has_updates = true;
        columns.push("avatar = ").push_bind(avatar);
    }

    if let Some(bio) = user_data.get("bio").and_then(|v| v.as_str()) {
        if bio.len() > 500 {
            return Err(AppError::BadRequest(
                "La bio ne peut pas dépasser 500 caractères".into(),
            ));
        }
        has_updates = true;
        columns.push("bio = ").push_bind(bio);
    }

    if !has_updates {
        return Err(AppError::BadRequest(
            "Aucun champ valide à mettre à jour".into(),
        ));
    }

    drop(columns);

    query_builder
        .push(" WHERE id = ")
        .push_bind(&claims.user_id);

    let query = query_builder.build();
    let result = query.execute(pool).await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Utilisateur introuvable".into()));
    }

    log_audit(pool, &claims.user_id, "profile_update", None, None, None)
        .await
        .ok();

    Ok(HttpResponse::Ok().json(json!({"message": "Profil mis à jour"})))
}

pub async fn get_my_server_owner_key(
    state: web::Data<AppState>,
    claims: Claims,
) -> Result<HttpResponse, AppError> {
    let info = ensure_user_owner_key(state.pool(), &claims.user_id).await?;
    Ok(HttpResponse::Ok().json(info))
}

pub async fn regenerate_my_server_owner_key(
    state: web::Data<AppState>,
    claims: Claims,
) -> Result<HttpResponse, AppError> {
    let info = rotate_user_owner_key(state.pool(), &claims.user_id).await?;

    log_audit(
        state.pool(),
        &claims.user_id,
        "profile_update",
        Some(&claims.user_id),
        Some("server owner key rotated"),
        None,
    )
    .await
    .ok();

    Ok(HttpResponse::Ok().json(info))
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/users")
            .route("/me/server-owner-key", web::get().to(get_my_server_owner_key))
            .route(
                "/me/server-owner-key/regenerate",
                web::post().to(regenerate_my_server_owner_key),
            )
            .route("/me", web::patch().to(update_user))
            .route("/{user_id}", web::get().to(get_user)),
    );
}

