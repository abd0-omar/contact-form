use crate::helpers::spawn_app;
use reqwest::Client;

use crate::helpers::cleanup_test_db;

#[tokio::test]
async fn health_check_works() {
    // Arrange
    let app = spawn_app().await;
    let client = Client::new();

    // Act
    let response = client
        .get(format!("{}/health_check", &app.address))
        .send()
        .await
        .expect("failed to execute a request to our server from reqwest client");
    // Assert
    // assert_eq!(result, expected)
    assert!(response.status().is_success());
    assert_eq!(response.content_length(), Some(0));

    cleanup_test_db(&app.db_name)
        .await
        .expect(&format!("Failed to delete test database {}", app.db_name));
}
