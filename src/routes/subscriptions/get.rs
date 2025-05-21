use axum::response::{Html, IntoResponse};
use rinja_axum::Template;

#[derive(Template)]
#[template(path = "subscriptions/index.html")]
struct SubscriptionsTemplate;

pub async fn subscribe_form() -> impl IntoResponse {
    Html(SubscriptionsTemplate.render().unwrap())
}
