use dioxus::prelude::*;

#[component]
pub fn EmptyState(title: String, detail: Option<String>) -> Element {
    rsx! {
        div { class: "state-empty",
            div { class: "icon", "[ ]" }
            p { class: "state-message", "{title}" }
            if let Some(d) = detail {
                p { class: "state-detail", "{d}" }
            }
        }
    }
}
