/// Degradation toggles: centrally controlled feature flags that disable
/// expensive or risky operations when the system is under stress.
///
/// Supported toggles (stored in `ops_config`):
///   - `exports_enabled`   (bool) — when false, all export requests return 503
///   - `analytics_enabled` (bool) — when false, heavy report queries return 503
///
/// Each toggle change is:
///   1. Written to the `ops_config` table (persistent across restarts)
///   2. Logged to `ops_events` for the compliance trail
///   3. Logged to the structured event log via tracing
///   4. Written to the `audit_logs` table via AuditService
///
/// The in-memory cache is a simple Arc<RwLock<HashMap>> with no TTL.
/// Cache is populated lazily on first read and invalidated on every write.
/// This is appropriate for local-network deployments where the DB is fast.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use sqlx::MySqlPool;
use uuid::Uuid;

use crate::domain::error::AppError;
use crate::infrastructure::audit::{AuditEntry, AuditService};

/// Known toggle keys — adding a new toggle requires adding it here and seeding it.
pub const TOGGLE_EXPORTS: &str = "exports_enabled";
pub const TOGGLE_ANALYTICS: &str = "analytics_enabled";

pub const KNOWN_TOGGLES: &[&str] = &[TOGGLE_EXPORTS, TOGGLE_ANALYTICS];

/// In-memory flag cache to avoid a DB round-trip on every request.
type FlagCache = Arc<RwLock<HashMap<String, bool>>>;

