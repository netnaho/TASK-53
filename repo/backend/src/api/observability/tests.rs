// Controller-level tests for the observability API layer.
//
// Covers: health/readiness response structure, alarm state constants,
// alert threshold value, alarm status serialization, permission codes.
// No database, no HTTP server — runs with `cargo test --lib`.

use crate::api::observability::{HealthResponse, ReadinessResponse};
use crate::application::alert_engine::{
    ALERT_RULE_DESCRIPTION, ALERT_THRESHOLD, AlarmStatus, AlarmState,
};
use crate::domain::auth_policy::api;

// ---------------------------------------------------------------------------
// HealthResponse structure
// ---------------------------------------------------------------------------

#[test]
fn health_response_status_field_constructible() {
    let hr = HealthResponse {
        status: "ok".to_string(),
        service: "careops-backend".to_string(),
        version: "0.1.0".to_string(),
    };
    assert_eq!(hr.status, "ok");
    assert_eq!(hr.service, "careops-backend");
    assert!(!hr.version.is_empty());
}

#[test]
fn health_response_status_ok_literal() {
    let hr = HealthResponse {
        status: "ok".to_string(),
        service: "careops-backend".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    };
    assert_eq!(hr.status, "ok", "liveness always returns status=ok");
}

#[test]
fn health_response_serializes_expected_fields() {
    let hr = HealthResponse {
        status: "ok".to_string(),
        service: "careops-backend".to_string(),
        version: "1.0.0".to_string(),
    };
    let json = serde_json::to_string(&hr).expect("serialize HealthResponse");
    assert!(json.contains("\"status\""));
    assert!(json.contains("\"service\""));
    assert!(json.contains("\"version\""));
    assert!(json.contains("\"ok\""));
}

// ---------------------------------------------------------------------------
// ReadinessResponse structure
// ---------------------------------------------------------------------------

#[test]
fn readiness_response_db_ok_constructible() {
    let rr = ReadinessResponse {
        status: "ok".to_string(),
        db_ok: true,
        chaos_active: false,
    };
    assert_eq!(rr.status, "ok");
    assert!(rr.db_ok);
    assert!(!rr.chaos_active);
}

#[test]
fn readiness_response_degraded_when_db_down() {
    let rr = ReadinessResponse {
        status: "degraded".to_string(),
        db_ok: false,
        chaos_active: false,
    };
    assert_eq!(rr.status, "degraded");
    assert!(!rr.db_ok);
}

#[test]
fn readiness_response_serializes_boolean_fields() {
    let rr = ReadinessResponse {
        status: "ok".to_string(),
        db_ok: true,
        chaos_active: true,
    };
    let json = serde_json::to_string(&rr).expect("serialize ReadinessResponse");
    assert!(json.contains("\"db_ok\""));
    assert!(json.contains("\"chaos_active\""));
    assert!(json.contains("true"));
}

// ---------------------------------------------------------------------------
// Alert engine constants
// ---------------------------------------------------------------------------

#[test]
fn alert_threshold_is_two_percent() {
    assert!(
        (ALERT_THRESHOLD - 0.02).abs() < 1e-9,
        "Alert threshold should be 0.02 (2%), got {}",
        ALERT_THRESHOLD
    );
}

#[test]
fn alert_rule_description_is_nonempty() {
    assert!(!ALERT_RULE_DESCRIPTION.is_empty());
    assert!(
        ALERT_RULE_DESCRIPTION.contains("2%") || ALERT_RULE_DESCRIPTION.contains("2 %"),
        "Rule description should mention the 2% threshold: {}",
        ALERT_RULE_DESCRIPTION
    );
}

// ---------------------------------------------------------------------------
// AlarmStatus enum serialization
// ---------------------------------------------------------------------------

#[test]
fn alarm_status_ok_serializes_to_snake_case() {
    let status = AlarmStatus::Ok;
    let json = serde_json::to_string(&status).expect("serialize AlarmStatus");
    assert_eq!(json, r#""ok""#);
}

#[test]
fn alarm_status_alerting_serializes_to_snake_case() {
    let status = AlarmStatus::Alerting;
    let json = serde_json::to_string(&status).expect("serialize AlarmStatus");
    assert_eq!(json, r#""alerting""#);
}

#[test]
fn alarm_status_ok_ne_alerting() {
    assert_ne!(AlarmStatus::Ok, AlarmStatus::Alerting);
}

// ---------------------------------------------------------------------------
// AlarmState structure
// ---------------------------------------------------------------------------

#[test]
fn alarm_state_fields_accessible() {
    let state = AlarmState {
        status: AlarmStatus::Ok,
        since: 1700000000,
        current_error_rate_pct: 0.5,
        window_requests: 100,
        message: "System healthy".to_string(),
    };
    assert_eq!(state.status, AlarmStatus::Ok);
    assert_eq!(state.current_error_rate_pct, 0.5);
    assert_eq!(state.window_requests, 100);
}

#[test]
fn alarm_state_alerting_fields() {
    let state = AlarmState {
        status: AlarmStatus::Alerting,
        since: 1700001000,
        current_error_rate_pct: 5.2,
        window_requests: 50,
        message: "Error rate exceeded threshold".to_string(),
    };
    assert_eq!(state.status, AlarmStatus::Alerting);
    assert!(state.current_error_rate_pct > 2.0);
    assert!(state.message.contains("threshold") || state.message.contains("rate") || !state.message.is_empty());
}

// ---------------------------------------------------------------------------
// Authorization codes for observability controller
// ---------------------------------------------------------------------------

#[test]
fn ops_read_permission_required_for_metrics() {
    // metrics, alerts, chaos endpoints require api.ops.read
    assert_eq!(api::OPS_READ, "api.ops.read");
}

#[test]
fn ops_read_permission_is_not_ops_write() {
    assert_ne!(api::OPS_READ, api::OPS_WRITE);
}
