use crate::app::AppState;
use crate::domain::{
    build_local_user, build_oauth_user, AuthResponse, Claims, CreateUser, LoginUser, User,
};
use crate::error::AppError;
use crate::repositories::{get_next_user_number, log_audit};
use crate::services::auth::{create_token, jwt_kid, jwt_signing_mode, rsa_public_key_pem, JwtSigningMode};
use crate::services::owner_keys::ensure_user_owner_key;
use actix_web::{web, HttpResponse};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use bcrypt::{hash, verify, DEFAULT_COST};
use jsonwebtoken::jwk::{AlgorithmParameters, CommonParameters, Jwk, JwkSet, KeyAlgorithm, PublicKeyUse, RSAKeyParameters};
use rsa::pkcs8::DecodePublicKey;
use rsa::traits::PublicKeyParts;
use rsa::RsaPublicKey;
use serde::Deserialize;
use serde_json::json;
use sqlx::Row;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct GoogleLoginRequest {
    #[serde(rename = "accessToken")]
    pub access_token: String,
}

#[derive(Debug, Deserialize)]
pub struct ChangePasswordRequest {
    pub old_password: String,
    pub new_password: String,
}

#[derive(Debug, Deserialize)]
struct GoogleUserInfo {
    sub: String,
    email: String,
    name: Option<String>,
    picture: Option<String>,
    email_verified: Option<bool>,
}

const GOOGLE_USERINFO_URL: &str = "https://www.googleapis.com/oauth2/v3/userinfo";

