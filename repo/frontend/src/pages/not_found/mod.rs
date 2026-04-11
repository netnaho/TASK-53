use dioxus::prelude::*;

#[component]
pub fn NotFound(segments: Vec<String>) -> Element {
    let path = segments.join("/");
    rsx! {
        div { class: "state-error", style: "min-height: 60vh;",
            div { class: "icon", "404" }
            p { class: "state-message", "Page Not Found" }
            p { class: "state-detail", "The path /{path} does not exist." }
            Link { to: crate::router::Route::Dashboard {},
                button { class: "btn btn-primary", style: "margin-top: 16px;", "Back to Dashboard" }
            }
        }
    }
}
