/// Pure ops/admin logic: flag display, toggle descriptions, permission checks.
/// No browser APIs — compiles and tests on native targets.

/// Returns a human-readable description for a known ops flag key.
pub fn flag_description(key: &str) -> &'static str {
    match key {
        "exports_enabled" => "Data export endpoints (POST /api/reports/export)",
        "analytics_enabled" => "Analytics/reporting endpoints (GET /api/reports/*)",
        _ => "Unknown feature flag",
    }
}

/// Returns the display label for a flag's current value.
pub fn flag_value_label(enabled: bool) -> &'static str {
    if enabled { "Enabled" } else { "Disabled" }
}

/// Returns true if the user has ops write permission (can toggle flags).
pub fn can_toggle_flags(permissions: &[&str]) -> bool {
    permissions.contains(&"api.ops.write")
}

/// Returns true if the user has ops read permission (can view metrics/alerts).
pub fn can_view_ops(permissions: &[&str]) -> bool {
    permissions.contains(&"api.ops.read") || permissions.contains(&"api.ops.write")
}

/// Known ops flag keys.
pub const TOGGLE_EXPORTS: &str = "exports_enabled";
pub const TOGGLE_ANALYTICS: &str = "analytics_enabled";

/// Returns true if the given key is a recognized toggle.
pub fn is_known_toggle(key: &str) -> bool {
    matches!(key, "exports_enabled" | "analytics_enabled")
}
