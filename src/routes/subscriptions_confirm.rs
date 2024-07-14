use axum::{extract::Query, response::IntoResponse};

#[derive(serde::Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

#[tracing::instrument(name = "Confirm a pending subscriber", skip(subscription_token))]
pub async fn confirm(Query(subscription_token): Query<Parameters>) -> impl IntoResponse {
    axum::http::StatusCode::OK
}
