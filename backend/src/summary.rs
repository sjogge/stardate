//! Per-day and range totals for a date range, e.g. a week or a month.

use axum::{Router, extract::State, routing::get};
use serde::Serialize;
use sqlx::SqlitePool;
use time::{Date, Duration};

use crate::{
    auth::CurrentUser,
    calc,
    days::{self, DateRange, DayKind},
    error::AppError,
    extract::{AppJson, AppPath, AppQuery},
    jobs,
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SummaryResponse {
    from: Date,
    to: Date,
    days: Vec<DaySummary>,
    totals: RangeTotals,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DaySummary {
    work_date: Date,
    kind: DayKind,
    /// Values of an open day are "so far"; it is left out of the range totals
    open: bool,
    worked_minutes: i64,
    expected_work_minutes: i64,
    balance_minutes: i64,
}

/// Sums of the closed days in the range, plus the adjustments dated in it.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RangeTotals {
    work_days: usize,
    flex_days: usize,
    worked_minutes: i64,
    expected_work_minutes: i64,
    days_balance_minutes: i64,
    adjustments_minutes: i64,
    balance_minutes: i64,
}

pub fn routes() -> Router<SqlitePool> {
    Router::new().route("/summary", get(summary))
}

async fn summary(
    State(pool): State<SqlitePool>,
    user: CurrentUser,
    AppPath(job_id): AppPath<i64>,
    AppQuery(range): AppQuery<DateRange>,
) -> Result<AppJson<SummaryResponse>, AppError> {
    range.validate()?;

    let mut tx = pool.begin().await?;
    jobs::ensure_owned(&mut *tx, job_id, user.id).await?;

    let day_totals = days::totals_in_range(
        &mut tx,
        job_id,
        Some(range.from),
        Some(range.to),
        days::now(),
    )
    .await?;

    let adjustment_minutes = sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE(SUM(minutes), 0) FROM flex_adjustments \
         WHERE job_id = ? AND adjusted_on BETWEEN ? AND ?",
    )
    .bind(job_id)
    .bind(range.from)
    .bind(range.to)
    .fetch_one(&mut *tx)
    .await?;

    // Sum seconds first and round once, so the totals don't drift from rounding each day
    let closed = || day_totals.iter().filter(|day| !day.open);
    let worked: Duration = closed().map(|day| day.totals.worked).sum();
    let expected: Duration = closed().map(|day| day.totals.expected_work).sum();
    let days_balance: Duration = closed().map(|day| day.totals.balance).sum();
    let adjustments = Duration::minutes(adjustment_minutes);

    let totals = RangeTotals {
        work_days: closed().filter(|day| day.kind == DayKind::Work).count(),
        flex_days: closed().filter(|day| day.kind == DayKind::Flex).count(),
        worked_minutes: calc::whole_minutes(worked),
        expected_work_minutes: calc::whole_minutes(expected),
        days_balance_minutes: calc::whole_minutes(days_balance),
        adjustments_minutes: calc::whole_minutes(adjustments),
        balance_minutes: calc::whole_minutes(days_balance + adjustments),
    };

    let days = day_totals
        .into_iter()
        .map(|day| DaySummary {
            work_date: day.work_date,
            kind: day.kind,
            open: day.open,
            worked_minutes: calc::whole_minutes(day.totals.worked),
            expected_work_minutes: calc::whole_minutes(day.totals.expected_work),
            balance_minutes: calc::whole_minutes(day.totals.balance),
        })
        .collect();

    Ok(AppJson(SummaryResponse {
        from: range.from,
        to: range.to,
        days,
        totals,
    }))
}
