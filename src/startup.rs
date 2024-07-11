use std::sync::Arc;

use crate::email_client::EmailClient;
use crate::routes::{
    greet::greet, health_check::health_check, index::index, subscriptions::subscribe,
};
use axum::{
    routing::{get, post},
    Router,
};
use sqlx::PgPool;
use tower_http::trace::TraceLayer;
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub email_client: Arc<EmailClient>,
}

pub fn build_router(
    pool: PgPool,
    email_client: EmailClient,
) -> Result<Router, Box<dyn std::error::Error>> {
    let email_client = Arc::new(email_client);
    let app_state = AppState { pool, email_client };

    let app = Router::new()
        .route("/", get(index))
        .route("/subscriptions", post(subscribe))
        .route("/health_check", get(health_check))
        .route("/path", get(greet))
        .route("/path/:name", get(greet))
        .layer(TraceLayer::new_for_http().make_span_with(
            |request: &axum::http::Request<axum::body::Body>| {
                let request_id = Uuid::new_v4();

                tracing::span!(
                    tracing::Level::DEBUG,
                    "request",
                    method = tracing::field::display(request.method()),
                    uri = tracing::field::display(request.uri()),
                    version = tracing::field::debug(request.version()),
                    request_id = tracing::field::display(request_id)
                )
            },
        ))
        .nest_service(
            "/templates",
            tower_http::services::ServeFile::new(format!(
                "{}/templates/output.css",
                std::env::current_dir()?.to_str().unwrap()
            )),
        )
        .with_state(app_state);
    Ok(app)
}
