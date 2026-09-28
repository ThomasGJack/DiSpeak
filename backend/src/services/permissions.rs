use crate::domain::permissions::{
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

// /// Structure pour les overrides de permissions
// #[derive(Debug, Clone)]
// pub struct PermissionOverride {
//     pub allow_bitfield: i32,
//     pub deny_bitfield: i32,
// }

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
        "SELECT sr.permissions_bitfield \
         FROM server_roles sr\
         JOIN member_roles mr ON sr.id = mr.role_id\
         JOIN server_members sm ON mr.member_id = sm.id\
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
        "SELECT c.server_id, c.category_id, c.type\
         FROM channels c \
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
            "SELECT cro.allow_bitfield, cro.deny_bitfield\
             FROM category_role_overrides cro\
             JOIN member_roles mr ON cro.role_id = mr.role_id\
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
                "SELECT allow_bitfield, deny_bitfield\
                 FROM member_category_overrides\
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
        "SELECT cro.allow_bitfield, cro.deny_bitfield\
         FROM channel_role_overrides cro\
         JOIN member_roles mr ON cro.role_id = mr.role_id\
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
            "SELECT allow_bitfield, deny_bitfield\
             FROM member_channel_overrides\
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
        "SELECT cro.allow_bitfield, cro.deny_bitfield\
         FROM category_role_overrides cro\
         JOIN member_roles mr ON cro.role_id = mr.role_id\
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
            "SELECT allow_bitfield, deny_bitfield\
             FROM member_category_overrides\
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

/// Vérifie si un utilisateur peut gérer les canaux d'un serveur
pub async fn can_manage_channels(
    pool: &SqlitePool,
    user_id: &str,
    server_id: &str,
) -> Result<bool, sqlx::Error> {
    check_permission(pool, user_id, server_id, MANAGE_CHANNELS).await
}

// /// Vérifie si un utilisateur peut gérer un serveur
// pub async fn can_manage_server(
//     pool: &SqlitePool,
//     user_id: &str,
//     server_id: &str,
// ) -> Result<bool, sqlx::Error> {
//     check_permission(pool, user_id, server_id, MANAGE_SERVER).await
// }

/// Vérifie si un utilisateur peut gérer les rôles d'un serveur
pub async fn can_manage_roles(
    pool: &SqlitePool,
    user_id: &str,
    server_id: &str,
) -> Result<bool, sqlx::Error> {
    check_permission(pool, user_id, server_id, MANAGE_ROLES).await
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

// /// Vérifie si un utilisateur peut couper le micro d'autres membres
// pub async fn can_mute_members(
//     pool: &SqlitePool,
//     user_id: &str,
//     server_id: &str,
// ) -> Result<bool, sqlx::Error> {
//     check_permission(pool, user_id, server_id, MUTE_MEMBERS).await
// }

// /// Vérifie si un utilisateur peut couper le son d'autres membres
// pub async fn can_deafen_members(
//     pool: &SqlitePool,
//     user_id: &str,
//     server_id: &str,
// ) -> Result<bool, sqlx::Error> {
//     check_permission(pool, user_id, server_id, DEAFEN_MEMBERS).await
// }

// /// Vérifie si un utilisateur peut déplacer d'autres membres entre salons
// pub async fn can_move_members(
//     pool: &SqlitePool,
//     user_id: &str,
//     server_id: &str,
// ) -> Result<bool, sqlx::Error> {
//     check_permission(pool, user_id, server_id, MOVE_MEMBERS).await
// }

// /// Vérifie si un utilisateur est propriétaire d'un serveur
// pub async fn is_server_owner(
//     pool: &SqlitePool,
//     user_id: &str,
//     server_id: &str,
// ) -> Result<bool, sqlx::Error> {
//     let owner_check = sqlx::query(
//         "SELECT 1 FROM server_members WHERE server_id = ? AND user_id = ? AND role = 'owner'",
//     )
//     .bind(server_id)
//     .bind(user_id)
//     .fetch_optional(pool)
//     .await?;

//     Ok(owner_check.is_some())
// }

// /// Fonction utilitaire pour vérifier plusieurs permissions à la fois
// pub async fn check_multiple_permissions(
//     pool: &SqlitePool,
//     user_id: &str,
//     server_id: &str,
//     permissions: &[i32],
// ) -> Result<bool, sqlx::Error> {
//     let user_permissions = get_user_permissions(pool, user_id, server_id).await?;

//     // Si l'utilisateur a ADMINISTRATOR, il a toutes les permissions
//     if (user_permissions & ADMINISTRATOR) != 0 {
//         return Ok(true);
//     }

//     // Vérifier si l'utilisateur a au moins une des permissions demandées
//     for permission in permissions {
//         if (user_permissions & permission) != 0 {
//             return Ok(true);
//         }
//     }

//     Ok(false)
// }

/// Définit les overrides de permissions pour un rôle sur un canal
pub async fn set_channel_role_overrides(
    pool: &SqlitePool,
    channel_id: &str,
    role_id: &str,
    allow_bitfield: i32,
    deny_bitfield: i32,
) -> Result<(), sqlx::Error> {
    let override_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT OR REPLACE INTO channel_role_overrides \
         (id, channel_id, role_id, allow_bitfield, deny_bitfield, created_at, updated_at)\
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&override_id)
    .bind(channel_id)
    .bind(role_id)
    .bind(allow_bitfield)
    .bind(deny_bitfield)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;

    Ok(())
}

/// Définit les overrides de permissions pour un rôle sur une catégorie
pub async fn set_category_role_overrides(
    pool: &SqlitePool,
    category_id: &str,
    role_id: &str,
    allow_bitfield: i32,
    deny_bitfield: i32,
) -> Result<(), sqlx::Error> {
    let override_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT OR REPLACE INTO category_role_overrides \
         (id, category_id, role_id, allow_bitfield, deny_bitfield, created_at, updated_at)\
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&override_id)
    .bind(category_id)
    .bind(role_id)
    .bind(allow_bitfield)
    .bind(deny_bitfield)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;

    Ok(())
}

