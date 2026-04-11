use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use sha2::{Digest, Sha256};
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::domain::auth_types::{JwtClaims, LoginRequest, LoginResponse, UserProfile};
use crate::domain::error::AppError;
use crate::infrastructure::audit::{AuditEntry, AuditService, actions};
use crate::infrastructure::permission_cache::PermissionCache;

#[derive(Clone)]
pub struct AuthService {
    pool: MySqlPool,
    jwt_secret: String,
    session_ttl_hours: u64,
    audit: AuditService,
    perm_cache: PermissionCache,
}

impl AuthService {
    pub fn new(
        pool: MySqlPool,
        jwt_secret: String,
        session_ttl_hours: u64,
        audit: AuditService,
        perm_cache: PermissionCache,
    ) -> Self {
        Self { pool, jwt_secret, session_ttl_hours, audit, perm_cache }
    }

    /// Hash a password using Argon2id.
    pub fn hash_password(password: &str) -> Result<String, AppError> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| AppError::Internal(format!("Password hashing failed: {}", e)))?;
        Ok(hash.to_string())
    }

    /// Verify a password against its Argon2 hash.
    pub fn verify_password(password: &str, hash: &str) -> Result<bool, AppError> {
        let parsed_hash = PasswordHash::new(hash)
            .map_err(|e| AppError::Internal(format!("Invalid hash format: {}", e)))?;
        Ok(Argon2::default().verify_password(password.as_bytes(), &parsed_hash).is_ok())
    }

    /// Authenticate a user and issue a JWT session token.
    pub async fn login(
        &self,
        req: &LoginRequest,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> Result<LoginResponse, AppError> {
        // Find user by username
        let user_row: Option<(String, String, Option<String>, String, String, String)> = sqlx::query_as(
            "SELECT u.id, u.org_id, u.department_id, u.username, u.email, u.status
             FROM users u WHERE u.username = ?"
        )
        .bind(&req.username)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        let (user_id, org_id, department_id, username, email, status) = match user_row {
            Some(row) => row,
            None => {
                // Log a SHA-256 prefix of the attempted username instead of the
                // raw value.  This prevents the audit log from becoming a
                // dictionary of valid/invalid usernames while still allowing
                // correlation of repeated attempts with the same input.
                let username_hash = hex::encode(Sha256::digest(req.username.as_bytes()));
                let username_hash_prefix = &username_hash[..16];
                self.audit.log(AuditEntry {
                    user_id: None,
                    action: actions::LOGIN_FAILED.to_string(),
                    resource_type: "auth".to_string(),
                    resource_id: None,
                    org_id: None,
                    details: Some(serde_json::json!({
                        "reason": "user_not_found",
                        "username_hash": username_hash_prefix,
                    })),
                    ip_address: ip_address.clone(),
                    trace_id: None,
                }).await;
                return Err(AppError::Unauthorized("Invalid credentials".to_string()));
            }
        };

        if status != "active" {
            self.audit.log(AuditEntry {
                user_id: Some(user_id.clone()),
                action: actions::LOGIN_FAILED.to_string(),
                resource_type: "auth".to_string(),
                resource_id: Some(user_id.clone()),
                org_id: Some(org_id.clone()),
                details: Some(serde_json::json!({"reason": "account_inactive", "status": &status})),
                ip_address: ip_address.clone(),
                trace_id: None,
            }).await;
            return Err(AppError::Unauthorized("Account is not active".to_string()));
        }

        // Check credentials
        let cred_row: Option<(String, i32, Option<chrono::NaiveDateTime>)> = sqlx::query_as(
            "SELECT password_hash, failed_attempts, locked_until FROM user_credentials WHERE user_id = ?"
        )
        .bind(&user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        let (password_hash, failed_attempts, locked_until) = match cred_row {
            Some(row) => row,
            None => return Err(AppError::Unauthorized("Invalid credentials".to_string())),
        };

        // Check account lockout
        if let Some(locked) = locked_until {
            if locked > Utc::now().naive_utc() {
                return Err(AppError::Unauthorized("Account is temporarily locked".to_string()));
            }
        }

        // Verify password
        if !Self::verify_password(&req.password, &password_hash)? {
            let new_attempts = failed_attempts + 1;
            let lock_until = if new_attempts >= 5 {
                Some(Utc::now().naive_utc() + Duration::minutes(15))
            } else {
                None
            };

            sqlx::query(
                "UPDATE user_credentials SET failed_attempts = ?, locked_until = ? WHERE user_id = ?"
            )
            .bind(new_attempts)
            .bind(lock_until)
            .bind(&user_id)
            .execute(&self.pool)
            .await
            .ok();

            self.audit.log(AuditEntry {
                user_id: Some(user_id.clone()),
                action: actions::LOGIN_FAILED.to_string(),
                resource_type: "auth".to_string(),
                resource_id: Some(user_id.clone()),
                org_id: Some(org_id.clone()),
                details: Some(serde_json::json!({"reason": "bad_password", "attempts": new_attempts})),
                ip_address: ip_address.clone(),
                trace_id: None,
            }).await;

            return Err(AppError::Unauthorized("Invalid credentials".to_string()));
        }

        // Reset failed attempts on success
        sqlx::query("UPDATE user_credentials SET failed_attempts = 0, locked_until = NULL WHERE user_id = ?")
            .bind(&user_id)
            .execute(&self.pool)
            .await
            .ok();

        // Create session
        let session_id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let expires_at = now + Duration::hours(self.session_ttl_hours as i64);

        let claims = JwtClaims {
            sub: user_id.clone(),
            session_id: session_id.clone(),
            org_id: org_id.clone(),
            exp: expires_at.timestamp() as usize,
            iat: now.timestamp() as usize,
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        )
        .map_err(|e| AppError::Internal(format!("Token generation failed: {}", e)))?;

        // Store session with hashed token
        let token_hash = hex::encode(Sha256::digest(token.as_bytes()));
        sqlx::query(
            "INSERT INTO sessions (id, user_id, token_hash, issued_at, expires_at, ip_address, user_agent)
             VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&session_id)
        .bind(&user_id)
        .bind(&token_hash)
        .bind(now.naive_utc())
        .bind(expires_at.naive_utc())
        .bind(&ip_address)
        .bind(&user_agent)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        // Load permissions for the response
        let perms = self.perm_cache.get_permissions(&user_id).await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        let profile = UserProfile {
            id: user_id.clone(),
            org_id: org_id.clone(),
            department_id,
            username,
            email,
            status,
            roles: perms.role_names.into_iter().collect(),
            permissions: perms.permission_codes.into_iter().collect(),
        };

        self.audit.log(AuditEntry {
            user_id: Some(user_id),
            action: actions::LOGIN_SUCCESS.to_string(),
            resource_type: "auth".to_string(),
            resource_id: Some(session_id),
            org_id: Some(org_id),
            details: None,
            ip_address,
            trace_id: None,
        }).await;

        Ok(LoginResponse { token, user: profile })
    }

    /// Validate a JWT token and return the claims.
    pub async fn validate_token(&self, token: &str) -> Result<JwtClaims, AppError> {
        let claims = decode::<JwtClaims>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_bytes()),
            &Validation::default(),
        )
        .map_err(|e| AppError::Unauthorized(format!("Invalid token: {}", e)))?
        .claims;

        // Verify session is not revoked
        let token_hash = hex::encode(Sha256::digest(token.as_bytes()));
        let session: Option<(Option<chrono::NaiveDateTime>,)> = sqlx::query_as(
            "SELECT revoked_at FROM sessions WHERE id = ? AND token_hash = ?"
        )
        .bind(&claims.session_id)
        .bind(&token_hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        match session {
            Some((Some(_revoked),)) => Err(AppError::Unauthorized("Session has been revoked".to_string())),
            Some((None,)) => Ok(claims),
            None => Err(AppError::Unauthorized("Session not found".to_string())),
        }
    }

    /// Invalidate a session (logout).
    pub async fn logout(&self, session_id: &str, user_id: &str) -> Result<(), AppError> {
        sqlx::query("UPDATE sessions SET revoked_at = NOW() WHERE id = ? AND user_id = ?")
            .bind(session_id)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        self.audit.log(AuditEntry {
            user_id: Some(user_id.to_string()),
            action: actions::LOGOUT.to_string(),
            resource_type: "auth".to_string(),
            resource_id: Some(session_id.to_string()),
            org_id: None,
            details: None,
            ip_address: None,
            trace_id: None,
        }).await;

        Ok(())
    }

    /// Get the current user's profile with permissions.
    pub async fn get_current_user(&self, user_id: &str) -> Result<UserProfile, AppError> {
        let row: (String, String, Option<String>, String, String, String) = sqlx::query_as(
            "SELECT id, org_id, department_id, username, email, status FROM users WHERE id = ?"
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::NotFound(format!("User not found: {}", e)))?;

        let perms = self.perm_cache.get_permissions(user_id).await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(UserProfile {
            id: row.0,
            org_id: row.1,
            department_id: row.2,
            username: row.3,
            email: row.4,
            status: row.5,
            roles: perms.role_names.into_iter().collect(),
            permissions: perms.permission_codes.into_iter().collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_hash_and_verify() {
        let password = "SecureP@ss123!";
        let hash = AuthService::hash_password(password).unwrap();
        assert!(hash.starts_with("$argon2"));
        assert!(AuthService::verify_password(password, &hash).unwrap());
        assert!(!AuthService::verify_password("wrong", &hash).unwrap());
    }

    #[test]
    fn test_different_passwords_different_hashes() {
        let hash1 = AuthService::hash_password("password1").unwrap();
        let hash2 = AuthService::hash_password("password1").unwrap();
        // Same password produces different hashes (random salt)
        assert_ne!(hash1, hash2);
    }
}
