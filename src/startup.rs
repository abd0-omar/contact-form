use std::sync::Arc;

use crate::configuration::{DatabaseSettings, Settings};
use crate::email_client::EmailClient;
use crate::routes::greet::greet;
use crate::routes::subscriptions_confirm::confirm;
use crate::routes::{health_check::health_check, index::index, subscriptions::subscribe};
use crate::routes::{home, login, login_form, publish_newsletter};
use axum::serve::Serve;
use axum::{
    routing::{get, post},
    Router,
};
use secrecy::Secret;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::SqlitePool;
use tower_http::trace::TraceLayer;
use uuid::Uuid;

pub struct Application {
    port: u16,
    server: Serve<Router, Router>,
}

impl Application {
    pub async fn build(configuration: Settings) -> Result<Self, std::io::Error> {
        let pool = get_pool(&configuration.database);

        let sender_email = configuration
            .email_client
            .sender()
            .expect("Invalid sender email address.");
        let timeout = configuration.email_client.timeout();
        let email_client = EmailClient::new(
            configuration.email_client.base_url,
            sender_email,
            configuration.email_client.authorization_token,
            timeout,
        );

        let address = format!(
            "{}:{}",
            configuration.application.host, configuration.application.port
        );
        let listener = std::net::TcpListener::bind(address)?;
        let port = listener.local_addr().unwrap().port();
        let server = run(
            listener,
            pool,
            email_client,
            configuration.application.base_url,
            configuration.application.hmac_secret,
        )?;

        Ok(Self { port, server })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }
}

use axum_messages::MessagesManagerLayer;
use tower_sessions::{MemoryStore, SessionManagerLayer};

#[derive(Clone)]
pub struct HmacSecret(pub Secret<String>);

pub fn run(
    listener: std::net::TcpListener,
    pool: SqlitePool,
    email_client: EmailClient,
    base_url: String,
    hmac_secret: Secret<String>,
) -> Result<Serve<Router, Router>, std::io::Error> {
    // `session_store` and `session_layer` are used for flash messages
    let session_store = MemoryStore::default();
    // it could be put on the state using axum flash crate, but it's not maintained-ish
    // so we'll use the layering technique using axum messages along with tower sessions
    let session_layer = SessionManagerLayer::new(session_store);

    let email_client = Arc::new(email_client);
    let base_url = Arc::new(base_url);
    let app_state = AppState {
        pool,
        email_client,
        base_url,
        secret: HmacSecret(hmac_secret),
    };

    let app = Router::new()
        .route("/", get(index))
        .route("/subscriptions", post(subscribe))
        .route("/health_check", get(health_check))
        .route("/path", get(greet))
        .route("/path/:name", get(greet))
        .route("/subscriptions/confirm", get(confirm))
        .route("/newsletters", post(publish_newsletter))
        .route("/home", get(home))
        .route("/login", get(login_form))
        .route("/login", post(login))
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
        .layer(MessagesManagerLayer)
        // `actix_session` needs a key which will be used for signing the session cookies
        // https://docs.rs/actix-session/latest/actix_session/struct.SessionMiddleware.html#method.new
        // this is not the case for `tower_sessions` see https://github.com/maxcountryman/tower-sessions/discussions/100
        // > tower-sessions doesn't provide signing because no data is stored in the cookie.
        // > In other words, the cookie value is a pointer to the data stored server side.
        //
        // so no need for hmac like the actix-web example
        // let message_store = CookieMessageStore::builder(
        // Key::from(hmac_secret.expose_secret().as_bytes())
        // ).build();
        .layer(session_layer)
        .with_state(app_state);

    listener.set_nonblocking(true)?;
    let listener = tokio::net::TcpListener::from_std(listener)?;

    let server = axum::serve(listener, app);
    Ok(server)
}

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub email_client: Arc<EmailClient>,
    pub base_url: Arc<String>,
    pub secret: HmacSecret,
}

// we made this function so that build_router_and_listener could work on tests/helpers/spawn_app() fn with no problems
pub fn get_pool(configuration: &DatabaseSettings) -> SqlitePool {
    SqlitePoolOptions::new()
        .connect_lazy_with(configuration.connect_options_with_db_file_or_create_if_missing())
}
