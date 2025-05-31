use axum::response::{Html, IntoResponse, Redirect};
use axum_messages::Messages;
use rinja_axum::Template;

use crate::session_state::TypedSession;
use crate::utils::e500;

#[derive(Template)]
#[template(path = "change_password/index.html")]
struct ChangePasswordTemplate {
    errors: Vec<String>,
}

pub async fn change_password_form(
    session: TypedSession,
    messages: Messages,
) -> Result<axum::response::Response, axum::response::Response> {
    if session.get_user_id().await.map_err(e500)?.is_none() {
        return Ok(Redirect::to("/login").into_response());
    };
    Ok(Html(
        ChangePasswordTemplate {
            errors: messages.into_iter().map(|m| m.message).collect(),
        }
        .render()
        .unwrap(),
    )
    .into_response())
}
