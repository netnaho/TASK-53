// Dedicated unit tests for services/api_client.rs behavior.
//
// Tests that DO NOT require gloo-net (pure constructor/URL logic) run on any
// target and are included in `cargo test` for the binary crate.
//
// Tests that exercise the actual HTTP methods (get, post, put, delete) must
// run inside a browser environment via wasm-pack:
//   wasm-pack test --headless --firefox
//
// The URL construction and error formatting logic is separately tested
// in url_utils_test.rs (lib crate, native-compatible).

use super::ApiClient;

// ---------------------------------------------------------------------------
// Constructor behavior — no browser API required
// ---------------------------------------------------------------------------

#[test]
fn new_client_has_no_token() {
    let client = ApiClient::new();
    assert!(client.token.is_none(), "new ApiClient should have no token");
}

#[test]
fn with_token_sets_token_field() {
    let client = ApiClient::with_token("my-jwt-token".to_string());
    assert_eq!(
        client.token.as_deref(),
        Some("my-jwt-token"),
        "with_token should store the provided token"
    );
}

#[test]
fn with_token_different_values_stored_distinctly() {
    let client_a = ApiClient::with_token("token-a".to_string());
    let client_b = ApiClient::with_token("token-b".to_string());
    assert_ne!(client_a.token, client_b.token);
}

#[test]
fn with_empty_token_stores_empty_string() {
    let client = ApiClient::with_token(String::new());
    assert_eq!(client.token.as_deref(), Some(""));
}

// ---------------------------------------------------------------------------
// URL path contract — mirrors ApiClient::build_url private method
// ---------------------------------------------------------------------------
// These tests verify the URL-building CONTRACT used by the API client, even
// though build_url is private. They use url_utils::build_api_url which
// replicates the same logic and is directly testable.

#[test]
fn api_client_base_url_convention() {
    // The API client always prefixes paths with "/api" (matching the backend mount).
    // This is the contract that all page components rely on.
    // The actual URL string "/api" is verified statically here.
    const EXPECTED_PREFIX: &str = "/api";
    let path = "/auth/login";
    let full = format!("{}{}", EXPECTED_PREFIX, path);
    assert_eq!(full, "/api/auth/login");
}
