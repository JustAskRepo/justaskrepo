// infrastructure/http/error.rs
//
// The ONE place AppError becomes an HTTP response. Modules stay ignorant of
// status codes; change the mapping here and every endpoint changes with it.

use axum::{
    Json,
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::Serialize;

use crate::shared_kernel::error::AppError;

/// The only error shape the API ever returns. `code` is stable and safe to
/// branch on; `message` is for humans and may change.
#[derive(Serialize)]
struct ErrorBody {
    code: &'static str,
    message: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = match self {
            AppError::NotFound { .. } => StatusCode::NOT_FOUND,
            AppError::Unauthorized => StatusCode::UNAUTHORIZED,
            AppError::Forbidden => StatusCode::FORBIDDEN,
            AppError::Validation(_) => StatusCode::UNPROCESSABLE_ENTITY,
            AppError::Conflict(_) => StatusCode::CONFLICT,
            AppError::RateLimited { .. } => StatusCode::TOO_MANY_REQUESTS,
            AppError::Unavailable { .. } => StatusCode::SERVICE_UNAVAILABLE,
            AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        // Internal errors carry stack context and dependency detail. Log the
        // whole thing, tell the client nothing — an error message is an
        // information disclosure channel.
        let message = match &self {
            AppError::Internal(source) => {
                tracing::error!(error = ?source, "unhandled internal error");
                "internal server error".to_owned()
            }
            other => other.to_string(),
        };

        let retry_after = match &self {
            AppError::RateLimited { retry_after_secs } => Some(*retry_after_secs),
            _ => None,
        };

        let mut response = (
            status,
            Json(ErrorBody {
                code: self.code(),
                message,
            }),
        )
            .into_response();

        if let Some(seconds) = retry_after {
            response
                .headers_mut()
                .insert(header::RETRY_AFTER, HeaderValue::from(seconds));
        }

        response
    }
}
