use std::sync::Arc;

use crate::authentication::validate_credentials;
use crate::authentication::AuthError;
use crate::authentication::Credentials;
use crate::domain::SubscriberEmail;
use crate::routes::error_chain_fmt;
use crate::startup::AppState;
use anyhow::Context;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;
use base64::Engine;
use reqwest::StatusCode;
use secrecy::SecretString;
use sqlx::SqlitePool;

#[derive(serde::Deserialize)]
pub struct BodyData {
    title: String,
    content: Content,
}

#[derive(serde::Deserialize)]
pub struct Content {
    html: String,
    text: String,
}

#[derive(thiserror::Error)]
pub enum PublishError {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
}

impl std::fmt::Debug for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl IntoResponse for PublishError {
    fn into_response(self) -> axum::response::Response {
        match self {
            PublishError::UnexpectedError(e) => {
                tracing::error!(cause_chain = ?e);
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
            PublishError::AuthError(e) => {
                tracing::error!(cause_chain = ?e);
                (
                    StatusCode::UNAUTHORIZED,
                    [(
                        axum::http::header::WWW_AUTHENTICATE,
                        "Basic realm=\"publish\"",
                    )],
                )
                    .into_response()
                // other way of doing it
                // axum::http::Response::builder()
                // .status(StatusCode::UNAUTHORIZED)
                // .header(
                //     axum::http::header::WWW_AUTHENTICATE,
                //     "Basic realm=\"publish\"",
                // )
                // .body(Body::empty())
                // .unwrap()
            }
        }
    }
}

#[tracing::instrument(name = "Publish newsletter",
 skip(headers, body, app_state),
 fields(username=tracing::field::Empty, user_uuid=tracing::field::Empty))]
pub async fn publish_newsletter(
    headers: axum::http::HeaderMap,
    State(app_state): State<Arc<AppState>>,
    Json(body): Json<BodyData>,
) -> Result<impl IntoResponse, PublishError> {
    let credentials = basic_authentication(&headers).map_err(PublishError::AuthError)?;

    tracing::Span::current().record("username", &tracing::field::display(&credentials.username));
    let user_uuid = validate_credentials(credentials, &app_state.pool)
        .await
        .map_err(|e| match e {
            AuthError::InvalidCredentials(_) => PublishError::AuthError(e.into()),
            AuthError::UnexpectedError(_) => PublishError::UnexpectedError(e.into()),
        })?;
    tracing::Span::current().record("user_uuid", &tracing::field::display(&user_uuid));

    let subscribers = get_confirmed_subscribers(&app_state.pool).await?;
    for subscriber in subscribers {
        match subscriber {
            Ok(subscriber) => {
                app_state
                    .email_client
                    .send_email(
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
    Ok(StatusCode::OK)
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

pub fn basic_authentication(headers: &axum::http::HeaderMap) -> Result<Credentials, anyhow::Error> {
    let auth_header = headers
        .get(axum::http::header::AUTHORIZATION)
        .context("The 'Authorization' header was missing")?
        .to_str()
        .context("The 'Authorization' header was not a valid UTF8 string.")?;
    let base64encoded_segment = auth_header
        .strip_prefix("Basic ")
        .context("The authorization scheme was not 'Basic'.")?;
    let decoded_bytes = base64::engine::general_purpose::STANDARD
        .decode(base64encoded_segment)
        .context("failed to base64-decode 'Basic' credentials")?;
    let decoded_credentials = String::from_utf8(decoded_bytes)
        .context("The decoded credential string is not a valid UTF8.")?;

    let mut credentials = decoded_credentials.splitn(2, ':');
    let username = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("A username must be provided in 'Basic' auth."))?
        .to_string();
    let password = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("A password must be provided in 'Basic' auth."))?
        .to_string();
    Ok(Credentials {
        username,
        password: SecretString::from(password),
    })
}
