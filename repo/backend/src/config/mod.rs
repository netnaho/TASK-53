use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub database_url: String,
    pub app_env: String,
    pub jwt_secret: String,
    pub encryption_key: String,
    pub session_ttl_hours: u64,
    pub cors_allowed_origins: Vec<String>,
}

impl AppConfig {
    /// Build config from environment variables.
    ///
    /// In non-local environments (`APP_ENV != "local"`), `JWT_SECRET`,
    /// `ENCRYPTION_KEY`, and `DATABASE_URL` are **required** — the process
    /// will panic at startup if any is missing.  In `local` mode only,
    /// insecure development defaults are used as a convenience.
    pub fn from_env() -> Self {
        let app_env = std::env::var("APP_ENV").unwrap_or_else(|_| "local".to_string());
        let is_local = app_env == "local";

        let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            if is_local {
                "mysql://careops_user:careops_pass@db:3306/careops".to_string()
            } else {
                panic!("DATABASE_URL must be set in non-local environments (APP_ENV={})", app_env);
            }
        });

        let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| {
            if is_local {
                tracing::warn!("JWT_SECRET not set — using insecure development default. Do NOT use in production.");
                "careops-dev-jwt-secret-change-in-production-min-32-chars!".to_string()
            } else {
                panic!("JWT_SECRET must be set in non-local environments (APP_ENV={})", app_env);
            }
        });

        let encryption_key = std::env::var("ENCRYPTION_KEY").unwrap_or_else(|_| {
            if is_local {
                tracing::warn!("ENCRYPTION_KEY not set — using insecure development default. Do NOT use in production.");
                "careops-dev-encryption-key-0123456789abcdef".to_string()
            } else {
                panic!("ENCRYPTION_KEY must be set in non-local environments (APP_ENV={})", app_env);
            }
        });

        let cors_allowed_origins = std::env::var("CORS_ALLOWED_ORIGINS")
            .map(|v| v.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect::<Vec<_>>())
            .unwrap_or_else(|_| {
                if is_local {
                    vec![
                        "http://localhost:8080".to_string(),
                        "http://127.0.0.1:8080".to_string(),
                        "http://localhost:8000".to_string(),
                        "http://127.0.0.1:8000".to_string(),
                        "http://localhost:3000".to_string(),
                        "http://127.0.0.1:3000".to_string(),
                    ]
                } else {
                    panic!(
                        "CORS_ALLOWED_ORIGINS must be set in non-local environments (APP_ENV={}). \
                         Provide a comma-separated list of allowed origins.",
                        app_env
                    );
                }
            });

        Self {
            database_url,
            app_env,
            jwt_secret,
            encryption_key,
            session_ttl_hours: std::env::var("SESSION_TTL_HOURS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(24),
            cors_allowed_origins,
        }
    }

    pub fn is_local(&self) -> bool {
        self.app_env == "local"
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_cors_origin_parsing() {
        let raw = "http://localhost:8080, https://app.example.com";
        let origins: Vec<String> = raw.split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        assert_eq!(origins, vec![
            "http://localhost:8080".to_string(),
            "https://app.example.com".to_string(),
        ]);
    }

    #[test]
    fn test_cors_empty_string_filtered() {
        let raw = "http://localhost:8080,,";
        let origins: Vec<String> = raw.split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        assert_eq!(origins, vec!["http://localhost:8080".to_string()]);
    }

    #[test]
    fn test_wildcard_not_in_default_dev_origins() {
        let defaults = vec![
            "http://localhost:8080",
            "http://127.0.0.1:8080",
            "http://localhost:8000",
            "http://127.0.0.1:8000",
        ];
        for origin in &defaults {
            assert_ne!(*origin, "*", "wildcard must not appear in default origins");
        }
    }
}
