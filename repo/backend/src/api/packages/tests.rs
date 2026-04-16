// Controller-level tests for the packages API layer.
//
// Covers: package/rule request deserialization, package rule validation
// (rule_type, rate, tier config), permission codes, and error mapping.
// No database, no HTTP server — runs with `cargo test --lib`.

use crate::domain::auth_policy::{action, api};
use crate::domain::catalog_types::{
    CreatePackageRequest, CreatePackageRuleRequest, TierEntry,
    UpdatePackageRequest, validate_package_rule, validate_rule_type,
};
use crate::domain::error::AppError;

// ---------------------------------------------------------------------------
// CreatePackageRequest
// ---------------------------------------------------------------------------

#[test]
fn create_package_request_required_fields() {
    let json = r#"{
        "code": "PKG-BASIC",
        "name": "Basic Care Package",
        "rules": []
    }"#;
    let req: CreatePackageRequest =
        serde_json::from_str(json).expect("deserialize CreatePackageRequest");
    assert_eq!(req.code, "PKG-BASIC");
    assert_eq!(req.name, "Basic Care Package");
    assert!(req.description.is_none());
    assert!(req.rules.is_empty());
}

#[test]
fn create_package_request_with_description() {
    let json = r#"{
        "code": "PKG-ADV",
        "name": "Advanced Package",
        "description": "Comprehensive care bundle",
        "rules": []
    }"#;
    let req: CreatePackageRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.description.as_deref(), Some("Comprehensive care bundle"));
}

#[test]
fn create_package_request_missing_code_fails() {
    let json = r#"{"name":"Bad","rules":[]}"#;
    let result: Result<CreatePackageRequest, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

#[test]
fn create_package_request_missing_name_fails() {
    let json = r#"{"code":"PKG-X","rules":[]}"#;
    let result: Result<CreatePackageRequest, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// CreatePackageRuleRequest
// ---------------------------------------------------------------------------

#[test]
fn create_package_rule_per_visit() {
    let json = r#"{
        "service_item_id": "svc-1",
        "rule_type": "per_visit",
        "rate": 75.00
    }"#;
    let rule: CreatePackageRuleRequest = serde_json::from_str(json).unwrap();
    assert_eq!(rule.rule_type, "per_visit");
    assert_eq!(rule.rate, 75.0);
    assert!(rule.min_increment.is_none());
    assert!(rule.tier_config.is_none());
}

#[test]
fn create_package_rule_hourly_with_increment() {
    let json = r#"{
        "service_item_id": "svc-2",
        "rule_type": "hourly",
        "rate": 60.00,
        "min_increment": 0.25,
        "max_units_per_delivery": 8.0
    }"#;
    let rule: CreatePackageRuleRequest = serde_json::from_str(json).unwrap();
    assert_eq!(rule.min_increment, Some(0.25));
    assert_eq!(rule.max_units_per_delivery, Some(8.0));
}

#[test]
fn create_package_rule_tiered_with_config() {
    let json = r#"{
        "service_item_id": "svc-3",
        "rule_type": "tiered",
        "rate": 0.0,
        "tier_config": [
            {"up_to": 4.0, "rate": 55.00},
            {"up_to": null, "rate": 45.00}
        ]
    }"#;
    let rule: CreatePackageRuleRequest = serde_json::from_str(json).unwrap();
    assert!(rule.tier_config.is_some());
    let tiers = rule.tier_config.unwrap();
    assert_eq!(tiers.len(), 2);
    assert_eq!(tiers[0].rate, 55.0);
    assert!(tiers[1].up_to.is_none());
}

// ---------------------------------------------------------------------------
// UpdatePackageRequest
// ---------------------------------------------------------------------------

#[test]
fn update_package_request_all_optional() {
    let json = r#"{}"#;
    let req: UpdatePackageRequest = serde_json::from_str(json).unwrap();
    assert!(req.name.is_none());
    assert!(req.description.is_none());
    assert!(req.is_active.is_none());
}

#[test]
fn update_package_request_deactivate() {
    let json = r#"{"is_active":false}"#;
    let req: UpdatePackageRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.is_active, Some(false));
}

#[test]
fn update_package_request_rename() {
    let json = r#"{"name":"Renamed Package","description":"Updated desc"}"#;
    let req: UpdatePackageRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.name.as_deref(), Some("Renamed Package"));
    assert_eq!(req.description.as_deref(), Some("Updated desc"));
}

// ---------------------------------------------------------------------------
// validate_rule_type (shared with service_catalog controller)
// ---------------------------------------------------------------------------

