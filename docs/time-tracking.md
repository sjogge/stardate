# Time tracking specification

## Concepts

- **Job**: an employment, workplace or project. A user has one or more jobs.
- **Schedule**: one named week of planned shifts for a job (e.g. "Daytime", "Nights"): per weekday a
  start and end time with lunch and break minutes, and whether lunch and breaks are paid. A job can
  have several; every day is registered against the one the user picks. Schedules can be linked
  into a rotation (`next_schedule_id`: the schedule of the following week).
- **Work day**: one registered calendar day for a job. Either `work` (with start and end) or `flex`
  (a day off taken from the flex balance).
- **Pause**: lunch, break or other (unpaid absence, e.g. a dentist visit) within a work day.
- **Flex balance**: sum of the balance of all registered days plus manual adjustments.

## Time handling

- Instants (start, end, pause times) are stored as `INTEGER` Unix seconds (UTC).
  The API accepts RFC 3339 with any offset, normalizes to UTC and truncates to whole seconds.
  Responses use RFC 3339 in UTC.
- Calendar dates (`work_date`, `adjusted_on`) are stored as `TEXT` `YYYY-MM-DD`.
- A schedule's planned times are local clock times, stored as minutes after midnight and sent as
  `HH:MM`. They describe a shift, not instants, so they need no time zone.
- The server is timezone-agnostic. The client sends the local work date when clocking in or
  registering a day, and formats instants in local time for display.
- Weekdays are derived from the work date (ISO 8601, 1 = Monday). No timezone is needed for that.
- Durations are computed in seconds and exposed in the API as whole minutes (rounded to nearest,
  halves away from zero).
  Sum seconds first, convert last, so totals do not drift.

## Data model

Migration `0001_time_tracking.sql`:

