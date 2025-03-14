use axum::{extract::State, response::IntoResponse, Form};
use chrono::Utc;
use reqwest::StatusCode;
use serde::Deserialize;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::domain::{NewSubscriber, SubscriberEmail, SubscriberName};

#[derive(Deserialize)]
pub struct FormData {
    name: String,
    email: String,
}

impl TryFrom<FormData> for NewSubscriber {
    type Error = String;

    fn try_from(value: FormData) -> Result<Self, Self::Error> {
        let name = SubscriberName::parse(value.name)?;
        let email = SubscriberEmail::parse(value.email)?;
        Ok(Self { name, email })
    }
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
) -> Result<impl IntoResponse, SubscribeError> {
    // same as `NewSubscriber::try_from()`.
    let new_subscriber = form.try_into().map_err(SubscribeError::InvalidSubscriber)?;
    insert_subscriber(&pool, &new_subscriber).await?;

    Ok(StatusCode::OK)
}

#[derive(thiserror::Error, Debug)]
pub enum SubscribeError {
    #[error("invalid subscriber, {0}")]
    InvalidSubscriber(String),
    #[error("couldn't insert new_subscriber to the database, sqlx error {0}")]
    SqlxError(#[from] sqlx::Error),
}

impl IntoResponse for SubscribeError {
    fn into_response(self) -> axum::response::Response {
        match self {
            SubscribeError::SqlxError(e) => {
                tracing::error!("sqlx error: {:?}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            }
            SubscribeError::InvalidSubscriber(e) => {
                tracing::error!("Subscriber name or email error: {:?}", e);
                StatusCode::BAD_REQUEST
            }
        }
        .into_response()
    }
}

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(new_subscriber, pool)
)]
pub async fn insert_subscriber(
    pool: &SqlitePool,
    new_subscriber: &NewSubscriber,
) -> Result<(), sqlx::Error> {
    let id = Uuid::new_v4().to_string();
    let timestamptz = Utc::now().to_string();
    let name = new_subscriber.name.as_ref();
    let email = new_subscriber.email.as_ref();
    sqlx::query!(
        r#"
            INSERT INTO subscriptions(id, name, email, subscribed_at) VALUES($1, $2, $3, $4)
            "#,
        id,
        name,
        email,
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
