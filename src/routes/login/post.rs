use askama_axum::IntoResponse;
use axum::{extract::State, response::Redirect, Form};
use axum_extra::extract::{cookie::Cookie, CookieJar};
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
    UnexpectedError(anyhow::Error),
}

impl std::fmt::Debug for LoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl IntoResponse for LoginError {
    fn into_response(self) -> askama_axum::Response {
        match &self {
            LoginError::AuthError(_e) => {
                tracing::error!(error_chain = ?self);
                // more manual way of doing it
                // headers.insert(axum::http::header::SET_COOKIE, format!("_flash={}", self).parse().unwrap());

                // simpler manual way to add cookies to headers
                // let mut headers = HeaderMap::new();
                // let cookie = cookie::Cookie::new("_flash", self.to_string());
                // headers.insert(
                //     axum::http::header::SET_COOKIE,
                //     cookie.to_string().parse().unwrap(),
                // );

                let jar = CookieJar::new().add(Cookie::new("_flash", self.to_string()));
                // CookieJar add fn docs
                // pub fn add<C: Into<Cookie<'static>>>(self, cookie: C) -> Self
                // Add a cookie to the jar.
                //
                // The value will automatically be percent-encoded
                // // that's why we later decode it in the tests
                (jar, Redirect::to("/login")).into_response()
            }
            LoginError::UnexpectedError(_e) => {
                tracing::error!(error_chain = ?self);
                let jar = CookieJar::new().add(Cookie::new("_flash", self.to_string()));
                (jar, Redirect::to("/login")).into_response()
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
    let user_id = validate_credentials(credentials, &app_state.pool)
        .await
        .map_err(|e| match e {
            AuthError::InvalidCredentials(e) => LoginError::AuthError(e.into()),
            AuthError::UnexpectedError(e) => LoginError::UnexpectedError(e.into()),
        })?;
    tracing::Span::current().record("user_id", tracing::field::display(&user_id));

    Ok(Redirect::to("/home"))
}
