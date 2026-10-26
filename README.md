# Stardate

A small self-hosted web app for tracking worked time: clock in and out, record
lunch and breaks, compare against a work schedule (e.g. 9 h/day with 1 h lunch
and 30 min breaks), and register time for past days. Supports multiple jobs and
keeps a flex balance.

Runs as a single Docker container on a Raspberry Pi, so the footprint is kept
small: one Rust binary with SQLite compiled in.

> Learning project: Rust (backend) and Svelte (frontend).

## Status

| Area                                                                                              | State                                              |
| ------------------------------------------------------------------------------------------------- | -------------------------------------------------- |
| Backend infrastructure (config, DB, migrations, logging, errors, health check, graceful shutdown) | Done                                               |
| Docker image + Compose                                                                            | Done                                               |
| Data model and schema                                                                             | Done (`0001_time_tracking.sql`)                    |
| API: jobs, schedules                                                                              | Done                                               |
| API: days, clock-in/out, pauses, summary, flex, status                                            | Done                                               |
| Frontend (Svelte)                                                                                 | In progress (scaffolded in `web/`)                 |
| Tests                                                                                             | Later                                              |
| Authentication                                                                                    | Later. **Do not expose beyond the LAN until then** |

The domain model, rules and full API are specified in
[`docs/time-tracking.md`](docs/time-tracking.md).

## Tech stack

- **Backend:** Rust, axum, sqlx (SQLite), tokio, time, tracing, anyhow
- **Database:** SQLite (bundled into the binary, file stored in a Docker volume)
- **Frontend:** SvelteKit 2 + Svelte 5 SPA (TypeScript, Tailwind CSS v4, Skeleton v5), built to
  static files that axum will serve from the same origin
- **Runtime:** Docker on a Raspberry Pi (arm64)

## Project structure

```
stardate/
├── backend/
│   ├── Cargo.toml
│   ├── build.rs             # rebuild when migrations change
│   ├── Dockerfile           # multi-stage: rust:alpine build -> alpine runtime
│   ├── .dockerignore
│   ├── migrations/          # SQL migrations, embedded in the binary at compile time
│   ├── http/                # REST Client requests: seed.http, stardate.http
│   └── src/
│       ├── main.rs          # startup, router, logging, graceful shutdown
│       ├── config.rs        # configuration from environment variables
│       ├── db.rs            # SQLite pool, pragmas, begin_write (BEGIN IMMEDIATE)
│       ├── error.rs         # AppError -> HTTP response mapping
│       ├── extract.rs       # AppJson / AppPath: axum extractors with JSON errors
│       ├── auth.rs          # CurrentUser extractor (seeded user until auth exists)
│       ├── concurrency.rs   # If-Match extractor and ETag responses
│       ├── health.rs        # GET /api/health
│       ├── calc.rs          # pure day calculations: worked time, balance, leave-at
│       ├── jobs.rs          # jobs API
│       ├── schedules.rs     # schedules API (with weekday rows)
│       ├── days.rs          # days API (register, edit, delete days)
│       ├── live.rs          # clock-in/out and pause start/end
│       ├── summary.rs       # per-day and range totals
│       ├── flex.rs          # flex balance and adjustments
│       └── status.rs        # start-screen status of all active jobs
├── web/                     # Svelte SPA; rules in web/CLAUDE.md
│   ├── package.json         # npm scripts: dev, check, lint, format, test, build
│   ├── vite.config.ts       # SvelteKit, static adapter, /api dev proxy, Vitest
│   └── src/
│       ├── app.html         # sets theme and light/dark mode before first paint
│       ├── lib/
│       │   ├── api/client.ts  # the only fetch: ApiError, If-Match, one global error handler
│       │   ├── api/types.ts   # response types, matching the Rust DTOs
│       │   ├── components/    # reusable UI: AppNavigation (bottom bar / side rail), ConfirmDialog
│       │   ├── features/      # per feature: workday/ (Today), jobs/, schedules/, settings/
│       │   ├── format.ts      # display formatting: durations, balances, local times and dates
│       │   ├── theme.svelte.ts  # theme and light/dark/system choice, stored in localStorage
│       │   └── toast.ts       # the toaster; identical error toasts are shown once
│       ├── params/          # route matchers: integer ids, e.g. /jobs/[jobId=integer]
│       └── routes/          # thin pages (/, /jobs, /jobs/{id}, /settings); +layout.ts turns off SSR
├── docs/
│   └── time-tracking.md     # domain and API specification
├── compose.yaml
├── CLAUDE.md                # instructions for Claude Code
└── README.md
```

