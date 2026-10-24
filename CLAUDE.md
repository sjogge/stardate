# Stardate

Personal time-tracking web app: clock in/out, lunch and breaks, per-job schedules and a flex balance.
Runs as a single Docker container on a Raspberry Pi, so keep resource use and dependencies minimal.

The domain model and API are specified in `docs/time-tracking.md`. Read it before working on
anything time-tracking related; it is the source of truth for rules and calculations.

## Stack

- Backend: Rust (edition 2024), Axum 0.8, Tokio, sqlx 0.9 with SQLite, `tracing`, `anyhow` in `main`
- Time: the `time` crate (`serde`, `formatting`, `parsing` features). No timezone database on the server.
- Frontend (not created yet): Svelte + Vite SPA, served as static files by Axum later. The SPA stays
  "dumb": all calculations happen in the backend.
- Deployment: multi-stage `backend/Dockerfile` (Alpine, non-root UID 10001), `compose.yaml` with a
  hardened service and the database on a volume at `/data`.

## Layout

```
backend/
  build.rs                 # rerun build when migrations change
  migrations/              # SQL migrations, embedded with sqlx::migrate!() and run at startup
  src/
    main.rs                # startup, router, graceful shutdown
    config.rs              # env config: DATABASE_URL (required), BIND_ADDR, RUST_LOG
    db.rs                  # pool: WAL, synchronous=NORMAL, foreign_keys, busy_timeout
    error.rs               # AppError -> HTTP status + {"message": ...}
    health.rs              # GET /api/health
compose.yaml
docs/time-tracking.md      # domain spec
```

Follow the existing pattern: one module per resource with a `routes()` function nested under `/api`,
row structs with `FromRow`, separate request/response DTOs.

## Commands

Run from `backend/`:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo build
DATABASE_URL=sqlite:stardate.db cargo run
```

From the repo root: `docker compose up -d --build`.

Before considering work done: fmt, clippy (no warnings) and build must pass.

## Conventions

- JSON: `camelCase` (`#[serde(rename_all = "camelCase")]`), all routes under `/api`.
- Errors: handlers return `Result<_, AppError>`. No `unwrap`/`expect` in request paths.
  Client-facing messages are generic; details go to the log via `tracing` with `%e`.
- SQL: runtime-checked `sqlx::query`/`query_as` with bind parameters. Do not use the `query!`
  macros (they need a database at build time, which breaks the Docker build).
- Migrations: never edit an applied migration; add a new numbered file. Tables use `STRICT`.
- Request tracing: `.layer(TraceLayer)` only wraps routes added before it. Add new API routes
  above the layer; `/api/health` stays below it to avoid log noise.
- Keep calculation logic in a pure module (no I/O, no database) so it is easy to test later.
- Tests are deferred by decision; do not add test infrastructure unless asked.
- Authentication is deferred. Use the seeded user (id 1) and scope all data through
  `jobs.user_id` so authentication only has to supply the user id later.

## Working rules

- Read existing code and follow its conventions before changing anything.
- Comments explain why, not what. No change-log comments. Remove dead code instead of commenting it out.
- Never hardcode secrets; never log sensitive data.
- Keep dependencies minimal, maintained and pinned via `Cargo.lock` (builds use `--locked`).
- Commits: imperative subject of at most 72 characters, body explains why, one logical change per commit.
- Ask before destructive or shared-impact actions: pushing, force-pushing, deleting branches or files,
  dropping data. Never use `--no-verify`.

