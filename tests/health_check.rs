use std::collections::HashMap;

use configuration::{get_configuration, DatabaseSettings};
use contact_form::*;
use email_client::EmailClient;
use once_cell::sync::Lazy;
use reqwest::{Client, StatusCode};
use sqlx::{Connection, Executor, PgConnection, PgPool};
use startup::build_router;
use uuid::Uuid;

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
}

async fn spawn_app() -> TestApp {
    // The first time `initialize` is invoked the code in `TRACING` is executed.
    // All other invocations will instead skip execution.
    Lazy::force(&TRACING);

    let mut configuration = get_configuration().expect("failed to read configuration");
    configuration.database.database_name = Uuid::new_v4().to_string();
    // create table from the random db name you just generated
    let pool = configure_database(&configuration.database).await;

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

    let application = build_router(pool.clone(), email_client).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    // our random port
    let port = listener.local_addr().unwrap().port();
    // we put async move because axum::serve() is an async fn
    let _ = tokio::spawn(async move { axum::serve(listener, application).await.unwrap() });

    TestApp {
        address: format!("127.0.0.1:{}", port),
        pool,
    }
}

async fn configure_database(config: &DatabaseSettings) -> PgPool {
    // it's really a pool, it's just one connection, so connection would be a better name
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

#[tokio::test]
async fn health_check_works() {
    // arrange
    let app = spawn_app().await;
    let client = Client::new();

    // act
    let response = client
        .get(format!("http://{}/health_check", &app.address))
        .send()
        .await
        .expect("failed to execute a request to our server from reqwest client");
    // assert
    // assert_eq!(result, expected)
    assert!(response.status().is_success());
    assert_eq!(response.content_length(), Some(0));
}

// insert table_name into values(_,_);
// select
#[derive(Debug, sqlx::FromRow)]
struct SubscriberInfo {
    name: String,
    email: String,
}

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    // arrange
    let app = spawn_app().await;
    let client = Client::new();

    // act
    let mut params = HashMap::new();
    params.insert("name", "hamada_test");
    params.insert("email", "hamada_test@yahoo.com");
    let response = client
        .post(format!("http://{}/subscriptions", &app.address))
        .form(&params)
        .send()
        .await
        .expect("failed to execute a request to our server from reqwest client");

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

    //assert
    assert_eq!(response.status(), StatusCode::OK);

    // letsaved= sqlx::query!("SELECTemail,nameFROMsubscriptions",)
    // .fetch_one(&mut connection)
    // .await
    // .expect("Failedtofetchsaved subscription.");

    // no query_as! cuz my pc would be too slow for compile times
    // but may do it if it is needed in the CI.
    let query = sqlx::query_as!(SubscriberInfo, "SELECT email, name FROM subscriptions")
        .fetch_one(&app.pool)
        .await
        .expect(
            "Failed to fetch subscriber's info, maybe because he's not in the db or something else",
        );

    assert_eq!(query.name, "hamada_test");
    assert_eq!(query.email, "hamada_test@yahoo.com");
}

// just parsing form input if it is valid check no db values check
#[tokio::test]
async fn subscribe_returns_a_422_when_data_is_missing() {
    // arrange
    let app = spawn_app().await;
    let client = Client::new();

    // act
    let test_cases = vec![
        (
            HashMap::from([("", "hamada@yahoo.com")]),
            String::from("missing name"),
        ),
        (
            HashMap::from([("hamada", "")]),
            String::from("missing email"),
        ),
        (HashMap::from([("", "")]), String::from("missing both")),
    ];
    for (body, error_message) in test_cases {
        let response = client
            .post(format!("http://{}/subscriptions", &app.address))
            .form(&body)
            .send()
            .await
            .expect("failed to execute a request to our server from reqwest client");

        assert_eq!(
            response.status(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "API did not fail with 422 Bad Request when the payload was {}.",
            error_message
        );
    }
}

#[tokio::test]
async fn subscribe_returns_a_422_when_fields_are_present_but_invalid() {
    // Arrange
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let test_cases = vec![
        ("name=&email=ursula_le_guin%40gmail.com", "empty name"),
        ("name=Ursula&email=", "empty email"),
        ("name=Ursula&email=definitely-not-an-email", "invalid email"),
    ];

    for (body, description) in test_cases {
        // Act
        let response = client
            .post(&format!("http://{}/subscriptions", &app.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request.");

        // Assert
        assert_eq!(
            response.status(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "The API did not return a 422 Bad Request when the payload was {}.",
            description
        );
    }
}
