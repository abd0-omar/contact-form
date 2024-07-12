use std::collections::HashMap;

use crate::helpers::spawn_app;
use reqwest::StatusCode;

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

    // act
    let mut params = HashMap::new();
    params.insert("name", "hamada_test");
    params.insert("email", "hamada_test@yahoo.com");

    let response = app.post_subscription(params).await;

    //assert
    assert_eq!(response.status(), StatusCode::OK);

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

    // act
    let test_cases = vec![
        (
            HashMap::from([("name", ""), ("email", "hamada@yahoo.com")]),
            String::from("missing name"),
        ),
        (
            HashMap::from([("name", "hamada"), ("email", "")]),
            String::from("missing email"),
        ),
        (HashMap::from([("", "")]), String::from("missing both")),
    ];
    for (body, error_message) in test_cases {
        let response = app.post_subscription(body).await;

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
        let response = app.post_subscription(body).await;

        // Assert
        assert_eq!(
            response.status(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "The API did not return a 422 Bad Request when the payload was {}.",
            description
        );
    }
}
