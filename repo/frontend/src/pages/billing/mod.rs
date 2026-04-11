/// Billing page: tabbed interface covering Charges, Invoices, Payments & Refunds,
/// and Reconciliation.  Permission-aware: Billing Specialists see all write actions;
/// Auditors see read-only views.

use dioxus::prelude::*;

use crate::models::{
    ChargeRow, InvoiceRow, PaginatedInvoices, PaginatedPayments, ReconciliationRunRow,
};
use crate::services::api_client::ApiClient;
use crate::state::{perms, AuthState};

// Active tab state
#[derive(Debug, Clone, PartialEq)]
enum BillingTab {
    Charges,
    Invoices,
    Payments,
    Reconciliation,
}

#[component]
pub fn Billing() -> Element {
    let auth = use_context::<Signal<AuthState>>();
    let mut active_tab = use_signal(|| BillingTab::Invoices);

    let can_generate = auth.read().has_permission("action.billing.generate");
    let can_approve = auth.read().has_permission("action.billing.approve");
    let can_record_payment = auth.read().has_permission("action.payments.record");
    let can_refund = auth.read().has_permission("action.payments.refund");
    let can_billing_read = auth.read().has_permission("api.billing.read");

    if auth.read().token.is_none() {
        return rsx! {
            div { class: "page-header",
                h1 { "Billing" }
                p { style: "color: var(--color-text-secondary);",
                    "Your session has expired. Please log in again to access billing."
                }
            }
        };
    }

    if !can_billing_read {
        return rsx! {
            div { class: "page-header",
                h1 { "Billing" }
                p { style: "color: var(--color-text-secondary);",
                    "You do not have permission to view billing data."
                }
            }
        };
    }

    rsx! {
        div {
            div { class: "page-header",
                h1 { "Billing" }
                p { "Generate invoices from verified delivery entries, manage invoice lifecycle, record payments and refunds." }
            }

            // Tab navigation
            div { style: "display: flex; gap: 4px; border-bottom: 2px solid var(--color-border); margin-bottom: 24px;",
                TabButton {
                    label: "Invoices",
                    active: *active_tab.read() == BillingTab::Invoices,
                    onclick: move |_| { *active_tab.write() = BillingTab::Invoices; }
                }
                TabButton {
                    label: "Charges",
                    active: *active_tab.read() == BillingTab::Charges,
                    onclick: move |_| { *active_tab.write() = BillingTab::Charges; }
                }
                TabButton {
                    label: "Payments & Refunds",
                    active: *active_tab.read() == BillingTab::Payments,
                    onclick: move |_| { *active_tab.write() = BillingTab::Payments; }
                }
                TabButton {
                    label: "Reconciliation",
                    active: *active_tab.read() == BillingTab::Reconciliation,
                    onclick: move |_| { *active_tab.write() = BillingTab::Reconciliation; }
                }
            }

            // Tab content
            match *active_tab.read() {
                BillingTab::Invoices => rsx! {
                    InvoicesTab {
                        can_generate: can_generate,
                        can_approve: can_approve,
                    }
                },
                BillingTab::Charges => rsx! {
                    ChargesTab { can_generate: can_generate }
                },
                BillingTab::Payments => rsx! {
                    PaymentsTab {
                        can_record_payment: can_record_payment,
                        can_refund: can_refund,
                    }
                },
                BillingTab::Reconciliation => rsx! {
                    ReconciliationTab { can_generate: can_billing_read }
                },
            }
        }
    }
}

// ============================================================
// Tab button helper
// ============================================================

#[component]
fn TabButton(label: &'static str, active: bool, onclick: EventHandler<MouseEvent>) -> Element {
    let style = if active {
        "padding: 8px 16px; border: none; border-bottom: 2px solid var(--color-primary); \
         background: none; cursor: pointer; color: var(--color-primary); font-weight: 600; margin-bottom: -2px;"
    } else {
        "padding: 8px 16px; border: none; background: none; cursor: pointer; \
         color: var(--color-text-secondary);"
    };
    rsx! {
        button { style: style, onclick: move |e| onclick.call(e), "{label}" }
    }
}

// ============================================================
// Invoices Tab
// ============================================================

