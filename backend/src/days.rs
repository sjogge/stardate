use std::collections::HashMap;

use axum::{Router, extract::State, http::StatusCode, routing::get};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqliteConnection, SqliteExecutor, SqlitePool};
use time::{Date, Duration, OffsetDateTime};

use crate::{
    auth::CurrentUser,
    calc,
    concurrency::{ExpectedVersion, Tagged, Versioned, WritePrecondition},
    db,
    error::AppError,
    extract::{AppJson, AppPath, AppQuery},
    jobs,
};

const MAX_DAY_SECONDS: i64 = 24 * 60 * 60;
const MAX_MINUTES_PER_DAY: i64 = 24 * 60;
const MAX_NOTE_CHARS: usize = 1000;
const MAX_PAUSES: usize = 50;
const MAX_RANGE_DAYS: i64 = 366;

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(rename_all = "lowercase")]
pub enum DayKind {
    Work,
    Flex,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(rename_all = "lowercase")]
pub enum PauseKind {
    Lunch,
    Break,
    Other,
}

impl From<PauseKind> for calc::PauseKind {
    fn from(kind: PauseKind) -> Self {
        match kind {
            PauseKind::Lunch => calc::PauseKind::Lunch,
            PauseKind::Break => calc::PauseKind::Break,
            PauseKind::Other => calc::PauseKind::Other,
        }
    }
}

#[derive(FromRow)]
struct DayRow {
    id: i64,
    work_date: Date,
    kind: DayKind,
    started_at: Option<OffsetDateTime>,
    ended_at: Option<OffsetDateTime>,
    schedule_id: Option<i64>,
    expected_work_minutes: i64,
    expected_lunch_minutes: i64,
    expected_break_minutes: i64,
    lunch_paid: bool,
    breaks_paid: bool,
    note: Option<String>,
    version: i64,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

#[derive(FromRow)]
struct PauseRow {
    id: i64,
    work_day_id: i64,
    kind: PauseKind,
    started_at: Option<OffsetDateTime>,
    ended_at: Option<OffsetDateTime>,
    minutes: Option<i64>,
}

// ---------- Responses ----------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DayResponse {
    id: i64,
    work_date: Date,
    kind: DayKind,
    #[serde(with = "time::serde::rfc3339::option")]
    started_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    ended_at: Option<OffsetDateTime>,
    note: Option<String>,
    /// The schedule the snapshot was taken from; None if there was none or it was deleted
    schedule_id: Option<i64>,
    expected: ExpectedDto,
    pauses: Vec<PauseResponse>,
    totals: TotalsDto,
    version: i64,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    updated_at: OffsetDateTime,
}

