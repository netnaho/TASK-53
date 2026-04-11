use chrono::NaiveDate;
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::domain::catalog_types::*;
use crate::domain::error::AppError;
use crate::infrastructure::audit::{AuditEntry, AuditService};
use crate::infrastructure::encryption::EncryptionService;

#[derive(Clone)]
pub struct DeliveryService {
    pool: MySqlPool,
    audit: AuditService,
    encryption: EncryptionService,
}

impl DeliveryService {
    pub fn new(pool: MySqlPool, audit: AuditService, encryption: EncryptionService) -> Self {
        Self { pool, audit, encryption }
    }

    pub async fn list_entries(
        &self,
        org_id: &str,
        plan_id: Option<&str>,
        provider_id: Option<&str>,
        status_filter: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<DeliveryEntryRow>, i64), AppError> {
        let mut where_clause = "WHERE d.org_id = ?".to_string();
        let mut binds: Vec<String> = vec![org_id.to_string()];

        if let Some(pid) = plan_id {
            where_clause.push_str(" AND d.plan_id = ?");
            binds.push(pid.to_string());
        }
        if let Some(prov) = provider_id {
            where_clause.push_str(" AND d.provider_id = ?");
            binds.push(prov.to_string());
        }
        if let Some(st) = status_filter {
            where_clause.push_str(" AND d.status = ?");
            binds.push(st.to_string());
        }

        let count_q = format!("SELECT COUNT(*) FROM delivery_entries d {}", where_clause);
        let mut cq = sqlx::query_as::<_, (i64,)>(&count_q);
        for b in &binds { cq = cq.bind(b); }
        let (total,) = cq.fetch_one(&self.pool).await.map_err(|e| AppError::Internal(e.to_string()))?;

        let data_q = format!(
            "SELECT d.id, d.org_id, d.plan_id, d.plan_package_id, d.service_item_id, d.provider_id,
                    d.delivery_date, d.start_time, d.end_time, d.units, d.mileage, d.status,
                    d.verified_by, d.verified_at, d.created_at, d.updated_at
             FROM delivery_entries d {} ORDER BY d.delivery_date DESC, d.created_at DESC LIMIT ? OFFSET ?",
            where_clause
        );
        let mut dq = sqlx::query_as::<_, DeliveryEntryRow>(&data_q);
        for b in &binds { dq = dq.bind(b); }
        dq = dq.bind(limit).bind(offset);

        let rows = dq.fetch_all(&self.pool).await.map_err(|e| AppError::Internal(e.to_string()))?;
        Ok((rows, total))
    }

    pub async fn get_entry(&self, id: &str) -> Result<DeliveryEntryRow, AppError> {
        sqlx::query_as::<_, DeliveryEntryRow>(
            "SELECT id, org_id, plan_id, plan_package_id, service_item_id, provider_id,
                    delivery_date, start_time, end_time, units, mileage, status,
                    verified_by, verified_at, created_at, updated_at
             FROM delivery_entries WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("Delivery entry not found".to_string()))
    }

