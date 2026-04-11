use dioxus::prelude::*;

#[component]
pub fn PermissionDenied(action: Option<String>) -> Element {
    let msg = action.unwrap_or_else(|| "access this page".to_string());
    rsx! {
        div { class: "state-permission-denied",
            div { class: "icon", "X" }
            p { class: "state-message", "Permission Denied" }
            p { class: "state-detail", "You do not have permission to {msg}. Contact your administrator for access." }
        }
    }
}