#[test]
fn per_visit_is_valid_rule_type() {
    assert!(validate_rule_type("per_visit").is_ok());
}

#[test]
fn hourly_is_valid_rule_type() {
    assert!(validate_rule_type("hourly").is_ok());
}

#[test]
fn tiered_is_valid_rule_type() {
    assert!(validate_rule_type("tiered").is_ok());
}

#[test]
fn daily_is_invalid_rule_type() {
    assert!(validate_rule_type("daily").is_err());
}

// ---------------------------------------------------------------------------
// validate_package_rule — composite validator
// ---------------------------------------------------------------------------

#[test]
fn valid_per_visit_rule_passes() {
    let rule = CreatePackageRuleRequest {
        service_item_id: "svc-1".to_string(),
        rule_type: "per_visit".to_string(),
        rate: 75.0,
        min_increment: None,
        tier_config: None,
        max_units_per_delivery: None,
        max_units_per_period: None,
    };
    assert!(validate_package_rule(&rule).is_ok());
}

#[test]
fn negative_rate_fails_validation() {
    let rule = CreatePackageRuleRequest {
        service_item_id: "svc-1".to_string(),
        rule_type: "per_visit".to_string(),
        rate: -5.0,
        min_increment: None,
        tier_config: None,
        max_units_per_delivery: None,
        max_units_per_period: None,
    };
    assert!(validate_package_rule(&rule).is_err());
}

#[test]
fn invalid_rule_type_fails_validation() {
    let rule = CreatePackageRuleRequest {
        service_item_id: "svc-1".to_string(),
        rule_type: "unknown_type".to_string(),
        rate: 50.0,
        min_increment: None,
        tier_config: None,
        max_units_per_delivery: None,
        max_units_per_period: None,
    };
    assert!(validate_package_rule(&rule).is_err());
}

#[test]
fn tiered_rule_requires_tier_config() {
    let rule = CreatePackageRuleRequest {
        service_item_id: "svc-1".to_string(),
        rule_type: "tiered".to_string(),
        rate: 0.0,
        min_increment: None,
        tier_config: None, // missing
        max_units_per_delivery: None,
        max_units_per_period: None,
    };
    // tiered without tiers should fail
    assert!(validate_package_rule(&rule).is_err());
}

// ---------------------------------------------------------------------------
// TierEntry structure
// ---------------------------------------------------------------------------

#[test]
fn tier_entry_with_bounded_up_to() {
    let json = r#"{"up_to":4.0,"rate":55.0}"#;
    let tier: TierEntry = serde_json::from_str(json).unwrap();
    assert_eq!(tier.up_to, Some(4.0));
    assert_eq!(tier.rate, 55.0);
}

#[test]
fn tier_entry_unbounded_last_tier() {
    let json = r#"{"up_to":null,"rate":45.0}"#;
    let tier: TierEntry = serde_json::from_str(json).unwrap();
    assert!(tier.up_to.is_none());
    assert_eq!(tier.rate, 45.0);
}

// ---------------------------------------------------------------------------
// Authorization codes for packages controller
// ---------------------------------------------------------------------------

#[test]
fn catalog_read_permission_is_used_for_packages() {
    assert_eq!(api::CATALOG_READ, "api.catalog.read");
}

#[test]
fn catalog_write_permission_is_used_for_packages() {
    assert_eq!(api::CATALOG_WRITE, "api.catalog.write");
}

#[test]
fn create_package_action_code_is_correct() {
    assert_eq!(action::CREATE_PACKAGE, "action.packages.create");
}

// ---------------------------------------------------------------------------
// Error mapping for packages controller paths
// ---------------------------------------------------------------------------

#[test]
fn package_not_found_maps_to_not_found() {
    let err = AppError::NotFound("Package not found".to_string());
    let env = err.envelope();
    assert_eq!(env.error.code, "NOT_FOUND");
    assert!(env.error.message.contains("Package not found"));
}

#[test]
fn duplicate_package_code_maps_to_conflict() {
    let err = AppError::Conflict("Package code already exists".to_string());
    assert_eq!(err.envelope().error.code, "CONFLICT");
}

#[test]
fn invalid_package_data_maps_to_bad_request() {
    let err = AppError::BadRequest("Invalid rule type".to_string());
    assert_eq!(err.envelope().error.code, "BAD_REQUEST");
}

#[test]
fn forbidden_package_access_maps_to_forbidden() {
    let err = AppError::Forbidden("Data scope check failed".to_string());
    assert_eq!(err.envelope().error.code, "FORBIDDEN");
}
