use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    pub sub: String,       // user_id
    pub session_id: String,
    pub org_id: String,
    pub exp: usize,        // expiration timestamp
    pub iat: usize,        // issued at timestamp
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct UserRow {
    pub id: String,
    pub org_id: String,
    pub department_id: Option<String>,
    pub username: String,
    pub email: String,
    pub status: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize)]
pub struct UserProfile {
    pub id: String,
    pub org_id: String,
    pub department_id: Option<String>,
    pub username: String,
    pub email: String,
    pub status: String,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct RoleRow {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub is_system: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct PermissionRow {
    pub id: String,
    pub code: String,
    pub name: String,
    pub category: String,
    pub description: Option<String>,
    pub resource: Option<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct OrgRow {
    pub id: String,
    pub name: String,
    pub status: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct DepartmentRow {
    pub id: String,
    pub org_id: String,
    pub name: String,
    pub status: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ProjectRow {
    pub id: String,
    pub org_id: String,
    pub department_id: Option<String>,
    pub name: String,
    pub status: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: UserProfile,
}

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub email: String,
    pub password: String,
    pub org_id: String,
    pub department_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUserRequest {
    pub email: Option<String>,
    pub status: Option<String>,
    pub department_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateRoleRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AssignRoleRequest {
    pub role_id: String,
}

#[derive(Debug, Deserialize)]
pub struct AssignPermissionRequest {
    pub permission_id: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateOrgRequest {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateOrgRequest {
    pub name: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateDepartmentRequest {
    pub name: String,
    pub org_id: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateProjectRequest {
    pub name: String,
    pub org_id: String,
    pub department_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AssignScopeRequest {
    pub org_id: String,
    pub department_id: Option<String>,
    pub project_id: Option<String>,
    pub access_level: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct PaginationParams {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub search: Option<String>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
}

impl PaginationParams {
    pub fn limit(&self) -> i64 {
        self.per_page.unwrap_or(25).min(100).max(1)
    }

    pub fn offset(&self) -> i64 {
        let page = self.page.unwrap_or(1).max(1);
        (page - 1) * self.limit()
    }
}

#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T: Serialize> {
    pub data: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}
