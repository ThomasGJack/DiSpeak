use bcrypt::{hash, DEFAULT_COST};
use sqlx::SqlitePool;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 Création d'un compte admin avec licence...");

    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:backend.db".to_string());

    let pool = SqlitePool::connect(&database_url).await?;

    // Créer un admin avec user_number = 1
    let admin_id = Uuid::new_v4().to_string();
    let password_hash = hash("admin123", DEFAULT_COST)?;

    let result = sqlx::query(
        "INSERT INTO users (id, user_number, username, email, provider, password_hash, avatar, status, role, bio, is_active, created_at) 
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, datetime('now'))"
    )
    .bind(&admin_id)
    .bind(1)
    .bind("Admin")
    .bind("admin@dispeak.local")
    .bind("local")
    .bind(&password_hash)
    .bind(None::<String>)
    .bind("online")
    .bind("admin")
    .bind("Compte administrateur")
    .bind(true)
    .execute(&pool)
    .await;

    match result {
        Ok(_) => {
            println!("✅ Compte admin créé !");
            println!("   Email: admin@dispeak.local");
            println!("   Password: admin123");
            println!("   User #: #0001");
            println!("   ID: {}", admin_id);
        }
        Err(e) => {
            println!("⚠️  Erreur (le compte existe peut-être déjà) : {}", e);
        }
    }

    // Créer une licence pour l'admin
    let license_id = Uuid::new_v4().to_string();
    let license_result = sqlx::query(
        "INSERT INTO licenses (id, user_id, server_slots, active, created_at) 
         VALUES (?, ?, ?, ?, datetime('now'))",
    )
    .bind(&license_id)
    .bind(&admin_id)
    .bind(10) // 10 slots de serveur
    .bind(true)
    .execute(&pool)
    .await;

    match license_result {
        Ok(_) => {
            println!("✅ Licence créée (10 slots serveur)");
        }
        Err(e) => {
            println!("⚠️  Erreur création licence : {}", e);
        }
    }

    // Créer un serveur de démonstration
    let server_id = Uuid::new_v4().to_string();
    let server_result = sqlx::query(
        "INSERT INTO servers (id, owner_id, name, host, port, max_users, is_online, created_at) 
         VALUES (?, ?, ?, ?, ?, ?, ?, datetime('now'))",
    )
    .bind(&server_id)
    .bind(&admin_id)
    .bind("Serveur de Démo")
    .bind("127.0.0.1")
    .bind(8001)
    .bind(50)
    .bind(false)
    .execute(&pool)
    .await;

    match server_result {
        Ok(_) => {
            println!("✅ Serveur de démo créé !");
            println!("   Nom: Serveur de Démo");
            println!("   ID: {}", server_id);
            println!();
            println!("📝 Pour configurer le heartbeat dans server/.env :");
            println!("   SERVER_ID={}", server_id);
            println!("   BACKEND_URL=http://localhost:8000");
            println!("   AUTH_TOKEN=<token-obtenu-apres-connexion>");
        }
        Err(e) => {
            println!("⚠️  Erreur création serveur : {}", e);
        }
    }

    println!();
    println!("🎉 Configuration terminée !");
    println!();
    println!("🔑 Pour obtenir ton AUTH_TOKEN :");
    println!("   1. Connecte-toi avec admin@dispeak.local / admin123");
    println!("   2. Ouvre la console (F12) → localStorage.getItem('token')");
    println!("   3. Copie le token dans server/.env");

    Ok(())
}