```sql
CREATE TABLE users (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    name       TEXT    NOT NULL CHECK (length(name) BETWEEN 1 AND 100),
    created_at INTEGER NOT NULL DEFAULT (unixepoch())
) STRICT;

-- Single user until authentication exists
INSERT INTO users (id, name) VALUES (1, 'default');

CREATE TABLE jobs (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id     INTEGER NOT NULL REFERENCES users(id),
    name        TEXT    NOT NULL CHECK (length(name) BETWEEN 1 AND 100),
    archived_at INTEGER,
    version     INTEGER NOT NULL DEFAULT 1,
    created_at  INTEGER NOT NULL DEFAULT (unixepoch()),
    updated_at  INTEGER NOT NULL DEFAULT (unixepoch()),
    UNIQUE (user_id, name)
) STRICT;

-- One week of planned shifts. Every day is registered against a schedule the user picks;
-- `next_schedule_id` links weeks into a rotation (evenings -> daytime -> nights -> evenings).
CREATE TABLE schedules (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    job_id           INTEGER NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
    name             TEXT    NOT NULL CHECK (length(name) BETWEEN 1 AND 100),
    -- No defaults: paid or unpaid must be a deliberate choice per employment
    lunch_paid       INTEGER NOT NULL CHECK (lunch_paid IN (0, 1)),
    breaks_paid      INTEGER NOT NULL CHECK (breaks_paid IN (0, 1)),
    next_schedule_id INTEGER REFERENCES schedules(id) ON DELETE SET NULL,
    version          INTEGER NOT NULL DEFAULT 1,
    created_at       INTEGER NOT NULL DEFAULT (unixepoch()),
    updated_at       INTEGER NOT NULL DEFAULT (unixepoch()),
    UNIQUE (job_id, name)
) STRICT;

-- A planned shift as clock times (minutes after midnight); an end before the start is on the
-- next day. A missing weekday means no expected work that day.
CREATE TABLE schedule_days (
    schedule_id   INTEGER NOT NULL REFERENCES schedules(id) ON DELETE CASCADE,
    weekday       INTEGER NOT NULL CHECK (weekday BETWEEN 1 AND 7),
    start_minute  INTEGER NOT NULL CHECK (start_minute BETWEEN 0 AND 1439),
    end_minute    INTEGER NOT NULL CHECK (end_minute BETWEEN 0 AND 1439),
    lunch_minutes INTEGER NOT NULL CHECK (lunch_minutes BETWEEN 0 AND 1440),
    break_minutes INTEGER NOT NULL CHECK (break_minutes BETWEEN 0 AND 1440),
    PRIMARY KEY (schedule_id, weekday),
    CHECK (start_minute != end_minute)
) STRICT;

CREATE TABLE work_days (
    id                     INTEGER PRIMARY KEY AUTOINCREMENT,
    job_id                 INTEGER NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
    work_date              TEXT    NOT NULL CHECK (date(work_date) = work_date),
    kind                   TEXT    NOT NULL DEFAULT 'work' CHECK (kind IN ('work', 'flex')),
    started_at             INTEGER,
    ended_at               INTEGER,
    -- The schedule picked for the day, and a snapshot of its expected values taken then,
    -- so schedule changes never alter history
    schedule_id            INTEGER REFERENCES schedules(id) ON DELETE SET NULL,
    expected_work_minutes  INTEGER NOT NULL CHECK (expected_work_minutes >= 0),
    expected_lunch_minutes INTEGER NOT NULL CHECK (expected_lunch_minutes >= 0),
    expected_break_minutes INTEGER NOT NULL CHECK (expected_break_minutes >= 0),
    lunch_paid             INTEGER NOT NULL CHECK (lunch_paid IN (0, 1)),
    breaks_paid            INTEGER NOT NULL CHECK (breaks_paid IN (0, 1)),
    note                   TEXT,
    version                INTEGER NOT NULL DEFAULT 1,
    created_at             INTEGER NOT NULL DEFAULT (unixepoch()),
    updated_at             INTEGER NOT NULL DEFAULT (unixepoch()),
    UNIQUE (job_id, work_date),
    -- Only work days have times
    CHECK ((kind = 'work') = (started_at IS NOT NULL)),
    -- SQLite CHECKs pass on NULL, so guard the end time explicitly
    CHECK (started_at IS NOT NULL OR ended_at IS NULL),
    CHECK (ended_at IS NULL OR ended_at > started_at)
) STRICT;

CREATE UNIQUE INDEX work_days_one_open
    ON work_days(job_id) WHERE kind = 'work' AND ended_at IS NULL;

CREATE TABLE pauses (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    work_day_id INTEGER NOT NULL REFERENCES work_days(id) ON DELETE CASCADE,
    kind        TEXT    NOT NULL CHECK (kind IN ('lunch', 'break', 'other')),
    started_at  INTEGER,
    ended_at    INTEGER,
    minutes     INTEGER CHECK (minutes >= 0),
    -- Either timed (start, end once finished) or a plain duration, never both
    CHECK (
        (started_at IS NOT NULL AND minutes IS NULL)
        OR (started_at IS NULL AND ended_at IS NULL AND minutes IS NOT NULL)
    ),
    CHECK (ended_at IS NULL OR ended_at > started_at)
) STRICT;

CREATE INDEX pauses_work_day ON pauses(work_day_id);
CREATE UNIQUE INDEX pauses_one_open
    ON pauses(work_day_id) WHERE started_at IS NOT NULL AND ended_at IS NULL;

CREATE TABLE flex_adjustments (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    job_id      INTEGER NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
    adjusted_on TEXT    NOT NULL CHECK (date(adjusted_on) = adjusted_on),
    minutes     INTEGER NOT NULL CHECK (minutes != 0),
    note        TEXT,
    version     INTEGER NOT NULL DEFAULT 1,
    created_at  INTEGER NOT NULL DEFAULT (unixepoch()),
    updated_at  INTEGER NOT NULL DEFAULT (unixepoch())
) STRICT;

CREATE INDEX flex_adjustments_job ON flex_adjustments(job_id);
```

Deleting a job removes all its data through `ON DELETE CASCADE`: schedules (and their weekday rows),
work days (and their pauses) and flex adjustments. Only archived jobs can be deleted (409 otherwise),
so active history cannot be lost by a single request.

