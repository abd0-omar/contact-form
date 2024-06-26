mod routes;
use crate::routes::form::accept_form;
use crate::routes::index::index;
use axum::{
    http::StatusCode,
    routing::{get, post},
    Router,
};

pub fn build_router() -> Result<Router, Box<dyn std::error::Error>> {
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
        );
    Ok(app)
}

async fn health_check() -> impl axum::response::IntoResponse {
    StatusCode::OK
}

async fn greet(name: Option<axum::extract::Path<String>>) -> impl axum::response::IntoResponse {
    if let Some(n) = name {
        let n = n.0;
        format!("hola {}!", n)
    } else {
        String::from("hola mundoz!")
    }
}
