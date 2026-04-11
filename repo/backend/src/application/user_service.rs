use sqlx::MySqlPool;
use uuid::Uuid;

use crate::application::auth_service::AuthService;
use crate::domain::auth_types::*;
use crate::domain::error::AppError;
use crate::infrastructure::audit::{AuditEntry, AuditService, actions};
use crate::infrastructure::permission_cache::PermissionCache;

#[derive(Clone)]
pub struct UserService {
    pool: MySqlPool,
    audit: AuditService,
    perm_cache: PermissionCache,
}

impl UserService {
    pub fn new(pool: MySqlPool, audit: AuditService, perm_cache: PermissionCache) -> Self {
        Self { pool, audit, perm_cache }
    }

    pub async fn list_users(
        &self,
        org_id: &str,
        params: &PaginationParams,
    ) -> Result<PaginatedResponse<UserRow>, AppError> {
        let limit = params.limit();
        let offset = params.offset();

        let mut where_clause = "WHERE u.org_id = ?".to_string();
        if let Some(ref search) = params.search {
            where_clause.push_str(&format!(
                " AND (u.username LIKE '%{}%' OR u.email LIKE '%{}%')",
                search.replace('\'', ""), search.replace('\'', "")
            ));
        }

        let count_query = format!("SELECT COUNT(*) FROM users u {}", where_clause);
        let (total,): (i64,) = sqlx::query_as(&count_query)
            .bind(org_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        let sort = match params.sort_by.as_deref() {
            Some("username") => "u.username",
            Some("email") => "u.email",
            Some("created_at") => "u.created_at",
            _ => "u.created_at",
        };
        let order = if params.sort_order.as_deref() == Some("asc") { "ASC" } else { "DESC" };

        let data_query = format!(
            "SELECT u.id, u.org_id, u.department_id, u.username, u.email, u.status, u.created_at, u.updated_at
             FROM users u {} ORDER BY {} {} LIMIT ? OFFSET ?",
            where_clause, sort, order
        );

        let users: Vec<UserRow> = sqlx::query_as(&data_query)
            .bind(org_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(PaginatedResponse {
            data: users,
            total,
            page: params.page.unwrap_or(1),
            per_page: limit,
        })
    }

    pub async fn get_user(&self, user_id: &str) -> Result<UserRow, AppError> {
        sqlx::query_as::<_, UserRow>(
            "SELECT id, org_id, department_id, username, email, status, created_at, updated_at
             FROM users WHERE id = ?"
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))
    }

    pub async fn create_user(
        &self,
        req: &CreateUserRequest,
        actor_id: &str,
    ) -> Result<UserRow, AppError> {
        // Check username uniqueness
        let existing: Option<(String,)> = sqlx::query_as(
            "SELECT id FROM users WHERE username = ?"
        )
        .bind(&req.username)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        if existing.is_some() {
            return Err(AppError::BadRequest("Username already exists".to_string()));
        }

        let user_id = Uuid::new_v4().to_string();
        let password_hash = AuthService::hash_password(&req.password)?;

        // Insert user
        sqlx::query(
            "INSERT INTO users (id, org_id, department_id, username, email, status)
             VALUES (?, ?, ?, ?, ?, 'active')"
        )
        .bind(&user_id)
        .bind(&req.org_id)
        .bind(&req.department_id)
        .bind(&req.username)
        .bind(&req.email)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        // Insert credentials
        sqlx::query(
            "INSERT INTO user_credentials (user_id, password_hash) VALUES (?, ?)"
        )
        .bind(&user_id)
        .bind(&password_hash)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        self.audit.log(AuditEntry {
            user_id: Some(actor_id.to_string()),
            action: actions::USER_CREATED.to_string(),
            resource_type: "user".to_string(),
            resource_id: Some(user_id.clone()),
            org_id: Some(req.org_id.clone()),
            details: Some(serde_json::json!({"username": &req.username, "email": &req.email})),
            ip_address: None,
            trace_id: None,
        }).await;

        self.get_user(&user_id).await
    }

    pub async fn update_user(
        &self,
        user_id: &str,
        req: &UpdateUserRequest,
        actor_id: &str,
    ) -> Result<UserRow, AppError> {
        let current = self.get_user(user_id).await?;
        let mut changes = serde_json::Map::new();

        if let Some(ref email) = req.email {
            sqlx::query("UPDATE users SET email = ? WHERE id = ?")
                .bind(email)
                .bind(user_id)
                .execute(&self.pool)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;
            changes.insert("email".to_string(), serde_json::json!({"from": current.email, "to": email}));
        }

        if let Some(ref status) = req.status {
            sqlx::query("UPDATE users SET status = ? WHERE id = ?")
                .bind(status)
                .bind(user_id)
                .execute(&self.pool)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;
            changes.insert("status".to_string(), serde_json::json!({"from": current.status, "to": status}));
        }

        if let Some(ref dept_id) = req.department_id {
            sqlx::query("UPDATE users SET department_id = ? WHERE id = ?")
                .bind(dept_id)
                .bind(user_id)
                .execute(&self.pool)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;
            changes.insert("department_id".to_string(), serde_json::json!(dept_id));
        }

        if !changes.is_empty() {
            self.audit.log(AuditEntry {
                user_id: Some(actor_id.to_string()),
                action: actions::USER_UPDATED.to_string(),
                resource_type: "user".to_string(),
                resource_id: Some(user_id.to_string()),
                org_id: Some(current.org_id.clone()),
                details: Some(serde_json::Value::Object(changes)),
                ip_address: None,
                trace_id: None,
            }).await;
        }

        self.get_user(user_id).await
    }

    pub async fn assign_role(
        &self,
        user_id: &str,
        role_id: &str,
        actor_id: &str,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT IGNORE INTO user_roles (user_id, role_id, assigned_by) VALUES (?, ?, ?)"
        )
        .bind(user_id)
        .bind(role_id)
        .bind(actor_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        self.perm_cache.invalidate().await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        // Get role name for audit
        let role_name: Option<(String,)> = sqlx::query_as("SELECT name FROM roles WHERE id = ?")
            .bind(role_id)
            .fetch_optional(&self.pool)
            .await
            .ok()
            .flatten();

        self.audit.log(AuditEntry {
            user_id: Some(actor_id.to_string()),
            action: actions::ROLE_ASSIGNED.to_string(),
            resource_type: "user_role".to_string(),
            resource_id: Some(user_id.to_string()),
            org_id: None,
            details: Some(serde_json::json!({
                "role_id": role_id,
                "role_name": role_name.map(|r| r.0)
            })),
            ip_address: None,
            trace_id: None,
        }).await;

        Ok(())
    }

    pub async fn revoke_role(
        &self,
        user_id: &str,
        role_id: &str,
        actor_id: &str,
    ) -> Result<(), AppError> {
        sqlx::query("DELETE FROM user_roles WHERE user_id = ? AND role_id = ?")
            .bind(user_id)
            .bind(role_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        self.perm_cache.invalidate().await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        self.audit.log(AuditEntry {
            user_id: Some(actor_id.to_string()),
            action: actions::ROLE_REVOKED.to_string(),
            resource_type: "user_role".to_string(),
            resource_id: Some(user_id.to_string()),
            org_id: None,
            details: Some(serde_json::json!({"role_id": role_id})),
            ip_address: None,
            trace_id: None,
        }).await;

        Ok(())
    }

    pub async fn assign_scope(
        &self,
        user_id: &str,
        req: &AssignScopeRequest,
        actor_id: &str,
    ) -> Result<(), AppError> {
        let scope_id = Uuid::new_v4().to_string();

        sqlx::query(
            "INSERT INTO user_data_scopes (id, user_id, org_id, department_id, project_id, access_level, granted_by)
             VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&scope_id)
        .bind(user_id)
        .bind(&req.org_id)
        .bind(&req.department_id)
        .bind(&req.project_id)
        .bind(&req.access_level)
        .bind(actor_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        self.perm_cache.invalidate().await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        self.audit.log(AuditEntry {
            user_id: Some(actor_id.to_string()),
            action: actions::SCOPE_GRANTED.to_string(),
            resource_type: "user_data_scope".to_string(),
            resource_id: Some(user_id.to_string()),
            org_id: Some(req.org_id.clone()),
            details: Some(serde_json::json!({
                "department_id": &req.department_id,
                "project_id": &req.project_id,
                "access_level": &req.access_level,
            })),
            ip_address: None,
            trace_id: None,
        }).await;

        Ok(())
    }

    pub async fn revoke_scope(
        &self,
        scope_id: &str,
        actor_id: &str,
    ) -> Result<(), AppError> {
        sqlx::query("DELETE FROM user_data_scopes WHERE id = ?")
            .bind(scope_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        self.perm_cache.invalidate().await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        self.audit.log(AuditEntry {
            user_id: Some(actor_id.to_string()),
            action: actions::SCOPE_REVOKED.to_string(),
            resource_type: "user_data_scope".to_string(),
            resource_id: Some(scope_id.to_string()),
            org_id: None,
            details: None,
            ip_address: None,
            trace_id: None,
        }).await;

        Ok(())
    }

    pub async fn get_user_roles(&self, user_id: &str) -> Result<Vec<RoleRow>, AppError> {
        let roles: Vec<RoleRow> = sqlx::query_as(
            "SELECT r.id, r.name, r.description, r.is_system, r.created_at, r.updated_at
             FROM roles r INNER JOIN user_roles ur ON ur.role_id = r.id
             WHERE ur.user_id = ?"
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(roles)
    }

    /// Load a single data-scope row by ID; used for org-boundary checks on revoke.
    pub async fn get_scope(&self, scope_id: &str) -> Result<UserScopeRow, AppError> {
        sqlx::query_as::<_, UserScopeRow>(
            "SELECT id, user_id, org_id, department_id, project_id, access_level, granted_at
             FROM user_data_scopes WHERE id = ?"
        )
        .bind(scope_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("Scope not found".to_string()))
    }

    pub async fn get_user_scopes(&self, user_id: &str) -> Result<Vec<UserScopeRow>, AppError> {
        let scopes: Vec<UserScopeRow> = sqlx::query_as(
            "SELECT uds.id, uds.user_id, uds.org_id, uds.department_id, uds.project_id,
                    uds.access_level, uds.granted_at
             FROM user_data_scopes uds WHERE uds.user_id = ?"
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(scopes)
    }
}

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct UserScopeRow {
    pub id: String,
    pub user_id: String,
    pub org_id: String,
    pub department_id: Option<String>,
    pub project_id: Option<String>,
    pub access_level: String,
    pub granted_at: chrono::NaiveDateTime,
}
