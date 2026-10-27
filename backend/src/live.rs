//! Live actions on today's work day: clock in and out, start and end pauses.
//! They need no If-Match: each is guarded by the day's state (409 when it does not
//! fit), and each bumps the day's version so open edit forms see the change.

use axum::{Router, extract::State, http::StatusCode, routing::post};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqliteConnection, SqlitePool};
use time::{Date, Duration, OffsetDateTime};

use crate::{
    auth::CurrentUser,
    concurrency::Tagged,
    days::{self, DayResponse, PauseKind, invalid},
    db,
    error::AppError,
    extract::{AppJson, AppPath},
};

/// How far `at` may lie in the future, to absorb clock differences between devices
const MAX_CLOCK_SKEW: Duration = Duration::minutes(1);

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClockInRequest {
    /// The local calendar date, as the client sees it
    work_date: Date,
    #[serde(default, with = "time::serde::rfc3339::option")]
    at: Option<OffsetDateTime>,
}

#[derive(Deserialize)]
pub struct AtRequest {
    #[serde(default, with = "time::serde::rfc3339::option")]
    at: Option<OffsetDateTime>,
}

#[derive(Deserialize)]
pub struct PauseStartRequest {
    kind: PauseKind,
    #[serde(default, with = "time::serde::rfc3339::option")]
    at: Option<OffsetDateTime>,
}

/// A live action that today's state allows. Mirrors the guards of the handlers below,
/// so clients only offer actions that will not answer 409.
#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LiveAction {
    ClockIn,
    ClockOut,
    StartPause,
    EndPause,
}

/// `today` is the open day, or else the day registered on the client's date.
/// `schedule_covers_date`: clock-in needs a schedule valid on that date.
/// Expects an active job: archived jobs allow no action.
pub fn available_actions(
    today: Option<&DayResponse>,
    schedule_covers_date: bool,
) -> Vec<LiveAction> {
    match today {
        None if schedule_covers_date => vec![LiveAction::ClockIn],
        None => Vec::new(),
        Some(day) if !day.is_open() => Vec::new(),
        // Clocking out also ends the running pause
        Some(day) if day.running_pause().is_some() => {
            vec![LiveAction::EndPause, LiveAction::ClockOut]
        }
        Some(_) => vec![LiveAction::StartPause, LiveAction::ClockOut],
    }
}

pub fn routes() -> Router<SqlitePool> {
    Router::new()
        .route("/clock-in", post(clock_in))
        .route("/clock-out", post(clock_out))
        .route("/pauses/start", post(start_pause))
        .route("/pauses/end", post(end_pause))
}

async fn clock_in(
    State(pool): State<SqlitePool>,
    user: CurrentUser,
    AppPath(job_id): AppPath<i64>,
    AppJson(request): AppJson<ClockInRequest>,
) -> Result<(StatusCode, Tagged<DayResponse>), AppError> {
    let at = resolve_at(request.at)?;
    // The client's local date can differ from the UTC date by up to a day
    if (request.work_date - at.date()).whole_days().abs() > 1 {
        return Err(invalid("workDate is too far from the clock-in time"));
    }

    let mut tx = db::begin_write(&pool).await?;

    let day_id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO work_days (job_id, work_date, kind, started_at, \
         expected_work_minutes, expected_lunch_minutes, expected_break_minutes, \
         lunch_paid, breaks_paid) \
         SELECT id, ?, 'work', ?, 0, 0, 0, 0, 0 FROM jobs \
         WHERE id = ? AND user_id = ? AND archived_at IS NULL \
         AND NOT EXISTS (SELECT 1 FROM work_days \
             WHERE job_id = jobs.id AND (work_date = ? OR (kind = 'work' AND ended_at IS NULL))) \
         RETURNING id",
    )
    .bind(request.work_date)
    .bind(at.unix_timestamp())
    .bind(job_id)
    .bind(user.id)
    .bind(request.work_date)
    .fetch_optional(&mut *tx)
    .await?;

    let Some(day_id) = day_id else {
        tx.rollback().await?;
        let state = live_state(&pool, job_id, user.id).await?;
        return Err(if state.archived {
            AppError::Conflict("the job is archived".into())
        } else if let Some(open_date) = state.open_work_date {
            AppError::Conflict(format!("already clocked in on {open_date}"))
        } else {
            AppError::Conflict("a day is already registered for this date".into())
        });
    };

    days::take_snapshot(&mut tx, day_id, request.work_date).await?;
    let day = load(&mut tx, job_id, request.work_date, user.id).await?;
    tx.commit().await?;

    Ok((StatusCode::CREATED, Tagged(day)))
}

