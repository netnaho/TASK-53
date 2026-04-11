/// Reports & Exports page: date-range filtered reports with department/region
/// filters, KPI summary cards, and permission-gated export with masking indicator.

use dioxus::prelude::*;

use crate::models::{
    ExportResult, KpiSummary, OrderVolumeRow, RevenueReportRow, UtilizationRow,
};
use crate::services::ApiClient;
use crate::state::AuthState;

#[derive(Debug, Clone, PartialEq)]
enum ReportTab {
    OrderVolume,
    Revenue,
    Utilization,
    Kpi,
    Export,
}

#[component]
pub fn Reports() -> Element {
    let auth = use_context::<Signal<AuthState>>();
    let mut active_tab = use_signal(|| ReportTab::Kpi);

    let can_read = auth.read().has_permission("api.reports.read");
    let can_export = auth.read().has_permission("action.reports.export");
    let can_unmasked = auth.read().has_permission("api.export.unmasked");

    if auth.read().token.is_none() {
        return rsx! {
            div { class: "page-header",
                h1 { "Reports & Exports" }
                p { style: "color: var(--color-text-secondary);",
                    "Your session has expired. Please log in again."
                }
            }
        };
    }

    if !can_read {
        return rsx! {
            div { class: "page-header",
                h1 { "Reports & Exports" }
                p { style: "color: var(--color-text-secondary);",
                    "You do not have permission to view reports."
                }
            }
        };
    }

    rsx! {
        div {
            div { class: "page-header",
                h1 { "Reports & Exports" }
                p { "Operational, financial, and quality analytics derived from stored data. Exports are masked by default." }
            }

            div { style: "display: flex; gap: 4px; border-bottom: 2px solid var(--color-border); margin-bottom: 24px; flex-wrap: wrap;",
                TabButton {
                    label: "KPI Summary",
                    active: *active_tab.read() == ReportTab::Kpi,
                    onclick: move |_| { *active_tab.write() = ReportTab::Kpi; }
                }
                TabButton {
                    label: "Order Volume",
                    active: *active_tab.read() == ReportTab::OrderVolume,
                    onclick: move |_| { *active_tab.write() = ReportTab::OrderVolume; }
                }
                TabButton {
                    label: "Revenue",
                    active: *active_tab.read() == ReportTab::Revenue,
                    onclick: move |_| { *active_tab.write() = ReportTab::Revenue; }
                }
                TabButton {
                    label: "Utilization",
                    active: *active_tab.read() == ReportTab::Utilization,
                    onclick: move |_| { *active_tab.write() = ReportTab::Utilization; }
                }
                if can_export {
                    TabButton {
                        label: "Export",
                        active: *active_tab.read() == ReportTab::Export,
                        onclick: move |_| { *active_tab.write() = ReportTab::Export; }
                    }
                }
            }

            match *active_tab.read() {
                ReportTab::Kpi          => rsx! { KpiTab {} },
                ReportTab::OrderVolume  => rsx! { OrderVolumeTab {} },
                ReportTab::Revenue      => rsx! { RevenueTab {} },
                ReportTab::Utilization  => rsx! { UtilizationTab {} },
                ReportTab::Export       => rsx! { ExportTab { can_unmasked: can_unmasked } },
            }
        }
    }
}

// ============================================================
// KPI tab
// ============================================================