Planned: the `web/` build is served by axum from the same container.

## Configuration

| Variable       | Required | Default                         | Description                                                           |
| -------------- | -------- | ------------------------------- | --------------------------------------------------------------------- |
| `DATABASE_URL` | Yes      | none                            | SQLite URL, e.g. `sqlite:stardate.db` or `sqlite:///data/stardate.db` |
| `BIND_ADDR`    | No       | `0.0.0.0:3000`                  | Address and port to listen on                                         |
| `RUST_LOG`     | No       | `stardate=info,tower_http=info` | Log filter, e.g. `debug`                                              |

The app refuses to start if a required variable is missing.

## Run locally

```sh
cd backend
DATABASE_URL=sqlite:stardate.db cargo run
```

PowerShell: `$env:DATABASE_URL="sqlite:stardate.db"; cargo run`

The database file is created on first start, and migrations run automatically.

Before committing, from `backend/`:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo build
```

### Frontend

With the backend running on port 3000, from `web/`:

```sh
npm ci
npm run dev        # http://localhost:5173, /api is proxied to the backend
```

Before committing, from `web/`:

```sh
npm run check
npm run lint
npm test           # component tests need Chromium once: npx playwright install chromium
npm run build
```

## Run with Docker

```sh
docker compose up --build -d
docker compose logs -f api
curl localhost:3000/api/health
```

Data lives in the `stardate-data` named volume and survives rebuilds.
`docker compose down -v` **deletes the database**.

### Trying the API

The `backend/http/` folder has requests for the VS Code
[REST Client](https://marketplace.visualstudio.com/items?itemName=humao.rest-client)
extension:

- `seed.http` builds one job with a schedule history on an empty database.
- `stardate.http` exercises every implemented endpoint and the expected errors.

### Inspecting the container

```sh
docker compose exec api sh                          # shell inside the running container
docker compose run --rm api sh                      # shell when the app won't start
docker compose cp api:/data/stardate.db ./copy.db   # copy the DB out (stop first, or also copy -wal/-shm)
```

## API

All routes are under `/api`, and JSON uses `camelCase`. Instants are RFC 3339 in
UTC; dates are `YYYY-MM-DD`.

### Implemented

| Method | Path                                    | Description                                                                        |
| ------ | --------------------------------------- | ---------------------------------------------------------------------------------- |
| GET    | `/api/health`                           | 200 if the app and database respond                                                |
| GET    | `/api/jobs`                             | List jobs, active first, then archived                                             |
| POST   | `/api/jobs`                             | Create a job: `{ "name": "..." }`                                                  |
| GET    | `/api/jobs/{id}`                        | Get a job                                                                          |
| PUT    | `/api/jobs/{id}`                        | Rename a job (requires `If-Match`)                                                 |
| DELETE | `/api/jobs/{id}`                        | Delete an archived job and all its data (requires `If-Match`)                      |
| POST   | `/api/jobs/{id}/archive`                | Archive a job, keeping its history (idempotent)                                    |
| GET    | `/api/jobs/{id}/schedules`              | List schedule versions, ordered by `validFrom`, with weekday rows                  |
| POST   | `/api/jobs/{id}/schedules`              | Create a schedule version                                                          |
| GET    | `/api/jobs/{id}/schedules/{scheduleId}` | Get a schedule version                                                             |
| PUT    | `/api/jobs/{id}/schedules/{scheduleId}` | Replace a schedule version and its weekday rows (requires `If-Match`)              |
| DELETE | `/api/jobs/{id}/schedules/{scheduleId}` | Delete a schedule version (requires `If-Match`); existing days keep their snapshot |
| GET    | `/api/jobs/{id}/days?from=&to=` | Days in an inclusive date range (at most 366 days), with totals |
| GET    | `/api/jobs/{id}/days/{date}` | One day with its pauses, snapshot and totals |
| PUT    | `/api/jobs/{id}/days/{date}` | Create (`If-None-Match: *`) or replace (`If-Match`) a day |
| DELETE | `/api/jobs/{id}/days/{date}` | Delete a day and its pauses (requires `If-Match`) |
| POST   | `/api/jobs/{id}/clock-in` | `{ "workDate": "...", "at"?: "..." }`: create today's day with its snapshot and open it |
| POST   | `/api/jobs/{id}/clock-out` | `{ "at"?: "..." }`: close the open day, ending a running pause |
| POST   | `/api/jobs/{id}/pauses/start` | `{ "kind": "lunch" \| "break" \| "other", "at"?: "..." }` on the open day |
| POST   | `/api/jobs/{id}/pauses/end` | `{ "at"?: "..." }`: end the running pause |
| GET    | `/api/jobs/{id}/summary?from=&to=` | Per-day totals and range totals (closed days plus adjustments in the range) |
| GET    | `/api/jobs/{id}/flex` | Flex balance: closed days plus all adjustments |
| GET    | `/api/jobs/{id}/flex-adjustments` | List manual adjustments |
| POST   | `/api/jobs/{id}/flex-adjustments` | `{ "adjustedOn": "...", "minutes": 120, "note"?: "..." }` |
| PUT    | `/api/jobs/{id}/flex-adjustments/{adjustmentId}` | Edit an adjustment (requires `If-Match`) |
| DELETE | `/api/jobs/{id}/flex-adjustments/{adjustmentId}` | Delete an adjustment (requires `If-Match`) |
| GET    | `/api/status?date=` | Every active job: clocked in, running pause, valid live actions, today's day, flex balance |

The full specification is in the API section of [`docs/time-tracking.md`](docs/time-tracking.md).

### Optimistic concurrency

Versioned resources return their version in the body (`version`) and in an
`ETag: "<version>"` header. Updates must send it back as `If-Match: "<version>"`.
If someone else changed the resource in the meantime, the update fails with 412,
and the client reloads and retries.

### Errors

Errors are returned as JSON `{ "message": "..." }`:

| Status | When                                                                                                                            |
| ------ | ------------------------------------------------------------------------------------------------------------------------------- |
| 400    | Validation failed, a CHECK / NOT NULL constraint was violated, malformed JSON, an invalid path value, or a malformed `If-Match` |
| 404    | Resource not found                                                                                                              |
| 409    | Unique or foreign key constraint violated (e.g. duplicate job name or deleting a job that is not archived)                      |
| 412    | `If-Match` version is out of date: the resource was modified by another request                                                 |
| 413    | Request body too large                                                                                                          |
| 415    | `Content-Type: application/json` missing on a request with a JSON body                                                          |
| 422    | JSON is valid but has the wrong shape (e.g. a missing or mistyped field)                                                        |
| 428    | `If-Match` header is missing on an update that requires it                                                                      |
| 500    | Unexpected database error (details only in the server log)                                                                      |
| 503    | Database connection pool exhausted (retry later)                                                                                |

## Design decisions

### SQLite in WAL mode

The database runs in WAL (write-ahead log) mode with `synchronous=NORMAL`, set in `db.rs`.
Writes are appended to `stardate.db-wal` and folded into the main file at checkpoints.

- **Why:** reads are never blocked by a write, and there are fewer disk syncs than with the
  rollback journal, which means less SD card wear on the Pi.
- **Trade-offs:**
    - Three files (`stardate.db`, `-wal`, `-shm`). A backup must use `VACUUM INTO` or copy all
      three with the app stopped; copying only `stardate.db` can miss recent writes.
    - With `synchronous=NORMAL`, a power cut can lose the last few commits, but never corrupts
      the database. `synchronous=FULL` would sync every commit, at the cost of more writes.
    - WAL does not work on network filesystems. The database must stay on a local volume.
- A clean shutdown (`pool.close()` in graceful shutdown) checkpoints the WAL into the main file.

### Write transactions start with `BEGIN IMMEDIATE`

SQLite allows one writer at a time for the whole database. A plain `BEGIN` takes no lock; the
first statement takes a snapshot. If that statement is a read and another request commits before
the transaction's first write, the write fails at once with `SQLITE_BUSY_SNAPSHOT`.

- Every transaction that writes starts with `db::begin_write(&pool)`, which sends
  `BEGIN IMMEDIATE`: the write lock is held from the start, so reads inside the transaction can
  never go stale. Never use `pool.begin()` for writes.
- Another write transaction waits up to `busy_timeout` (5 s). Reads are not blocked.
- Keep transactions short: only SQL between `begin_write` and `commit`.

### Conditional writes

Preconditions go into the write statement itself instead of a separate check before it:

| Precondition                     | Example                                                                                |
| -------------------------------- | -------------------------------------------------------------------------------------- |
| Ownership on insert              | `INSERT INTO schedules (...) SELECT id, ?, ... FROM jobs WHERE id = ? AND user_id = ?` |
| Ownership on update/delete       | `... AND job_id IN (SELECT id FROM jobs WHERE user_id = ?)`                            |
| Version (optimistic concurrency) | `UPDATE ... WHERE id = ? AND version = ?`                                              |
| State                            | `DELETE FROM jobs WHERE ... AND archived_at IS NOT NULL`                               |

- **Why:** the check and the write are one atomic step, so there is no gap between them for
  another request to change the data (no check-then-act race).
- A single conditional write needs no transaction.
- **0 rows affected** means a precondition failed. A read-only follow-up query then picks the
  status: 404 (doesn't exist for this user), 412 (stale version) or 409 (wrong state).
- Reads are scoped the same way, by joining through `jobs.user_id`. Authentication will only have
  to change how `CurrentUser` is resolved.

### Valid live actions come from the backend

`GET /api/status` returns `availableActions` per job (`clockIn`, `clockOut`, `startPause`,
`endPause`), computed by `live::available_actions` next to the handlers whose guards it mirrors.

- **Why:** which action is valid is a business rule. Every client must agree on it, so the
  frontend shows the buttons it is given instead of re-deriving them from `clockedIn` and
  `runningPause`.
- The list says what is allowed, not what to show first. Picking the primary button (e.g. lunch
  before break) is presentation and stays in the frontend.
- Without `date`, a day already registered today is unknown, so `clockIn` can still answer 409.

### Versions travel in the body; `If-Match` sends them back

Versioned resources return their version twice: in the body (`version`) and in the `ETag` header.
The frontend reads the body and sends the version back in the standard `If-Match` header.

- **Why not read the `ETag` header:** list responses (e.g. `GET /jobs/{id}/days`) hold many
  versioned items but can have only one header, so the body version is needed anyway. Reading it
  everywhere keeps one way of doing it, and the version stays with its data in Svelte state.
- **Why the backend still sends `ETag`:** it costs nothing and lets generic HTTP tools use it.
  Conditional `GET` (`If-None-Match`, 304) is not used: responses are small and the app always
  wants fresh data.

### The workday screen picks the primary button, the backend picks what is allowed

`GET /api/status` lists the allowed live actions per job (`availableActions`). The frontend only
decides how to show them (`features/workday/actions.ts`): the most likely next step becomes the
large button, the others are smaller.

- Order: clock in; else end the running pause; else start lunch while lunch is expected and none
  is logged; else clock out. Starting a break or an absence stays available as a smaller button.
- The screen reloads the status after every action (also after a failed one, since the state may
  have changed on another device), every minute while visible, and when the tab becomes visible
  again. Worked time and leave-at are computed by the server at each load, so there is no client
  clock to drift.

### The schedule form

A job's schedules are edited at `/jobs/{id}` in a dialog (full screen on phones).

- **A new version starts as a copy of the latest one,** since a change from a certain date usually
  changes little. The "applies from" date and its effect are shown first: days already registered
  keep their snapshot, so a schedule change never alters history.
- **Paid lunch and breaks have no default** for a job's first schedule, matching the backend
  (`ScheduleRequest` has no defaults): the form asks until both are chosen.
- **Durations are typed as `8`, `8:00` or `0:45`** (`format.ts`, `parseDuration`) instead of in
  minutes or with `<input type="time">`, which would show a clock (AM/PM in some locales) for what
  is a duration. A weekday left empty means no work is expected that day.
- **The form validates itself** (`novalidate`, `scheduleForm.ts`) and shows the first problem in one
  alert, naming the weekday and field; the backend still validates every request.

### Back-dated actions send a local date and time

"Use a different time" on the workday screen sends the next action with `at` (an RFC 3339
instant) instead of letting the server use its current time; e.g. for a forgotten clock-in.

- The date and time inputs are read in the device's time zone and sent in UTC
  (`format.ts`, `toInstant`). The server stays timezone-agnostic.
- A back-dated clock-in is registered on the chosen date (`workDate`), so a forgotten night shift
  lands on the day it started. The server accepts a `workDate` at most one day from `at`.
- The choice applies to one action and then resets, so the next action happens now again.
- Times in the future are left to the server (400, shown as a toast); the inputs only block an
  incomplete date or time.
- A whole forgotten day is registered with `PUT /jobs/{id}/days/{date}` instead (History and
  edits).

### Theme applied before first paint

The theme and light/dark mode are set on `<html>` (`data-theme`, `data-mode`) by an inline script in
`web/src/app.html`, before the app loads. After that, `lib/theme.svelte.ts` keeps them up to date.

- **Why:** a script that runs after the app loads would show the default theme for a moment
  (a flash) on every reload.
- **Trade-off:** the inline script cannot import TypeScript, so the theme list and the storage keys
  are in three places: `app.html`, `lib/theme.svelte.ts` and the theme imports in
  `routes/layout.css`. Each has a comment pointing to the others.
- Dark mode uses `data-mode` instead of only the OS setting (a `@custom-variant dark` in
  `layout.css`), so "light" and "dark" can override the OS; "system" follows it, also while the app
  is open.

## Deploy to the Raspberry Pi

Build an arm64 image on a PC and copy it to the Pi.

One-time setup on the PC (Arch Linux):

```sh
sudo pacman -S docker-buildx qemu-user-static qemu-user-static-binfmt
sudo systemctl restart systemd-binfmt
docker buildx ls        # linux/arm64 should be listed
```

The commands below assume `compose.yaml` names the image
(`image: stardate-api:latest` under the `api` service) and that `pi` is an SSH
host alias.

Build and ship:

```sh
docker buildx build --platform linux/arm64 -t stardate-api:latest --load ./backend
docker save stardate-api:latest | gzip | ssh pi 'gunzip | docker load'
scp compose.yaml pi:~/stardate/
ssh pi 'cd ~/stardate && docker compose up -d'
```

## Possible improvements

Ideas that are not needed now but are worth knowing about.

- **Batch insert weekday rows.** `schedules::insert_days` inserts one row per weekday in a loop.
  sqlx's `QueryBuilder::push_values` can build a single multi-row `INSERT ... VALUES (...), (...)`
  instead. Not worth it today: SQLite runs in-process (no network round trips) and the loop runs
  inside one transaction, so seven inserts are as fast as one. Reconsider if row counts grow.
  An empty list must be skipped, since `VALUES` without rows is invalid SQL.

- **Handle response serialization failures in `AppJson`.** `AppJson` delegates responses to
  `axum::Json`, which on a serialization error sends the serde error text as a plain-text 500 and
  logs nothing. Serializing in `AppJson` itself (`serde_json::to_vec`) would log the cause with
  `tracing` and return a generic `{"message": "Internal server error"}` via a new
  `AppError::Internal`; `Tagged` should then use `AppJson` too. Not urgent: the current response
  types cannot fail to serialize. It would matter for types such as maps with non-string keys, or
  dates outside years 0000–9999.

- **Clock-out in the same second a pause started.** Clocking out at the same second as the
  running pause's start answers 400 "a pause lies after the clock-out time". `clock_out` only
  ends a running pause when `started_at < at`, so the pause stays open and the following
  "outside the day" check rejects it. Rare by hand, but easy to hit from a script or a double
  tap. Possible fixes: delete a running pause that has no duration yet, or answer 409 with a
  clearer message.

- **Handle 401 in the frontend once authentication exists.** `web/CLAUDE.md` asks for a 401 to
  redirect to sign-in instead of showing a toast. There is no authentication or sign-in page yet,
  so today a 401 is shown like any other error. When authentication lands: in
  `web/src/lib/api/client.ts`, send a 401 to the sign-in page instead of the error handler, and add
  a client test for it.

- **Friendlier message for a duplicate job name.** Creating or renaming a job to a name that
  already exists answers 409 with the generic "resource already exists" (`AppError` maps every
  unique violation to it). `jobs.rs` could check for that case and answer "a job with this name
  already exists" instead, so the toast says what to change.

## Roadmap

- [x] Data model: jobs, versioned schedules, work days, pauses, flex adjustments (UTC instants)
- [x] Replace the `pizza` placeholder and its migration
- [x] Jobs API with optimistic concurrency
- [x] JSON error bodies for rejected requests (bad JSON, bad path values)
- [x] Schedules API
- [x] Delete archived jobs with all their data (`ON DELETE CASCADE` from `jobs`)
- [x] Calculation module (worked time, balance, leave-at)
- [x] Days API
- [x] Clock-in/out and pauses
- [x] Summary, flex balance, flex adjustments and status
- [ ] Tests
- [ ] Svelte frontend, served by axum
    - [x] Scaffold, API client with error toasts, app shell with theme settings
    - [x] Workday screen: status per job, live actions
    - [x] Back-dated entries (date and time pickers)
    - [ ] History and edits
    - [x] Jobs: create, rename, archive, delete
    - [x] Schedules per job
    - [ ] Served by axum from the same container
- [ ] Authentication
- [ ] Backups (`VACUUM INTO` + off-device copy) and a `deploy.sh`
- [ ] CI (fmt, clippy, test, arm64 image build)

