use crate::domain::auth::Claims;
use crate::error::AppError;
use actix_web::FromRequest;
use actix_web::{dev::Payload, HttpRequest};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::Serialize;
use std::future::{ready, Ready};
use std::fs;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum JwtSigningMode {
    Hs256,
    Rs256,
}

fn is_dev_like_env() -> bool {
    matches!(
        std::env::var("BACKEND_ENV")
            .or_else(|_| std::env::var("APP_ENV"))
            .unwrap_or_else(|_| "development".to_string())
            .to_lowercase()
            .as_str(),
        "dev" | "development" | "local" | "test"
    )
}

pub fn jwt_signing_mode() -> JwtSigningMode {
    let resolved = std::env::var("JWT_SIGNING_MODE").unwrap_or_else(|_| {
        if is_dev_like_env() {
            "hs256".to_string()
        } else {
            "rs256".to_string()
        }
    });

    match resolved.to_lowercase().as_str() {
        "rs256" => JwtSigningMode::Rs256,
        _ => JwtSigningMode::Hs256,
    }
}

fn hs256_secret() -> String {
    std::env::var("JWT_SECRET").unwrap_or_else(|_| "secret".to_string())
}

fn rsa_private_key_pem() -> Result<Vec<u8>, AppError> {
    let path = std::env::var("JWT_PRIVATE_KEY_PATH")
        .map_err(|_| AppError::internal("JWT_PRIVATE_KEY_PATH manquant (RS256)"))?;
    fs::read(&path).map_err(|err| {
        tracing::error!(error = ?err, path, "Impossible de lire la clé privée JWT");
        AppError::internal("Impossible de lire la clé privée JWT")
    })
}

pub fn rsa_public_key_pem() -> Result<Vec<u8>, AppError> {
    let path = std::env::var("JWT_PUBLIC_KEY_PATH")
        .map_err(|_| AppError::internal("JWT_PUBLIC_KEY_PATH manquant (RS256)"))?;
    fs::read(&path).map_err(|err| {
        tracing::error!(error = ?err, path, "Impossible de lire la clé publique JWT");
        AppError::internal("Impossible de lire la clé publique JWT")
    })
}

pub fn jwt_kid() -> Option<String> {
    std::env::var("JWT_KID").ok().filter(|v| !v.trim().is_empty())
}

pub fn create_token(user_id: &str) -> Result<String, AppError> {
    let claims = Claims {
        user_id: user_id.to_string(),
        exp: (chrono::Utc::now() + chrono::Duration::hours(24)).timestamp() as usize,
    };

    let (header, key) = match jwt_signing_mode() {
        JwtSigningMode::Hs256 => (
            Header::default(),
            EncodingKey::from_secret(hs256_secret().as_ref()),
        ),
        JwtSigningMode::Rs256 => {
            let mut header = Header::new(Algorithm::RS256);
            header.kid = jwt_kid();
            let private_pem = rsa_private_key_pem()?;
            let key = EncodingKey::from_rsa_pem(&private_pem).map_err(|err| {
                tracing::error!(error = ?err, "Impossible de charger la clé privée RSA JWT");
                AppError::internal("Clé privée JWT invalide")
            })?;
            (header, key)
        }
    };

    encode(&header, &claims, &key).map_err(|err| {
        tracing::error!(error = ?err, "Impossible de créer le token JWT");
        AppError::internal("Impossible de créer le token d'authentification")
    })
}

pub fn validate_token(token: &str) -> Result<Claims, AppError> {
    let (key, validation) = match jwt_signing_mode() {
        JwtSigningMode::Hs256 => (
            DecodingKey::from_secret(hs256_secret().as_ref()),
            Validation::new(Algorithm::HS256),
        ),
        JwtSigningMode::Rs256 => {
            let public_pem = rsa_public_key_pem()?;
            let key = DecodingKey::from_rsa_pem(&public_pem).map_err(|err| {
                tracing::error!(error = ?err, "Impossible de charger la clé publique RSA JWT");
                AppError::internal("Clé publique JWT invalide")
            })?;
            (key, Validation::new(Algorithm::RS256))
        }
    };

    decode::<Claims>(token, &key, &validation)
        .map(|data| data.claims)
        .map_err(|err| {
            tracing::warn!(error = ?err, "Token JWT invalide");
            AppError::Unauthorized("Token invalide".to_string())
        })
}

impl FromRequest for Claims {
    type Error = actix_web::Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let auth_header = req.headers().get("Authorization");

        match auth_header {
            Some(header) => {
                let header_str = header.to_str().unwrap_or("");
                if let Some(token) = header_str.strip_prefix("Bearer ") {
                    match validate_token(token) {
                        Ok(claims) => ready(Ok(claims)),
                        Err(err) => ready(Err(actix_web::error::ErrorUnauthorized(err))),
                    }
                } else {
                    ready(Err(actix_web::error::ErrorUnauthorized(
                        "Authorization header invalide",
                    )))
                }
            }
            None => ready(Err(actix_web::error::ErrorUnauthorized(
                "Authorization header manquant",
            ))),
        }
    }
}

