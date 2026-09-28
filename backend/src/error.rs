use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use serde::Serialize;
use std::fmt::Debug;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{0}")]
    BadRequest(String),
    #[error("{0}")]
    Unauthorized(String),
    #[error("{0}")]
    Forbidden(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Conflict(String),
    #[error("{0}")]
    Internal(String),
}

impl AppError {
    pub fn code(&self) -> &'static str {
        match self {
            AppError::BadRequest(_) => "bad_request",
            AppError::Unauthorized(_) => "unauthorized",
            AppError::Forbidden(_) => "forbidden",
            AppError::NotFound(_) => "not_found",
            AppError::Conflict(_) => "conflict",
            AppError::Internal(_) => "internal_error",
        }
    }

    pub fn message(&self) -> &str {
        match self {
            AppError::BadRequest(msg)
            | AppError::Unauthorized(msg)
            | AppError::Forbidden(msg)
            | AppError::NotFound(msg)
            | AppError::Conflict(msg)
            | AppError::Internal(msg) => msg.as_str(),
        }
    }

    fn http_status(&self) -> StatusCode {
        match self {
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            AppError::Forbidden(_) => StatusCode::FORBIDDEN,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Conflict(_) => StatusCode::CONFLICT,
            AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[derive(Debug, Serialize)]
struct ErrorBody<'a> {
    code: &'a str,
    message: &'a str,
}

impl ResponseError for AppError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.http_status()).json(ErrorBody {
            code: self.code(),
            message: self.message(),
        })
    }

    fn status_code(&self) -> StatusCode {
        self.http_status()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(error: sqlx::Error) -> Self {
        match error {
            sqlx::Error::RowNotFound => AppError::NotFound("Resource introuvable".to_string()),
            other => {
                tracing::error!(error = ?other, "Erreur SQL");
                AppError::Internal("Erreur base de données".to_string())
            }
        }
    }
}

impl From<reqwest::Error> for AppError {
    fn from(error: reqwest::Error) -> Self {
        tracing::error!(error = ?error, "Erreur HTTP externe");
        AppError::Internal("Erreur de communication externe".to_string())
    }
}

impl From<anyhow::Error> for AppError {
    fn from(error: anyhow::Error) -> Self {
        tracing::error!(error = ?error, "Erreur applicative");
        AppError::Internal("Erreur interne".to_string())
    }
}

impl AppError {
    pub fn internal<T: Into<String>>(message: T) -> Self {
        AppError::Internal(message.into())
    }

    pub fn bad_request<T: Into<String>>(message: T) -> Self {
        AppError::BadRequest(message.into())
    }
}
