use dioxus::prelude::*;
use crate::components::sidebar::Sidebar;
use crate::components::topbar::Topbar;
use crate::state::AuthState;
use crate::router::Route;

#[component]
pub fn AppLayout() -> Element {
    let auth: Signal<AuthState> = use_context();

    // Redirect to login if not authenticated
    let nav = use_navigator();
    if !auth.read().is_authenticated() {
        nav.replace(Route::Login {});
        return rsx! {
            div {
                p { "Redirecting to login..." }
            }
        };
    }

    rsx! {
        div { class: "app-shell",
            Sidebar {}
            div { class: "main-area",
                Topbar {}
                div { class: "page-content",
                    Outlet::<Route> {}
                }
            }
        }
    }
}