#[component]
fn KpiTab() -> Element {
    let auth = use_context::<Signal<AuthState>>();
    let mut from_date = use_signal(|| "".to_string());
    let mut to_date = use_signal(|| "".to_string());
    let mut department_id = use_signal(|| "".to_string());
    let mut service_route = use_signal(|| "".to_string());
    let mut error_msg = use_signal(|| String::new());
    let mut kpi_data = use_signal(|| None::<KpiSummary>);

    let on_run = {
        let from_date = from_date.clone();
        let to_date = to_date.clone();
        let department_id = department_id.clone();
        let service_route = service_route.clone();
        let mut error_msg = error_msg.clone();
        let mut kpi_data = kpi_data.clone();
        move |_| {
            let f = from_date.read().clone();
            let t_d = to_date.read().clone();
            let dept = department_id.read().clone();
            let route = service_route.read().clone();
            if f.is_empty() || t_d.is_empty() {
                *error_msg.write() = "From and To dates are required.".to_string();
                return;
            }
            let t = auth.read().token.clone();
            spawn(async move {
                let mut url = format!("/reports/kpi?from_date={}&to_date={}", f, t_d);
                if !dept.is_empty() { url.push_str(&format!("&department_id={}", dept)); }
                if !route.is_empty() { url.push_str(&format!("&service_route={}", route)); }
                match ApiClient::get::<KpiSummary>(&url, t.as_deref()).await {
                    Ok(data) => {
                        *kpi_data.write() = Some(data);
                        *error_msg.write() = String::new();
                    }
                    Err(e) => {
                        *error_msg.write() = e;
                        *kpi_data.write() = None;
                    }
                }
            });
        }
    };

    rsx! {
        div {
            DateRangeFilter {
                from_date: from_date,
                to_date: to_date,
                department_id: department_id,
                service_route: service_route,
                on_run: on_run,
            }

            if !error_msg.read().is_empty() {
                div { class: "alert alert-error", "{error_msg}" }
            }

            if let Some(kpi) = &*kpi_data.read() {
                div { style: "display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 16px;",
                    KpiCard { title: "Attendance Rate", value: format!("{:.1}", kpi.attendance_rate_pct), unit: "%" }
                    KpiCard { title: "Repurchase Rate", value: format!("{:.1}", kpi.repurchase_rate_pct), unit: "%" }
                    KpiCard { title: "Staff Utilization", value: format!("{:.1}", kpi.staff_utilization_pct), unit: "%" }
                    KpiCard { title: "Avg Quality Score", value: kpi.avg_score.map(|s| format!("{:.1}", s)).unwrap_or_else(|| "—".to_string()), unit: "/100" }
                    KpiCard { title: "Second Review Rate", value: format!("{:.1}", kpi.second_review_rate_pct), unit: "%" }
                }
            } else {
                div { style: "display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 16px;",
                    KpiCard { title: "Attendance Rate", value: "—".to_string(), unit: "%" }
                    KpiCard { title: "Repurchase Rate", value: "—".to_string(), unit: "%" }
                    KpiCard { title: "Staff Utilization", value: "—".to_string(), unit: "%" }
                    KpiCard { title: "Avg Quality Score", value: "—".to_string(), unit: "/100" }
                    KpiCard { title: "Second Review Rate", value: "—".to_string(), unit: "%" }
                }
            }
        }
    }
}

#[component]
fn KpiCard(title: &'static str, value: String, unit: &'static str) -> Element {
    rsx! {
        div { class: "card",
            p { style: "font-size: 0.75rem; color: var(--color-text-secondary); margin-bottom: 4px;", "{title}" }
            p { style: "font-size: 2rem; font-weight: 700; color: var(--color-primary); margin-bottom: 4px;",
                "{value}"
                span { style: "font-size: 0.875rem; font-weight: 400; color: var(--color-text-secondary);", "{unit}" }
            }
        }
    }
}

// ============================================================
// Order volume tab
// ============================================================

