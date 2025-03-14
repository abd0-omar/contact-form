use argon2::{Algorithm, Params, PasswordHasher, Version};
use std::{collections::HashMap, fs::remove_file};

use argon2::{password_hash::SaltString, Argon2};
use configuration::{get_configuration, DatabaseSettings};
use contact_form::*;
use once_cell::sync::Lazy;
use sqlx::SqlitePool;
use startup::{get_pool, Application};
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
    pub pool: SqlitePool,
    pub email_server: MockServer,
    pub port: u16,
    pub db_name: String,
    pub test_user: TestUser,
    pub api_client: reqwest::Client,
}

pub struct ConfirmationLinks {
    pub html: reqwest::Url,
    pub plain_text: reqwest::Url,
}

pub fn assert_is_redirect_to(response: &reqwest::Response, location: &str) {
    assert_eq!(response.status().as_u16(), 303);
    assert_eq!(response.headers().get("Location").unwrap(), location);
}

impl TestApp {
    pub async fn get_login_html(&self) -> String {
        self.api_client
            .get(&format!("{}/login", &self.address))
            .send()
            .await
            .expect("Failed to execute request.")
            .text()
            .await
            .unwrap()
    }

    pub async fn post_login<Body>(&self, body: &Body) -> reqwest::Response
    where
        Body: serde::Serialize,
    {
        self.api_client
            // to allow testing redirects
            .post(format!("{}/login", self.address))
            .form(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn post_newsletters(&self, body: serde_json::Value) -> reqwest::Response {
        self.api_client
            .post(&format!("{}/newsletters", &self.address))
            .basic_auth(&self.test_user.username, Some(&self.test_user.password))
            .json(&body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn post_subscriptions(&self, form: HashMap<&str, &str>) -> reqwest::Response {
        self.api_client
            .post(format!("{}/subscriptions", self.address))
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

    // get the port before spawning the application
    // because spawn would move `application`
    let _ = tokio::spawn(application.run_until_stopped());

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .cookie_store(true)
        .build()
        .unwrap();

    let test_app = TestApp {
        address: format!("http://localhost:{}", application_port),
        pool: get_pool(&configuration.database),
        email_server,
        port: application_port,
        db_name: format!("{}", configuration.database.database_name),
        test_user: TestUser::generate(),
        api_client: client,
    };
    test_app.test_user.store(&test_app.pool).await;
    test_app
}

async fn configure_database(config: &DatabaseSettings) -> SqlitePool {
    let pool = get_pool(&config);
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to migrate the database");

    pool
}

pub struct TestUser {
    user_id: Uuid,
    pub username: String,
    pub password: String,
}

impl TestUser {
    fn generate() -> Self {
        Self {
            user_id: Uuid::new_v4(),
            username: Uuid::new_v4().to_string(),
            password: Uuid::new_v4().to_string(),
        }
    }

    async fn store(&self, pool: &SqlitePool) {
        let salt = SaltString::generate(&mut rand::thread_rng());
        // We// Match parameters of the default password
        let password_hash = Argon2::new(
            Algorithm::Argon2id,
            Version::V0x13,
            Params::new(15000, 2, 1, None).unwrap(),
        )
        .hash_password(self.password.as_bytes(), &salt)
        .unwrap()
        .to_string();

        // because sqlite doesn't have uuid type
        let user_id = self.user_id.to_string();
        sqlx::query!(
            r#"
        INSERT INTO users(user_id, username, password_hash) VALUES($1, $2, $3)
        "#,
            user_id,
            self.username,
            password_hash
        )
        .execute(pool)
        .await
        .expect("Failed to store test user.");
    }
}

pub async fn cleanup_test_db(db_name: &String) -> Result<(), sqlx::Error> {
    remove_file(&format!("{}.db", db_name))?;
    Ok(())
}
