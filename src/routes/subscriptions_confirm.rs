use std::sync::Arc;

use anyhow::Context;
use axum::{
    extract::{Query, State},
    response::IntoResponse,
};
use reqwest::StatusCode;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::startup::AppState;

use super::error_chain_fmt;

#[derive(serde::Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

#[derive(thiserror::Error)]
pub enum ConfirmationError {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
    #[error("There is no subscriber associated with the provided token.")]
    UnknownToken,
}

impl std::fmt::Debug for ConfirmationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl IntoResponse for ConfirmationError {
    fn into_response(self) -> axum::response::Response {
        match self {
            Self::UnknownToken => {
                tracing::error!(cause_chain = ?self);
                StatusCode::UNAUTHORIZED
            }
            Self::UnexpectedError(e) => {
                tracing::error!(cause_chain = ?e);
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
        .into_response()
    }
}

// could later take only the pool from the state, if you want to do it check the
// axum's State docs
#[tracing::instrument(name = "Confirm a pending subscriber", skip(parameters, app_state))]
pub async fn confirm(
    State(app_state): State<Arc<AppState>>,
    Query(parameters): Query<Parameters>,
) -> Result<impl IntoResponse, ConfirmationError> {
    let subscriber_id =
        get_subscriber_id_from_token(&app_state.pool, &parameters.subscription_token)
            .await
            .context("Failed to retrieve the subscriber id associated with the provided token.")?
            .ok_or(ConfirmationError::UnknownToken)?;

    confirm_subscriber(&app_state.pool, subscriber_id)
        .await
        .context("Failed to update the subscriber status to `confirmed`.")?;

    Ok(StatusCode::OK)
}

#[tracing::instrument(name = "Mark subscriber as confirmed", skip(subscriber_id, pool))]
pub async fn confirm_subscriber(pool: &SqlitePool, subscriber_id: Uuid) -> Result<(), sqlx::Error> {
    let subscriber_id = subscriber_id.to_string();
    sqlx::query!(
        r#"UPDATE subscriptions SET status = 'confirmed' WHERE uuid = $1"#,
        subscriber_id,
    )
    .execute(pool)
    .await?;
    Ok(())
}

#[tracing::instrument(name = "Get subscriber_id from token", skip(subscription_token, pool))]
pub async fn get_subscriber_id_from_token(
    pool: &SqlitePool,
    subscription_token: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    let result = sqlx::query!(
        r#"SELECT subscriber_id FROM subscription_tokens WHERE subscription_token = $1"#,
        subscription_token,
    )
    .fetch_optional(pool)
    .await?;
    Ok(result.map(|r| Uuid::try_parse(&r.subscriber_id).unwrap()))
}
