use sqlx::MySqlPool;
use uuid::Uuid;

use crate::application::auth_service::AuthService;
use crate::domain::auth_policy;

/// Seeds default roles, permissions, demo organization, and demo users.
/// Idempotent: checks _seed_history before applying each seed.
pub async fn run_seeds(pool: &MySqlPool) {
    seed_permissions(pool).await;
    seed_roles(pool).await;
    seed_demo_org_and_users(pool).await;
    seed_ops_config(pool).await;
}

async fn is_seed_applied(pool: &MySqlPool, seed_name: &str) -> bool {
    let result: Option<(String,)> = sqlx::query_as(
        "SELECT seed_name FROM _seed_history WHERE seed_name = ?"
    )
    .bind(seed_name)
    .fetch_optional(pool)
    .await
    .unwrap_or(None);
    result.is_some()
}

async fn mark_seed_applied(pool: &MySqlPool, seed_name: &str) {
    sqlx::query("INSERT INTO _seed_history (seed_name) VALUES (?)")
        .bind(seed_name)
        .execute(pool)
        .await
        .ok();
}

async fn seed_permissions(pool: &MySqlPool) {
    if is_seed_applied(pool, "permissions_v1").await {
        tracing::info!("Seed 'permissions_v1' already applied, skipping");
        return;
    }

    tracing::info!("Seeding permissions...");
    for (code, name, category, description, resource) in auth_policy::all_permissions() {
        let id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT IGNORE INTO permissions (id, code, name, category, description, resource)
             VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(&id)
        .bind(code)
        .bind(name)
        .bind(category)
        .bind(description)
        .bind(resource)
        .execute(pool)
        .await
        .ok();
    }

    mark_seed_applied(pool, "permissions_v1").await;
    tracing::info!("Permissions seeded: {} entries", auth_policy::all_permissions().len());
}

async fn seed_roles(pool: &MySqlPool) {
    if is_seed_applied(pool, "roles_v1").await {
        tracing::info!("Seed 'roles_v1' already applied, skipping");
        return;
    }

    tracing::info!("Seeding roles and role-permission mappings...");
    for (role_name, description, permission_codes) in auth_policy::default_role_permissions() {
        let role_id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO roles (id, name, description, is_system) VALUES (?, ?, ?, 1)"
        )
        .bind(&role_id)
        .bind(role_name)
        .bind(description)
        .execute(pool)
        .await
        .ok();

        for code in permission_codes {
            let perm_id: Option<(String,)> = sqlx::query_as(
                "SELECT id FROM permissions WHERE code = ?"
            )
            .bind(code)
            .fetch_optional(pool)
            .await
            .unwrap_or(None);

            if let Some((pid,)) = perm_id {
                sqlx::query(
                    "INSERT IGNORE INTO role_permissions (role_id, permission_id) VALUES (?, ?)"
                )
                .bind(&role_id)
                .bind(&pid)
                .execute(pool)
                .await
                .ok();
            }
        }
    }

    mark_seed_applied(pool, "roles_v1").await;
    tracing::info!("Roles seeded");
}

