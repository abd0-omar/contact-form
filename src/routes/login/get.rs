use askama::Template;
use askama_axum::IntoResponse;
use axum::response::Html;
use axum_messages::{Level, Messages};

#[derive(Template)]
#[template(path = "login.html")]
pub struct ErrorTemplate {
    errors: Vec<String>,
}

pub async fn login_form(flash_messages: Messages) -> impl IntoResponse {
    let flash_messages = flash_messages
        .into_iter()
        .filter(|m| m.level == Level::Error)
        .map(|message| message.to_string())
        .collect::<Vec<_>>();

    Html(
        ErrorTemplate {
            errors: flash_messages,
        }
        .render()
        .unwrap(),
    )
}
