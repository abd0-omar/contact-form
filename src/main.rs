use axum::{routing::get, Router};

use crate::routes::{form::accept_form, index::index};
mod routes;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(index).post(accept_form))
        .nest_service(
            "/templates",
            tower_http::services::ServeFile::new(format!(
                "{}/templates/output.css",
                std::env::current_dir().unwrap().to_str().unwrap()
            )),
        );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8000")
        .await
        .unwrap();
    println!("listnening live on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap()
}
