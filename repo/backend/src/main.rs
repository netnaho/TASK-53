mod api;
mod application;
mod bootstrap;
mod config;
mod domain;
mod infrastructure;

use bootstrap::build_rocket;

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    infrastructure::logging::init();
    tracing::info!("Starting CareOps backend");

    let rocket = build_rocket().await;
    rocket.launch().await?;
    Ok(())
}
