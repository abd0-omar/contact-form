use std::sync::Arc;

use axum::{
    extract::State,
    response::{IntoResponse, Redirect, Response},
    Form,
};
use axum_messages::Messages;
use secrecy::SecretString;

use crate::{
    authentication::{validate_credentials, AuthError, Credentials},
    routes::error_chain_fmt,
    session_state::TypedSession,
    startup::AppState,
};

#[derive(serde::Deserialize)]
pub struct FormData {
    username: String,
    password: SecretString,
}

#[tracing::instrument(
    skip(form, app_state, session, messages),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn login(
    State(app_state): State<Arc<AppState>>,
    session: TypedSession,
    messages: Messages,
    Form(form): Form<FormData>,
) -> Result<Response, Response> {
    let credentials = Credentials {
        username: form.username,
        password: form.password,
    };
    tracing::Span::current().record("username", tracing::field::display(&credentials.username));
    match validate_credentials(credentials, &app_state.pool).await {
        Ok(user_id) => {
            tracing::Span::current().record("user_id", tracing::field::display(&user_id));

            if let Err(e) = session.rotate_id().await {
                let err = LoginError::UnexpectedError(e.into());
                tracing::error!(cause_chain = ?err);
                messages.error("Could not rotate session id");
                return Err(Redirect::to("/login").into_response());
            }

            if let Err(e) = session.insert_user_id(user_id).await {
                let err = LoginError::UnexpectedError(e.into());
                tracing::error!(cause_chain = ?err);
                messages.error("Could not insert user id");
                return Err(Redirect::to("/login").into_response());
            }

            Ok(Redirect::to("/admin/dashboard").into_response())
        }
        Err(e) => {
            let e = match e {
                // we don't show the error message to the user
                // so we don't do this
                // AuthError::InvalidCredentials(e) => {
                AuthError::InvalidCredentials(_) => {
                    tracing::warn!(cause_chain = ?e);
                    LoginError::AuthError(e.into())
                }
                AuthError::UnexpectedError(_) => {
                    tracing::error!(cause_chain = ?e);
                    LoginError::UnexpectedError(e.into())
                }
            };

            messages.error(e.to_string());
            Ok(Redirect::to("/login").into_response())
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
//                 // login redirect
//                 // return a redirect to the login page with the error message

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
