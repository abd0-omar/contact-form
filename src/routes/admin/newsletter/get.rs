use axum::response::{Html, IntoResponse};
use axum_messages::Messages;
use rinja_axum::Template;

#[derive(Template)]
#[template(path = "publish_newsletter/index.html")]
struct PublishNewsletterTemplate {
    errors: Vec<String>,
}

#[tracing::instrument(name = "Publish newsletter form", skip(messages))]
pub async fn publish_newsletter_form(
    messages: Messages,
) -> Result<axum::response::Response, axum::response::Response> {
    Ok(Html(
        PublishNewsletterTemplate {
            errors: messages.into_iter().map(|m| m.message).collect(),
        }
        .render()
        .unwrap(),
    )
    .into_response())
}
