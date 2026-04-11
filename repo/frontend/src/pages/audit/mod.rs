use dioxus::prelude::*;
use crate::components::loading::Loading;
use crate::components::error_state::ErrorState;
use crate::components::permission_denied::PermissionDenied;
use crate::models::AuditLogRow;
use crate::services::ApiClient;
use crate::state::{AuthState, perms};

#[component]
pub fn Audit() -> Element {
    let auth: Signal<AuthState> = use_context();

    if !auth.read().has_permission(perms::MENU_AUDIT) {
        return rsx! { PermissionDenied { action: "view audit logs".to_string() } };
    }

    let token = auth.read().token.clone();
    let logs = use_resource(move || {
        let t = token.clone();
        async move {
            ApiClient::get::<Vec<AuditLogRow>>("/audit/?limit=100", t.as_deref()).await
        }
    });

    rsx! {
        div {
            div { class: "page-header",
                h1 { "Audit Log" }
                p { "Comprehensive record of system actions for compliance tracking and operational transparency." }
            }

            div { class: "card",
                match &*logs.read() {
                    None => rsx! { Loading { message: Some("Loading audit logs...".to_string()) } },
                    Some(Err(e)) => rsx! { ErrorState { message: "Failed to load audit logs".to_string(), detail: Some(e.clone()) } },
                    Some(Ok(entries)) => rsx! {
                        table { style: "width: 100%; border-collapse: collapse; font-size: 13px;",
                            thead {
                                tr { style: "border-bottom: 2px solid var(--color-border); text-align: left;",
                                    th { style: "padding: 8px 10px; font-size: 11px; text-transform: uppercase; color: var(--color-text-secondary);", "Time" }
                                    th { style: "padding: 8px 10px; font-size: 11px; text-transform: uppercase; color: var(--color-text-secondary);", "Action" }
                                    th { style: "padding: 8px 10px; font-size: 11px; text-transform: uppercase; color: var(--color-text-secondary);", "Resource" }
                                    th { style: "padding: 8px 10px; font-size: 11px; text-transform: uppercase; color: var(--color-text-secondary);", "User" }
                                    th { style: "padding: 8px 10px; font-size: 11px; text-transform: uppercase; color: var(--color-text-secondary);", "IP" }
                                }
                            }
                            tbody {
                                for entry in entries.iter() {
                                    tr { style: "border-bottom: 1px solid var(--color-border);",
                                        key: "{entry.id}",
                                        td { style: "padding: 8px 10px; white-space: nowrap;", "{entry.timestamp}" }
                                        td { style: "padding: 8px 10px;",
                                            span { style: "padding: 2px 6px; border-radius: 4px; font-size: 12px; background: #f1f5f9;",
                                                "{entry.action}"
                                            }
                                        }
                                        td { style: "padding: 8px 10px; color: var(--color-text-secondary);",
                                            "{entry.resource_type}"
                                            if let Some(ref rid) = entry.resource_id {
                                                span { style: "margin-left: 4px; font-size: 11px;", "({rid})" }
                                            }
                                        }
                                        td { style: "padding: 8px 10px; color: var(--color-text-secondary);",
                                            "{entry.user_id.as_deref().unwrap_or(\"-\")}"
                                        }
                                        td { style: "padding: 8px 10px; color: var(--color-text-secondary);",
                                            "{entry.ip_address.as_deref().unwrap_or(\"-\")}"
                                        }
                                    }
                                }
                            }
                        }
                        if entries.is_empty() {
                            p { style: "text-align: center; padding: 24px; color: var(--color-text-secondary);",
                                "No audit entries found."
                            }
                        }
                    }
                }
            }
        }
    }
}
