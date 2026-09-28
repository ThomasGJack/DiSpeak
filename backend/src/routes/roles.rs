use actix_web::{web, HttpResponse, Result};
use serde_json::json;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::domain::{
    AssignRoles, Claims, CreateRole, ServerMember, ServerMemberWithRoles, ServerRole,
    ServerRoleWithMembers, UpdateRole,
};
use crate::services::permissions::{
    can_manage_roles, get_effective_category_permissions, remove_category_role_overrides,
    set_category_role_overrides, set_member_category_overrides,
};

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/servers/{server_id}")
            .route("/roles", web::post().to(create_role))
            .route("/roles", web::get().to(get_server_roles))
            .route("/roles/{role_id}", web::put().to(update_role))
            .route("/roles/{role_id}", web::delete().to(delete_role))
            .route(
                "/roles/{role_id}/permissions",
                web::post().to(set_role_permissions),
            )
            .route(
                "/roles/{role_id}/permissions",
                web::get().to(get_role_permissions),
            )
            .route(
                "/members/{user_id}/roles",
                web::post().to(assign_member_roles),
            )
            .route("/members-with-roles", web::get().to(get_members_with_roles))
            .route(
                "/categories/{category_id}/permissions",
                web::post().to(set_category_permissions),
            )
            .route(
                "/categories/{category_id}/permissions",
                web::get().to(get_category_permissions),
            )
            .route(
                "/categories/{category_id}/permissions/{role_id}",
                web::delete().to(remove_category_permissions),
            )
            .route(
                "/categories/{category_id}/members/{member_id}/permissions",
                web::post().to(set_member_category_permissions),
            ),
    );
}

