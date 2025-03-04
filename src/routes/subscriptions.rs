use axum::{extract::State, response::IntoResponse, Form};
use reqwest::StatusCode;
use serde::Deserialize;
use sqlx::{types::chrono::Utc, SqlitePool};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct FormData {
    name: String,
    email: String,
}

pub async fn subscribe(
    State(pool): State<SqlitePool>,
    Form(form_data): Form<FormData>,
) -> impl IntoResponse {
    let id = Uuid::new_v4().to_string();
    let timestamptz = Utc::now().to_string();
    match sqlx::query!(
        r#"
            INSERT INTO subscriptions(id, name, email, subscribed_at) VALUES($1, $2, $3, $4)
            "#,
        id,
        form_data.name,
        form_data.email,
        timestamptz,
    )
    .execute(&pool)
    .await
    {
        Ok(_) => return StatusCode::OK,
        Err(_err) => return StatusCode::INTERNAL_SERVER_ERROR,
    };
}
