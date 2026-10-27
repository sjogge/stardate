//! What the start screen shows: for each active job, whether you are clocked in,
//! today's day and the flex balance. One request instead of several per job.

use axum::extract::State;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use time::Date;

use crate::{
    auth::CurrentUser,
    calc,
    days::{self, DayResponse, RunningPause},
    error::AppError,
    extract::{AppJson, AppQuery},
    flex,
    live::{self, LiveAction},
};

#[derive(Deserialize)]
pub struct StatusQuery {
    /// The client's local date, used to show today's day when not clocked in
    date: Option<Date>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobStatus {
    job_id: i64,
    job_name: String,
    clocked_in: bool,
    running_pause: Option<RunningPause>,
    /// The live actions that are valid now. Without `date`, a day already registered
    /// today is unknown, so `clockIn` may still answer 409.
    available_actions: Vec<LiveAction>,
    /// The open day, or else the day registered on `date`, if any
    today: Option<DayResponse>,
    /// Closed days and all adjustments; today's open day counts once clocked out
    flex_balance_minutes: i64,
}

pub async fn status(
    State(pool): State<SqlitePool>,
    user: CurrentUser,
    AppQuery(query): AppQuery<StatusQuery>,
) -> Result<AppJson<Vec<JobStatus>>, AppError> {
    let now = days::now();
    // Read-only transaction: every job's values come from one snapshot
    let mut tx = pool.begin().await?;

    // Without `date` the client's day is unknown, so clock-in is offered and checked when sent
    let jobs = sqlx::query_as::<_, (i64, String, Option<Date>, bool)>(
        "SELECT j.id, j.name, \
         (SELECT work_date FROM work_days \
          WHERE job_id = j.id AND kind = 'work' AND ended_at IS NULL), \
         ? IS NULL OR EXISTS (SELECT 1 FROM schedules WHERE job_id = j.id AND valid_from <= ?) \
         FROM jobs j WHERE j.user_id = ? AND j.archived_at IS NULL ORDER BY j.name",
    )
    .bind(query.date)
    .bind(query.date)
    .bind(user.id)
    .fetch_all(&mut *tx)
    .await?;

    let mut statuses = Vec::with_capacity(jobs.len());
    for (job_id, job_name, open_date, schedule_covers_date) in jobs {
        let today = match open_date.or(query.date) {
            Some(date) => days::load_day(&mut tx, job_id, date, user.id).await?,
            None => None,
        };
        let balance = flex::balance(&mut tx, job_id, now).await?;

        statuses.push(JobStatus {
            job_id,
            job_name,
            clocked_in: today.as_ref().is_some_and(DayResponse::is_open),
            running_pause: today.as_ref().and_then(DayResponse::running_pause),
            available_actions: live::available_actions(today.as_ref(), schedule_covers_date),
            today,
            flex_balance_minutes: calc::whole_minutes(balance.total()),
        });
    }

    Ok(AppJson(statuses))
}
