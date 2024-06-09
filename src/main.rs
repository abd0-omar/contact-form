use std::time::Duration;

use axum::{routing::get, Router};
use sqlx::postgres::PgPoolOptions;

use crate::routes::{form::accept_form, index::index};
mod routes;

#[tokio::main]
async fn main() {
    let db_url = std::env::var("DATABASE_URL").expect("there must be a db url to work");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(3))
        .connect(&db_url)
        .await
        .expect("can't connect to db");

    let app = Router::new()
        .route("/", get(index).post(accept_form))
        .nest_service(
            "/templates",
            tower_http::services::ServeFile::new(format!(
                "{}/templates/output.css",
                std::env::current_dir().unwrap().to_str().unwrap()
            )),
        )
        .with_state(pool);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8000")
        .await
        .unwrap();
    println!("listnening live on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap()
}
