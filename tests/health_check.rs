use std::error::Error;

use contact_form::*;
#[tokio::test]
async fn it_works() {
    // arrange
    spawn_app()
        .await
        .unwrap()
        .run_until_stopped()
        .await
        .unwrap();
    // act
    let client = reqwest::Client::new();
    let response = client
        .get("https://127.0.0.1:8000/health_check")
        .send()
        .await
        .expect("failed to execute a request to our server from reqwest client");
    // assert
    // assert_eq!(result, expected)
    assert!(response.status().is_success());
    assert_eq!(response.content_length(), Some(0));
}

pub async fn spawn_app() -> Result<Application, Box<dyn Error>> {
    Application::run().await
}
