use crate::models::{
    Category, CategoryWithChannels, Channel, ChatMessage, Server, ServerMember, ServerStructure,
};
use sqlx::SqlitePool;

#[derive(Debug, Clone)]
pub struct MessageHistory {
    pub messages: Vec<ChatMessage>,
    pub total_count: i64,
    pub has_more: bool,
    pub next_cursor: Option<String>,
}

pub async fn fetch_server(pool: &SqlitePool, server_id: &str) -> Result<Server, sqlx::Error> {
    sqlx::query_as::<_, Server>(
        "SELECT id, owner_id, name, host, port, max_users, is_online, last_heartbeat, actual_host, actual_port, icon, color, description, created_at FROM servers WHERE id = ?",
    )
    .bind(server_id)
    .fetch_one(pool)
    .await
}

pub async fn fetch_categories_by_server(
    pool: &SqlitePool,
    server_id: &str,
) -> Result<Vec<Category>, sqlx::Error> {
    sqlx::query_as::<_, Category>(
        "SELECT id, server_id, name, position, created_at FROM categories WHERE server_id = ? ORDER BY position ASC, created_at ASC",
    )
    .bind(server_id)
    .fetch_all(pool)
    .await
}

pub async fn fetch_channels_by_server(
    pool: &SqlitePool,
    server_id: &str,
) -> Result<Vec<Channel>, sqlx::Error> {
    sqlx::query_as::<_, Channel>(
        "SELECT id, server_id, name, type, category_id, position, description, created_at FROM channels WHERE server_id = ? ORDER BY position ASC, created_at ASC",
    )
    .bind(server_id)
    .fetch_all(pool)
    .await
}

pub async fn fetch_members_by_server(
    pool: &SqlitePool,
    server_id: &str,
) -> Result<Vec<ServerMember>, sqlx::Error> {
    sqlx::query_as::<_, ServerMember>(
        "SELECT id, server_id, user_id, username, avatar, role, joined_at, last_seen, is_online FROM server_members WHERE server_id = ? ORDER BY (role = 'owner') DESC, (role = 'admin') DESC, is_online DESC, username COLLATE NOCASE ASC",
    )
    .bind(server_id)
    .fetch_all(pool)
    .await
}

pub async fn fetch_server_structure(
    pool: &SqlitePool,
    server_id: &str,
) -> Result<ServerStructure, sqlx::Error> {
    let server = fetch_server(pool, server_id).await?;
    let categories = fetch_categories_by_server(pool, server_id).await?;
    let channels = fetch_channels_by_server(pool, server_id).await?;
    let members = fetch_members_by_server(pool, server_id).await?;

    let mut categories_with_channels = Vec::with_capacity(categories.len());

    for category in categories.into_iter() {
        let category_channels: Vec<Channel> = channels
            .iter()
            .filter(|channel| channel.category_id.as_deref() == Some(&category.id))
            .cloned()
            .collect();

        categories_with_channels.push(CategoryWithChannels {
            category,
            channels: category_channels,
        });
    }

    let uncategorized_channels: Vec<Channel> = channels
        .iter()
        .filter(|channel| channel.category_id.is_none())
        .cloned()
        .collect();

    Ok(ServerStructure {
        server,
        categories: categories_with_channels,
        uncategorized_channels,
        members,
    })
}

pub async fn fetch_channel_history(
    pool: &SqlitePool,
    channel_id: &str,
    limit: i64,
    before_message_id: Option<&str>,
) -> Result<MessageHistory, sqlx::Error> {
    let capped_limit = limit.clamp(1, 200);

    let messages = if let Some(before_id) = before_message_id {
        sqlx::query_as::<_, ChatMessage>(
            "SELECT id, channel_id, user_id, username, content, created_at, edited_at, reply_to_message_id, attachments, reactions, is_pinned, is_system FROM messages WHERE channel_id = ? AND created_at < (SELECT created_at FROM messages WHERE id = ?) ORDER BY created_at DESC LIMIT ?",
        )
        .bind(channel_id)
        .bind(before_id)
        .bind(capped_limit)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, ChatMessage>(
            "SELECT id, channel_id, user_id, username, content, created_at, edited_at, reply_to_message_id, attachments, reactions, is_pinned, is_system FROM messages WHERE channel_id = ? ORDER BY created_at DESC LIMIT ?",
        )
        .bind(channel_id)
        .bind(capped_limit)
        .fetch_all(pool)
        .await?
    };

    let total_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM messages WHERE channel_id = ?")
        .bind(channel_id)
        .fetch_one(pool)
        .await?;

    let has_more = (messages.len() as i64) == capped_limit && total_count > capped_limit;
    let next_cursor = if has_more {
        messages.last().map(|message| message.id.clone())
    } else {
        None
    };

    Ok(MessageHistory {
        messages,
        total_count,
        has_more,
        next_cursor,
    })
}