## Rules

### Schedules, choice and snapshot

- **Every day names its schedule.** Creating a day (also by clock-in, flex days included) requires
  `scheduleId`, a schedule of the same job (400 otherwise). A job without schedules cannot register
  days, so worked time never silently counts as overtime.
- A weekday row is a planned shift: `start` and `end` (an end before the start is on the next day,
  so 22:00 to 07:00 is 9 hours), lunch and break minutes. Its expected work is derived:
  `expected_work = (end - start) - unpaid lunch - unpaid breaks`. A shift belongs to the weekday it
  starts on.
- When a day is **created**, the chosen schedule's row for the day's weekday is copied into it
  (expected work, lunch and break minutes, paid flags): the **snapshot**. No row for that weekday
  (e.g. a Saturday) means all expected values are 0, so all worked time is overtime.
- Schedules and their weekday rows may be edited or deleted freely. This never changes existing
  work days. Sending `scheduleId` again when replacing a day re-takes the snapshot (another
  schedule, or the same one after editing it).
- A **bonus day** is work on a day the schedule expects work, but nothing is owed (e.g. a public
  holiday): it is registered with `expected` work, lunch and break minutes set to 0, so all worked
  time counts as plus.

### Suggested schedule

The schedule a client preselects for a new day on a date is computed by the backend, so every
device suggests the same:

- Take the job's latest day before the date that has a schedule, and move along `next_schedule_id`
  once per calendar week (Monday to Monday) between that day and the date. An unlinked schedule
  repeats. Weeks without registered days (a vacation) still move a rotation on.
- No earlier day: the job's first schedule (lowest id). No schedules: none.
- It is only a suggestion: the user may pick any schedule of the job.

### Worked time for a `work` day

Per pause kind (lunch, break):

- **actual** = sum of logged pauses of that kind. If none are logged, it defaults to the expected
  minutes. A timed pause counts `ended_at - started_at`, a running pause counts until now, a
  duration-only pause counts `minutes`. A logged duration of 0 means the pause was skipped.
- **allowance** = expected minutes if that kind is paid, otherwise 0. The allowance is credited
  regardless of the actual pause length.

`other` pauses have no default and no allowance; they are always deducted.

```
presence = (ended_at or now) - started_at
worked   = presence - actual_lunch - actual_break - other + allowance_lunch + allowance_break
balance  = worked - expected_work
```

So a shorter paid lunch lets you leave earlier, and a longer one costs the extra time.

### `flex` day

- `worked = 0`, so `balance = -expected_work` (from the snapshot).
- No times and no pauses.
- Reject with 400 if the expected work for that date is 0 (it would cost nothing).

### Leave at (daily only, never based on the flex balance)

While a work day is open:

```
leave_at = started_at + expected_work - allowance_lunch - allowance_break + planned pauses
```

Planned pauses, where actual-so-far includes a running pause up to now:

- **lunch**: the larger of actual-so-far and expected while no lunch is logged or one is running;
  the actual minutes once all logged lunch pauses are finished. A short lunch moves `leave_at`
  earlier when it ends.
- **break**: the larger of actual-so-far and expected for as long as the day is open. Breaks are
  often split into several, so a finished first break must not be read as "no more breaks today".
- **other**: actual-so-far (no expectation).

`leave_at` is a forecast. `worked` and `balance` always use the actual pauses (with the defaults
above), so a short break still counts in full once the day is closed.

### Flex balance

```
flex = sum(balance of every closed registered day) + sum(flex_adjustments.minutes)
```

A day that is still open (clocked in, not out) is left out until it is clocked out: its balance is
only "so far" and would swing from negative to positive during the day.

Days that are not registered are ignored (sick days, vacation and forgotten days have no effect).
Forgotten days can be registered later.

### Validation

- `ended_at > started_at`, and a day is at most 24 hours.
- A schedule's weekday row: start and end differ, and lunch plus breaks fit in the shift.
  Schedule names are unique per job; `nextScheduleId` must be a schedule of the same job.
