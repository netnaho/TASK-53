use sqlx::MySqlPool;
use uuid::Uuid;

use crate::domain::catalog_types::*;
use crate::domain::error::AppError;
use crate::infrastructure::audit::{AuditEntry, AuditService};

#[derive(Clone)]
pub struct CatalogService {
    pool: MySqlPool,
    audit: AuditService,
}

impl CatalogService {
    pub fn new(pool: MySqlPool, audit: AuditService) -> Self {
        Self { pool, audit }
    }

    pub async fn list_items(
        &self,
        org_id: &str,
        category: Option<&str>,
        active_only: bool,
    ) -> Result<Vec<ServiceItemRow>, AppError> {
        let mut query = "SELECT id, org_id, code, name, description, category, unit_type, default_rate, is_active, created_at, updated_at FROM service_catalog_items WHERE org_id = ?".to_string();

        if active_only {
            query.push_str(" AND is_active = 1");
        }

        if let Some(cat) = category {
            query.push_str(&format!(" AND category = '{}'", cat.replace('\'', "")));
        }

        query.push_str(" ORDER BY category, name");

        sqlx::query_as::<_, ServiceItemRow>(&query)
            .bind(org_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))
    }

    pub async fn get_item(&self, id: &str) -> Result<ServiceItemRow, AppError> {
        sqlx::query_as::<_, ServiceItemRow>(
            "SELECT id, org_id, code, name, description, category, unit_type, default_rate, is_active, created_at, updated_at
             FROM service_catalog_items WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("Service item not found".to_string()))
    }

    pub async fn create_item(
        &self,
        org_id: &str,
        req: &CreateServiceItemRequest,
        actor_id: &str,
    ) -> Result<ServiceItemRow, AppError> {
        validate_category(&req.category).map_err(AppError::BadRequest)?;
        validate_unit_type(&req.unit_type).map_err(AppError::BadRequest)?;

        if req.code.is_empty() || req.code.len() > 50 {
            return Err(AppError::BadRequest("Code must be 1-50 characters".to_string()));
        }
        if req.name.is_empty() {
            return Err(AppError::BadRequest("Name is required".to_string()));
        }
        if req.default_rate < 0.0 {
            return Err(AppError::BadRequest("Default rate cannot be negative".to_string()));
        }

        // Check code uniqueness within org
        let existing: Option<(String,)> = sqlx::query_as(
            "SELECT id FROM service_catalog_items WHERE org_id = ? AND code = ?"
        )
        .bind(org_id)
        .bind(&req.code)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        if existing.is_some() {
            return Err(AppError::BadRequest(format!("Service code '{}' already exists in this organization", req.code)));
        }

        let id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO service_catalog_items (id, org_id, code, name, description, category, unit_type, default_rate)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&id)
        .bind(org_id)
        .bind(&req.code)
        .bind(&req.name)
        .bind(&req.description)
        .bind(&req.category)
        .bind(&req.unit_type)
        .bind(req.default_rate)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        self.audit.log(AuditEntry {
            user_id: Some(actor_id.to_string()),
            action: "catalog.item.created".to_string(),
            resource_type: "service_catalog_item".to_string(),
            resource_id: Some(id.clone()),
            org_id: Some(org_id.to_string()),
            details: Some(serde_json::json!({"code": &req.code, "name": &req.name, "category": &req.category})),
            ip_address: None,
            trace_id: None,
        }).await;

        self.get_item(&id).await
    }

    pub async fn update_item(
        &self,
        id: &str,
        req: &UpdateServiceItemRequest,
        actor_id: &str,
    ) -> Result<ServiceItemRow, AppError> {
        let current = self.get_item(id).await?;

        if let Some(ref cat) = req.category {
            validate_category(cat).map_err(AppError::BadRequest)?;
            sqlx::query("UPDATE service_catalog_items SET category = ? WHERE id = ?")
                .bind(cat).bind(id).execute(&self.pool).await
                .map_err(|e| AppError::Internal(e.to_string()))?;
        }
        if let Some(ref name) = req.name {
            sqlx::query("UPDATE service_catalog_items SET name = ? WHERE id = ?")
                .bind(name).bind(id).execute(&self.pool).await
                .map_err(|e| AppError::Internal(e.to_string()))?;
        }
        if let Some(ref desc) = req.description {
            sqlx::query("UPDATE service_catalog_items SET description = ? WHERE id = ?")
                .bind(desc).bind(id).execute(&self.pool).await
                .map_err(|e| AppError::Internal(e.to_string()))?;
        }
        if let Some(rate) = req.default_rate {
            if rate < 0.0 {
                return Err(AppError::BadRequest("Rate cannot be negative".to_string()));
            }
            sqlx::query("UPDATE service_catalog_items SET default_rate = ? WHERE id = ?")
                .bind(rate).bind(id).execute(&self.pool).await
                .map_err(|e| AppError::Internal(e.to_string()))?;
        }
        if let Some(active) = req.is_active {
            sqlx::query("UPDATE service_catalog_items SET is_active = ? WHERE id = ?")
                .bind(active).bind(id).execute(&self.pool).await
                .map_err(|e| AppError::Internal(e.to_string()))?;
        }

        self.audit.log(AuditEntry {
            user_id: Some(actor_id.to_string()),
            action: "catalog.item.updated".to_string(),
            resource_type: "service_catalog_item".to_string(),
            resource_id: Some(id.to_string()),
            org_id: Some(current.org_id.clone()),
            details: None,
            ip_address: None,
            trace_id: None,
        }).await;

        self.get_item(id).await
    }
}
