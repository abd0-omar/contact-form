use anyhow::Context;
use axum::http::StatusCode;
use axum::{
    extract::{Query, State},
    response::IntoResponse,
};
use sqlx::SqlitePool;
use thiserror;
use uuid::Uuid;

use crate::startup::AppState;

use super::error_chain_fmt;

#[derive(serde::Deserialize)]
pub struct Parameters {
    #[allow(dead_code)]
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

// steps for implementing error
// IntoResponse status code, Dispaly, Debug error chain, error for source
impl IntoResponse for ConfirmationError {
    fn into_response(self) -> askama_axum::Response {
        match self {
            ConfirmationError::UnexpectedError(e) => {
                tracing::error!(error_chain = ?e);
                StatusCode::INTERNAL_SERVER_ERROR
            }
            ConfirmationError::UnknownToken => {
                tracing::error!("There is no subscriber associated with the provided token.",);
                StatusCode::UNAUTHORIZED
            }
        }
        .into_response()
    }
}

#[tracing::instrument(name = "Confirm a pending subscriber", skip(parameter, app_state))]
#[allow(unused_variables)]
pub async fn confirm(
    Query(parameter): Query<Parameters>,
    State(app_state): State<AppState>,
) -> Result<StatusCode, ConfirmationError> {
    // parameter have the subscription_token
    // confirm?my_token
    // extract the token and get the subscription table through the subscription_id
    let subscriber_id =
        get_subscriber_id_from_token(&app_state.pool, &parameter.subscription_token)
            .await
            .context("Failed to retrieve the subscriber id associated with the provided token.")?
            .ok_or(ConfirmationError::UnknownToken)?;

    confirm_subscriber(&app_state.pool, subscriber_id)
        .await
        .context("Failed to update the subscriber status to `confirmed`.")?;
    Ok(StatusCode::OK)
}

#[tracing::instrument(
    name = "Get subscriber_id from token"
    skip(sbuscription_token, pool)
)]
pub async fn get_subscriber_id_from_token(
    pool: &SqlitePool,
    sbuscription_token: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    let result = sqlx::query!(
        r#"SELECT subscriber_id FROM subscription_tokens WHERE subscription_token = $1"#,
        sbuscription_token
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;

    Ok(result.map(|r| Uuid::parse_str(&r.subscriber_id).unwrap()))
}

#[tracing::instrument(name = "Mark subscriber as confirmed", skip(pool, subscriber_id))]
pub async fn confirm_subscriber(pool: &SqlitePool, subscriber_id: Uuid) -> Result<(), sqlx::Error> {
    let subscriber_id_string = subscriber_id.to_string();
    sqlx::query!(
        r#"UPDATE subscriptions SET status = 'confirmed' WHERE id = $1"#,
        subscriber_id_string
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(())
}
