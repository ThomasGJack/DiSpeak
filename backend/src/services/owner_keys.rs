use crate::error::AppError;
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use bcrypt::{hash, verify, DEFAULT_COST};
use rand::{rngs::OsRng, RngCore};
use serde::Serialize;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct OwnerKeyInfo {
    pub key: String,
    pub created_at: String,
    pub rotated_at: Option<String>,
    pub last_used_at: Option<String>,
    pub last_used_server_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct VerifiedOwnerKey {
    pub user_id: String,
    pub kid: String,
}

const OWNER_KEY_PREFIX: &str = "dso";

fn parse_owner_key(input: &str) -> Result<(String, String), AppError> {
    let mut parts = input.splitn(3, '_');
    let prefix = parts.next().unwrap_or_default();
    let kid = parts.next().unwrap_or_default();
    let secret = parts.next().unwrap_or_default();

    if prefix != OWNER_KEY_PREFIX || kid.is_empty() || secret.is_empty() {
        return Err(AppError::Unauthorized(
            "Clé propriétaire invalide (format)".to_string(),
        ));
    }

    Ok((kid.to_string(), secret.to_string()))
}

fn format_owner_key(kid: &str, secret: &str) -> String {
    format!("{OWNER_KEY_PREFIX}_{kid}_{secret}")
}

fn random_hex(len_bytes: usize) -> String {
    let mut bytes = vec![0u8; len_bytes];
    OsRng.fill_bytes(&mut bytes);
    hex::encode(bytes)
}

fn random_urlsafe_like(len_bytes: usize) -> String {
    let mut bytes = vec![0u8; len_bytes];
    OsRng.fill_bytes(&mut bytes);
    // Standard base64 then sanitize for copy/paste friendliness.
    B64.encode(bytes)
        .replace('+', "-")
        .replace('/', "_")
        .trim_end_matches('=')
        .to_string()
}

fn owner_key_cipher() -> Result<Aes256Gcm, AppError> {
    let is_prod_env = std::env::var("BACKEND_ENV")
        .or_else(|_| std::env::var("APP_ENV"))
        .unwrap_or_else(|_| "development".to_string())
        .eq_ignore_ascii_case("production");
    let allow_insecure = std::env::var("BACKEND_ALLOW_INSECURE_OWNER_KEY_FALLBACK")
        .ok()
        .map(|v| v.eq_ignore_ascii_case("true"))
        .unwrap_or(!is_prod_env);

    let key_bytes = match std::env::var("OWNER_KEY_ENCRYPTION_KEY") {
        Ok(value) if !value.trim().is_empty() => B64
            .decode(value.trim())
            .map_err(|_| AppError::internal("OWNER_KEY_ENCRYPTION_KEY invalide (base64)"))?,
        _ if allow_insecure => {
            tracing::warn!("OWNER_KEY_ENCRYPTION_KEY absent: fallback de dev non sécurisé utilisé");
            b"0123456789abcdef0123456789abcdef".to_vec()
        }
        _ => {
            return Err(AppError::internal(
                "OWNER_KEY_ENCRYPTION_KEY requis pour gérer les clés propriétaire",
            ))
        }
    };

    if key_bytes.len() != 32 {
        return Err(AppError::internal(
            "OWNER_KEY_ENCRYPTION_KEY doit décoder en 32 octets",
        ));
    }

    Aes256Gcm::new_from_slice(&key_bytes)
        .map_err(|_| AppError::internal("Impossible d'initialiser le chiffrement des clés"))
}

fn encrypt_secret(plain_secret: &str) -> Result<String, AppError> {
    let cipher = owner_key_cipher()?;
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plain_secret.as_bytes())
        .map_err(|_| AppError::internal("Impossible de chiffrer la clé propriétaire"))?;

    let mut payload = nonce_bytes.to_vec();
    payload.extend_from_slice(&ciphertext);
    Ok(B64.encode(payload))
}

fn decrypt_secret(encoded: &str) -> Result<String, AppError> {
    let cipher = owner_key_cipher()?;
    let payload = B64
        .decode(encoded)
        .map_err(|_| AppError::internal("Clé propriétaire chiffrée corrompue"))?;
    if payload.len() < 13 {
        return Err(AppError::internal("Clé propriétaire chiffrée invalide"));
    }
    let (nonce_bytes, ciphertext) = payload.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);
    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| AppError::internal("Impossible de déchiffrer la clé propriétaire"))?;
    String::from_utf8(plaintext)
        .map_err(|_| AppError::internal("Clé propriétaire déchiffrée invalide (UTF-8)"))
}

