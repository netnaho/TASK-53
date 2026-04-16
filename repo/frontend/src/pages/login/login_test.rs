// Dedicated unit tests for login page logic.
//
// Tests validate_login_input and map_login_error — pure functions that
// contain no browser APIs and run with `cargo test` on any target.

use super::{map_login_error, validate_login_input};

// ---------------------------------------------------------------------------
// validate_login_input
// ---------------------------------------------------------------------------

#[test]
fn both_fields_empty_returns_error() {
    let result = validate_login_input("", "");
    assert!(result.is_some());
    assert!(result.unwrap().contains("required"));
}

#[test]
fn username_empty_returns_error() {
    let result = validate_login_input("", "secret123");
    assert!(result.is_some(), "empty username should fail validation");
}

#[test]
fn password_empty_returns_error() {
    let result = validate_login_input("admin", "");
    assert!(result.is_some(), "empty password should fail validation");
}

#[test]
fn whitespace_only_username_returns_error() {
    let result = validate_login_input("   ", "password");
    assert!(result.is_some(), "whitespace-only username should fail");
}

#[test]
fn valid_credentials_returns_none() {
    let result = validate_login_input("admin", "Admin123!");
    assert!(result.is_none(), "valid credentials should pass");
}

#[test]
fn valid_credentials_all_demo_accounts() {
    let pairs = [
        ("admin", "Admin123!"),
        ("ops_manager", "OpsManager123!"),
        ("billing_staff", "Billing123!"),
        ("coach", "Coach123!"),
        ("qa_reviewer", "QAReview123!"),
        ("auditor", "Auditor123!"),
    ];
    for (u, p) in &pairs {
        assert!(
            validate_login_input(u, p).is_none(),
            "Demo account {} should pass validation",
            u
        );
    }
}

// ---------------------------------------------------------------------------
// map_login_error
// ---------------------------------------------------------------------------

#[test]
fn http_401_maps_to_invalid_credentials_message() {
    let msg = map_login_error("HTTP 401: Unauthorized");
    assert_eq!(msg, "Invalid username or password");
}

#[test]
fn http_401_with_body_maps_to_invalid_credentials() {
    let msg = map_login_error("HTTP 401: {\"error\":{\"code\":\"UNAUTHORIZED\"}}");
    assert_eq!(msg, "Invalid username or password");
}

#[test]
fn connection_error_maps_to_generic_login_failed() {
    let msg = map_login_error("connection refused");
    assert!(msg.starts_with("Login failed:"), "non-401 errors should say 'Login failed:'");
    assert!(msg.contains("connection refused"));
}

#[test]
fn http_500_maps_to_generic_login_failed() {
    let msg = map_login_error("HTTP 500: Internal Server Error");
    assert!(msg.starts_with("Login failed:"));
}

#[test]
fn http_403_maps_to_generic_login_failed() {
    // 403 is not an auth failure — maps to generic
    let msg = map_login_error("HTTP 403: Forbidden");
    assert!(msg.starts_with("Login failed:"));
    assert!(!msg.eq("Invalid username or password"));
}