/// Créer un nouveau rôle personnalisé
pub async fn create_role(
    pool: web::Data<SqlitePool>,
    claims: Claims,
    path: web::Path<String>,
    role_data: web::Json<CreateRole>,
) -> Result<HttpResponse> {
    let server_id = path.into_inner();
    let payload = role_data.into_inner();

    // Vérifier que l'utilisateur est propriétaire du serveur
    let owner_check = sqlx::query(
        "SELECT 1 FROM server_members WHERE server_id = ? AND user_id = ? AND role = 'owner'",
    )
    .bind(&server_id)
    .bind(&claims.user_id)
    .fetch_optional(pool.get_ref())
    .await
    .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    if owner_check.is_none() {
        return Ok(HttpResponse::Forbidden().json(json!({
            "error": "Only server owner can create roles"
        })));
    }

    // Générer un ID unique pour le rôle
    let role_id = Uuid::new_v4().to_string();

    // Définir les valeurs par défaut
    let color = payload.color.unwrap_or_else(|| "#5865F2".to_string());
    let permissions = payload.permissions_bitfield.unwrap_or(0);
    let position = payload.position.unwrap_or(0);
    let now = chrono::Utc::now().to_rfc3339();

    // Insérer le nouveau rôle
    sqlx::query(
        "INSERT INTO server_roles (id, server_id, name, color, position, permissions_bitfield, is_default, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&role_id)
    .bind(&server_id)
    .bind(&payload.name)
    .bind(&color)
    .bind(position)
    .bind(permissions)
    .bind(false) // is_default = false pour les rôles personnalisés
    .bind(&now)
    .execute(pool.get_ref())
    .await
    .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    // Récupérer le rôle créé
    let role = sqlx::query_as::<_, ServerRole>(
        "SELECT id, server_id, name, color, position, permissions_bitfield, is_default, created_at FROM server_roles WHERE id = ?"
    )
    .bind(&role_id)
    .fetch_one(pool.get_ref())
    .await
    .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    Ok(HttpResponse::Created().json(role))
}

/// Lister tous les rôles d'un serveur
pub async fn get_server_roles(
    pool: web::Data<SqlitePool>,
    claims: Claims,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let server_id = path.into_inner();

    // Vérifier que l'utilisateur est membre du serveur
    let member_check =
        sqlx::query("SELECT 1 FROM server_members WHERE server_id = ? AND user_id = ?")
            .bind(&server_id)
            .bind(&claims.user_id)
            .fetch_optional(pool.get_ref())
            .await
            .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    if member_check.is_none() {
        return Ok(HttpResponse::Forbidden().json(json!({
            "error": "Not a member of this server"
        })));
    }

    // Récupérer les rôles avec le nombre de membres
    let roles_rows = sqlx::query(
        "SELECT 
            sr.id, sr.server_id, sr.name, sr.color, sr.position, sr.permissions_bitfield, sr.is_default, sr.created_at,
            COUNT(mr.id) as members_count
         FROM server_roles sr
         LEFT JOIN member_roles mr ON sr.id = mr.role_id
         WHERE sr.server_id = ?
         GROUP BY sr.id
         ORDER BY sr.position DESC, sr.created_at ASC"
    )
    .bind(&server_id)
    .fetch_all(pool.get_ref())
    .await
    .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    let mut roles_with_members = Vec::new();
    for row in roles_rows {
        let role = ServerRole {
            id: row.get("id"),
            server_id: row.get("server_id"),
            name: row.get("name"),
            color: row.get("color"),
            position: row.get("position"),
            permissions_bitfield: row.get("permissions_bitfield"),
            is_default: row.get("is_default"),
            created_at: row.get("created_at"),
        };
        let members_count: i32 = row.get("members_count");

        roles_with_members.push(ServerRoleWithMembers {
            role,
            members_count,
        });
    }

    Ok(HttpResponse::Ok().json(roles_with_members))
}

/// Modifier un rôle existant
pub async fn update_role(
    pool: web::Data<SqlitePool>,
    claims: Claims,
    path: web::Path<(String, String)>,
    role_data: web::Json<UpdateRole>,
) -> Result<HttpResponse> {
    let (server_id, role_id) = path.into_inner();
    let payload = role_data.into_inner();

    // Vérifier que l'utilisateur est propriétaire du serveur
    let owner_check = sqlx::query(
        "SELECT 1 FROM server_members WHERE server_id = ? AND user_id = ? AND role = 'owner'",
    )
    .bind(&server_id)
    .bind(&claims.user_id)
    .fetch_optional(pool.get_ref())
    .await
    .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    if owner_check.is_none() {
        return Ok(HttpResponse::Forbidden().json(json!({
            "error": "Only server owner can modify roles"
        })));
    }

    // Vérifier que le rôle existe et appartient au serveur
    let role_check =
        sqlx::query("SELECT is_default FROM server_roles WHERE id = ? AND server_id = ?")
            .bind(&role_id)
            .bind(&server_id)
            .fetch_optional(pool.get_ref())
            .await
            .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    if let Some(row) = role_check {
        let is_default: bool = row.get("is_default");
        if is_default {
            return Ok(HttpResponse::BadRequest().json(json!({
                "error": "Cannot modify default roles"
            })));
        }
    } else {
        return Ok(HttpResponse::NotFound().json(json!({
            "error": "Role not found"
        })));
    }

    // Construire la requête de mise à jour dynamiquement
    let mut query = "UPDATE server_roles SET ".to_string();
    let mut set_clauses = Vec::new();

    if let Some(ref _name) = payload.name {
        set_clauses.push("name = ?");
    }

    if let Some(ref _color) = payload.color {
        set_clauses.push("color = ?");
    }

    if let Some(ref _permissions) = payload.permissions_bitfield {
        set_clauses.push("permissions_bitfield = ?");
    }

    if let Some(ref _position) = payload.position {
        set_clauses.push("position = ?");
    }

    if set_clauses.is_empty() {
        return Ok(HttpResponse::BadRequest().json(json!({
            "error": "No fields to update"
        })));
    }

    query.push_str(&set_clauses.join(", "));
    query.push_str(" WHERE id = ? AND server_id = ?");

    // Construire la requête avec les valeurs
    let mut query_builder = sqlx::query(&query);

    if let Some(name) = payload.name {
        query_builder = query_builder.bind(name);
    }
    if let Some(color) = payload.color {
        query_builder = query_builder.bind(color);
    }
    if let Some(permissions) = payload.permissions_bitfield {
        query_builder = query_builder.bind(permissions);
    }
    if let Some(position) = payload.position {
        query_builder = query_builder.bind(position);
    }

    query_builder = query_builder.bind(&role_id).bind(&server_id);

    // Exécuter la mise à jour
    query_builder
        .execute(pool.get_ref())
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    // Récupérer le rôle mis à jour
    let updated_role = sqlx::query_as::<_, ServerRole>(
        "SELECT id, server_id, name, color, position, permissions_bitfield, is_default, created_at FROM server_roles WHERE id = ?"
    )
    .bind(&role_id)
    .fetch_one(pool.get_ref())
    .await
    .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    Ok(HttpResponse::Ok().json(updated_role))
}

/// Supprimer un rôle personnalisé
pub async fn delete_role(
    pool: web::Data<SqlitePool>,
    claims: Claims,
    path: web::Path<(String, String)>,
) -> Result<HttpResponse> {
    let (server_id, role_id) = path.into_inner();

    // Vérifier que l'utilisateur est propriétaire du serveur
    let owner_check = sqlx::query(
        "SELECT 1 FROM server_members WHERE server_id = ? AND user_id = ? AND role = 'owner'",
    )
    .bind(&server_id)
    .bind(&claims.user_id)
    .fetch_optional(pool.get_ref())
    .await
    .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    if owner_check.is_none() {
        return Ok(HttpResponse::Forbidden().json(json!({
            "error": "Only server owner can delete roles"
        })));
    }

    // Vérifier que le rôle existe et n'est pas un rôle par défaut
    let role_check =
        sqlx::query("SELECT is_default FROM server_roles WHERE id = ? AND server_id = ?")
            .bind(&role_id)
            .bind(&server_id)
            .fetch_optional(pool.get_ref())
            .await
            .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    if let Some(row) = role_check {
        let is_default: bool = row.get("is_default");
        if is_default {
            return Ok(HttpResponse::BadRequest().json(json!({
                "error": "Cannot delete default roles"
            })));
        }
    } else {
        return Ok(HttpResponse::NotFound().json(json!({
            "error": "Role not found"
        })));
    }

    // Supprimer le rôle (les contraintes de clé étrangère supprimeront automatiquement les assignations)
    sqlx::query("DELETE FROM server_roles WHERE id = ? AND server_id = ?")
        .bind(&role_id)
        .bind(&server_id)
        .execute(pool.get_ref())
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    Ok(HttpResponse::Ok().json(json!({
        "message": "Role deleted successfully"
    })))
}

/// Assigner des rôles à un membre
pub async fn assign_member_roles(
    pool: web::Data<SqlitePool>,
    claims: Claims,
    path: web::Path<(String, String)>,
    roles_data: web::Json<AssignRoles>,
) -> Result<HttpResponse> {
    let (server_id, user_id) = path.into_inner();
    let payload = roles_data.into_inner();

    // Vérifier que l'utilisateur est propriétaire du serveur
    let owner_check = sqlx::query(
        "SELECT 1 FROM server_members WHERE server_id = ? AND user_id = ? AND role = 'owner'",
    )
    .bind(&server_id)
    .bind(&claims.user_id)
    .fetch_optional(pool.get_ref())
    .await
    .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    if owner_check.is_none() {
        return Ok(HttpResponse::Forbidden().json(json!({
            "error": "Only server owner can assign roles"
        })));
    }

    // Vérifier que le membre existe
    let member_check =
        sqlx::query("SELECT id FROM server_members WHERE server_id = ? AND user_id = ?")
            .bind(&server_id)
            .bind(&user_id)
            .fetch_optional(pool.get_ref())
            .await
            .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    if member_check.is_none() {
        return Ok(HttpResponse::NotFound().json(json!({
            "error": "Member not found"
        })));
    }

    let member_id: String = member_check.unwrap().get("id");

    // Vérifier que tous les rôles appartiennent au serveur
    if !payload.role_ids.is_empty() {
        let role_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM server_roles WHERE id IN (SELECT value FROM json_each(?)) AND server_id = ?"
        )
        .bind(serde_json::to_string(&payload.role_ids).unwrap())
        .bind(&server_id)
        .fetch_one(pool.get_ref())
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

        if role_count != payload.role_ids.len() as i64 {
            return Ok(HttpResponse::BadRequest().json(json!({
                "error": "One or more roles do not belong to this server"
            })));
        }
    }

    // Supprimer les assignations existantes
    sqlx::query("DELETE FROM member_roles WHERE member_id = ?")
        .bind(&member_id)
        .execute(pool.get_ref())
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    // Ajouter les nouvelles assignations
    let now = chrono::Utc::now().to_rfc3339();
    for role_id in payload.role_ids {
        let assignment_id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO member_roles (id, member_id, role_id, assigned_at) VALUES (?, ?, ?, ?)",
        )
        .bind(&assignment_id)
        .bind(&member_id)
        .bind(&role_id)
        .bind(&now)
        .execute(pool.get_ref())
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;
    }

    Ok(HttpResponse::Ok().json(json!({
        "message": "Roles assigned successfully"
    })))
}

