mod db;
mod error;
mod health;
mod pizza;
mod config;

use anyhow::{Context};
use axum::{Router, routing::get};
use tokio::signal;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::Level;
use std::io::IsTerminal;
use tracing_subscriber::EnvFilter;

const DEFAULT_LOG_FILTER: &str = "stardate=info,tower_http=info";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    let config = config::Config::from_env()?;

    let pool = db::connect(&config.database_url)
        .await
        .with_context(|| format!("failed to open database {}", config.database_url))?;

    sqlx::migrate!()
        .run(&pool)
        .await
        .context("failed to run migrations")?;

    let app: Router = Router::new()
        .nest("/api/pizzas", pizza::routes())
        .route("/api/health", get(health::health_check))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        )
        .with_state(pool.clone());

    let listener = tokio::net::TcpListener::bind(&config.bind_addr)
        .await
        .with_context(|| format!("failed to bind {}", config.bind_addr))?;

    println!("listening on {}", listener.local_addr()?);
    tracing::info!(addr = %listener.local_addr()?, "listening");    

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .with_context(|| "failed to start server")?;

    pool.close().await;

    Ok(())
}

async fn shutdown_signal() {
    tracing::info!("shutting down");

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

fn init_tracing() {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(DEFAULT_LOG_FILTER));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_ansi(std::io::stdout().is_terminal())
        .init();
}