#[component]
fn OrderVolumeTab() -> Element {
    let auth = use_context::<Signal<AuthState>>();
    let mut from_date = use_signal(|| "".to_string());
    let mut to_date = use_signal(|| "".to_string());
    let mut department_id = use_signal(|| "".to_string());
    let mut service_route = use_signal(|| "".to_string());
    let mut error_msg = use_signal(|| String::new());
    let mut report_data = use_signal(|| None::<Vec<OrderVolumeRow>>);

    let on_run = {
        let from_date = from_date.clone();
        let to_date = to_date.clone();
        let department_id = department_id.clone();
        let service_route = service_route.clone();
        let mut error_msg = error_msg.clone();
        let mut report_data = report_data.clone();
        move |_| {
            let f = from_date.read().clone();
            let t_d = to_date.read().clone();
            let dept = department_id.read().clone();
            let route = service_route.read().clone();
            if f.is_empty() || t_d.is_empty() {
                *error_msg.write() = "From and To dates are required.".to_string();
                return;
            }
            let t = auth.read().token.clone();
            spawn(async move {
                let mut url = format!("/reports/order-volume?from_date={}&to_date={}", f, t_d);
                if !dept.is_empty() { url.push_str(&format!("&department_id={}", dept)); }
                if !route.is_empty() { url.push_str(&format!("&service_route={}", route)); }
                match ApiClient::get::<Vec<OrderVolumeRow>>(&url, t.as_deref()).await {
                    Ok(data) => { *report_data.write() = Some(data); *error_msg.write() = String::new(); }
                    Err(e) => { *error_msg.write() = e; *report_data.write() = None; }
                }
            });
        }
    };

    rsx! {
        div {
            DateRangeFilter { from_date: from_date, to_date: to_date, department_id: department_id, service_route: service_route, on_run: on_run }
            if !error_msg.read().is_empty() { div { class: "alert alert-error", "{error_msg}" } }
            div { class: "card", style: "overflow-x: auto;",
                table { style: "width: 100%; border-collapse: collapse;",
                    thead {
                        tr { style: "border-bottom: 1px solid var(--color-border);",
                            th { style: "padding: 10px 12px; text-align: left; font-size: 0.875rem;", "Week" }
                            th { style: "padding: 10px 12px; text-align: right; font-size: 0.875rem;", "Deliveries" }
                            th { style: "padding: 10px 12px; text-align: right; font-size: 0.875rem;", "Unique Plans" }
                            th { style: "padding: 10px 12px; text-align: right; font-size: 0.875rem;", "Unique Providers" }
                        }
                    }
                    tbody {
                        match &*report_data.read() {
                            Some(rows) if !rows.is_empty() => rsx! {
                                for row in rows {
                                    tr { style: "border-bottom: 1px solid var(--color-border);",
                                        td { style: "padding: 8px 12px; font-size: 0.875rem;", "{row.period}" }
                                        td { style: "padding: 8px 12px; text-align: right; font-size: 0.875rem; font-weight: 600;", "{row.delivery_count}" }
                                        td { style: "padding: 8px 12px; text-align: right; font-size: 0.875rem;", "{row.unique_plans}" }
                                        td { style: "padding: 8px 12px; text-align: right; font-size: 0.875rem;", "{row.unique_providers}" }
                                    }
                                }
                            },
                            Some(_) => rsx! {
                                tr {
                                    td { colspan: "4", style: "padding: 24px; text-align: center; color: var(--color-text-secondary); font-size: 0.875rem;",
                                        "No order volume data for the selected period."
                                    }
                                }
                            },
                            None => rsx! {
                                tr {
                                    td { colspan: "4", style: "padding: 24px; text-align: center; color: var(--color-text-secondary); font-size: 0.875rem;",
                                        "Select a date range and click Run Report."
                                    }
                                }
                            },
                        }
                    }
                }
            }
        }
    }
}

// ============================================================
// Revenue tab
// ============================================================