#[component]
fn InvoicesTab(can_generate: bool, can_approve: bool) -> Element {
    let auth = use_context::<Signal<AuthState>>();
    let mut show_generate_form = use_signal(|| false);
    let mut show_issue_form = use_signal(|| false);

    // Form fields
    let mut plan_id = use_signal(|| String::new());
    let mut period_start = use_signal(|| String::new());
    let mut period_end = use_signal(|| String::new());
    let mut inv_notes = use_signal(|| String::new());
    let mut error_msg = use_signal(|| String::new());
    let mut success_msg = use_signal(|| String::new());

    let invoices = use_resource(move || {
        let t = auth.read().token.clone();
        async move {
            ApiClient::get::<PaginatedInvoices>("/billing/invoices?limit=50", t.as_deref())
                .await
                .ok()
        }
    });

    let on_generate = {
        let plan_id = plan_id.clone();
        let period_start = period_start.clone();
        let period_end = period_end.clone();
        let inv_notes = inv_notes.clone();
        let mut error_msg = error_msg.clone();
        let mut success_msg = success_msg.clone();
        let mut show_generate_form = show_generate_form.clone();
        move |_| {
            let p = plan_id.read().clone();
            let s = period_start.read().clone();
            let e = period_end.read().clone();
            let n = inv_notes.read().clone();
            if p.is_empty() || s.is_empty() || e.is_empty() {
                *error_msg.write() = "Plan ID, period start, and period end are required.".to_string();
                return;
            }
            let t = auth.read().token.clone();
            spawn(async move {
                let body = serde_json::json!({
                    "plan_id": p,
                    "billing_period_start": s,
                    "billing_period_end": e,
                    "notes": if n.is_empty() { serde_json::Value::Null } else { serde_json::Value::String(n) },
                });
                match ApiClient::post::<serde_json::Value, _>("/billing/invoices/generate", &body, t.as_deref()).await {
                    Ok(_) => {
                        *success_msg.write() = "Invoice generated successfully.".to_string();
                        *error_msg.write() = String::new();
                        *show_generate_form.write() = false;
                    }
                    Err(e) => {
                        *error_msg.write() = e;
                        *success_msg.write() = String::new();
                    }
                }
            });
        }
    };

    rsx! {
        div {
            // Actions bar
            div { style: "display: flex; gap: 12px; margin-bottom: 16px; align-items: center;",
                if can_generate {
                    button {
                        class: "btn btn-primary",
                        onclick: move |_| {
                            let current = *show_generate_form.read();
                            *show_generate_form.write() = !current;
                        },
                        if *show_generate_form.read() { "Cancel" } else { "Generate Invoice" }
                    }
                }
            }

            // Messages
            if !error_msg.read().is_empty() {
                div { class: "alert alert-error", "{error_msg}" }
            }
            if !success_msg.read().is_empty() {
                div { class: "alert alert-success", "{success_msg}" }
            }

            // Generate form
            if *show_generate_form.read() {
                div { class: "card", style: "margin-bottom: 20px;",
                    h3 { style: "margin-bottom: 16px;", "Generate Invoice" }
                    p { style: "color: var(--color-text-secondary); font-size: 13px; margin-bottom: 12px;",
                        "Collects all pending verified charges for the plan within the billing period \
                         and packages them into a draft invoice."
                    }
                    div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                        div {
                            label { style: "display: block; margin-bottom: 4px; font-size: 13px;", "Plan ID *" }
                            input {
                                class: "input",
                                r#type: "text",
                                placeholder: "UUID of client plan",
                                value: "{plan_id}",
                                oninput: move |e| *plan_id.write() = e.value(),
                            }
                        }
                        div {
                            label { style: "display: block; margin-bottom: 4px; font-size: 13px;", "Notes" }
                            input {
                                class: "input",
                                r#type: "text",
                                placeholder: "Optional invoice notes",
                                value: "{inv_notes}",
                                oninput: move |e| *inv_notes.write() = e.value(),
                            }
                        }
                        div {
                            label { style: "display: block; margin-bottom: 4px; font-size: 13px;", "Period Start *" }
                            input {
                                class: "input",
                                r#type: "date",
                                value: "{period_start}",
                                oninput: move |e| *period_start.write() = e.value(),
                            }
                        }
                        div {
                            label { style: "display: block; margin-bottom: 4px; font-size: 13px;", "Period End *" }
                            input {
                                class: "input",
                                r#type: "date",
                                value: "{period_end}",
                                oninput: move |e| *period_end.write() = e.value(),
                            }
                        }
                    }
                    div { style: "margin-top: 16px;",
                        button { class: "btn btn-primary", onclick: on_generate, "Generate" }
                    }
                }
            }

            // Invoice list
            div { class: "card",
                h3 { style: "margin-bottom: 12px;", "Invoices" }
                match &*invoices.read() {
                    Some(Some(resp)) if !resp.data.is_empty() => rsx! {
                        table { style: "width: 100%; border-collapse: collapse;",
                            thead {
                                tr {
                                    for col in ["Invoice #", "Plan", "Period", "Total", "Status", "Generated"] {
                                        th { style: "text-align: left; padding: 8px; border-bottom: 1px solid var(--color-border); font-size: 13px;",
                                            "{col}"
                                        }
                                    }
                                }
                            }
                            tbody {
                                for inv in &resp.data {
                                    InvoiceRowCard { invoice: inv.clone(), can_approve: can_approve }
                                }
                            }
                        }
                        p { style: "color: var(--color-text-secondary); font-size: 13px; margin-top: 8px;",
                            "Total: {resp.total} invoice(s)"
                        }
                    },
                    Some(Some(_)) => rsx! {
                        p { style: "color: var(--color-text-secondary);", "No invoices found." }
                    },
                    Some(None) => rsx! {
                        p { style: "color: var(--color-error);", "Failed to load invoices." }
                    },
                    None => rsx! {
                        p { style: "color: var(--color-text-secondary);", "Loading..." }
                    },
                }
            }
        }
    }
}

