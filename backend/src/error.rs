use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use chrono::Utc;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<String>,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<HashMap<String, String>>,
    timestamp: chrono::DateTime<Utc>,
}

pub enum AppError {
    Validation(HashMap<String, String>),
    NotFound(String),
    BadRequest(String),
    Forbidden(String),
    Internal(anyhow::Error),
}

impl AppError {
    fn status_and_code(&self) -> (StatusCode, &'static str) {
        match self {
            AppError::Validation(_) => (StatusCode::BAD_REQUEST, "VALIDATION_ERROR"),
            AppError::NotFound(_) => (StatusCode::NOT_FOUND, "RESOURCE_NOT_FOUND"),
            AppError::BadRequest(_) => (StatusCode::BAD_REQUEST, "INVALID_ARGUMENT"),
            AppError::Forbidden(_) => (StatusCode::FORBIDDEN, "ACCESS_DENIED"),
            AppError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_SERVER_ERROR"),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code) = self.status_and_code();

        let (message, details) = match self {
            AppError::Validation(details) => ("Validation failed".to_string(), Some(details)),
            AppError::NotFound(msg) => (msg, None),
            AppError::BadRequest(msg) => (msg, None),
            AppError::Forbidden(msg) => (msg, None),
            AppError::Internal(err) => {
                tracing::error!("Unexpected error occurred: {err:?}");
                ("An unexpected error occurred".to_string(), None)
            }
        };

        let body = ErrorResponse {
            code: Some(code.to_string()),
            message,
            details,
            timestamp: Utc::now(),
        };

        (status, Json(body)).into_response()
    }
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        AppError::Internal(err)
    }
}

impl From<mongodb::error::Error> for AppError {
    fn from(err: mongodb::error::Error) -> Self {
        AppError::Internal(err.into())
    }
}

pub type AppResult<T> = Result<T, AppError>;

/// Mirrors `@NotBlank`/`@NotEmpty` validation, producing the same
/// `{field: message}` shape the Java `MethodArgumentNotValidException`
/// handler produced.
pub fn require_non_blank(fields: &[(&str, &str)]) -> AppResult<()> {
    let details: HashMap<String, String> = fields
        .iter()
        .filter(|(_, value)| value.trim().is_empty())
        .map(|(name, _)| (name.to_string(), format!("{name} is required")))
        .collect();

    if details.is_empty() {
        Ok(())
    } else {
        Err(AppError::Validation(details))
    }
}
