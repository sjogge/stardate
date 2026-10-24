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
    // The If-Match version no longer matches: someone else changed the resource
    PreconditionFailed,
    // A write that requires If-Match was sent without it
    PreconditionRequired,
    Database(sqlx::Error),
}

// Lets `?` convert sqlx errors automatically
impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        match e {
            // fetch_one() found no row: the requested resource doesn't exist
            sqlx::Error::RowNotFound => AppError::NotFound,

            // Pool exhausted or shutting down: a temporary condition, the client may retry
            sqlx::Error::PoolTimedOut | sqlx::Error::PoolClosed => {
                tracing::warn!(error = %e, "database temporarily unavailable");
                AppError::Unavailable
            }
            // Errors reported by SQLite itself. `ref` borrows instead of moving,
            // so `e` can still be moved into AppError::Database if unclassified.
            sqlx::Error::Database(ref db_error) => match db_error.kind() {
                ErrorKind::UniqueViolation => {
                    tracing::warn!(error = %db_error, "unique constraint violated");
                    AppError::Conflict("resource already exists".into())
                }
                ErrorKind::ForeignKeyViolation => {
                    tracing::warn!(error = %db_error, "foreign key constraint violated");
                    AppError::Conflict("referenced resource is missing or still in use".into())
                }
                ErrorKind::CheckViolation | ErrorKind::NotNullViolation => {
                    tracing::warn!(error = %db_error, "check constraint violated");
                    AppError::Validation("invalid value".into())
                }
                // Any other database error (including "database is locked") is a 500
                _ => AppError::Database(e),
            },

            // Anything else (I/O, decoding, ...) is unexpected: a 500
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
            AppError::PreconditionFailed => (
                StatusCode::PRECONDITION_FAILED,
                "Resource was modified by another request; reload and retry".to_string(),
            ),
            AppError::PreconditionRequired => (
                StatusCode::PRECONDITION_REQUIRED,
                "If-Match header is required".to_string(),
            ),
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
