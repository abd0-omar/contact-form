use askama_axum::IntoResponse;
use axum::{extract::State, response::Redirect, Form};
use reqwest::StatusCode;
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
    AuthError(#[source] anyhow::Error),
    #[error("Something went wrong, my favourite error message")]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for LoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl IntoResponse for LoginError {
    fn into_response(self) -> askama_axum::Response {
        match self {
            LoginError::AuthError(e) => {
                tracing::error!(error_chain = ?e);
                let encoded_error = urlencoding::Encoded::new(e.to_string());
                Redirect::to(&format!("/login?error={}", encoded_error)).into_response()
            }
            LoginError::UnexpectedError(e) => {
                tracing::error!(error_chain = ?e);
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

#[tracing::instrument(
    skip(app_state, form_fields),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn login(
    State(app_state): State<AppState>,
    Form(form_fields): Form<FormFields>,
) -> Result<impl IntoResponse, LoginError> {
    let credentials = Credentials {
        username: form_fields.username,
        password: form_fields.password,
    };

    tracing::Span::current().record("username", tracing::field::display(&credentials.username));
    dbg!("wow");
    let user_id = validate_credentials(credentials, &app_state.pool)
        .await
        .map_err(|e| match e {
            AuthError::InvalidCredentials(_) => LoginError::AuthError(e.into()),
            AuthError::UnexpectedError(_) => LoginError::UnexpectedError(e.into()),
        })?;
    tracing::Span::current().record("user_id", tracing::field::display(&user_id));
    dbg!("wowzers");

    Ok(Redirect::to("/home"))
}
