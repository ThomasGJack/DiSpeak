use crate::app::AppState;
use crate::domain::{Claims, User};
use crate::error::AppError;
use crate::middleware::require_role::require_admin;
use crate::repositories::log_audit;
use actix_web::{web, HttpResponse};
use serde::Deserialize;
use serde_json::json;
use sqlx::Row;

#[derive(Debug, Deserialize)]
pub struct DeactivateUserRequest {
    pub reason: Option<String>,
}

pub async fn get_all_users(
    state: web::Data<AppState>,
    claims: Claims,
) -> Result<HttpResponse, AppError> {
    let pool = state.pool();
    require_admin(pool, &claims.user_id).await?;

    let rows = sqlx::query(
        "SELECT id, user_number, username, email, provider, provider_id, avatar, status, role, bio, is_active, deleted_at, created_at FROM users ORDER BY user_number ASC",
    )
    .fetch_all(pool)
    .await?;

    let users: Vec<User> = rows
        .into_iter()
        .map(|row| User {
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
        })
        .collect();

    Ok(HttpResponse::Ok().json(users))
}

pub async fn get_user(
    state: web::Data<AppState>,
    claims: Claims,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let pool = state.pool();
    require_admin(pool, &claims.user_id).await?;

    let user_id = path.into_inner();

    let row = sqlx::query(
        "SELECT id, user_number, username, email, provider, provider_id, avatar, status, role, bio, is_active, deleted_at, created_at FROM users WHERE id = ?",
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

pub async fn deactivate_user(
    state: web::Data<AppState>,
    claims: Claims,
    path: web::Path<String>,
    body: web::Json<DeactivateUserRequest>,
) -> Result<HttpResponse, AppError> {
    let pool = state.pool();
    require_admin(pool, &claims.user_id).await?;

    let user_id = path.into_inner();
    if user_id == claims.user_id {
        return Err(AppError::BadRequest(
            "Vous ne pouvez pas désactiver votre propre compte".into(),
        ));
    }

    let result = sqlx::query(
        "UPDATE users SET is_active = 0, deleted_at = datetime('now'), deleted_by = ?, deletion_reason = ? WHERE id = ? AND deleted_at IS NULL",
    )
    .bind(&claims.user_id)
    .bind(&body.reason)
    .bind(&user_id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(
            "Utilisateur introuvable ou déjà désactivé".into(),
        ));
    }

    let details = body
        .reason
        .as_ref()
        .map(|reason| json!({ "reason": reason }).to_string());

    log_audit(
        pool,
        &user_id,
        "deactivated",
        Some(&claims.user_id),
        details.as_deref(),
        None,
    )
    .await
    .ok();

    Ok(HttpResponse::Ok().json(json!({
        "message": "Utilisateur désactivé avec succès"
    })))
}

pub async fn reactivate_user(
    state: web::Data<AppState>,
    claims: Claims,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let pool = state.pool();
    require_admin(pool, &claims.user_id).await?;

    let user_id = path.into_inner();

    let result = sqlx::query(
        "UPDATE users SET is_active = 1, deleted_at = NULL, deleted_by = NULL, deletion_reason = NULL WHERE id = ?",
    )
    .bind(&user_id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Utilisateur introuvable".into()));
    }

    log_audit(
        pool,
        &user_id,
        "reactivated",
        Some(&claims.user_id),
        None,
        None,
    )
    .await
    .ok();

    Ok(HttpResponse::Ok().json(json!({
        "message": "Utilisateur réactivé avec succès"
    })))
}

pub async fn get_user_audit(
    state: web::Data<AppState>,
    claims: Claims,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let pool = state.pool();
    require_admin(pool, &claims.user_id).await?;

    let user_id = path.into_inner();

    let audit_rows = sqlx::query(
        "SELECT id, user_id, action, performed_by, details, ip_address, created_at FROM user_audit_log WHERE user_id = ? ORDER BY created_at DESC",
    )
    .bind(&user_id)
    .fetch_all(pool)
    .await?;

    let audit_logs: Vec<serde_json::Value> = audit_rows
        .into_iter()
        .map(|row| {
            json!({
                "id": row.get::<String, _>("id"),
                "user_id": row.get::<String, _>("user_id"),
                "action": row.get::<String, _>("action"),
                "performed_by": row.get::<Option<String>, _>("performed_by"),
                "details": row.get::<Option<String>, _>("details"),
                "ip_address": row.get::<Option<String>, _>("ip_address"),
                "created_at": row.get::<String, _>("created_at"),
            })
        })
        .collect();

    Ok(HttpResponse::Ok().json(audit_logs))
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/admin")
            .route("/users", web::get().to(get_all_users))
            .route("/users/{user_id}", web::get().to(get_user))
            .route(
                "/users/{user_id}/deactivate",
                web::post().to(deactivate_user),
            )
            .route(
                "/users/{user_id}/reactivate",
                web::post().to(reactivate_user),
            )
            .route("/users/{user_id}/audit", web::get().to(get_user_audit)),
    );
}

