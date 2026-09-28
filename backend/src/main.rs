use actix_cors::Cors;
use actix_web::{web, App, HttpServer};
use reqwest::Client;
use sqlx::SqlitePool;
use std::env;
use std::io;
use tokio::time::{sleep, Duration};
use tracing_actix_web::TracingLogger;

mod api;
mod app;
mod domain;
mod error;
mod middleware;
mod repositories;
mod routes;
mod services;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    tracing_subscriber::fmt::init();

    let is_prod_env = env::var("BACKEND_ENV")
        .or_else(|_| env::var("APP_ENV"))
        .unwrap_or_else(|_| "development".to_string())
        .eq_ignore_ascii_case("production");
    let allow_insecure_owner_key_fallback = env::var("BACKEND_ALLOW_INSECURE_OWNER_KEY_FALLBACK")
        .ok()
        .map(|v| v.eq_ignore_ascii_case("true"))
        .unwrap_or(!is_prod_env);
    let owner_key_encryption_present = env::var("OWNER_KEY_ENCRYPTION_KEY")
        .ok()
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false);
    if !owner_key_encryption_present && !allow_insecure_owner_key_fallback {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "OWNER_KEY_ENCRYPTION_KEY requis en production (ou activer BACKEND_ALLOW_INSECURE_OWNER_KEY_FALLBACK=true pour le dev)",
        ));
    } else if !owner_key_encryption_present {
        eprintln!("⚠️ OWNER_KEY_ENCRYPTION_KEY absent: fallback de dev activé (non sécurisé)");
    }

    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:backend.db".to_string());
    let bind_host = env::var("BACKEND_BIND_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());

    // Créer le fichier de base de données s'il n'existe pas
    if !std::path::Path::new("backend.db").exists() {
        std::fs::File::create("backend.db")
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to create database file: {e}")))?;
    }

    let pool = SqlitePool::connect(&database_url)
        .await
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to connect to database: {e}")))?;

    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to run migrations: {e}")))?;

    let port = env::var("PORT")
        .unwrap_or_else(|_| "8000".to_string())
        .parse::<u16>()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, format!("Invalid PORT: {e}")))?;

    println!("🚀 DiSpeak Backend démarré sur {}:{}", bind_host, port);

    // Démarrer la tâche de vérification des serveurs
    let pool_clone = pool.clone();
    let healthcheck_timeout_ms = env::var("BACKEND_HEALTHCHECK_TIMEOUT_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(2000);
    let health_http_client = Client::builder()
        .timeout(Duration::from_millis(healthcheck_timeout_ms))
        .build()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to build health HTTP client: {e}")))?;
    tokio::spawn(async move {
        server_health_checker(pool_clone, health_http_client).await;
    });

    let shared_http_client = Client::new();
    let app_state = web::Data::new(app::AppState::new(pool.clone(), shared_http_client.clone()));

    let cors_origins = env::var("BACKEND_CORS_ORIGINS")
        .ok()
        .map(|csv| {
            csv.split(',')
                .map(str::trim)
                .filter(|origin| !origin.is_empty())
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .filter(|origins| !origins.is_empty())
        .unwrap_or_else(|| {
            vec![
                "http://localhost:5173".to_string(),
                "http://127.0.0.1:5173".to_string(),
            ]
        });

    HttpServer::new(move || {
        let mut cors = Cors::default()
            .allow_any_header()
            .allow_any_method();
        for origin in &cors_origins {
            cors = cors.allowed_origin(origin);
        }

        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(shared_http_client.clone()))
            .app_data(app_state.clone())
            .wrap(TracingLogger::default())
            .wrap(cors)
            .service(
                web::scope("/api")
                    .configure(api::auth::config)
                    .configure(api::users::config)
                    .configure(api::licenses::config)
                    .configure(api::servers::config)
                    .configure(api::server_hosting::config)
                    .configure(api::channels::config)
                    .configure(api::roles::config)
                    .configure(api::admin::config),
            )
    })
    .bind(format!("{}:{}", bind_host, port))?
    .run()
    .await
}

/// Tâche périodique pour vérifier la santé des serveurs
async fn server_health_checker(pool: SqlitePool, http_client: Client) {
    println!("🔍 Démarrage du vérificateur de santé des serveurs (toutes les 10s)");
    let configured_internal_api_port = env::var("SERVER_INTERNAL_API_PORT")
        .ok()
        .and_then(|value| value.parse::<i32>().ok());

    loop {
        sleep(Duration::from_secs(10)).await;

        // Récupérer tous les serveurs marqués comme en ligne
        match sqlx::query_as::<_, (String, String, i32)>(
            "SELECT id, actual_host, actual_port FROM servers WHERE is_online = 1 AND actual_host IS NOT NULL AND actual_port IS NOT NULL"
        )
        .fetch_all(&pool)
        .await
        {
            Ok(servers) => {
                for (server_id, host, port) in servers {
                    // Le serveur de santé utilise le port suivant (port + 1)
                    let health_port = configured_internal_api_port.unwrap_or(port + 1);
                    let health_url = format!("http://{}:{}/health", host, health_port);

                    // Vérifier si le serveur répond
                    match http_client.get(&health_url).send().await {
                        Ok(response) => {
                            if response.status().is_success() {
                                // Serveur répond, mettre à jour le heartbeat
                                let _ = sqlx::query(
                                    "UPDATE servers SET last_heartbeat = datetime('now') WHERE id = ?"
                                )
                                .bind(&server_id)
                                .execute(&pool)
                                .await;
                            } else {
                                // Serveur ne répond pas correctement
                                println!("⚠️ Serveur {} ne répond pas correctement (status: {})", server_id, response.status());
                                let _ = sqlx::query(
                                    "UPDATE servers SET is_online = 0 WHERE id = ?"
                                )
                                .bind(&server_id)
                                .execute(&pool)
                                .await;
                            }
                        }
                        Err(err) => {
                            // Serveur inaccessible
                            if err.is_timeout() {
                                println!(
                                    "⏱️ Timeout healthcheck pour le serveur {} sur {}:{}",
                                    server_id, host, port
                                );
                            } else {
                                println!("❌ Serveur {} inaccessible sur {}:{} ({})", server_id, host, port, err);
                            }
                            let _ = sqlx::query(
                                "UPDATE servers SET is_online = 0 WHERE id = ?"
                            )
                            .bind(&server_id)
                            .execute(&pool)
                            .await;
                        }
                    }
                }
            }
            Err(e) => {
                println!("❌ Erreur lors de la vérification des serveurs: {}", e);
            }
        }
    }
}
