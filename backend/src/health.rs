use crate::error::AppError;
use axum::extract::State;
use sqlx::SqlitePool;

pub async fn health_check(State(pool): State<SqlitePool>) -> Result<(), AppError> {
    sqlx::query("SELECT 1").execute(&pool).await?;

    Ok(())
}