#[component]
fn RevenueTab() -> Element {
    let auth = use_context::<Signal<AuthState>>();
    let mut from_date = use_signal(|| "".to_string());
    let mut to_date = use_signal(|| "".to_string());
    let mut department_id = use_signal(|| "".to_string());
    let mut service_route = use_signal(|| "".to_string());
    let mut error_msg = use_signal(|| String::new());
    let mut report_data = use_signal(|| None::<Vec<RevenueReportRow>>);

    let on_run = {
        let from_date = from_date.clone();
        let to_date = to_date.clone();
        let department_id = department_id.clone();
        let service_route = service_route.clone();
        let mut error_msg = error_msg.clone();
        let mut report_data = report_data.clone();
        move |_| {
            let f = from_date.read().clone();
            let t_d = to_date.read().clone();
            let dept = department_id.read().clone();
            let route = service_route.read().clone();
            if f.is_empty() || t_d.is_empty() {
                *error_msg.write() = "From and To dates are required.".to_string();
                return;
            }
            let t = auth.read().token.clone();
            spawn(async move {
                let mut url = format!("/reports/revenue?from_date={}&to_date={}", f, t_d);
                if !dept.is_empty() { url.push_str(&format!("&department_id={}", dept)); }
                if !route.is_empty() { url.push_str(&format!("&service_route={}", route)); }
                match ApiClient::get::<Vec<RevenueReportRow>>(&url, t.as_deref()).await {
                    Ok(data) => { *report_data.write() = Some(data); *error_msg.write() = String::new(); }
                    Err(e) => { *error_msg.write() = e; *report_data.write() = None; }
                }
            });
        }
    };

    rsx! {
        div {
            DateRangeFilter { from_date: from_date, to_date: to_date, department_id: department_id, service_route: service_route, on_run: on_run }
            if !error_msg.read().is_empty() { div { class: "alert alert-error", "{error_msg}" } }
            div { class: "card", style: "overflow-x: auto;",
                table { style: "width: 100%; border-collapse: collapse;",
                    thead {
                        tr { style: "border-bottom: 1px solid var(--color-border);",
                            th { style: "padding: 10px 12px; text-align: left; font-size: 0.875rem;", "Week" }
                            th { style: "padding: 10px 12px; text-align: right; font-size: 0.875rem;", "Gross Charges" }
                            th { style: "padding: 10px 12px; text-align: right; font-size: 0.875rem;", "Net Charges" }
                            th { style: "padding: 10px 12px; text-align: right; font-size: 0.875rem;", "Invoiced" }
                            th { style: "padding: 10px 12px; text-align: right; font-size: 0.875rem;", "Paid" }
                            th { style: "padding: 10px 12px; text-align: right; font-size: 0.875rem;", "Refunded" }
                            th { style: "padding: 10px 12px; text-align: right; font-size: 0.875rem;", "Refund Rate" }
                        }
                    }
                    tbody {
                        match &*report_data.read() {
                            Some(rows) if !rows.is_empty() => rsx! {
                                for row in rows {
                                    tr { style: "border-bottom: 1px solid var(--color-border);",
                                        td { style: "padding: 8px 12px; font-size: 0.875rem;", "{row.period}" }
                                        td { style: "padding: 8px 12px; text-align: right; font-size: 0.875rem;", "${row.gross_charges:.2}" }
                                        td { style: "padding: 8px 12px; text-align: right; font-size: 0.875rem;", "${row.net_charges:.2}" }
                                        td { style: "padding: 8px 12px; text-align: right; font-size: 0.875rem;", "${row.total_invoiced:.2}" }
                                        td { style: "padding: 8px 12px; text-align: right; font-size: 0.875rem;", "${row.total_paid:.2}" }
                                        td { style: "padding: 8px 12px; text-align: right; font-size: 0.875rem;", "${row.total_refunded:.2}" }
                                        td { style: "padding: 8px 12px; text-align: right; font-size: 0.875rem;", "{row.refund_rate_pct:.1}%" }
                                    }
                                }
                            },
                            Some(_) => rsx! {
                                tr { td { colspan: "7", style: "padding: 24px; text-align: center; color: var(--color-text-secondary); font-size: 0.875rem;", "No revenue data for the selected period." } }
                            },
                            None => rsx! {
                                tr { td { colspan: "7", style: "padding: 24px; text-align: center; color: var(--color-text-secondary); font-size: 0.875rem;", "Select a date range and click Run Report." } }
                            },
                        }
                    }
                }
            }
        }
    }
}

// ============================================================
// Utilization tab
// ============================================================