/// Définit les overrides de permissions pour un membre sur un canal
pub async fn set_member_channel_overrides(
    pool: &SqlitePool,
    channel_id: &str,
    member_id: &str,
    allow_bitfield: i32,
    deny_bitfield: i32,
) -> Result<(), sqlx::Error> {
    let override_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT OR REPLACE INTO member_channel_overrides \
         (id, channel_id, member_id, allow_bitfield, deny_bitfield, created_at, updated_at)\
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&override_id)
    .bind(channel_id)
    .bind(member_id)
    .bind(allow_bitfield)
    .bind(deny_bitfield)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;

    Ok(())
}

/// Définit les overrides de permissions pour un membre sur une catégorie
pub async fn set_member_category_overrides(
    pool: &SqlitePool,
    category_id: &str,
    member_id: &str,
    allow_bitfield: i32,
    deny_bitfield: i32,
) -> Result<(), sqlx::Error> {
    let override_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT OR REPLACE INTO member_category_overrides \
         (id, category_id, member_id, allow_bitfield, deny_bitfield, created_at, updated_at)\
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&override_id)
    .bind(category_id)
    .bind(member_id)
    .bind(allow_bitfield)
    .bind(deny_bitfield)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;

    Ok(())
}

/// Supprime les overrides de permissions pour un rôle sur un canal
pub async fn remove_channel_role_overrides(
    pool: &SqlitePool,
    channel_id: &str,
    role_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM channel_role_overrides WHERE channel_id = ? AND role_id = ?")
        .bind(channel_id)
        .bind(role_id)
        .execute(pool)
        .await?;

    Ok(())
}

/// Supprime les overrides de permissions pour un rôle sur une catégorie
pub async fn remove_category_role_overrides(
    pool: &SqlitePool,
    category_id: &str,
    role_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM category_role_overrides WHERE category_id = ? AND role_id = ?")
        .bind(category_id)
        .bind(role_id)
        .execute(pool)
        .await?;

    Ok(())
}

// /// Supprime les overrides de permissions pour un membre sur un canal
// pub async fn remove_member_channel_overrides(
//     pool: &SqlitePool,
//     channel_id: &str,
//     member_id: &str,
// ) -> Result<(), sqlx::Error> {
//     sqlx::query(
//         "DELETE FROM member_channel_overrides WHERE channel_id = ? AND member_id = ?",
//     )
//     .bind(channel_id)
//     .bind(member_id)
//     .execute(pool)
//     .await?;

//     Ok(())
// }

// /// Supprime les overrides de permissions pour un membre sur une catégorie
// pub async fn remove_member_category_overrides(
//     pool: &SqlitePool,
//     category_id: &str,
//     member_id: &str,
// ) -> Result<(), sqlx::Error> {
//     sqlx::query(
//         "DELETE FROM member_category_overrides WHERE category_id = ? AND member_id = ?",
//     )
//     .bind(category_id)
//     .bind(member_id)
//     .execute(pool)
//     .await?;

//     Ok(())
// }

// Fonctions de compatibilité pour l'ancien système (à supprimer progressivement)
#[allow(dead_code)]
pub async fn get_user_role(
    pool: &SqlitePool,
    user_id: &str,
    server_id: &str,
) -> Result<Option<String>, sqlx::Error> {
    let row = sqlx::query("SELECT role FROM server_members WHERE user_id = ? AND server_id = ?")
        .bind(user_id)
        .bind(server_id)
        .fetch_optional(pool)
        .await?;

    Ok(row.map(|r| r.get("role")))
}

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

#[allow(dead_code)]
pub async fn check_category_permission(
    pool: &SqlitePool,
    user_id: &str,
    category_id: &str,
    action: &str,
) -> Result<bool, sqlx::Error> {
    let server_row = sqlx::query("SELECT server_id FROM categories WHERE id = ?")
        .bind(category_id)
        .fetch_optional(pool)
        .await?;

    let server_id = match server_row {
        Some(row) => row.get::<String, _>("server_id"),
        None => return Ok(false),
    };

    match action {
        "view" => check_permission(pool, user_id, &server_id, VIEW_CHANNELS).await,
        "send_messages" => check_permission(pool, user_id, &server_id, SEND_MESSAGES).await,
        "manage_messages" => check_permission(pool, user_id, &server_id, MANAGE_MESSAGES).await,
        _ => Ok(false),
    }
}

