use sqlx::{Row, SqlitePool};
use uuid::Uuid;

/// Initialise manuellement une base SQLite et applique les migrations.
///
/// Cette fonction est conservée pour d'éventuels scripts/CLI. L'application
/// principale utilise actuellement `SqlitePool::connect` directement.
#[allow(dead_code)]
pub async fn init_db(database_url: &str) -> Result<SqlitePool, sqlx::Error> {
    let pool = SqlitePool::connect(database_url).await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(pool)
}

/// Obtenir le prochain user_number disponible
/// Cette fonction garantit que chaque utilisateur a un numéro unique et séquentiel
pub async fn get_next_user_number(pool: &SqlitePool) -> Result<i32, sqlx::Error> {
    let mut tx = pool.begin().await?;

    let max_number = sqlx::query("SELECT MAX(user_number) as max_num FROM users")
        .fetch_one(&mut *tx)
        .await?
        .get::<Option<i32>, _>("max_num");

    let next_number = max_number.map(|n| n + 1).unwrap_or(1);

    tx.commit().await?;

    Ok(next_number)
}

/// Formater un user_number en string avec padding
/// Exemples: 1 -> "#0001", 42 -> "#0042", 1000 -> "#1000"
#[allow(dead_code)]
pub fn format_user_number(num: i32) -> String {
    format!("#{:04}", num)
}

/// Enregistrer une entrée dans l'audit log
pub async fn log_audit(
    pool: &SqlitePool,
    user_id: &str,
    action: &str,
    performed_by: Option<&str>,
    details: Option<&str>,
    ip_address: Option<&str>,
) -> Result<(), sqlx::Error> {
    let id = Uuid::new_v4().to_string();

    sqlx::query(
        "INSERT INTO user_audit_log (id, user_id, action, performed_by, details, ip_address, created_at) VALUES (?, ?, ?, ?, ?, ?, datetime('now'))",
    )
    .bind(id)
    .bind(user_id)
    .bind(action)
    .bind(performed_by)
    .bind(details)
    .bind(ip_address)
    .execute(pool)
    .await?;

    Ok(())
}
