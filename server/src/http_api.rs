use axum::{
    extract::{Path, Query, State},
    http::{header::AUTHORIZATION, HeaderMap, StatusCode},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::models::{Category, Channel, ChatMessage, ServerMember, ServerStructure};
use crate::services::{
    fetch_categories_by_server, fetch_channel_history, fetch_channels_by_server,
    fetch_members_by_server, fetch_server_structure, MessageHistory,
};

#[derive(Clone)]
pub struct ApiState {
    pub pool: SqlitePool,
    pub internal_api_token: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChannelHistoryResponse {
    pub messages: Vec<ChatMessage>,
    pub total_count: i64,
    pub has_more: bool,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    pub limit: Option<u32>,
    pub before: Option<String>,
}

pub fn build_router(pool: SqlitePool, internal_api_token: Option<String>) -> Router {
    let state = ApiState {
        pool,
        internal_api_token,
    };

    let api_router = Router::new()
        .route("/servers/:server_id/structure", get(server_structure_handler))
        .route("/servers/:server_id/channels", get(server_channels_handler))
        .route("/servers/:server_id/categories", get(server_categories_handler))
        .route("/servers/:server_id/members", get(server_members_handler))
        .route(
            "/servers/:server_id/channels/:channel_id/messages",
            get(channel_history_handler),
        );

    Router::new()
        .route("/health", get(health))
        .nest("/api", api_router)
        .with_state(state)
}

async fn health() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok" }))
}

async fn server_structure_handler(
    Path(server_id): Path<String>,
    headers: HeaderMap,
    State(state): State<ApiState>,
) -> Result<Json<ServerStructure>, StatusCode> {
    require_internal_api_auth(&headers, &state)?;
    fetch_server_structure(&state.pool, &server_id)
        .await
        .map(Json)
        .map_err(map_sqlx_error)
}

async fn server_channels_handler(
    Path(server_id): Path<String>,
    headers: HeaderMap,
    State(state): State<ApiState>,
) -> Result<Json<Vec<Channel>>, StatusCode> {
    require_internal_api_auth(&headers, &state)?;
    fetch_channels_by_server(&state.pool, &server_id)
        .await
        .map(Json)
        .map_err(map_sqlx_error)
}

async fn server_categories_handler(
    Path(server_id): Path<String>,
    headers: HeaderMap,
    State(state): State<ApiState>,
) -> Result<Json<Vec<Category>>, StatusCode> {
    require_internal_api_auth(&headers, &state)?;
    fetch_categories_by_server(&state.pool, &server_id)
        .await
        .map(Json)
        .map_err(map_sqlx_error)
}

async fn server_members_handler(
    Path(server_id): Path<String>,
    headers: HeaderMap,
    State(state): State<ApiState>,
) -> Result<Json<Vec<ServerMember>>, StatusCode> {
    require_internal_api_auth(&headers, &state)?;
    fetch_members_by_server(&state.pool, &server_id)
        .await
        .map(Json)
        .map_err(map_sqlx_error)
}

async fn channel_history_handler(
    Path((server_id, channel_id)): Path<(String, String)>,
    Query(query): Query<HistoryQuery>,
    headers: HeaderMap,
    State(state): State<ApiState>,
) -> Result<Json<ChannelHistoryResponse>, StatusCode> {
    require_internal_api_auth(&headers, &state)?;
    // Vérifier que le canal appartient bien au serveur demandé
    let belongs_to_server: Option<String> =
        sqlx::query_scalar("SELECT server_id FROM channels WHERE id = ?")
            .bind(&channel_id)
            .fetch_optional(&state.pool)
            .await
            .map_err(map_sqlx_error)?;

    match belongs_to_server {
        Some(owner) if owner == server_id => {
            let limit = query.limit.unwrap_or(100).clamp(1, 200) as i64;
            let history: MessageHistory =
                fetch_channel_history(&state.pool, &channel_id, limit, query.before.as_deref())
                    .await
                    .map_err(map_sqlx_error)?;

            Ok(Json(ChannelHistoryResponse {
                messages: history.messages,
                total_count: history.total_count,
                has_more: history.has_more,
                next_cursor: history.next_cursor,
            }))
        }
        _ => Err(StatusCode::NOT_FOUND),
    }
}

fn map_sqlx_error(error: sqlx::Error) -> StatusCode {
    match error {
        sqlx::Error::RowNotFound => StatusCode::NOT_FOUND,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

fn require_internal_api_auth(headers: &HeaderMap, state: &ApiState) -> Result<(), StatusCode> {
    let expected = state
        .internal_api_token
        .as_deref()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    if expected.trim().is_empty() {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    let Some(auth_value) = headers.get(AUTHORIZATION) else {
        return Err(StatusCode::UNAUTHORIZED);
    };

    let Ok(auth_str) = auth_value.to_str() else {
        return Err(StatusCode::UNAUTHORIZED);
    };

    let Some(token) = auth_str.strip_prefix("Bearer ") else {
        return Err(StatusCode::UNAUTHORIZED);
    };

    if token == expected {
        Ok(())
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}