impl Versioned for DayResponse {
    fn version(&self) -> i64 {
        self.version
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ExpectedDto {
    work_minutes: i64,
    lunch_minutes: i64,
    break_minutes: i64,
    lunch_paid: bool,
    breaks_paid: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PauseResponse {
    id: i64,
    kind: PauseKind,
    #[serde(with = "time::serde::rfc3339::option")]
    started_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    ended_at: Option<OffsetDateTime>,
    minutes: Option<i64>,
}

/// Computed values, in whole minutes. Lunch and break are the actual minutes,
/// defaulting to the expected ones when none were logged.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TotalsDto {
    presence_minutes: i64,
    lunch_minutes: i64,
    break_minutes: i64,
    other_minutes: i64,
    allowance_minutes: i64,
    worked_minutes: i64,
    expected_work_minutes: i64,
    balance_minutes: i64,
    #[serde(with = "time::serde::rfc3339::option")]
    leave_at: Option<OffsetDateTime>,
}

impl DayResponse {
    fn new(row: DayRow, pause_rows: Vec<PauseRow>, now: OffsetDateTime) -> Self {
        let expected = calc::Expected {
            work_minutes: row.expected_work_minutes,
            lunch_minutes: row.expected_lunch_minutes,
            break_minutes: row.expected_break_minutes,
            lunch_paid: row.lunch_paid,
            breaks_paid: row.breaks_paid,
        };
        let calc_pauses: Vec<calc::Pause> = pause_rows.iter().map(to_calc_pause).collect();
        let day = match (row.kind, row.started_at) {
            (DayKind::Work, Some(started_at)) => calc::Day::Work {
                started_at,
                ended_at: row.ended_at,
                pauses: &calc_pauses,
            },
            // The CHECK constraints rule out a work day without a start
            _ => calc::Day::Flex,
        };
        let totals = calc::day_totals(&day, &expected, now);

        Self {
            id: row.id,
            work_date: row.work_date,
            kind: row.kind,
            started_at: row.started_at,
            ended_at: row.ended_at,
            note: row.note,
            schedule_id: row.schedule_id,
            expected: ExpectedDto {
                work_minutes: expected.work_minutes,
                lunch_minutes: expected.lunch_minutes,
                break_minutes: expected.break_minutes,
                lunch_paid: expected.lunch_paid,
                breaks_paid: expected.breaks_paid,
            },
            pauses: pause_rows
                .into_iter()
                .map(|pause| PauseResponse {
                    id: pause.id,
                    kind: pause.kind,
                    started_at: pause.started_at,
                    ended_at: pause.ended_at,
                    minutes: pause.minutes,
                })
                .collect(),
            totals: TotalsDto {
                presence_minutes: calc::whole_minutes(totals.presence),
                lunch_minutes: calc::whole_minutes(totals.lunch),
                break_minutes: calc::whole_minutes(totals.breaks),
                other_minutes: calc::whole_minutes(totals.other),
                allowance_minutes: calc::whole_minutes(totals.allowance),
                worked_minutes: calc::whole_minutes(totals.worked),
                expected_work_minutes: calc::whole_minutes(totals.expected_work),
                balance_minutes: calc::whole_minutes(totals.balance),
                leave_at: totals.leave_at,
            },
            version: row.version,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

fn to_calc_pause(row: &PauseRow) -> calc::Pause {
    let time = match (row.started_at, row.minutes) {
        (Some(started_at), _) => calc::PauseTime::Timed {
            started_at,
            ended_at: row.ended_at,
        },
        // The CHECK constraint guarantees minutes when there is no start
        (None, minutes) => calc::PauseTime::Minutes(minutes.unwrap_or(0)),
    };
    calc::Pause {
        kind: row.kind.into(),
        time,
    }
}

// ---------- Requests ----------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DayRequest {
    kind: DayKind,
    #[serde(default, with = "time::serde::rfc3339::option")]
    started_at: Option<OffsetDateTime>,
    #[serde(default, with = "time::serde::rfc3339::option")]
    ended_at: Option<OffsetDateTime>,
    #[serde(default)]
    pauses: Vec<PauseRequest>,
    note: Option<String>,
    /// Overrides individual snapshot values
    expected: Option<ExpectedOverride>,
    /// Re-takes the snapshot from the schedule valid on the work date
    #[serde(default)]
    refresh_expected: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PauseRequest {
    kind: PauseKind,
    #[serde(default, with = "time::serde::rfc3339::option")]
    started_at: Option<OffsetDateTime>,
    #[serde(default, with = "time::serde::rfc3339::option")]
    ended_at: Option<OffsetDateTime>,
    minutes: Option<i64>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct ExpectedOverride {
    work_minutes: Option<i64>,
    lunch_minutes: Option<i64>,
    break_minutes: Option<i64>,
    lunch_paid: Option<bool>,
    breaks_paid: Option<bool>,
}

/// A request that passed validation, with instants as Unix seconds (UTC, whole
/// seconds) ready to bind.
struct ValidDay {
    kind: DayKind,
    started_at: Option<i64>,
    ended_at: Option<i64>,
    pauses: Vec<ValidPause>,
    note: Option<String>,
    expected: ExpectedOverride,
    refresh_expected: bool,
}

struct ValidPause {
    kind: PauseKind,
    started_at: Option<i64>,
    ended_at: Option<i64>,
    minutes: Option<i64>,
}

fn invalid(message: &str) -> AppError {
    AppError::Validation(message.into())
}

impl DayRequest {
    fn validate(self, now: OffsetDateTime) -> Result<ValidDay, AppError> {
        if self.expected.is_some() && self.refresh_expected {
            return Err(invalid("send either expected or refreshExpected, not both"));
        }
        let expected = self.expected.unwrap_or_default();
        for minutes in [
            expected.work_minutes,
            expected.lunch_minutes,
            expected.break_minutes,
        ]
        .into_iter()
        .flatten()
        {
            if !(0..=MAX_MINUTES_PER_DAY).contains(&minutes) {
                return Err(invalid("expected minutes must be between 0 and 1440"));
            }
        }

        let note = self.note.map(|note| note.trim().to_string());
        if note
            .as_ref()
            .is_some_and(|n| n.chars().count() > MAX_NOTE_CHARS)
        {
            return Err(invalid("note must be at most 1000 characters"));
        }
        // An empty note is stored as no note
        let note = note.filter(|n| !n.is_empty());

        let started_at = self.started_at.map(OffsetDateTime::unix_timestamp);
        let ended_at = self.ended_at.map(OffsetDateTime::unix_timestamp);

        let pauses = match self.kind {
            DayKind::Flex => {
                if started_at.is_some() || ended_at.is_some() || !self.pauses.is_empty() {
                    return Err(invalid("a flex day has no times and no pauses"));
                }
                Vec::new()
            }
            DayKind::Work => {
                let Some(start) = started_at else {
                    return Err(invalid("a work day needs startedAt"));
                };
                if let Some(end) = ended_at {
                    if end <= start {
                        return Err(invalid("endedAt must be after startedAt"));
                    }
                    if end - start > MAX_DAY_SECONDS {
                        return Err(invalid("a day is at most 24 hours"));
                    }
                }
                // An open day's pauses may run up to now
                let day_end = ended_at.unwrap_or_else(|| now.unix_timestamp());
                validate_pauses(self.pauses, start, ended_at, day_end)?
            }
        };

        Ok(ValidDay {
            kind: self.kind,
            started_at,
            ended_at,
            pauses,
            note,
            expected,
            refresh_expected: self.refresh_expected,
        })
    }
}

fn validate_pauses(
    pauses: Vec<PauseRequest>,
    day_start: i64,
    day_ended_at: Option<i64>,
    day_end: i64,
) -> Result<Vec<ValidPause>, AppError> {
    if pauses.len() > MAX_PAUSES {
        return Err(invalid("a day has at most 50 pauses"));
    }

    let mut valid = Vec::with_capacity(pauses.len());
    // (start, end) of timed pauses; a running pause ends "now", i.e. day_end
    let mut intervals = Vec::new();

    for pause in pauses {
        let started_at = pause.started_at.map(OffsetDateTime::unix_timestamp);
        let ended_at = pause.ended_at.map(OffsetDateTime::unix_timestamp);

        match (started_at, ended_at, pause.minutes) {
            (Some(start), end, None) => {
                if end.is_none() && day_ended_at.is_some() {
                    return Err(invalid("a closed day cannot have a running pause"));
                }
                let end_or_now = end.unwrap_or(day_end);
                if end.is_some_and(|end| end <= start) {
                    return Err(invalid("a pause must end after it starts"));
                }
                if start < day_start || end_or_now > day_end {
                    return Err(invalid("pauses must lie within the day"));
                }
                intervals.push((start, end_or_now, end.is_none()));
            }
            (None, None, Some(minutes)) => {
                if !(0..=MAX_MINUTES_PER_DAY).contains(&minutes) {
                    return Err(invalid("pause minutes must be between 0 and 1440"));
                }
            }
            _ => {
                return Err(invalid(
                    "a pause has either startedAt (and endedAt once finished) or minutes",
                ));
            }
        }

        valid.push(ValidPause {
            kind: pause.kind,
            started_at,
            ended_at,
            minutes: pause.minutes,
        });
    }

    intervals.sort_unstable();
    if intervals.iter().filter(|(_, _, running)| *running).count() > 1 {
        return Err(invalid("only one pause can be running"));
    }
    if intervals.windows(2).any(|pair| pair[1].0 < pair[0].1) {
        return Err(invalid("timed pauses must not overlap"));
    }

    Ok(valid)
}

#[derive(Deserialize)]
pub struct DateRange {
    from: Date,
    to: Date,
}

// ---------- Routes ----------

pub fn routes() -> Router<SqlitePool> {
    Router::new()
        .route("/", get(list_days))
        .route("/{date}", get(get_day).put(put_day).delete(delete_day))
}

async fn list_days(
    State(pool): State<SqlitePool>,
    user: CurrentUser,
    AppPath(job_id): AppPath<i64>,
    AppQuery(range): AppQuery<DateRange>,
) -> Result<AppJson<Vec<DayResponse>>, AppError> {
    if range.to < range.from {
        return Err(invalid("to must not be before from"));
    }
    if range.to - range.from >= Duration::days(MAX_RANGE_DAYS) {
        return Err(invalid("a range covers at most 366 days"));
    }

    // Read-only transaction: days and pauses come from one snapshot
    let mut tx = pool.begin().await?;
    jobs::ensure_owned(&mut *tx, job_id, user.id).await?;

    let rows = sqlx::query_as::<_, DayRow>(
        "SELECT id, work_date, kind, started_at, ended_at, schedule_id, expected_work_minutes, \
         expected_lunch_minutes, expected_break_minutes, lunch_paid, breaks_paid, note, version, \
         created_at, updated_at \
         FROM work_days WHERE job_id = ? AND work_date BETWEEN ? AND ? ORDER BY work_date",
    )
    .bind(job_id)
    .bind(range.from)
    .bind(range.to)
    .fetch_all(&mut *tx)
    .await?;

    let pause_rows = sqlx::query_as::<_, PauseRow>(
        "SELECT p.id, p.work_day_id, p.kind, p.started_at, p.ended_at, p.minutes \
         FROM pauses p JOIN work_days w ON w.id = p.work_day_id \
         WHERE w.job_id = ? AND w.work_date BETWEEN ? AND ? \
         ORDER BY p.work_day_id, p.started_at, p.id",
    )
    .bind(job_id)
    .bind(range.from)
    .bind(range.to)
    .fetch_all(&mut *tx)
    .await?;

    let mut pauses_by_day: HashMap<i64, Vec<PauseRow>> = HashMap::new();
    for pause in pause_rows {
        pauses_by_day
            .entry(pause.work_day_id)
            .or_default()
            .push(pause);
    }

    let now = OffsetDateTime::now_utc();
    let days = rows
        .into_iter()
        .map(|row| {
            let pauses = pauses_by_day.remove(&row.id).unwrap_or_default();
            DayResponse::new(row, pauses, now)
        })
        .collect();

    Ok(AppJson(days))
}

async fn get_day(
    State(pool): State<SqlitePool>,
    user: CurrentUser,
    AppPath((job_id, work_date)): AppPath<(i64, Date)>,
) -> Result<Tagged<DayResponse>, AppError> {
    let mut tx = pool.begin().await?;
    let day = load_day(&mut tx, job_id, work_date, user.id)
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(Tagged(day))
}

/// Creates the day (`If-None-Match: *`) or replaces it (`If-Match`). The schedule
/// snapshot is taken on create and on `refreshExpected`; otherwise it is kept.
async fn put_day(
    State(pool): State<SqlitePool>,
    user: CurrentUser,
    AppPath((job_id, work_date)): AppPath<(i64, Date)>,
    precondition: WritePrecondition,
    AppJson(request): AppJson<DayRequest>,
) -> Result<(StatusCode, Tagged<DayResponse>), AppError> {
    let day = request.validate(OffsetDateTime::now_utc())?;

    let mut tx = db::begin_write(&pool).await?;

    let (day_id, status) = match precondition {
        WritePrecondition::CreateOnly => {
            // NOT EXISTS turns "day already exists" into 0 rows instead of a UNIQUE error
            let day_id = sqlx::query_scalar::<_, i64>(
                "INSERT INTO work_days (job_id, work_date, kind, started_at, ended_at, note, \
                 expected_work_minutes, expected_lunch_minutes, expected_break_minutes, \
                 lunch_paid, breaks_paid) \
                 SELECT id, ?, ?, ?, ?, ?, 0, 0, 0, 0, 0 FROM jobs \
                 WHERE id = ? AND user_id = ? \
                 AND NOT EXISTS (SELECT 1 FROM work_days WHERE job_id = ? AND work_date = ?) \
                 RETURNING id",
            )
            .bind(work_date)
            .bind(day.kind)
            .bind(day.started_at)
            .bind(day.ended_at)
            .bind(&day.note)
            .bind(job_id)
            .bind(user.id)
            .bind(job_id)
            .bind(work_date)
            .fetch_optional(&mut *tx)
            .await?;

            let Some(day_id) = day_id else {
                tx.rollback().await?;
                // The job exists for this user, so the day must already exist
                jobs::ensure_owned(&pool, job_id, user.id).await?;
                return Err(AppError::PreconditionFailed);
            };
            take_snapshot(&mut tx, day_id, work_date).await?;
            (day_id, StatusCode::CREATED)
        }
        WritePrecondition::Replace(version) => {
            let day_id = sqlx::query_scalar::<_, i64>(
                "UPDATE work_days SET kind = ?, started_at = ?, ended_at = ?, note = ?, \
                 version = version + 1, updated_at = unixepoch() \
                 WHERE job_id = ? AND work_date = ? AND version = ? \
                 AND job_id IN (SELECT id FROM jobs WHERE user_id = ?) \
                 RETURNING id",
            )
            .bind(day.kind)
            .bind(day.started_at)
            .bind(day.ended_at)
            .bind(&day.note)
            .bind(job_id)
            .bind(work_date)
            .bind(version)
            .bind(user.id)
            .fetch_optional(&mut *tx)
            .await?;

            let Some(day_id) = day_id else {
                tx.rollback().await?;
                return Err(not_found_or_modified(&pool, job_id, work_date, user.id).await);
            };
            if day.refresh_expected {
                take_snapshot(&mut tx, day_id, work_date).await?;
            }
            (day_id, StatusCode::OK)
        }
    };

    apply_overrides(&mut tx, day_id, &day.expected).await?;
    replace_pauses(&mut tx, day_id, &day.pauses).await?;

    let response = load_day(&mut tx, job_id, work_date, user.id)
        .await?
        .ok_or(AppError::NotFound)?;
    if response.kind == DayKind::Flex && response.expected.work_minutes == 0 {
        tx.rollback().await?;
        return Err(invalid(
            "a flex day needs expected work that day; there is nothing to take from the balance",
        ));
    }
    tx.commit().await?;

    Ok((status, Tagged(response)))
}

async fn delete_day(
    State(pool): State<SqlitePool>,
    user: CurrentUser,
    AppPath((job_id, work_date)): AppPath<(i64, Date)>,
    ExpectedVersion(version): ExpectedVersion,
) -> Result<StatusCode, AppError> {
    // Pauses go with it through ON DELETE CASCADE
    let result = sqlx::query(
        "DELETE FROM work_days WHERE job_id = ? AND work_date = ? AND version = ? \
         AND job_id IN (SELECT id FROM jobs WHERE user_id = ?)",
    )
    .bind(job_id)
    .bind(work_date)
    .bind(version)
    .bind(user.id)
    .execute(&pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(not_found_or_modified(&pool, job_id, work_date, user.id).await);
    }
    Ok(StatusCode::NO_CONTENT)
}

// ---------- Helpers ----------

/// Copies the expected values of the schedule valid on `work_date` into the day.
/// No schedule, or no row for that weekday, means everything expected is 0.
async fn take_snapshot(
    tx: &mut SqliteConnection,
    day_id: i64,
    work_date: Date,
) -> Result<(), AppError> {
    let weekday = i64::from(work_date.weekday().number_from_monday());

    sqlx::query(
        "UPDATE work_days SET schedule_id = ( \
             SELECT id FROM schedules \
             WHERE job_id = work_days.job_id AND valid_from <= work_days.work_date \
             ORDER BY valid_from DESC LIMIT 1) \
         WHERE id = ?",
    )
    .bind(day_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "UPDATE work_days SET \
         expected_work_minutes = COALESCE((SELECT work_minutes FROM schedule_days \
             WHERE schedule_id = work_days.schedule_id AND weekday = ?), 0), \
         expected_lunch_minutes = COALESCE((SELECT lunch_minutes FROM schedule_days \
             WHERE schedule_id = work_days.schedule_id AND weekday = ?), 0), \
         expected_break_minutes = COALESCE((SELECT break_minutes FROM schedule_days \
             WHERE schedule_id = work_days.schedule_id AND weekday = ?), 0), \
         lunch_paid = COALESCE((SELECT lunch_paid FROM schedules WHERE id = work_days.schedule_id), 0), \
         breaks_paid = COALESCE((SELECT breaks_paid FROM schedules WHERE id = work_days.schedule_id), 0) \
         WHERE id = ?",
    )
    .bind(weekday)
    .bind(weekday)
    .bind(weekday)
    .bind(day_id)
    .execute(&mut *tx)
    .await?;

    Ok(())
}

/// Overwrites only the snapshot values the request names; NULL keeps the current one.
async fn apply_overrides(
    tx: &mut SqliteConnection,
    day_id: i64,
    overrides: &ExpectedOverride,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE work_days SET \
         expected_work_minutes = COALESCE(?, expected_work_minutes), \
         expected_lunch_minutes = COALESCE(?, expected_lunch_minutes), \
         expected_break_minutes = COALESCE(?, expected_break_minutes), \
         lunch_paid = COALESCE(?, lunch_paid), \
         breaks_paid = COALESCE(?, breaks_paid) \
         WHERE id = ?",
    )
    .bind(overrides.work_minutes)
    .bind(overrides.lunch_minutes)
    .bind(overrides.break_minutes)
    .bind(overrides.lunch_paid)
    .bind(overrides.breaks_paid)
    .bind(day_id)
    .execute(&mut *tx)
    .await?;
    Ok(())
}

/// Pauses are replaced as a whole list, like the request describes them.
async fn replace_pauses(
    tx: &mut SqliteConnection,
    day_id: i64,
    pauses: &[ValidPause],
) -> Result<(), AppError> {
    sqlx::query("DELETE FROM pauses WHERE work_day_id = ?")
        .bind(day_id)
        .execute(&mut *tx)
        .await?;

    for pause in pauses {
        sqlx::query(
            "INSERT INTO pauses (work_day_id, kind, started_at, ended_at, minutes) \
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(day_id)
        .bind(pause.kind)
        .bind(pause.started_at)
        .bind(pause.ended_at)
        .bind(pause.minutes)
        .execute(&mut *tx)
        .await?;
    }
    Ok(())
}

/// Loads a day with its pauses and computes its totals. None if the day does not
/// exist for this user and job.
async fn load_day(
    tx: &mut SqliteConnection,
    job_id: i64,
    work_date: Date,
    user_id: i64,
) -> Result<Option<DayResponse>, AppError> {
    let row = sqlx::query_as::<_, DayRow>(
        "SELECT w.id, w.work_date, w.kind, w.started_at, w.ended_at, w.schedule_id, \
         w.expected_work_minutes, w.expected_lunch_minutes, w.expected_break_minutes, \
         w.lunch_paid, w.breaks_paid, w.note, w.version, w.created_at, w.updated_at \
         FROM work_days w JOIN jobs j ON j.id = w.job_id \
         WHERE w.job_id = ? AND w.work_date = ? AND j.user_id = ?",
    )
    .bind(job_id)
    .bind(work_date)
    .bind(user_id)
    .fetch_optional(&mut *tx)
    .await?;

    let Some(row) = row else {
        return Ok(None);
    };
    let pauses = load_pauses(&mut *tx, row.id).await?;
    Ok(Some(DayResponse::new(
        row,
        pauses,
        OffsetDateTime::now_utc(),
    )))
}

async fn load_pauses(
    executor: impl SqliteExecutor<'_>,
    day_id: i64,
) -> Result<Vec<PauseRow>, AppError> {
    Ok(sqlx::query_as::<_, PauseRow>(
        "SELECT id, work_day_id, kind, started_at, ended_at, minutes FROM pauses \
         WHERE work_day_id = ? ORDER BY started_at, id",
    )
    .bind(day_id)
    .fetch_all(executor)
    .await?)
}

/// A versioned write that matched no row: 404 if the day doesn't exist for this
/// user and job, 412 if it exists at another version.
async fn not_found_or_modified(
    pool: &SqlitePool,
    job_id: i64,
    work_date: Date,
    user_id: i64,
) -> AppError {
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (SELECT 1 FROM work_days w JOIN jobs j ON j.id = w.job_id \
         WHERE w.job_id = ? AND w.work_date = ? AND j.user_id = ?)",
    )
    .bind(job_id)
    .bind(work_date)
    .bind(user_id)
    .fetch_one(pool)
    .await;

    match exists {
        Ok(true) => AppError::PreconditionFailed,
        Ok(false) => AppError::NotFound,
        Err(e) => e.into(),
    }
}
