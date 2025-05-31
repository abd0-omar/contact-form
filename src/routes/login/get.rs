use axum::response::{Html, IntoResponse};
use axum_messages::Messages;
use rinja_axum::Template;

#[derive(Template)]
#[template(path = "login/index.html")]
struct LoginTemplate {
    errors: Vec<String>,
}

#[tracing::instrument(name = "Login form", skip(messages))]
pub async fn login_form(messages: Messages) -> impl IntoResponse {
    Html(
        LoginTemplate {
            errors: messages.into_iter().map(|m| m.message).collect(),
        }
        .render()
        .unwrap(),
    )
}
