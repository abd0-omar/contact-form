use std::collections::HashMap;

use crate::helpers::{process_multipart, spawn_app};
use reqwest::StatusCode;

use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

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
    let mut body = HashMap::new();
    body.insert("name", "hamada_test");
    body.insert("email", "hamada_test@yahoo.com");

    Mock::given(path("/messages"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    // act
    let response = app.post_subscriptions(body).await;

    //assert
    assert_eq!(response.status(), StatusCode::OK);

    let saved = sqlx::query_as!(SubscriberInfo, "SELECT email, name FROM subscriptions")
        .fetch_one(&app.pool)
        .await
        .expect(
            "Failed to fetch subscriber's info, maybe because he's not in the db or something else",
        );

    assert_eq!(saved.name, "hamada_test");
    assert_eq!(saved.email, "hamada_test@yahoo.com");
}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_for_valid_data() {
    // Arrange
    let app = spawn_app().await;
    let body = HashMap::from([("name", "Shady Khalifa"), ("email", "shekohex@gmail.com")]);

    Mock::given(path("/messages"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    // Act
    app.post_subscriptions(body).await;

    // Assert
    // Mock asserts on drop
}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_with_a_link() {
    // Arrange
    let app = spawn_app().await;
    let mut body = HashMap::new();
    body.insert("name", "hamada_test");
    body.insert("email", "hamada_test@yahoo.com");

    Mock::given(path("/messages"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    // Act
    app.post_subscriptions(body.into()).await;

    // Assert
    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let body = std::str::from_utf8(&email_request.body).unwrap();
    dbg!(body);

    // let boundary = if let Some(head) = email_request.headers.get("Content-Type") {
    //     dbg!(&head);
    //     head.to_str().unwrap()
    //     //&head = "multipart/form-data; boundary=595e1f4aecec014a-e57b96d1e71c256c-4971e61e04ce9267-e68f8c85e1cde7e1"
    // } else {
    //     panic!("couldn't get boundary=");
    // };

    // let index = boundary.find("boundary=").unwrap();
    // // 9 is the len of 'boundry='
    // dbg!(&boundary[index + 9..]);

    let form_data = process_multipart(body.to_string()).await;
    dbg!(&form_data);

    // Extract the link from one of the request fields.
    let get_link = |s: &str| {
        let links: Vec<_> = linkify::LinkFinder::new()
            .links(s)
            .filter(|l| *l.kind() == linkify::LinkKind::Url)
            .collect();
        dbg!(&links);
        assert_eq!(links.len(), 1);
        links[0].as_str().to_owned()
    };
    let html_link = get_link(&form_data["html"]);
    let text_link = get_link(&form_data["text"]);
    dbg!(&html_link);
    dbg!(&text_link);

    assert_eq!(html_link, text_link);
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
            HashMap::from([
                ("name", "Steve Harvey"),
                ("email", "definitely-not-an-email"),
            ]),
            "invalid email",
        ),
    ];

    for (body, description) in test_cases {
        // Act
        let response = app.post_subscriptions(body).await;

        // Assert
        assert_eq!(
            response.status(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "The API did not return a 422 Bad Request when the payload was {}.",
            description
        );
    }
}
