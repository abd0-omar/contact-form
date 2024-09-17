use anyhow::Context;
use askama::Template;
use askama_axum::IntoResponse;
use axum::{extract::State, Form};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use serde::Deserialize;

use axum::http::StatusCode;

use chrono::Utc;
use sqlx::{Executor, Sqlite, Transaction};
use uuid::Uuid;

use crate::{
    domain::{NewSubscriber, SubscriberEmail, SubscriberName},
    email_client::EmailClient,
    startup::AppState,
};

#[derive(Deserialize, Debug, Template, sqlx::FromRow, Clone)]
#[template(path = "succession.html")]
pub struct FormData {
    name: String,
    email: String,
    error: Option<FormError>,
}

#[derive(Deserialize, Clone, Debug)]
enum FormError {
    BadEmail,
    ConflictOrQueryBlewUp,
}

impl TryFrom<FormData> for NewSubscriber {
    type Error = String;

    fn try_from(value: FormData) -> Result<Self, Self::Error> {
        let name = SubscriberName::parse(value.name)?;
        let email = SubscriberEmail::parse(value.email)?;
        Ok(Self { email, name })
    }
}

#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(form, app_state),
    fields(
        subscriber_email = %form.email,
        subscriber_name = %form.name
    )
)]
pub async fn subscribe(
    State(app_state): State<AppState>,
    Form(form): Form<FormData>,
) -> Result<StatusCode, SubscribeError> {
    let new_subscriber: NewSubscriber = form
        .clone()
        .try_into()
        .map_err(SubscribeError::ValidationError)?;

    let mut transaction = app_state
        .pool
        .begin()
        .await
        .context("Failed to acquire a Sqlite connection from the pool")?;

    let subscriber_id = insert_subscriber(&mut transaction, new_subscriber.clone())
        .await
        .context("Failed to insert new subscriber in the database.")?;

    let subscription_token = generate_subscription_token();

    store_subscription_token(&mut transaction, subscriber_id, &subscription_token)
        .await
        .context("Failed to store the confirmation token for a new subscriber.")?;

    transaction
        .commit()
        .await
        .context("Failed to commit SQL transaction to store a new subscriber.")?;

    let email_client = app_state.email_client;

    send_confirmation_email(
        &email_client,
        &new_subscriber,
        &app_state.base_url,
        &subscription_token,
    )
    .await
    .context("Failed to send a confirmation email.")?;

    Ok(StatusCode::OK)
}

#[tracing::instrument(
    name = "Send a confirmation email to a new subscriber",
    skip(email_client, new_subscriber, base_url, subscription_token)
)]
pub async fn send_confirmation_email(
    email_client: &EmailClient,
    new_subscriber: &NewSubscriber,
    base_url: &str,
    subscription_token: &str,
) -> Result<(), reqwest::Error> {
    // base_url could be 127.0.0.1 for testing or our Digital Ocean domain name
    let confirmation_link = format!(
        "{}/subscriptions/confirm?subscription_token={}",
        base_url, subscription_token
    );

    let plain_body = format!(
        "Wilkommen zu wir Newzletter!\nVisit {} to confirm your subscription.",
        confirmation_link
    );
    let html_body = format!(
        "Wilkommen zu wir Newzletter!<br />Click <a href=\"{}\">here</a> to confirm your subscription.",
        confirmation_link
    );
    email_client
        .send_email_postmark(
            new_subscriber.clone().email,
            "Wilkommen!",
            &html_body,
            &plain_body,
        )
        .await
}

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(new_subscriber, transaction)
)]
pub async fn insert_subscriber(
    transaction: &mut Transaction<'_, Sqlite>,
    new_subscriber: NewSubscriber,
) -> Result<Uuid, sqlx::Error> {
    let subscriber_id = Uuid::new_v4();
    let subscriber_id_string = subscriber_id.to_string();
    let subscriber_name = new_subscriber.name.as_ref();
    let subscriber_email = new_subscriber.email.as_ref();
    let time_now = Utc::now();
    let query = sqlx::query!(
        r#"
    INSERT INTO subscriptions (id, email, name, subscribed_at, status)
    VALUES ($1, $2, $3, $4, 'pending_confirmation')
            "#,
        subscriber_id_string,
        subscriber_email,
        subscriber_name,
        time_now
    );
    transaction.execute(query).await.map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(subscriber_id)
}

fn generate_subscription_token() -> String {
    let mut rng = thread_rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}

#[tracing::instrument(
    name = "Store subscription token in the database",
    skip(subscription_token, transaction)
)]
async fn store_subscription_token(
    transaction: &mut Transaction<'_, Sqlite>,
    subscriber_id: Uuid,
    subscription_token: &str,
) -> Result<(), StoreTokenError> {
    let subscriber_id_string = subscriber_id.to_string();
    let query = sqlx::query!(
        r#"
    INSERT INTO subscription_tokens (subscription_token , subscriber_id)
    VALUES ($1, $2)
        "#,
        subscription_token,
        subscriber_id_string
    );
    transaction.execute(query).await.map_err(|e| {
        StoreTokenError(e)
        // tracing::error!("Failed to execute query: {:?}", e);
        // e
    })?;
    Ok(())
}

pub struct StoreTokenError(sqlx::Error);

impl std::fmt::Display for StoreTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "A database error was encountered while \
            trying to store a subscription token."
        )
    }
}

impl std::fmt::Debug for StoreTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl axum::response::IntoResponse for StoreTokenError {
    fn into_response(self) -> askama_axum::Response {
        // let sqlx_error = self.0;
        // let body = format!(
        //     "error when inserting a store token with sqlx error: {}",
        //     sqlx_error
        // );
        //
        // (StatusCode::INTERNAL_SERVER_ERROR, body).into_response()
        format!("{}", self).into_response()
    }
}

impl std::error::Error for StoreTokenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}

fn error_chain_fmt(
    e: &impl std::error::Error,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    writeln!(f, "{}\n", e)?;
    let mut current = e.source();
    while let Some(cause) = current {
        writeln!(f, "Caused by:\n\t{}", cause)?;
        current = cause.source();
    }
    Ok(())
}

#[derive(thiserror::Error)]
pub enum SubscribeError {
    #[error("{0}")]
    ValidationError(String),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for SubscribeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl IntoResponse for SubscribeError {
    fn into_response(self) -> askama_axum::Response {
        match self {
            SubscribeError::ValidationError(_) => StatusCode::BAD_REQUEST,
            SubscribeError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
        .into_response()
    }
}
