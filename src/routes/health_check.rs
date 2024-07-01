use axum::http::StatusCode;

pub async fn health_check() -> impl axum::response::IntoResponse {
    StatusCode::OK
}
