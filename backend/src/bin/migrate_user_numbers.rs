use sqlx::Row;
use sqlx::SqlitePool;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔄 Migration des user_numbers...");

    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:backend.db".to_string());

    let pool = SqlitePool::connect(&database_url).await?;

    // Récupérer tous les utilisateurs par ordre de création
    let users = sqlx::query(
        "SELECT id, created_at FROM users WHERE user_number IS NULL ORDER BY created_at ASC",
    )
    .fetch_all(&pool)
    .await?;

    if users.is_empty() {
        println!("✅ Aucun utilisateur à migrer");
        return Ok(());
    }

    println!("📊 {} utilisateurs à migrer", users.len());

    // Attribuer les numéros séquentiels
    for (index, user_row) in users.iter().enumerate() {
        let user_id: String = user_row.get("id");
        let user_number = (index + 1) as i32;

        sqlx::query("UPDATE users SET user_number = ? WHERE id = ?")
            .bind(user_number)
            .bind(&user_id)
            .execute(&pool)
            .await?;

        println!("  ✓ Utilisateur {} → #{:04}", user_id, user_number);
    }

    // Vérifier qu'il n'y a plus de NULL
    let null_count: i32 =
        sqlx::query("SELECT COUNT(*) as count FROM users WHERE user_number IS NULL")
            .fetch_one(&pool)
            .await?
            .get("count");

    if null_count > 0 {
        println!(
            "⚠️  Attention : {} utilisateurs ont encore user_number NULL",
            null_count
        );
        return Err("Migration incomplète".into());
    }

    println!("✅ Migration terminée avec succès !");
    println!("📝 Total: {} utilisateurs numérotés", users.len());

    Ok(())
}
