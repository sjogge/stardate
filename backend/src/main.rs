mod pizza;
mod error;

use axum::Router;
use sqlx::SqlitePool;
use tokio::signal;

const DEFAULT_DATABASE_URL: &str = "sqlite:stardate.db?mode=rwc";
const DEFAULT_BIND_ADDR: &str = "0.0.0.0:3000";

#[tokio::main]
async fn main() {
    let pool = SqlitePool::connect(DEFAULT_DATABASE_URL)
        .await
        .unwrap();

    sqlx::migrate!().run(&pool).await.unwrap();   

    let app: Router = Router::new()
       .nest("/api/pizzas", pizza::routes())
       .with_state(pool.clone());

    // run our app with hyper, listening globally on port
    let listener = tokio::net::TcpListener::bind(DEFAULT_BIND_ADDR).await.unwrap();

    println!("listening on {}", listener.local_addr().unwrap());

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();

    pool.close().await;
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