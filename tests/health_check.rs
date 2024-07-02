use std::collections::HashMap;

use configuration::get_configuration;
use contact_form::*;
use reqwest::{Client, StatusCode};
use sqlx::postgres::PgPoolOptions;
use startup::build_router;
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

// insert table_name into values(_,_);
// select
#[derive(sqlx::FromRow)]
struct SubscriberInfo {
    name: String,
    email: String,
}

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    // arrange
    let address = spawn_app().await;
    let configuration = get_configuration().expect("Failed to read configuation");
    let db_url = configuration.database.db_connection_string();

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .unwrap();

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

    // letsaved= sqlx::query!("SELECTemail,nameFROMsubscriptions",)
    // .fetch_one(&mut connection)
    // .await
    // .expect("Failedtofetchsaved subscription.");

    // no query_as! cuz my pc would be too slow for compile times
    // but may do it if it is needed in the CI.
    let query: SubscriberInfo = sqlx::query_as("SELECT email, name FROM subscriptions LIMIT 1")
        .fetch_one(&pool)
        .await
        .expect(
            "Failed to fetch subscriber's info, maybe because he's not in the db or something else",
        );

    assert_eq!(query.name, "hamada");
    assert_eq!(query.email, "hamada@yahoo.com");
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
    let configuration = get_configuration().expect("failed to read configuration");
    let db_url = configuration.database.db_connection_string();
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .unwrap();

    let application = build_router(pool).unwrap();
    //                                                                   random port
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    // we put async move because axum::serve() is an async fn
    let _ = tokio::spawn(async move { axum::serve(listener, application).await.unwrap() });
    format!("127.0.0.1:{}", port)
}
