use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Template error: {0}")]
    Template(#[from] tera::Error),

    #[error("HTTP client error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    BadRequest(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("{0}")]
    Internal(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Payment required: {0}")]
    PaymentRequired(String),

    #[error("Rate limited — try again in {0} seconds")]
    RateLimited(u64),

    #[error("Service unavailable: {0}")]
    Unavailable(String),

    #[error("{0}")]
    Anyhow(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = match &self {
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            AppError::Forbidden(_) => StatusCode::FORBIDDEN,
            AppError::PaymentRequired(_) => StatusCode::PAYMENT_REQUIRED,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::RateLimited(_) => StatusCode::TOO_MANY_REQUESTS,
            AppError::Unavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let msg = self.to_string();
        if let AppError::Template(e) = &self {
            tracing::error!(template_error_debug = ?e, "template render failure");
        }
        tracing::error!(%status, error = %msg);

        (
            status,
            [(axum::http::header::CONTENT_TYPE, "application/json")],
            serde_json::json!({"error": msg}).to_string(),
        )
            .into_response()
    }
}