#[component]
fn UtilizationTab() -> Element {
    let auth = use_context::<Signal<AuthState>>();
    let mut from_date = use_signal(|| "".to_string());
    let mut to_date = use_signal(|| "".to_string());
    let mut department_id = use_signal(|| "".to_string());
    let mut service_route = use_signal(|| "".to_string());
    let mut error_msg = use_signal(|| String::new());
    let mut report_data = use_signal(|| None::<Vec<UtilizationRow>>);

    let on_run = {
        let from_date = from_date.clone();
        let to_date = to_date.clone();
        let department_id = department_id.clone();
        let service_route = service_route.clone();
        let mut error_msg = error_msg.clone();
        let mut report_data = report_data.clone();
        move |_| {
            let f = from_date.read().clone();
            let t_d = to_date.read().clone();
            let dept = department_id.read().clone();
            let route = service_route.read().clone();
            if f.is_empty() || t_d.is_empty() {
                *error_msg.write() = "From and To dates are required.".to_string();
                return;
            }
            let t = auth.read().token.clone();
            spawn(async move {
                let mut url = format!("/reports/utilization?from_date={}&to_date={}", f, t_d);
                if !dept.is_empty() { url.push_str(&format!("&department_id={}", dept)); }
                if !route.is_empty() { url.push_str(&format!("&service_route={}", route)); }
                match ApiClient::get::<Vec<UtilizationRow>>(&url, t.as_deref()).await {
                    Ok(data) => { *report_data.write() = Some(data); *error_msg.write() = String::new(); }
                    Err(e) => { *error_msg.write() = e; *report_data.write() = None; }
                }
            });
        }
    };

    rsx! {
        div {
            DateRangeFilter { from_date: from_date, to_date: to_date, department_id: department_id, service_route: service_route, on_run: on_run }
            if !error_msg.read().is_empty() { div { class: "alert alert-error", "{error_msg}" } }
            div { class: "card", style: "overflow-x: auto;",
                table { style: "width: 100%; border-collapse: collapse;",
                    thead {
                        tr { style: "border-bottom: 1px solid var(--color-border);",
                            th { style: "padding: 10px 12px; text-align: left; font-size: 0.875rem;", "Provider" }
                            th { style: "padding: 10px 12px; text-align: left; font-size: 0.875rem;", "Week" }
                            th { style: "padding: 10px 12px; text-align: right; font-size: 0.875rem;", "Visits" }
                            th { style: "padding: 10px 12px; text-align: right; font-size: 0.875rem;", "Units" }
                            th { style: "padding: 10px 12px; text-align: right; font-size: 0.875rem;", "Mileage" }
                        }
                    }
                    tbody {
                        match &*report_data.read() {
                            Some(rows) if !rows.is_empty() => rsx! {
                                for row in rows {
                                    tr { style: "border-bottom: 1px solid var(--color-border);",
                                        td { style: "padding: 8px 12px; font-size: 0.8rem; font-family: monospace;", "{&row.provider_id[..8]}..." }
                                        td { style: "padding: 8px 12px; font-size: 0.875rem;", "{row.period}" }
                                        td { style: "padding: 8px 12px; text-align: right; font-size: 0.875rem;", "{row.total_visits}" }
                                        td { style: "padding: 8px 12px; text-align: right; font-size: 0.875rem;", "{row.total_units:.2}" }
                                        td { style: "padding: 8px 12px; text-align: right; font-size: 0.875rem;", "{row.total_mileage:.1}" }
                                    }
                                }
                            },
                            Some(_) => rsx! {
                                tr { td { colspan: "5", style: "padding: 24px; text-align: center; color: var(--color-text-secondary); font-size: 0.875rem;", "No utilization data for the selected period." } }
                            },
                            None => rsx! {
                                tr { td { colspan: "5", style: "padding: 24px; text-align: center; color: var(--color-text-secondary); font-size: 0.875rem;", "Select a date range and click Run Report." } }
                            },
                        }
                    }
                }
            }
        }
    }
}

// ============================================================
// Export tab
// ============================================================

