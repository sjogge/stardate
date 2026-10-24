mod db;
mod error;
mod health;
mod pizza;

use anyhow::{Context, Ok};
use axum::{Router, routing::get};
use tokio::signal;

const DEFAULT_DATABASE_URL: &str = "sqlite:stardate.db";
const DEFAULT_BIND_ADDR: &str = "0.0.0.0:3000";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| DEFAULT_DATABASE_URL.to_string());
    let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| DEFAULT_BIND_ADDR.to_string());

    let pool = db::connect(&database_url)
        .await
        .with_context(|| format!("failed to open database {database_url}"))?;

    sqlx::migrate!()
        .run(&pool)
        .await
        .context("failed to run migrations")?;

    let app: Router = Router::new()
        .nest("/api/pizzas", pizza::routes())
        .route("/api/health", get(health::health_check))
        .with_state(pool.clone());

    // run our app with hyper, listening globally on port
    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .with_context(|| format!("failed to bind {bind_addr}"))?;

    println!("listening on {}", listener.local_addr().unwrap());

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap(); // TODO: ALso use anyhow?

    pool.close().await;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
/*
TODO: Improve error handling for startup steps.
Optionally, turn .unwrap() into .expect("failed to connect to database") and similar for each startup step.
 If the Pi fails to start, docker compose logs then tells you which step failed instead of only showing the error.
*/