#[component]
fn InvoiceRowCard(invoice: InvoiceRow, can_approve: bool) -> Element {
    let status_color = match invoice.status.as_str() {
        "paid" => "#22c55e",
        "partially_paid" => "#f59e0b",
        "draft" => "#94a3b8",
        "issued" => "#3b82f6",
        "voided" => "#ef4444",
        _ => "#94a3b8",
    };

    rsx! {
        tr { style: "border-bottom: 1px solid var(--color-border);",
            td { style: "padding: 8px; font-size: 13px; font-family: monospace;", "{invoice.invoice_number}" }
            td { style: "padding: 8px; font-size: 13px;", "{&invoice.plan_id[..8]}..." }
            td { style: "padding: 8px; font-size: 13px;",
                "{invoice.billing_period_start} – {invoice.billing_period_end}"
            }
            td { style: "padding: 8px; font-size: 13px; font-weight: 600;",
                "${invoice.total_amount:.2}"
            }
            td { style: "padding: 8px;",
                span {
                    style: "font-size: 11px; padding: 2px 8px; border-radius: 10px; background: {status_color}22; color: {status_color}; font-weight: 600;",
                    "{invoice.status}"
                }
            }
            td { style: "padding: 8px; font-size: 12px; color: var(--color-text-secondary);",
                "{&invoice.created_at[..10]}"
            }
        }
    }
}

// ============================================================
// Charges Tab
// ============================================================

