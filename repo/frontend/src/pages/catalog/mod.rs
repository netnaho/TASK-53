use dioxus::prelude::*;
use crate::components::loading::Loading;
use crate::components::error_state::ErrorState;
use crate::components::permission_denied::PermissionDenied;
use crate::models::{ServiceItemRow, PackageRow};
use crate::services::ApiClient;
use crate::state::{AuthState, perms};

#[component]
pub fn Catalog() -> Element {
    let auth: Signal<AuthState> = use_context();

    if !auth.read().has_permission(perms::MENU_CATALOG) {
        return rsx! { PermissionDenied { action: "view service catalog".to_string() } };
    }

    let mut active_tab = use_signal(|| "services");

    rsx! {
        div {
            div { class: "page-header",
                h1 { "Service Catalog" }
                p { "Define billable services, set unit rates, and manage service packages with billing rules." }
            }

            div { style: "display: flex; gap: 0; border-bottom: 2px solid var(--color-border); margin-bottom: 20px;",
                TabBtn { label: "Services", active: *active_tab.read() == "services", onclick: move |_| active_tab.set("services") }
                TabBtn { label: "Packages", active: *active_tab.read() == "packages", onclick: move |_| active_tab.set("packages") }
            }

            if *active_tab.read() == "services" {
                ServicesTab {}
            } else {
                PackagesTab {}
            }
        }
    }
}

#[component]
fn TabBtn(label: &'static str, active: bool, onclick: EventHandler<MouseEvent>) -> Element {
    let style = if active {
        "padding: 10px 20px; border: none; background: none; font-size: 14px; font-weight: 600; cursor: pointer; border-bottom: 2px solid var(--color-primary); color: var(--color-primary); margin-bottom: -2px;"
    } else {
        "padding: 10px 20px; border: none; background: none; font-size: 14px; cursor: pointer; color: var(--color-text-secondary); margin-bottom: -2px;"
    };
    rsx! { button { style: "{style}", onclick: move |e| onclick.call(e), "{label}" } }
}

