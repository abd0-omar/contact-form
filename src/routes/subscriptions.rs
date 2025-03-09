use axum::{extract::State, response::IntoResponse, Form};
use chrono::Utc;
use reqwest::StatusCode;
use serde::Deserialize;
use sqlx::SqlitePool;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct FormData {
    name: String,
    email: String,
}

#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(form, pool),
    fields(
        subscriber_name = %form.name,
        subscriber_email = %form.email
    )
)]
pub async fn subscribe(
    State(pool): State<SqlitePool>,
    Form(form): Form<FormData>,
) -> impl IntoResponse {
    match insert_subscriber(&pool, &form).await {
        Ok(_) => StatusCode::OK,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(form, pool)
)]
pub async fn insert_subscriber(pool: &SqlitePool, form: &FormData) -> Result<(), sqlx::Error> {
    let id = Uuid::new_v4().to_string();
    let timestamptz = Utc::now().to_string();
    sqlx::query!(
        r#"
            INSERT INTO subscriptions(id, name, email, subscribed_at) VALUES($1, $2, $3, $4)
            "#,
        id,
        form.name,
        form.email,
        timestamptz,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(())
}
