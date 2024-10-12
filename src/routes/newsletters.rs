use crate::authenticaiton::validate_credentials;
use crate::authenticaiton::Credentials;
use anyhow::Context;
use askama_axum::IntoResponse;
use axum::{extract::State, http::HeaderMap, Json};
use base64::Engine;
use reqwest::StatusCode;
use secrecy::Secret;
use serde::Deserialize;
use sqlx::SqlitePool;

use crate::{authenticaiton::AuthError, domain::SubscriberEmail, startup::AppState};

use super::error_chain_fmt;

#[tracing::instrument(
 name = "Publishanewsletterissue",
 skip(headers, app_state, body),
 fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
 )]
pub async fn publish_newsletter(
    headers: HeaderMap,
    State(app_state): State<AppState>,
    Json(body): Json<BodyData>,
) -> Result<impl IntoResponse, PublishError> {
    let pool = app_state.pool;
    let email_client = app_state.email_client;

    let credentials = basic_authentication(&headers).map_err(PublishError::AuthError)?;
    tracing::Span::current().record("username", tracing::field::display(&credentials.username));

    let user_id = validate_credentials(credentials, &pool)
        .await
        // We match on `AuthError`'s variants, but we pass the **whole** error
        // into the constructors for `PublishError` variants. This ensures that
        // the context of the top-level wrapper is preserved when the error is
        // logged by our middleware.
        .map_err(|e| match e {
            AuthError::InvalidCredentials(_) => PublishError::AuthError(e.into()),
            AuthError::UnexpectedError(_) => PublishError::UnexpectedError(e.into()),
        })?;

    tracing::Span::current().record("user_id", &tracing::field::display(&user_id));

    let subscribers = get_confirmed_subscribers(&pool).await?;
    for subscriber in subscribers {
        match subscriber {
            Ok(subscriber) => {
                email_client
                    .send_email_postmark(
                        &subscriber.email,
                        &body.title,
                        &body.content.html,
                        &body.content.text,
                    )
                    .await
                    .with_context(|| {
                        format!("Failed to send newsletter issue to {}", subscriber.email)
                    })?;
            }
            Err(error) => {
                tracing::warn!(
                    error.cause_chain = ?error,
                    "Skipping a confirmed subscriber. \
                    Their stored contact details are invalid",
                );
            }
        }
    }

    Ok(StatusCode::OK.into_response())
}

fn basic_authentication(headers: &HeaderMap) -> Result<Credentials, anyhow::Error> {
    let header_value = headers
        .get("Authorization")
        .context("The Authorization header was missing")?
        .to_str()
        .context("The Authorization was not a valid UTF-8")?;
    let base64encoded_segment = header_value
        .strip_prefix("Basic ")
        .context("The authorization scheme was not 'Basic'")?;

    let decoded_bytes = base64::engine::general_purpose::STANDARD
        .decode(base64encoded_segment)
        .context("Failed tobase64-decode'Basic'credentials.")?;

    let decoded_credentials = String::from_utf8(decoded_bytes)
        .context("The decoded credential string is not valid UTF8.")?;
    let mut credentials = decoded_credentials.splitn(2, ':');
    let username = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("Ausernamemustbeprovidedin'Basic'auth."))?
        .to_string();
    let password = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("Apasswordmustbeprovidedin'Basic'auth."))?
        .to_string();
    Ok(Credentials {
        username,
        password: Secret::new(password),
    })
}

#[derive(thiserror::Error)]
pub enum PublishError {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
}

impl IntoResponse for PublishError {
    fn into_response(self) -> askama_axum::Response {
        match &self {
            PublishError::UnexpectedError(_e) => {
                tracing::error!(error_chain = ?self);
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
            PublishError::AuthError(e) => {
                tracing::error!(error_chain = ?e);
                // make a 401 response with header
                // WWW-Authenticate: Basic realm="publish"
                // it's called a "Challenge"

                let mut header_map = HeaderMap::new();
                header_map.insert(
                    "WWW-Authenticate",
                    "Basic realm=\"publish\"".parse().unwrap(),
                );

                (StatusCode::UNAUTHORIZED, header_map).into_response()
            }
        }
        .into_response()
    }
}

impl std::fmt::Debug for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

#[derive(Deserialize)]
pub struct BodyData {
    title: String,
    content: Content,
}

#[derive(Deserialize)]
pub struct Content {
    html: String,
    text: String,
}

pub struct ConfirmedSubscriber {
    email: SubscriberEmail,
}

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
