use actix_web::error::ErrorForbidden;
use actix_web::Error;
use sqlx::{Row, SqlitePool};

/// Middleware pour vérifier qu'un utilisateur est actif
#[allow(dead_code)]
pub async fn check_user_active(pool: &SqlitePool, user_id: &str) -> Result<(), Error> {
    let user_row = sqlx::query("SELECT is_active, deleted_at FROM users WHERE id = ?")
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(|_| ErrorForbidden("Database error"))?;

    match user_row {
        Some(row) => {
            let is_active: bool = row.get("is_active");
            let deleted_at: Option<String> = row.get("deleted_at");

            if !is_active || deleted_at.is_some() {
                return Err(ErrorForbidden("Compte désactivé ou supprimé"));
            }

            Ok(())
        }
        None => Err(ErrorForbidden("User not found")),
    }
}
