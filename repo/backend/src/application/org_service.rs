use sqlx::MySqlPool;
use uuid::Uuid;

use crate::domain::auth_types::*;
use crate::domain::error::AppError;
use crate::infrastructure::audit::{AuditEntry, AuditService, actions};

#[derive(Clone)]
pub struct OrgService {
    pool: MySqlPool,
    audit: AuditService,
}

impl OrgService {
    pub fn new(pool: MySqlPool, audit: AuditService) -> Self {
        Self { pool, audit }
    }

    // --- Organizations ---

    pub async fn list_orgs(&self) -> Result<Vec<OrgRow>, AppError> {
        sqlx::query_as::<_, OrgRow>(
            "SELECT id, name, status, created_at, updated_at FROM organizations ORDER BY name"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))
    }

    pub async fn get_org(&self, org_id: &str) -> Result<OrgRow, AppError> {
        sqlx::query_as::<_, OrgRow>(
            "SELECT id, name, status, created_at, updated_at FROM organizations WHERE id = ?"
        )
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("Organization not found".to_string()))
    }

    pub async fn create_org(&self, req: &CreateOrgRequest, actor_id: &str) -> Result<OrgRow, AppError> {
        let org_id = Uuid::new_v4().to_string();

        sqlx::query("INSERT INTO organizations (id, name, status) VALUES (?, ?, 'active')")
            .bind(&org_id)
            .bind(&req.name)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        self.audit.log(AuditEntry {
            user_id: Some(actor_id.to_string()),
            action: actions::ORG_CREATED.to_string(),
            resource_type: "organization".to_string(),
            resource_id: Some(org_id.clone()),
            org_id: Some(org_id.clone()),
            details: Some(serde_json::json!({"name": &req.name})),
            ip_address: None,
            trace_id: None,
        }).await;

        self.get_org(&org_id).await
    }

    pub async fn update_org(
        &self,
        org_id: &str,
        req: &UpdateOrgRequest,
        actor_id: &str,
    ) -> Result<OrgRow, AppError> {
        if let Some(ref name) = req.name {
            sqlx::query("UPDATE organizations SET name = ? WHERE id = ?")
                .bind(name)
                .bind(org_id)
                .execute(&self.pool)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;
        }
        if let Some(ref status) = req.status {
            sqlx::query("UPDATE organizations SET status = ? WHERE id = ?")
                .bind(status)
                .bind(org_id)
                .execute(&self.pool)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;
        }

        self.audit.log(AuditEntry {
            user_id: Some(actor_id.to_string()),
            action: actions::ORG_UPDATED.to_string(),
            resource_type: "organization".to_string(),
            resource_id: Some(org_id.to_string()),
            org_id: Some(org_id.to_string()),
            details: Some(serde_json::json!({"name": &req.name, "status": &req.status})),
            ip_address: None,
            trace_id: None,
        }).await;

        self.get_org(org_id).await
    }

    // --- Departments ---

    pub async fn list_departments(&self, org_id: &str) -> Result<Vec<DepartmentRow>, AppError> {
        sqlx::query_as::<_, DepartmentRow>(
            "SELECT id, org_id, name, status, created_at, updated_at
             FROM departments WHERE org_id = ? ORDER BY name"
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))
    }

    pub async fn create_department(
        &self,
        req: &CreateDepartmentRequest,
        actor_id: &str,
    ) -> Result<DepartmentRow, AppError> {
        let dept_id = Uuid::new_v4().to_string();

        sqlx::query("INSERT INTO departments (id, org_id, name, status) VALUES (?, ?, ?, 'active')")
            .bind(&dept_id)
            .bind(&req.org_id)
            .bind(&req.name)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        self.audit.log(AuditEntry {
            user_id: Some(actor_id.to_string()),
            action: actions::DEPT_CREATED.to_string(),
            resource_type: "department".to_string(),
            resource_id: Some(dept_id.clone()),
            org_id: Some(req.org_id.clone()),
            details: Some(serde_json::json!({"name": &req.name})),
            ip_address: None,
            trace_id: None,
        }).await;

        sqlx::query_as::<_, DepartmentRow>(
            "SELECT id, org_id, name, status, created_at, updated_at FROM departments WHERE id = ?"
        )
        .bind(&dept_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))
    }

    // --- Projects ---

    pub async fn list_projects(
        &self,
        org_id: &str,
        department_id: Option<&str>,
    ) -> Result<Vec<ProjectRow>, AppError> {
        if let Some(dept_id) = department_id {
            sqlx::query_as::<_, ProjectRow>(
                "SELECT id, org_id, department_id, name, status, created_at, updated_at
                 FROM projects WHERE org_id = ? AND department_id = ? ORDER BY name"
            )
            .bind(org_id)
            .bind(dept_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))
        } else {
            sqlx::query_as::<_, ProjectRow>(
                "SELECT id, org_id, department_id, name, status, created_at, updated_at
                 FROM projects WHERE org_id = ? ORDER BY name"
            )
            .bind(org_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))
        }
    }

    pub async fn create_project(
        &self,
        req: &CreateProjectRequest,
        actor_id: &str,
    ) -> Result<ProjectRow, AppError> {
        let project_id = Uuid::new_v4().to_string();

        sqlx::query(
            "INSERT INTO projects (id, org_id, department_id, name, status) VALUES (?, ?, ?, ?, 'active')"
        )
        .bind(&project_id)
        .bind(&req.org_id)
        .bind(&req.department_id)
        .bind(&req.name)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        self.audit.log(AuditEntry {
            user_id: Some(actor_id.to_string()),
            action: actions::PROJECT_CREATED.to_string(),
            resource_type: "project".to_string(),
            resource_id: Some(project_id.clone()),
            org_id: Some(req.org_id.clone()),
            details: Some(serde_json::json!({"name": &req.name, "department_id": &req.department_id})),
            ip_address: None,
            trace_id: None,
        }).await;

        sqlx::query_as::<_, ProjectRow>(
            "SELECT id, org_id, department_id, name, status, created_at, updated_at FROM projects WHERE id = ?"
        )
        .bind(&project_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))
    }
}
