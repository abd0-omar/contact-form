use crate::authentication::{self, validate_credentials, AuthError, Credentials, UserId};
use crate::routes::admin::dashboard::get_username;
use crate::startup::AppState;
use crate::utils::e500;
use axum::extract::State;
use axum::response::{IntoResponse, Redirect};
use axum::{Extension, Form};
use axum_messages::Messages;
use secrecy::{ExposeSecret, SecretString};
use std::sync::Arc;

#[derive(serde::Deserialize)]
pub struct FormData {
    current_password: SecretString,
    new_password: SecretString,
    new_password_check: SecretString,
}

pub async fn change_password(
    State(app_state): State<Arc<AppState>>,
    messages: Messages,
    Extension(user_id): Extension<UserId>,
    Form(form): Form<FormData>,
) -> Result<axum::response::Response, axum::response::Response> {
    if form.new_password.expose_secret() != form.new_password_check.expose_secret() {
        messages.error("You entered two different new passwords - the field values must match.");
        return Ok(Redirect::to("/admin/password").into_response());
    }

    let username = get_username(*user_id, &app_state.pool)
        .await
        .map_err(e500)?;

    let credentials = Credentials {
        username,
        password: form.current_password,
    };

    if let Err(e) = validate_credentials(credentials, &app_state.pool).await {
        return match e {
            AuthError::InvalidCredentials(err) => {
                messages.error("The current password is incorrect.");
                tracing::error!(chain_error = ?err);
                Ok(Redirect::to("/admin/password").into_response())
            }
            AuthError::UnexpectedError(_) => Err(e500(e).into_response()),
        };
    }

    authentication::change_password(*user_id, form.new_password, &app_state.pool)
        .await
        .map_err(e500)?;
    messages.success("Your password has been changed.");
    Ok(Redirect::to("/admin/password").into_response())
}