#[component]
fn ExportTab(can_unmasked: bool) -> Element {
    let auth = use_context::<Signal<AuthState>>();
    let mut export_type = use_signal(|| "deliveries".to_string());
    let mut from_date = use_signal(|| "".to_string());
    let mut to_date = use_signal(|| "".to_string());
    let mut service_route = use_signal(|| "".to_string());
    let mut unmasked = use_signal(|| false);
    let mut error_msg = use_signal(|| String::new());
    let mut success_msg = use_signal(|| String::new());
    let mut export_result = use_signal(|| None::<ExportResult>);

    let on_export = {
        let export_type = export_type.clone();
        let from_date = from_date.clone();
        let to_date = to_date.clone();
        let service_route = service_route.clone();
        let unmasked = unmasked.clone();
        let mut error_msg = error_msg.clone();
        let mut success_msg = success_msg.clone();
        let mut export_result = export_result.clone();
        move |_| {
            let et = export_type.read().clone();
            let f = from_date.read().clone();
            let td = to_date.read().clone();
            let route = service_route.read().clone();
            let um = *unmasked.read();
            if f.is_empty() || td.is_empty() {
                *error_msg.write() = "From and To dates are required.".to_string();
                return;
            }
            let t = auth.read().token.clone();
            spawn(async move {
                let mut body = serde_json::json!({
                    "export_type": et,
                    "from_date": f,
                    "to_date": td,
                    "unmasked": um,
                });
                if !route.is_empty() {
                    body["service_route"] = serde_json::Value::String(route);
                }
                match ApiClient::post::<ExportResult, _>("/reports/export", &body, t.as_deref()).await {
                    Ok(result) => {
                        let count = result.row_count;
                        let masked_label = if result.masked { "masked" } else { "unmasked" };
                        *success_msg.write() = format!("Export complete: {} rows ({}). Log ID: {}", count, masked_label, &result.export_log_id[..8]);
                        *error_msg.write() = String::new();
                        *export_result.write() = Some(result);
                    }
                    Err(e) => {
                        *error_msg.write() = e;
                        *success_msg.write() = String::new();
                        *export_result.write() = None;
                    }
                }
            });
        }
    };

    rsx! {
        div {
            div { class: "card", style: "max-width: 540px;",
                h3 { style: "margin-bottom: 16px;", "Data Export" }

                div { style: "padding: 10px 14px; background: var(--color-warning-light, #fef3c7); border-left: 3px solid var(--color-warning, #f59e0b); border-radius: 4px; font-size: 0.875rem; margin-bottom: 16px;",
                    strong { "Default: identifiers are masked." }
                    " Client names appear as \"****\" and provider IDs are truncated. "
                    if can_unmasked {
                        "You have permission to request unmasked exports."
                    } else {
                        "Contact your administrator to obtain unmasked export access."
                    }
                }

                if !error_msg.read().is_empty() {
                    div { class: "alert alert-error", "{error_msg}" }
                }
                if !success_msg.read().is_empty() {
                    div { class: "alert alert-success", "{success_msg}" }
                }

                div { style: "display: flex; flex-direction: column; gap: 14px;",
                    div {
                        label { style: "display: block; font-size: 0.875rem; font-weight: 500; margin-bottom: 4px;", "Export Type" }
                        select {
                            style: "width: 100%; padding: 8px 12px; border: 1px solid var(--color-border); border-radius: 4px; font-size: 0.875rem;",
                            onchange: move |e| { *export_type.write() = e.value().clone(); },
                            option { value: "deliveries", "Deliveries" }
                            option { value: "evaluations", "Evaluations" }
                            option { value: "revenue", "Revenue / Invoices" }
                        }
                    }
                    div { style: "display: flex; gap: 12px;",
                        div { style: "flex: 1;",
                            label { style: "display: block; font-size: 0.875rem; font-weight: 500; margin-bottom: 4px;", "From *" }
                            input {
                                r#type: "date",
                                style: "width: 100%; padding: 8px 12px; border: 1px solid var(--color-border); border-radius: 4px; font-size: 0.875rem;",
                                value: "{from_date}",
                                oninput: move |e| { *from_date.write() = e.value().clone(); },
                            }
                        }
                        div { style: "flex: 1;",
                            label { style: "display: block; font-size: 0.875rem; font-weight: 500; margin-bottom: 4px;", "To *" }
                            input {
                                r#type: "date",
                                style: "width: 100%; padding: 8px 12px; border: 1px solid var(--color-border); border-radius: 4px; font-size: 0.875rem;",
                                value: "{to_date}",
                                oninput: move |e| { *to_date.write() = e.value().clone(); },
                            }
                        }
                    }
                    div {
                        label { style: "display: block; font-size: 0.875rem; font-weight: 500; margin-bottom: 4px;", "Service Route (optional)" }
                        input {
                            r#type: "text",
                            placeholder: "e.g. north-metro",
                            style: "width: 100%; padding: 8px 12px; border: 1px solid var(--color-border); border-radius: 4px; font-size: 0.875rem;",
                            value: "{service_route}",
                            oninput: move |e| { *service_route.write() = e.value().clone(); },
                        }
                    }
                    if can_unmasked {
                        label { style: "display: flex; align-items: center; gap: 8px; font-size: 0.875rem; cursor: pointer;",
                            input {
                                r#type: "checkbox",
                                checked: *unmasked.read(),
                                onchange: move |e| { *unmasked.write() = e.checked(); },
                            }
                            "Include unmasked identifiers (client names, provider IDs)"
                        }
                    }
                    div { style: "display: flex; gap: 8px; align-items: center;",
                        button {
                            class: "btn btn-primary",
                            onclick: on_export,
                            "Export JSON"
                        }
                        span { style: "font-size: 0.75rem; color: var(--color-text-secondary);",
                            if *unmasked.read() && can_unmasked {
                                "Unmasked export — logged in audit trail"
                            } else {
                                "Masked export — identifiers hidden"
                            }
                        }
                    }
                }
            }

            if let Some(result) = &*export_result.read() {
                div { class: "card", style: "margin-top: 16px;",
                    h4 { style: "margin-bottom: 8px;", "Export Preview ({result.row_count} rows)" }
                    div { style: "max-height: 300px; overflow: auto; font-size: 0.8rem; font-family: monospace; background: var(--color-surface); padding: 12px; border-radius: 4px; border: 1px solid var(--color-border);",
                        pre {
                            {serde_json::to_string_pretty(&result.rows).unwrap_or_else(|_| "[]".to_string())}
                        }
                    }
                }
            }
        }
    }
}