#[component]
fn ServicesTab() -> Element {
    let auth: Signal<AuthState> = use_context();
    let token = auth.read().token.clone();
    let mut refresh = use_signal(|| 0u32);
    let mut show_form = use_signal(|| false);

    let services = use_resource(move || {
        let t = token.clone();
        let _r = refresh.read();
        async move { ApiClient::get::<Vec<ServiceItemRow>>("/catalog/?active_only=false", t.as_deref()).await }
    });

    let mut code = use_signal(String::new);
    let mut name = use_signal(String::new);
    let mut category = use_signal(|| "nursing".to_string());
    let mut unit_type = use_signal(|| "visit".to_string());
    let mut rate = use_signal(|| "0.00".to_string());
    let mut description = use_signal(String::new);
    let mut form_error = use_signal(|| None::<String>);
    let mut form_success = use_signal(|| false);

    let on_submit = move |_| {
        let t = auth.read().token.clone();
        let c = code.read().clone();
        let n = name.read().clone();
        let cat = category.read().clone();
        let ut = unit_type.read().clone();
        let r: f64 = rate.read().parse().unwrap_or(0.0);
        let desc = description.read().clone();

        spawn(async move {
            form_error.set(None);
            form_success.set(false);
            let body = serde_json::json!({
                "code": c, "name": n, "category": cat,
                "unit_type": ut, "default_rate": r,
                "description": if desc.is_empty() { None } else { Some(desc) }
            });
            match ApiClient::post::<ServiceItemRow, _>("/catalog/", &body, t.as_deref()).await {
                Ok(_) => {
                    form_success.set(true);
                    code.set(String::new());
                    name.set(String::new());
                    description.set(String::new());
                    rate.set("0.00".to_string());
                    *refresh.write() += 1;
                }
                Err(e) => form_error.set(Some(e)),
            }
        });
    };

    rsx! {
        div {
            if auth.read().has_permission("action.catalog.create") {
                div { style: "margin-bottom: 16px;",
                    button { class: "btn btn-primary", onclick: move |_| show_form.toggle(),
                        if *show_form.read() { "Cancel" } else { "Add Service" }
                    }
                }
            }

            if *show_form.read() {
                div { class: "card", style: "margin-bottom: 20px;",
                    h3 { style: "margin-bottom: 16px;", "New Service Item" }
                    if form_error.read().is_some() {
                        div { style: "background: #fef2f2; border: 1px solid #fecaca; color: #dc2626; padding: 8px 12px; border-radius: var(--radius); margin-bottom: 12px; font-size: 13px;",
                            "Error creating service"
                        }
                    }
                    if *form_success.read() {
                        div { style: "background: #f0fdf4; border: 1px solid #bbf7d0; color: #16a34a; padding: 8px 12px; border-radius: var(--radius); margin-bottom: 12px; font-size: 13px;", "Service created successfully" }
                    }
                    div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 16px;",
                        FormField { label: "Code", value: code, placeholder: "e.g. SVC-NURS-001" }
                        FormField { label: "Name", value: name, placeholder: "e.g. Skilled Nursing Visit" }
                        div {
                            label { style: "display: block; font-size: 13px; font-weight: 500; margin-bottom: 4px;", "Category" }
                            select { style: "width: 100%; padding: 8px 12px; border: 1px solid var(--color-border); border-radius: var(--radius);",
                                value: "{category}", onchange: move |e| category.set(e.value()),
                                option { value: "nursing", "Nursing" }
                                option { value: "rehab", "Rehab" }
                                option { value: "meals", "Meals" }
                                option { value: "companionship", "Companionship" }
                                option { value: "transportation", "Transportation" }
                                option { value: "other", "Other" }
                            }
                        }
                        div {
                            label { style: "display: block; font-size: 13px; font-weight: 500; margin-bottom: 4px;", "Unit Type" }
                            select { style: "width: 100%; padding: 8px 12px; border: 1px solid var(--color-border); border-radius: var(--radius);",
                                value: "{unit_type}", onchange: move |e| unit_type.set(e.value()),
                                option { value: "visit", "Per Visit" }
                                option { value: "hour", "Per Hour" }
                                option { value: "mile", "Per Mile" }
                                option { value: "meal", "Per Meal" }
                                option { value: "session", "Per Session" }
                            }
                        }
                        FormField { label: "Default Rate", value: rate, placeholder: "0.00" }
                    }
                    div { style: "margin-top: 12px;",
                        label { style: "display: block; font-size: 13px; font-weight: 500; margin-bottom: 4px;", "Description" }
                        textarea { style: "width: 100%; padding: 8px 12px; border: 1px solid var(--color-border); border-radius: var(--radius); min-height: 60px;",
                            value: "{description}", oninput: move |e| description.set(e.value()),
                        }
                    }
                    div { style: "margin-top: 16px;",
                        button { class: "btn btn-primary", onclick: on_submit, "Create Service" }
                    }
                }
            }

            div { class: "card",
                {
                    match &*services.read() {
                        None => rsx! { Loading {} },
                        Some(Err(e)) => rsx! { ErrorState { message: "Failed to load services".to_string(), detail: Some(e.clone()) } },
                        Some(Ok(items)) => rsx! {
                            table { style: "width: 100%; border-collapse: collapse;",
                                thead {
                                tr { style: "border-bottom: 2px solid var(--color-border); text-align: left;",
                                    th { style: "padding: 8px 12px; font-size: 12px; text-transform: uppercase; color: var(--color-text-secondary);", "Code" }
                                    th { style: "padding: 8px 12px; font-size: 12px; text-transform: uppercase; color: var(--color-text-secondary);", "Name" }
                                    th { style: "padding: 8px 12px; font-size: 12px; text-transform: uppercase; color: var(--color-text-secondary);", "Category" }
                                    th { style: "padding: 8px 12px; font-size: 12px; text-transform: uppercase; color: var(--color-text-secondary);", "Unit" }
                                    th { style: "padding: 8px 12px; font-size: 12px; text-transform: uppercase; color: var(--color-text-secondary);", "Rate" }
                                    th { style: "padding: 8px 12px; font-size: 12px; text-transform: uppercase; color: var(--color-text-secondary);", "Status" }
                                }
                            }
                            tbody {
                                if items.is_empty() {
                                    tr {
                                        td { colspan: "6", style: "text-align: center; padding: 24px; color: var(--color-text-secondary);", "No service items yet. Add your first service above." }
                                    }
                                }
                                for item in items.iter() {
                                    tr { style: "border-bottom: 1px solid var(--color-border);", key: "{item.id}",
                                        td { style: "padding: 10px 12px; font-family: monospace; font-size: 13px;", "{item.code}" }
                                        td { style: "padding: 10px 12px; font-weight: 500;", "{item.name}" }
                                        td { style: "padding: 10px 12px;",
                                            span { style: "padding: 2px 8px; border-radius: 12px; font-size: 12px; background: #e0e7ff; color: #4338ca;", "{item.category}" }
                                        }
                                        td { style: "padding: 10px 12px; color: var(--color-text-secondary);", "{item.unit_type}" }
                                        td { style: "padding: 10px 12px; font-weight: 500;", "{item.default_rate}" }
                                        td { style: "padding: 10px 12px;",
                                            {
                                                let badge_style = if item.is_active {
                                                    "padding: 2px 8px; border-radius: 12px; font-size: 12px; background: #dcfce7; color: #16a34a;"
                                                } else {
                                                    "padding: 2px 8px; border-radius: 12px; font-size: 12px; background: #fef3c7; color: #d97706;"
                                                };
                                                let badge_text = if item.is_active { "Active" } else { "Inactive" };
                                                rsx! { span { style: "{badge_style}", "{badge_text}" } }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                }
            }
        }
    }
}

#[component]
fn PackagesTab() -> Element {
    let auth: Signal<AuthState> = use_context();
    let token = auth.read().token.clone();

    let packages = use_resource(move || {
        let t = token.clone();
        async move { ApiClient::get::<Vec<PackageRow>>("/packages/?active_only=false", t.as_deref()).await }
    });

    rsx! {
        div {
            if auth.read().has_permission("action.packages.create") {
                div { style: "margin-bottom: 16px;",
                    button { class: "btn btn-primary", "Create Package" }
                }
            }

            div { class: "card",
                {
                    match &*packages.read() {
                        None => rsx! { Loading {} },
                        Some(Err(e)) => rsx! { ErrorState { message: "Failed to load packages".to_string(), detail: Some(e.clone()) } },
                        Some(Ok(pkgs)) => rsx! {
                        div { style: "display: grid; grid-template-columns: repeat(auto-fill, minmax(300px, 1fr)); gap: 16px;",
                            if pkgs.is_empty() {
                                p { style: "text-align: center; padding: 24px; color: var(--color-text-secondary);", "No packages yet." }
                            }
                            for pkg in pkgs.iter() {
                                div { key: "{pkg.id}", style: "padding: 16px; border: 1px solid var(--color-border); border-radius: var(--radius);",
                                    div { style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 8px;",
                                        span { style: "font-weight: 600;", "{pkg.name}" }
                                        span { style: "font-family: monospace; font-size: 12px; color: var(--color-text-secondary);", "{pkg.code}" }
                                    }
                                    {
                                        let badge_style = if pkg.is_active {
                                            "padding: 2px 8px; border-radius: 12px; font-size: 12px; background: #dcfce7; color: #16a34a;"
                                        } else {
                                            "padding: 2px 8px; border-radius: 12px; font-size: 12px; background: #fef3c7; color: #d97706;"
                                        };
                                        let badge_text = if pkg.is_active { "Active" } else { "Inactive" };
                                        rsx! { span { style: "{badge_style}", "{badge_text}" } }
                                    }
                                }
                            }
                        }
                    }
                }
                }
            }
        }
    }
}

#[component]
fn FormField(label: &'static str, value: Signal<String>, placeholder: &'static str) -> Element {
    rsx! {
        div {
            label { style: "display: block; font-size: 13px; font-weight: 500; margin-bottom: 4px;", "{label}" }
            input {
                style: "width: 100%; padding: 8px 12px; border: 1px solid var(--color-border); border-radius: var(--radius);",
                value: "{value}", oninput: move |e| value.set(e.value()), placeholder: "{placeholder}",
            }
        }
    }
}
