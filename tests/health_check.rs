use contact_form::*;
#[tokio::test]
async fn it_works() {
    // arrange
    spawn_app().await;
    // act
    let client = reqwest::Client::new();
    let response = client
        .get("http://127.0.0.1:8000/health_check")
        .send()
        .await
        .expect("failed to execute a request to our server from reqwest client");
    // assert
    // assert_eq!(result, expected)
    assert!(response.status().is_success());
    assert_eq!(response.content_length(), Some(0));
}

pub async fn spawn_app() {
    let application = build_router().unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8000")
        .await
        .unwrap();
async fn spawn_app() -> String {
    let application = build_router().unwrap();
    //                                                                   random port
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    // we put async move because axum::serve() is an async fn
    let _ = tokio::spawn(async move { axum::serve(listener, application).await.unwrap() });
    format!("127.0.0.1:{}", port)
}
