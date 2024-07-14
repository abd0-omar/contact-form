use crate::helpers::spawn_app;
use reqwest::Client;

#[tokio::test]
async fn health_check_works() {
    // arrange
    let app = spawn_app().await;
    let client = Client::new();

    // act
    let response = client
        .get(format!("http://{}/health_check", &app.address))
        .send()
        .await
        .expect("failed to execute a request to our server from reqwest client");
    // assert
    // assert_eq!(result, expected)
    assert!(response.status().is_success());
    assert_eq!(response.content_length(), Some(0));
}