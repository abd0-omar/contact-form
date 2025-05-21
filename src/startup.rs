use std::{sync::Arc, time::Duration};

use axum::{
    extract::{FromRef, Request},
    response::Response,
    routing::{get, post},
    serve::Serve,
    Router,
};
use secrecy::SecretString;
use sqlx::SqlitePool;
use tokio::net::TcpListener;
use tower_http::{services::ServeDir, trace::TraceLayer};
use tracing::{info, info_span, Span};
use uuid::Uuid;

use crate::{
    configuration::{configure_database, Settings},
    email_client::EmailClient,
    routes::{
        confirm, health_check::health_check, home::home, login, login_form, publish_newsletter,
        subscribe_form, subscriptions::subscribe,
    },
};

pub struct AppState {
    pub pool: SqlitePool,
    pub email_client: EmailClient,
    pub base_url: ApplicationBaseUrl,
    pub hmac_secret: HmacSecret,
}

// substate
impl FromRef<Arc<AppState>> for HmacSecret {
    fn from_ref(input: &Arc<AppState>) -> Self {
        input.hmac_secret.clone()
    }
}

pub struct ApplicationBaseUrl(pub String);

pub async fn run(
    listener: TcpListener,
    pool: SqlitePool,
    email_client: EmailClient,
    base_url: String,
    hmac_secret: SecretString,
) -> anyhow::Result<Serve<TcpListener, Router, Router>> {
    // Wrapped in an Arc pointer to allow cheap cloning of AppState across handlers.
    // This prevents unnecessary cloning of EmailClient, which has two String fields,
    // since cloning an Arc is negligible.
    let app_state = Arc::new(AppState {
        pool,
        email_client,
        base_url: ApplicationBaseUrl(base_url),
        hmac_secret: HmacSecret(SecretString::from(hmac_secret)),
    });

    let app = Router::new()
        .route("/", get(home))
        .route("/login", get(login_form))
        .route("/login", post(login))
        .route("/health_check", get(health_check))
        .route("/subscriptions", post(subscribe))
        .route("/subscriptions", get(subscribe_form))
        .route("/subscriptions/confirm", get(confirm))
        .route("/newsletters", post(publish_newsletter))
        .fallback_service(ServeDir::new("frontend/dist"))
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
                })
                // By default `TraceLayer` will log 5xx responses but we're doing our specific
                // logging of errors so disable that
                .on_failure(()),
        )
        .with_state(app_state);

    Ok(axum::serve(listener, app))
}

#[derive(Clone)]
pub struct HmacSecret(pub SecretString);

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

        let server = run(
            listener,
            pool,
            email_client,
            configuration.application.base_url,
            configuration.application.hmac_secret,
        )
        .await
        .unwrap();

        Ok(Self { server, port })
    }

    pub async fn run_until_stopped(self) -> anyhow::Result<()> {
        Ok(self.server.await?)
    }

    pub fn port(&self) -> u16 {
        self.port
    }
}