- Timed pauses lie within the day (`started_at` to `ended_at`, or now if open) and do not overlap.
- Only one open work day per job and one running pause per day (also enforced by indexes).
- No clock-in on an archived job (409).
- A day has a single interval. A mid-day absence is a pause of kind `other`.

### Editing and deleting days

- Editing a day replaces its times, pauses (as a whole list), note and kind. The snapshot is kept,
  unless the edit sends `scheduleId`: then it is re-taken from that schedule.
- The edit may override snapshot values via `expected` (any of `workMinutes`, `lunchMinutes`,
  `breakMinutes`, `lunchPaid`, `breaksPaid`); with `scheduleId`, the overrides apply after the
  snapshot is re-taken.
- Moving a day to another date is a delete plus a new registration, since the snapshot depends on
  the date.
- Deleting a day removes its pauses (cascade) and its effect on the flex balance.

## API

All routes under `/api`, JSON in `camelCase`, errors as `{"message": "..."}`.

| Method           | Path                                         | Purpose                                                                                                 |
| ---------------- | -------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| GET, POST        | `/jobs`                                      | List / create jobs                                                                                      |
| GET, PUT, DELETE | `/jobs/{id}`                                 | Get / rename / delete (archived jobs only)                                                              |
| POST             | `/jobs/{id}/archive`                         | Archive (history kept)                                                                                  |
| GET, POST        | `/jobs/{id}/schedules`                       | List / create schedules (with weekday rows)                                                             |
| GET              | `/jobs/{id}/schedules/suggested?date=`       | `{ scheduleId }`: the schedule to preselect for a new day on `date`                                     |
| GET, PUT, DELETE | `/jobs/{id}/schedules/{scheduleId}`          | Get / edit / delete a schedule                                                                          |
| GET              | `/jobs/{id}/days?from=&to=`                  | Days in a date range, with totals                                                                       |
| GET, PUT, DELETE | `/jobs/{id}/days/{date}`                     | Get / create or replace / delete a day                                                                  |
| POST             | `/jobs/{id}/clock-in`                        | `{ workDate, scheduleId, at? }` creates the day with snapshot, opens it                                 |
| POST             | `/jobs/{id}/clock-out`                       | `{ at? }` closes the open day                                                                           |
| POST             | `/jobs/{id}/pauses/start`                    | `{ kind, at? }` on the open day                                                                         |
| POST             | `/jobs/{id}/pauses/end`                      | `{ at? }` ends the running pause                                                                        |
| GET              | `/jobs/{id}/summary?from=&to=`               | Per-day totals and range totals                                                                         |
| GET              | `/jobs/{id}/flex`                            | Total flex balance                                                                                      |
| GET, POST        | `/jobs/{id}/flex-adjustments`                | List / add adjustments                                                                                  |
| PUT, DELETE      | `/jobs/{id}/flex-adjustments/{adjustmentId}` | Edit / delete                                                                                           |
| GET              | `/status`                                    | Per active job: clocked in, valid live actions, running pause, today's day (with `leaveAt`), total flex |

`at` defaults to the server's current time. It may lie at most one minute in the future (clock
differences between devices). Live action bodies are JSON objects; send `{}` to use the defaults.

Live actions (`clock-in`, `clock-out`, `pauses/start`, `pauses/end`) need no `If-Match`. They are
guarded by the day's state and answer 409 when it does not fit:

