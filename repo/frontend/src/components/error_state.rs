use dioxus::prelude::*;

#[component]
pub fn ErrorState(message: String, detail: Option<String>) -> Element {
    rsx! {
        div { class: "state-error",
            div { class: "icon", "!" }
            p { class: "state-message", "{message}" }
            if let Some(d) = detail {
                p { class: "state-detail", "{d}" }
            }
        }
    }
}
