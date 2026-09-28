use sqlx::SqlitePool;

/// Script pour marquer comme offline les serveurs qui n'ont pas envoyé de heartbeat depuis 60 secondes
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔄 Nettoyage des serveurs hors ligne...");

    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:backend.db".to_string());

    let pool = SqlitePool::connect(&database_url).await?;

    // Marquer comme offline les serveurs sans heartbeat depuis 60 secondes
    let result = sqlx::query(
        "UPDATE servers SET is_online = 0 WHERE is_online = 1 AND (last_heartbeat IS NULL OR datetime(last_heartbeat) < datetime('now', '-60 seconds'))"
    )
    .execute(&pool)
    .await?;

    let count = result.rows_affected();

    if count > 0 {
        println!("✅ {} serveur(s) marqué(s) comme hors ligne", count);
    } else {
        println!("✅ Tous les serveurs sont à jour");
    }

    Ok(())
}