pub async fn register(
    state: web::Data<AppState>,
    user_data: web::Json<CreateUser>,
) -> Result<HttpResponse, AppError> {
    let pool = state.pool();

    if let Some(row) =
        sqlx::query("SELECT id, provider FROM users WHERE email = ? AND deleted_at IS NULL")
            .bind(&user_data.email)
            .fetch_optional(pool)
            .await?
    {
        let provider: String = row.get("provider");
        return Err(AppError::Conflict(format!(
            "Email déjà utilisé avec un compte {}",
            provider
        )));
    }

    let user_number = get_next_user_number(pool).await?;
    let password_hash = hash(&user_data.password, DEFAULT_COST).map_err(|err| {
        tracing::error!(error = ?err, "Échec du hachage du mot de passe");
        AppError::internal("Impossible de sécuriser le mot de passe")
    })?;

    let user = build_local_user(
        user_data.username.clone(),
        user_data.email.clone(),
        user_number,
    );

    sqlx::query(
        "INSERT INTO users (id, user_number, username, email, provider, provider_id, password_hash, avatar, status, role, bio, is_active, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&user.id)
    .bind(&user.user_number)
    .bind(&user.username)
    .bind(&user.email)
    .bind(&user.provider)
    .bind(&user.provider_id)
    .bind(&password_hash)
    .bind(&user.avatar)
    .bind(&user.status)
    .bind(&user.role)
    .bind(&user.bio)
    .bind(&user.is_active)
    .bind(&user.created_at)
    .execute(pool)
    .await?;

    // Génère la clé propriétaire utilisateur dès la création de compte.
    let _ = ensure_user_owner_key(pool, &user.id).await?;

    log_audit(pool, &user.id, "register", None, None, None)
        .await
        .ok();

    let token = create_token(&user.id)?;

    Ok(HttpResponse::Created().json(AuthResponse { user, token }))
}

pub async fn login(
    state: web::Data<AppState>,
    login_data: web::Json<LoginUser>,
) -> Result<HttpResponse, AppError> {
    let pool = state.pool();

    let row = sqlx::query(
        "SELECT id, user_number, username, email, provider, provider_id, password_hash, avatar, status, role, bio, is_active, deleted_at, created_at FROM users WHERE email = ? AND deleted_at IS NULL",
    )
    .bind(&login_data.email)
    .fetch_optional(pool)
    .await?;

    let row = match row {
        Some(row) => row,
        None => return Err(AppError::Unauthorized("Identifiants invalides".into())),
    };

    let is_active: bool = row.get("is_active");
    if !is_active {
        return Err(AppError::Forbidden(
            "Compte désactivé. Contactez un administrateur.".into(),
        ));
    }

    let provider: String = row.get("provider");
    if provider != "local" {
        return Err(AppError::Unauthorized(
            "Veuillez utiliser la connexion via votre fournisseur".into(),
        ));
    }

    let stored_hash: String = row.get("password_hash");
    if !verify(&login_data.password, &stored_hash).map_err(|err| {
        tracing::error!(error = ?err, "Échec vérification mot de passe");
        AppError::internal("Impossible de vérifier le mot de passe")
    })? {
        return Err(AppError::Unauthorized("Identifiants invalides".into()));
    }

    let user = User {
        id: row.get("id"),
        user_number: row.get("user_number"),
        username: row.get("username"),
        email: row.get("email"),
        provider,
        provider_id: row.get("provider_id"),
        avatar: row.get("avatar"),
        status: row.get("status"),
        role: row.get("role"),
        bio: row.get("bio"),
        is_active,
        deleted_at: row.get("deleted_at"),
        created_at: row.get("created_at"),
    };

    log_audit(pool, &user.id, "login", None, None, None)
        .await
        .ok();

    let token = create_token(&user.id)?;
    Ok(HttpResponse::Ok().json(AuthResponse { user, token }))
}

pub async fn login_google(
    state: web::Data<AppState>,
    payload: web::Json<GoogleLoginRequest>,
) -> Result<HttpResponse, AppError> {
    if payload.access_token.trim().is_empty() {
        return Err(AppError::BadRequest("Access token manquant".into()));
    }

    let pool = state.pool();
    let client = state.http_client().clone();

    let user_info = client
        .get(GOOGLE_USERINFO_URL)
        .bearer_auth(&payload.access_token)
        .send()
        .await
        .map_err(|err| {
            tracing::error!(error = ?err, "Erreur Google OAuth");
            AppError::Internal("Erreur Google OAuth".into())
        })?
        .json::<GoogleUserInfo>()
        .await
        .map_err(|err| {
            tracing::warn!(error = ?err, "Lecture réponse Google échouée");
            AppError::Unauthorized("Impossible de lire la réponse Google".into())
        })?;

    if matches!(user_info.email_verified, Some(false)) {
        return Err(AppError::Unauthorized("Email Google non vérifié".into()));
    }

    let existing_user_by_google = sqlx::query(
        "SELECT id, user_number, username, email, provider, provider_id, avatar, status, role, bio, is_active, deleted_at, created_at FROM users WHERE provider = ? AND provider_id = ? AND deleted_at IS NULL",
    )
    .bind("google")
    .bind(&user_info.sub)
    .fetch_optional(pool)
    .await?
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
    });

    let user = if let Some(user) = existing_user_by_google {
        if !user.is_active {
            return Err(AppError::Forbidden(
                "Compte désactivé. Contactez un administrateur.".into(),
            ));
        }

        log_audit(pool, &user.id, "login", None, None, None)
            .await
            .ok();
        user
    } else {
        if let Some(row) =
            sqlx::query("SELECT id, provider FROM users WHERE email = ? AND deleted_at IS NULL")
                .bind(&user_info.email)
                .fetch_optional(pool)
                .await?
        {
            let provider: String = row.get("provider");
            return Err(AppError::Conflict(format!(
                "Email déjà utilisé avec un compte {}",
                provider
            )));
        }

        let user_number = get_next_user_number(pool).await?;
        let username = user_info.name.clone().unwrap_or_else(|| {
            user_info
                .email
                .split('@')
                .next()
                .unwrap_or("user")
                .to_string()
        });

        let new_user = build_oauth_user(
            username,
            user_info.email.clone(),
            "google".to_string(),
            user_info.sub.clone(),
            user_info.picture.clone(),
            user_number,
        );

        let placeholder_hash = hash(&Uuid::new_v4().to_string(), DEFAULT_COST).map_err(|err| {
            tracing::error!(error = ?err, "Échec génération hash placeholder");
            AppError::internal("Impossible de sécuriser le compte Google")
        })?;

        sqlx::query(
            "INSERT INTO users (id, user_number, username, email, provider, provider_id, password_hash, avatar, status, role, bio, is_active, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&new_user.id)
        .bind(&new_user.user_number)
        .bind(&new_user.username)
        .bind(&new_user.email)
        .bind(&new_user.provider)
        .bind(&new_user.provider_id)
        .bind(&placeholder_hash)
        .bind(&new_user.avatar)
        .bind(&new_user.status)
        .bind(&new_user.role)
        .bind(&new_user.bio)
        .bind(&new_user.is_active)
        .bind(&new_user.created_at)
        .execute(pool)
        .await?;

        // Génère la clé propriétaire utilisateur dès la création de compte Google.
        let _ = ensure_user_owner_key(pool, &new_user.id).await?;

        log_audit(
            pool,
            &new_user.id,
            "register",
            None,
            Some("via Google OAuth"),
            None,
        )
        .await
        .ok();

        new_user
    };

    let token = create_token(&user.id)?;
    Ok(HttpResponse::Ok().json(AuthResponse { user, token }))
}

