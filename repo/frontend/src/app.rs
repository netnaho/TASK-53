use dioxus::prelude::*;
use crate::router::Route;
use crate::state::AuthState;

#[component]
pub fn App() -> Element {
    // Provide auth state at the app root
    use_context_provider(|| Signal::new(AuthState::default()));

    rsx! {
        Router::<Route> {}
    }
}

#[cfg(test)]
mod app_test;
