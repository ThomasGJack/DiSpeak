use crate::models::{
    ADMINISTRATOR, CONNECT_VOICE, MANAGE_CHANNELS, MANAGE_MESSAGES, MANAGE_ROLES, SEND_MESSAGES,
    SPEAK_VOICE, VIEW_CHANNELS,
};
use sqlx::{Row, SqlitePool};

/// Structure pour représenter les permissions effectives d'un utilisateur
#[derive(Debug, Clone)]
pub struct EffectivePermissions {
    pub allow_bitfield: i32,
    pub deny_bitfield: i32,
    pub final_permissions: i32,
}

/// Récupère toutes les permissions d'un utilisateur sur un serveur (combinaison de tous ses rôles)
pub async fn get_user_permissions(
    pool: &SqlitePool,
    user_id: &str,
    server_id: &str,
) -> Result<i32, sqlx::Error> {
    // Vérifier si l'utilisateur est propriétaire du serveur
    let owner_check = sqlx::query(
        "SELECT 1 FROM server_members WHERE server_id = ? AND user_id = ? AND role = 'owner'",
    )
    .bind(server_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    if owner_check.is_some() {
        return Ok(ADMINISTRATOR); // Le propriétaire a toutes les permissions
    }

    // Récupérer toutes les permissions des rôles de l'utilisateur
    let permissions_rows = sqlx::query(
        "SELECT sr.permissions_bitfield 
         FROM server_roles sr
         JOIN member_roles mr ON sr.id = mr.role_id
         JOIN server_members sm ON mr.member_id = sm.id
         WHERE sm.user_id = ? AND sm.server_id = ?",
    )
    .bind(user_id)
    .bind(server_id)
    .fetch_all(pool)
    .await?;

    // Combiner toutes les permissions (OR logique)
    let mut combined_permissions = 0;
    for row in permissions_rows {
        let permissions: i32 = row.get("permissions_bitfield");
        combined_permissions |= permissions;
    }

    Ok(combined_permissions)
}

/// Calcule les permissions effectives d'un utilisateur pour un canal spécifique
/// Prend en compte les rôles + overrides de catégorie + overrides de canal + overrides individuels
pub async fn get_effective_channel_permissions(
    pool: &SqlitePool,
    user_id: &str,
    channel_id: &str,
) -> Result<EffectivePermissions, sqlx::Error> {
    // Récupérer les informations du canal et de sa catégorie
    let channel_info = sqlx::query(
        "SELECT c.server_id, c.category_id, c.type
         FROM channels c 
         WHERE c.id = ?",
    )
    .bind(channel_id)
    .fetch_optional(pool)
    .await?;

    let (server_id, category_id, _channel_type) = match channel_info {
        Some(row) => (
            row.get::<String, _>("server_id"),
            row.get::<Option<String>, _>("category_id"),
            row.get::<String, _>("type"),
        ),
        None => return Err(sqlx::Error::RowNotFound),
    };

    // Vérifier si l'utilisateur est propriétaire du serveur
    let owner_check = sqlx::query(
        "SELECT 1 FROM server_members WHERE server_id = ? AND user_id = ? AND role = 'owner'",
    )
    .bind(&server_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    if owner_check.is_some() {
        return Ok(EffectivePermissions {
            allow_bitfield: ADMINISTRATOR,
            deny_bitfield: 0,
            final_permissions: ADMINISTRATOR,
        });
    }

    // Récupérer les permissions de base des rôles de l'utilisateur
    let base_permissions = get_user_permissions(pool, user_id, &server_id).await?;
    let mut allow_bitfield = base_permissions;
    let mut deny_bitfield = 0;

    // Récupérer l'ID du membre pour les overrides individuels
    let member_id = sqlx::query_scalar::<_, Option<String>>(
        "SELECT id FROM server_members WHERE server_id = ? AND user_id = ?",
    )
    .bind(&server_id)
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    // Appliquer les overrides de catégorie si le canal en a une
    if let Some(ref cat_id) = category_id {
        // Overrides de rôles pour la catégorie
        let category_role_overrides = sqlx::query(
            "SELECT cro.allow_bitfield, cro.deny_bitfield
             FROM category_role_overrides cro
             JOIN member_roles mr ON cro.role_id = mr.role_id
             WHERE cro.category_id = ? AND mr.member_id = ?",
        )
        .bind(cat_id)
        .bind(&member_id)
        .fetch_all(pool)
        .await?;

        for row in category_role_overrides {
            let allow: i32 = row.get("allow_bitfield");
            let deny: i32 = row.get("deny_bitfield");
            allow_bitfield |= allow;
            deny_bitfield |= deny;
        }

        // Overrides individuels pour la catégorie
        if let Some(ref mem_id) = member_id {
            let member_category_override = sqlx::query(
                "SELECT allow_bitfield, deny_bitfield
                 FROM member_category_overrides
                 WHERE category_id = ? AND member_id = ?",
            )
            .bind(cat_id)
            .bind(mem_id)
            .fetch_optional(pool)
            .await?;

            if let Some(row) = member_category_override {
                let allow: i32 = row.get("allow_bitfield");
                let deny: i32 = row.get("deny_bitfield");
                allow_bitfield |= allow;
                deny_bitfield |= deny;
            }
        }
    }

    // Appliquer les overrides de canal
    // Overrides de rôles pour le canal
    let channel_role_overrides = sqlx::query(
        "SELECT cro.allow_bitfield, cro.deny_bitfield
         FROM channel_role_overrides cro
         JOIN member_roles mr ON cro.role_id = mr.role_id
         WHERE cro.channel_id = ? AND mr.member_id = ?",
    )
    .bind(channel_id)
    .bind(&member_id)
    .fetch_all(pool)
    .await?;

    for row in channel_role_overrides {
        let allow: i32 = row.get("allow_bitfield");
        let deny: i32 = row.get("deny_bitfield");
        allow_bitfield |= allow;
        deny_bitfield |= deny;
    }

    // Overrides individuels pour le canal
    if let Some(ref mem_id) = member_id {
        let member_channel_override = sqlx::query(
            "SELECT allow_bitfield, deny_bitfield
             FROM member_channel_overrides
             WHERE channel_id = ? AND member_id = ?",
        )
        .bind(channel_id)
        .bind(mem_id)
        .fetch_optional(pool)
        .await?;

        if let Some(row) = member_channel_override {
            let allow: i32 = row.get("allow_bitfield");
            let deny: i32 = row.get("deny_bitfield");
            allow_bitfield |= allow;
            deny_bitfield |= deny;
        }
    }

    // Calculer les permissions finales : allow - deny
    let final_permissions = (allow_bitfield & !deny_bitfield) | (allow_bitfield & deny_bitfield);

    Ok(EffectivePermissions {
        allow_bitfield,
        deny_bitfield,
        final_permissions,
    })
}

/// Calcule les permissions effectives d'un utilisateur pour une catégorie spécifique
pub async fn get_effective_category_permissions(
    pool: &SqlitePool,
    user_id: &str,
    category_id: &str,
) -> Result<EffectivePermissions, sqlx::Error> {
    // Récupérer le serveur de la catégorie
    let server_id =
        sqlx::query_scalar::<_, String>("SELECT server_id FROM categories WHERE id = ?")
            .bind(category_id)
            .fetch_one(pool)
            .await?;

    // Vérifier si l'utilisateur est propriétaire du serveur
    let owner_check = sqlx::query(
        "SELECT 1 FROM server_members WHERE server_id = ? AND user_id = ? AND role = 'owner'",
    )
    .bind(&server_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    if owner_check.is_some() {
        return Ok(EffectivePermissions {
            allow_bitfield: ADMINISTRATOR,
            deny_bitfield: 0,
            final_permissions: ADMINISTRATOR,
        });
    }

    // Récupérer les permissions de base des rôles de l'utilisateur
    let base_permissions = get_user_permissions(pool, user_id, &server_id).await?;
    let mut allow_bitfield = base_permissions;
    let mut deny_bitfield = 0;

    // Récupérer l'ID du membre pour les overrides individuels
    let member_id = sqlx::query_scalar::<_, Option<String>>(
        "SELECT id FROM server_members WHERE server_id = ? AND user_id = ?",
    )
    .bind(&server_id)
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    // Appliquer les overrides de catégorie
    // Overrides de rôles pour la catégorie
    let category_role_overrides = sqlx::query(
        "SELECT cro.allow_bitfield, cro.deny_bitfield
         FROM category_role_overrides cro
         JOIN member_roles mr ON cro.role_id = mr.role_id
         WHERE cro.category_id = ? AND mr.member_id = ?",
    )
    .bind(category_id)
    .bind(&member_id)
    .fetch_all(pool)
    .await?;

    for row in category_role_overrides {
        let allow: i32 = row.get("allow_bitfield");
        let deny: i32 = row.get("deny_bitfield");
        allow_bitfield |= allow;
        deny_bitfield |= deny;
    }

    // Overrides individuels pour la catégorie
    if let Some(ref mem_id) = member_id {
        let member_category_override = sqlx::query(
            "SELECT allow_bitfield, deny_bitfield
             FROM member_category_overrides
             WHERE category_id = ? AND member_id = ?",
        )
        .bind(category_id)
        .bind(mem_id)
        .fetch_optional(pool)
        .await?;

        if let Some(row) = member_category_override {
            let allow: i32 = row.get("allow_bitfield");
            let deny: i32 = row.get("deny_bitfield");
            allow_bitfield |= allow;
            deny_bitfield |= deny;
        }
    }

    // Calculer les permissions finales : allow - deny
    let final_permissions = (allow_bitfield & !deny_bitfield) | (allow_bitfield & deny_bitfield);

    Ok(EffectivePermissions {
        allow_bitfield,
        deny_bitfield,
        final_permissions,
    })
}

/// Vérifie si un utilisateur a une permission spécifique sur un serveur
pub async fn check_permission(
    pool: &SqlitePool,
    user_id: &str,
    server_id: &str,
    permission_flag: i32,
) -> Result<bool, sqlx::Error> {
    let user_permissions = get_user_permissions(pool, user_id, server_id).await?;
    Ok((user_permissions & permission_flag) != 0 || (user_permissions & ADMINISTRATOR) != 0)
}

/// Vérifie si un utilisateur peut gérer les canaux d'un serveur
pub async fn can_manage_channels(
    pool: &SqlitePool,
    user_id: &str,
    server_id: &str,
) -> Result<bool, sqlx::Error> {
    check_permission(pool, user_id, server_id, MANAGE_CHANNELS).await
}

/// Vérifie si un utilisateur peut gérer les rôles d'un serveur
pub async fn can_manage_roles(
    pool: &SqlitePool,
    user_id: &str,
    server_id: &str,
) -> Result<bool, sqlx::Error> {
    check_permission(pool, user_id, server_id, MANAGE_ROLES).await
}

/// Vérifie si un utilisateur peut voir un canal
pub async fn can_view_channel(
    pool: &SqlitePool,
    user_id: &str,
    channel_id: &str,
) -> Result<bool, sqlx::Error> {
    let effective_permissions =
        get_effective_channel_permissions(pool, user_id, channel_id).await?;
    Ok(
        (effective_permissions.final_permissions & VIEW_CHANNELS) != 0
            || (effective_permissions.final_permissions & ADMINISTRATOR) != 0,
    )
}

/// Vérifie si un utilisateur peut envoyer des messages dans un canal
pub async fn can_send_messages(
    pool: &SqlitePool,
    user_id: &str,
    channel_id: &str,
) -> Result<bool, sqlx::Error> {
    let effective_permissions =
        get_effective_channel_permissions(pool, user_id, channel_id).await?;
    Ok(
        (effective_permissions.final_permissions & SEND_MESSAGES) != 0
            || (effective_permissions.final_permissions & ADMINISTRATOR) != 0,
    )
}

/// Vérifie si un utilisateur peut gérer les messages d'un canal
pub async fn can_manage_messages(
    pool: &SqlitePool,
    user_id: &str,
    channel_id: &str,
) -> Result<bool, sqlx::Error> {
    let effective_permissions =
        get_effective_channel_permissions(pool, user_id, channel_id).await?;
    Ok(
        (effective_permissions.final_permissions & MANAGE_MESSAGES) != 0
            || (effective_permissions.final_permissions & ADMINISTRATOR) != 0,
    )
}

/// Vérifie si un utilisateur peut se connecter à un salon vocal
pub async fn can_connect_voice(
    pool: &SqlitePool,
    user_id: &str,
    channel_id: &str,
) -> Result<bool, sqlx::Error> {
    let effective_permissions =
        get_effective_channel_permissions(pool, user_id, channel_id).await?;
    Ok(
        (effective_permissions.final_permissions & CONNECT_VOICE) != 0
            || (effective_permissions.final_permissions & ADMINISTRATOR) != 0,
    )
}

/// Vérifie si un utilisateur peut parler dans un salon vocal
pub async fn can_speak_voice(
    pool: &SqlitePool,
    user_id: &str,
    channel_id: &str,
) -> Result<bool, sqlx::Error> {
    let effective_permissions =
        get_effective_channel_permissions(pool, user_id, channel_id).await?;
    Ok((effective_permissions.final_permissions & SPEAK_VOICE) != 0
        || (effective_permissions.final_permissions & ADMINISTRATOR) != 0)
}

// Fonctions de compatibilité pour l'ancien système
#[allow(dead_code)]
pub async fn check_channel_permission(
    pool: &SqlitePool,
    user_id: &str,
    channel_id: &str,
    action: &str,
) -> Result<bool, sqlx::Error> {
    match action {
        "view" => can_view_channel(pool, user_id, channel_id).await,
        "send_messages" => can_send_messages(pool, user_id, channel_id).await,
        "manage_messages" => can_manage_messages(pool, user_id, channel_id).await,
        "connect_voice" => can_connect_voice(pool, user_id, channel_id).await,
        "speak" => can_speak_voice(pool, user_id, channel_id).await,
        _ => Ok(false),
    }
}

/// Récupère le rôle d'un utilisateur sur un serveur (pour compatibilité)
pub async fn get_user_role(
    pool: &SqlitePool,
    user_id: &str,
    server_id: &str,
) -> Result<Option<String>, sqlx::Error> {
    let row = sqlx::query("SELECT role FROM server_members WHERE server_id = ? AND user_id = ?")
        .bind(server_id)
        .bind(user_id)
        .fetch_optional(pool)
        .await?;

    Ok(row.map(|r| r.get("role")))
}

/// Vérifie si un utilisateur peut gérer un serveur (pour compatibilité)
pub async fn can_manage_server(
    pool: &SqlitePool,
    user_id: &str,
    server_id: &str,
) -> Result<bool, sqlx::Error> {
    // Pour le moment, on considère que seul le propriétaire peut gérer le serveur
    let owner_check = sqlx::query(
        "SELECT 1 FROM server_members WHERE server_id = ? AND user_id = ? AND role = 'owner'",
    )
    .bind(server_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    Ok(owner_check.is_some())
}

/// Vérifie si un utilisateur est propriétaire d'un serveur (pour compatibilité)
pub async fn is_server_owner(
    pool: &SqlitePool,
    user_id: &str,
    server_id: &str,
) -> Result<bool, sqlx::Error> {
    let owner_check = sqlx::query(
        "SELECT 1 FROM server_members WHERE server_id = ? AND user_id = ? AND role = 'owner'",
    )
    .bind(server_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    Ok(owner_check.is_some())
}
