use axum::{
    extract::{FromRequest, FromRequestParts},
    response::{IntoResponse, Response},
};
use serde::Serialize;

use crate::error::AppError;

/// `axum::Json`, but a malformed body becomes an `AppError` so every error the
/// API returns has the same `{"message": ...}` shape.
#[derive(FromRequest)]
#[from_request(via(axum::Json), rejection(AppError))]
pub struct AppJson<T>(pub T);

// Lets handlers use the same Json type for responses
impl<T: Serialize> IntoResponse for AppJson<T> {
    fn into_response(self) -> Response {
        axum::Json(self.0).into_response()
    }
}

/// `axum::extract::Path`, with an invalid path parameter (e.g. `/jobs/abc`) as an `AppError`.
#[derive(FromRequestParts)]
#[from_request(via(axum::extract::Path), rejection(AppError))]
pub struct AppPath<T>(pub T);
