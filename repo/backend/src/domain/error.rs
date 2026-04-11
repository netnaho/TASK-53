use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::response::Responder;
use rocket::Request;
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct ErrorEnvelope {
    pub error: ErrorBody,
}

#[derive(Debug, Serialize)]
pub struct ErrorBody {
    pub code: String,
    pub message: String,
    pub trace_id: String,
}

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Not implemented: {0}")]
    NotImplemented(String),

    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl AppError {
    fn status(&self) -> Status {
        match self {
            AppError::NotFound(_) => Status::NotFound,
            AppError::BadRequest(_) => Status::BadRequest,
            AppError::Unauthorized(_) => Status::Unauthorized,
            AppError::Forbidden(_) => Status::Forbidden,
            AppError::Conflict(_) => Status::Conflict,
            AppError::NotImplemented(_) => Status::NotImplemented,
            AppError::ServiceUnavailable(_) => Status::ServiceUnavailable,
            AppError::Internal(_) => Status::InternalServerError,
        }
    }

    fn code(&self) -> &str {
        match self {
            AppError::NotFound(_) => "NOT_FOUND",
            AppError::BadRequest(_) => "BAD_REQUEST",
            AppError::Unauthorized(_) => "UNAUTHORIZED",
            AppError::Forbidden(_) => "FORBIDDEN",
            AppError::Conflict(_) => "CONFLICT",
            AppError::NotImplemented(_) => "NOT_IMPLEMENTED",
            AppError::ServiceUnavailable(_) => "SERVICE_UNAVAILABLE",
            AppError::Internal(_) => "INTERNAL_ERROR",
        }
    }

    pub fn envelope(&self) -> ErrorEnvelope {
        ErrorEnvelope {
            error: ErrorBody {
                code: self.code().to_string(),
                message: self.to_string(),
                trace_id: Uuid::new_v4().to_string(),
            },
        }
    }
}

impl<'r> Responder<'r, 'static> for AppError {
    fn respond_to(self, req: &'r Request<'_>) -> rocket::response::Result<'static> {
        let status = self.status();
        let body = Json(self.envelope());
        rocket::response::Response::build_from(body.respond_to(req)?)
            .status(status)
            .ok()
    }
}
