use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use sqlx::error::ErrorKind;

pub enum AppError {
    Validation(String),
    NotFound,
    Conflict(String),
    Unavailable,
    Database(sqlx::Error),
}

// Lets `?` convert sqlx errors automatically
impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        if matches!(e, sqlx::Error::PoolTimedOut | sqlx::Error::PoolClosed) {
            tracing::warn!(error = %e, "database temporarily unavailable");
            return AppError::Unavailable;
        }

        // This comes from inside the Database and is a variant of sqlx::Error.
        let kind = e.as_database_error().map(|db| db.kind());

        match kind {
            Some(ErrorKind::UniqueViolation) => {
                tracing::warn!(error = %e, "unique constraint violated");
                AppError::Conflict("resource already exists".into())
            }
            Some(ErrorKind::ForeignKeyViolation) => {
                tracing::warn!(error = %e, "foreign key constraint violated");
                AppError::Conflict("referenced resource is missing or still in use".into())
            }
            Some(ErrorKind::CheckViolation | ErrorKind::NotNullViolation) => {
                tracing::warn!(error = %e, "check constraint violated");
                AppError::Validation("invalid value".into())
            }
            // Everything else (connection errors, RowNotFound, ...) is a 500
            _ => AppError::Database(e),
        }
    }
}

// Lets Axum turn an AppError into an HTTP response
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        #[derive(Serialize)]
        struct ErrorResponse {
            message: String,
        }

        let (status, msg) = match self {
            AppError::Validation(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::NotFound => (StatusCode::NOT_FOUND, "Not Found".to_string()),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, msg),
            AppError::Unavailable => (
                StatusCode::SERVICE_UNAVAILABLE,
                "Service unavailable".to_string(),
            ),
            AppError::Database(err) => {
                tracing::error!(error = %err, "database error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Database error".to_string(),
                )
            }
        };

        (status, Json(ErrorResponse { message: msg })).into_response()
    }
}
