use chrono::NaiveDate;
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::domain::catalog_types::*;
use crate::domain::error::AppError;
use crate::infrastructure::audit::{AuditEntry, AuditService};
use crate::infrastructure::encryption::EncryptionService;

#[derive(Clone)]
pub struct PlanService {
    pool: MySqlPool,
    audit: AuditService,
    encryption: EncryptionService,
}

impl PlanService {
    pub fn new(pool: MySqlPool, audit: AuditService, encryption: EncryptionService) -> Self {
        Self { pool, audit, encryption }
    }

    pub async fn list_plans(
        &self,
        org_id: &str,
        status_filter: Option<&str>,
    ) -> Result<Vec<ClientPlanRow>, AppError> {
        // Validate status against known values to prevent injection
        if let Some(status) = status_filter {
            match status {
                "draft" | "active" | "paused" | "completed" | "cancelled" => {}
                _ => return Err(AppError::BadRequest(format!("Invalid status filter: {}", status))),
            }
        }

        if let Some(status) = status_filter {
            sqlx::query_as::<_, ClientPlanRow>(
                "SELECT id, org_id, department_id, project_id, client_name, status, start_date, end_date, created_by, created_at, updated_at
                 FROM client_plans WHERE org_id = ? AND status = ? ORDER BY created_at DESC"
            )
            .bind(org_id)
            .bind(status)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))
        } else {
            sqlx::query_as::<_, ClientPlanRow>(
                "SELECT id, org_id, department_id, project_id, client_name, status, start_date, end_date, created_by, created_at, updated_at
                 FROM client_plans WHERE org_id = ? ORDER BY created_at DESC"
            )
            .bind(org_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))
        }
    }

    pub async fn get_plan(&self, id: &str) -> Result<ClientPlanRow, AppError> {
        sqlx::query_as::<_, ClientPlanRow>(
            "SELECT id, org_id, department_id, project_id, client_name, status, start_date, end_date, created_by, created_at, updated_at
             FROM client_plans WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("Client plan not found".to_string()))
    }

    pub async fn create_plan(
        &self,
        org_id: &str,
        req: &CreateClientPlanRequest,
        actor_id: &str,
    ) -> Result<ClientPlanRow, AppError> {
        if req.client_name.is_empty() {
            return Err(AppError::BadRequest("Client name is required".to_string()));
        }

        let start_date = NaiveDate::parse_from_str(&req.start_date, "%Y-%m-%d")
            .map_err(|_| AppError::BadRequest("Invalid start_date format (expected YYYY-MM-DD)".to_string()))?;

        let end_date = req.end_date.as_ref().map(|d| {
            NaiveDate::parse_from_str(d, "%Y-%m-%d")
                .map_err(|_| AppError::BadRequest("Invalid end_date format".to_string()))
        }).transpose()?;

        if let Some(ed) = end_date {
            if ed <= start_date {
                return Err(AppError::BadRequest("end_date must be after start_date".to_string()));
            }
        }

        let id = Uuid::new_v4().to_string();

        // Encrypt sensitive fields
        let client_id_enc = req.client_identifier.as_ref()
            .map(|ci| self.encryption.encrypt(ci))
            .transpose()
            .map_err(|_| AppError::Internal("Failed to encrypt client identifier".to_string()))?;

        let notes_enc = req.notes.as_ref()
            .map(|n| self.encryption.encrypt(n))
            .transpose()
            .map_err(|_| AppError::Internal("Failed to encrypt notes".to_string()))?;

        sqlx::query(
            "INSERT INTO client_plans (id, org_id, department_id, project_id, client_name, client_identifier_enc, start_date, end_date, notes_enc, created_by)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&id)
        .bind(org_id)
        .bind(&req.department_id)
        .bind(&req.project_id)
        .bind(&req.client_name)
        .bind(&client_id_enc)
        .bind(start_date)
        .bind(end_date)
        .bind(&notes_enc)
        .bind(actor_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        self.audit.log(AuditEntry {
            user_id: Some(actor_id.to_string()),
            action: "plan.created".to_string(),
            resource_type: "client_plan".to_string(),
            resource_id: Some(id.clone()),
            org_id: Some(org_id.to_string()),
            details: Some(serde_json::json!({"client_name": &req.client_name})),
            ip_address: None,
            trace_id: None,
        }).await;

        self.get_plan(&id).await
    }

    pub async fn update_plan(
        &self,
        id: &str,
        req: &UpdateClientPlanRequest,
        actor_id: &str,
    ) -> Result<ClientPlanRow, AppError> {
        let current = self.get_plan(id).await?;

        if let Some(ref status) = req.status {
            match status.as_str() {
                "draft" | "active" | "paused" | "completed" | "cancelled" => {}
                _ => return Err(AppError::BadRequest(format!("Invalid status: {}", status))),
            }
            sqlx::query("UPDATE client_plans SET status = ? WHERE id = ?")
                .bind(status).bind(id).execute(&self.pool).await
                .map_err(|e| AppError::Internal(e.to_string()))?;
        }
        if let Some(ref end_date) = req.end_date {
            let ed = NaiveDate::parse_from_str(end_date, "%Y-%m-%d")
                .map_err(|_| AppError::BadRequest("Invalid end_date format".to_string()))?;
            sqlx::query("UPDATE client_plans SET end_date = ? WHERE id = ?")
                .bind(ed).bind(id).execute(&self.pool).await
                .map_err(|e| AppError::Internal(e.to_string()))?;
        }
        if let Some(ref notes) = req.notes {
            let notes_enc = self.encryption.encrypt(notes)
                .map_err(|_| AppError::Internal("Failed to encrypt notes".to_string()))?;
            sqlx::query("UPDATE client_plans SET notes_enc = ? WHERE id = ?")
                .bind(&notes_enc).bind(id).execute(&self.pool).await
                .map_err(|e| AppError::Internal(e.to_string()))?;
        }

        self.audit.log(AuditEntry {
            user_id: Some(actor_id.to_string()),
            action: "plan.updated".to_string(),
            resource_type: "client_plan".to_string(),
            resource_id: Some(id.to_string()),
            org_id: Some(current.org_id),
            details: Some(serde_json::json!({"status": &req.status})),
            ip_address: None,
            trace_id: None,
        }).await;

        self.get_plan(id).await
    }

    pub async fn assign_package(
        &self,
        plan_id: &str,
        req: &AssignPackageRequest,
        actor_id: &str,
    ) -> Result<PlanPackageRow, AppError> {
        let plan = self.get_plan(plan_id).await?;

        // Verify package exists and belongs to same org
        let pkg: Option<(String,)> = sqlx::query_as(
            "SELECT org_id FROM package_definitions WHERE id = ? AND is_active = 1"
        )
        .bind(&req.package_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        match pkg {
            None => return Err(AppError::NotFound("Active package not found".to_string())),
            Some((pkg_org,)) if pkg_org != plan.org_id => {
                return Err(AppError::Forbidden("Package belongs to a different organization".to_string()));
            }
            _ => {}
        }

        let eff_date = NaiveDate::parse_from_str(&req.effective_date, "%Y-%m-%d")
            .map_err(|_| AppError::BadRequest("Invalid effective_date format".to_string()))?;
        let end_date = req.end_date.as_ref().map(|d| {
            NaiveDate::parse_from_str(d, "%Y-%m-%d")
                .map_err(|_| AppError::BadRequest("Invalid end_date format".to_string()))
        }).transpose()?;

        let id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO client_plan_packages (id, plan_id, package_id, effective_date, end_date, assigned_by)
             VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(&id)
        .bind(plan_id)
        .bind(&req.package_id)
        .bind(eff_date)
        .bind(end_date)
        .bind(actor_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        self.audit.log(AuditEntry {
            user_id: Some(actor_id.to_string()),
            action: "plan.package.assigned".to_string(),
            resource_type: "client_plan_package".to_string(),
            resource_id: Some(id.clone()),
            org_id: Some(plan.org_id),
            details: Some(serde_json::json!({"plan_id": plan_id, "package_id": &req.package_id})),
            ip_address: None,
            trace_id: None,
        }).await;

        sqlx::query_as::<_, PlanPackageRow>(
            "SELECT id, plan_id, package_id, effective_date, end_date, status, assigned_by, created_at, updated_at
             FROM client_plan_packages WHERE id = ?"
        )
        .bind(&id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))
    }

    pub async fn get_plan_packages(&self, plan_id: &str) -> Result<Vec<PlanPackageRow>, AppError> {
        sqlx::query_as::<_, PlanPackageRow>(
            "SELECT id, plan_id, package_id, effective_date, end_date, status, assigned_by, created_at, updated_at
             FROM client_plan_packages WHERE plan_id = ? ORDER BY effective_date"
        )
        .bind(plan_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))
    }
}