// ============================================================
// Shared components
// ============================================================

#[component]
fn DateRangeFilter(
    from_date: Signal<String>,
    to_date: Signal<String>,
    department_id: Signal<String>,
    service_route: Signal<String>,
    on_run: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        div { style: "display: flex; gap: 12px; margin-bottom: 16px; flex-wrap: wrap; align-items: flex-end;",
            div {
                label { style: "display: block; font-size: 0.75rem; font-weight: 500; margin-bottom: 4px; color: var(--color-text-secondary);", "From *" }
                input {
                    r#type: "date",
                    style: "padding: 6px 10px; border: 1px solid var(--color-border); border-radius: 4px; font-size: 0.875rem;",
                    value: "{from_date}",
                    oninput: move |e| { *from_date.write() = e.value().clone(); },
                }
            }
            div {
                label { style: "display: block; font-size: 0.75rem; font-weight: 500; margin-bottom: 4px; color: var(--color-text-secondary);", "To *" }
                input {
                    r#type: "date",
                    style: "padding: 6px 10px; border: 1px solid var(--color-border); border-radius: 4px; font-size: 0.875rem;",
                    value: "{to_date}",
                    oninput: move |e| { *to_date.write() = e.value().clone(); },
                }
            }
            div {
                label { style: "display: block; font-size: 0.75rem; font-weight: 500; margin-bottom: 4px; color: var(--color-text-secondary);", "Department ID (optional)" }
                input {
                    r#type: "text",
                    placeholder: "Filter by dept...",
                    style: "padding: 6px 10px; border: 1px solid var(--color-border); border-radius: 4px; font-size: 0.875rem;",
                    value: "{department_id}",
                    oninput: move |e| { *department_id.write() = e.value().clone(); },
                }
            }
            div {
                label { style: "display: block; font-size: 0.75rem; font-weight: 500; margin-bottom: 4px; color: var(--color-text-secondary);", "Service Route (optional)" }
                input {
                    r#type: "text",
                    placeholder: "e.g. north-metro",
                    style: "padding: 6px 10px; border: 1px solid var(--color-border); border-radius: 4px; font-size: 0.875rem;",
                    value: "{service_route}",
                    oninput: move |e| { *service_route.write() = e.value().clone(); },
                }
            }
            button {
                class: "btn btn-primary",
                style: "align-self: flex-end;",
                onclick: move |e| on_run.call(e),
                "Run Report"
            }
        }
    }
}

#[component]
fn TabButton(label: &'static str, active: bool, onclick: EventHandler<MouseEvent>) -> Element {
    let style = if active {
        "padding: 8px 16px; border: none; border-bottom: 2px solid var(--color-primary); background: transparent; font-weight: 600; cursor: pointer; color: var(--color-primary);"
    } else {
        "padding: 8px 16px; border: none; border-bottom: 2px solid transparent; background: transparent; cursor: pointer; color: var(--color-text-secondary);"
    };
    rsx! {
        button { style: style, onclick: move |e| onclick.call(e), "{label}" }
    }
}