/// Obtenir les membres avec leurs rôles
pub async fn get_members_with_roles(
    pool: web::Data<SqlitePool>,
    claims: Claims,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let server_id = path.into_inner();

    // Vérifier que l'utilisateur est membre du serveur
    let member_check =
        sqlx::query("SELECT 1 FROM server_members WHERE server_id = ? AND user_id = ?")
            .bind(&server_id)
            .bind(&claims.user_id)
            .fetch_optional(pool.get_ref())
            .await
            .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    if member_check.is_none() {
        return Ok(HttpResponse::Forbidden().json(json!({
            "error": "Not a member of this server"
        })));
    }

    // Récupérer tous les membres avec leurs rôles
    let members = sqlx::query_as::<_, ServerMember>(
        "SELECT id, server_id, user_id, username, avatar, role, joined_at, last_seen, is_online FROM server_members WHERE server_id = ? ORDER BY (role = 'owner') DESC, is_online DESC, joined_at ASC"
    )
    .bind(&server_id)
    .fetch_all(pool.get_ref())
    .await
    .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    let mut members_with_roles = Vec::new();

    for member in members {
        let roles = sqlx::query_as::<_, ServerRole>(
            "SELECT sr.id, sr.server_id, sr.name, sr.color, sr.position, sr.permissions_bitfield, sr.is_default, sr.created_at 
             FROM server_roles sr 
             JOIN member_roles mr ON sr.id = mr.role_id 
             WHERE mr.member_id = ? 
             ORDER BY sr.position DESC"
        )
        .bind(&member.id)
        .fetch_all(pool.get_ref())
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

        members_with_roles.push(ServerMemberWithRoles { member, roles });
    }

    Ok(HttpResponse::Ok().json(members_with_roles))
}

