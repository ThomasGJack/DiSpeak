use reqwest::Client;
use sqlx::SqlitePool;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub http_client: Client,
}

impl AppState {
    pub fn new(pool: SqlitePool, http_client: Client) -> Self {
        Self { pool, http_client }
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub fn http_client(&self) -> &Client {
        &self.http_client
    }
}

