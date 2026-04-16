// Dedicated unit tests for url_utils — API URL construction and HTTP helpers.
// Mirrors the logic used by services/api_client.rs without needing gloo-net.
// Runs with `cargo test --lib` on native targets.

use crate::url_utils::{bearer_header, build_api_url, format_http_error, is_401_error, is_403_error};

// ---------------------------------------------------------------------------
// build_api_url
// ---------------------------------------------------------------------------

#[test]
fn build_api_url_prepends_api_prefix() {
    assert_eq!(build_api_url("/auth/login"), "/api/auth/login");
}

#[test]
fn build_api_url_catalog_list() {
    assert_eq!(build_api_url("/catalog/"), "/api/catalog/");
}

#[test]
fn build_api_url_path_with_id() {
    assert_eq!(build_api_url("/users/abc-123"), "/api/users/abc-123");
}

#[test]
fn build_api_url_nested_path() {
    assert_eq!(build_api_url("/delivery/eid-1/notes"), "/api/delivery/eid-1/notes");
}

#[test]
fn build_api_url_query_params_preserved() {
    assert_eq!(build_api_url("/plans/?status=active"), "/api/plans/?status=active");
}

#[test]
fn build_api_url_empty_path() {
    assert_eq!(build_api_url(""), "/api");
}

// ---------------------------------------------------------------------------
// format_http_error
// ---------------------------------------------------------------------------

#[test]
fn format_http_error_401() {
    assert_eq!(format_http_error(401, "Unauthorized"), "HTTP 401: Unauthorized");
}

#[test]
fn format_http_error_403() {
    assert_eq!(format_http_error(403, "Forbidden"), "HTTP 403: Forbidden");
}

#[test]
fn format_http_error_404() {
    assert_eq!(
        format_http_error(404, r#"{"error":{"code":"NOT_FOUND"}}"#),
        r#"HTTP 404: {"error":{"code":"NOT_FOUND"}}"#
    );
}

#[test]
fn format_http_error_500() {
    assert_eq!(format_http_error(500, "Internal Server Error"), "HTTP 500: Internal Server Error");
}

#[test]
fn format_http_error_empty_body() {
    assert_eq!(format_http_error(503, ""), "HTTP 503: ");
}

// ---------------------------------------------------------------------------
// bearer_header
// ---------------------------------------------------------------------------

#[test]
fn bearer_header_formats_correctly() {
    assert_eq!(bearer_header("mytoken123"), "Bearer mytoken123");
}

#[test]
fn bearer_header_with_jwt_like_token() {
    let token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.payload.signature";
    let header = bearer_header(token);
    assert!(header.starts_with("Bearer "));
    assert!(header.contains(token));
}

#[test]
fn bearer_header_empty_token() {
    assert_eq!(bearer_header(""), "Bearer ");
}

// ---------------------------------------------------------------------------
// is_401_error / is_403_error
// ---------------------------------------------------------------------------

#[test]
fn is_401_error_detects_401_status() {
    assert!(is_401_error("HTTP 401: Unauthorized"));
    assert!(is_401_error("HTTP 401: "));
}

#[test]
fn is_401_error_false_for_403() {
    assert!(!is_401_error("HTTP 403: Forbidden"));
}

#[test]
fn is_401_error_false_for_500() {
    assert!(!is_401_error("HTTP 500: Internal Server Error"));
}

#[test]
fn is_403_error_detects_403_status() {
    assert!(is_403_error("HTTP 403: Forbidden"));
}

#[test]
fn is_403_error_false_for_401() {
    assert!(!is_403_error("HTTP 401: Unauthorized"));
}

// ---------------------------------------------------------------------------
// Cross-function: API client URL + error response simulation
// ---------------------------------------------------------------------------

#[test]
fn login_url_is_correct() {
    let url = build_api_url("/auth/login");
    assert_eq!(url, "/api/auth/login");
}

#[test]
fn login_error_401_detection() {
    let err = format_http_error(401, "Invalid credentials");
    assert!(is_401_error(&err));
    assert!(!is_403_error(&err));
}

#[test]
fn protected_endpoint_403_detection() {
    let err = format_http_error(403, "Missing permission: api.billing.read");
    assert!(is_403_error(&err));
    assert!(!is_401_error(&err));
}
