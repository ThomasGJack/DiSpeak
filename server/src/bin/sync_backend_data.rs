use dotenv::dotenv;
use sqlx::{Row, SqlitePool};
use std::env;

// Script pour synchroniser les données du backend vers le serveur WebSocket
async fn sync_backend_to_server(
    backend_pool: &SqlitePool,
    server_pool: &SqlitePool,
) -> Result<(), sqlx::Error> {
    println!("🔄 Synchronisation des données backend -> serveur WebSocket");

    // 1. Synchroniser les serveurs
    let servers = sqlx::query(
        "SELECT id, owner_id, name, host, port, max_users, is_online, last_heartbeat, actual_host, actual_port, icon, color, description, created_at FROM servers"
    )
    .fetch_all(backend_pool)
    .await?;

    println!("📊 Trouvé {} serveurs dans le backend", servers.len());

    for server in servers {
        let server_id: String = server.get("id");
        let owner_id: String = server.get("owner_id");
        let name: String = server.get("name");
        let host: String = server.get("host");
        let port: i32 = server.get("port");
        let max_users: i32 = server.get("max_users");
        let is_online: bool = server.get("is_online");
        let last_heartbeat: Option<String> = server.get("last_heartbeat");
        let actual_host: Option<String> = server.get("actual_host");
        let actual_port: Option<i32> = server.get("actual_port");
        let icon: Option<String> = server.get("icon");
        let color: Option<String> = server.get("color");
        let description: Option<String> = server.get("description");
        let created_at: String = server.get("created_at");

        // Insérer ou mettre à jour le serveur dans la base du serveur WebSocket
        sqlx::query(
            "INSERT OR REPLACE INTO servers (id, owner_id, name, host, port, max_users, is_online, last_heartbeat, actual_host, actual_port, icon, color, description, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&server_id)
        .bind(&owner_id)
        .bind(&name)
        .bind(&host)
        .bind(port)
        .bind(max_users)
        .bind(is_online)
        .bind(&last_heartbeat)
        .bind(&actual_host)
        .bind(actual_port)
        .bind(&icon)
        .bind(&color)
        .bind(&description)
        .bind(&created_at)
        .execute(server_pool)
        .await?;

        println!("✅ Serveur '{}' synchronisé", name);
    }

    // 2. Synchroniser les membres des serveurs
    let members = sqlx::query(
        "SELECT id, server_id, user_id, username, avatar, role, joined_at, last_seen, is_online FROM server_members"
    )
    .fetch_all(backend_pool)
    .await?;

    for member in members {
        let member_id: String = member.get("id");
        let server_id: String = member.get("server_id");
        let user_id: String = member.get("user_id");
        let username: String = member.get("username");
        let avatar: Option<String> = member.get("avatar");
        let role: String = member.get("role");
        let joined_at: String = member.get("joined_at");
        let last_seen: String = member.get("last_seen");
        let is_online: bool = member.get("is_online");

        // Insérer ou mettre à jour le membre dans la base du serveur WebSocket
        sqlx::query(
            "INSERT OR REPLACE INTO server_members (id, server_id, user_id, username, avatar, role, joined_at, last_seen, is_online) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&member_id)
        .bind(&server_id)
        .bind(&user_id)
        .bind(&username)
        .bind(&avatar)
        .bind(&role)
        .bind(&joined_at)
        .bind(&last_seen)
        .bind(is_online)
        .execute(server_pool)
        .await?;

        println!("✅ Membre '{}' synchronisé", username);
    }

    // 3. Synchroniser les canaux
    let channels = sqlx::query(
        "SELECT id, server_id, name, type, category_id, position, description, created_at FROM channels"
    )
    .fetch_all(backend_pool)
    .await?;

    for channel in channels {
        let channel_id: String = channel.get("id");
        let server_id: String = channel.get("server_id");
        let name: String = channel.get("name");
        let channel_type: String = channel.get("type");
        let category_id: Option<String> = channel.get("category_id");
        let position: i32 = channel.get("position");
        let description: Option<String> = channel.get("description");
        let created_at: String = channel.get("created_at");

        // Insérer ou mettre à jour le canal dans la base du serveur WebSocket
        sqlx::query(
            "INSERT OR REPLACE INTO channels (id, server_id, name, type, category_id, position, description, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&channel_id)
        .bind(&server_id)
        .bind(&name)
        .bind(&channel_type)
        .bind(&category_id)
        .bind(position)
        .bind(&description)
        .bind(&created_at)
        .execute(server_pool)
        .await?;

        println!("✅ Canal '{}' synchronisé", name);
    }

    // 4. Synchroniser les catégories
    let categories =
        sqlx::query("SELECT id, server_id, name, position, created_at FROM categories")
            .fetch_all(backend_pool)
            .await?;

    for category in categories {
        let category_id: String = category.get("id");
        let server_id: String = category.get("server_id");
        let name: String = category.get("name");
        let position: i32 = category.get("position");
        let created_at: String = category.get("created_at");

        // Insérer ou mettre à jour la catégorie dans la base du serveur WebSocket
        sqlx::query(
            "INSERT OR REPLACE INTO categories (id, server_id, name, position, created_at) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(&category_id)
        .bind(&server_id)
        .bind(&name)
        .bind(position)
        .bind(&created_at)
        .execute(server_pool)
        .await?;

        println!("✅ Catégorie '{}' synchronisée", name);
    }

    println!("🎉 Synchronisation terminée avec succès !");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    println!("🔄 Script de synchronisation backend -> serveur WebSocket");

    // Connexion aux bases de données
    let backend_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite:E:/Projet/DiSpeak/backend/backend.db".to_string());

    let server_url = "sqlite:server.db";

    println!("🔗 Connexion au backend: {}", backend_url);
    let backend_pool = SqlitePool::connect(&backend_url).await?;
    println!("✅ Connexion backend réussie");

    let server_pool = SqlitePool::connect(server_url).await?;
    println!("✅ Connexion serveur réussie");

    // Exécuter les migrations du serveur
    sqlx::migrate!("./migrations").run(&server_pool).await?;
    println!("✅ Migrations du serveur exécutées");

    // Synchroniser les données
    sync_backend_to_server(&backend_pool, &server_pool).await?;

    println!("✅ Script terminé avec succès !");
    println!("💡 Les propriétaires peuvent maintenant rejoindre leurs serveurs");

    Ok(())
}
