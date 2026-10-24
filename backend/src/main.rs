mod pizza;
mod error;

use axum::Router;
use sqlx::SqlitePool;
use tokio::signal;

#[tokio::main]
async fn main() {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite:stardate.db?mode=rwc".to_string());

    let bind_addr = std::env::var("BIND_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:3000".to_string());

    let pool = SqlitePool::connect(&database_url)
        .await
        .unwrap();

    sqlx::migrate!().run(&pool).await.unwrap();   

    let app: Router = Router::new()
       .nest("/api/pizzas", pizza::routes())
       .with_state(pool);

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind(&bind_addr).await.unwrap();

    println!("listening on {}", listener.local_addr().unwrap());

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();

    // TODO: How to close the pool? https://docs.rs/sqlx/latest/sqlx/struct.Pool.html?utm_source=chatgpt.com#method.close
    // pool.close().await;
}

async fn shutdown_signal() {
    // TODO: Is this correct?
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