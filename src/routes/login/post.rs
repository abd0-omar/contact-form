use std::sync::Arc;

use axum::{
    extract::State,
    response::{IntoResponse, Redirect},
    Form,
};
use hmac::{Hmac, Mac};
use secrecy::{ExposeSecret, SecretString};

use crate::{
    authentication::{validate_credentials, AuthError, Credentials},
    routes::error_chain_fmt,
    startup::AppState,
};

#[derive(serde::Deserialize)]
pub struct FormData {
    username: String,
    password: SecretString,
}

#[tracing::instrument(
    skip(form, app_state),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn login(
    State(app_state): State<Arc<AppState>>,
    Form(form): Form<FormData>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    let credentials = Credentials {
        username: form.username,
        password: form.password,
    };
    tracing::Span::current().record("username", tracing::field::display(&credentials.username));
    match validate_credentials(credentials, &app_state.pool).await {
        Ok(user_id) => {
            tracing::Span::current().record("user_id", tracing::field::display(&user_id));
            Ok(Redirect::to("/"))
        }
        Err(e) => {
            let e = match e {
                AuthError::InvalidCredentials(_) => {
                    tracing::warn!(cause_chain = ?e);
                    LoginError::AuthError(e.into())
                }
                AuthError::UnexpectedError(_) => {
                    tracing::error!(cause_chain = ?e);
                    LoginError::UnexpectedError(e.into())
                }
            };

            let query_string = format!("error={}", urlencoding::Encoded::new(e.to_string()));
            let hmac_tag = {
                let mut mac = Hmac::<sha2::Sha256>::new_from_slice(
                    &app_state.hmac_secret.0.expose_secret().as_bytes(),
                )
                .unwrap();
                mac.update(query_string.as_bytes());
                mac.finalize().into_bytes()
            };

            Err(Redirect::to(&format!(
                "/login?{query_string}&tag={hmac_tag:x}"
            )))
        }
    }
}

#[derive(thiserror::Error)]
pub enum LoginError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error("Something went wrong")]
    UnexpectedError(#[from] anyhow::Error),
}

// not used because we need to redirect, so we return a redirect in the handler
// plus logging in the handler
// impl IntoResponse for LoginError {
//     fn into_response(self) -> axum::response::Response {
//         match self {
//             LoginError::AuthError(error) => {
//                 tracing::warn!(cause_chain = ?error);
//                 StatusCode::UNAUTHORIZED.into_response()
//             }
//             LoginError::UnexpectedError(error) => {
//                 tracing::error!(cause_chain = ?error);
//                 StatusCode::INTERNAL_SERVER_ERROR.into_response()
//             }
//         }
//     }
// }

impl std::fmt::Debug for LoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}