/// Définir les permissions d'un rôle
pub async fn set_role_permissions(
    pool: web::Data<SqlitePool>,
    claims: Claims,
    path: web::Path<(String, String)>,
    permissions_data: web::Json<serde_json::Value>,
) -> Result<HttpResponse> {
    let (server_id, role_id) = path.into_inner();

    // Vérifier que l'utilisateur peut gérer les rôles
    if !can_manage_roles(pool.get_ref(), &claims.user_id, &server_id)
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?
    {
        return Ok(HttpResponse::Forbidden().json(json!({
            "error": "You don't have permission to manage roles"
        })));
    }

    // Vérifier que le rôle existe et appartient au serveur
    let role_check =
        sqlx::query("SELECT is_default FROM server_roles WHERE id = ? AND server_id = ?")
            .bind(&role_id)
            .bind(&server_id)
            .fetch_optional(pool.get_ref())
            .await
            .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    if let Some(row) = role_check {
        let is_default: bool = row.get("is_default");
        if is_default {
            return Ok(HttpResponse::BadRequest().json(json!({
                "error": "Cannot modify default roles"
            })));
        }
    } else {
        return Ok(HttpResponse::NotFound().json(json!({
            "error": "Role not found"
        })));
    }

    let permissions_bitfield = permissions_data
        .get("permissions_bitfield")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| actix_web::error::ErrorBadRequest("Missing permissions_bitfield"))?
        as i32;

    // Mettre à jour les permissions du rôle
    sqlx::query("UPDATE server_roles SET permissions_bitfield = ? WHERE id = ? AND server_id = ?")
        .bind(permissions_bitfield)
        .bind(&role_id)
        .bind(&server_id)
        .execute(pool.get_ref())
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    Ok(HttpResponse::Ok().json(json!({
        "message": "Role permissions updated successfully"
    })))
}

