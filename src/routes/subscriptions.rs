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
) -> impl IntoResponse {
    let new_subscriber: NewSubscriber = match form.clone().try_into() {
        Ok(form) => form,
        Err(_) => {
            return (
                axum::http::StatusCode::UNPROCESSABLE_ENTITY,
                FormData {
                    name: form.name,
                    email: form.email,
                    error: Some(FormError::BadEmail),
                },
            )
                .into_response();
        }
    };

    let mut transaction = match app_state.pool.begin().await {
        Ok(transaction) => transaction,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    // returning a template
    let subscriber_id = match insert_subscriber(&mut transaction, new_subscriber.clone()).await {
        Ok(subscriber_id) => subscriber_id,
        Err(_) => {
            return (
                // email already in db or could be that query didn't make it
                axum::http::StatusCode::CONFLICT,
                FormData {
                    name: new_subscriber.name.into(),
                    email: new_subscriber.email.into(),
                    error: Some(FormError::ConflictOrQueryBlewUp),
                },
            )
                .into_response();
        }
    };

    let subscription_token = generate_subscription_token();
    if store_subscription_token(&mut transaction, subscriber_id, &subscription_token)
        .await
        .is_err()
    {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    if transaction.commit().await.is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    let email_client = app_state.email_client;

    if send_confirmation_email(
        &email_client,
        &new_subscriber,
        &app_state.base_url,
        &subscription_token,
    )
    .await
    .is_err()
    {
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            FormData {
                name: new_subscriber.name.into(),
                email: new_subscriber.email.into(),
                error: Some(FormError::ConflictOrQueryBlewUp),
            },
        )
            .into_response();
    }

    // status code by default would be 200 OK
    FormData {
        name: new_subscriber.name.into(),
        email: new_subscriber.email.into(),
        error: None,
    }
    .into_response()
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
        .send_email_mailgun(
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
) -> Result<(), sqlx::Error> {
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
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(())
}
