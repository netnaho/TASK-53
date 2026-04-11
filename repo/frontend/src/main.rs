mod app;
mod router;
mod layouts;
mod components;
mod services;
mod state;
mod models;
mod pages;

use dioxus::prelude::*;

fn main() {
    dioxus::launch(app::App);
}
