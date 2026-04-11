use sqlx::MySqlPool;
use uuid::Uuid;

use crate::domain::catalog_types::*;
use crate::domain::error::AppError;
use crate::infrastructure::audit::{AuditEntry, AuditService};

#[derive(Clone)]
pub struct PackageService {
    pool: MySqlPool,
    audit: AuditService,
}

impl PackageService {
    pub fn new(pool: MySqlPool, audit: AuditService) -> Self {
        Self { pool, audit }
    }

    pub async fn list_packages(
        &self,
        org_id: &str,
        active_only: bool,
    ) -> Result<Vec<PackageRow>, AppError> {
        let query = if active_only {
            "SELECT id, org_id, code, name, description, is_active, created_at, updated_at
             FROM package_definitions WHERE org_id = ? AND is_active = 1 ORDER BY name"
        } else {
            "SELECT id, org_id, code, name, description, is_active, created_at, updated_at
             FROM package_definitions WHERE org_id = ? ORDER BY name"
        };

        sqlx::query_as::<_, PackageRow>(query)
            .bind(org_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))
    }

    pub async fn get_package(&self, id: &str) -> Result<PackageRow, AppError> {
        sqlx::query_as::<_, PackageRow>(
            "SELECT id, org_id, code, name, description, is_active, created_at, updated_at
             FROM package_definitions WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("Package not found".to_string()))
    }

    pub async fn get_package_detail(&self, id: &str) -> Result<PackageDetail, AppError> {
        let package = self.get_package(id).await?;
        let rules = self.get_rules(id).await?;
        Ok(PackageDetail { package, rules })
    }

    pub async fn get_rules(&self, package_id: &str) -> Result<Vec<PackageRuleRow>, AppError> {
        sqlx::query_as::<_, PackageRuleRow>(
            "SELECT id, package_id, service_item_id, rule_type, rate, min_increment, tier_config,
                    max_units_per_delivery, max_units_per_period, is_active, created_at, updated_at
             FROM package_rule_definitions WHERE package_id = ? ORDER BY created_at"
        )
        .bind(package_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))
    }

    pub async fn create_package(
        &self,
        org_id: &str,
        req: &CreatePackageRequest,
        actor_id: &str,
    ) -> Result<PackageDetail, AppError> {
        if req.code.is_empty() || req.code.len() > 50 {
            return Err(AppError::BadRequest("Package code must be 1-50 characters".to_string()));
        }
        if req.name.is_empty() {
            return Err(AppError::BadRequest("Package name is required".to_string()));
        }
        if req.rules.is_empty() {
            return Err(AppError::BadRequest("Package must have at least one rule".to_string()));
        }

        // Check code uniqueness
        let existing: Option<(String,)> = sqlx::query_as(
            "SELECT id FROM package_definitions WHERE org_id = ? AND code = ?"
        )
        .bind(org_id)
        .bind(&req.code)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        if existing.is_some() {
            return Err(AppError::BadRequest(format!("Package code '{}' already exists", req.code)));
        }

        // Validate all rules before inserting anything
        for (i, rule) in req.rules.iter().enumerate() {
            if let Err(errors) = validate_package_rule(rule) {
                return Err(AppError::BadRequest(format!("Rule {}: {}", i + 1, errors.join("; "))));
            }
            // Verify service item exists and belongs to this org
            let svc: Option<(String,)> = sqlx::query_as(
                "SELECT org_id FROM service_catalog_items WHERE id = ?"
            )
            .bind(&rule.service_item_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

            match svc {
                None => return Err(AppError::BadRequest(format!("Rule {}: service item not found", i + 1))),
                Some((svc_org,)) if svc_org != org_id => {
                    return Err(AppError::Forbidden("Service item belongs to a different organization".to_string()));
                }
                _ => {}
            }
        }

        // Insert package
        let pkg_id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO package_definitions (id, org_id, code, name, description)
             VALUES (?, ?, ?, ?, ?)"
        )
        .bind(&pkg_id)
        .bind(org_id)
        .bind(&req.code)
        .bind(&req.name)
        .bind(&req.description)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        // Insert rules
        for rule in &req.rules {
            let rule_id = Uuid::new_v4().to_string();
            let tier_json = rule.tier_config.as_ref().map(|t| serde_json::to_string(t).unwrap());
            let min_inc = if rule.rule_type == "hourly" {
                rule.min_increment.or(Some(0.25))
            } else {
                rule.min_increment
            };

            sqlx::query(
                "INSERT INTO package_rule_definitions
                 (id, package_id, service_item_id, rule_type, rate, min_increment, tier_config, max_units_per_delivery, max_units_per_period)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(&rule_id)
            .bind(&pkg_id)
            .bind(&rule.service_item_id)
            .bind(&rule.rule_type)
            .bind(rule.rate)
            .bind(min_inc)
            .bind(&tier_json)
            .bind(rule.max_units_per_delivery)
            .bind(rule.max_units_per_period)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
        }

        self.audit.log(AuditEntry {
            user_id: Some(actor_id.to_string()),
            action: "package.created".to_string(),
            resource_type: "package_definition".to_string(),
            resource_id: Some(pkg_id.clone()),
            org_id: Some(org_id.to_string()),
            details: Some(serde_json::json!({"code": &req.code, "name": &req.name, "rule_count": req.rules.len()})),
            ip_address: None,
            trace_id: None,
        }).await;

        self.get_package_detail(&pkg_id).await
    }

    pub async fn update_package(
        &self,
        id: &str,
        req: &UpdatePackageRequest,
        actor_id: &str,
    ) -> Result<PackageRow, AppError> {
        let current = self.get_package(id).await?;

        if let Some(ref name) = req.name {
            sqlx::query("UPDATE package_definitions SET name = ? WHERE id = ?")
                .bind(name).bind(id).execute(&self.pool).await
                .map_err(|e| AppError::Internal(e.to_string()))?;
        }
        if let Some(ref desc) = req.description {
            sqlx::query("UPDATE package_definitions SET description = ? WHERE id = ?")
                .bind(desc).bind(id).execute(&self.pool).await
                .map_err(|e| AppError::Internal(e.to_string()))?;
        }
        if let Some(active) = req.is_active {
            sqlx::query("UPDATE package_definitions SET is_active = ? WHERE id = ?")
                .bind(active).bind(id).execute(&self.pool).await
                .map_err(|e| AppError::Internal(e.to_string()))?;
        }

        self.audit.log(AuditEntry {
            user_id: Some(actor_id.to_string()),
            action: "package.updated".to_string(),
            resource_type: "package_definition".to_string(),
            resource_id: Some(id.to_string()),
            org_id: Some(current.org_id),
            details: None,
            ip_address: None,
            trace_id: None,
        }).await;

        self.get_package(id).await
    }
}
