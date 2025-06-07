use axum::{
    extract::Path,
    response::{Html, IntoResponse},
};
use std::fs;
use std::path::PathBuf;

/// Handler for the blog index page that lists all blog posts
pub async fn blog_index() -> impl IntoResponse {
    let blog_path = PathBuf::from("frontend/dist/blog/index.html");
    match fs::read_to_string(blog_path) {
        Ok(content) => Html(content).into_response(),
        Err(_) => (axum::http::StatusCode::NOT_FOUND, "Blog index not found").into_response(),
    }
}

/// Handler for individual blog posts
pub async fn blog_post(Path(slug): Path<String>) -> impl IntoResponse {
    let blog_path = PathBuf::from(format!("frontend/dist/blog/{}/index.html", slug));
    match fs::read_to_string(blog_path) {
        Ok(content) => Html(content).into_response(),
        Err(_) => (axum::http::StatusCode::NOT_FOUND, "Blog post not found").into_response(),
    }
}
