use std::{fs, sync::LazyLock};

use newzletter::{
    configuration::{configure_database, get_configuration},
    startup::Application,
    telemetry::{get_subscriber, init_subscriber},
};
use serde::Serialize;
use sqlx::sqlite::SqlitePool;
use tokio::fs::remove_file;
use uuid::Uuid;
use wiremock::MockServer;
// Ensure that the `tracing` stack is only initialised once using `once_cell`
static TRACING: LazyLock<()> = LazyLock::new(|| {
    let default_filter_level = "info".to_string();
    let subscriber_name = "test".to_string();
    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::stdout);
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::sink);
        init_subscriber(subscriber);
    };
});

pub struct TestApp {
    pub address: String,
    pub port: u16,
    pub db_pool: SqlitePool,
    pub email_server: MockServer,
    // to later delete it
    pub db_path: String,
}

#[derive(Serialize)]
pub struct FormData {
    pub name: Option<String>,
    pub email: Option<String>,
}

pub struct ConfirmationLinks {
    pub html: reqwest::Url,
    pub plain_text: reqwest::Url,
}

impl TestApp {
    pub async fn post_subscriptions(&self, form_data: &FormData) -> reqwest::Response {
        reqwest::Client::new()
            .post(&format!("{}/subscriptions", &self.address))
            .form(form_data)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    /// Extract the confirmation links embedded in the request to the email API.
    pub fn get_confirmation_links(&self, email_request: &wiremock::Request) -> ConfirmationLinks {
        let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();

        // Extract the link from one of the request fields.
        let get_link = |s: &str| {
            let links: Vec<_> = linkify::LinkFinder::new()
                .links(s)
                .filter(|l| *l.kind() == linkify::LinkKind::Url)
                .collect();
            assert_eq!(links.len(), 1);
            let raw_link = links[0].as_str().to_owned();
            let mut confirmation_link = reqwest::Url::parse(&raw_link).unwrap();
            // Let's make sure we don't call random APIs on the web
            assert_eq!(confirmation_link.host_str().unwrap(), "127.0.0.1");
            confirmation_link.set_port(Some(self.port)).unwrap();
            confirmation_link
        };

        let html = get_link(body["HtmlBody"].as_str().unwrap());
        let plain_text = get_link(body["TextBody"].as_str().unwrap());
        ConfirmationLinks { html, plain_text }
    }

    pub async fn cleanup_test_db(&self) -> Result<(), sqlx::Error> {
        remove_file(&format!("{}.db", self.db_path)).await?;
        Ok(())
    }
}

pub async fn spawn_app() -> anyhow::Result<TestApp> {
    // The first time `initialize` is invoked the code in `TRACING` is executed.
    // All other invocations will instead skip execution.
    LazyLock::force(&TRACING);

    fs::create_dir_all("scripts/a_place_for_test_dbs_to_spawn_in_it,supposed_to_be_empty_cuz_tests_terminate_after_success_execution/")?;

    let email_server = MockServer::start().await;

    let configuration = {
        let mut configuration = get_configuration().expect("Failed to read configuration");
        configuration.application.port = 0;
        configuration.database.database_path = format!("scripts/a_place_for_test_dbs_to_spawn_in_it,supposed_to_be_empty_cuz_tests_terminate_after_success_execution/{}", Uuid::new_v4().to_string());
        configuration.database.create_if_missing = true;
        configuration.database.journal_mode = "MEMORY".to_string();
        configuration.database.synchronous = "OFF".to_string();
        configuration.database.busy_timeout = 1;
        configuration.database.foreign_keys = true;
        configuration.database.auto_vacuum = "NONE".to_string();
        configuration.database.page_size = 4096;
        configuration.database.cache_size = "-10000".to_string();
        configuration.database.mmap_size = "0".to_string();
        configuration.database.temp_store = "MEMORY".to_string();
        configuration.email_client.base_url = email_server.uri();
        configuration
    };

    let db_pool = configure_database(&configuration.database).await?;
    sqlx::migrate!("./migrations").run(&db_pool).await?;

    let application = Application::build(configuration.clone()).await?;

    let db_path = configuration.database.database_path;
    let application_host = configuration.application.host;
    let application_port = application.port();

    let address = format!("http://{}:{}", application_host, application.port());

    tokio::spawn(async move { application.run_until_stopped().await.unwrap() });
    Ok(TestApp {
        address,
        port: application_port,
        db_pool,
        db_path,
        email_server,
    })
}
