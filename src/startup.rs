use crate::routes::{
    greet::greet, health_check::health_check, index::index, subscriptions::accept_form,
};
use axum::{
    routing::{get, post},
    Router,
};
use sqlx::PgPool;

pub fn build_router(pool: PgPool) -> Result<Router, Box<dyn std::error::Error>> {
    let app = Router::new()
        .route("/", get(index))
        .route("/subscriptions", post(accept_form))
        .route("/health_check", get(health_check))
        .route("/path", get(greet))
        .route("/path/:name", get(greet))
        .nest_service(
            "/templates",
            tower_http::services::ServeFile::new(format!(
                "{}/templates/output.css",
                std::env::current_dir()?.to_str().unwrap()
            )),
        )
        .with_state(pool);
    Ok(app)
}