    pub async fn create_entry(
        &self,
        org_id: &str,
        req: &CreateDeliveryEntryRequest,
        provider_id: &str,
    ) -> Result<DeliveryEntryRow, AppError> {
        // Validate date
        let delivery_date = NaiveDate::parse_from_str(&req.delivery_date, "%Y-%m-%d")
            .map_err(|_| AppError::BadRequest("Invalid delivery_date format (YYYY-MM-DD)".to_string()))?;

        // Validate plan exists and belongs to org
        let plan: Option<(String, String)> = sqlx::query_as(
            "SELECT org_id, status FROM client_plans WHERE id = ?"
        )
        .bind(&req.plan_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        match plan {
            None => return Err(AppError::NotFound("Client plan not found".to_string())),
            Some((plan_org, _)) if plan_org != org_id => {
                return Err(AppError::Forbidden("Plan belongs to a different organization".to_string()));
            }
            Some((_, status)) if status != "active" => {
                return Err(AppError::BadRequest(format!("Plan is not active (status: {})", status)));
            }
            _ => {}
        }

        // Validate plan-package assignment exists
        let pp: Option<(String,)> = sqlx::query_as(
            "SELECT status FROM client_plan_packages WHERE id = ? AND plan_id = ?"
        )
        .bind(&req.plan_package_id)
        .bind(&req.plan_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        match pp {
            None => return Err(AppError::BadRequest("Plan-package assignment not found".to_string())),
            Some((st,)) if st != "active" => {
                return Err(AppError::BadRequest("Plan-package assignment is not active".to_string()));
            }
            _ => {}
        }

        // Validate service item is part of the package
        let rule: Option<(String, String, Option<sqlx::types::Decimal>)> = sqlx::query_as(
            "SELECT prd.rule_type, prd.id, prd.max_units_per_delivery
             FROM package_rule_definitions prd
             INNER JOIN client_plan_packages cpp ON cpp.package_id = prd.package_id
             WHERE cpp.id = ? AND prd.service_item_id = ? AND prd.is_active = 1"
        )
        .bind(&req.plan_package_id)
        .bind(&req.service_item_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        let (rule_type, _rule_id, max_units) = match rule {
            Some(r) => r,
            None => return Err(AppError::BadRequest(
                "Service item is not included in the assigned package".to_string()
            )),
        };

        // Validate units
        if req.units <= 0.0 {
            return Err(AppError::BadRequest("Units must be greater than 0".to_string()));
        }

        // Hourly rules require quarter-hour increments
        if rule_type == "hourly" {
            validate_quarter_hour(req.units).map_err(AppError::BadRequest)?;
        }

        // Check max units per delivery
        if let Some(max) = max_units {
            let max_f: f64 = max.to_string().parse().unwrap_or(f64::MAX);
            if req.units > max_f {
                return Err(AppError::BadRequest(format!(
                    "Units ({}) exceed maximum allowed per delivery ({})", req.units, max_f
                )));
            }
        }

        // Validate mileage
        if let Some(mileage) = req.mileage {
            validate_mileage(mileage).map_err(AppError::BadRequest)?;
        }

        // Encrypt notes
        let notes_enc = req.notes.as_ref()
            .map(|n| self.encryption.encrypt(n))
            .transpose()
            .map_err(|_| AppError::Internal("Failed to encrypt notes".to_string()))?;

        let id = Uuid::new_v4().to_string();

        let start_time = req.start_time.as_ref().map(|t| {
            chrono::NaiveTime::parse_from_str(t, "%H:%M")
                .or_else(|_| chrono::NaiveTime::parse_from_str(t, "%H:%M:%S"))
                .map_err(|_| AppError::BadRequest("Invalid start_time format (HH:MM)".to_string()))
        }).transpose()?;

        let end_time = req.end_time.as_ref().map(|t| {
            chrono::NaiveTime::parse_from_str(t, "%H:%M")
                .or_else(|_| chrono::NaiveTime::parse_from_str(t, "%H:%M:%S"))
                .map_err(|_| AppError::BadRequest("Invalid end_time format (HH:MM)".to_string()))
        }).transpose()?;

        sqlx::query(
            "INSERT INTO delivery_entries
             (id, org_id, plan_id, plan_package_id, service_item_id, provider_id,
              delivery_date, start_time, end_time, units, mileage, notes_enc, status)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'submitted')"
        )
        .bind(&id)
        .bind(org_id)
        .bind(&req.plan_id)
        .bind(&req.plan_package_id)
        .bind(&req.service_item_id)
        .bind(provider_id)
        .bind(delivery_date)
        .bind(start_time)
        .bind(end_time)
        .bind(req.units)
        .bind(req.mileage)
        .bind(&notes_enc)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        self.audit.log(AuditEntry {
            user_id: Some(provider_id.to_string()),
            action: "delivery.entry.created".to_string(),
            resource_type: "delivery_entry".to_string(),
            resource_id: Some(id.clone()),
            org_id: Some(org_id.to_string()),
            details: Some(serde_json::json!({
                "plan_id": &req.plan_id,
                "service_item_id": &req.service_item_id,
                "units": req.units,
                "delivery_date": &req.delivery_date,
            })),
            ip_address: None,
            trace_id: None,
        }).await;

        self.get_entry(&id).await
    }

    pub async fn update_entry(
        &self,
        id: &str,
        req: &UpdateDeliveryEntryRequest,
        actor_id: &str,
    ) -> Result<DeliveryEntryRow, AppError> {
        let current = self.get_entry(id).await?;

        if current.status == "billed" {
            return Err(AppError::BadRequest("Cannot modify a billed delivery entry".to_string()));
        }

        if let Some(ref status) = req.status {
            match status.as_str() {
                "draft" | "submitted" | "verified" | "rejected" => {}
                _ => return Err(AppError::BadRequest(format!("Invalid status: {}", status))),
            }

            if status == "verified" {
                sqlx::query("UPDATE delivery_entries SET status = ?, verified_by = ?, verified_at = NOW() WHERE id = ?")
                    .bind(status).bind(actor_id).bind(id)
                    .execute(&self.pool).await
                    .map_err(|e| AppError::Internal(e.to_string()))?;
            } else {
                sqlx::query("UPDATE delivery_entries SET status = ? WHERE id = ?")
                    .bind(status).bind(id).execute(&self.pool).await
                    .map_err(|e| AppError::Internal(e.to_string()))?;
            }
        }

        if let Some(units) = req.units {
            if units <= 0.0 {
                return Err(AppError::BadRequest("Units must be greater than 0".to_string()));
            }
            sqlx::query("UPDATE delivery_entries SET units = ? WHERE id = ?")
                .bind(units).bind(id).execute(&self.pool).await
                .map_err(|e| AppError::Internal(e.to_string()))?;
        }

        if let Some(mileage) = req.mileage {
            validate_mileage(mileage).map_err(AppError::BadRequest)?;
            sqlx::query("UPDATE delivery_entries SET mileage = ? WHERE id = ?")
                .bind(mileage).bind(id).execute(&self.pool).await
                .map_err(|e| AppError::Internal(e.to_string()))?;
        }

        if let Some(ref notes) = req.notes {
            let enc = self.encryption.encrypt(notes)
                .map_err(|_| AppError::Internal("Failed to encrypt notes".to_string()))?;
            sqlx::query("UPDATE delivery_entries SET notes_enc = ? WHERE id = ?")
                .bind(&enc).bind(id).execute(&self.pool).await
                .map_err(|e| AppError::Internal(e.to_string()))?;
        }

        self.audit.log(AuditEntry {
            user_id: Some(actor_id.to_string()),
            action: "delivery.entry.updated".to_string(),
            resource_type: "delivery_entry".to_string(),
            resource_id: Some(id.to_string()),
            org_id: Some(current.org_id),
            details: Some(serde_json::json!({"status": &req.status})),
            ip_address: None,
            trace_id: None,
        }).await;

        self.get_entry(id).await
    }

    pub async fn create_note(
        &self,
        org_id: &str,
        req: &CreateEligibilityNoteRequest,
        author_id: &str,
    ) -> Result<EligibilityNoteRow, AppError> {
        if req.note.is_empty() {
            return Err(AppError::BadRequest("Note content is required".to_string()));
        }
        if req.plan_id.is_none() && req.delivery_entry_id.is_none() {
            return Err(AppError::BadRequest("Note must be linked to a plan or delivery entry".to_string()));
        }

        let note_enc = self.encryption.encrypt(&req.note)
            .map_err(|_| AppError::Internal("Failed to encrypt note".to_string()))?;

        let note_type = req.note_type.as_deref().unwrap_or("eligibility");
        let id = Uuid::new_v4().to_string();

        sqlx::query(
            "INSERT INTO eligibility_notes (id, org_id, plan_id, delivery_entry_id, author_id, note_enc, note_type)
             VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&id)
        .bind(org_id)
        .bind(&req.plan_id)
        .bind(&req.delivery_entry_id)
        .bind(author_id)
        .bind(&note_enc)
        .bind(note_type)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        sqlx::query_as::<_, EligibilityNoteRow>(
            "SELECT id, org_id, plan_id, delivery_entry_id, author_id, note_type, created_at
             FROM eligibility_notes WHERE id = ?"
        )
        .bind(&id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))
    }

    pub async fn list_notes(
        &self,
        plan_id: Option<&str>,
        delivery_entry_id: Option<&str>,
    ) -> Result<Vec<EligibilityNoteRow>, AppError> {
        if let Some(pid) = plan_id {
            sqlx::query_as::<_, EligibilityNoteRow>(
                "SELECT id, org_id, plan_id, delivery_entry_id, author_id, note_type, created_at
                 FROM eligibility_notes WHERE plan_id = ? ORDER BY created_at DESC"
            )
            .bind(pid)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))
        } else if let Some(did) = delivery_entry_id {
            sqlx::query_as::<_, EligibilityNoteRow>(
                "SELECT id, org_id, plan_id, delivery_entry_id, author_id, note_type, created_at
                 FROM eligibility_notes WHERE delivery_entry_id = ? ORDER BY created_at DESC"
            )
            .bind(did)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))
        } else {
            Ok(vec![])
        }
    }
}
