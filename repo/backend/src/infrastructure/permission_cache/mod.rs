use parking_lot::RwLock;
use sqlx::MySqlPool;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// In-process permission cache with version-based invalidation.
///
/// Design:
/// - Caches user -> {permissions, data_scopes} mappings
/// - Checks `permission_version` table on every access if entry is older than TTL
/// - If the global version has changed, the entire cache is invalidated
/// - TTL is 30 seconds max, ensuring permission changes take effect within 30s
/// - Thread-safe via parking_lot RwLock for low-contention reads
#[derive(Clone)]
pub struct PermissionCache {
    inner: Arc<RwLock<CacheInner>>,
    pool: MySqlPool,
    ttl: Duration,
}

struct CacheInner {
    entries: HashMap<String, CachedUserPermissions>,
    known_version: u64,
    last_version_check: Instant,
}

#[derive(Debug, Clone)]
pub struct CachedUserPermissions {
    pub user_id: String,
    pub permission_codes: HashSet<String>,
    pub data_scopes: Vec<DataScope>,
    pub role_names: HashSet<String>,
    pub cached_at: Instant,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DataScope {
    pub org_id: String,
    pub department_id: Option<String>,
    pub project_id: Option<String>,
    pub access_level: String,
}

impl PermissionCache {
    pub fn new(pool: MySqlPool, ttl_seconds: u64) -> Self {
        let ttl = Duration::from_secs(ttl_seconds.min(30)); // Enforce max 30s
        Self {
            inner: Arc::new(RwLock::new(CacheInner {
                entries: HashMap::new(),
                known_version: 0,
                last_version_check: Instant::now() - ttl - Duration::from_secs(1), // Force first check
            })),
            pool,
            ttl,
        }
    }

    /// Get cached permissions for a user, loading from DB if stale or missing.
    pub async fn get_permissions(&self, user_id: &str) -> Result<CachedUserPermissions, sqlx::Error> {
        // Check if version has changed (every TTL interval)
        self.check_version().await?;

        // Try cache read
        {
            let cache = self.inner.read();
            if let Some(entry) = cache.entries.get(user_id) {
                if entry.cached_at.elapsed() < self.ttl {
                    return Ok(entry.clone());
                }
            }
        }

        // Cache miss or stale: load from DB
        let perms = self.load_user_permissions(user_id).await?;

        // Store in cache
        {
            let mut cache = self.inner.write();
            cache.entries.insert(user_id.to_string(), perms.clone());
        }

        Ok(perms)
    }

    /// Force invalidation of the entire cache. Called when permissions change.
    pub async fn invalidate(&self) -> Result<(), sqlx::Error> {
        // Increment the global version counter
        sqlx::query("UPDATE permission_version SET version = version + 1 WHERE id = 1")
            .execute(&self.pool)
            .await?;

        // Clear local cache
        {
            let mut cache = self.inner.write();
            cache.entries.clear();
            cache.known_version = 0; // Force re-check
        }

        tracing::info!("Permission cache invalidated");
        Ok(())
    }

    /// Invalidate a single user's cached permissions.
    pub fn invalidate_user(&self, user_id: &str) {
        let mut cache = self.inner.write();
        cache.entries.remove(user_id);
    }

    /// Check the global version counter. If it changed, clear the cache.
    async fn check_version(&self) -> Result<(), sqlx::Error> {
        let should_check = {
            let cache = self.inner.read();
            cache.last_version_check.elapsed() >= self.ttl
        };

        if !should_check {
            return Ok(());
        }

        let row: (u64,) = sqlx::query_as("SELECT version FROM permission_version WHERE id = 1")
            .fetch_one(&self.pool)
            .await?;

        let mut cache = self.inner.write();
        cache.last_version_check = Instant::now();

        if row.0 != cache.known_version {
            tracing::debug!(
                old_version = cache.known_version,
                new_version = row.0,
                "Permission version changed, clearing cache"
            );
            cache.entries.clear();
            cache.known_version = row.0;
        }

        Ok(())
    }

