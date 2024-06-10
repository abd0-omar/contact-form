mod routes;
use crate::routes::form::accept_form;
use crate::routes::index::index;
use axum::{http::StatusCode, routing::get, serve::Serve, Router};

pub struct Application {
    server: Serve<Router, Router>,
}

impl Application {
    pub async fn run() -> Result<Self, Box<dyn std::error::Error>> {
        let app = Router::new()
            .route("/", get(index).post(accept_form))
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
        let listener = tokio::net::TcpListener::bind("127.0.0.1:8000").await?;
        println!("listnening live on {}", listener.local_addr().unwrap());
        let server = axum::serve(listener, app);
        Ok(Application { server })
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }
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