pub async fn me(state: web::Data<AppState>, claims: Claims) -> Result<HttpResponse, AppError> {
    let pool = state.pool();

    let user_row = sqlx::query(
        "SELECT id, user_number, username, email, provider, provider_id, avatar, status, role, bio, is_active, deleted_at, created_at FROM users WHERE id = ? AND deleted_at IS NULL",
    )
    .bind(&claims.user_id)
    .fetch_optional(pool)
    .await?;

    let row = match user_row {
        Some(row) => row,
        None => return Err(AppError::NotFound("Utilisateur introuvable".into())),
    };

    if !row.get::<bool, _>("is_active") {
        return Err(AppError::Forbidden("Compte désactivé".into()));
    }

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

pub async fn refresh(claims: Claims) -> Result<HttpResponse, AppError> {
    let new_token = create_token(&claims.user_id)?;
    Ok(HttpResponse::Ok().json(json!({ "token": new_token })))
}

pub async fn jwks() -> Result<HttpResponse, AppError> {
    match jwt_signing_mode() {
        JwtSigningMode::Rs256 => {
            let public_pem = rsa_public_key_pem()?;
            let public_pem = String::from_utf8(public_pem)
                .map_err(|_| AppError::internal("Clé publique JWT invalide (UTF-8)"))?;
            let rsa_pub = RsaPublicKey::from_public_key_pem(&public_pem).map_err(|err| {
                tracing::error!(error = ?err, "Impossible de parser la clé publique RSA en JWK");
                AppError::internal("Clé publique JWT invalide")
            })?;

            let n = URL_SAFE_NO_PAD.encode(rsa_pub.n().to_bytes_be());
            let e = URL_SAFE_NO_PAD.encode(rsa_pub.e().to_bytes_be());
            let jwk = Jwk {
                common: CommonParameters {
                    public_key_use: Some(PublicKeyUse::Signature),
                    key_algorithm: Some(KeyAlgorithm::RS256),
                    key_id: jwt_kid(),
                    ..Default::default()
                },
                algorithm: AlgorithmParameters::RSA(RSAKeyParameters {
                    key_type: Default::default(),
                    n,
                    e,
                }),
            };
            Ok(HttpResponse::Ok().json(JwkSet { keys: vec![jwk] }))
        }
        JwtSigningMode::Hs256 => Ok(HttpResponse::Ok().json(json!({
            "keys": [],
            "mode": "hs256",
            "warning": "JWKS non disponible en mode HS256"
        }))),
    }
}

pub async fn change_password(
    state: web::Data<AppState>,
    claims: Claims,
    password_data: web::Json<ChangePasswordRequest>,
) -> Result<HttpResponse, AppError> {
    if password_data.new_password.len() < 8 {
        return Err(AppError::BadRequest(
            "Le mot de passe doit contenir au moins 8 caractères".into(),
        ));
    }

    let pool = state.pool();

    let user_row = sqlx::query(
        "SELECT provider, password_hash FROM users WHERE id = ? AND deleted_at IS NULL AND is_active = 1",
    )
    .bind(&claims.user_id)
    .fetch_optional(pool)
    .await?;

    let row = match user_row {
        Some(row) => row,
        None => return Err(AppError::NotFound("Utilisateur introuvable".into())),
    };

    let provider: String = row.get("provider");
    if provider != "local" {
        return Err(AppError::Forbidden(
            "Les comptes OAuth ne peuvent pas changer de mot de passe. Utilisez votre fournisseur."
                .into(),
        ));
    }

    let stored_hash: String = row.get("password_hash");
    if !verify(&password_data.old_password, &stored_hash).map_err(|err| {
        tracing::error!(error = ?err, "Échec vérification ancien mot de passe");
        AppError::internal("Impossible de vérifier le mot de passe actuel")
    })? {
        return Err(AppError::Unauthorized(
            "Ancien mot de passe incorrect".into(),
        ));
    }

    let new_hash = hash(&password_data.new_password, DEFAULT_COST).map_err(|err| {
        tracing::error!(error = ?err, "Échec hachage nouveau mot de passe");
        AppError::internal("Impossible de sécuriser le nouveau mot de passe")
    })?;

    sqlx::query("UPDATE users SET password_hash = ? WHERE id = ?")
        .bind(new_hash)
        .bind(&claims.user_id)
        .execute(pool)
        .await?;

    log_audit(
        pool,
        &claims.user_id,
        "password_changed",
        Some(&claims.user_id),
        None,
        None,
    )
    .await
    .ok();

    Ok(HttpResponse::Ok().json(json!({
        "message": "Mot de passe mis à jour avec succès"
    })))
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/auth")
            .route("/register", web::post().to(register))
            .route("/login", web::post().to(login))
            .route("/login/google", web::post().to(login_google))
            .route("/me", web::get().to(me))
            .route("/jwks", web::get().to(jwks))
            .route("/refresh", web::post().to(refresh))
            .route("/change-password", web::post().to(change_password)),
    );
}

