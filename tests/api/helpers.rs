use std::collections::HashMap;

use configuration::{get_configuration, DatabaseSettings};
use contact_form::*;
use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use startup::{get_connection_pool, Application};
use uuid::Uuid;
use wiremock::MockServer;

use contact_form::telemetry::{get_subscriber, init_subscriber};

static TRACING: Lazy<()> = Lazy::new(|| {
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
    pub pool: PgPool,
    pub email_server: MockServer,
    pub port: u16,
}

// TODO: check if spawn_app should be in in impl block of TestApp
impl TestApp {
    pub async fn post_subscriptions(&self, form: HashMap<&str, &str>) -> reqwest::Response {
        reqwest::Client::new()
            .post(format!("http://{}/subscriptions", self.address))
            .form(&form)
            .send()
            .await
            .expect("failed to fire a response from reqwest")

        // same as below

        // https://developer.mozilla.org/en-US/docs/Web/HTTP/Methods/POST#example
        // A simple form using the default application/x-www-form-urlencoded content type:
        // HTTP
        // Copy to Clipboard
        // POST /test HTTP/1.1
        // Host: foo.example
        // Content-Type: application/x-www-form-urlencoded
        // Content-Length: 27
        // field1=value1&field2=value2

        // https://www.w3schools.com/tags/ref_urlencode.ASP
        // "space" -> "%20"
        // "@" -> "%40"

        // let response = client
        //     .post(format!("http://{}/", address))
        //     .header("Content-Type", "application/x-www-form-urlencoded")
        //     .body("name=hamada&email=hamada%40yahoo.com")
        //     .send()
        //     .await
        //     .expect("failed to execute a request to our server from reqwest client");

        // dbg!(&response);
    }

    pub fn get_confirmation_links(&self, email_request: &wiremock::Request) -> ConfirmationLinks {
        // postmark
        // let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();
        // mailgun
        let body = String::from_utf8(email_request.body.clone()).unwrap();
        let body = process_multipart(body);

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

        // let html = get_link(body["HtmlBody"].as_str().unwrap());
        // let plain_text = get_link(body["TextBody"].as_str().unwrap());

        let html = get_link(body["html"].as_str());
        let plain_text = get_link(body["text"].as_str());

        ConfirmationLinks { html, plain_text }
    }
}

pub async fn spawn_app() -> TestApp {
    // The first time `initialize` is invoked the code in `TRACING` is executed.
    // All other invocations will instead skip execution.
    Lazy::force(&TRACING);

    let email_server = MockServer::start().await;

    let configuration = {
        let mut c = get_configuration().expect("failed to read configuration");
        // Use a different database for each test case
        c.database.database_name = Uuid::new_v4().to_string();
        // random port
        c.application.port = 0;
        //Use the mock server as email API
        c.email_client.base_url = email_server.uri();
        c
    };

    // create and migrate the database
    // we don't the pgpool from here, we just create and migrate
    // then we later get the pool from startup.rs/get_connection_pool() fn
    configure_database(&configuration.database).await;

    let application = Application::build(configuration.clone())
        .await
        .expect("Failed to build the application");

    let application_port = application.port();

    // // get the port before spawning the application
    // // because spawn would move `application`
    // let address = format!("127.0.0.1:{}", application.port());
    let _ = tokio::spawn(application.run_until_stopped());

    TestApp {
        address: format!("127.0.0.1:{}", application_port),
        pool: get_connection_pool(&configuration.database),
        email_server,
        port: application_port,
    }
}

async fn configure_database(config: &DatabaseSettings) -> PgPool {
    // it's not really a pool, it's just one connection, so connection would be a better name
    // but we'll leave it as pool

    // create database without a name, `PgConnection` just a connection
    let mut pool = PgConnection::connect_with(&config.without_db())
        .await
        .expect("Failed to connect to postgres");

    // create database with our random database name
    pool.execute(format!(r#"CREATE DATABASE "{}";"#, config.database_name).as_str())
        .await
        .expect("Failed to create database");

    // migrate our new random database
    // `PgPool`
    let pool = PgPool::connect_with(config.with_db())
        .await
        .expect("Failed to connect to postgres");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to migrate the database");

    pool
}

pub fn process_multipart(mut body: String) -> HashMap<String, String> {
    let mut form_data = HashMap::new();

    let disposal = "Content-Disposition: form-data; name=";

    dbg!(&body);

    loop {
        // let index = if let Some(idx) = body.find(disposal).unwrap() + disposal.len() + 1;
        let index = if let Some(idx) = body.find(disposal) {
            idx + disposal.len() + 1
        } else {
            break;
        };
        let index_end = body[index..].find("\r\n\r\n").unwrap() + index - 1;
        dbg!(index);
        dbg!(index_end);
        let key = &body[index..index_end].to_string();
        dbg!(&key);
        let second_end = body[index_end + 10..].find("\r\n").unwrap() + index_end + 10;
        dbg!(index_end);
        dbg!(second_end);
        let value = &body[index_end + 5..second_end].to_string();
        dbg!(&value);
        body = body[second_end..].to_string();
        dbg!(&body);
        form_data.insert(key.to_owned(), value.to_owned());
    }

    form_data
}

pub struct ConfirmationLinks {
    pub html: reqwest::Url,
    pub plain_text: reqwest::Url,
}
