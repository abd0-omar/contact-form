use reqwest::StatusCode;

use crate::helpers::{spawn_app, FormData};

#[tokio::test]
pub async fn subscribe_returns_a_200_for_valid_form_data() {
    // Arrange
    let app = spawn_app().await.unwrap();
    let fake_user_form_data = FormData {
        name: Some("abood".to_string()),
        email: Some("3la_el_7doood@yahoo.com".to_string()),
    };
    // Act
    let response = app.post_subscriptions(&fake_user_form_data).await;
    // Assert
    let saved = sqlx::query!(
        r#"
    SELECT name, email
    FROM subscriptions
    "#
    )
    .fetch_one(&app.pool)
    .await
    .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(saved.name, fake_user_form_data.name.unwrap());
    assert_eq!(saved.email, fake_user_form_data.email.unwrap());

    app.cleanup_test_db().await.unwrap();
}

#[tokio::test]
pub async fn subscribe_returns_a_422_when_data_is_missing() {
    // Arrange

    let app = spawn_app().await.unwrap();

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
        let response = app.post_subscriptions(&invalid_form).await;
        // Assert
        assert_eq!(
            response.status(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "the API did not fail with 422 Bad Request when the payload was {}",
            error_message
        );
    }

    app.cleanup_test_db().await.unwrap();
}

#[tokio::test]
async fn subscribe_returns_a_400_when_fields_are_present_but_invalid() {
    // Arrange
    let app = spawn_app().await.unwrap();
    let test_cases = [
        (
            FormData {
                name: Some("".to_string()),
                email: Some("hamada123@yahoo.com".to_string()),
            },
            "name present (gift) but empty",
        ),
        (
            FormData {
                name: Some("hamada".to_string()),
                email: Some("".to_string()),
            },
            "empty email",
        ),
        (
            FormData {
                name: Some("hamada".to_string()),
                email: Some("definitely-not-(blitzcrank)-an-email".to_string()),
            },
            "invalid email",
        ),
    ];

    for (form_data, description) in test_cases {
        // Act
        let response = app.post_subscriptions(&form_data).await;

        // Assert
        assert_eq!(
            StatusCode::BAD_REQUEST,
            response.status(),
            "The API did not return a 200 OK when the payload was {}.",
            description
        );
    }

    app.cleanup_test_db().await.unwrap();
}
