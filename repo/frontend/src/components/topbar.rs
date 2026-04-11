use dioxus::prelude::*;
use crate::state::AuthState;
use crate::router::Route;

#[component]
pub fn Topbar() -> Element {
    let mut auth: Signal<AuthState> = use_context();
    let username = auth.read().user.as_ref().map(|u| u.username.clone()).unwrap_or_default();
    let roles = auth.read().user.as_ref()
        .map(|u| u.roles.join(", "))
        .unwrap_or_default();

    rsx! {
        header { class: "topbar",
            div { class: "topbar-title", "CareOps Portal" }
            div { class: "topbar-actions", style: "display: flex; align-items: center; gap: 16px;",
                div { style: "text-align: right;",
                    div { style: "font-weight: 500; font-size: 14px;", "{username}" }
                    div { style: "font-size: 11px; color: var(--color-text-secondary);", "{roles}" }
                }
                button {
                    class: "btn",
                    style: "border-color: var(--color-border); font-size: 13px;",
                    onclick: move |_| {
                        auth.set(AuthState::default());
                    },
                    "Sign Out"
                }
            }
        }
    }
}
