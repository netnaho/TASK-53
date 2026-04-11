use sqlx::mysql::MySqlPoolOptions;
use sqlx::MySqlPool;

pub async fn create_pool(database_url: &str) -> MySqlPool {
    tracing::info!("Connecting to database");
    MySqlPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await
        .expect("Failed to connect to database")
}

pub async fn run_migrations(pool: &MySqlPool) {
    tracing::info!("Running database migrations");

    // Repair dirty migrations: if a previous run crashed mid-migration,
    // sqlx records success=0 and refuses to retry.  Delete the dirty
    // record so the (now-idempotent) migration file can be re-applied.
    let _ = sqlx::query(
        "DELETE FROM _sqlx_migrations WHERE success = 0"
    )
    .execute(pool)
    .await; // Ignore error — table may not exist on first run.

    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .expect("Failed to run migrations");
    tracing::info!("Migrations completed");
}
