use std::collections::HashMap;

use crate::helpers::{cleanup_test_db, spawn_app};
use reqwest::StatusCode;

use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

// insert table_name(_,_) into values(_,_);
#[derive(Debug, sqlx::FromRow)]
struct SubscriberInfo {
    name: String,
    email: String,
    status: String,
}

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    // arrange
    let app = spawn_app().await;
    let mut body = HashMap::new();
    body.insert("name", "hamada_test");
    body.insert("email", "hamada_test@yahoo.com");

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    // act
    let response = app.post_subscriptions(body).await;

    //assert
    assert_eq!(response.status(), StatusCode::OK);

    cleanup_test_db(&app.db_name)
        .await
        .expect(&format!("Failed to delete test database {}", app.db_name));
}

#[tokio::test]
async fn subscribe_persists_the_new_subscriber() {
    // arrange
    let app = spawn_app().await;
    let mut body = HashMap::new();
    body.insert("name", "hamada_test");
    body.insert("email", "hamada_test@yahoo.com");

    Mock::given(path("/messages"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    // act
    app.post_subscriptions(body).await;

    //assert
    let saved = sqlx::query_as!(
        SubscriberInfo,
        "SELECT email, name, status FROM subscriptions"
    )
    .fetch_one(&app.pool)
    .await
    .expect(
        "Failed to fetch subscriber's info, maybe because he's not in the db or something else",
    );

    assert_eq!(saved.name, "hamada_test");
    assert_eq!(saved.email, "hamada_test@yahoo.com");
    assert_eq!(saved.status, "pending_confirmation");

    cleanup_test_db(&app.db_name)
        .await
        .expect(&format!("Failed to delete test database {}", app.db_name));
}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_for_valid_data() {
    // Arrange
    let app = spawn_app().await;
    let body = HashMap::from([("name", "Shady Khalifa"), ("email", "shekohex@gmail.com")]);

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    // Act
    app.post_subscriptions(body).await;

    // Assert
    // Mock asserts on drop
    cleanup_test_db(&app.db_name)
        .await
        .expect(&format!("Failed to delete test database {}", app.db_name));
}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_with_a_link() {
    // Arrange
    let app = spawn_app().await;
    let body = HashMap::from([("name", "hamada_test"), ("email", "hamada_test@yahoo.com")]);

    Mock::given(path("/messages"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    // Act
    app.post_subscriptions(body).await;

    // Assert
    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = app.get_confirmation_links(&email_request);

    assert_eq!(confirmation_links.html, confirmation_links.plain_text);

    cleanup_test_db(&app.db_name)
        .await
        .expect(&format!("Failed to delete test database {}", app.db_name));
}

// just parsing form input if it is valid check no db values check
#[tokio::test]
async fn subscribe_returns_a_422_when_data_is_missing() {
    // arrange
    let app = spawn_app().await;

    // act
    let test_cases = vec![
        (
            HashMap::from([("email", "hamada@yahoo.com")]),
            String::from("missing name"),
        ),
        (
            HashMap::from([("name", "hamada")]),
            String::from("missing email"),
        ),
        (HashMap::new(), String::from("missing both")),
    ];

    for (body, error_message) in test_cases {
        let response = app.post_subscriptions(body).await;

        assert_eq!(
            response.status(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "API did not fail with 422 Bad Request when the payload was {}.",
            error_message
        );
    }

    cleanup_test_db(&app.db_name)
        .await
        .expect(&format!("Failed to delete test database {}", app.db_name));
}

#[tokio::test]
async fn subscribe_returns_a_422_when_fields_are_present_but_invalid() {
    // Arrange
    let app = spawn_app().await;
    let test_cases = vec![
        (
            HashMap::from([("name", ""), ("email", "good@answer.com")]),
            "empty name",
        ),
        (
            HashMap::from([("name", "Steve Harvey"), ("email", "")]),
            "empty email",
        ),
        (
            HashMap::from([("name", "Steve Harvey"), ("email", "show-me-invalid-email")]),
            "invalid email",
        ),
    ];

    for (body, description) in test_cases {
        // Act
        let response = app.post_subscriptions(body).await;

        // Assert
        assert_eq!(
            response.status(),
            StatusCode::BAD_REQUEST,
            "The API did not return a 422 Bad Request when the payload was {}.",
            description
        );
    }

    cleanup_test_db(&app.db_name)
        .await
        .expect(&format!("Failed to delete test database {}", app.db_name));
}

#[tokio::test]
async fn subscribe_fails_if_there_is_a_fatal_database_error() {
    // Arrange
    let app = spawn_app().await;
    let body = HashMap::from([("name", "le guin"), ("email", "ursula_le_guin@gmail.com")]);

    // Sabotage the database
    sqlx::query!("ALTER TABLE subscriptions DROP COLUMN name;")
        .execute(&app.pool)
        .await
        .unwrap();

    // Act
    let response = app.post_subscriptions(body).await;

    // Assert
    assert_eq!(response.status().as_u16(), 500);

    cleanup_test_db(&app.db_name)
        .await
        .expect(&format!("Failed to delete test database {}", app.db_name));
}