/// Closes the open day. A pause still running is ended at the same time.
async fn clock_out(
    State(pool): State<SqlitePool>,
    user: CurrentUser,
    AppPath(job_id): AppPath<i64>,
    AppJson(request): AppJson<AtRequest>,
) -> Result<Tagged<DayResponse>, AppError> {
    let at = resolve_at(request.at)?.unix_timestamp();

    let mut tx = db::begin_write(&pool).await?;

    let closed = sqlx::query_as::<_, (i64, Date)>(
        "UPDATE work_days SET ended_at = ?, version = version + 1, updated_at = unixepoch() \
         WHERE job_id = ? AND kind = 'work' AND ended_at IS NULL \
         AND started_at < ? AND ? - started_at <= ? \
         AND job_id IN (SELECT id FROM jobs WHERE user_id = ?) \
         RETURNING id, work_date",
    )
    .bind(at)
    .bind(job_id)
    .bind(at)
    .bind(at)
    .bind(days::MAX_DAY_SECONDS)
    .bind(user.id)
    .fetch_optional(&mut *tx)
    .await?;

    let Some((day_id, work_date)) = closed else {
        tx.rollback().await?;
        let state = live_state(&pool, job_id, user.id).await?;
        return Err(match state.open_started_at {
            None => AppError::Conflict("not clocked in".into()),
            Some(started_at) if at <= started_at => invalid("clock-out must be after clock-in"),
            Some(_) => invalid("a day is at most 24 hours"),
        });
    };

    sqlx::query(
        "UPDATE pauses SET ended_at = ? \
         WHERE work_day_id = ? AND started_at IS NOT NULL AND ended_at IS NULL AND started_at < ?",
    )
    .bind(at)
    .bind(day_id)
    .bind(at)
    .execute(&mut *tx)
    .await?;

    // Pauses logged with times later than the clock-out would lie outside the day
    let outside = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (SELECT 1 FROM pauses WHERE work_day_id = ? \
         AND started_at IS NOT NULL AND (started_at >= ? OR ended_at > ?))",
    )
    .bind(day_id)
    .bind(at)
    .bind(at)
    .fetch_one(&mut *tx)
    .await?;
    if outside {
        tx.rollback().await?;
        return Err(invalid("a pause lies after the clock-out time"));
    }

    let day = load(&mut tx, job_id, work_date, user.id).await?;
    tx.commit().await?;
    Ok(Tagged(day))
}

async fn start_pause(
    State(pool): State<SqlitePool>,
    user: CurrentUser,
    AppPath(job_id): AppPath<i64>,
    AppJson(request): AppJson<PauseStartRequest>,
) -> Result<Tagged<DayResponse>, AppError> {
    let at = resolve_at(request.at)?.unix_timestamp();

    let mut tx = db::begin_write(&pool).await?;

    // The open day, a start within it, and no running or later-ending pause
    let day_id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO pauses (work_day_id, kind, started_at) \
         SELECT w.id, ?, ? FROM work_days w JOIN jobs j ON j.id = w.job_id \
         WHERE w.job_id = ? AND j.user_id = ? AND w.kind = 'work' AND w.ended_at IS NULL \
         AND ? >= w.started_at \
         AND NOT EXISTS (SELECT 1 FROM pauses p WHERE p.work_day_id = w.id \
             AND p.started_at IS NOT NULL AND (p.ended_at IS NULL OR p.ended_at > ?)) \
         RETURNING work_day_id",
    )
    .bind(request.kind)
    .bind(at)
    .bind(job_id)
    .bind(user.id)
    .bind(at)
    .bind(at)
    .fetch_optional(&mut *tx)
    .await?;

    let Some(day_id) = day_id else {
        tx.rollback().await?;
        let state = live_state(&pool, job_id, user.id).await?;
        return Err(match state.open_started_at {
            None => AppError::Conflict("not clocked in".into()),
            Some(_) if state.running_pause => {
                AppError::Conflict("a pause is already running".into())
            }
            Some(started_at) if at < started_at => invalid("a pause cannot start before clock-in"),
            Some(_) => invalid("a pause cannot start before the previous pause ended"),
        });
    };

    let day = bump_and_load(&mut tx, day_id, job_id, user.id).await?;
    tx.commit().await?;
    Ok(Tagged(day))
}

