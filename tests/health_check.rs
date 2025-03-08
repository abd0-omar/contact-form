use std::sync::LazyLock;

use newzletter::{
    configuration::{configure_database, get_configuration},
    startup::Application,
    telemetry::{get_subscriber, init_subscriber},
};
use reqwest::{Client, StatusCode};
use serde::Serialize;
use sqlx::sqlite::SqlitePool;
use tokio::fs::remove_file;
use uuid::Uuid;

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
    address: String,
    pool: SqlitePool,
    // to later delete it
    db_name: String,
}

async fn spawn_app() -> anyhow::Result<TestApp> {
    // The first time `initialize` is invoked the code in `TRACING` is executed.
    // All other invocations will instead skip execution.
    LazyLock::force(&TRACING);

    let configuration = {
        let mut configuration = get_configuration().expect("Failed to read configuration");
        configuration.application_port = 0;
        configuration.database.database_name = Uuid::new_v4().to_string();
        configuration
    };

    let pool = configure_database(&configuration.database).await?;

    let application = Application::build(&configuration).await?;

    let address = format!("http://127.0.0.1:{}", application.port());

    tokio::spawn(async move { application.run_until_stopped().await.unwrap() });
    Ok(TestApp {
        address,
        pool,
        db_name: configuration.database.database_name,
    })
}

pub async fn cleanup_test_db(db_name: &String) -> Result<(), sqlx::Error> {
    remove_file(&format!("{}.db", db_name)).await?;
    Ok(())
}

#[tokio::test]
pub async fn health_check_works() {
    // Arrange
    let TestApp {
        address,
        pool: _,
        db_name,
    } = spawn_app().await.unwrap();
    let client = Client::new();

    // Act
    let response = client
        .get(&format!("{}/health_check", address))
        .send()
        .await
        .unwrap();

    // Assert
    assert!(response.status().is_success());
    assert_eq!(response.content_length(), Some(0));

    cleanup_test_db(&db_name).await.unwrap();
}

#[derive(Serialize)]
struct FormData {
    name: Option<String>,
    email: Option<String>,
}
#[tokio::test]
pub async fn subscribe_returns_a_200_for_valid_form_data() {
    // Arrange
    let TestApp {
        address,
        pool,
        db_name,
    } = spawn_app().await.unwrap();
    let client = Client::new();
    let fake_user_form_data = FormData {
        name: Some("abood".to_string()),
        email: Some("3la el 7doood".to_string()),
    };

    // Act
    let response = client
        .post(format!("{}/subscribe", address))
        .form(&fake_user_form_data)
        .send()
        .await
        .unwrap();
    // Assert
    let saved = sqlx::query!(
        r#"
    SELECT name, email
    FROM subscriptions
    "#
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(saved.name, fake_user_form_data.name.unwrap());
    assert_eq!(saved.email, fake_user_form_data.email.unwrap());

    cleanup_test_db(&db_name).await.unwrap();
}

#[tokio::test]
pub async fn subscribe_returns_a_422_when_data_is_missing() {
    // Arrange
    let TestApp {
        address,
        pool: _,
        db_name,
    } = spawn_app().await.unwrap();
    let client = Client::new();

    let test_cases = vec![
        (
            FormData {
                name: Some("abood".to_string()),
                email: None,
            },
            "missing the email",
        ),
        (
            FormData {
                name: None,
                email: Some("email@email_proivderdotcom".to_string()),
            },
            "missing the name",
        ),
        (
            FormData {
                name: None,
                email: None,
            },
            "missing both",
        ),
    ];
    // Act
    for (invalid_form, error_message) in test_cases {
        let response = client
            .post(format!("{}/subscribe", address))
            .form(&invalid_form)
            .send()
            .await
            .unwrap();
        // Assert
        assert_eq!(
            response.status(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "the API did not fail with 422 Bad Request when the payload was {}",
            error_message
        );
    }

    cleanup_test_db(&db_name).await.unwrap();
}