#[component]
fn ChargesTab(can_generate: bool) -> Element {
    let auth = use_context::<Signal<AuthState>>();
    let mut plan_id_filter = use_signal(|| String::new());
    let mut status_filter = use_signal(|| String::new());
    let mut error_msg = use_signal(|| String::new());
    let mut success_msg = use_signal(|| String::new());

    // Generate charges form
    let mut show_gen_form = use_signal(|| false);
    let mut gen_plan_id = use_signal(|| String::new());
    let mut gen_from = use_signal(|| String::new());
    let mut gen_to = use_signal(|| String::new());

    let on_gen_charges = {
        let gen_plan_id = gen_plan_id.clone();
        let gen_from = gen_from.clone();
        let gen_to = gen_to.clone();
        let mut error_msg = error_msg.clone();
        let mut success_msg = success_msg.clone();
        let mut show_gen_form = show_gen_form.clone();
        move |_| {
            let p = gen_plan_id.read().clone();
            if p.is_empty() {
                *error_msg.write() = "Plan ID is required.".to_string();
                return;
            }
            let from = gen_from.read().clone();
            let to = gen_to.read().clone();
            let t = auth.read().token.clone();
            spawn(async move {
                let mut body = serde_json::json!({ "plan_id": p });
                if !from.is_empty() { body["from_date"] = serde_json::Value::String(from); }
                if !to.is_empty() { body["to_date"] = serde_json::Value::String(to); }
                match ApiClient::post::<serde_json::Value, _>("/billing/charges/generate", &body, t.as_deref()).await {
                    Ok(resp) => {
                        let generated = resp.get("generated").and_then(|v| v.as_u64()).unwrap_or(0);
                        let skipped = resp.get("skipped").and_then(|v| v.as_u64()).unwrap_or(0);
                        *success_msg.write() = format!(
                            "Charge generation complete: {} generated, {} skipped.",
                            generated, skipped
                        );
                        *error_msg.write() = String::new();
                        *show_gen_form.write() = false;
                    }
                    Err(e) => {
                        *error_msg.write() = e;
                        *success_msg.write() = String::new();
                    }
                }
            });
        }
    };

    let charges = use_resource(move || {
        let t = auth.read().token.clone();
        let plan_filter = plan_id_filter.read().clone();
        let st = status_filter.read().clone();
        async move {
            let mut url = "/billing/charges?limit=50".to_string();
            if !plan_filter.is_empty() {
                url.push_str(&format!("&plan_id={}", plan_filter));
            }
            if !st.is_empty() {
                url.push_str(&format!("&status={}", st));
            }
            ApiClient::get::<serde_json::Value>(&url, t.as_deref())
                .await
                .ok()
        }
    });

    rsx! {
        div {
            div { style: "display: flex; gap: 12px; margin-bottom: 16px;",
                if can_generate {
                    button {
                        class: "btn btn-primary",
                        onclick: move |_| {
                            let cur = *show_gen_form.read();
                            *show_gen_form.write() = !cur;
                        },
                        if *show_gen_form.read() { "Cancel" } else { "Generate Charges" }
                    }
                }
                input {
                    class: "input",
                    style: "width: 240px;",
                    r#type: "text",
                    placeholder: "Filter by Plan ID",
                    value: "{plan_id_filter}",
                    oninput: move |e| *plan_id_filter.write() = e.value(),
                }
                select {
                    class: "input",
                    style: "width: 140px;",
                    onchange: move |e| *status_filter.write() = e.value(),
                    option { value: "", "All statuses" }
                    option { value: "pending", "Pending" }
                    option { value: "adjusted", "Adjusted" }
                    option { value: "invoiced", "Invoiced" }
                    option { value: "voided", "Voided" }
                }
            }

            if !error_msg.read().is_empty() {
                div { class: "alert alert-error", "{error_msg}" }
            }
            if !success_msg.read().is_empty() {
                div { class: "alert alert-success", "{success_msg}" }
            }

            // Generate charges form
            if *show_gen_form.read() {
                div { class: "card", style: "margin-bottom: 20px;",
                    h3 { style: "margin-bottom: 12px;", "Generate Charges from Verified Deliveries" }
                    p { style: "color: var(--color-text-secondary); font-size: 13px; margin-bottom: 12px;",
                        "Creates charge records for all verified delivery entries in the plan that \
                         do not yet have a charge. Skips entries not in 'verified' status."
                    }
                    div { style: "display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 12px;",
                        div {
                            label { style: "display: block; margin-bottom: 4px; font-size: 13px;", "Plan ID *" }
                            input {
                                class: "input",
                                r#type: "text",
                                placeholder: "UUID",
                                value: "{gen_plan_id}",
                                oninput: move |e| *gen_plan_id.write() = e.value(),
                            }
                        }
                        div {
                            label { style: "display: block; margin-bottom: 4px; font-size: 13px;", "From Date" }
                            input {
                                class: "input",
                                r#type: "date",
                                value: "{gen_from}",
                                oninput: move |e| *gen_from.write() = e.value(),
                            }
                        }
                        div {
                            label { style: "display: block; margin-bottom: 4px; font-size: 13px;", "To Date" }
                            input {
                                class: "input",
                                r#type: "date",
                                value: "{gen_to}",
                                oninput: move |e| *gen_to.write() = e.value(),
                            }
                        }
                    }
                    div { style: "margin-top: 16px;",
                        button { class: "btn btn-primary", onclick: on_gen_charges, "Generate" }
                    }
                }
            }

            div { class: "card",
                h3 { style: "margin-bottom: 12px;", "Charges" }
                match &*charges.read() {
                    Some(Some(resp)) => {
                        let rows = resp.get("data").and_then(|v| v.as_array()).cloned().unwrap_or_default();
                        let total = resp.get("total").and_then(|v| v.as_i64()).unwrap_or(0);
                        if rows.is_empty() {
                            rsx! {
                                p { style: "color: var(--color-text-secondary);",
                                    "No charges found. Generate charges from verified delivery entries."
                                }
                            }
                        } else {
                            rsx! {
                                table { style: "width: 100%; border-collapse: collapse;",
                                    thead {
                                        tr {
                                            for col in ["Delivery Date", "Rule Type", "Units", "Rate", "Gross", "Adj", "Net", "Status"] {
                                                th { style: "text-align: left; padding: 8px; border-bottom: 1px solid var(--color-border); font-size: 13px;",
                                                    "{col}"
                                                }
                                            }
                                        }
                                    }
                                    tbody {
                                        for row in &rows {
                                            tr { style: "border-bottom: 1px solid var(--color-border);",
                                                td { style: "padding: 8px; font-size: 13px;",
                                                    "{row[\"delivery_entry_id\"].as_str().map(|s| &s[..8]).unwrap_or(\"-\")}..."
                                                }
                                                td { style: "padding: 8px; font-size: 13px;",
                                                    "{row[\"rule_type\"].as_str().unwrap_or(\"-\")}"
                                                }
                                                td { style: "padding: 8px; font-size: 13px;",
                                                    "{row[\"computed_units\"].as_f64().unwrap_or(0.0):.2}"
                                                }
                                                td { style: "padding: 8px; font-size: 13px;",
                                                    "${row[\"rate_applied\"].as_f64().unwrap_or(0.0):.2}"
                                                }
                                                td { style: "padding: 8px; font-size: 13px;",
                                                    "${row[\"gross_amount\"].as_f64().unwrap_or(0.0):.2}"
                                                }
                                                td { style: "padding: 8px; font-size: 13px;",
                                                    "${row[\"adjustment_total\"].as_f64().unwrap_or(0.0):.2}"
                                                }
                                                td { style: "padding: 8px; font-size: 13px; font-weight: 600;",
                                                    "${row[\"net_amount\"].as_f64().unwrap_or(0.0):.2}"
                                                }
                                                td { style: "padding: 8px; font-size: 11px;",
                                                    "{row[\"status\"].as_str().unwrap_or(\"-\")}"
                                                }
                                            }
                                        }
                                    }
                                }
                                p { style: "color: var(--color-text-secondary); font-size: 13px; margin-top: 8px;",
                                    "Total: {total} charge(s)"
                                }
                            }
                        }
                    },
                    _ => rsx! {
                        p { style: "color: var(--color-text-secondary);", "Loading charges..." }
                    },
                }
            }
        }
    }
}

// ============================================================
// Payments & Refunds Tab
// ============================================================

