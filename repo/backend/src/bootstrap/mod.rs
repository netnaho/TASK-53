use rocket::Rocket;
use rocket::Build;
use rocket_cors::{AllowedOrigins, CorsOptions};

use crate::api;
use crate::application::alert_engine::AlertEngine;
use crate::application::auth_service::AuthService;
use crate::application::billing_service::BillingService;
use crate::application::catalog_service::CatalogService;
use crate::application::chaos_service::ChaosService;
use crate::application::degradation_service::DegradationService;
use crate::application::delivery_service::DeliveryService;
use crate::application::export_service::ExportService;
use crate::application::metrics_service::MetricsService;
use crate::application::org_service::OrgService;
use crate::application::package_service::PackageService;
use crate::application::payment_service::PaymentService;
use crate::application::plan_service::PlanService;
use crate::application::reconciliation_service::ReconciliationService;
use crate::application::report_service::ReportService;
use crate::application::role_service::RoleService;
use crate::application::scoring_service::ScoringService;
use crate::application::seed_service;
use crate::application::user_service::UserService;
use crate::config::AppConfig;
use crate::infrastructure::audit::AuditService;
use crate::infrastructure::database;
use crate::infrastructure::encryption::EncryptionService;
use crate::infrastructure::permission_cache::PermissionCache;

pub async fn build_rocket() -> Rocket<Build> {
    let config = AppConfig::from_env();

    let pool = database::create_pool(&config.database_url).await;
    database::run_migrations(&pool).await;
    seed_service::run_seeds(&pool).await;

    let encryption = EncryptionService::new(&config.encryption_key);
    let audit = AuditService::new(pool.clone());
    let perm_cache = PermissionCache::new(pool.clone(), 30);

    let auth_service = AuthService::new(
        pool.clone(), config.jwt_secret.clone(), config.session_ttl_hours,
        audit.clone(), perm_cache.clone(),
    );
    let user_service = UserService::new(pool.clone(), audit.clone(), perm_cache.clone());
    let role_service = RoleService::new(pool.clone(), audit.clone(), perm_cache.clone());
    let org_service = OrgService::new(pool.clone(), audit.clone());
    let catalog_service = CatalogService::new(pool.clone(), audit.clone());
    let package_service = PackageService::new(pool.clone(), audit.clone());
    let plan_service = PlanService::new(pool.clone(), audit.clone(), encryption.clone());
    let delivery_service = DeliveryService::new(pool.clone(), audit.clone(), encryption.clone());
    let billing_service = BillingService::new(pool.clone(), audit.clone());
    let payment_service = PaymentService::new(pool.clone(), audit.clone());
    let reconciliation_service = ReconciliationService::new(pool.clone(), audit.clone());
    let scoring_service = ScoringService::new(pool.clone(), audit.clone());

    // Observability and resilience services
    let metrics_service = MetricsService::new();
    let degradation_service = DegradationService::new(pool.clone(), audit.clone());
    let alert_engine = AlertEngine::new(metrics_service.clone(), pool.clone());
    let chaos_service = ChaosService::new(pool.clone());

    // Seed degradation toggle defaults using the system admin user
    // We do a best-effort seed; if it fails we log and continue
    if let Err(e) = degradation_service.seed_defaults("system").await {
        tracing::warn!(error = %e, "Could not seed degradation toggle defaults (non-fatal)");
    }

    let report_service = ReportService::new(pool.clone(), degradation_service.clone());
    let export_service = ExportService::new(pool.clone(), audit.clone(), degradation_service.clone());

    // ------------------------------------------------------------------
    // Background tasks
    // ------------------------------------------------------------------

    // Alert evaluation: every 30 seconds, check if error rate > 2%
    {
        let engine = alert_engine.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
            loop {
                interval.tick().await;
                engine.evaluate().await;
            }
        });
    }

    // Chaos drill monitor: every 60 seconds, log drill start/stop transitions
    // Only active when CHAOS_ENABLED=true
    if ChaosService::is_chaos_armed() {
        let chaos = chaos_service.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
            let mut was_active = false;
            loop {
                interval.tick().await;
                let now_active = ChaosService::drill_active();
                if now_active && !was_active {
                    chaos.log_drill_started().await;
                } else if !now_active && was_active {
                    chaos.log_drill_stopped().await;
                }
                was_active = now_active;
            }
        });
    }

    let exact_origins: Vec<&str> = config.cors_allowed_origins.iter().map(|s| s.as_str()).collect();
    let allowed_origins = AllowedOrigins::some_exact(&exact_origins);
    let cors = CorsOptions {
        allowed_origins,
        allow_credentials: true,
        ..Default::default()
    }
    .to_cors()
    .expect("CORS configuration failed — check CORS_ALLOWED_ORIGINS");

    rocket::build()
        .manage(pool)
        .manage(config)
        .manage(encryption)
        .manage(audit.clone())
        .manage(perm_cache)
        .manage(auth_service)
        .manage(user_service)
        .manage(role_service)
        .manage(org_service)
        .manage(catalog_service)
        .manage(package_service)
        .manage(plan_service)
        .manage(delivery_service)
        .manage(billing_service)
        .manage(payment_service)
        .manage(reconciliation_service)
        .manage(scoring_service)
        .manage(report_service)
        .manage(export_service)
        .manage(metrics_service)
        .manage(alert_engine)
        .manage(degradation_service)
        .manage(chaos_service)
        .attach(cors)
        .attach(api::tracing_fairing::TracingFairing)
        .mount("/api/health", api::observability::routes())
        .mount("/api/auth", api::auth::routes())
        .mount("/api/admin/org", api::admin_org::routes())
        .mount("/api/users", api::users_roles_permissions::user_routes())
        .mount("/api/roles", api::users_roles_permissions::role_routes())
        .mount("/api/catalog", api::service_catalog::routes())
        .mount("/api/packages", api::packages::routes())
        .mount("/api/plans", api::client_plans::routes())
        .mount("/api/delivery", api::delivery_entries::routes())
        .mount("/api/billing", api::billing::routes())
        .mount("/api/payments", api::payments_refunds::routes())
        .mount("/api/scoring", api::scoring_reviews::routes())
        .mount("/api/reports", api::reports_exports::routes())
        .mount("/api/ops", api::ops::routes())
        .mount("/api/audit", api::audit_api::routes())
}
