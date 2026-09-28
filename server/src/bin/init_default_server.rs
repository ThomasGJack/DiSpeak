use dotenv::dotenv;
use sqlx::SqlitePool;
use std::env;
use uuid::Uuid;

// Script pour initialiser un serveur par défaut avec des canaux Discord-like
async fn initialize_default_server(pool: &SqlitePool, server_id: &str) -> Result<(), sqlx::Error> {
    println!("🚀 Initialisation du serveur par défaut: {}", server_id);

    // 1. Créer une catégorie "Général"
    let general_category_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO categories (id, server_id, name, position, created_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&general_category_id)
    .bind(server_id)
    .bind("Général")
    .bind(0)
    .bind(&now)
    .execute(pool)
    .await?;

    println!("✅ Catégorie 'Général' créée");

    // 2. Créer des canaux texte par défaut
    let channels = vec![
        ("général", "Canal général pour discuter", "text"),
        ("annonces", "Annonces importantes du serveur", "text"),
        ("règles", "Règles et informations du serveur", "text"),
    ];

    for (i, (name, description, channel_type)) in channels.iter().enumerate() {
        let channel_id = Uuid::new_v4().to_string();

        sqlx::query(
            "INSERT INTO channels (id, server_id, name, type, category_id, position, description, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
        )
            .bind(&channel_id)
            .bind(server_id)
            .bind(name)
            .bind(channel_type)
            .bind(&general_category_id)
            .bind(i as i32)
            .bind(description)
            .bind(&now)
            .execute(pool)
            .await?;

        println!("✅ Canal '{}' créé", name);
    }

    // 3. Créer une catégorie "Vocal"
    let voice_category_id = Uuid::new_v4().to_string();

    sqlx::query(
        "INSERT INTO categories (id, server_id, name, position, created_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&voice_category_id)
    .bind(server_id)
    .bind("Vocal")
    .bind(1)
    .bind(&now)
    .execute(pool)
    .await?;

    println!("✅ Catégorie 'Vocal' créée");

    // 4. Créer des canaux vocaux par défaut
    let voice_channels = vec![
        ("Général", "Canal vocal général"),
        ("Gaming", "Canal pour jouer ensemble"),
        ("Musique", "Canal pour écouter de la musique"),
    ];

    for (i, (name, description)) in voice_channels.iter().enumerate() {
        let channel_id = Uuid::new_v4().to_string();

        sqlx::query(
            "INSERT INTO channels (id, server_id, name, type, category_id, position, description, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
        )
            .bind(&channel_id)
            .bind(server_id)
            .bind(name)
            .bind("voice")
            .bind(&voice_category_id)
            .bind(i as i32)
            .bind(description)
            .bind(&now)
            .execute(pool)
            .await?;

        println!("✅ Canal vocal '{}' créé", name);
    }

    println!("🎉 Serveur par défaut initialisé avec succès !");
    println!("📋 Structure créée :");
    println!("   📁 Général");
    for (name, _, _) in channels.iter() {
        println!("     💬 {}", name);
    }
    println!("   📁 Vocal");
    for (name, _) in voice_channels.iter() {
        println!("     🎤 {}", name);
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    println!("🚀 Script d'initialisation du serveur par défaut");

    // Connexion à la base de données
    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:server.db".to_string());

    let pool = SqlitePool::connect(&database_url).await?;

    // Exécuter les migrations
    sqlx::migrate!("./migrations").run(&pool).await?;
    println!("✅ Migrations exécutées");

    // Demander l'ID du serveur à initialiser
    println!("Entrez l'ID du serveur à initialiser (ou appuyez sur Entrée pour utiliser un ID par défaut):");

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    let server_id = if input.trim().is_empty() {
        "default-server-123".to_string()
    } else {
        input.trim().to_string()
    };

    // Vérifier si le serveur existe (en vérifiant s'il y a des canaux)
    let server_exists =
        sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM channels WHERE server_id = ?)")
            .bind(&server_id)
            .fetch_one(&pool)
            .await?;

    if server_exists {
        println!("⚠️  Le serveur {} a déjà des canaux.", server_id);
        println!("Voulez-vous continuer et ajouter les canaux par défaut ? (y/N)");

        let mut confirm = String::new();
        std::io::stdin().read_line(&mut confirm)?;

        if !confirm.trim().to_lowercase().starts_with('y') {
            println!("❌ Initialisation annulée.");
            return Ok(());
        }
    }

    // Initialiser le serveur par défaut
    initialize_default_server(&pool, &server_id).await?;

    println!("✅ Script terminé avec succès !");
    println!(
        "Vous pouvez maintenant rejoindre le serveur {} dans l'interface.",
        server_id
    );

    Ok(())
}
