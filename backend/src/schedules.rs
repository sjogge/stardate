use std::collections::{HashMap, HashSet};

use axum::{Router, extract::State, http::StatusCode, routing::get};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqliteConnection, SqliteExecutor, SqlitePool};
use time::{Date, Duration, OffsetDateTime};

use crate::{
    auth::CurrentUser,
    calc,
    concurrency::{ExpectedVersion, Tagged, Versioned},
    db,
    error::AppError,
    extract::{AppJson, AppPath, AppQuery},
    jobs,
};

const MAX_MINUTES_PER_DAY: i64 = 24 * 60;
const MAX_NAME_CHARS: usize = 100;

#[derive(FromRow)]
struct ScheduleRow {
    id: i64,
    name: String,
    lunch_paid: bool,
    breaks_paid: bool,
    next_schedule_id: Option<i64>,
    version: i64,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

#[derive(FromRow)]
struct ScheduleDayRow {
    schedule_id: i64,
    weekday: i64,
    start_minute: i64,
    end_minute: i64,
    lunch_minutes: i64,
    break_minutes: i64,
}

/// One weekday as the client sends it. Times are local clock times, `HH:MM`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleDayRequest {
    /// ISO 8601 weekday: 1 = Monday, 7 = Sunday
    weekday: i64,
    start: String,
    /// Before `start` means the next day (a night shift)
    end: String,
    lunch_minutes: i64,
    break_minutes: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleDayResponse {
    weekday: i64,
    start: String,
    end: String,
    lunch_minutes: i64,
    break_minutes: i64,
    /// Derived: the shift minus its unpaid pauses; what a day on this schedule expects
    work_minutes: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleResponse {
    id: i64,
    name: String,
    lunch_paid: bool,
    breaks_paid: bool,
    /// The schedule of the week after this one, if they rotate
    next_schedule_id: Option<i64>,
    /// Sorted by weekday; a missing weekday means no expected work that day
    days: Vec<ScheduleDayResponse>,
    version: i64,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    updated_at: OffsetDateTime,
}

impl ScheduleResponse {
    fn new(row: ScheduleRow, day_rows: Vec<ScheduleDayRow>) -> Self {
        let days = day_rows
            .into_iter()
            .map(|day| ScheduleDayResponse {
                weekday: day.weekday,
                start: format_clock(day.start_minute),
                end: format_clock(day.end_minute),
                lunch_minutes: day.lunch_minutes,
                break_minutes: day.break_minutes,
                work_minutes: calc::planned_work_minutes(
                    day.start_minute,
                    day.end_minute,
                    day.lunch_minutes,
                    day.break_minutes,
                    row.lunch_paid,
                    row.breaks_paid,
                ),
            })
            .collect();
        Self {
            id: row.id,
            name: row.name,
            lunch_paid: row.lunch_paid,
            breaks_paid: row.breaks_paid,
            next_schedule_id: row.next_schedule_id,
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
    name: String,
    // No defaults: paid or unpaid must be a deliberate choice per employment
    lunch_paid: bool,
    breaks_paid: bool,
    #[serde(default)]
    next_schedule_id: Option<i64>,
    days: Vec<ScheduleDayRequest>,
}

/// A weekday that passed validation, with clock times as minutes after midnight.
struct ValidDay {
    weekday: i64,
    start_minute: i64,
    end_minute: i64,
    lunch_minutes: i64,
    break_minutes: i64,
}

impl ScheduleRequest {
    /// Returns the trimmed name and the weekdays ready to store.
    fn validate(&self) -> Result<(&str, Vec<ValidDay>), AppError> {
        let name = self.name.trim();
        if name.is_empty() || name.chars().count() > MAX_NAME_CHARS {
            return Err(AppError::Validation(format!(
                "name must be 1 to {MAX_NAME_CHARS} characters"
            )));
        }

        let mut seen = HashSet::new();
        let mut days = Vec::with_capacity(self.days.len());
        for day in &self.days {
            let weekday = day.weekday;
            if !(1..=7).contains(&weekday) {
                return Err(AppError::Validation(
                    "weekday must be between 1 (Monday) and 7 (Sunday)".into(),
                ));
            }
            // Caught here because the primary key violation would surface as a misleading 409
            if !seen.insert(weekday) {
                return Err(AppError::Validation(format!(
                    "weekday {weekday} is listed more than once"
                )));
            }
            let (Some(start_minute), Some(end_minute)) =
                (parse_clock(&day.start), parse_clock(&day.end))
            else {
                return Err(AppError::Validation(format!(
                    "weekday {weekday}: start and end must be times like 08:00"
                )));
            };
            if start_minute == end_minute {
                return Err(AppError::Validation(format!(
                    "weekday {weekday}: start and end must differ"
                )));
            }
            if [day.lunch_minutes, day.break_minutes]
                .iter()
                .any(|m| !(0..=MAX_MINUTES_PER_DAY).contains(m))
            {
                return Err(AppError::Validation(format!(
                    "lunch and break minutes must be between 0 and {MAX_MINUTES_PER_DAY}"
                )));
            }
            // A 17:00 lunch typed into an 8-hour shift would make every day without a logged
            // lunch deeply negative
            if day.lunch_minutes + day.break_minutes > calc::shift_minutes(start_minute, end_minute)
            {
                return Err(AppError::Validation(format!(
                    "weekday {weekday}: lunch and breaks must fit in the shift"
                )));
            }
            days.push(ValidDay {
                weekday,
                start_minute,
                end_minute,
                lunch_minutes: day.lunch_minutes,
                break_minutes: day.break_minutes,
            });
        }
        Ok((name, days))
    }
}

#[derive(Deserialize)]
pub struct SuggestedQuery {
    date: Date,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestedResponse {
    /// None only when the job has no schedules
    schedule_id: Option<i64>,
}

pub fn routes() -> Router<SqlitePool> {
    Router::new()
        .route("/", get(list_schedules).post(create_schedule))
        .route("/suggested", get(suggested_schedule))
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
    // A read-only transaction: all three queries see the same snapshot, so a schedule
    // edited meanwhile cannot pair old schedule rows with new weekday rows
    let mut tx = pool.begin().await?;
    jobs::ensure_owned(&mut *tx, job_id, user.id).await?;

    let rows = sqlx::query_as::<_, ScheduleRow>(
        "SELECT id, name, lunch_paid, breaks_paid, next_schedule_id, version, created_at, \
         updated_at FROM schedules WHERE job_id = ? ORDER BY name",
    )
    .bind(job_id)
    .fetch_all(&mut *tx)
    .await?;

    // One query for the weekday rows of all schedules instead of one per schedule
    let day_rows = sqlx::query_as::<_, ScheduleDayRow>(
        "SELECT d.schedule_id, d.weekday, d.start_minute, d.end_minute, d.lunch_minutes, \
         d.break_minutes \
         FROM schedule_days d JOIN schedules s ON s.id = d.schedule_id \
         WHERE s.job_id = ? ORDER BY d.schedule_id, d.weekday",
    )
    .bind(job_id)
    .fetch_all(&mut *tx)
    .await?;

    let mut days_by_schedule: HashMap<i64, Vec<ScheduleDayRow>> = HashMap::new();
    for row in day_rows {
        days_by_schedule
            .entry(row.schedule_id)
            .or_default()
            .push(row);
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
    // Read-only transaction: the schedule row and its weekday rows come from one snapshot
    let mut tx = pool.begin().await?;

    let row = sqlx::query_as::<_, ScheduleRow>(
        "SELECT s.id, s.name, s.lunch_paid, s.breaks_paid, s.next_schedule_id, s.version, \
         s.created_at, s.updated_at \
         FROM schedules s JOIN jobs j ON j.id = s.job_id \
         WHERE s.id = ? AND s.job_id = ? AND j.user_id = ?",
    )
    .bind(schedule_id)
    .bind(job_id)
    .bind(user.id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AppError::NotFound)?;

    let days = load_days(&mut *tx, row.id).await?;
    Ok(Tagged(ScheduleResponse::new(row, days)))
}

async fn create_schedule(
    State(pool): State<SqlitePool>,
    user: CurrentUser,
    AppPath(job_id): AppPath<i64>,
    AppJson(request): AppJson<ScheduleRequest>,
) -> Result<(StatusCode, Tagged<ScheduleResponse>), AppError> {
    let (name, days) = request.validate()?;

    let mut tx = db::begin_write(&pool).await?;

    // Ownership is part of the INSERT itself: one statement, so it takes the write lock
    // at once and the check cannot go stale before the write. No row means no such job
    // for this user. A second schedule with the same name violates UNIQUE (job_id, name): a 409
    let row = sqlx::query_as::<_, ScheduleRow>(
        "INSERT INTO schedules (job_id, name, lunch_paid, breaks_paid, next_schedule_id) \
         SELECT id, ?, ?, ?, ? FROM jobs WHERE id = ? AND user_id = ? \
         RETURNING id, name, lunch_paid, breaks_paid, next_schedule_id, version, created_at, \
         updated_at",
    )
    .bind(name)
    .bind(request.lunch_paid)
    .bind(request.breaks_paid)
    .bind(request.next_schedule_id)
    .bind(job_id)
    .bind(user.id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AppError::NotFound)?;

    // Dropping the transaction on this error rolls the insert back
    ensure_next_in_job(&mut tx, job_id, request.next_schedule_id).await?;
    insert_days(&mut tx, row.id, &days).await?;
    let day_rows = load_days(&mut *tx, row.id).await?;
    tx.commit().await?;

    Ok((
        StatusCode::CREATED,
        Tagged(ScheduleResponse::new(row, day_rows)),
    ))
}

async fn update_schedule(
    State(pool): State<SqlitePool>,
    user: CurrentUser,
    AppPath((job_id, schedule_id)): AppPath<(i64, i64)>,
    ExpectedVersion(version): ExpectedVersion,
    AppJson(request): AppJson<ScheduleRequest>,
) -> Result<Tagged<ScheduleResponse>, AppError> {
    let (name, days) = request.validate()?;

    let mut tx = db::begin_write(&pool).await?;

    // The versioned UPDATE comes first so SQLite takes the write lock immediately
    let row = sqlx::query_as::<_, ScheduleRow>(
        "UPDATE schedules SET name = ?, lunch_paid = ?, breaks_paid = ?, next_schedule_id = ?, \
         version = version + 1, updated_at = unixepoch() \
         WHERE id = ? AND job_id = ? AND version = ? \
         AND job_id IN (SELECT id FROM jobs WHERE user_id = ?) \
         RETURNING id, name, lunch_paid, breaks_paid, next_schedule_id, version, created_at, \
         updated_at",
    )
    .bind(name)
    .bind(request.lunch_paid)
    .bind(request.breaks_paid)
    .bind(request.next_schedule_id)
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

    ensure_next_in_job(&mut tx, job_id, request.next_schedule_id).await?;
    // The weekday rows are replaced as a whole, like the request describes them
    sqlx::query("DELETE FROM schedule_days WHERE schedule_id = ?")
        .bind(row.id)
        .execute(&mut *tx)
        .await?;
    insert_days(&mut tx, row.id, &days).await?;
    let day_rows = load_days(&mut *tx, row.id).await?;
    tx.commit().await?;

    Ok(Tagged(ScheduleResponse::new(row, day_rows)))
}

/// Existing work days keep their snapshot; their schedule_id becomes NULL, and so does the
/// link of a schedule that rotated into this one.
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

async fn suggested_schedule(
    State(pool): State<SqlitePool>,
    user: CurrentUser,
    AppPath(job_id): AppPath<i64>,
    AppQuery(query): AppQuery<SuggestedQuery>,
) -> Result<AppJson<SuggestedResponse>, AppError> {
    // Read-only transaction: schedules and days come from one snapshot
    let mut tx = pool.begin().await?;
    jobs::ensure_owned(&mut *tx, job_id, user.id).await?;
    let schedule_id = suggested(&mut tx, job_id, query.date).await?;
    Ok(AppJson(SuggestedResponse { schedule_id }))
}

/// The schedule to preselect for a new day on `date`: the schedule of the job's latest day
/// before it, moved along the rotation links once per calendar week in between. Computed from
/// the stored days alone, so every device suggests the same, and weeks without registered days
/// (a vacation) still move the rotation on. Without earlier days: the job's first schedule.
pub(crate) async fn suggested(
    conn: &mut SqliteConnection,
    job_id: i64,
    date: Date,
) -> Result<Option<i64>, AppError> {
    let links = sqlx::query_as::<_, (i64, Option<i64>)>(
        "SELECT id, next_schedule_id FROM schedules WHERE job_id = ? ORDER BY id",
    )
    .bind(job_id)
    .fetch_all(&mut *conn)
    .await?;
    let Some(&(first_id, _)) = links.first() else {
        return Ok(None);
    };

    let latest = sqlx::query_as::<_, (Date, i64)>(
        "SELECT work_date, schedule_id FROM work_days \
         WHERE job_id = ? AND work_date < ? AND schedule_id IS NOT NULL \
         ORDER BY work_date DESC LIMIT 1",
    )
    .bind(job_id)
    .bind(date)
    .fetch_optional(&mut *conn)
    .await?;
    let Some((latest_date, latest_schedule)) = latest else {
        return Ok(Some(first_id));
    };

    let next: HashMap<i64, Option<i64>> = links.into_iter().collect();
    let weeks = (week_start(date) - week_start(latest_date)).whole_weeks();
    let mut current = latest_schedule;
    for _ in 0..weeks {
        match next.get(&current).copied().flatten() {
            Some(following) => current = following,
            // An unlinked schedule simply repeats
            None => break,
        }
    }
    Ok(Some(current))
}

/// The Monday of the date's week, so weeks are counted Monday to Monday.
fn week_start(date: Date) -> Date {
    date - Duration::days(i64::from(date.weekday().number_days_from_monday()))
}

/// A rotation link may only point to a schedule of the same job.
async fn ensure_next_in_job(
    tx: &mut SqliteConnection,
    job_id: i64,
    next_schedule_id: Option<i64>,
) -> Result<(), AppError> {
    let Some(next_id) = next_schedule_id else {
        return Ok(());
    };
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (SELECT 1 FROM schedules WHERE id = ? AND job_id = ?)",
    )
    .bind(next_id)
    .bind(job_id)
    .fetch_one(&mut *tx)
    .await?;
    if exists {
        Ok(())
    } else {
        Err(AppError::Validation(
            "nextScheduleId must be a schedule of the same job".into(),
        ))
    }
}

async fn insert_days(
    tx: &mut SqliteConnection,
    schedule_id: i64,
    days: &[ValidDay],
) -> Result<(), AppError> {
    for day in days {
        sqlx::query(
            "INSERT INTO schedule_days \
             (schedule_id, weekday, start_minute, end_minute, lunch_minutes, break_minutes) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(schedule_id)
        .bind(day.weekday)
        .bind(day.start_minute)
        .bind(day.end_minute)
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
) -> Result<Vec<ScheduleDayRow>, AppError> {
    Ok(sqlx::query_as::<_, ScheduleDayRow>(
        "SELECT schedule_id, weekday, start_minute, end_minute, lunch_minutes, break_minutes \
         FROM schedule_days WHERE schedule_id = ? ORDER BY weekday",
    )
    .bind(schedule_id)
    .fetch_all(executor)
    .await?)
}

/// `HH:MM` (24-hour, two digits each) as minutes after midnight.
fn parse_clock(text: &str) -> Option<i64> {
    let (hours, minutes) = text.split_once(':')?;
    if hours.len() != 2 || minutes.len() != 2 {
        return None;
    }
    let hours: i64 = hours.parse().ok()?;
    let minutes: i64 = minutes.parse().ok()?;
    ((0..24).contains(&hours) && (0..60).contains(&minutes)).then_some(hours * 60 + minutes)
}

fn format_clock(minute_of_day: i64) -> String {
    format!("{:02}:{:02}", minute_of_day / 60, minute_of_day % 60)
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
