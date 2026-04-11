use dioxus::prelude::*;
use crate::components::loading::Loading;
use crate::components::error_state::ErrorState;
use crate::components::permission_denied::PermissionDenied;
use crate::models::{OrgRow, DepartmentRow, RoleRow};
use crate::services::ApiClient;
use crate::state::{AuthState, perms};

#[component]
pub fn Admin() -> Element {
    let auth: Signal<AuthState> = use_context();

    if !auth.read().has_permission(perms::MENU_ADMIN) {
        return rsx! { PermissionDenied { action: "access administration".to_string() } };
    }

    let token = auth.read().token.clone();

    let orgs = use_resource(move || {
        let t = token.clone();
        async move { ApiClient::get::<Vec<OrgRow>>("/admin/org/", t.as_deref()).await }
    });

    let org_id = auth.read().user.as_ref().map(|u| u.org_id.clone()).unwrap_or_default();
    let token2 = auth.read().token.clone();
    let oid = org_id.clone();
    let departments = use_resource(move || {
        let t = token2.clone();
        let o = oid.clone();
        async move {
            ApiClient::get::<Vec<DepartmentRow>>(&format!("/admin/org/{}/departments", o), t.as_deref()).await
        }
    });

    let token3 = auth.read().token.clone();
    let roles = use_resource(move || {
        let t = token3.clone();
        async move { ApiClient::get::<Vec<RoleRow>>("/roles/", t.as_deref()).await }
    });

    rsx! {
        div {
            div { class: "page-header",
                h1 { "Administration" }
                p { "Manage organization settings, departments, projects, roles, and system configuration." }
            }

            div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 20px;",
                // Organizations
                div { class: "card",
                    div { style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px;",
                        h3 { "Organizations" }
                        if auth.read().has_permission(perms::ACTION_MANAGE_ORG) {
                            button { class: "btn btn-primary", style: "font-size: 13px; padding: 6px 12px;", "Add" }
                        }
                    }
                    match &*orgs.read() {
                        None => rsx! { Loading { message: Some("Loading organizations...".to_string()) } },
                        Some(Err(e)) => rsx! { ErrorState { message: "Failed to load".to_string(), detail: Some(e.clone()) } },
                        Some(Ok(list)) => rsx! {
                            for org in list.iter() {
                                div {
                                    key: "{org.id}",
                                    style: "padding: 10px 0; border-bottom: 1px solid var(--color-border);",
                                    div { style: "font-weight: 500;", "{org.name}" }
                                    div { style: "font-size: 12px; color: var(--color-text-secondary);", "Status: {org.status}" }
                                }
                            }
                        }
                    }
                }

                // Departments
                div { class: "card",
                    div { style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px;",
                        h3 { "Departments" }
                        if auth.read().has_permission(perms::ACTION_MANAGE_DEPT) {
                            button { class: "btn btn-primary", style: "font-size: 13px; padding: 6px 12px;", "Add" }
                        }
                    }
                    match &*departments.read() {
                        None => rsx! { Loading { message: Some("Loading departments...".to_string()) } },
                        Some(Err(e)) => rsx! { ErrorState { message: "Failed to load".to_string(), detail: Some(e.clone()) } },
                        Some(Ok(list)) => rsx! {
                            if list.is_empty() {
                                p { style: "color: var(--color-text-secondary);", "No departments configured." }
                            }
                            for dept in list.iter() {
                                div {
                                    key: "{dept.id}",
                                    style: "padding: 10px 0; border-bottom: 1px solid var(--color-border);",
                                    div { style: "font-weight: 500;", "{dept.name}" }
                                    div { style: "font-size: 12px; color: var(--color-text-secondary);", "Status: {dept.status}" }
                                }
                            }
                        }
                    }
                }

                // Roles
                div { class: "card", style: "grid-column: 1 / -1;",
                    div { style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px;",
                        h3 { "Roles" }
                        if auth.read().has_permission(perms::ACTION_MANAGE_ROLES) {
                            button { class: "btn btn-primary", style: "font-size: 13px; padding: 6px 12px;", "Create Role" }
                        }
                    }
                    match &*roles.read() {
                        None => rsx! { Loading { message: Some("Loading roles...".to_string()) } },
                        Some(Err(e)) => rsx! { ErrorState { message: "Failed to load".to_string(), detail: Some(e.clone()) } },
                        Some(Ok(list)) => rsx! {
                            div { style: "display: grid; grid-template-columns: repeat(auto-fill, minmax(280px, 1fr)); gap: 12px;",
                                for role in list.iter() {
                                    div {
                                        key: "{role.id}",
                                        style: "padding: 12px; border: 1px solid var(--color-border); border-radius: var(--radius);",
                                        div { style: "display: flex; justify-content: space-between; align-items: center;",
                                            span { style: "font-weight: 500;", "{role.name}" }
                                            if role.is_system {
                                                span { style: "font-size: 11px; padding: 2px 8px; background: #e0e7ff; color: #4338ca; border-radius: 10px;", "System" }
                                            }
                                        }
                                        if let Some(ref desc) = role.description {
                                            p { style: "font-size: 12px; color: var(--color-text-secondary); margin-top: 4px;", "{desc}" }
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
