# Stardate

A small self-hosted web app for tracking worked time: clock in and out, record
lunch and breaks, compare against a work schedule (e.g. 9 h/day with 1 h lunch
and 30 min breaks), and register time for past days. Supports multiple jobs.

Runs as a single Docker container on a Raspberry Pi, so the footprint is kept
small: one Rust binary with SQLite compiled in.

> Learning project: Rust (backend) and Svelte (frontend).

## Status

| Area                                                                                              | State                                                                     |
| ------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------- |
| Backend infrastructure (config, DB, migrations, logging, errors, health check, graceful shutdown) | Done                                                                      |
| Docker image + Compose                                                                            | Done                                                                      |
| Data model (jobs, schedules, time entries)                                                        | Next. `pizza` is a placeholder resource used to set up the infrastructure |
| Frontend (Svelte)                                                                                 | Not started                                                               |
| Tests                                                                                             | Later                                                                     |
| Authentication                                                                                    | Later. **Do not expose beyond the LAN until then**                        |

## Tech stack

- **Backend:** Rust, axum, sqlx (SQLite), tokio, tracing, anyhow
- **Database:** SQLite (bundled into the binary, file stored in a Docker volume)
- **Frontend:** Svelte + Vite (planned)
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
│       ├── health.rs        # GET /api/health
│       └── pizza.rs         # placeholder CRUD resource (to be replaced)
├── compose.yaml
└── README.md
```

Planned additions: `models/` and `routes/` for jobs, schedules and entries;
`auth.rs`; a `frontend/` folder whose build is served by axum from the same
container.

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

| Method            | Path               | Description                             |
| ----------------- | ------------------ | --------------------------------------- |
| GET               | `/api/health`      | 200 if the app and database respond     |
| GET, POST, DELETE | `/api/pizzas`      | Placeholder: list / create / delete all |
| GET, PUT, DELETE  | `/api/pizzas/{id}` | Placeholder: get / update / delete one  |

Errors are returned as JSON `{ "message": "..." }`:

| Status | When                                                             |
| ------ | ---------------------------------------------------------------- |
| 400    | Validation failed, or a CHECK / NOT NULL constraint was violated |
| 404    | Resource not found                                               |
| 409    | Unique or foreign key constraint violated                        |
| 503    | Database busy/locked or connection pool exhausted (retry later)  |
| 500    | Unexpected database error (details only in the server log)       |

## Deploy to the Raspberry Pi

Build an arm64 image on a PC and copy it to the Pi.

One-time setup on the PC (Arch Linux):

```sh
sudo pacman -S docker-buildx qemu-user-static qemu-user-static-binfmt
sudo systemctl restart systemd-binfmt
docker buildx ls        # linux/arm64 should be listed
```

Build and ship:

```sh
docker buildx build --platform linux/arm64 -t stardate-api:latest --load ./backend
docker save stardate-api:latest | gzip | ssh pi 'gunzip | docker load'
scp compose.yaml pi:~/stardate/
ssh pi 'cd ~/stardate && docker compose up -d'
```

## Roadmap

- [ ] Data model: jobs, versioned schedules, time entries (work/lunch/break), stored in UTC
- [ ] Replace the `pizza` placeholder and its migration
- [ ] Tests
- [ ] Svelte frontend, served by axum
- [ ] Authentication
- [ ] Backups (`VACUUM INTO` + off-device copy) and a `deploy.sh`
- [ ] CI (fmt, clippy, test, arm64 image build)