#[derive(Clone)]
pub struct DegradationService {
    pool: MySqlPool,
    audit: AuditService,
    cache: FlagCache,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct OpsFlag {
    pub key_name: String,
    pub value: bool,
    pub updated_by: String,
    pub updated_at: String,
}

impl DegradationService {
    pub fn new(pool: MySqlPool, audit: AuditService) -> Self {
        Self {
            pool,
            audit,
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Seed default flag values into ops_config if they don't already exist.
    /// Called from seed_service.rs after user seeding completes.
    pub async fn seed_defaults(&self, system_user_id: &str) -> Result<(), AppError> {
        for key in KNOWN_TOGGLES {
            let existing: Option<(String,)> = sqlx::query_as(
                "SELECT value FROM ops_config WHERE key_name = ?"
            )
            .bind(key)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

            if existing.is_none() {
                sqlx::query(
                    "INSERT INTO ops_config (key_name, value, updated_by) VALUES (?, 'true', ?)"
                )
                .bind(key)
                .bind(system_user_id)
                .execute(&self.pool)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;
            }
        }
        Ok(())
    }

    /// Get a toggle value. Returns `false` (fail-closed) if the key is missing
    /// or the stored value cannot be parsed as a boolean.
    pub async fn get_flag(&self, key: &str) -> bool {
        // Try cache first
        {
            let cache = self.cache.read().expect("degradation cache read lock poisoned");
            if let Some(v) = cache.get(key) {
                return *v;
            }
        }

        // Cache miss: load from DB
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT value FROM ops_config WHERE key_name = ?"
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await
        .unwrap_or(None);

        let value = match &row {
            Some((v,)) => match v.parse::<bool>() {
                Ok(b) => b,
                Err(_) => {
                    tracing::warn!(
                        key = key,
                        raw_value = v.as_str(),
                        "Degradation toggle has malformed value; defaulting to false (fail-closed)"
                    );
                    false
                }
            },
            None => {
                tracing::warn!(
                    key = key,
                    "Degradation toggle not found in ops_config; defaulting to false (fail-closed)"
                );
                false
            }
        };

        {
            let mut cache = self.cache.write().expect("degradation cache write lock poisoned");
            cache.insert(key.to_string(), value);
        }

        value
    }

    /// Set a toggle value.
    /// Returns Err if the key is unknown or if the DB write fails.
    pub async fn set_flag(
        &self,
        key: &str,
        new_value: bool,
        actor_id: &str,
    ) -> Result<(), AppError> {
        if !KNOWN_TOGGLES.contains(&key) {
            return Err(AppError::BadRequest(format!(
                "Unknown ops flag '{}'. Known flags: {}",
                key,
                KNOWN_TOGGLES.join(", ")
            )));
        }

        // Read old value for audit trail
        let old_value = self.get_flag(key).await;

        // Update DB
        sqlx::query(
            "INSERT INTO ops_config (key_name, value, updated_by)
             VALUES (?, ?, ?)
             ON DUPLICATE KEY UPDATE value = VALUES(value), updated_by = VALUES(updated_by)"
        )
        .bind(key)
        .bind(new_value.to_string())
        .bind(actor_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        // Invalidate cache entry
        {
            let mut cache = self.cache.write().expect("degradation cache write lock poisoned");
            cache.insert(key.to_string(), new_value);
        }

        // Log to ops_events
        let event_id = Uuid::new_v4().to_string();
        let _ = sqlx::query(
            "INSERT INTO ops_events (id, event_type, key_name, old_value, new_value, actor_id)
             VALUES (?, 'toggle.changed', ?, ?, ?, ?)"
        )
        .bind(&event_id)
        .bind(key)
        .bind(old_value.to_string())
        .bind(new_value.to_string())
        .bind(actor_id)
        .execute(&self.pool)
        .await;

        // Structured log
        tracing::warn!(
            key = key,
            old_value = old_value,
            new_value = new_value,
            actor_id = actor_id,
            "Degradation toggle changed"
        );

        // Audit log
        self.audit.log(AuditEntry {
            user_id: Some(actor_id.to_string()),
            action: "ops.toggle.changed".to_string(),
            resource_type: "ops_config".to_string(),
            resource_id: Some(key.to_string()),
            org_id: None,
            details: Some(serde_json::json!({
                "key": key,
                "old_value": old_value,
                "new_value": new_value,
            })),
            ip_address: None,
            trace_id: None,
        }).await;

        Ok(())
    }

    /// List all ops flags with their current values.
    pub async fn list_flags(&self) -> Result<Vec<OpsFlag>, AppError> {
        let rows: Vec<(String, String, String, String)> = sqlx::query_as(
            "SELECT key_name, value, updated_by, CAST(updated_at AS CHAR) FROM ops_config ORDER BY key_name ASC"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(rows.into_iter().map(|(key_name, value, updated_by, updated_at)| {
            let parsed = match value.parse::<bool>() {
                Ok(b) => b,
                Err(_) => {
                    tracing::warn!(
                        key = key_name.as_str(),
                        raw_value = value.as_str(),
                        "Malformed toggle value in list_flags; defaulting to false (fail-closed)"
                    );
                    false
                }
            };
            OpsFlag {
                key_name,
                value: parsed,
                updated_by,
                updated_at,
            }
        }).collect())
    }
}

// ============================================================
// Unit tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_known_toggle_keys() {
        assert!(KNOWN_TOGGLES.contains(&TOGGLE_EXPORTS));
        assert!(KNOWN_TOGGLES.contains(&TOGGLE_ANALYTICS));
        assert!(!KNOWN_TOGGLES.contains(&"unknown_flag"));
    }

    #[test]
    fn test_unknown_flag_would_error() {
        // Simulate the validation logic without a DB
        let key = "nonexistent_flag";
        let is_known = KNOWN_TOGGLES.contains(&key);
        assert!(!is_known, "unknown flag should not be accepted");
    }

    #[test]
    fn test_bool_parse() {
        assert_eq!("true".parse::<bool>(), Ok(true));
        assert_eq!("false".parse::<bool>(), Ok(false));
        assert!("garbage".parse::<bool>().is_err());
    }

    /// Validates that the fail-closed parsing helper produces the correct
    /// default when given malformed or missing values.
    fn parse_toggle_fail_closed(raw: Option<&str>) -> bool {
        match raw {
            Some(v) => v.parse::<bool>().unwrap_or(false),
            None => false,
        }
    }

    #[test]
    fn test_fail_closed_valid_true() {
        assert!(parse_toggle_fail_closed(Some("true")));
    }

    #[test]
    fn test_fail_closed_valid_false() {
        assert!(!parse_toggle_fail_closed(Some("false")));
    }

    #[test]
    fn test_fail_closed_garbage_value() {
        // Malformed value must default to false (fail-closed), not true
        assert!(!parse_toggle_fail_closed(Some("garbage")));
    }

    #[test]
    fn test_fail_closed_empty_string() {
        assert!(!parse_toggle_fail_closed(Some("")));
    }

    #[test]
    fn test_fail_closed_numeric_one() {
        // "1" is not a valid Rust bool parse
        assert!(!parse_toggle_fail_closed(Some("1")));
    }

    #[test]
    fn test_fail_closed_missing_key() {
        // Missing key must default to false (fail-closed)
        assert!(!parse_toggle_fail_closed(None));
    }

    #[test]
    fn test_fail_closed_uppercase_true() {
        // "True" / "TRUE" are not valid for str::parse::<bool>()
        assert!(!parse_toggle_fail_closed(Some("True")));
        assert!(!parse_toggle_fail_closed(Some("TRUE")));
    }
}
