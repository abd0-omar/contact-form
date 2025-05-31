use axum::{
    body::Body,
    http::Request,
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
};
use std::ops::Deref;
use uuid::Uuid;

use crate::{routes::error_chain_fmt, session_state::TypedSession};

#[derive(Copy, Clone, Debug)]
pub struct UserId(Uuid);

impl std::fmt::Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Deref for UserId {
    type Target = Uuid;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(thiserror::Error)]
pub enum AuthMiddlewareError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
}

impl std::fmt::Debug for AuthMiddlewareError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl IntoResponse for AuthMiddlewareError {
    fn into_response(self) -> Response {
        match self {
            AuthMiddlewareError::AuthError(e) => {
                tracing::error!(cause_chain = ?e);
                Redirect::to("/login").into_response()
            }
        }
    }
}

pub async fn reject_anonymous_users(
    session: TypedSession,
    request: Request<Body>,
    next: Next,
) -> Result<Response, AuthMiddlewareError> {
    match session
        .get_user_id()
        .await
        .map_err(|e| AuthMiddlewareError::AuthError(e.into()))?
    {
        Some(user_id) => {
            let mut request = request;
            request.extensions_mut().insert(UserId(user_id));
            Ok(next.run(request).await)
        }
        None => Err(AuthMiddlewareError::AuthError(anyhow::anyhow!(
            "The user has not logged in"
        ))),
    }
}
