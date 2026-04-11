use dioxus::prelude::*;
use crate::router::Route;
use crate::state::{AuthState, perms};

/// Permission-aware sidebar navigation.
/// Links are only rendered if the user has the corresponding menu permission.
#[component]
pub fn Sidebar() -> Element {
    let auth: Signal<AuthState> = use_context();
    let auth_state = auth.read();

    rsx! {
        nav { class: "sidebar",
            div { class: "sidebar-brand", "CareOps" }
            div { class: "sidebar-nav",
                if auth_state.has_permission(perms::MENU_DASHBOARD) {
                    SidebarLink { to: Route::Dashboard {}, label: "Dashboard" }
                }
                if auth_state.has_permission(perms::MENU_CATALOG) {
                    SidebarLink { to: Route::Catalog {}, label: "Service Catalog" }
                }
                if auth_state.has_permission(perms::MENU_PLANS) {
                    SidebarLink { to: Route::Plans {}, label: "Client Plans" }
                }
                if auth_state.has_permission(perms::MENU_DELIVERY) {
                    SidebarLink { to: Route::Delivery {}, label: "Service Delivery" }
                }
                if auth_state.has_permission(perms::MENU_BILLING) {
                    SidebarLink { to: Route::Billing {}, label: "Billing" }
                }
                if auth_state.has_permission(perms::MENU_SCORING) {
                    SidebarLink { to: Route::Scoring {}, label: "Quality Scoring" }
                }
                if auth_state.has_permission(perms::MENU_REPORTS) {
                    SidebarLink { to: Route::Reports {}, label: "Reports" }
                }
                if auth_state.has_permission(perms::MENU_AUDIT) {
                    SidebarLink { to: Route::Audit {}, label: "Audit Log" }
                }
                if auth_state.has_permission(perms::MENU_ADMIN) {
                    SidebarLink { to: Route::Admin {}, label: "Administration" }
                }
                if auth_state.has_permission(perms::MENU_USERS) {
                    SidebarLink { to: Route::Users {}, label: "User Management" }
                }
                if auth_state.has_permission(perms::API_OPS_READ) {
                    SidebarLink { to: Route::Ops {}, label: "Ops Controls" }
                }
            }
        }
    }
}

#[component]
fn SidebarLink(to: Route, label: &'static str) -> Element {
    rsx! {
        Link { class: "sidebar-link", to: to,
            span { "{label}" }
        }
    }
}
