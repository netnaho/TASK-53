/// Pure URL construction and HTTP response utilities extracted from
/// `services/api_client.rs` for native-target unit testing.
///
/// These functions contain no browser APIs, no gloo-net, and compile on
/// any target including x86_64 (standard `cargo test --lib`).

const API_BASE: &str = "/api";

/// Build the full API URL for a given path segment.
/// ```
/// assert_eq!(build_api_url("/auth/login"), "/api/auth/login");
/// assert_eq!(build_api_url("/catalog/"), "/api/catalog/");
/// ```
pub fn build_api_url(path: &str) -> String {
    format!("{}{}", API_BASE, path)
}

/// Format an HTTP error response into the string the API client returns.
/// ```
/// assert_eq!(format_http_error(401, "Unauthorized"), "HTTP 401: Unauthorized");
/// ```
pub fn format_http_error(status: u16, body: &str) -> String {
    format!("HTTP {}: {}", status, body)
}

/// Build the Authorization header value from a bearer token.
/// ```
/// assert_eq!(bearer_header("tok123"), "Bearer tok123");
/// ```
pub fn bearer_header(token: &str) -> String {
    format!("Bearer {}", token)
}

/// Check whether an error string produced by the API client indicates a 401.
/// This mirrors the inline check in `login/mod.rs`.
/// ```
/// assert!(is_401_error("HTTP 401: Unauthorized"));
/// assert!(!is_401_error("HTTP 403: Forbidden"));
/// ```
pub fn is_401_error(err: &str) -> bool {
    err.contains("401")
}

/// Check whether an error string indicates a 403.
pub fn is_403_error(err: &str) -> bool {
    err.contains("403")
}

// Tests are declared in lib.rs as a top-level module: `mod url_utils_test;`
