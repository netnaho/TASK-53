use dioxus::prelude::*;
use crate::components::loading::Loading;
use crate::components::error_state::ErrorState;
use crate::components::permission_denied::PermissionDenied;
use crate::models::{PaginatedResponse, UserRow};
use crate::services::ApiClient;
use crate::state::{AuthState, perms};

#[component]
pub fn Users() -> Element {
    let auth: Signal<AuthState> = use_context();

    if !auth.read().has_permission(perms::MENU_USERS) {
        return rsx! { PermissionDenied { action: "view user management".to_string() } };
    }

    let token = auth.read().token.clone();
    let users_resource = use_resource(move || {
        let t = token.clone();
        async move {
            ApiClient::get::<PaginatedResponse<UserRow>>("/users?per_page=50", t.as_deref()).await
        }
    });

    rsx! {
        div {
            div { class: "page-header",
                h1 { "User Management" }
                p { "View and manage user accounts, role assignments, and data scope access." }
            }

            if auth.read().has_permission(perms::ACTION_CREATE_USER) {
                div { style: "margin-bottom: 20px;",
                    button { class: "btn btn-primary", "Create User" }
                }
            }

            match &*users_resource.read() {
                None => rsx! { Loading {} },
                Some(Err(e)) => rsx! { ErrorState { message: "Failed to load users".to_string(), detail: Some(e.clone()) } },
                Some(Ok(resp)) => rsx! {
                    div { class: "card",
                        div { style: "margin-bottom: 12px; display: flex; justify-content: space-between; align-items: center;",
                            h3 { "Users ({resp.total} total)" }
                        }
                        table { style: "width: 100%; border-collapse: collapse;",
                            thead {
                                tr { style: "border-bottom: 2px solid var(--color-border); text-align: left;",
                                    th { style: "padding: 8px 12px; font-size: 12px; text-transform: uppercase; color: var(--color-text-secondary);", "Username" }
                                    th { style: "padding: 8px 12px; font-size: 12px; text-transform: uppercase; color: var(--color-text-secondary);", "Email" }
                                    th { style: "padding: 8px 12px; font-size: 12px; text-transform: uppercase; color: var(--color-text-secondary);", "Status" }
                                    th { style: "padding: 8px 12px; font-size: 12px; text-transform: uppercase; color: var(--color-text-secondary);", "Created" }
                                }
                            }
                            tbody {
                                for user in resp.data.iter() {
                                    tr { style: "border-bottom: 1px solid var(--color-border);",
                                        key: "{user.id}",
                                        td { style: "padding: 10px 12px; font-weight: 500;", "{user.username}" }
                                        td { style: "padding: 10px 12px; color: var(--color-text-secondary);", "{user.email}" }
                                        td { style: "padding: 10px 12px;",
                                            span {
                                                style: "padding: 2px 8px; border-radius: 12px; font-size: 12px; background: {status_bg(&user.status)}; color: {status_fg(&user.status)};",
                                                "{user.status}"
                                            }
                                        }
                                        td { style: "padding: 10px 12px; color: var(--color-text-secondary); font-size: 13px;",
                                            "{user.created_at}"
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

fn status_bg(status: &str) -> &'static str {
    match status {
        "active" => "#dcfce7",
        "inactive" => "#fef3c7",
        "locked" => "#fef2f2",
        _ => "#f1f5f9",
    }
}

fn status_fg(status: &str) -> &'static str {
    match status {
        "active" => "#16a34a",
        "inactive" => "#d97706",
        "locked" => "#dc2626",
        _ => "#64748b",
    }
}