/// Récupérer les permissions d'un rôle
pub async fn get_role_permissions(
    pool: web::Data<SqlitePool>,
    claims: Claims,
    path: web::Path<(String, String)>,
) -> Result<HttpResponse> {
    let (server_id, role_id) = path.into_inner();

    // Vérifier que l'utilisateur est membre du serveur
    let member_check =
        sqlx::query("SELECT 1 FROM server_members WHERE server_id = ? AND user_id = ?")
            .bind(&server_id)
            .bind(&claims.user_id)
            .fetch_optional(pool.get_ref())
            .await
            .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    if member_check.is_none() {
        return Ok(HttpResponse::Forbidden().json(json!({
            "error": "Not a member of this server"
        })));
    }

    // Récupérer les permissions du rôle
    let role = sqlx::query_as::<_, ServerRole>(
        "SELECT id, server_id, name, color, position, permissions_bitfield, is_default, created_at 
         FROM server_roles WHERE id = ? AND server_id = ?",
    )
    .bind(&role_id)
    .bind(&server_id)
    .fetch_optional(pool.get_ref())
    .await
    .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    match role {
        Some(role) => Ok(HttpResponse::Ok().json(json!({
            "role_id": role.id,
            "permissions_bitfield": role.permissions_bitfield,
            "is_default": role.is_default
        }))),
        None => Ok(HttpResponse::NotFound().json(json!({
            "error": "Role not found"
        }))),
    }
}

