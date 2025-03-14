use std::time::Duration;

use axum::{
    extract::Request,
    response::Response,
    routing::{get, post},
    serve::Serve,
    Router,
};
use sqlx::SqlitePool;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing::{info, info_span, Span};
use uuid::Uuid;

use crate::{
    configuration::{configure_database, Settings},
    routes::{health_check::health_check, subscriptions::subscribe},
};

pub async fn run(
    listener: TcpListener,
    pool: SqlitePool,
) -> anyhow::Result<Serve<TcpListener, Router, Router>> {
    let app = Router::new()
        .route("/health_check", get(health_check))
        .route("/subscriptions", post(subscribe))
        .with_state(pool)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|request: &Request<_>| {
                    let request_id = Uuid::new_v4();
                    info_span!(
                        "http_request",
                        method = ?request.method(),
                        uri = ?request.uri(),
                        version = ?request.version(),
                        request_id = ?request_id,
                    )
                })
                .on_response(|response: &Response, latency: Duration, span: &Span| {
                    let status = response.status();
                    let headers = response.headers();
                    span.record("status", &status.as_u16());
                    info!(parent: span, ?status, ?headers, ?latency, "Response sent");
                }),
        );

    Ok(axum::serve(listener, app))
}

pub struct Application {
    port: u16,
    server: Serve<TcpListener, Router, Router>,
}

impl Application {
    // build is the one that invokes the `run()` function
    // then any fn invokes `run_until_stopped`
    pub async fn build(configuration: &Settings) -> anyhow::Result<Self> {
        let listener = TcpListener::bind(format!(
            "{}:{}",
            configuration.application.host, configuration.application.port
        ))
        .await?;
        let port = listener.local_addr()?.port();

        let pool = configure_database(&configuration.database).await?;

        let server = run(listener, pool).await.unwrap();

        Ok(Self { server, port })
    }

    pub async fn run_until_stopped(self) -> anyhow::Result<()> {
        Ok(self.server.await?)
    }

    pub fn port(&self) -> u16 {
        self.port
    }
}
