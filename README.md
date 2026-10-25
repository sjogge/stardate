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
| API: jobs                                                                                         | Done                                               |
| API: schedules, days, clock-in/out, pauses, summary, flex, status                                 | In progress                                        |
| Frontend (Svelte)                                                                                 | Not started                                        |
| Tests                                                                                             | Later                                              |
| Authentication                                                                                    | Later. **Do not expose beyond the LAN until then** |

The domain model, rules and full API are specified in
[`docs/time-tracking.md`](docs/time-tracking.md).

## Tech stack

- **Backend:** Rust, axum, sqlx (SQLite), tokio, time, tracing, anyhow
- **Database:** SQLite (bundled into the binary, file stored in a Docker volume)
- **Frontend:** Svelte + Vite (planned), served as static files by axum
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
│   └── src/
│       ├── main.rs          # startup, router, logging, graceful shutdown
│       ├── config.rs        # configuration from environment variables
│       ├── db.rs            # SQLite connection pool and pragmas
│       ├── error.rs         # AppError -> HTTP response mapping
│       ├── auth.rs          # CurrentUser extractor (seeded user until auth exists)
│       ├── concurrency.rs   # If-Match extractor and ETag responses
│       ├── health.rs        # GET /api/health
│       └── jobs.rs          # jobs API
├── docs/
│   └── time-tracking.md     # domain and API specification
├── compose.yaml
├── CLAUDE.md                # instructions for Claude Code
└── README.md
```

Planned additions: modules for schedules, days, flex and status; a pure
calculation module; a `frontend/` folder whose build is served by axum from the
same container.

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

## Run with Docker

```sh
docker compose up --build -d
docker compose logs -f api
curl localhost:3000/api/health
```

Data lives in the `stardate-data` named volume and survives rebuilds.
`docker compose down -v` **deletes the database**.

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

| Method | Path                     | Description                                     |
| ------ | ------------------------ | ----------------------------------------------- |
| GET    | `/api/health`            | 200 if the app and database respond             |
| GET    | `/api/jobs`              | List jobs, active first, then archived          |
| POST   | `/api/jobs`              | Create a job: `{ "name": "..." }`               |
| GET    | `/api/jobs/{id}`         | Get a job                                       |
| PUT    | `/api/jobs/{id}`         | Rename a job (requires `If-Match`)              |
| POST   | `/api/jobs/{id}/archive` | Archive a job, keeping its history (idempotent) |

### Planned

Schedules, days, clock-in/out, pauses, summary, flex balance and adjustments, and
status. See the API section of [`docs/time-tracking.md`](docs/time-tracking.md).

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
| 409    | Unique or foreign key constraint violated (e.g. duplicate job name)                                                             |
| 412    | `If-Match` version is out of date: the resource was modified by another request                                                 |
| 413    | Request body too large                                                                                                          |
| 415    | `Content-Type: application/json` missing on a request with a JSON body                                                          |
| 422    | JSON is valid but has the wrong shape (e.g. a missing or mistyped field)                                                        |
| 428    | `If-Match` header is missing on an update that requires it                                                                      |
| 500    | Unexpected database error (details only in the server log)                                                                      |
| 503    | Database connection pool exhausted (retry later)                                                                                |

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

## Roadmap

- [x] Data model: jobs, versioned schedules, work days, pauses, flex adjustments (UTC instants)
- [x] Replace the `pizza` placeholder and its migration
- [x] Jobs API with optimistic concurrency
- [x] JSON error bodies for rejected requests (bad JSON, bad path values)
- [ ] Schedules API
- [ ] Calculation module (worked time, balance, leave-at)
- [ ] Days API, clock-in/out and pauses
- [ ] Summary, flex balance, flex adjustments and status
- [ ] Tests
- [ ] Svelte frontend, served by axum
- [ ] Authentication
- [ ] Backups (`VACUUM INTO` + off-device copy) and a `deploy.sh`
- [ ] CI (fmt, clippy, test, arm64 image build)