- `clock-in`: 409 if the job is archived, if a day is already open ("already clocked in on <date>"),
  or if a day is already registered for `workDate`. `scheduleId` is required (a schedule of the
  same job, 400 otherwise). `workDate` may differ from the UTC date of `at` by at most one day (the
  client's local date). The day is created with the snapshot of that schedule.
- `clock-out`: 409 if not clocked in. A running pause is ended at the same time. 400 if `at` is not
  after clock-in, the day would exceed 24 hours, or a logged pause lies after `at`.
- `pauses/start`: 409 if not clocked in or a pause is already running. 400 if `at` is before
  clock-in or before the previous pause ended.
- `pauses/end`: 409 if no pause is running. 400 if `at` is not after the pause start.

Each returns the day like `GET /jobs/{id}/days/{date}` (201 for clock-in, otherwise 200).

`GET /jobs/{id}/summary?from=&to=` (inclusive, at most 366 days) returns `days` (per day: `workDate`,
`kind`, `open`, `workedMinutes`, `expectedWorkMinutes`, `balanceMinutes`) and `totals` (`workDays`,
`flexDays`, `workedMinutes`, `expectedWorkMinutes`, `daysBalanceMinutes`, `adjustmentsMinutes` dated
in the range, `balanceMinutes`). Totals cover closed days only and are summed in seconds before
rounding.

`GET /jobs/{id}/flex` returns `balanceMinutes`, `daysMinutes` and `adjustmentsMinutes`.

Flex adjustments take `{ adjustedOn, minutes, note? }`; `minutes` is non-zero (positive adds,
negative takes) and at most a year of full days.

`GET /status?date=` returns one entry per active job: `jobId`, `jobName`, `clockedIn`, `runningPause`
(`kind`, `startedAt`), `availableActions`, `suggestedScheduleId`, `today` and `flexBalanceMinutes`.
`today` is the open day, or else the day registered on `date` (the client's local date, optional), in
the same shape as a day response. `suggestedScheduleId` is the suggested schedule for clocking in on
`date` (see "Suggested schedule"); null when a day is already there, without `date`, or without
schedules.

`availableActions` lists the live actions that will not answer 409, so clients do not re-derive
the rules:

| State                                    | `availableActions`           |
| ---------------------------------------- | ---------------------------- |
| No day on `date`, the job has a schedule | `["clockIn"]`                |
| No day on `date`, no schedules           | `[]`                         |
| Clocked in, no pause running             | `["startPause", "clockOut"]` |
| Clocked in, a pause running              | `["endPause", "clockOut"]`   |
| Day on `date` closed, or flex day        | `[]`                         |

`startPause` covers every pause kind. Without `date`, a day already registered today is unknown,
so `clockIn` may still answer 409.

Example of registering a past day:

```json
PUT /api/jobs/1/days/2026-10-23
If-None-Match: *

{
  "kind": "work",
  "scheduleId": 1,
  "startedAt": "2026-10-23T08:00:00+02:00",
  "endedAt": "2026-10-23T17:00:00+02:00",
  "pauses": [
    { "kind": "lunch", "minutes": 60 },
    { "kind": "break", "startedAt": "2026-10-23T10:00:00+02:00", "endedAt": "2026-10-23T10:15:00+02:00" }
  ],
  "note": "Forgot to clock in"
}
```

A schedule (`POST /jobs/{id}/schedules`, or `PUT` with `If-Match` to replace it and its rows):

```json
{
  "name": "Nights",
  "lunchPaid": false,
  "breaksPaid": true,
  "nextScheduleId": 1,
  "days": [
    { "weekday": 1, "start": "22:00", "end": "07:00", "lunchMinutes": 30, "breakMinutes": 30 }
  ]
}
```

Responses add the derived `workMinutes` per weekday (here 8:30) and `nextScheduleId` may be omitted
or null. Deleting a schedule keeps the snapshots of its days (their `scheduleId` becomes null) and
clears links to it.

`PUT /jobs/{id}/days/{date}` details:

- `kind` is `work` (needs `startedAt`; `endedAt` may be omitted for an open day) or `flex` (no times,
  no pauses).
- Each pause has `kind` (`lunch`, `break`, `other`) and either `startedAt` (plus `endedAt` once
  finished) or `minutes`. Only an open day can have a running pause.
- `scheduleId` is required on create (400 without it). On replace it is optional: sending it
  re-takes the snapshot from that schedule.
- `expected` overrides individual snapshot values (`workMinutes`, `lunchMinutes`, `breakMinutes`,
  `lunchPaid`, `breaksPaid`) on create or replace, after any snapshot.
- Only one open work day per job: a second open day is a 409.

`GET /jobs/{id}/days?from=&to=` takes inclusive dates and covers at most 366 days.

A day response contains the stored data, the snapshot and the computed values, in whole minutes:

```json
{
  "id": 5, "workDate": "2026-10-23", "kind": "work",
  "startedAt": "2026-10-23T06:00:00Z", "endedAt": "2026-10-23T15:00:00Z",
  "note": "Forgot to clock in", "scheduleId": 3,
  "expected": { "workMinutes": 480, "lunchMinutes": 60, "breakMinutes": 30,
                "lunchPaid": false, "breaksPaid": true },
  "pauses": [ { "id": 9, "kind": "lunch", "startedAt": null, "endedAt": null, "minutes": 60 } ],
  "totals": { "presenceMinutes": 540, "lunchMinutes": 60, "breakMinutes": 15, "otherMinutes": 0,
              "allowanceMinutes": 30, "workedMinutes": 495, "expectedWorkMinutes": 480,
              "balanceMinutes": 15, "leaveAt": null },
  "version": 1, "createdAt": "...", "updatedAt": "..."
}
```

`totals.lunchMinutes` and `totals.breakMinutes` are the actual minutes, defaulting to the expected
ones when none were logged. `leaveAt` is set only while the day is open.

## Optimistic concurrency

- `jobs`, `schedules` (covering their weekday rows), `work_days` (covering their pauses) and
  `flex_adjustments` have a `version`.
- Single-resource responses include `version` in the body and `ETag: "<version>"`. List items include
  `version`.
- `PUT` and `DELETE` require `If-Match: "<version>"`. The update is one statement:
  `UPDATE ... SET ..., version = version + 1, updated_at = unixepoch() WHERE id = ? AND version = ?`.
  If no row is affected: 404 if the row does not exist, otherwise 412.
- `PUT /jobs/{id}/days/{date}` also accepts `If-None-Match: *` for create-only (412 if the day exists).
  Without either header: 428.
- Clock-in/out and pause start/end need no precondition (they are guarded by state and return 409
  on an invalid state) but they bump the day's version.
- Implement `If-Match` parsing as a small extractor (e.g. `ExpectedVersion(i64)`), and add
  `AppError::PreconditionFailed` (412) and `AppError::PreconditionRequired` (428).

## Database access

- SQLite runs in WAL mode with `synchronous=NORMAL` (see `db.rs`). Backups use `VACUUM INTO`, not a
  copy of the main file alone.
- Preconditions (ownership, version, state) are part of the write statement:
  `UPDATE ... WHERE id = ? AND version = ? AND job_id IN (SELECT id FROM jobs WHERE user_id = ?)`,
  `INSERT INTO ... SELECT ... FROM jobs WHERE id = ? AND user_id = ?`. A single conditional write
  needs no transaction.
- If a conditional write affects no rows, a read-only follow-up query decides the status: 404 if the
  row does not exist for this user, 412 if the version differs, 409 if the state is wrong.
- Multi-statement writes run in a transaction started with `BEGIN IMMEDIATE` (`db::begin_write`),
  never `pool.begin()`. The write lock is held from the start, so reads inside the transaction cannot
  go stale (`SQLITE_BUSY_SNAPSHOT`). Roll back if the first write affects no rows.
- The schedule snapshot (on create and when `scheduleId` is sent) is taken inside the day's write
  transaction (`db::begin_write`), so it can never mix values from a half-edited schedule.

## Suggested implementation order

One commit per step, each building and passing clippy:

1. Concurrency building blocks (extractor, ETag helper, new error variants).
2. Migration `0001_time_tracking.sql`; jobs CRUD and archive.
3. Schedules with weekday rows.
   - 3b. Delete archived jobs: `ON DELETE CASCADE` from `jobs`, `DELETE /jobs/{id}` with `If-Match`.
4. Pure calculation module (worked time, balance, leave-at).
5. Days: GET list/single, PUT, DELETE, including snapshot handling.
6. Live actions: clock-in/out, pause start/end.
7. Summary, flex balance, flex adjustments, status.

