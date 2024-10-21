use askama_axum::IntoResponse;
use axum::{extract::State, response::Redirect, Form};
use axum_messages::Messages;
use secrecy::Secret;

use crate::{
    authenticaiton::{validate_credentials, AuthError, Credentials},
    routes::error_chain_fmt,
    startup::AppState,
};

#[derive(serde::Deserialize)]
pub struct FormFields {
    username: String,
    password: Secret<String>,
}

#[derive(thiserror::Error)]
pub enum LoginError {
    #[error("Authentication failed.")]
    AuthError(#[source] anyhow::Error, Messages),
    #[error("Something went wrong, my favourite error message")]
    UnexpectedError(anyhow::Error, Messages),
}

impl std::fmt::Debug for LoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl IntoResponse for LoginError {
    fn into_response(self) -> askama_axum::Response {
        match &self {
            // remove cookies and utilize flash messages
            LoginError::AuthError(_e, flash) => {
                // I think it'll send an error that way
                let _flash = flash.clone().error(self.to_string());
                tracing::error!(error_chain = ?self);
                Redirect::to("/login").into_response()
            }
            LoginError::UnexpectedError(_e, flash) => {
                let _flash = flash.clone().error(self.to_string());
                tracing::error!(error_chain = ?self);
                Redirect::to("/login").into_response()
            }
        }
    }
}

#[tracing::instrument(
    skip(app_state, form_fields, flash),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn login(
    State(app_state): State<AppState>,
    flash: Messages,
    Form(form_fields): Form<FormFields>,
) -> Result<impl IntoResponse, LoginError> {
    let credentials = Credentials {
        username: form_fields.username,
        password: form_fields.password,
    };

    tracing::Span::current().record("username", tracing::field::display(&credentials.username));
    let user_id = validate_credentials(credentials, &app_state.pool)
        .await
        .map_err(|e| match e {
            AuthError::InvalidCredentials(e) => LoginError::AuthError(e.into(), flash),
            AuthError::UnexpectedError(e) => LoginError::UnexpectedError(e.into(), flash),
        })?;
    tracing::Span::current().record("user_id", tracing::field::display(&user_id));

    Ok(Redirect::to("/home"))
}
