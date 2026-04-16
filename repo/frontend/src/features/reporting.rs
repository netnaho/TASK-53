/// Pure reporting logic: date range validation and report filter helpers.
/// No browser APIs — compiles and tests on native targets.

/// Validate a reporting date range (YYYY-MM-DD strings).
/// Returns `Err` with a human-readable message if invalid.
pub fn validate_date_range(from: &str, to: &str) -> Result<(), String> {
    if from.is_empty() {
        return Err("Start date is required".to_string());
    }
    if to.is_empty() {
        return Err("End date is required".to_string());
    }
    if from > to {
        return Err(format!(
            "Start date ({}) must not be after end date ({})",
            from, to
        ));
    }
    Ok(())
}

/// Clamp a pagination limit to the API-supported range [1, 200].
pub fn clamp_limit(requested: i64) -> i64 {
    requested.max(1).min(200)
}

/// Default limit for report pagination when none is specified.
pub const DEFAULT_REPORT_LIMIT: i64 = 50;

/// Default offset for pagination.
pub const DEFAULT_REPORT_OFFSET: i64 = 0;

/// Format a report period label for display, e.g. "2024-Q1" from month range.
/// `month` is 1-based (1 = January, 12 = December).
pub fn quarter_label(year: i32, month: u32) -> String {
    let q = ((month - 1) / 3) + 1;
    format!("{}-Q{}", year, q)
}

/// Determine whether the given permission code grants access to unmasked exports.
pub fn can_export_unmasked(permissions: &[&str]) -> bool {
    permissions.contains(&"api.export.unmasked")
}

/// Build the query path for order-volume report with date filters.
pub fn order_volume_path(from: &str, to: &str) -> String {
    format!("/reports/order-volume?from_date={}&to_date={}", from, to)
}
