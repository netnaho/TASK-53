/// Pure billing/invoice logic: amount formatting, status labels, payment method display.
/// No browser APIs — compiles and tests on native targets.

/// Format a monetary amount for display (2 decimal places, USD prefix).
pub fn format_amount(amount: f64) -> String {
    format!("${:.2}", amount)
}

/// Returns a human-readable label for an invoice status string.
pub fn invoice_status_label(status: &str) -> &'static str {
    match status {
        "draft" => "Draft",
        "issued" => "Issued",
        "partially_paid" => "Partially Paid",
        "paid" => "Paid",
        "voided" => "Voided",
        _ => "Unknown",
    }
}

/// Returns true if the invoice can still be voided.
pub fn can_void_invoice(status: &str) -> bool {
    matches!(status, "draft" | "issued" | "partially_paid")
}

/// Returns true if the invoice can accept a payment.
pub fn can_record_payment(status: &str) -> bool {
    matches!(status, "issued" | "partially_paid")
}

/// Returns a display-friendly payment method label.
pub fn payment_method_label(method: &str) -> &'static str {
    match method {
        "check" => "Check",
        "ach" => "ACH Transfer",
        "wire" => "Wire Transfer",
        "credit_card" => "Credit Card",
        "cash" => "Cash",
        "other" => "Other",
        _ => "Unknown",
    }
}

/// Validate that a payment amount is positive.
pub fn validate_payment_amount(amount: f64) -> Result<(), String> {
    if amount <= 0.0 {
        Err(format!("Payment amount must be positive (got {})", amount))
    } else {
        Ok(())
    }
}

/// Compute net collected = total_paid - total_refunded.
pub fn compute_net_collected(total_paid: f64, total_refunded: f64) -> f64 {
    total_paid - total_refunded
}
