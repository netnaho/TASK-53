use dioxus::prelude::*;

#[component]
pub fn ValidationMessage(message: String, is_success: Option<bool>) -> Element {
    let class = if is_success.unwrap_or(false) {
        "validation-message success"
    } else {
        "validation-message"
    };
    rsx! {
        span { class: "{class}", "{message}" }
    }
}