fn generate_owner_key_material() -> Result<(String, String, String, String), AppError> {
    let kid = random_hex(6);
    let secret = random_urlsafe_like(24);
    let secret_hash = hash(&secret, DEFAULT_COST)
        .map_err(|_| AppError::internal("Impossible de sécuriser la clé propriétaire"))?;
    let encrypted_secret = encrypt_secret(&secret)?;
    Ok((kid, secret, secret_hash, encrypted_secret))
}

pub async fn ensure_user_owner_key(pool: &SqlitePool, user_id: &str) -> Result<OwnerKeyInfo, AppError> {
    if let Some(row) = sqlx::query(
        "SELECT kid, encrypted_secret, created_at, rotated_at, last_used_at, last_used_server_id FROM user_owner_keys WHERE user_id = ? AND is_active = 1"
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?
    {
        let kid: String = row.get("kid");
        let encrypted_secret: String = row.get("encrypted_secret");
        let secret = decrypt_secret(&encrypted_secret)?;
        return Ok(OwnerKeyInfo {
            key: format_owner_key(&kid, &secret),
            created_at: row.get("created_at"),
            rotated_at: row.get("rotated_at"),
            last_used_at: row.get("last_used_at"),
            last_used_server_id: row.get("last_used_server_id"),
        });
    }

    let (kid, secret, secret_hash, encrypted_secret) = generate_owner_key_material()?;
    let id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO user_owner_keys (id, user_id, kid, secret_hash, encrypted_secret, is_active, created_at) VALUES (?, ?, ?, ?, ?, 1, datetime('now'))"
    )
    .bind(id)
    .bind(user_id)
    .bind(&kid)
    .bind(secret_hash)
    .bind(encrypted_secret)
    .execute(pool)
    .await?;

    let row = sqlx::query(
        "SELECT created_at, rotated_at, last_used_at, last_used_server_id FROM user_owner_keys WHERE user_id = ?"
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    Ok(OwnerKeyInfo {
        key: format_owner_key(&kid, &secret),
        created_at: row.get("created_at"),
        rotated_at: row.get("rotated_at"),
        last_used_at: row.get("last_used_at"),
        last_used_server_id: row.get("last_used_server_id"),
    })
}

pub async fn rotate_user_owner_key(pool: &SqlitePool, user_id: &str) -> Result<OwnerKeyInfo, AppError> {
    // Ensure row exists first (legacy accounts).
    let _ = ensure_user_owner_key(pool, user_id).await?;

    let (kid, secret, secret_hash, encrypted_secret) = generate_owner_key_material()?;

    sqlx::query(
        "UPDATE user_owner_keys SET kid = ?, secret_hash = ?, encrypted_secret = ?, rotated_at = datetime('now'), last_used_at = NULL, last_used_ip = NULL, last_used_server_id = NULL, is_active = 1 WHERE user_id = ?"
    )
    .bind(&kid)
    .bind(secret_hash)
    .bind(encrypted_secret)
    .bind(user_id)
    .execute(pool)
    .await?;

    let row = sqlx::query(
        "SELECT created_at, rotated_at, last_used_at, last_used_server_id FROM user_owner_keys WHERE user_id = ?"
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    Ok(OwnerKeyInfo {
        key: format_owner_key(&kid, &secret),
        created_at: row.get("created_at"),
        rotated_at: row.get("rotated_at"),
        last_used_at: row.get("last_used_at"),
        last_used_server_id: row.get("last_used_server_id"),
    })
}

pub async fn verify_owner_key(
    pool: &SqlitePool,
    owner_key: &str,
    usage_server_id: Option<&str>,
    usage_ip: Option<&str>,
) -> Result<VerifiedOwnerKey, AppError> {
    let (kid, secret) = parse_owner_key(owner_key)?;

    let row = sqlx::query(
        "SELECT user_id, secret_hash FROM user_owner_keys WHERE kid = ? AND is_active = 1"
    )
    .bind(&kid)
    .fetch_optional(pool)
    .await?;

    let row = row.ok_or_else(|| AppError::Unauthorized("Clé propriétaire invalide".to_string()))?;
    let user_id: String = row.get("user_id");
    let secret_hash: String = row.get("secret_hash");

    let is_valid = verify(&secret, &secret_hash)
        .map_err(|_| AppError::internal("Impossible de vérifier la clé propriétaire"))?;
    if !is_valid {
        return Err(AppError::Unauthorized("Clé propriétaire invalide".to_string()));
    }

    sqlx::query(
        "UPDATE user_owner_keys SET last_used_at = datetime('now'), last_used_ip = ?, last_used_server_id = ? WHERE kid = ?"
    )
    .bind(usage_ip)
    .bind(usage_server_id)
    .bind(&kid)
    .execute(pool)
    .await?;

    Ok(VerifiedOwnerKey { user_id, kid })
}
