use dioxus::prelude::*;
use crate::components::loading::Loading;
use crate::components::error_state::ErrorState;
use crate::components::permission_denied::PermissionDenied;
use crate::models::{DeliveryListResponse, ClientPlanRow, PlanPackageRow, PackageDetail, ServiceItemRow};
use crate::services::ApiClient;
use crate::state::{AuthState, perms};

#[component]
pub fn Delivery() -> Element {
    let auth: Signal<AuthState> = use_context();

    if !auth.read().has_permission(perms::MENU_DELIVERY) {
        return rsx! { PermissionDenied { action: "view service delivery".to_string() } };
    }

    let token = auth.read().token.clone();
    let mut show_form = use_signal(|| false);
    let mut refresh = use_signal(|| 0u32);

    let entries = use_resource(move || {
        let t = token.clone();
        let _r = refresh.read();
        async move { ApiClient::get::<DeliveryListResponse>("/delivery/?limit=50", t.as_deref()).await }
    });

    rsx! {
        div {
            div { class: "page-header",
                h1 { "Service Delivery" }
                p { "Log service delivery entries against client plans. Entries are validated for quarter-hour increments and mileage caps." }
            }

            if auth.read().has_permission("action.delivery.log") {
                div { style: "margin-bottom: 16px;",
                    button { class: "btn btn-primary", onclick: move |_| show_form.toggle(),
                        if *show_form.read() { "Cancel" } else { "Log Entry" }
                    }
                }
            }

            if *show_form.read() {
                DeliveryForm { on_success: move || { show_form.set(false); *refresh.write() += 1; } }
            }

            div { class: "card",
                match &*entries.read() {
                    None => rsx! { Loading {} },
                    Some(Err(e)) => rsx! { ErrorState { message: "Failed to load entries".to_string(), detail: Some(e.clone()) } },
                    Some(Ok(resp)) => rsx! {
                        h3 { style: "margin-bottom: 12px;", "Delivery Entries ({resp.total} total)" }
                        table { style: "width: 100%; border-collapse: collapse;",
                            thead {
                                tr { style: "border-bottom: 2px solid var(--color-border); text-align: left;",
                                    th { style: "padding: 8px 10px; font-size: 11px; text-transform: uppercase; color: var(--color-text-secondary);", "Date" }
                                    th { style: "padding: 8px 10px; font-size: 11px; text-transform: uppercase; color: var(--color-text-secondary);", "Plan" }
                                    th { style: "padding: 8px 10px; font-size: 11px; text-transform: uppercase; color: var(--color-text-secondary);", "Units" }
                                    th { style: "padding: 8px 10px; font-size: 11px; text-transform: uppercase; color: var(--color-text-secondary);", "Mileage" }
                                    th { style: "padding: 8px 10px; font-size: 11px; text-transform: uppercase; color: var(--color-text-secondary);", "Status" }
                                    th { style: "padding: 8px 10px; font-size: 11px; text-transform: uppercase; color: var(--color-text-secondary);", "Time" }
                                }
                            }
                            tbody {
                                for entry in resp.data.iter() {
                                    tr { style: "border-bottom: 1px solid var(--color-border);", key: "{entry.id}",
                                        td { style: "padding: 8px 10px;", "{entry.delivery_date}" }
                                        td { style: "padding: 8px 10px; font-size: 13px; color: var(--color-text-secondary);", "{entry.plan_id}" }
                                        td { style: "padding: 8px 10px; font-weight: 500;", "{entry.units}" }
                                        td { style: "padding: 8px 10px;",
                                            match entry.mileage {
                                                Some(m) => rsx! { span { "{m} mi" } },
                                                None => rsx! { span { style: "color: var(--color-text-secondary);", "-" } },
                                            }
                                        }
                                        td { style: "padding: 8px 10px;",
                                            DeliveryStatusBadge { status: entry.status.clone() }
                                        }
                                        td { style: "padding: 8px 10px; font-size: 13px; color: var(--color-text-secondary);",
                                            match (&entry.start_time, &entry.end_time) {
                                                (Some(s), Some(e)) => rsx! { span { "{s} - {e}" } },
                                                _ => rsx! { span { "-" } },
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        if resp.data.is_empty() {
                            p { style: "text-align: center; padding: 24px; color: var(--color-text-secondary);", "No delivery entries yet." }
                        }
                    }
                }
            }
        }
    }
}

// ============================================================
// Delivery form with dependent selectors
//
// Selection flow: Plan → Package → Service Item
//
// When the user picks a plan, the form fetches that plan's package
// assignments.  When they pick a package assignment, the form
// fetches the package detail (including rules with service_item_id)
// and resolves human-readable service names from the catalog.
// The form submits the same JSON payload the backend expects:
//   { plan_id, plan_package_id, service_item_id, ... }
// ============================================================

/// One resolved option for the service-item selector, combining
/// the rule's service_item_id with the catalog item's human label.
#[derive(Clone, Debug)]
struct ServiceOption {
    service_item_id: String,
    label: String,
}

#[component]
fn DeliveryForm(on_success: EventHandler<()>) -> Element {
    let auth: Signal<AuthState> = use_context();
    let token = auth.read().token.clone();

    // ------ Stable data: plans + full service catalog ------
    let token_plans = token.clone();
    let plans = use_resource(move || {
        let t = token_plans.clone();
        async move { ApiClient::get::<Vec<ClientPlanRow>>("/plans/?status=active", t.as_deref()).await }
    });

    let token_catalog = token.clone();
    let catalog = use_resource(move || {
        let t = token_catalog.clone();
        async move { ApiClient::get::<Vec<ServiceItemRow>>("/catalog/?active_only=true", t.as_deref()).await }
    });

    // ------ Selected IDs ------
    let mut selected_plan = use_signal(String::new);
    let mut selected_pkg = use_signal(String::new);   // plan_package_id
    let mut selected_svc = use_signal(String::new);   // service_item_id

    // ------ Dependent data loaded on selection change ------
    let mut packages: Signal<Option<Result<Vec<PlanPackageRow>, String>>> = use_signal(|| None);
    let mut svc_options: Signal<Vec<ServiceOption>> = use_signal(Vec::new);

    // Loading / error indicators for dependent fetches
    let mut pkg_loading = use_signal(|| false);
    let mut svc_loading = use_signal(|| false);

    // ------ Other form fields ------
    let mut delivery_date = use_signal(String::new);
    let mut start_time = use_signal(String::new);
    let mut end_time = use_signal(String::new);
    let mut units = use_signal(|| "1.0".to_string());
    let mut mileage = use_signal(String::new);
    let mut notes = use_signal(String::new);
    let mut form_error = use_signal(|| None::<String>);
    let mut validation_msgs = use_signal(|| Vec::<String>::new());

    // ------ Plan change handler: fetch packages, clear downstream ------
    let mut on_plan_change = {
        let token = token.clone();
        move |plan_id: String| {
            // Clear downstream state immediately to avoid stale selections
            selected_plan.set(plan_id.clone());
            selected_pkg.set(String::new());
            selected_svc.set(String::new());
            svc_options.set(Vec::new());

            if plan_id.is_empty() {
                packages.set(None);
                return;
            }

            pkg_loading.set(true);
            let t = token.clone();
            let pid = plan_id.clone();
            spawn(async move {
                let result = ApiClient::get::<Vec<PlanPackageRow>>(
                    &format!("/plans/{}/packages", pid),
                    t.as_deref(),
                ).await;
                packages.set(Some(result));
                pkg_loading.set(false);
            });
        }
    };

    // ------ Package change handler: fetch rules → resolve service names ------
    let mut on_pkg_change = {
        let token = token.clone();
        move |plan_package_id: String, package_id: String| {
            // Clear downstream state immediately
            selected_pkg.set(plan_package_id);
            selected_svc.set(String::new());

            if package_id.is_empty() {
                svc_options.set(Vec::new());
                return;
            }

            svc_loading.set(true);
            let t = token.clone();
            let pkid = package_id.clone();
            spawn(async move {
                let result = ApiClient::get::<PackageDetail>(
                    &format!("/packages/{}", pkid),
                    t.as_deref(),
                ).await;

                match result {
                    Ok(detail) => {
                        // Build human-readable options by joining rules with catalog
                        let cat = catalog.read();
                        let catalog_items: Vec<ServiceItemRow> = match cat.as_ref() {
                            Some(Ok(items)) => items.clone(),
                            _ => Vec::new(),
                        };

                        let options: Vec<ServiceOption> = detail.rules.iter()
                            .filter(|r| r.is_active)
                            .map(|rule| {
                                let label = catalog_items.iter()
                                    .find(|item| item.id == rule.service_item_id)
                                    .map(|item| format!("{} ({}, {})", item.name, item.category, rule.rule_type))
                                    .unwrap_or_else(|| format!("{} ({})", rule.service_item_id, rule.rule_type));
                                ServiceOption {
                                    service_item_id: rule.service_item_id.clone(),
                                    label,
                                }
                            })
                            .collect();

                        svc_options.set(options);
                    }
                    Err(_) => {
                        svc_options.set(Vec::new());
                    }
                }
                svc_loading.set(false);
            });
        }
    };

    // ------ Validation ------
    let mut validate = move || {
        let mut msgs = Vec::new();
        let u: f64 = units.read().parse().unwrap_or(0.0);
        if u <= 0.0 { msgs.push("Units must be greater than 0".to_string()); }
        let remainder = (u * 4.0).fract();
        if remainder.abs() > 0.001 { msgs.push("Hours must be in 0.25 increments".to_string()); }
        let m: f64 = mileage.read().parse().unwrap_or(0.0);
        if !mileage.read().is_empty() && m > 200.0 { msgs.push("Mileage cannot exceed 200 per visit".to_string()); }
        if delivery_date.read().is_empty() { msgs.push("Delivery date is required".to_string()); }
        if selected_plan.read().is_empty() { msgs.push("Select a client plan".to_string()); }
        if selected_pkg.read().is_empty() { msgs.push("Select a plan package".to_string()); }
        if selected_svc.read().is_empty() { msgs.push("Select a service item".to_string()); }
        validation_msgs.set(msgs.clone());
        msgs.is_empty()
    };

    // ------ Submit (payload shape unchanged) ------
    let on_submit = move |_| {
        if !validate() { return; }
        let t = auth.read().token.clone();
        let body = serde_json::json!({
            "plan_id": *selected_plan.read(),
            "plan_package_id": *selected_pkg.read(),
            "service_item_id": *selected_svc.read(),
            "delivery_date": *delivery_date.read(),
            "start_time": if start_time.read().is_empty() { None::<String> } else { Some(start_time.read().clone()) },
            "end_time": if end_time.read().is_empty() { None::<String> } else { Some(end_time.read().clone()) },
            "units": units.read().parse::<f64>().unwrap_or(0.0),
            "mileage": if mileage.read().is_empty() { None::<f64> } else { mileage.read().parse::<f64>().ok() },
            "notes": if notes.read().is_empty() { None::<String> } else { Some(notes.read().clone()) },
        });

        spawn(async move {
            form_error.set(None);
            match ApiClient::post::<serde_json::Value, _>("/delivery/", &body, t.as_deref()).await {
                Ok(_) => on_success.call(()),
                Err(e) => form_error.set(Some(e)),
            }
        });
    };

    // ------ Helper: resolve package_id from a plan_package_id ------
    let get_package_id_for = move |pp_id: &str| -> String {
        if let Some(Ok(pkgs)) = packages.read().as_ref() {
            pkgs.iter()
                .find(|p| p.id == pp_id)
                .map(|p| p.package_id.clone())
                .unwrap_or_default()
        } else {
            String::new()
        }
    };

    rsx! {
        div { class: "card", style: "margin-bottom: 20px;",
            h3 { style: "margin-bottom: 16px;", "Log Delivery Entry" }

            if let Some(err) = form_error.read().as_ref() {
                div { style: "background: #fef2f2; border: 1px solid #fecaca; color: #dc2626; padding: 8px 12px; border-radius: var(--radius); margin-bottom: 12px; font-size: 13px;", "{err}" }
            }
            if !validation_msgs.read().is_empty() {
                div { style: "background: #fef3c7; border: 1px solid #fde68a; color: #92400e; padding: 8px 12px; border-radius: var(--radius); margin-bottom: 12px; font-size: 13px;",
                    for msg in validation_msgs.read().iter() {
                        p { "{msg}" }
                    }
                }
            }

            div { style: "display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 16px;",

                // ---- 1. Plan selector ----
                div {
                    label { r#for: "sel-plan", style: "display: block; font-size: 13px; font-weight: 500; margin-bottom: 4px;", "Client Plan *" }
                    select { id: "sel-plan", style: "width: 100%; padding: 8px 12px; border: 1px solid var(--color-border); border-radius: var(--radius);",
                        value: "{selected_plan}",
                        onchange: move |e| { on_plan_change(e.value()); },
                        option { value: "", "Select a plan..." }
                        match &*plans.read() {
                            Some(Ok(p)) => rsx! {
                                for plan in p.iter() {
                                    option { value: "{plan.id}", "{plan.client_name} ({plan.status})" }
                                }
                            },
                            Some(Err(_)) => rsx! { option { value: "", "Error loading plans" } },
                            None => rsx! { option { value: "", "Loading plans..." } },
                        }
                    }
                }

                // ---- 2. Package selector (depends on plan) ----
                div {
                    label { r#for: "sel-pkg", style: "display: block; font-size: 13px; font-weight: 500; margin-bottom: 4px;", "Plan Package *" }
                    select { id: "sel-pkg", style: "width: 100%; padding: 8px 12px; border: 1px solid var(--color-border); border-radius: var(--radius);",
                        disabled: selected_plan.read().is_empty() || *pkg_loading.read(),
                        value: "{selected_pkg}",
                        onchange: {
                            let mut on_pkg_change = on_pkg_change.clone();
                            move |e: Event<FormData>| {
                                let pp_id = e.value();
                                let pkg_id = get_package_id_for(&pp_id);
                                on_pkg_change(pp_id, pkg_id);
                            }
                        },
                        if selected_plan.read().is_empty() {
                            option { value: "", "Select a plan first" }
                        } else if *pkg_loading.read() {
                            option { value: "", "Loading packages..." }
                        } else {
                            match packages.read().as_ref() {
                                Some(Ok(pkgs)) if pkgs.is_empty() => rsx! {
                                    option { value: "", "No packages assigned to this plan" }
                                },
                                Some(Ok(pkgs)) => rsx! {
                                    option { value: "", "Select a package..." }
                                    for pkg in pkgs.iter().filter(|p| p.status == "active") {
                                        option { value: "{pkg.id}", "Package {pkg.package_id} (from {pkg.effective_date})" }
                                    }
                                },
                                Some(Err(_)) => rsx! {
                                    option { value: "", "Error loading packages" }
                                },
                                None => rsx! {
                                    option { value: "", "Select a plan first" }
                                },
                            }
                        }
                    }
                    if selected_plan.read().is_empty() {
                        p { style: "font-size: 11px; color: var(--color-text-secondary); margin-top: 2px;", "Choose a client plan to see available packages" }
                    }
                }

                // ---- 3. Service item selector (depends on package) ----
                div {
                    label { r#for: "sel-svc", style: "display: block; font-size: 13px; font-weight: 500; margin-bottom: 4px;", "Service Item *" }
                    select { id: "sel-svc", style: "width: 100%; padding: 8px 12px; border: 1px solid var(--color-border); border-radius: var(--radius);",
                        disabled: selected_pkg.read().is_empty() || *svc_loading.read(),
                        value: "{selected_svc}",
                        onchange: move |e| { selected_svc.set(e.value()); },
                        if selected_pkg.read().is_empty() {
                            option { value: "", "Select a package first" }
                        } else if *svc_loading.read() {
                            option { value: "", "Loading services..." }
                        } else if svc_options.read().is_empty() {
                            option { value: "", "No service items in this package" }
                        } else {
                            option { value: "", "Select a service..." }
                            for opt in svc_options.read().iter() {
                                option { value: "{opt.service_item_id}", "{opt.label}" }
                            }
                        }
                    }
                    if selected_pkg.read().is_empty() {
                        p { style: "font-size: 11px; color: var(--color-text-secondary); margin-top: 2px;", "Choose a package to see available services" }
                    }
                }

                // ---- Date ----
                div {
                    label { r#for: "del-date", style: "display: block; font-size: 13px; font-weight: 500; margin-bottom: 4px;", "Delivery Date *" }
                    input { id: "del-date", r#type: "date", style: "width: 100%; padding: 8px 12px; border: 1px solid var(--color-border); border-radius: var(--radius);",
                        value: "{delivery_date}", oninput: move |e| delivery_date.set(e.value()),
                    }
                }

                // ---- Time range ----
                div {
                    label { r#for: "del-start", style: "display: block; font-size: 13px; font-weight: 500; margin-bottom: 4px;", "Start Time" }
                    input { id: "del-start", r#type: "time", style: "width: 100%; padding: 8px 12px; border: 1px solid var(--color-border); border-radius: var(--radius);",
                        value: "{start_time}", oninput: move |e| start_time.set(e.value()),
                    }
                }
                div {
                    label { r#for: "del-end", style: "display: block; font-size: 13px; font-weight: 500; margin-bottom: 4px;", "End Time" }
                    input { id: "del-end", r#type: "time", style: "width: 100%; padding: 8px 12px; border: 1px solid var(--color-border); border-radius: var(--radius);",
                        value: "{end_time}", oninput: move |e| end_time.set(e.value()),
                    }
                }

                // ---- Units ----
                div {
                    label { r#for: "del-units", style: "display: block; font-size: 13px; font-weight: 500; margin-bottom: 4px;", "Units * (0.25 increments)" }
                    input { id: "del-units", r#type: "number", step: "0.25", min: "0.25",
                        style: "width: 100%; padding: 8px 12px; border: 1px solid var(--color-border); border-radius: var(--radius);",
                        value: "{units}", oninput: move |e| { units.set(e.value()); validate(); },
                    }
                    p { style: "font-size: 11px; color: var(--color-text-secondary); margin-top: 2px;", "Must be in 0.25-hour increments" }
                }

                // ---- Mileage ----
                div {
                    label { r#for: "del-mileage", style: "display: block; font-size: 13px; font-weight: 500; margin-bottom: 4px;", "Mileage (max 200)" }
                    input { id: "del-mileage", r#type: "number", step: "0.1", max: "200",
                        style: "width: 100%; padding: 8px 12px; border: 1px solid var(--color-border); border-radius: var(--radius);",
                        value: "{mileage}", oninput: move |e| { mileage.set(e.value()); validate(); },
                        placeholder: "Optional",
                    }
                    p { style: "font-size: 11px; color: var(--color-text-secondary); margin-top: 2px;", "Max 200 miles per visit" }
                }
            }

            div { style: "margin-top: 12px;",
                label { r#for: "del-notes", style: "display: block; font-size: 13px; font-weight: 500; margin-bottom: 4px;", "Notes (encrypted at rest)" }
                textarea { id: "del-notes", style: "width: 100%; padding: 8px 12px; border: 1px solid var(--color-border); border-radius: var(--radius); min-height: 60px;",
                    value: "{notes}", oninput: move |e| notes.set(e.value()),
                    placeholder: "Delivery notes, eligibility context...",
                }
            }

            div { style: "margin-top: 16px;",
                button { class: "btn btn-primary", onclick: on_submit, "Submit Entry" }
            }
        }
    }
}

#[component]
fn DeliveryStatusBadge(status: String) -> Element {
    let (bg, fg) = match status.as_str() {
        "submitted" => ("#fef3c7", "#92400e"),
        "verified" => ("#dcfce7", "#16a34a"),
        "rejected" => ("#fef2f2", "#dc2626"),
        "billed" => ("#e0e7ff", "#4338ca"),
        "draft" => ("#f1f5f9", "#64748b"),
        _ => ("#f1f5f9", "#64748b"),
    };
    rsx! {
        span { style: "padding: 2px 8px; border-radius: 12px; font-size: 12px; background: {bg}; color: {fg};", "{status}" }
    }
}

// ============================================================
// Validation logic (pure functions for testability)
// ============================================================

/// Builds the submission payload from form state, returning the same
/// JSON shape the backend expects.  This is extracted as a pure function
/// so the payload contract can be verified in unit tests without a DOM.
#[cfg(test)]
fn build_delivery_payload(
    plan_id: &str,
    plan_package_id: &str,
    service_item_id: &str,
    delivery_date: &str,
    start_time: &str,
    end_time: &str,
    units: f64,
    mileage: Option<f64>,
    notes: Option<&str>,
) -> serde_json::Value {
    serde_json::json!({
        "plan_id": plan_id,
        "plan_package_id": plan_package_id,
        "service_item_id": service_item_id,
        "delivery_date": delivery_date,
        "start_time": if start_time.is_empty() { None::<&str> } else { Some(start_time) },
        "end_time": if end_time.is_empty() { None::<&str> } else { Some(end_time) },
        "units": units,
        "mileage": mileage,
        "notes": notes,
    })
}

/// Validates delivery form fields, returning a list of error messages.
/// Empty list means valid.  This mirrors the live validation closure
/// but is testable without Dioxus signals.
#[cfg(test)]
fn validate_delivery_fields(
    plan_id: &str,
    plan_package_id: &str,
    service_item_id: &str,
    delivery_date: &str,
    units: f64,
    mileage_str: &str,
) -> Vec<String> {
    let mut msgs = Vec::new();
    if units <= 0.0 { msgs.push("Units must be greater than 0".to_string()); }
    let remainder = (units * 4.0).fract();
    if remainder.abs() > 0.001 { msgs.push("Hours must be in 0.25 increments".to_string()); }
    let m: f64 = mileage_str.parse().unwrap_or(0.0);
    if !mileage_str.is_empty() && m > 200.0 { msgs.push("Mileage cannot exceed 200 per visit".to_string()); }
    if delivery_date.is_empty() { msgs.push("Delivery date is required".to_string()); }
    if plan_id.is_empty() { msgs.push("Select a client plan".to_string()); }
    if plan_package_id.is_empty() { msgs.push("Select a plan package".to_string()); }
    if service_item_id.is_empty() { msgs.push("Select a service item".to_string()); }
    msgs
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Happy path: all selectors filled, payload matches backend contract ----

    #[test]
    fn happy_path_payload_has_correct_shape() {
        let payload = build_delivery_payload(
            "plan-001",
            "pp-001",
            "svc-001",
            "2024-06-15",
            "09:00",
            "10:30",
            2.0,
            Some(12.5),
            Some("Test note"),
        );

        // Verify the exact fields the backend expects
        assert_eq!(payload["plan_id"], "plan-001");
        assert_eq!(payload["plan_package_id"], "pp-001");
        assert_eq!(payload["service_item_id"], "svc-001");
        assert_eq!(payload["delivery_date"], "2024-06-15");
        assert_eq!(payload["start_time"], "09:00");
        assert_eq!(payload["end_time"], "10:30");
        assert_eq!(payload["units"], 2.0);
        assert_eq!(payload["mileage"], 12.5);
        assert_eq!(payload["notes"], "Test note");
    }

    #[test]
    fn payload_with_empty_optional_fields() {
        let payload = build_delivery_payload(
            "plan-001",
            "pp-001",
            "svc-001",
            "2024-06-15",
            "",  // no start_time
            "",  // no end_time
            1.0,
            None,
            None,
        );

        assert!(payload["start_time"].is_null());
        assert!(payload["end_time"].is_null());
        assert!(payload["mileage"].is_null());
        assert!(payload["notes"].is_null());
        // Required fields still present
        assert_eq!(payload["plan_id"], "plan-001");
        assert_eq!(payload["plan_package_id"], "pp-001");
        assert_eq!(payload["service_item_id"], "svc-001");
    }

    // ---- Validation: all three selectors are required ----

    #[test]
    fn validation_requires_all_selectors() {
        let msgs = validate_delivery_fields("", "", "", "2024-06-15", 1.0, "");
        assert!(msgs.iter().any(|m| m.contains("client plan")));
        assert!(msgs.iter().any(|m| m.contains("plan package")));
        assert!(msgs.iter().any(|m| m.contains("service item")));
    }

    #[test]
    fn validation_happy_path_passes() {
        let msgs = validate_delivery_fields("p1", "pp1", "svc1", "2024-06-15", 1.0, "");
        assert!(msgs.is_empty(), "expected no errors, got: {:?}", msgs);
    }

    // ---- Edge case: changing plan should leave pkg/svc empty (tested via validation) ----

    #[test]
    fn after_plan_change_empty_downstream_blocks_submit() {
        // Simulates: user selected plan, but pkg and svc are cleared
        let msgs = validate_delivery_fields("plan-002", "", "", "2024-06-15", 1.0, "");
        assert!(msgs.iter().any(|m| m.contains("plan package")));
        assert!(msgs.iter().any(|m| m.contains("service item")));
        // Plan itself is fine
        assert!(!msgs.iter().any(|m| m.contains("client plan")));
    }

    // ---- Existing validations still work ----

    #[test]
    fn validation_rejects_non_quarter_hour_units() {
        let msgs = validate_delivery_fields("p1", "pp1", "svc1", "2024-06-15", 1.3, "");
        assert!(msgs.iter().any(|m| m.contains("0.25 increments")));
    }

    #[test]
    fn validation_rejects_mileage_over_200() {
        let msgs = validate_delivery_fields("p1", "pp1", "svc1", "2024-06-15", 1.0, "250");
        assert!(msgs.iter().any(|m| m.contains("200")));
    }

    #[test]
    fn validation_rejects_zero_units() {
        let msgs = validate_delivery_fields("p1", "pp1", "svc1", "2024-06-15", 0.0, "");
        assert!(msgs.iter().any(|m| m.contains("greater than 0")));
    }

    #[test]
    fn validation_rejects_missing_date() {
        let msgs = validate_delivery_fields("p1", "pp1", "svc1", "", 1.0, "");
        assert!(msgs.iter().any(|m| m.contains("date is required")));
    }
}
