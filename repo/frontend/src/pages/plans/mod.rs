use dioxus::prelude::*;
use crate::components::loading::Loading;
use crate::components::error_state::ErrorState;
use crate::components::permission_denied::PermissionDenied;
use crate::models::{ClientPlanRow, PackageRow};
use crate::services::ApiClient;
use crate::state::{AuthState, perms};

#[component]
pub fn Plans() -> Element {
    let auth: Signal<AuthState> = use_context();

    if !auth.read().has_permission(perms::MENU_PLANS) {
        return rsx! { PermissionDenied { action: "view client plans".to_string() } };
    }

    let token = auth.read().token.clone();
    let mut refresh = use_signal(|| 0u32);
    let mut show_form = use_signal(|| false);

    let plans = use_resource(move || {
        let t = token.clone();
        let _r = refresh.read();
        async move { ApiClient::get::<Vec<ClientPlanRow>>("/plans/", t.as_deref()).await }
    });

    // Form state
    let mut client_name = use_signal(String::new);
    let mut start_date = use_signal(String::new);
    let mut end_date = use_signal(String::new);
    let mut form_error = use_signal(|| None::<String>);
    let mut form_success = use_signal(|| false);

    let can_create = auth.read().has_permission("action.plans.create");

    let on_create = move |_| {
        let t = auth.read().token.clone();
        let cn = client_name.read().clone();
        let sd = start_date.read().clone();
        let ed = end_date.read().clone();

        spawn(async move {
            form_error.set(None);
            form_success.set(false);
            let body = serde_json::json!({
                "client_name": cn,
                "start_date": sd,
                "end_date": if ed.is_empty() { None::<String> } else { Some(ed) },
            });
            match ApiClient::post::<ClientPlanRow, _>("/plans/", &body, t.as_deref()).await {
                Ok(_) => {
                    form_success.set(true);
                    client_name.set(String::new());
                    start_date.set(String::new());
                    end_date.set(String::new());
                    show_form.set(false);
                    let r = *refresh.read();
                    refresh.set(r + 1);
                }
                Err(e) => form_error.set(Some(e)),
            }
        });
    };

    rsx! {
        div {
            div { class: "page-header",
                h1 { "Client Plans" }
                p { "Create and manage client service plans. Assign packages to define authorized services and billing rules." }
            }

            if can_create {
                div { style: "margin-bottom: 16px; display: flex; gap: 12px;",
                    button { class: "btn btn-primary", onclick: move |_| show_form.toggle(),
                        if *show_form.read() { "Cancel" } else { "New Plan" }
                    }
                }
            }

            if *show_form.read() {
                div { class: "card", style: "margin-bottom: 20px;",
                    h3 { style: "margin-bottom: 16px;", "Create Client Plan" }
                    if let Some(err) = form_error.read().as_ref() {
                        div { style: "background: #fef2f2; border: 1px solid #fecaca; color: #dc2626; padding: 8px 12px; border-radius: var(--radius); margin-bottom: 12px; font-size: 13px;", "{err}" }
                    }
                    div { style: "display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 16px;",
                        div {
                            label { style: "display: block; font-size: 13px; font-weight: 500; margin-bottom: 4px;", "Client Name *" }
                            input { style: "width: 100%; padding: 8px 12px; border: 1px solid var(--color-border); border-radius: var(--radius);",
                                value: "{client_name}", oninput: move |e| client_name.set(e.value()), placeholder: "Full client name",
                            }
                        }
                        div {
                            label { style: "display: block; font-size: 13px; font-weight: 500; margin-bottom: 4px;", "Start Date *" }
                            input { r#type: "date", style: "width: 100%; padding: 8px 12px; border: 1px solid var(--color-border); border-radius: var(--radius);",
                                value: "{start_date}", oninput: move |e| start_date.set(e.value()),
                            }
                        }
                        div {
                            label { style: "display: block; font-size: 13px; font-weight: 500; margin-bottom: 4px;", "End Date" }
                            input { r#type: "date", style: "width: 100%; padding: 8px 12px; border: 1px solid var(--color-border); border-radius: var(--radius);",
                                value: "{end_date}", oninput: move |e| end_date.set(e.value()),
                            }
                        }
                    }
                    div { style: "margin-top: 16px;",
                        button { class: "btn btn-primary", onclick: on_create, "Create Plan" }
                    }
                }
            }

            if *form_success.read() {
                div { style: "background: #f0fdf4; border: 1px solid #bbf7d0; color: #16a34a; padding: 8px 12px; border-radius: var(--radius); margin-bottom: 16px; font-size: 13px;", "Plan created successfully" }
            }

            div { class: "card",
                match &*plans.read() {
                    None => rsx! { Loading {} },
                    Some(Err(e)) => rsx! { ErrorState { message: "Failed to load plans".to_string(), detail: Some(e.clone()) } },
                    Some(Ok(plan_list)) => rsx! {
                        table { style: "width: 100%; border-collapse: collapse;",
                            thead {
                                tr { style: "border-bottom: 2px solid var(--color-border); text-align: left;",
                                    th { style: "padding: 8px 12px; font-size: 12px; text-transform: uppercase; color: var(--color-text-secondary);", "Client" }
                                    th { style: "padding: 8px 12px; font-size: 12px; text-transform: uppercase; color: var(--color-text-secondary);", "Status" }
                                    th { style: "padding: 8px 12px; font-size: 12px; text-transform: uppercase; color: var(--color-text-secondary);", "Start" }
                                    th { style: "padding: 8px 12px; font-size: 12px; text-transform: uppercase; color: var(--color-text-secondary);", "End" }
                                }
                            }
                            tbody {
                                for plan in plan_list.iter() {
                                    tr { style: "border-bottom: 1px solid var(--color-border);", key: "{plan.id}",
                                        td { style: "padding: 10px 12px; font-weight: 500;", "{plan.client_name}" }
                                        td { style: "padding: 10px 12px;",
                                            PlanStatusBadge { status: plan.status.clone() }
                                        }
                                        td { style: "padding: 10px 12px; color: var(--color-text-secondary);", "{plan.start_date}" }
                                        td { style: "padding: 10px 12px; color: var(--color-text-secondary);",
                                            "{plan.end_date.as_deref().unwrap_or(\"-\")}"
                                        }
                                    }
                                }
                            }
                        }
                        if plan_list.is_empty() {
                            p { style: "text-align: center; padding: 24px; color: var(--color-text-secondary);", "No client plans yet." }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn PlanStatusBadge(status: String) -> Element {
    let (bg, fg) = match status.as_str() {
        "active" => ("#dcfce7", "#16a34a"),
        "draft" => ("#f1f5f9", "#64748b"),
        "paused" => ("#fef3c7", "#d97706"),
        "completed" => ("#e0e7ff", "#4338ca"),
        "cancelled" => ("#fef2f2", "#dc2626"),
        _ => ("#f1f5f9", "#64748b"),
    };
    rsx! {
        span { style: "padding: 2px 8px; border-radius: 12px; font-size: 12px; background: {bg}; color: {fg};", "{status}" }
    }
}
