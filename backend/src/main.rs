mod pizza;
mod error;

use axum::Router;
use sqlx::SqlitePool;

#[tokio::main]
async fn main() {
    let pool = SqlitePool::connect("sqlite:stardate.db?mode=rwc")
        .await
        .unwrap();

    sqlx::migrate!().run(&pool).await.unwrap();   

    let app: Router = Router::new()
       .nest("/api/pizzas", pizza::routes())
       .with_state(pool);

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    println!("listening on {}", listener.local_addr().unwrap());

    axum::serve(listener, app).await.unwrap();
}
