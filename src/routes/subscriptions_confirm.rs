use axum::http::StatusCode;
use axum::{
    extract::{Query, State},
    response::IntoResponse,
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::startup::AppState;

#[derive(serde::Deserialize)]
pub struct Parameters {
    #[allow(dead_code)]
    subscription_token: String,
}

#[tracing::instrument(name = "Confirm a pending subscriber", skip(parameter, app_state))]
#[allow(unused_variables)]
pub async fn confirm(
    Query(parameter): Query<Parameters>,
    State(app_state): State<AppState>,
) -> impl IntoResponse {
    // parameter have the subscription_token
    // confirm?my_token
    // extract the token and get the subscription table through the subscription_id
    let id =
        match get_subscriber_id_from_token(&app_state.pool, &parameter.subscription_token).await {
            Ok(id) => id,
            Err(_) => return StatusCode::INTERNAL_SERVER_ERROR,
        };

    match id {
        // Non-existing token
        None => return StatusCode::UNAUTHORIZED,
        Some(subscriber_id) => {
            if confirm_subscriber(&app_state.pool, subscriber_id)
                .await
                .is_err()
            {
                return StatusCode::INTERNAL_SERVER_ERROR;
            }
            StatusCode::OK
        }
    }
}

#[tracing::instrument(
    name = "Get subscriber_id from token"
    skip(sbuscription_token, pool)
)]
pub async fn get_subscriber_id_from_token(
    pool: &PgPool,
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

    Ok(result.map(|r| r.subscriber_id))
}

#[tracing::instrument(name = "Mark subscriber as confirmed", skip(pool, subscriber_id))]
pub async fn confirm_subscriber(pool: &PgPool, subscriber_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"UPDATE subscriptions SET status = 'confirmed' WHERE id = $1"#,
        subscriber_id
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(())
}
