use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{Route, State};

use crate::api::guards::AuthenticatedUser;
use crate::application::auth_service::AuthService;
use crate::domain::auth_types::{LoginRequest, LoginResponse, UserProfile};
use crate::domain::error::AppError;

#[rocket::post("/login", data = "<body>")]
pub async fn login(
    body: Json<LoginRequest>,
    auth_service: &State<AuthService>,
) -> Result<Json<LoginResponse>, AppError> {
    let response = auth_service.login(&body.into_inner(), None, None).await?;
    Ok(Json(response))
}

#[rocket::post("/logout")]
pub async fn logout(
    user: AuthenticatedUser,
    auth_service: &State<AuthService>,
) -> Result<Status, AppError> {
    auth_service.logout(&user.session_id, &user.user_id).await?;
    Ok(Status::NoContent)
}

#[rocket::get("/me")]
pub async fn current_user(
    user: AuthenticatedUser,
    auth_service: &State<AuthService>,
) -> Result<Json<UserProfile>, AppError> {
    let profile = auth_service.get_current_user(&user.user_id).await?;
    Ok(Json(profile))
}

pub fn routes() -> Vec<Route> {
    rocket::routes![login, logout, current_user]
}
