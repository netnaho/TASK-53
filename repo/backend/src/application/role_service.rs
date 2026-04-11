use sqlx::MySqlPool;
use uuid::Uuid;

use crate::domain::auth_types::*;
use crate::domain::error::AppError;
use crate::infrastructure::audit::{AuditEntry, AuditService, actions};
use crate::infrastructure::permission_cache::PermissionCache;

#[derive(Clone)]
pub struct RoleService {
    pool: MySqlPool,
    audit: AuditService,
    perm_cache: PermissionCache,
}

impl RoleService {
    pub fn new(pool: MySqlPool, audit: AuditService, perm_cache: PermissionCache) -> Self {
        Self { pool, audit, perm_cache }
    }

    pub async fn list_roles(&self) -> Result<Vec<RoleRow>, AppError> {
        sqlx::query_as::<_, RoleRow>(
            "SELECT id, name, description, is_system, created_at, updated_at FROM roles ORDER BY name"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))
    }

    pub async fn get_role(&self, role_id: &str) -> Result<RoleRow, AppError> {
        sqlx::query_as::<_, RoleRow>(
            "SELECT id, name, description, is_system, created_at, updated_at FROM roles WHERE id = ?"
        )
        .bind(role_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("Role not found".to_string()))
    }

    pub async fn create_role(
        &self,
        req: &CreateRoleRequest,
        actor_id: &str,
    ) -> Result<RoleRow, AppError> {
        let role_id = Uuid::new_v4().to_string();

        sqlx::query("INSERT INTO roles (id, name, description, is_system) VALUES (?, ?, ?, 0)")
            .bind(&role_id)
            .bind(&req.name)
            .bind(&req.description)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        self.audit.log(AuditEntry {
            user_id: Some(actor_id.to_string()),
            action: actions::ROLE_CREATED.to_string(),
            resource_type: "role".to_string(),
            resource_id: Some(role_id.clone()),
            org_id: None,
            details: Some(serde_json::json!({"name": &req.name})),
            ip_address: None,
            trace_id: None,
        }).await;

        self.get_role(&role_id).await
    }

    pub async fn get_role_permissions(&self, role_id: &str) -> Result<Vec<PermissionRow>, AppError> {
        sqlx::query_as::<_, PermissionRow>(
            "SELECT p.id, p.code, p.name, p.category, p.description, p.resource
             FROM permissions p
             INNER JOIN role_permissions rp ON rp.permission_id = p.id
             WHERE rp.role_id = ?
             ORDER BY p.category, p.code"
        )
        .bind(role_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))
    }

    pub async fn assign_permission(
        &self,
        role_id: &str,
        permission_id: &str,
        actor_id: &str,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT IGNORE INTO role_permissions (role_id, permission_id) VALUES (?, ?)"
        )
        .bind(role_id)
        .bind(permission_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        self.perm_cache.invalidate().await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        self.audit.log(AuditEntry {
            user_id: Some(actor_id.to_string()),
            action: actions::PERMISSION_GRANTED.to_string(),
            resource_type: "role_permission".to_string(),
            resource_id: Some(role_id.to_string()),
            org_id: None,
            details: Some(serde_json::json!({"permission_id": permission_id})),
            ip_address: None,
            trace_id: None,
        }).await;

        Ok(())
    }

    pub async fn revoke_permission(
        &self,
        role_id: &str,
        permission_id: &str,
        actor_id: &str,
    ) -> Result<(), AppError> {
        sqlx::query("DELETE FROM role_permissions WHERE role_id = ? AND permission_id = ?")
            .bind(role_id)
            .bind(permission_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        self.perm_cache.invalidate().await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        self.audit.log(AuditEntry {
            user_id: Some(actor_id.to_string()),
            action: actions::PERMISSION_REVOKED.to_string(),
            resource_type: "role_permission".to_string(),
            resource_id: Some(role_id.to_string()),
            org_id: None,
            details: Some(serde_json::json!({"permission_id": permission_id})),
            ip_address: None,
            trace_id: None,
        }).await;

        Ok(())
    }

    pub async fn list_permissions(&self) -> Result<Vec<PermissionRow>, AppError> {
        sqlx::query_as::<_, PermissionRow>(
            "SELECT id, code, name, category, description, resource
             FROM permissions ORDER BY category, code"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))
    }
}