/// Définir les overrides de permissions pour un rôle sur une catégorie
pub async fn set_category_permissions(
    pool: web::Data<SqlitePool>,
    claims: Claims,
    path: web::Path<(String, String)>,
    permissions_data: web::Json<serde_json::Value>,
) -> Result<HttpResponse> {
    let (server_id, category_id) = path.into_inner();

    // Vérifier que l'utilisateur peut gérer les rôles
    if !can_manage_roles(pool.get_ref(), &claims.user_id, &server_id)
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?
    {
        return Ok(HttpResponse::Forbidden().json(json!({
            "error": "You don't have permission to manage roles"
        })));
    }

    // Vérifier que la catégorie appartient au serveur
    let category_check = sqlx::query("SELECT 1 FROM categories WHERE id = ? AND server_id = ?")
        .bind(&category_id)
        .bind(&server_id)
        .fetch_optional(pool.get_ref())
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    if category_check.is_none() {
        return Ok(HttpResponse::NotFound().json(json!({
            "error": "Category not found for this server"
        })));
    }

    let role_id = permissions_data
        .get("role_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| actix_web::error::ErrorBadRequest("Missing role_id"))?;

    let allow_bitfield = permissions_data
        .get("allow_bitfield")
        .and_then(|v| v.as_i64())
        .unwrap_or(0) as i32;

    let deny_bitfield = permissions_data
        .get("deny_bitfield")
        .and_then(|v| v.as_i64())
        .unwrap_or(0) as i32;

    // Vérifier que le rôle appartient au serveur
    let role_check = sqlx::query("SELECT 1 FROM server_roles WHERE id = ? AND server_id = ?")
        .bind(role_id)
        .bind(&server_id)
        .fetch_optional(pool.get_ref())
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    if role_check.is_none() {
        return Ok(HttpResponse::BadRequest().json(json!({
            "error": "Role not found for this server"
        })));
    }

    // Définir les overrides
    set_category_role_overrides(
        pool.get_ref(),
        &category_id,
        role_id,
        allow_bitfield,
        deny_bitfield,
    )
    .await
    .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    Ok(HttpResponse::Ok().json(json!({
        "message": "Category permissions updated successfully"
    })))
}

/// Définir les overrides de permissions pour un membre sur une catégorie
pub async fn set_member_category_permissions(
    pool: web::Data<SqlitePool>,
    claims: Claims,
    path: web::Path<(String, String, String)>,
    permissions_data: web::Json<serde_json::Value>,
) -> Result<HttpResponse> {
    let (server_id, category_id, member_id) = path.into_inner();

    // Vérifier que l'utilisateur peut gérer les rôles
    if !can_manage_roles(pool.get_ref(), &claims.user_id, &server_id)
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?
    {
        return Ok(HttpResponse::Forbidden().json(json!({
            "error": "You don't have permission to manage roles"
        })));
    }

    // Vérifier que la catégorie appartient au serveur
    let category_check = sqlx::query("SELECT 1 FROM categories WHERE id = ? AND server_id = ?")
        .bind(&category_id)
        .bind(&server_id)
        .fetch_optional(pool.get_ref())
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    if category_check.is_none() {
        return Ok(HttpResponse::NotFound().json(json!({
            "error": "Category not found for this server"
        })));
    }

    // Vérifier que le membre appartient au serveur
    let member_check = sqlx::query("SELECT id FROM server_members WHERE id = ? AND server_id = ?")
        .bind(&member_id)
        .bind(&server_id)
        .fetch_optional(pool.get_ref())
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    if member_check.is_none() {
        return Ok(HttpResponse::BadRequest().json(json!({
            "error": "Member not found for this server"
        })));
    }

    let allow_bitfield = permissions_data
        .get("allow_bitfield")
        .and_then(|v| v.as_i64())
        .unwrap_or(0) as i32;

    let deny_bitfield = permissions_data
        .get("deny_bitfield")
        .and_then(|v| v.as_i64())
        .unwrap_or(0) as i32;

    // Définir les overrides
    set_member_category_overrides(
        pool.get_ref(),
        &category_id,
        &member_id,
        allow_bitfield,
        deny_bitfield,
    )
    .await
    .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    Ok(HttpResponse::Ok().json(json!({
        "message": "Member category permissions updated successfully"
    })))
}

/// Récupérer les permissions effectives d'un utilisateur pour une catégorie
pub async fn get_category_permissions(
    pool: web::Data<SqlitePool>,
    claims: Claims,
    path: web::Path<(String, String)>,
) -> Result<HttpResponse> {
    let (server_id, category_id) = path.into_inner();

    // Vérifier que l'utilisateur est membre du serveur
    let member_check =
        sqlx::query("SELECT 1 FROM server_members WHERE server_id = ? AND user_id = ?")
            .bind(&server_id)
            .bind(&claims.user_id)
            .fetch_optional(pool.get_ref())
            .await
            .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    if member_check.is_none() {
        return Ok(HttpResponse::Forbidden().json(json!({
            "error": "Not a member of this server"
        })));
    }

    // Récupérer les permissions effectives
    let effective_permissions =
        get_effective_category_permissions(pool.get_ref(), &claims.user_id, &category_id)
            .await
            .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    Ok(HttpResponse::Ok().json(json!({
        "category_id": category_id,
        "allow_bitfield": effective_permissions.allow_bitfield,
        "deny_bitfield": effective_permissions.deny_bitfield,
        "final_permissions": effective_permissions.final_permissions,
    })))
}

/// Supprimer les overrides de permissions pour un rôle sur une catégorie
pub async fn remove_category_permissions(
    pool: web::Data<SqlitePool>,
    claims: Claims,
    path: web::Path<(String, String, String)>,
) -> Result<HttpResponse> {
    let (server_id, category_id, role_id) = path.into_inner();

    // Vérifier que l'utilisateur peut gérer les rôles
    if !can_manage_roles(pool.get_ref(), &claims.user_id, &server_id)
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?
    {
        return Ok(HttpResponse::Forbidden().json(json!({
            "error": "You don't have permission to manage roles"
        })));
    }

    // Supprimer les overrides
    remove_category_role_overrides(pool.get_ref(), &category_id, &role_id)
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

    Ok(HttpResponse::Ok().json(json!({
        "message": "Category permissions removed successfully"
    })))
}
