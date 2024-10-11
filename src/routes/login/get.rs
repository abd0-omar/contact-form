use askama::Template;
use askama_axum::IntoResponse;
use axum::{
    extract::{Query, State},
    response::Html,
};
use hmac::{Hmac, Mac};
use secrecy::ExposeSecret;

use crate::startup::{AppState, HmacSecret};

#[derive(serde::Deserialize)]
pub struct QueryParams {
    error: String,
    tag: String,
}

impl QueryParams {
    fn verify(self, secret: &HmacSecret) -> Result<String, anyhow::Error> {
        // the tag from `self`
        let tag = hex::decode(&self.tag)?;
        // the error from `self`
        let query_string = format!("error={}", urlencoding::Encoded::new(&self.error));

        // encode secret with the query string and tag
        // secret first
        let mut mac =
            Hmac::<sha2::Sha256>::new_from_slice(secret.0.expose_secret().as_bytes()).unwrap();
        // the query
        mac.update(query_string.as_bytes());
        // tag
        mac.verify_slice(&tag)?;
        Ok(self.error)
    }
}

#[derive(Template)]
#[template(path = "login.html")]
pub struct ErrorTemplate<'a> {
    error: Option<&'a str>,
}

pub async fn login_form(
    query: Option<Query<QueryParams>>,
    State(app_state): State<AppState>,
) -> impl IntoResponse {
    match query {
        None => Html(ErrorTemplate { error: None }.render().unwrap()),
        Some(query) => match query.0.verify(&app_state.secret) {
            Ok(error) => Html(
                ErrorTemplate {
                    error: Some(&error),
                }
                .render()
                .unwrap(),
            ),
            Err(e) => {
                tracing::warn!(
                error.message = %e,
                error.cause_chain = ?e,
                "Failed to verify query parameters using the HMAC tag"
                );
                Html(ErrorTemplate { error: None }.render().unwrap())
            }
        },
    }
    // if let Some(q) = query {
    //     Html(
    //         ErrorTemplate {
    //             error: Some(&q.error),
    //         }
    //         .render()
    //         .unwrap(),
    //     )
    // } else {
    //     Html(ErrorTemplate { error: None }.render().unwrap())
    // }
}

// without askama template engine && hmac
// pub async fn login_form(Query(query): Query<QueryParams>) -> impl IntoResponse {
//     let error_html = match query.error {
//         None => "".into(),
//         Some(error_message) => format!("<p><i>{error_message}</i></p>"),
//     };
//     (
//         Html(format!(
//             r#"<!DOCTYPE html>
// <html lang="en">
// <head>
// <meta http-equiv="content-type" content="text/html; charset=utf-8">
// <title>Login</title>
// </head>
// <body>
// {error_html}
// <form action="/login" method="post">
// <label>Username
// <input
// type="text"
// placeholder="Enter Username"
// name="username"
// >
// </label>
// <label>Password
// <input
// type="password"
// placeholder="Enter Password"
// name="password"
// >
// </label>
// <button type="submit">Login</button>
// </form>
// </body>
// </html>"#,
//         )),
//     )
// }
