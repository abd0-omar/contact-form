use std::{sync::Arc, time::Duration};

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
    email_client::EmailClient,
    routes::{health_check::health_check, subscriptions::subscribe},
};

pub struct AppState {
    pub pool: SqlitePool,
    pub email_client: EmailClient,
}

pub async fn run(
    listener: TcpListener,
    pool: SqlitePool,
    email_client: EmailClient,
) -> anyhow::Result<Serve<TcpListener, Router, Router>> {
    // Wrapped in an Arc pointer to allow cheap cloning of AppState across handlers.
    // This prevents unnecessary cloning of EmailClient, which has two String fields,
    // since cloning an Arc is negligible.
    let app_state = Arc::new(AppState { pool, email_client });

    let app = Router::new()
        .route("/", get(home_page))
        .route("/health_check", get(health_check))
        .route("/subscriptions", post(subscribe))
        .with_state(app_state)
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
    pub async fn build(configuration: Settings) -> anyhow::Result<Self> {
        let listener = TcpListener::bind(format!(
            "{}:{}",
            configuration.application.host, configuration.application.port
        ))
        .await?;
        let port = listener.local_addr()?.port();

        let pool = configure_database(&configuration.database).await?;

        let sender_email = configuration
            .email_client
            .sender()
            .expect("Invalid sender email address.");

        let timeout = configuration.email_client.timeout();
        let email_client = EmailClient::new(
            sender_email,
            configuration.email_client.base_url.clone(),
            configuration.email_client.authorization_token,
            timeout,
        );

        let server = run(listener, pool, email_client).await.unwrap();

        Ok(Self { server, port })
    }

    pub async fn run_until_stopped(self) -> anyhow::Result<()> {
        Ok(self.server.await?)
    }

    pub fn port(&self) -> u16 {
        self.port
    }
}

pub async fn home_page() -> impl axum::response::IntoResponse {
    "under constructions\nمنطقة عمل"
}
