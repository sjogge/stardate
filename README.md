stardate/
├── backend/
│   ├── Cargo.toml
│   ├── migrations/          # SQL files (0001_init.sql, ...)
│   └── src/
│       ├── main.rs          # startup, router assembly, static file serving
│       ├── config.rs        # env vars (port, db path, static dir)
│       ├── db.rs            # pool setup, migrations
│       ├── error.rs         # AppError + IntoResponse
│       ├── state.rs         # AppState (db pool, etc.)
│       ├── auth.rs          # password hashing, session/extractor
│       ├── models/          # structs mapping to tables + DTOs
│       │   ├── mod.rs
│       │   ├── job.rs
│       │   ├── schedule.rs
│       │   └── entry.rs
│       └── routes/          # one file per resource
│           ├── mod.rs       # builds the /api router
│           ├── auth.rs
│           ├── jobs.rs
│           └── entries.rs
├── frontend/
│   ├── package.json
│   ├── vite.config.ts
│   ├── svelte.config.js
│   ├── index.html
│   └── src/
│       ├── main.ts
│       ├── App.svelte
│       ├── lib/
│       │   ├── api.ts       # typed fetch wrapper
│       │   ├── time.ts      # worked-time calculations
│       │   └── components/
│       └── routes/          # pages (Today, History, Jobs, Settings)
├── Dockerfile               # multi-stage: frontend -> backend -> runtime
├── .dockerignore
├── docker-compose.yml       # volume for the SQLite file, port mapping
├── .gitignore
└── README.md
