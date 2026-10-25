#![expect(dead_code, reason = "used by the days module from step 5")]

//! Worked time, balance and leave-at for one day, as specified in docs/time-tracking.md.
//! Pure: no database, no clock. `now` is passed in, so the same input always gives
//! the same result.

use time::{Duration, OffsetDateTime};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PauseKind {
    Lunch,
    Break,
    Other,
}

/// A pause is either timed (open while `ended_at` is None) or a plain duration.
pub enum PauseTime {
    Timed {
        started_at: OffsetDateTime,
        ended_at: Option<OffsetDateTime>,
    },
    Minutes(i64),
}

pub struct Pause {
    pub kind: PauseKind,
    pub time: PauseTime,
}

/// The snapshot copied into the day when it was created.
pub struct Expected {
    pub work_minutes: i64,
    pub lunch_minutes: i64,
    pub break_minutes: i64,
    pub lunch_paid: bool,
    pub breaks_paid: bool,
}

/// A work day has times and pauses; a flex day has neither, so the type cannot
/// represent a flex day with times.
pub enum Day<'a> {
    Work {
        started_at: OffsetDateTime,
        ended_at: Option<OffsetDateTime>,
        pauses: &'a [Pause],
    },
    Flex,
}

/// Everything is a `Duration` (seconds); convert with `whole_minutes` only at the edge,
/// so sums over many days do not drift.
pub struct Totals {
    pub presence: Duration,
    pub lunch: Duration,
    pub breaks: Duration,
    pub other: Duration,
    pub allowance: Duration,
    pub worked: Duration,
    pub expected_work: Duration,
    pub balance: Duration,
    /// Only while the day is open
    pub leave_at: Option<OffsetDateTime>,
}

pub fn day_totals(day: &Day, expected: &Expected, now: OffsetDateTime) -> Totals {
    let expected_work = Duration::minutes(expected.work_minutes);

    let Day::Work {
        started_at,
        ended_at,
        pauses,
    } = day
    else {
        return Totals {
            presence: Duration::ZERO,
            lunch: Duration::ZERO,
            breaks: Duration::ZERO,
            other: Duration::ZERO,
            allowance: Duration::ZERO,
            worked: Duration::ZERO,
            expected_work,
            balance: -expected_work,
            leave_at: None,
        };
    };

    let expected_lunch = Duration::minutes(expected.lunch_minutes);
    let expected_break = Duration::minutes(expected.break_minutes);
    let lunch = PauseSum::of(pauses, PauseKind::Lunch, now);
    let breaks = PauseSum::of(pauses, PauseKind::Break, now);
    let other = PauseSum::of(pauses, PauseKind::Other, now);

    // Unlogged lunch or breaks default to what the schedule expects
    let actual_lunch = lunch.or_default(expected_lunch);
    let actual_breaks = breaks.or_default(expected_break);

    // Paid pauses are credited in full, whatever their actual length
    let allowance_lunch = if expected.lunch_paid {
        expected_lunch
    } else {
        Duration::ZERO
    };
    let allowance_breaks = if expected.breaks_paid {
        expected_break
    } else {
        Duration::ZERO
    };
    let allowance = allowance_lunch + allowance_breaks;

    let presence = non_negative(ended_at.unwrap_or(now) - *started_at);
    let worked = presence - actual_lunch - actual_breaks - other.total + allowance;

    let leave_at = ended_at.is_none().then(|| {
        // A finished short lunch moves leave-at earlier; a running or missing one
        // is planned at least as long as expected
        let planned_lunch = if lunch.logged == 0 || lunch.running {
            lunch.total.max(expected_lunch)
        } else {
            lunch.total
        };
        // Breaks are often split, so a finished break never means "no more breaks today"
        let planned_breaks = breaks.total.max(expected_break);

        *started_at + expected_work - allowance + planned_lunch + planned_breaks + other.total
    });

    Totals {
        presence,
        lunch: actual_lunch,
        breaks: actual_breaks,
        other: other.total,
        allowance,
        worked,
        expected_work,
        balance: worked - expected_work,
        leave_at,
    }
}

/// Rounds to the nearest whole minute, halves away from zero, so a balance of
/// +90 s and -90 s both round to the same size (2 minutes).
pub fn whole_minutes(duration: Duration) -> i64 {
    let seconds = duration.whole_seconds();
    let rounded = (seconds.abs() + 30) / 60;
    if seconds < 0 { -rounded } else { rounded }
}

/// The logged pauses of one kind: how many, whether one is running, and their total.
struct PauseSum {
    logged: usize,
    running: bool,
    total: Duration,
}

impl PauseSum {
    fn of(pauses: &[Pause], kind: PauseKind, now: OffsetDateTime) -> Self {
        let mut sum = PauseSum {
            logged: 0,
            running: false,
            total: Duration::ZERO,
        };
        for pause in pauses.iter().filter(|p| p.kind == kind) {
            sum.logged += 1;
            sum.total += match pause.time {
                PauseTime::Timed {
                    started_at,
                    ended_at: Some(ended_at),
                } => non_negative(ended_at - started_at),
                PauseTime::Timed {
                    started_at,
                    ended_at: None,
                } => {
                    sum.running = true;
                    non_negative(now - started_at)
                }
                PauseTime::Minutes(minutes) => Duration::minutes(minutes),
            };
        }
        sum
    }

    /// The logged total, or `expected` if nothing of this kind was logged.
    /// A logged duration of 0 counts as logged: the pause was skipped.
    fn or_default(&self, expected: Duration) -> Duration {
        if self.logged == 0 {
            expected
        } else {
            self.total
        }
    }
}

/// Clamps clock skew (e.g. `now` slightly before a start time) to zero instead of
/// letting it produce negative time.
fn non_negative(duration: Duration) -> Duration {
    duration.max(Duration::ZERO)
}
