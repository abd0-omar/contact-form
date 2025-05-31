use std::sync::Arc;

use crate::authentication::UserId;
use crate::domain::SubscriberEmail;
use crate::startup::AppState;
use crate::utils::e500;
use anyhow::Context;
use axum::extract::State;
use axum::response::{IntoResponse, Redirect};
use axum::{Extension, Form};
use axum_messages::Messages;
use sqlx::SqlitePool;

#[derive(serde::Deserialize)]
pub struct FormData {
    title: String,
    text_content: String,
    html_content: String,
}

#[tracing::instrument(
    name = "Publish a newsletter issue",
    skip(form, app_state, messages, user_id),
    fields(user_id=%user_id),
)]
pub async fn publish_newsletter(
    State(app_state): State<Arc<AppState>>,
    messages: Messages,
    Extension(user_id): Extension<UserId>,
    Form(form): Form<FormData>,
) -> Result<axum::response::Response, axum::response::Response> {
    let subscribers = get_confirmed_subscribers(&app_state.pool)
        .await
        .map_err(e500)?;
    for subscriber in subscribers {
        match subscriber {
            Ok(subscriber) => {
                app_state
                    .email_client
                    .send_email(
                        &subscriber.email,
                        &form.title,
                        &form.html_content,
                        &form.text_content,
                    )
                    .await
                    .with_context(|| {
                        format!("Failed to send newsletter issue to {}", subscriber.email)
                    })
                    .map_err(e500)?;
            }
            Err(error) => {
                tracing::warn!(
                    error.cause_chain = ?error,
                    error.message = %error,
                    "Skipping a confirmed subscriber. Their stored contact details are invalid",
                );
            }
        }
    }
    messages.info("The newsletter issue has been published!");
    Ok(Redirect::to("/admin/newsletters").into_response())
}

struct ConfirmedSubscriber {
    email: SubscriberEmail,
}

#[tracing::instrument(name = "Get confirmed subscribers", skip(pool))]
async fn get_confirmed_subscribers(
    pool: &SqlitePool,
) -> Result<Vec<Result<ConfirmedSubscriber, anyhow::Error>>, anyhow::Error> {
    let confirmed_subscribers = sqlx::query!(
        r#"
        SELECT email
        FROM subscriptions
        WHERE status = 'confirmed'
        "#,
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r| match SubscriberEmail::parse(r.email) {
        Ok(email) => Ok(ConfirmedSubscriber { email }),
        Err(error) => Err(anyhow::anyhow!(error)),
    })
    .collect();
    Ok(confirmed_subscribers)
}
