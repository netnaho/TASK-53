use dioxus::prelude::*;

#[component]
pub fn Loading(message: Option<String>) -> Element {
    let msg = message.unwrap_or_else(|| "Loading...".to_string());
    rsx! {
        div { class: "state-loading",
            div { class: "spinner" }
            p { class: "state-message", "{msg}" }
        }
    }
}
