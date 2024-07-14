use std::collections::HashMap;

use reqwest::Url;
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
    let response = reqwest::get(&format!("http://{}/subscriptions/confirm", app.address))
        .await
        .unwrap();

    // assert
    assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
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
    let response = reqwest::get(confirmation_links.html)
    .await
    .unwrap();

    // assert
    // The two links should be identical
    assert_eq!(response.status(), axum::http::StatusCode::OK);
}