#[component]
fn PaymentsTab(can_record_payment: bool, can_refund: bool) -> Element {
    let auth = use_context::<Signal<AuthState>>();
    let mut show_payment_form = use_signal(|| false);
    let mut show_refund_form = use_signal(|| false);
    let mut error_msg = use_signal(|| String::new());
    let mut success_msg = use_signal(|| String::new());

    // Payment form fields
    let mut pay_invoice_id = use_signal(|| String::new());
    let mut pay_idem_key = use_signal(|| String::new());
    let mut pay_method = use_signal(|| "check".to_string());
    let mut pay_amount = use_signal(|| String::new());
    let mut pay_ref = use_signal(|| String::new());
    let mut pay_date = use_signal(|| String::new());

    // Refund form fields
    let mut ref_invoice_id = use_signal(|| String::new());
    let mut ref_reason_code = use_signal(|| String::new());
    let mut ref_amount = use_signal(|| String::new());
    let mut ref_notes = use_signal(|| String::new());
    let mut ref_method = use_signal(|| "check".to_string());
    let mut ref_date = use_signal(|| String::new());

    let on_record_payment = {
        let pay_invoice_id = pay_invoice_id.clone();
        let pay_idem_key = pay_idem_key.clone();
        let pay_method = pay_method.clone();
        let pay_amount = pay_amount.clone();
        let pay_ref = pay_ref.clone();
        let pay_date = pay_date.clone();
        let mut error_msg = error_msg.clone();
        let mut success_msg = success_msg.clone();
        let mut show_payment_form = show_payment_form.clone();
        move |_| {
            let inv = pay_invoice_id.read().clone();
            let key = pay_idem_key.read().clone();
            let method = pay_method.read().clone();
            let amount_str = pay_amount.read().clone();
            let date = pay_date.read().clone();
            if inv.is_empty() || key.is_empty() || amount_str.is_empty() || date.is_empty() {
                *error_msg.write() = "Invoice ID, idempotency key, amount, and date are required.".to_string();
                return;
            }
            let amount: f64 = match amount_str.parse() {
                Ok(v) => v,
                Err(_) => {
                    *error_msg.write() = "Invalid amount.".to_string();
                    return;
                }
            };
            let ref_num = pay_ref.read().clone();
            let t = auth.read().token.clone();
            spawn(async move {
                let mut body = serde_json::json!({
                    "invoice_id": inv,
                    "idempotency_key": key,
                    "payment_method": method,
                    "amount": amount,
                    "payment_date": date,
                });
                if !ref_num.is_empty() {
                    body["reference_number"] = serde_json::Value::String(ref_num);
                }
                match ApiClient::post::<serde_json::Value, _>("/payments/", &body, t.as_deref()).await {
                    Ok(_) => {
                        *success_msg.write() = "Payment recorded successfully.".to_string();
                        *error_msg.write() = String::new();
                        *show_payment_form.write() = false;
                    }
                    Err(e) => {
                        *error_msg.write() = e;
                        *success_msg.write() = String::new();
                    }
                }
            });
        }
    };

    let on_record_refund = {
        let ref_invoice_id = ref_invoice_id.clone();
        let ref_reason_code = ref_reason_code.clone();
        let ref_amount = ref_amount.clone();
        let ref_notes = ref_notes.clone();
        let ref_method = ref_method.clone();
        let ref_date = ref_date.clone();
        let mut error_msg = error_msg.clone();
        let mut success_msg = success_msg.clone();
        let mut show_refund_form = show_refund_form.clone();
        move |_| {
            let inv = ref_invoice_id.read().clone();
            let code = ref_reason_code.read().clone();
            let amount_str = ref_amount.read().clone();
            let date = ref_date.read().clone();
            if inv.is_empty() || code.is_empty() || amount_str.is_empty() || date.is_empty() {
                *error_msg.write() = "Invoice ID, reason code, amount, and date are required.".to_string();
                return;
            }
            let amount: f64 = match amount_str.parse() {
                Ok(v) => v,
                Err(_) => {
                    *error_msg.write() = "Invalid amount.".to_string();
                    return;
                }
            };
            let notes = ref_notes.read().clone();
            let method = ref_method.read().clone();
            let t = auth.read().token.clone();
            spawn(async move {
                let mut body = serde_json::json!({
                    "invoice_id": inv,
                    "reason_code": code,
                    "amount": amount,
                    "refund_method": method,
                    "refund_date": date,
                });
                if !notes.is_empty() {
                    body["reason_notes"] = serde_json::Value::String(notes);
                }
                match ApiClient::post::<serde_json::Value, _>("/payments/refunds", &body, t.as_deref()).await {
                    Ok(_) => {
                        *success_msg.write() = "Refund recorded successfully.".to_string();
                        *error_msg.write() = String::new();
                        *show_refund_form.write() = false;
                    }
                    Err(e) => {
                        *error_msg.write() = e;
                        *success_msg.write() = String::new();
                    }
                }
            });
        }
    };

    let payments = use_resource(move || {
        let t = auth.read().token.clone();
        async move {
            ApiClient::get::<serde_json::Value>("/payments/?limit=30", t.as_deref())
                .await
                .ok()
        }
    });

    rsx! {
        div {
            div { style: "display: flex; gap: 12px; margin-bottom: 16px;",
                if can_record_payment {
                    button {
                        class: "btn btn-primary",
                        onclick: move |_| {
                            let cur = *show_payment_form.read();
                            *show_payment_form.write() = !cur;
                            if !cur { *show_refund_form.write() = false; }
                        },
                        if *show_payment_form.read() { "Cancel" } else { "Record Payment" }
                    }
                }
                if can_refund {
                    button {
                        class: "btn",
                        style: "border-color: var(--color-border);",
                        onclick: move |_| {
                            let cur = *show_refund_form.read();
                            *show_refund_form.write() = !cur;
                            if !cur { *show_payment_form.write() = false; }
                        },
                        if *show_refund_form.read() { "Cancel" } else { "Process Refund" }
                    }
                }
            }

            if !error_msg.read().is_empty() {
                div { class: "alert alert-error", "{error_msg}" }
            }
            if !success_msg.read().is_empty() {
                div { class: "alert alert-success", "{success_msg}" }
            }

            // Payment form
            if *show_payment_form.read() {
                div { class: "card", style: "margin-bottom: 20px;",
                    h3 { style: "margin-bottom: 4px;", "Record Payment" }
                    p { style: "color: var(--color-text-secondary); font-size: 13px; margin-bottom: 12px;",
                        "The idempotency key prevents duplicate submissions. \
                         The same key used within 5 minutes will be rejected with 409 Conflict."
                    }
                    div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                        div {
                            label { style: "display: block; margin-bottom: 4px; font-size: 13px;", "Invoice ID *" }
                            input {
                                class: "input",
                                r#type: "text",
                                value: "{pay_invoice_id}",
                                oninput: move |e| *pay_invoice_id.write() = e.value(),
                            }
                        }
                        div {
                            label { style: "display: block; margin-bottom: 4px; font-size: 13px;", "Idempotency Key *" }
                            input {
                                class: "input",
                                r#type: "text",
                                placeholder: "Unique key for this payment request",
                                value: "{pay_idem_key}",
                                oninput: move |e| *pay_idem_key.write() = e.value(),
                            }
                        }
                        div {
                            label { style: "display: block; margin-bottom: 4px; font-size: 13px;", "Method *" }
                            select {
                                class: "input",
                                onchange: move |e| *pay_method.write() = e.value(),
                                option { value: "check", "Check" }
                                option { value: "ach", "ACH" }
                                option { value: "wire", "Wire" }
                                option { value: "credit_card", "Credit Card" }
                                option { value: "cash", "Cash" }
                                option { value: "other", "Other" }
                            }
                        }
                        div {
                            label { style: "display: block; margin-bottom: 4px; font-size: 13px;", "Amount *" }
                            input {
                                class: "input",
                                r#type: "number",
                                step: "0.01",
                                min: "0.01",
                                value: "{pay_amount}",
                                oninput: move |e| *pay_amount.write() = e.value(),
                            }
                        }
                        div {
                            label { style: "display: block; margin-bottom: 4px; font-size: 13px;", "Reference # (check/wire)" }
                            input {
                                class: "input",
                                r#type: "text",
                                value: "{pay_ref}",
                                oninput: move |e| *pay_ref.write() = e.value(),
                            }
                        }
                        div {
                            label { style: "display: block; margin-bottom: 4px; font-size: 13px;", "Payment Date *" }
                            input {
                                class: "input",
                                r#type: "date",
                                value: "{pay_date}",
                                oninput: move |e| *pay_date.write() = e.value(),
                            }
                        }
                    }
                    div { style: "margin-top: 16px;",
                        button { class: "btn btn-primary", onclick: on_record_payment, "Record Payment" }
                    }
                }
            }

            // Refund form
            if *show_refund_form.read() {
                div { class: "card", style: "margin-bottom: 20px;",
                    h3 { style: "margin-bottom: 4px;", "Process Refund" }
                    p { style: "color: var(--color-text-secondary); font-size: 13px; margin-bottom: 12px;",
                        "Refund amount is capped at net paid amount (total payments − prior refunds). \
                         A reason code is required."
                    }
                    div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                        div {
                            label { style: "display: block; margin-bottom: 4px; font-size: 13px;", "Invoice ID *" }
                            input {
                                class: "input",
                                r#type: "text",
                                value: "{ref_invoice_id}",
                                oninput: move |e| *ref_invoice_id.write() = e.value(),
                            }
                        }
                        div {
                            label { style: "display: block; margin-bottom: 4px; font-size: 13px;", "Reason Code *" }
                            select {
                                class: "input",
                                onchange: move |e| *ref_reason_code.write() = e.value(),
                                option { value: "", "Select reason..." }
                                option { value: "BILLING_ERROR", "Billing Error" }
                                option { value: "SERVICE_NOT_REND", "Service Not Rendered" }
                                option { value: "DUPLICATE_CHARGE", "Duplicate Charge" }
                                option { value: "CONTRACT_CHANGE", "Contract/Rate Change" }
                                option { value: "CLIENT_REQUEST", "Client-Requested Adjustment" }
                                option { value: "PARTIAL_SERVICE", "Partial Service Delivered" }
                                option { value: "QUALITY_ISSUE", "Quality Issue" }
                                option { value: "INSURANCE_ADJ", "Insurance/Payer Adjustment" }
                                option { value: "OTHER", "Other" }
                            }
                        }
                        div {
                            label { style: "display: block; margin-bottom: 4px; font-size: 13px;", "Amount *" }
                            input {
                                class: "input",
                                r#type: "number",
                                step: "0.01",
                                min: "0.01",
                                value: "{ref_amount}",
                                oninput: move |e| *ref_amount.write() = e.value(),
                            }
                        }
                        div {
                            label { style: "display: block; margin-bottom: 4px; font-size: 13px;", "Method *" }
                            select {
                                class: "input",
                                onchange: move |e| *ref_method.write() = e.value(),
                                option { value: "check", "Check" }
                                option { value: "ach", "ACH" }
                                option { value: "wire", "Wire" }
                                option { value: "credit_card", "Credit Card" }
                                option { value: "cash", "Cash" }
                                option { value: "other", "Other" }
                            }
                        }
                        div {
                            label { style: "display: block; margin-bottom: 4px; font-size: 13px;", "Refund Date *" }
                            input {
                                class: "input",
                                r#type: "date",
                                value: "{ref_date}",
                                oninput: move |e| *ref_date.write() = e.value(),
                            }
                        }
                        div {
                            label { style: "display: block; margin-bottom: 4px; font-size: 13px;", "Notes" }
                            input {
                                class: "input",
                                r#type: "text",
                                value: "{ref_notes}",
                                oninput: move |e| *ref_notes.write() = e.value(),
                            }
                        }
                    }
                    div { style: "margin-top: 16px;",
                        button { class: "btn btn-primary", onclick: on_record_refund, "Process Refund" }
                    }
                }
            }

            // Payments list
            div { class: "card",
                h3 { style: "margin-bottom: 12px;", "Recent Payments" }
                match &*payments.read() {
                    Some(Some(resp)) => {
                        let rows = resp.get("data").and_then(|v| v.as_array()).cloned().unwrap_or_default();
                        if rows.is_empty() {
                            rsx! { p { style: "color: var(--color-text-secondary);", "No payments recorded." } }
                        } else {
                            rsx! {
                                table { style: "width: 100%; border-collapse: collapse;",
                                    thead {
                                        tr {
                                            for col in ["Invoice", "Method", "Amount", "Reference", "Date"] {
                                                th { style: "text-align: left; padding: 8px; border-bottom: 1px solid var(--color-border); font-size: 13px;",
                                                    "{col}"
                                                }
                                            }
                                        }
                                    }
                                    tbody {
                                        for row in &rows {
                                            tr { style: "border-bottom: 1px solid var(--color-border);",
                                                td { style: "padding: 8px; font-size: 13px; font-family: monospace;",
                                                    "{row[\"invoice_id\"].as_str().map(|s| &s[..8]).unwrap_or(\"-\")}..."
                                                }
                                                td { style: "padding: 8px; font-size: 13px;",
                                                    "{row[\"payment_method\"].as_str().unwrap_or(\"-\")}"
                                                }
                                                td { style: "padding: 8px; font-size: 13px; font-weight: 600;",
                                                    "${row[\"amount\"].as_f64().unwrap_or(0.0):.2}"
                                                }
                                                td { style: "padding: 8px; font-size: 12px; color: var(--color-text-secondary);",
                                                    "{row[\"reference_number\"].as_str().unwrap_or(\"-\")}"
                                                }
                                                td { style: "padding: 8px; font-size: 13px;",
                                                    "{row[\"payment_date\"].as_str().unwrap_or(\"-\")}"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    },
                    _ => rsx! { p { style: "color: var(--color-text-secondary);", "Loading..." } },
                }
            }
        }
    }
}

// ============================================================
// Reconciliation Tab
// ============================================================

#[component]
fn ReconciliationTab(can_generate: bool) -> Element {
    let auth = use_context::<Signal<AuthState>>();
    let mut show_form = use_signal(|| false);
    let mut period_start = use_signal(|| String::new());
    let mut period_end = use_signal(|| String::new());
    let mut error_msg = use_signal(|| String::new());
    let mut success_msg = use_signal(|| String::new());

    let on_generate = {
        let period_start = period_start.clone();
        let period_end = period_end.clone();
        let mut error_msg = error_msg.clone();
        let mut success_msg = success_msg.clone();
        let mut show_form = show_form.clone();
        move |_| {
            let s = period_start.read().clone();
            let e = period_end.read().clone();
            if s.is_empty() || e.is_empty() {
                *error_msg.write() = "Period start and end are required.".to_string();
                return;
            }
            let t = auth.read().token.clone();
            spawn(async move {
                let body = serde_json::json!({ "period_start": s, "period_end": e });
                match ApiClient::post::<serde_json::Value, _>("/payments/reconciliation", &body, t.as_deref()).await {
                    Ok(resp) => {
                        let balance = resp.get("outstanding_balance").and_then(|v| v.as_f64()).unwrap_or(0.0);
                        *success_msg.write() = format!(
                            "Reconciliation complete. Outstanding balance: ${:.2}", balance
                        );
                        *error_msg.write() = String::new();
                        *show_form.write() = false;
                    }
                    Err(e) => {
                        *error_msg.write() = e;
                        *success_msg.write() = String::new();
                    }
                }
            });
        }
    };

    let runs = use_resource(move || {
        let t = auth.read().token.clone();
        async move {
            ApiClient::get::<serde_json::Value>("/payments/reconciliation?limit=20", t.as_deref())
                .await
                .ok()
        }
    });

    rsx! {
        div {
            div { style: "display: flex; gap: 12px; margin-bottom: 16px;",
                if can_generate {
                    button {
                        class: "btn btn-primary",
                        onclick: move |_| {
                            let cur = *show_form.read();
                            *show_form.write() = !cur;
                        },
                        if *show_form.read() { "Cancel" } else { "Run Reconciliation" }
                    }
                }
            }

            if !error_msg.read().is_empty() {
                div { class: "alert alert-error", "{error_msg}" }
            }
            if !success_msg.read().is_empty() {
                div { class: "alert alert-success", "{success_msg}" }
            }

            if *show_form.read() {
                div { class: "card", style: "margin-bottom: 20px;",
                    h3 { style: "margin-bottom: 4px;", "Generate Reconciliation Snapshot" }
                    p { style: "color: var(--color-text-secondary); font-size: 13px; margin-bottom: 12px;",
                        "Generates an immutable point-in-time summary covering charges, invoiced totals, \
                         payments received, refunds issued, and outstanding balance for the period."
                    }
                    div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                        div {
                            label { style: "display: block; margin-bottom: 4px; font-size: 13px;", "Period Start *" }
                            input {
                                class: "input",
                                r#type: "date",
                                value: "{period_start}",
                                oninput: move |e| *period_start.write() = e.value(),
                            }
                        }
                        div {
                            label { style: "display: block; margin-bottom: 4px; font-size: 13px;", "Period End *" }
                            input {
                                class: "input",
                                r#type: "date",
                                value: "{period_end}",
                                oninput: move |e| *period_end.write() = e.value(),
                            }
                        }
                    }
                    div { style: "margin-top: 16px;",
                        button { class: "btn btn-primary", onclick: on_generate, "Generate" }
                    }
                }
            }

            div { class: "card",
                h3 { style: "margin-bottom: 12px;", "Reconciliation History" }
                match &*runs.read() {
                    Some(Some(resp)) => {
                        let rows = resp.get("data").and_then(|v| v.as_array()).cloned().unwrap_or_default();
                        if rows.is_empty() {
                            rsx! { p { style: "color: var(--color-text-secondary);", "No reconciliation runs yet." } }
                        } else {
                            rsx! {
                                table { style: "width: 100%; border-collapse: collapse;",
                                    thead {
                                        tr {
                                            for col in ["Period", "Invoiced", "Paid", "Refunded", "Net", "Outstanding", "Run At"] {
                                                th { style: "text-align: left; padding: 8px; border-bottom: 1px solid var(--color-border); font-size: 13px;",
                                                    "{col}"
                                                }
                                            }
                                        }
                                    }
                                    tbody {
                                        for row in &rows {
                                            tr { style: "border-bottom: 1px solid var(--color-border);",
                                                td { style: "padding: 8px; font-size: 13px;",
                                                    "{row[\"period_start\"].as_str().unwrap_or(\"-\")} – {row[\"period_end\"].as_str().unwrap_or(\"-\")}"
                                                }
                                                td { style: "padding: 8px; font-size: 13px;",
                                                    "${row[\"total_invoiced\"].as_f64().unwrap_or(0.0):.2}"
                                                }
                                                td { style: "padding: 8px; font-size: 13px;",
                                                    "${row[\"total_paid\"].as_f64().unwrap_or(0.0):.2}"
                                                }
                                                td { style: "padding: 8px; font-size: 13px;",
                                                    "${row[\"total_refunded\"].as_f64().unwrap_or(0.0):.2}"
                                                }
                                                td { style: "padding: 8px; font-size: 13px; font-weight: 600;",
                                                    "${row[\"net_collected\"].as_f64().unwrap_or(0.0):.2}"
                                                }
                                                td { style: "padding: 8px; font-size: 13px; color: var(--color-error);",
                                                    "${row[\"outstanding_balance\"].as_f64().unwrap_or(0.0):.2}"
                                                }
                                                td { style: "padding: 8px; font-size: 12px; color: var(--color-text-secondary);",
                                                    "{row[\"created_at\"].as_str().map(|s| &s[..10]).unwrap_or(\"-\")}"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    },
                    _ => rsx! { p { style: "color: var(--color-text-secondary);", "Loading..." } },
                }
            }
        }
    }
}
