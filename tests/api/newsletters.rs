use std::collections::HashMap;

use crate::helpers::{cleanup_test_db, spawn_app, ConfirmationLinks, TestApp};
use reqwest::StatusCode;
use wiremock::matchers::{any, method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn newsletters_are_not_delivered_to_unconfirmed_subscribers() {
    // Arrange
    let app = spawn_app().await;
    create_unconfirmed_subscriber(&app).await;
    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        //Weassert that norequest isfired atPostmark!
        .expect(0)
        .mount(&app.email_server)
        .await;
    // Act
    // Asketch of the newsletter payloadstructure.
    // We might change it later on.
    let newsletter_request_body = serde_json::json!({
    "title": "Newsletter title",
    "content": {
    "text": "Newsletter body as plain text",
    "html": "<p>Newsletter body as HTML</p>",
    }
    });
    let response = app.post_newsletters(newsletter_request_body).await;
    // Assert
    assert_eq!(response.status().as_u16(), 200);
    // Mock verifies on Drop that we haven't sent the newsletter email
    cleanup_test_db(&app.db_name)
        .await
        .expect(&format!("Failed to delete test database {}", app.db_name));
}
/// Use the public API of the application under test to create
/// an unconfirmed subscriber.
async fn create_unconfirmed_subscriber(app: &TestApp) -> ConfirmationLinks {
    let body = HashMap::from([("name", "le guin"), ("email", "ursula_le_guin@gmail.com")]);
    let _mock_guard = Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .named("Create unconfirmed subscriber")
        .expect(1)
        // mount as scoped, to have a mock guard that will drop at the end of the fn
        // because we have two mocks and we don't want them to step on each other's toes
        .mount_as_scoped(&app.email_server)
        .await;
    app.post_subscriptions(body)
        .await
        .error_for_status()
        .unwrap();
    let email_request = &app
        .email_server
        .received_requests()
        .await
        .unwrap()
        .pop()
        .unwrap();
    app.get_confirmation_links(&email_request)
}

async fn create_confirmed_subscriber(app: &TestApp) {
    let confirmation_links = create_unconfirmed_subscriber(app).await;
    reqwest::get(confirmation_links.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
}

#[tokio::test]
async fn newsletters_are_delivered_to_confirmed_subscribers() {
    // Arrange
    let app = spawn_app().await;

    create_confirmed_subscriber(&app).await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    // Act
    let newsletter_request_body = serde_json::json!({
    "title": "Newslettertitle",
    "content": {
    "text": "Newsletterbody as plaintext",
    "html": "<p>Newsletter bodyas HTML</p>",
    }
    });

    let response = app.post_newsletters(newsletter_request_body).await;
    // Assert
    assert_eq!(response.status().as_u16(), 200);
    // Mock verifies onDropthat wehave sentthenewsletter email

    cleanup_test_db(&app.db_name)
        .await
        .expect(&format!("Failed to delete test database {}", app.db_name));
}

#[tokio::test]
async fn newsletters_returns_400_for_invalid_data() {
    // Arrange
    let app = spawn_app().await;

    let test_cases = vec![
        (
            serde_json::json!({
            "content": {
            "text": "Newsletterbodyas plaintext",
            "html": "<p>Newsletterbodyas HTML</p>",
            }
            }),
            "missing title",
        ),
        (
            serde_json::json!({"title": "Newsletter!"}),
            "missing content",
        ),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = app.post_newsletters(invalid_body).await;

        //Assert
        assert_eq!(
            422,
            response.status().as_u16(),
            "The API did notfailwith 400Bad Requestwhenthe payloadwas {}.",
            error_message
        );
    }

    cleanup_test_db(&app.db_name)
        .await
        .expect(&format!("Failed to delete test database {}", app.db_name));
}

#[tokio::test]
async fn requests_missing_authorization_are_rejected() {
    // Arrange
    let app = spawn_app().await;

    // Act
    let body = serde_json::json!({
    "title": "Newsletter title",
    "content": {
    "text": "Newsletter body as plain text",
    "html": "<p>Newsletter body as HTML</p>",
    }
    });

    let response = reqwest::Client::new()
        .post(&format!("{}/newsletters", app.address))
        .json(&body)
        .send()
        .await
        .unwrap();

    // result, output
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    // WWW-Authenticate: Basic realm="publish"
    assert_eq!(
        r#"Basic realm="publish""#,
        response.headers()["WWW-Authenticate"]
    );

    cleanup_test_db(&app.db_name)
        .await
        .expect(&format!("Failed to delete test database {}", app.db_name));
}
