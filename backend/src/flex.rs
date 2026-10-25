//! The flex balance and manual flex adjustments (e.g. hours carried over from an
//! old system, or overtime paid out).

use axum::{
    Router,
    extract::State,
    http::StatusCode,
    routing::{get, put},
};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqliteConnection, SqlitePool};
use time::{Date, Duration, OffsetDateTime};

use crate::{
    auth::CurrentUser,
    calc,
    concurrency::{ExpectedVersion, Tagged, Versioned},
    days::{self, invalid},
    error::AppError,
    extract::{AppJson, AppPath},
    jobs,
};

const MAX_NOTE_CHARS: usize = 1000;
/// A year of full days; anything larger is a typo
const MAX_ADJUSTMENT_MINUTES: i64 = 366 * 24 * 60;

/// The balance split into its two sources, as `Duration`s so callers can sum
/// before rounding to minutes.
pub(crate) struct Balance {
    pub days: Duration,
    pub adjustments: Duration,
}

impl Balance {
    pub(crate) fn total(&self) -> Duration {
        self.days + self.adjustments
    }
}

/// The job's flex balance: every registered day's balance plus all adjustments.
/// A day still open is left out until it is clocked out, since its balance is
/// only "so far" and would swing from negative to positive during the day.
pub(crate) async fn balance(
    tx: &mut SqliteConnection,
    job_id: i64,
    now: OffsetDateTime,
) -> Result<Balance, AppError> {
    let days = days::totals_in_range(tx, job_id, None, None, now)
        .await?
        .iter()
        .filter(|day| !day.open)
        .map(|day| day.totals.balance)
        .sum();

    let adjustment_minutes = sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE(SUM(minutes), 0) FROM flex_adjustments WHERE job_id = ?",
    )
    .bind(job_id)
    .fetch_one(&mut *tx)
    .await?;

    Ok(Balance {
        days,
        adjustments: Duration::minutes(adjustment_minutes),
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BalanceResponse {
    balance_minutes: i64,
    days_minutes: i64,
    adjustments_minutes: i64,
}

#[derive(FromRow)]
struct AdjustmentRow {
    id: i64,
    adjusted_on: Date,
    minutes: i64,
    note: Option<String>,
    version: i64,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdjustmentResponse {
    id: i64,
    adjusted_on: Date,
    /// Positive adds to the balance, negative takes from it
    minutes: i64,
    note: Option<String>,
    version: i64,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    updated_at: OffsetDateTime,
}

impl From<AdjustmentRow> for AdjustmentResponse {
    fn from(row: AdjustmentRow) -> Self {
        Self {
            id: row.id,
            adjusted_on: row.adjusted_on,
            minutes: row.minutes,
            note: row.note,
            version: row.version,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

impl Versioned for AdjustmentResponse {
    fn version(&self) -> i64 {
        self.version
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdjustmentRequest {
    adjusted_on: Date,
    minutes: i64,
    note: Option<String>,
}

impl AdjustmentRequest {
    /// Returns the note as stored: trimmed, and None when empty.
    fn validate(&self) -> Result<Option<&str>, AppError> {
        if self.minutes == 0 {
            return Err(invalid("minutes must not be 0"));
        }
        if self.minutes.abs() > MAX_ADJUSTMENT_MINUTES {
            return Err(invalid("minutes must be at most a year of full days"));
        }
        let note = self
            .note
            .as_deref()
            .map(str::trim)
            .filter(|n| !n.is_empty());
        if note.is_some_and(|n| n.chars().count() > MAX_NOTE_CHARS) {
            return Err(invalid("note must be at most 1000 characters"));
        }
        Ok(note)
    }
}

pub fn routes() -> Router<SqlitePool> {
    Router::new()
        .route("/flex", get(get_balance))
        .route(
            "/flex-adjustments",
            get(list_adjustments).post(create_adjustment),
        )
        .route(
            "/flex-adjustments/{adjustment_id}",
            put(update_adjustment).delete(delete_adjustment),
        )
}

async fn get_balance(
    State(pool): State<SqlitePool>,
    user: CurrentUser,
    AppPath(job_id): AppPath<i64>,
) -> Result<AppJson<BalanceResponse>, AppError> {
    // Read-only transaction: days, pauses and adjustments from one snapshot
    let mut tx = pool.begin().await?;
    jobs::ensure_owned(&mut *tx, job_id, user.id).await?;
    let balance = balance(&mut tx, job_id, days::now()).await?;

    Ok(AppJson(BalanceResponse {
        balance_minutes: calc::whole_minutes(balance.total()),
        days_minutes: calc::whole_minutes(balance.days),
        adjustments_minutes: calc::whole_minutes(balance.adjustments),
    }))
}

async fn list_adjustments(
    State(pool): State<SqlitePool>,
    user: CurrentUser,
    AppPath(job_id): AppPath<i64>,
) -> Result<AppJson<Vec<AdjustmentResponse>>, AppError> {
    let mut tx = pool.begin().await?;
    jobs::ensure_owned(&mut *tx, job_id, user.id).await?;

    let rows = sqlx::query_as::<_, AdjustmentRow>(
        "SELECT id, adjusted_on, minutes, note, version, created_at, updated_at \
         FROM flex_adjustments WHERE job_id = ? ORDER BY adjusted_on, id",
    )
    .bind(job_id)
    .fetch_all(&mut *tx)
    .await?;

    Ok(AppJson(
        rows.into_iter().map(AdjustmentResponse::from).collect(),
    ))
}

async fn create_adjustment(
    State(pool): State<SqlitePool>,
    user: CurrentUser,
    AppPath(job_id): AppPath<i64>,
    AppJson(request): AppJson<AdjustmentRequest>,
) -> Result<(StatusCode, Tagged<AdjustmentResponse>), AppError> {
    let note = request.validate()?;

    // A single conditional write: ownership is part of the INSERT
    let row = sqlx::query_as::<_, AdjustmentRow>(
        "INSERT INTO flex_adjustments (job_id, adjusted_on, minutes, note) \
         SELECT id, ?, ?, ? FROM jobs WHERE id = ? AND user_id = ? \
         RETURNING id, adjusted_on, minutes, note, version, created_at, updated_at",
    )
    .bind(request.adjusted_on)
    .bind(request.minutes)
    .bind(note)
    .bind(job_id)
    .bind(user.id)
    .fetch_optional(&pool)
    .await?
    .ok_or(AppError::NotFound)?;

    Ok((StatusCode::CREATED, Tagged(row.into())))
}

async fn update_adjustment(
    State(pool): State<SqlitePool>,
    user: CurrentUser,
    AppPath((job_id, adjustment_id)): AppPath<(i64, i64)>,
    ExpectedVersion(version): ExpectedVersion,
    AppJson(request): AppJson<AdjustmentRequest>,
) -> Result<Tagged<AdjustmentResponse>, AppError> {
    let note = request.validate()?;

    let row = sqlx::query_as::<_, AdjustmentRow>(
        "UPDATE flex_adjustments SET adjusted_on = ?, minutes = ?, note = ?, \
         version = version + 1, updated_at = unixepoch() \
         WHERE id = ? AND job_id = ? AND version = ? \
         AND job_id IN (SELECT id FROM jobs WHERE user_id = ?) \
         RETURNING id, adjusted_on, minutes, note, version, created_at, updated_at",
    )
    .bind(request.adjusted_on)
    .bind(request.minutes)
    .bind(note)
    .bind(adjustment_id)
    .bind(job_id)
    .bind(version)
    .bind(user.id)
    .fetch_optional(&pool)
    .await?;

    match row {
        Some(row) => Ok(Tagged(row.into())),
        None => Err(not_found_or_modified(&pool, job_id, adjustment_id, user.id).await),
    }
}

async fn delete_adjustment(
    State(pool): State<SqlitePool>,
    user: CurrentUser,
    AppPath((job_id, adjustment_id)): AppPath<(i64, i64)>,
    ExpectedVersion(version): ExpectedVersion,
) -> Result<StatusCode, AppError> {
    let result = sqlx::query(
        "DELETE FROM flex_adjustments WHERE id = ? AND job_id = ? AND version = ? \
         AND job_id IN (SELECT id FROM jobs WHERE user_id = ?)",
    )
    .bind(adjustment_id)
    .bind(job_id)
    .bind(version)
    .bind(user.id)
    .execute(&pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(not_found_or_modified(&pool, job_id, adjustment_id, user.id).await);
    }
    Ok(StatusCode::NO_CONTENT)
}

/// A versioned write that matched no row: 404 if the adjustment doesn't exist for
/// this user and job, 412 if it exists at another version.
async fn not_found_or_modified(
    pool: &SqlitePool,
    job_id: i64,
    adjustment_id: i64,
    user_id: i64,
) -> AppError {
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (SELECT 1 FROM flex_adjustments a JOIN jobs j ON j.id = a.job_id \
         WHERE a.id = ? AND a.job_id = ? AND j.user_id = ?)",
    )
    .bind(adjustment_id)
    .bind(job_id)
    .bind(user_id)
    .fetch_one(pool)
    .await;

    match exists {
        Ok(true) => AppError::PreconditionFailed,
        Ok(false) => AppError::NotFound,
        Err(e) => e.into(),
    }
}
