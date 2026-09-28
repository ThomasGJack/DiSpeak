use crate::error::AppError;
use sqlx::{Row, SqlitePool};

/// Vérifier qu'un utilisateur a un rôle admin ou owner
pub async fn require_admin(pool: &SqlitePool, user_id: &str) -> Result<(), AppError> {
    let user_row =
        sqlx::query("SELECT role FROM users WHERE id = ? AND deleted_at IS NULL AND is_active = 1")
            .bind(user_id)
            .fetch_optional(pool)
            .await?;

    match user_row {
        Some(row) => {
            let role: String = row.get("role");
            if role == "admin" || role == "owner" {
                Ok(())
            } else {
                Err(AppError::Forbidden(
                    "Permission refusée. Rôle admin requis.".into(),
                ))
            }
        }
        None => Err(AppError::NotFound("Utilisateur introuvable".into())),
    }
}

/// Vérifier qu'un utilisateur a un rôle moderator, admin ou owner
#[allow(dead_code)]
pub async fn require_moderator(pool: &SqlitePool, user_id: &str) -> Result<(), AppError> {
    let user_row =
        sqlx::query("SELECT role FROM users WHERE id = ? AND deleted_at IS NULL AND is_active = 1")
            .bind(user_id)
            .fetch_optional(pool)
            .await?;

    match user_row {
        Some(row) => {
            let role: String = row.get("role");
            if role == "moderator" || role == "admin" || role == "owner" {
                Ok(())
            } else {
                Err(AppError::Forbidden(
                    "Permission refusée. Rôle moderator requis.".into(),
                ))
            }
        }
        None => Err(AppError::NotFound("Utilisateur introuvable".into())),
    }
}
