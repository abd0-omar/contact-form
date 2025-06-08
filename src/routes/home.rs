use axum::response::Html;
use rinja_axum::Template;

#[derive(Template)]
#[template(path = "index.html")]
struct HomeTemplate;

pub async fn home() -> impl axum::response::IntoResponse {
    Html(HomeTemplate.render().unwrap())
}
