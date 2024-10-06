use anyhow::Context;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use askama_axum::IntoResponse;
use axum::{extract::State, http::HeaderMap, Json};
use base64::Engine;
use reqwest::StatusCode;
use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;
use sqlx::SqlitePool;

use crate::{domain::SubscriberEmail, startup::AppState, telemetry::spawn_blocking_with_tracing};

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

    let credentials = basic_authentication(&headers).map_err(PublishError::AuthorizationError)?;
    tracing::Span::current().record("username", tracing::field::display(&credentials.username));

    // error chain bubbling to the base fn, which is this one
    let user_id = match validate_credentials(credentials, &pool).await {
        Ok(user_id) => user_id,
        Err(error) => {
            tracing::error!(error_chain = ?error, "Error while validating credentials, possibly PHC format issue.");
            return Err(error);
        }
    };
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

#[derive(Clone)]
struct Credentials {
    username: String,
    password: Secret<String>,
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
    AuthorizationError(#[source] anyhow::Error),
}

impl IntoResponse for PublishError {
    fn into_response(self) -> askama_axum::Response {
        match self {
            PublishError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            PublishError::AuthorizationError(_) => {
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

#[tracing::instrument(
    name = "Verify password hash",
    skip(expected_password_hash, password_candidate)
)]
fn verify_password_hash(
    expected_password_hash: Secret<String>,
    password_candidate: Secret<String>,
) -> Result<(), PublishError> {
    let expected_password_hash = PasswordHash::new(expected_password_hash.expose_secret())
        .context("Failed to parse hash in PHC string format.")
        .map_err(PublishError::UnexpectedError)?;
    Argon2::default()
        .verify_password(
            password_candidate.expose_secret().as_bytes(),
            &expected_password_hash,
        )
        .context("Invalid password.")
        .map_err(PublishError::AuthorizationError)
}

#[tracing::instrument(name = "Validate credentials", skip(credentials, pool))]
async fn validate_credentials(
    credentials: Credentials,
    pool: &SqlitePool,
) -> Result<uuid::Uuid, PublishError> {
    let mut user_id = None;
    // prevent timing attacks
    let mut expected_password_hash = Secret::new(
        "$argon2id$v=19$m=15000,t=2,p=1$\
            gZiV/M1gPc22ElAH/Jh1Hw$\
            CWOrkoo7oJBQ/iyh7uJ0LO2aLEfrHwTWllSAxT0zRno"
            .to_string(),
    );

    if let Some((stored_user_id, stored_password_hash)) =
        get_stored_credentials(&credentials.username, &pool)
            .await
            .map_err(PublishError::UnexpectedError)?
    {
        user_id = Some(stored_user_id);
        expected_password_hash = stored_password_hash;
    }

    // This executes before spawning the new thread
    spawn_blocking_with_tracing(move || {
        // We then pass ownership to it into the closure
        // and explicitly executes all our computation
        // within its scope.
        verify_password_hash(expected_password_hash, credentials.password)
    })
    .await
    .context("Failed to spawn blocking task")
    .map_err(PublishError::UnexpectedError)??;

    user_id.ok_or_else(|| PublishError::AuthorizationError(anyhow::anyhow!("Unknown username.")))
}

#[tracing::instrument(name = "Get stored credentials", skip(username, pool))]
async fn get_stored_credentials(
    username: &str,
    pool: &SqlitePool,
) -> Result<Option<(uuid::Uuid, Secret<String>)>, anyhow::Error> {
    let row = sqlx::query!(
        r#"
        SELECT user_id, password_hash
        FROM users
        WHERE username = $1
        "#,
        username,
    )
    .fetch_optional(pool)
    .await
    .context("Failed to perform a query to retrieve stored credentials.")?
    .map(|row| {
        (
            // todo remove unwraps
            uuid::Uuid::parse_str(&row.user_id.unwrap())
                .context("Failed to convert user_id to uuid")
                .map_err(PublishError::UnexpectedError)
                .unwrap(),
            Secret::new(row.password_hash),
        )
    });
    Ok(row)
}

impl std::fmt::Debug for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
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
