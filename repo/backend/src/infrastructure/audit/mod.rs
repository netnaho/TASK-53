use serde_json::Value as JsonValue;
use sqlx::MySqlPool;
use uuid::Uuid;

/// Immutable audit log service.
/// Records security-sensitive actions into the audit_logs table.
/// Designed to never log passwords, raw encryption keys, or full client identifiers.
#[derive(Clone)]
pub struct AuditService {
    pool: MySqlPool,
}

#[derive(Debug, Clone)]
pub struct AuditEntry {
    pub user_id: Option<String>,
    pub action: String,
    pub resource_type: String,
    pub resource_id: Option<String>,
    pub org_id: Option<String>,
    pub details: Option<JsonValue>,
    pub ip_address: Option<String>,
    pub trace_id: Option<String>,
}

/// Pre-defined audit actions for consistency.
pub mod actions {
    pub const LOGIN_SUCCESS: &str = "auth.login.success";
    pub const LOGIN_FAILED: &str = "auth.login.failed";
    pub const LOGOUT: &str = "auth.logout";
    pub const SESSION_EXPIRED: &str = "auth.session.expired";

    pub const USER_CREATED: &str = "user.created";
    pub const USER_UPDATED: &str = "user.updated";
    pub const USER_DEACTIVATED: &str = "user.deactivated";
    pub const PASSWORD_CHANGED: &str = "user.password.changed";

    pub const ROLE_CREATED: &str = "role.created";
    pub const ROLE_UPDATED: &str = "role.updated";
    pub const ROLE_DELETED: &str = "role.deleted";
    pub const ROLE_ASSIGNED: &str = "role.assigned";
    pub const ROLE_REVOKED: &str = "role.revoked";

    pub const PERMISSION_GRANTED: &str = "permission.granted";
    pub const PERMISSION_REVOKED: &str = "permission.revoked";

    pub const SCOPE_GRANTED: &str = "scope.granted";
    pub const SCOPE_REVOKED: &str = "scope.revoked";

    pub const ORG_CREATED: &str = "org.created";
    pub const ORG_UPDATED: &str = "org.updated";
    pub const DEPT_CREATED: &str = "department.created";
    pub const DEPT_UPDATED: &str = "department.updated";
    pub const PROJECT_CREATED: &str = "project.created";
    pub const PROJECT_UPDATED: &str = "project.updated";

    pub const CONFIG_CHANGED: &str = "config.changed";
}

impl AuditService {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    /// Log an audit entry. This is fire-and-forget for the caller;
    /// failures are logged but do not block the calling operation.
    pub async fn log(&self, entry: AuditEntry) {
        let id = Uuid::new_v4().to_string();
        let details_str = entry.details.map(|d| d.to_string());

        let result = sqlx::query(
            "INSERT INTO audit_logs (id, user_id, action, resource_type, resource_id, org_id, details, ip_address, trace_id)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&id)
        .bind(&entry.user_id)
        .bind(&entry.action)
        .bind(&entry.resource_type)
        .bind(&entry.resource_id)
        .bind(&entry.org_id)
        .bind(&details_str)
        .bind(&entry.ip_address)
        .bind(&entry.trace_id)
        .execute(&self.pool)
        .await;

        if let Err(e) = result {
            tracing::error!(error = %e, action = %entry.action, "Failed to write audit log");
        }
    }

    /// Query audit logs with filtering and pagination.
    pub async fn query(
        &self,
        org_id: Option<&str>,
        user_id: Option<&str>,
        action_prefix: Option<&str>,
        resource_type: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<AuditLogRow>, sqlx::Error> {
        // CAST details (JSON column) to CHAR so sqlx decodes it as Option<String>
        // matching AuditLogRow.details. The JSON stays valid and can be parsed
        // client-side if needed.
        let mut query = String::from(
            "SELECT id, timestamp, user_id, action, resource_type, resource_id, org_id,
                    CAST(details AS CHAR) AS details, ip_address, trace_id
             FROM audit_logs WHERE 1=1"
        );
        let mut binds: Vec<String> = Vec::new();

        if let Some(oid) = org_id {
            query.push_str(" AND org_id = ?");
            binds.push(oid.to_string());
        }
        if let Some(uid) = user_id {
            query.push_str(" AND user_id = ?");
            binds.push(uid.to_string());
        }
        if let Some(prefix) = action_prefix {
            query.push_str(" AND action LIKE ?");
            binds.push(format!("{}%", prefix));
        }
        if let Some(rt) = resource_type {
            query.push_str(" AND resource_type = ?");
            binds.push(rt.to_string());
        }

        query.push_str(" ORDER BY timestamp DESC LIMIT ? OFFSET ?");

        let mut q = sqlx::query_as::<_, AuditLogRow>(&query);
        for b in &binds {
            q = q.bind(b);
        }
        q = q.bind(limit).bind(offset);

        q.fetch_all(&self.pool).await
    }
}

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct AuditLogRow {
    pub id: String,
    pub timestamp: chrono::NaiveDateTime,
    pub user_id: Option<String>,
    pub action: String,
    pub resource_type: String,
    pub resource_id: Option<String>,
    pub org_id: Option<String>,
    pub details: Option<String>,
    pub ip_address: Option<String>,
    pub trace_id: Option<String>,
}
