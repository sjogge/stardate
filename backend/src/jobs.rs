use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use time::OffsetDateTime;

use crate::{
    auth::CurrentUser,
    concurrency::{ExpectedVersion, Tagged, Versioned},
    error::AppError,
};

const MAX_NAME_CHARS: usize = 100;

#[derive(FromRow)]
struct JobRow {
    id: i64,
    name: String,
    archived_at: Option<OffsetDateTime>,
    version: i64,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobResponse {
    id: i64,
    name: String,
    #[serde(with = "time::serde::rfc3339::option")]
    archived_at: Option<OffsetDateTime>,
    version: i64,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    updated_at: OffsetDateTime,
}

impl From<JobRow> for JobResponse {
    fn from(row: JobRow) -> Self {
        Self {
            id: row.id,
            name: row.name,
            archived_at: row.archived_at,
            version: row.version,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

impl Versioned for JobResponse {
    fn version(&self) -> i64 {
        self.version
    }
}

#[derive(Deserialize)]
pub struct JobRequest {
    name: String,
}

impl JobRequest {
    /// Returns the trimmed name, the form that is validated and stored.
    fn validated_name(&self) -> Result<&str, AppError> {
        let name = self.name.trim();
        if name.is_empty() {
            return Err(AppError::Validation("name must not be empty".into()));
        }
        if name.chars().count() > MAX_NAME_CHARS {
            return Err(AppError::Validation(format!(
                "name must be at most {MAX_NAME_CHARS} characters"
            )));
        }
        Ok(name)
    }
}

pub fn routes() -> Router<SqlitePool> {
    Router::new()
        .route("/", get(list_jobs).post(create_job))
        .route("/{id}", get(get_job).put(rename_job))
        .route("/{id}/archive", post(archive_job))
}

async fn list_jobs(
    State(pool): State<SqlitePool>,
    user: CurrentUser,
) -> Result<Json<Vec<JobResponse>>, AppError> {
    // Active jobs first, then archived ones
    let rows = sqlx::query_as::<_, JobRow>(
        "SELECT id, name, archived_at, version, created_at, updated_at FROM jobs WHERE user_id = ? \
         ORDER BY archived_at IS NOT NULL, name",
    )
    .bind(user.id)
    .fetch_all(&pool)
    .await?;

    Ok(Json(rows.into_iter().map(JobResponse::from).collect()))
}

async fn get_job(
    State(pool): State<SqlitePool>,
    user: CurrentUser,
    Path(id): Path<i64>,
) -> Result<Tagged<JobResponse>, AppError> {
    sqlx::query_as::<_, JobRow>(
        "SELECT id, name, archived_at, version, created_at, updated_at FROM jobs \
         WHERE id = ? AND user_id = ?",
    )
    .bind(id)
    .bind(user.id)
    .fetch_optional(&pool)
    .await?
    .map(|row| Tagged(row.into()))
    .ok_or(AppError::NotFound)
}

async fn create_job(
    State(pool): State<SqlitePool>,
    user: CurrentUser,
    Json(request): Json<JobRequest>,
) -> Result<(StatusCode, Tagged<JobResponse>), AppError> {
    let name = request.validated_name()?;

    // A duplicate name violates UNIQUE (user_id, name) and becomes a 409 in AppError
    let row = sqlx::query_as::<_, JobRow>(
        "INSERT INTO jobs (user_id, name) VALUES (?, ?) \
         RETURNING id, name, archived_at, version, created_at, updated_at",
    )
    .bind(user.id)
    .bind(name)
    .fetch_one(&pool)
    .await?;

    Ok((StatusCode::CREATED, Tagged(row.into())))
}

async fn rename_job(
    State(pool): State<SqlitePool>,
    user: CurrentUser,
    Path(id): Path<i64>,
    ExpectedVersion(version): ExpectedVersion,
    Json(request): Json<JobRequest>,
) -> Result<Tagged<JobResponse>, AppError> {
    let name = request.validated_name()?;

    // Single statement: the version check and the update cannot be interleaved
    // with another request's write
    let row = sqlx::query_as::<_, JobRow>(
        "UPDATE jobs SET name = ?, version = version + 1, updated_at = unixepoch() \
         WHERE id = ? AND user_id = ? AND version = ? \
         RETURNING id, name, archived_at, version, created_at, updated_at",
    )
    .bind(name)
    .bind(id)
    .bind(user.id)
    .bind(version)
    .fetch_optional(&pool)
    .await?;

    match row {
        Some(row) => Ok(Tagged(row.into())),
        None => Err(not_found_or_modified(&pool, id, user.id).await),
    }
}

/// Archiving needs no If-Match: it only moves the job forward to a final state.
/// Archiving an already archived job returns it unchanged.
async fn archive_job(
    State(pool): State<SqlitePool>,
    user: CurrentUser,
    Path(id): Path<i64>,
) -> Result<Tagged<JobResponse>, AppError> {
    let archived = sqlx::query_as::<_, JobRow>(
        "UPDATE jobs SET archived_at = unixepoch(), version = version + 1, updated_at = unixepoch() \
         WHERE id = ? AND user_id = ? AND archived_at IS NULL \
         RETURNING id, name, archived_at, version, created_at, updated_at",
    )
    .bind(id)
    .bind(user.id)
    .fetch_optional(&pool)
    .await?;

    match archived {
        Some(row) => Ok(Tagged(row.into())),
        None => get_job(State(pool), user, Path(id)).await,
    }
}

/// A versioned UPDATE that matched no row: tells "doesn't exist" (404) apart
/// from "exists, but at another version" (412).
async fn not_found_or_modified(pool: &SqlitePool, id: i64, user_id: i64) -> AppError {
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (SELECT 1 FROM jobs WHERE id = ? AND user_id = ?)",
    )
    .bind(id)
    .bind(user_id)
    .fetch_one(pool)
    .await;

    match exists {
        Ok(true) => AppError::PreconditionFailed,
        Ok(false) => AppError::NotFound,
        Err(e) => e.into(),
    }
}