async fn end_pause(
    State(pool): State<SqlitePool>,
    user: CurrentUser,
    AppPath(job_id): AppPath<i64>,
    AppJson(request): AppJson<AtRequest>,
) -> Result<Tagged<DayResponse>, AppError> {
    let at = resolve_at(request.at)?.unix_timestamp();

    let mut tx = db::begin_write(&pool).await?;

    let day_id = sqlx::query_scalar::<_, i64>(
        "UPDATE pauses SET ended_at = ? \
         WHERE started_at < ? AND id = ( \
             SELECT p.id FROM pauses p \
             JOIN work_days w ON w.id = p.work_day_id JOIN jobs j ON j.id = w.job_id \
             WHERE w.job_id = ? AND j.user_id = ? AND w.ended_at IS NULL \
             AND p.started_at IS NOT NULL AND p.ended_at IS NULL) \
         RETURNING work_day_id",
    )
    .bind(at)
    .bind(at)
    .bind(job_id)
    .bind(user.id)
    .fetch_optional(&mut *tx)
    .await?;

    let Some(day_id) = day_id else {
        tx.rollback().await?;
        let state = live_state(&pool, job_id, user.id).await?;
        return Err(if state.running_pause {
            invalid("a pause must end after it starts")
        } else {
            AppError::Conflict("no pause is running".into())
        });
    };

    let day = bump_and_load(&mut tx, day_id, job_id, user.id).await?;
    tx.commit().await?;
    Ok(Tagged(day))
}

/// `at` defaults to now. Whole seconds, since instants are stored as Unix seconds.
fn resolve_at(at: Option<OffsetDateTime>) -> Result<OffsetDateTime, AppError> {
    let now = days::now();
    let at = at.unwrap_or(now);
    if at > now + MAX_CLOCK_SKEW {
        return Err(invalid("at must not be in the future"));
    }
    Ok(at)
}

/// A pause change is a change to the day, so the day's version moves on.
async fn bump_and_load(
    tx: &mut SqliteConnection,
    day_id: i64,
    job_id: i64,
    user_id: i64,
) -> Result<DayResponse, AppError> {
    let work_date = sqlx::query_scalar::<_, Date>(
        "UPDATE work_days SET version = version + 1, updated_at = unixepoch() \
         WHERE id = ? RETURNING work_date",
    )
    .bind(day_id)
    .fetch_one(&mut *tx)
    .await?;
    load(tx, job_id, work_date, user_id).await
}

async fn load(
    tx: &mut SqliteConnection,
    job_id: i64,
    work_date: Date,
    user_id: i64,
) -> Result<DayResponse, AppError> {
    days::load_day(tx, job_id, work_date, user_id)
        .await?
        .ok_or(AppError::NotFound)
}

/// The job's live state, read after a guarded write matched nothing, to pick the error.
#[derive(FromRow)]
struct LiveState {
    archived: bool,
    open_work_date: Option<Date>,
    open_started_at: Option<i64>,
    running_pause: bool,
}

/// Fails with 404 if the job doesn't exist for this user.
async fn live_state(pool: &SqlitePool, job_id: i64, user_id: i64) -> Result<LiveState, AppError> {
    sqlx::query_as::<_, LiveState>(
        "SELECT j.archived_at IS NOT NULL AS archived, \
         w.work_date AS open_work_date, w.started_at AS open_started_at, \
         EXISTS (SELECT 1 FROM pauses p WHERE p.work_day_id = w.id \
             AND p.started_at IS NOT NULL AND p.ended_at IS NULL) AS running_pause \
         FROM jobs j \
         LEFT JOIN work_days w ON w.job_id = j.id AND w.kind = 'work' AND w.ended_at IS NULL \
         WHERE j.id = ? AND j.user_id = ?",
    )
    .bind(job_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)
}
