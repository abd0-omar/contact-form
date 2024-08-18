use std::collections::HashMap;

use crate::helpers::cleanup_test_db;
use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};

use crate::helpers::spawn_app;

#[tokio::test]
async fn confirmations_without_token_are_rejected_with_a_400() {
    // arrange
    let app = spawn_app().await;

    // act
    // must have a token query, confirm?To$KeN
    let response = reqwest::get(&format!("{}/subscriptions/confirm", app.address))
        .await
        .unwrap();

    // assert
    assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);

    cleanup_test_db(&app.db_name)
        .await
        .expect(&format!("Failed to delete test database {}", app.db_name));
}

#[tokio::test]
async fn the_link_returned_by_subscribe_returns_a_200_if_called() {
    // arrange
    let app = spawn_app().await;
    let body = HashMap::from([("name", "Steve Carell"), ("email", "theoffice@yahoo.com")]);

    // acts as the mailgun server
    Mock::given(path("/messages"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body).await;
    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = app.get_confirmation_links(email_request);

    // act
    let response = reqwest::get(confirmation_links.html).await.unwrap();

    // assert
    assert_eq!(response.status(), axum::http::StatusCode::OK);

    cleanup_test_db(&app.db_name)
        .await
        .expect(&format!("Failed to delete test database {}", app.db_name));
}

#[tokio::test]
async fn clicking_on_the_confirmation_link_confirms_a_subscriber() {
    // arrange
    let app = spawn_app().await;
    let body = HashMap::from([("name", "John Mayer"), ("email", "stop@train.com")]);

    Mock::given(path("/messages"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body).await;
    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = app.get_confirmation_links(&email_request);

    // act
    dbg!(&confirmation_links.html);
    reqwest::get(confirmation_links.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    // assert
    let saved = sqlx::query!("SELECT email, name, status FROM subscriptions")
        .fetch_one(&app.pool)
        .await
        .expect("Failed to fetch saved subscription.");

    assert_eq!(saved.email, "stop@train.com");
    assert_eq!(saved.name, "John Mayer");
    assert_eq!(saved.status, "confirmed");

    cleanup_test_db(&app.db_name)
        .await
        .expect(&format!("Failed to delete test database {}", app.db_name));
}
