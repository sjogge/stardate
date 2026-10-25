use std::collections::{HashMap, HashSet};

use axum::{Router, extract::State, http::StatusCode, routing::get};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqliteExecutor, SqlitePool};
use time::{Date, OffsetDateTime};

use crate::{
    auth::CurrentUser,
    concurrency::{ExpectedVersion, Tagged, Versioned},
    db,
    error::AppError,
    extract::{AppJson, AppPath},
    jobs,
};

const MAX_MINUTES_PER_DAY: i64 = 24 * 60;

#[derive(FromRow)]
struct ScheduleRow {
    id: i64,
    valid_from: Date,
    lunch_paid: bool,
    breaks_paid: bool,
    version: i64,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

#[derive(FromRow)]
struct ScheduleDayRow {
    schedule_id: i64,
    weekday: i64,
    work_minutes: i64,
    lunch_minutes: i64,
    break_minutes: i64,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleDay {
    /// ISO 8601 weekday: 1 = Monday, 7 = Sunday
    weekday: i64,
    work_minutes: i64,
    lunch_minutes: i64,
    break_minutes: i64,
}

impl From<ScheduleDayRow> for ScheduleDay {
    fn from(row: ScheduleDayRow) -> Self {
        Self {
            weekday: row.weekday,
            work_minutes: row.work_minutes,
            lunch_minutes: row.lunch_minutes,
            break_minutes: row.break_minutes,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleResponse {
    id: i64,
    valid_from: Date,
    lunch_paid: bool,
    breaks_paid: bool,
    /// Sorted by weekday; a missing weekday means no expected work that day
    days: Vec<ScheduleDay>,
    version: i64,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    updated_at: OffsetDateTime,
}

impl ScheduleResponse {
    fn new(row: ScheduleRow, days: Vec<ScheduleDay>) -> Self {
        Self {
            id: row.id,
            valid_from: row.valid_from,
            lunch_paid: row.lunch_paid,
            breaks_paid: row.breaks_paid,
            days,
            version: row.version,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

impl Versioned for ScheduleResponse {
    fn version(&self) -> i64 {
        self.version
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleRequest {
    valid_from: Date,
    // No defaults: paid or unpaid must be a deliberate choice per employment
    lunch_paid: bool,
    breaks_paid: bool,
    days: Vec<ScheduleDay>,
}

impl ScheduleRequest {
    fn validate(&self) -> Result<(), AppError> {
        let mut seen = HashSet::new();
        for day in &self.days {
            if !(1..=7).contains(&day.weekday) {
                return Err(AppError::Validation(
                    "weekday must be between 1 (Monday) and 7 (Sunday)".into(),
                ));
            }
            // Caught here because the primary key violation would surface as a misleading 409
            if !seen.insert(day.weekday) {
                return Err(AppError::Validation(format!(
                    "weekday {} is listed more than once",
                    day.weekday
                )));
            }
            let minutes = [day.work_minutes, day.lunch_minutes, day.break_minutes];
            if minutes
                .iter()
                .any(|m| !(0..=MAX_MINUTES_PER_DAY).contains(m))
            {
                return Err(AppError::Validation(format!(
                    "minutes must be between 0 and {MAX_MINUTES_PER_DAY}"
                )));
            }
        }
        Ok(())
    }
}

pub fn routes() -> Router<SqlitePool> {
    Router::new()
        .route("/", get(list_schedules).post(create_schedule))
        .route(
            "/{schedule_id}",
            get(get_schedule)
                .put(update_schedule)
                .delete(delete_schedule),
        )
}

async fn list_schedules(
    State(pool): State<SqlitePool>,
    user: CurrentUser,
    AppPath(job_id): AppPath<i64>,
) -> Result<AppJson<Vec<ScheduleResponse>>, AppError> {
    // TODO: Could this also be made atomic? As in ensuring and reading in the same query?
    jobs::ensure_owned(&pool, job_id, user.id).await?;

    let rows = sqlx::query_as::<_, ScheduleRow>(
        "SELECT id, valid_from, lunch_paid, breaks_paid, version, created_at, updated_at \
         FROM schedules WHERE job_id = ? ORDER BY valid_from",
    )
    .bind(job_id)
    .fetch_all(&pool)
    .await?;

    // TODO: Explain why to queries are needed? Im just wondering what is going on.
    // One query for the weekday rows of all schedules instead of one per schedule
    let day_rows = sqlx::query_as::<_, ScheduleDayRow>(
        "SELECT d.schedule_id, d.weekday, d.work_minutes, d.lunch_minutes, d.break_minutes \
         FROM schedule_days d JOIN schedules s ON s.id = d.schedule_id \
         WHERE s.job_id = ? ORDER BY d.schedule_id, d.weekday",
    )
    .bind(job_id)
    .fetch_all(&pool)
    .await?;

    let mut days_by_schedule: HashMap<i64, Vec<ScheduleDay>> = HashMap::new();
    for row in day_rows {
        days_by_schedule
            .entry(row.schedule_id)
            .or_default()
            .push(row.into());
    }

    let schedules = rows
        .into_iter()
        .map(|row| {
            let days = days_by_schedule.remove(&row.id).unwrap_or_default();
            ScheduleResponse::new(row, days)
        })
        .collect();

    Ok(AppJson(schedules))
}

async fn get_schedule(
    State(pool): State<SqlitePool>,
    user: CurrentUser,
    AppPath((job_id, schedule_id)): AppPath<(i64, i64)>,
) -> Result<Tagged<ScheduleResponse>, AppError> {
    let row = sqlx::query_as::<_, ScheduleRow>(
        "SELECT s.id, s.valid_from, s.lunch_paid, s.breaks_paid, s.version, s.created_at, s.updated_at \
         FROM schedules s JOIN jobs j ON j.id = s.job_id \
         WHERE s.id = ? AND s.job_id = ? AND j.user_id = ?",
    )
    .bind(schedule_id)
    .bind(job_id)
    .bind(user.id)
    .fetch_optional(&pool)
    .await?
    .ok_or(AppError::NotFound)?;

    let days = load_days(&pool, row.id).await?;
    Ok(Tagged(ScheduleResponse::new(row, days)))
}

async fn create_schedule(
    State(pool): State<SqlitePool>,
    user: CurrentUser,
    AppPath(job_id): AppPath<i64>,
    AppJson(request): AppJson<ScheduleRequest>,
) -> Result<(StatusCode, Tagged<ScheduleResponse>), AppError> {
    request.validate()?;

    let mut tx = db::begin_write(&pool).await?;

    // Ownership is part of the INSERT itself: one statement, so it takes the write lock
    // at once and the check cannot go stale before the write. No row means no such job
    // for this user. A second schedule on the same date violates UNIQUE (job_id, valid_from): a 409
    let row = sqlx::query_as::<_, ScheduleRow>(
        "INSERT INTO schedules (job_id, valid_from, lunch_paid, breaks_paid) \
         SELECT id, ?, ?, ? FROM jobs WHERE id = ? AND user_id = ? \
         RETURNING id, valid_from, lunch_paid, breaks_paid, version, created_at, updated_at",
    )
    .bind(request.valid_from)
    .bind(request.lunch_paid)
    .bind(request.breaks_paid)
    .bind(job_id)
    .bind(user.id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AppError::NotFound)?;

    insert_days(&mut tx, row.id, &request.days).await?;
    let days = load_days(&mut *tx, row.id).await?;
    tx.commit().await?;

    Ok((
        StatusCode::CREATED,
        Tagged(ScheduleResponse::new(row, days)),
    ))
}

async fn update_schedule(
    State(pool): State<SqlitePool>,
    user: CurrentUser,
    AppPath((job_id, schedule_id)): AppPath<(i64, i64)>,
    ExpectedVersion(version): ExpectedVersion,
    AppJson(request): AppJson<ScheduleRequest>,
) -> Result<Tagged<ScheduleResponse>, AppError> {
    request.validate()?;

    let mut tx = db::begin_write(&pool).await?;

    // The versioned UPDATE comes first so SQLite takes the write lock immediately
    let row = sqlx::query_as::<_, ScheduleRow>(
        "UPDATE schedules SET valid_from = ?, lunch_paid = ?, breaks_paid = ?, \
         version = version + 1, updated_at = unixepoch() \
         WHERE id = ? AND job_id = ? AND version = ? \
         AND job_id IN (SELECT id FROM jobs WHERE user_id = ?) \
         RETURNING id, valid_from, lunch_paid, breaks_paid, version, created_at, updated_at",
    )
    .bind(request.valid_from)
    .bind(request.lunch_paid)
    .bind(request.breaks_paid)
    .bind(schedule_id)
    .bind(job_id)
    .bind(version)
    .bind(user.id)
    .fetch_optional(&mut *tx)
    .await?;

    let Some(row) = row else {
        tx.rollback().await?;
        return Err(not_found_or_modified(&pool, job_id, schedule_id, user.id).await);
    };

    // The weekday rows are replaced as a whole, like the request describes them
    sqlx::query("DELETE FROM schedule_days WHERE schedule_id = ?")
        .bind(row.id)
        .execute(&mut *tx)
        .await?;
    insert_days(&mut tx, row.id, &request.days).await?;
    let days = load_days(&mut *tx, row.id).await?;
    tx.commit().await?;

    Ok(Tagged(ScheduleResponse::new(row, days)))
}

/// Existing work days keep their snapshot; their schedule_id becomes NULL.
async fn delete_schedule(
    State(pool): State<SqlitePool>,
    user: CurrentUser,
    AppPath((job_id, schedule_id)): AppPath<(i64, i64)>,
    ExpectedVersion(version): ExpectedVersion,
) -> Result<StatusCode, AppError> {
    // schedule_days rows go with it through ON DELETE CASCADE
    let result = sqlx::query(
        "DELETE FROM schedules WHERE id = ? AND job_id = ? AND version = ? \
         AND job_id IN (SELECT id FROM jobs WHERE user_id = ?)",
    )
    .bind(schedule_id)
    .bind(job_id)
    .bind(version)
    .bind(user.id)
    .execute(&pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(not_found_or_modified(&pool, job_id, schedule_id, user.id).await);
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn insert_days(
    tx: &mut sqlx::SqliteConnection,
    schedule_id: i64,
    days: &[ScheduleDay],
) -> Result<(), AppError> {
    for day in days {
        sqlx::query(
            "INSERT INTO schedule_days (schedule_id, weekday, work_minutes, lunch_minutes, break_minutes) \
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(schedule_id)
        .bind(day.weekday)
        .bind(day.work_minutes)
        .bind(day.lunch_minutes)
        .bind(day.break_minutes)
        .execute(&mut *tx)
        .await?;
    }
    Ok(())
}

async fn load_days(
    executor: impl SqliteExecutor<'_>,
    schedule_id: i64,
) -> Result<Vec<ScheduleDay>, AppError> {
    let rows = sqlx::query_as::<_, ScheduleDayRow>(
        "SELECT schedule_id, weekday, work_minutes, lunch_minutes, break_minutes \
         FROM schedule_days WHERE schedule_id = ? ORDER BY weekday",
    )
    .bind(schedule_id)
    .fetch_all(executor)
    .await?;

    Ok(rows.into_iter().map(ScheduleDay::from).collect())
}

/// A versioned write that matched no row: 404 if the schedule doesn't exist for
/// this user and job, 412 if it exists at another version.
async fn not_found_or_modified(
    pool: &SqlitePool,
    job_id: i64,
    schedule_id: i64,
    user_id: i64,
) -> AppError {
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (SELECT 1 FROM schedules s JOIN jobs j ON j.id = s.job_id \
         WHERE s.id = ? AND s.job_id = ? AND j.user_id = ?)",
    )
    .bind(schedule_id)
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