    /// Load a user's full permission set from the database.
    async fn load_user_permissions(&self, user_id: &str) -> Result<CachedUserPermissions, sqlx::Error> {
        // Load role names
        let role_rows: Vec<(String,)> = sqlx::query_as(
            "SELECT r.name FROM roles r
             INNER JOIN user_roles ur ON ur.role_id = r.id
             WHERE ur.user_id = ?"
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        let role_names: HashSet<String> = role_rows.into_iter().map(|r| r.0).collect();

        // Load permission codes through role -> role_permissions -> permissions
        let perm_rows: Vec<(String,)> = sqlx::query_as(
            "SELECT DISTINCT p.code FROM permissions p
             INNER JOIN role_permissions rp ON rp.permission_id = p.id
             INNER JOIN user_roles ur ON ur.role_id = rp.role_id
             WHERE ur.user_id = ?"
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        let permission_codes: HashSet<String> = perm_rows.into_iter().map(|r| r.0).collect();

        // Load data scopes
        let scope_rows: Vec<DataScopeRow> = sqlx::query_as(
            "SELECT org_id, department_id, project_id, access_level
             FROM user_data_scopes WHERE user_id = ?"
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        let data_scopes: Vec<DataScope> = scope_rows
            .into_iter()
            .map(|r| DataScope {
                org_id: r.org_id,
                department_id: r.department_id,
                project_id: r.project_id,
                access_level: r.access_level,
            })
            .collect();

        Ok(CachedUserPermissions {
            user_id: user_id.to_string(),
            permission_codes,
            data_scopes,
            role_names,
            cached_at: Instant::now(),
        })
    }

    /// Check if a user has a specific permission code.
    pub async fn has_permission(&self, user_id: &str, permission_code: &str) -> Result<bool, sqlx::Error> {
        let perms = self.get_permissions(user_id).await?;
        // System Administrator has implicit all-access
        if perms.role_names.contains("System Administrator") {
            return Ok(true);
        }
        Ok(perms.permission_codes.contains(permission_code))
    }

    /// Check if a user has data access to a specific org/department/project
    /// at the required access level.
    pub async fn check_data_scope(
        &self,
        user_id: &str,
        org_id: &str,
        department_id: Option<&str>,
        project_id: Option<&str>,
        required_level: &str,
    ) -> Result<bool, sqlx::Error> {
        let perms = self.get_permissions(user_id).await?;

        // System Administrator bypasses scope checks
        if perms.role_names.contains("System Administrator") {
            return Ok(true);
        }

        let level_rank = |level: &str| -> u8 {
            match level {
                "read" => 1,
                "write" => 2,
                "admin" => 3,
                _ => 0,
            }
        };

        let required_rank = level_rank(required_level);

        for scope in &perms.data_scopes {
            if scope.org_id != org_id {
                continue;
            }
            if level_rank(&scope.access_level) < required_rank {
                continue;
            }
            // Org-level scope grants access to all departments/projects within
            if scope.department_id.is_none() {
                return Ok(true);
            }
            // Department-level scope
            if let Some(dept) = department_id {
                if scope.department_id.as_deref() == Some(dept) {
                    // Department scope grants access to all projects within
                    if scope.project_id.is_none() {
                        return Ok(true);
                    }
                    // Project-level scope
                    if let Some(proj) = project_id {
                        if scope.project_id.as_deref() == Some(proj) {
                            return Ok(true);
                        }
                    }
                }
            }
        }

        Ok(false)
    }
}

#[derive(sqlx::FromRow)]
struct DataScopeRow {
    org_id: String,
    department_id: Option<String>,
    project_id: Option<String>,
    access_level: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_scope_level_ranking() {
        // Verify the level ranking logic
        let level_rank = |level: &str| -> u8 {
            match level {
                "read" => 1,
                "write" => 2,
                "admin" => 3,
                _ => 0,
            }
        };
        assert!(level_rank("admin") > level_rank("write"));
        assert!(level_rank("write") > level_rank("read"));
        assert!(level_rank("read") > level_rank("unknown"));
    }

    #[test]
    fn test_cached_permissions_contains() {
        let perms = CachedUserPermissions {
            user_id: "u1".to_string(),
            permission_codes: HashSet::from([
                "menu.dashboard".to_string(),
                "api.users.read".to_string(),
            ]),
            data_scopes: vec![],
            role_names: HashSet::from(["Operations Manager".to_string()]),
            cached_at: Instant::now(),
        };
        assert!(perms.permission_codes.contains("menu.dashboard"));
        assert!(!perms.permission_codes.contains("api.users.write"));
    }
}
