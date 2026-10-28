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
