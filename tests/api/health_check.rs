use reqwest::Client;

use crate::helpers::spawn_app;

#[tokio::test]
pub async fn health_check_works() {
    // Arrange
    let app = spawn_app().await;
    let client = Client::new();

    // Act
    let response = client
        .get(&format!("{}/health_check", &app.address))
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert!(response.status().is_success());
    assert_eq!(response.content_length(), Some(0));

    app.cleanup_test_db().await.unwrap();
}
