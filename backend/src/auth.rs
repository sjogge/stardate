use axum::{extract::FromRequestParts, http::request::Parts};

use crate::error::AppError;

// Seeded by migration 0002. Replaced by the authenticated user once auth exists.
const DEFAULT_USER_ID: i64 = 1;

/// The user making the request. Handlers scope all queries by this id, so adding
/// authentication later only changes how it is resolved here.
pub struct CurrentUser {
    pub id: i64,
}

impl<S: Send + Sync> FromRequestParts<S> for CurrentUser {
    type Rejection = AppError;

    async fn from_request_parts(_parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(CurrentUser {
            id: DEFAULT_USER_ID,
        })
    }
}
