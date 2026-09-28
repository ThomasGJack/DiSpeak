use crate::app::AppState;
use crate::domain::{build_license, Claims, CreateLicense, License};
use crate::error::AppError;
use actix_web::{web, HttpResponse};
use sqlx::Row;

pub async fn create_license(
    state: web::Data<AppState>,
    license_data: web::Json<CreateLicense>,
) -> Result<HttpResponse, AppError> {
    let pool = state.pool();
    let mut license = build_license(license_data.user_id.clone(), license_data.server_slots);
    license.expires_at = license_data.expires_at.clone();

    sqlx::query(
        "INSERT INTO licenses (id, user_id, server_slots, active, expires_at, created_at) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&license.id)
    .bind(&license.user_id)
    .bind(&license.server_slots)
    .bind(&license.active)
    .bind(&license.expires_at)
    .bind(&license.created_at)
    .execute(pool)
    .await?;

    Ok(HttpResponse::Created().json(license))
}

pub async fn get_license(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let pool = state.pool();
    let license_id = path.into_inner();

    let row = sqlx::query(
        "SELECT id, user_id, server_slots, active, expires_at, created_at FROM licenses WHERE id = ?",
    )
    .bind(&license_id)
    .fetch_optional(pool)
    .await?;

    let row = match row {
        Some(row) => row,
        None => return Err(AppError::NotFound("Licence introuvable".into())),
    };

    let license = License {
        id: row.get("id"),
        user_id: row.get("user_id"),
        server_slots: row.get("server_slots"),
        active: row.get("active"),
        expires_at: row.get("expires_at"),
        created_at: row.get("created_at"),
    };

    Ok(HttpResponse::Ok().json(license))
}

pub async fn get_user_licenses(
    state: web::Data<AppState>,
    claims: Claims,
) -> Result<HttpResponse, AppError> {
    let pool = state.pool();

    let rows = sqlx::query(
        "SELECT id, user_id, server_slots, active, expires_at, created_at FROM licenses WHERE user_id = ?",
    )
    .bind(&claims.user_id)
    .fetch_all(pool)
    .await?;

    let licenses: Vec<License> = rows
        .into_iter()
        .map(|row| License {
            id: row.get("id"),
            user_id: row.get("user_id"),
            server_slots: row.get("server_slots"),
            active: row.get("active"),
            expires_at: row.get("expires_at"),
            created_at: row.get("created_at"),
        })
        .collect();

    Ok(HttpResponse::Ok().json(licenses))
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/licenses")
            .route("", web::post().to(create_license))
            .route("/{license_id}", web::get().to(get_license))
            .route("/my", web::get().to(get_user_licenses)),
    );
}

