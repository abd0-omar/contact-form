use axum::{
    extract::State,
    response::{Html, IntoResponse},
};
use axum_extra::extract::OptionalQuery;
use hmac::{Hmac, Mac};
use rinja_axum::Template;
use secrecy::ExposeSecret;

use crate::startup::HmacSecret;

#[derive(serde::Deserialize)]
pub struct QueryParams {
    error: String,
    tag: String,
}

impl QueryParams {
    fn verify(self, secret: &HmacSecret) -> Result<String, anyhow::Error> {
        let tag = hex::decode(self.tag)?;
        let query_string = format!("error={}", urlencoding::Encoded::new(&self.error));

        let mut mac =
            <Hmac<sha2::Sha256>>::new_from_slice(secret.0.expose_secret().as_bytes()).unwrap();
        Mac::update(&mut mac, query_string.as_bytes());
        mac.verify_slice(&tag)?;

        Ok(self.error)
    }
}

#[derive(Template)]
#[template(path = "login/index.html")]
struct LoginTemplate {
    error: String,
}

pub async fn login_form(
    OptionalQuery(query): OptionalQuery<QueryParams>,
    State(hmac_secret): State<HmacSecret>,
) -> impl IntoResponse {
    let error = match query {
        None => "".into(),
        Some(query) => match query.verify(&hmac_secret) {
            Ok(error) => error,
            Err(e) => {
                tracing::warn!(
                    error.message = %e,
                    error.cause_chain = ?e,
                    "Failed to verify query parameters using the HMAC tag"
                );
                "".into()
            }
        },
    };
    Html(LoginTemplate { error }.render().unwrap())
}
