#![expect(dead_code, reason = "used by the jobs module from step 2")]

use axum::{
    Json,
    extract::FromRequestParts,
    http::{HeaderValue, header, request::Parts},
    response::{IntoResponse, Response},
};
use serde::Serialize;

use crate::error::AppError;

/// The version a client expects to modify, taken from `If-Match: "<version>"`.
/// Handlers that take this extractor reject requests without the header (428).
pub struct ExpectedVersion(pub i64);

impl<S: Send + Sync> FromRequestParts<S> for ExpectedVersion {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let value = parts
            .headers
            .get(header::IF_MATCH)
            .ok_or(AppError::PreconditionRequired)?;

        parse_etag(value).map(ExpectedVersion).ok_or_else(|| {
            AppError::Validation("If-Match must be a quoted version, e.g. \"3\"".into())
        })
    }
}

/// Parses a strong ETag of the form `"<version>"`. Weak ETags (`W/"3"`) and `*` are
/// rejected: If-Match uses strong comparison, and `*` would skip the version check.
fn parse_etag(value: &HeaderValue) -> Option<i64> {
    value
        .to_str()
        .ok()?
        .strip_prefix('"')?
        .strip_suffix('"')?
        .parse()
        .ok()
}

/// Implemented by response bodies that carry an optimistic-concurrency version.
pub trait Versioned {
    fn version(&self) -> i64;
}

/// A JSON response with an `ETag` header taken from the body's version, so the
/// header and the `version` field can never disagree.
pub struct Tagged<T>(pub T);

impl<T: Versioned + Serialize> IntoResponse for Tagged<T> {
    fn into_response(self) -> Response {
        let etag = format!("\"{}\"", self.0.version());
        ([(header::ETAG, etag)], Json(self.0)).into_response()
    }
}