async fn seed_demo_org_and_users(pool: &MySqlPool) {
    if is_seed_applied(pool, "demo_users_v1").await {
        tracing::info!("Seed 'demo_users_v1' already applied, skipping");
        return;
    }

    tracing::info!("Seeding demo organization and users...");

    // Create demo organization
    let org_id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO organizations (id, name, status) VALUES (?, 'CareOps Demo Org', 'active')")
        .bind(&org_id)
        .execute(pool)
        .await
        .ok();

    // Create demo departments
    let dept_clinical_id = Uuid::new_v4().to_string();
    let dept_billing_id = Uuid::new_v4().to_string();
    let dept_qa_id = Uuid::new_v4().to_string();

    for (dept_id, name) in [
        (&dept_clinical_id, "Clinical Services"),
        (&dept_billing_id, "Billing"),
        (&dept_qa_id, "Quality Assurance"),
    ] {
        sqlx::query("INSERT INTO departments (id, org_id, name, status) VALUES (?, ?, ?, 'active')")
            .bind(dept_id)
            .bind(&org_id)
            .bind(name)
            .execute(pool)
            .await
            .ok();
    }

    // Demo users: (username, email, password, role_name, department_id)
    let demo_users: Vec<(&str, &str, &str, &str, Option<&str>)> = vec![
        ("admin", "admin@careops.local", "Admin123!", "System Administrator", None),
        ("ops_manager", "ops@careops.local", "OpsManager123!", "Operations Manager", None),
        ("billing_staff", "billing@careops.local", "Billing123!", "Billing Specialist", Some(&dept_billing_id)),
        ("coach", "coach@careops.local", "Coach123!", "Coach/Clinician", Some(&dept_clinical_id)),
        ("qa_reviewer", "qa@careops.local", "QAReview123!", "QA Reviewer", Some(&dept_qa_id)),
        ("auditor", "auditor@careops.local", "Auditor123!", "Auditor", None),
    ];

    for (username, email, password, role_name, dept_id) in demo_users {
        let user_id = Uuid::new_v4().to_string();
        let password_hash = AuthService::hash_password(password)
            .expect("Failed to hash demo password");

        // Insert user
        sqlx::query(
            "INSERT INTO users (id, org_id, department_id, username, email, status)
             VALUES (?, ?, ?, ?, ?, 'active')"
        )
        .bind(&user_id)
        .bind(&org_id)
        .bind(dept_id)
        .bind(username)
        .bind(email)
        .execute(pool)
        .await
        .ok();

        // Insert credentials
        sqlx::query("INSERT INTO user_credentials (user_id, password_hash) VALUES (?, ?)")
            .bind(&user_id)
            .bind(&password_hash)
            .execute(pool)
            .await
            .ok();

        // Assign role
        let role_row: Option<(String,)> = sqlx::query_as(
            "SELECT id FROM roles WHERE name = ?"
        )
        .bind(role_name)
        .fetch_optional(pool)
        .await
        .unwrap_or(None);

        if let Some((role_id,)) = role_row {
            sqlx::query("INSERT INTO user_roles (user_id, role_id) VALUES (?, ?)")
                .bind(&user_id)
                .bind(&role_id)
                .execute(pool)
                .await
                .ok();
        }

        // Grant org-level data scope (admin access for admin, write for most, read for auditor)
        let access_level = match role_name {
            "System Administrator" => "admin",
            "Auditor" => "read",
            _ => "write",
        };

        let scope_id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO user_data_scopes (id, user_id, org_id, department_id, project_id, access_level)
             VALUES (?, ?, ?, NULL, NULL, ?)"
        )
        .bind(&scope_id)
        .bind(&user_id)
        .bind(&org_id)
        .bind(access_level)
        .execute(pool)
        .await
        .ok();

        tracing::info!(username = username, role = role_name, "Demo user seeded");
    }

    mark_seed_applied(pool, "demo_users_v1").await;
    tracing::info!("Demo organization and users seeded");
}

async fn seed_ops_config(pool: &MySqlPool) {
    if is_seed_applied(pool, "ops_config_v1").await {
        tracing::info!("Seed 'ops_config_v1' already applied, skipping");
        return;
    }

    // Find the admin user ID to satisfy the FK on ops_config
    let admin_id: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM users WHERE username = 'admin' LIMIT 1"
    )
    .fetch_optional(pool)
    .await
    .unwrap_or(None);

    if let Some((admin_id,)) = admin_id {
        let defaults = [
            ("exports_enabled", "true"),
            ("analytics_enabled", "true"),
        ];

        for (key, value) in &defaults {
            sqlx::query(
                "INSERT INTO ops_config (key_name, value, updated_by) VALUES (?, ?, ?)
                 ON DUPLICATE KEY UPDATE value = VALUES(value)"
            )
            .bind(key)
            .bind(value)
            .bind(&admin_id)
            .execute(pool)
            .await
            .ok();
        }

        mark_seed_applied(pool, "ops_config_v1").await;
        tracing::info!("Ops config defaults seeded: exports_enabled=true, analytics_enabled=true");
    } else {
        tracing::warn!("Could not find admin user for ops_config seed — skipping");
    }
}
