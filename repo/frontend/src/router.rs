use dioxus::prelude::*;
use crate::layouts::app_layout::AppLayout;
use crate::pages::*;

#[derive(Routable, Clone, PartialEq)]
pub enum Route {
    #[route("/login")]
    Login {},

    #[layout(AppLayout)]
        #[route("/")]
        Dashboard {},

        #[route("/admin")]
        Admin {},

        #[route("/users")]
        Users {},

        #[route("/catalog")]
        Catalog {},

        #[route("/plans")]
        Plans {},

        #[route("/delivery")]
        Delivery {},

        #[route("/billing")]
        Billing {},

        #[route("/scoring")]
        Scoring {},

        #[route("/reports")]
        Reports {},

        #[route("/audit")]
        Audit {},

        #[route("/ops")]
        Ops {},
    #[end_layout]

    #[route("/:..segments")]
    NotFound { segments: Vec<String> },
}
