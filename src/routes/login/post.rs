use askama_axum::IntoResponse;
use axum::{extract::State, response::Redirect, Form};
use hmac::{Hmac, Mac};
use secrecy::{ExposeSecret, Secret};

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
    AuthError(#[source] anyhow::Error, AppState),
    #[error("Something went wrong, my favourite error message")]
    UnexpectedError(anyhow::Error, AppState),
}

impl std::fmt::Debug for LoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

// we didn't use this because we couldn't access app_state which has the "secret" for the HMAC
impl IntoResponse for LoginError {
    fn into_response(self) -> askama_axum::Response {
        match self {
            // ignore the duplication for now
            LoginError::AuthError(e, state) => {
                tracing::error!(error_chain = ?e);
                let query_string = format!("error={}", urlencoding::Encoded::new(e.to_string()));
                let secret: &[u8] = state.secret.0.expose_secret().as_bytes();
                let hmac_tag = {
                    let mut mac = Hmac::<sha2::Sha256>::new_from_slice(secret).unwrap();
                    mac.update(query_string.as_bytes());
                    mac.finalize().into_bytes()
                };
                Redirect::to(&format!("/login?{query_string}&tag={hmac_tag:x}")).into_response()
            }
            LoginError::UnexpectedError(e, state) => {
                tracing::error!(error_chain = ?e);
                let query_string = format!("error={}", urlencoding::Encoded::new(e.to_string()));
                let secret: &[u8] = state.secret.0.expose_secret().as_bytes();
                let hmac_tag = {
                    let mut mac = Hmac::<sha2::Sha256>::new_from_slice(secret).unwrap();
                    mac.update(query_string.as_bytes());
                    mac.finalize().into_bytes()
                };
                Redirect::to(&format!("/login?{query_string}&tag={hmac_tag:x}")).into_response()
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
    // let secret = app_state.clone().secret;
    let credentials = Credentials {
        username: form_fields.username,
        password: form_fields.password,
    };

    tracing::Span::current().record("username", tracing::field::display(&credentials.username));
    let user_id = validate_credentials(credentials, &app_state.pool)
        .await
        .map_err(|e| match e {
            AuthError::InvalidCredentials(_) => LoginError::AuthError(e.into(), app_state.clone()),
            AuthError::UnexpectedError(_) => LoginError::UnexpectedError(e.into(), app_state),
        })?;
    tracing::Span::current().record("user_id", tracing::field::display(&user_id));

    Ok(Redirect::to("/home"))
}
