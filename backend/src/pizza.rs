use crate::error::AppError;
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::get,
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use sqlx::prelude::FromRow;

#[derive(Serialize, FromRow)]
pub struct Pizza {
    pub id: i64,
    pub name: String,
    pub price: f64,
}

#[derive(Deserialize)]
pub struct NewPizza {
    pub name: String,
    pub price: f64,
}

impl NewPizza {
    pub fn validate(&self) -> Result<(), &'static str> {
        let name = self.name.trim();
        if name.is_empty() {
            return Err("name must not be empty");
        }
        if name.chars().count() > 50 {
            return Err("name must be at most 50 characters");
        }
        if !(0.0..=500.0).contains(&self.price) {
            return Err("price must be between 0 and 500");
        }
        Ok(())
    }
}

pub fn routes() -> Router<SqlitePool> {
    Router::new()
        .route(
            "/",
            get(get_pizzas).post(create_pizza).delete(delete_pizzas),
        )
        .route(
            "/{id}",
            get(get_pizza).delete(delete_pizza).put(update_pizza),
        )
}

async fn get_pizza(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
) -> Result<Json<Pizza>, AppError> {
    sqlx::query_as::<_, Pizza>("SELECT id, name, price FROM pizzas WHERE id = ?")
        .bind(id)
        .fetch_optional(&pool)
        .await?
        .map(Json)
        .ok_or(AppError::NotFound)
}

async fn delete_pizza(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    let result = sqlx::query("DELETE FROM pizzas WHERE id = ?")
        .bind(id)
        .execute(&pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn get_pizzas(State(pool): State<SqlitePool>) -> Result<Json<Vec<Pizza>>, AppError> {
    let pizzas = sqlx::query_as::<_, Pizza>("SELECT id, name, price FROM pizzas")
        .fetch_all(&pool)
        .await?;

    Ok(Json(pizzas))
}

async fn delete_pizzas(State(pool): State<SqlitePool>) -> Result<StatusCode, AppError> {
    sqlx::query("DELETE FROM pizzas").execute(&pool).await?;

    Ok(StatusCode::NO_CONTENT)
}

async fn create_pizza(
    State(pool): State<SqlitePool>,
    Json(new): Json<NewPizza>,
) -> Result<(StatusCode, Json<Pizza>), AppError> {
    new.validate()
        .map_err(|msg| AppError::Validation(msg.to_string()))?;

    let pizza = sqlx::query_as::<_, Pizza>(
        "INSERT INTO pizzas (name, price) VALUES (?, ?) RETURNING id, name, price",
    )
    .bind(new.name.trim())
    .bind(new.price)
    .fetch_one(&pool)
    .await?;

    Ok((StatusCode::CREATED, Json(pizza)))
}

async fn update_pizza(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
    Json(new): Json<NewPizza>,
) -> Result<Json<Pizza>, AppError> {
    new.validate()
        .map_err(|msg| AppError::Validation(msg.to_string()))?;

    sqlx::query_as::<_, Pizza>(
        "UPDATE pizzas SET name = ?, price = ? WHERE id = ? RETURNING id, name, price",
    )
    .bind(new.name.trim())
    .bind(new.price)
    .bind(id)
    .fetch_optional(&pool)
    .await?
    .map(Json)
    .ok_or(AppError::NotFound)
}
