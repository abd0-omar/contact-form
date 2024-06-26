use std::collections::HashMap;

use contact_form::*;
use reqwest::{Client, StatusCode};
#[tokio::test]
async fn health_check_works() {
    // arrange
    let address = spawn_app().await;
    // act
    let client = Client::new();
    let response = client
        .get(format!("http://{}/health_check", address))
        .send()
        .await
        .expect("failed to execute a request to our server from reqwest client");
    // assert
    // assert_eq!(result, expected)
    assert!(response.status().is_success());
    assert_eq!(response.content_length(), Some(0));
}

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    // arrange
    let address = spawn_app().await;

    let client = Client::new();

    // act
    let mut params = HashMap::new();
    params.insert("name", "hamada");
    params.insert("email", "hamada@yahoo.com");
    let response = client
        .post(format!("http://{}/subscriptions", address))
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
}

#[tokio::test]
async fn subscribe_returns_a_422_when_data_is_missing() {
    // arrange
    let address = spawn_app().await;

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
            .post(format!("http://{}/subscriptions", address))
            .form(&body)
            .send()
            .await
            .expect("failed to execute a request to our server from reqwest client");

        assert_eq!(
            response.status(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "API did not fail with 400 Bad Request when the payload was {}.",
            error_message
        );
    }
}

async fn spawn_app() -> String {
    let application = build_router().unwrap();
    //                                                                   random port
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    // we put async move because axum::serve() is an async fn
    let _ = tokio::spawn(async move { axum::serve(listener, application).await.unwrap() });
    format!("127.0.0.1:{}", port)
}
