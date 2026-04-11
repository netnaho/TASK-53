use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

// ============================================================
// Service Catalog
// ============================================================

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ServiceItemRow {
    pub id: String,
    pub org_id: String,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub category: String,
    pub unit_type: String,
    pub default_rate: sqlx::types::Decimal,
    pub is_active: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateServiceItemRequest {
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub category: String,
    pub unit_type: String,
    pub default_rate: f64,
}

#[derive(Debug, Deserialize)]
pub struct UpdateServiceItemRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub category: Option<String>,
    pub default_rate: Option<f64>,
    pub is_active: Option<bool>,
}

// ============================================================
// Package Definitions
// ============================================================

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct PackageRow {
    pub id: String,
    pub org_id: String,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub is_active: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreatePackageRequest {
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub rules: Vec<CreatePackageRuleRequest>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePackageRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub is_active: Option<bool>,
}

// ============================================================
// Package Rules
// ============================================================

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct PackageRuleRow {
    pub id: String,
    pub package_id: String,
    pub service_item_id: String,
    pub rule_type: String,
    pub rate: sqlx::types::Decimal,
    pub min_increment: Option<sqlx::types::Decimal>,
    pub tier_config: Option<serde_json::Value>,
    pub max_units_per_delivery: Option<sqlx::types::Decimal>,
    pub max_units_per_period: Option<i32>,
    pub is_active: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierEntry {
    pub up_to: Option<f64>,
    pub rate: f64,
}

#[derive(Debug, Deserialize)]
pub struct CreatePackageRuleRequest {
    pub service_item_id: String,
    pub rule_type: String,
    pub rate: f64,
    pub min_increment: Option<f64>,
    pub tier_config: Option<Vec<TierEntry>>,
    pub max_units_per_delivery: Option<f64>,
    pub max_units_per_period: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct PackageDetail {
    pub package: PackageRow,
    pub rules: Vec<PackageRuleRow>,
}

// ============================================================
// Client Plans
// ============================================================

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ClientPlanRow {
    pub id: String,
    pub org_id: String,
    pub department_id: Option<String>,
    pub project_id: Option<String>,
    pub client_name: String,
    pub status: String,
    pub start_date: chrono::NaiveDate,
    pub end_date: Option<chrono::NaiveDate>,
    pub created_by: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateClientPlanRequest {
    pub client_name: String,
    pub client_identifier: Option<String>,
    pub department_id: Option<String>,
    pub project_id: Option<String>,
    pub start_date: String,
    pub end_date: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateClientPlanRequest {
    pub status: Option<String>,
    pub end_date: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AssignPackageRequest {
    pub package_id: String,
    pub effective_date: String,
    pub end_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct PlanPackageRow {
    pub id: String,
    pub plan_id: String,
    pub package_id: String,
    pub effective_date: chrono::NaiveDate,
    pub end_date: Option<chrono::NaiveDate>,
    pub status: String,
    pub assigned_by: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

// ============================================================
// Delivery Entries
// ============================================================

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct DeliveryEntryRow {
    pub id: String,
    pub org_id: String,
    pub plan_id: String,
    pub plan_package_id: String,
    pub service_item_id: String,
    pub provider_id: String,
    pub delivery_date: chrono::NaiveDate,
    pub start_time: Option<chrono::NaiveTime>,
    pub end_time: Option<chrono::NaiveTime>,
    pub units: sqlx::types::Decimal,
    pub mileage: Option<sqlx::types::Decimal>,
    pub status: String,
    pub verified_by: Option<String>,
    pub verified_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateDeliveryEntryRequest {
    pub plan_id: String,
    pub plan_package_id: String,
    pub service_item_id: String,
    pub delivery_date: String,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub units: f64,
    pub mileage: Option<f64>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateDeliveryEntryRequest {
    pub status: Option<String>,
    pub units: Option<f64>,
    pub mileage: Option<f64>,
    pub notes: Option<String>,
}

// ============================================================
// Eligibility Notes
// ============================================================

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct EligibilityNoteRow {
    pub id: String,
    pub org_id: String,
    pub plan_id: Option<String>,
    pub delivery_entry_id: Option<String>,
    pub author_id: String,
    pub note_type: String,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateEligibilityNoteRequest {
    pub plan_id: Option<String>,
    pub delivery_entry_id: Option<String>,
    pub note: String,
    pub note_type: Option<String>,
}

// ============================================================
// Validation helpers
// ============================================================

/// Validate that a value is in 0.25-hour (quarter-hour) increments.
pub fn validate_quarter_hour(hours: f64) -> Result<(), String> {
    if hours <= 0.0 {
        return Err("Hours must be greater than 0".to_string());
    }
    let remainder = (hours * 4.0).fract();
    if remainder.abs() > 0.001 {
        return Err(format!(
            "Hours must be in 0.25-hour increments (got {}). Valid examples: 0.25, 0.5, 1.0, 1.25",
            hours
        ));
    }
    Ok(())
}

/// Validate mileage cap per delivery visit.
pub fn validate_mileage(mileage: f64) -> Result<(), String> {
    if mileage < 0.0 {
        return Err("Mileage cannot be negative".to_string());
    }
    if mileage > 200.0 {
        return Err(format!(
            "Mileage cannot exceed 200 miles per visit (got {})",
            mileage
        ));
    }
    Ok(())
}

/// Validate a rule type string.
pub fn validate_rule_type(rule_type: &str) -> Result<(), String> {
    match rule_type {
        "per_visit" | "hourly" | "tiered" => Ok(()),
        _ => Err(format!("Invalid rule type: {}. Must be per_visit, hourly, or tiered", rule_type)),
    }
}

/// Validate a service category string.
pub fn validate_category(category: &str) -> Result<(), String> {
    match category {
        "nursing" | "rehab" | "meals" | "companionship" | "transportation" | "other" => Ok(()),
        _ => Err(format!("Invalid category: {}", category)),
    }
}

/// Validate a unit type string.
pub fn validate_unit_type(unit_type: &str) -> Result<(), String> {
    match unit_type {
        "visit" | "hour" | "mile" | "meal" | "session" => Ok(()),
        _ => Err(format!("Invalid unit type: {}", unit_type)),
    }
}

/// Validate tier configuration.
pub fn validate_tier_config(tiers: &[TierEntry]) -> Result<(), String> {
    if tiers.is_empty() {
        return Err("Tiered rule requires at least one tier entry".to_string());
    }
    for (i, tier) in tiers.iter().enumerate() {
        if tier.rate < 0.0 {
            return Err(format!("Tier {} rate cannot be negative", i + 1));
        }
        // Last tier should have up_to = None (unbounded)
        if i == tiers.len() - 1 && tier.up_to.is_some() {
            return Err("Last tier must have up_to as null (unbounded)".to_string());
        }
        if i < tiers.len() - 1 && tier.up_to.is_none() {
            return Err(format!("Tier {} must have an up_to value (only last tier can be unbounded)", i + 1));
        }
    }
    Ok(())
}

/// Validate a package rule request.
pub fn validate_package_rule(rule: &CreatePackageRuleRequest) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    if let Err(e) = validate_rule_type(&rule.rule_type) {
        errors.push(e);
    }

    if rule.rate < 0.0 {
        errors.push("Rate cannot be negative".to_string());
    }

    match rule.rule_type.as_str() {
        "hourly" => {
            if let Some(inc) = rule.min_increment {
                if inc <= 0.0 {
                    errors.push("Minimum increment must be positive".to_string());
                }
            }
        }
        "tiered" => {
            match &rule.tier_config {
                Some(tiers) => {
                    if let Err(e) = validate_tier_config(tiers) {
                        errors.push(e);
                    }
                }
                None => errors.push("Tiered rule requires tier_config".to_string()),
            }
        }
        _ => {}
    }

    if let Some(max) = rule.max_units_per_delivery {
        if max <= 0.0 {
            errors.push("max_units_per_delivery must be positive".to_string());
        }
    }

    if errors.is_empty() { Ok(()) } else { Err(errors) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quarter_hour_valid() {
        assert!(validate_quarter_hour(0.25).is_ok());
        assert!(validate_quarter_hour(0.5).is_ok());
        assert!(validate_quarter_hour(0.75).is_ok());
        assert!(validate_quarter_hour(1.0).is_ok());
        assert!(validate_quarter_hour(1.25).is_ok());
        assert!(validate_quarter_hour(2.5).is_ok());
        assert!(validate_quarter_hour(8.0).is_ok());
    }

    #[test]
    fn test_quarter_hour_invalid() {
        assert!(validate_quarter_hour(0.1).is_err());
        assert!(validate_quarter_hour(0.3).is_err());
        assert!(validate_quarter_hour(1.1).is_err());
        assert!(validate_quarter_hour(0.0).is_err());
        assert!(validate_quarter_hour(-1.0).is_err());
    }

    #[test]
    fn test_mileage_valid() {
        assert!(validate_mileage(0.0).is_ok());
        assert!(validate_mileage(100.0).is_ok());
        assert!(validate_mileage(200.0).is_ok());
    }

    #[test]
    fn test_mileage_invalid() {
        assert!(validate_mileage(200.1).is_err());
        assert!(validate_mileage(500.0).is_err());
        assert!(validate_mileage(-1.0).is_err());
    }

    #[test]
    fn test_tier_config_valid() {
        let tiers = vec![
            TierEntry { up_to: Some(4.0), rate: 50.0 },
            TierEntry { up_to: Some(8.0), rate: 45.0 },
            TierEntry { up_to: None, rate: 40.0 },
        ];
        assert!(validate_tier_config(&tiers).is_ok());
    }

    #[test]
    fn test_tier_config_empty() {
        assert!(validate_tier_config(&[]).is_err());
    }

    #[test]
    fn test_tier_config_last_must_be_unbounded() {
        let tiers = vec![
            TierEntry { up_to: Some(4.0), rate: 50.0 },
            TierEntry { up_to: Some(8.0), rate: 45.0 },
        ];
        assert!(validate_tier_config(&tiers).is_err());
    }

    #[test]
    fn test_rule_validation() {
        let rule = CreatePackageRuleRequest {
            service_item_id: "svc1".to_string(),
            rule_type: "per_visit".to_string(),
            rate: 75.0,
            min_increment: None,
            tier_config: None,
            max_units_per_delivery: None,
            max_units_per_period: None,
        };
        assert!(validate_package_rule(&rule).is_ok());

        let bad_rule = CreatePackageRuleRequest {
            service_item_id: "svc1".to_string(),
            rule_type: "invalid".to_string(),
            rate: -10.0,
            min_increment: None,
            tier_config: None,
            max_units_per_delivery: None,
            max_units_per_period: None,
        };
        let errors = validate_package_rule(&bad_rule).unwrap_err();
        assert!(errors.len() >= 2);
    }
}
