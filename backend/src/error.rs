use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

pub enum AppError {
    Validation(String),
    NotFound,
    Database(sqlx::Error)
}
// Lets `?` convert sqlx errors automatically
impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        match e {
            sqlx::Error::RowNotFound => AppError::NotFound,
            other => AppError::Database(other),
        }
    }
}

// Lets Axum turn an AppError into an HTTP response
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        // How we want errors responses to be serialized
        #[derive(Serialize)]
        struct ErrorResponse {
            message: String,
        }
        
        let (status, msg) = match self {
            AppError::Validation(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::NotFound => (StatusCode::NOT_FOUND, "Not Found".to_string()),
            AppError::Database(err) => {
                eprintln!("Database error: {}", err);
                (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string())
            }
        };
        
        (status, Json(ErrorResponse { message: msg })).into_response()
    }
}