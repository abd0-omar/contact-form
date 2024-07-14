use askama::Template;
use askama_axum::IntoResponse;
use axum::{extract::State, Form};
use serde::Deserialize;

use chrono::Utc;
use uuid::Uuid;

use crate::{
    domain::{NewSubscriber, SubscriberEmail, SubscriberName},
    email_client::EmailClient,
    startup::AppState,
};

#[derive(Deserialize, Debug, Template, sqlx::FromRow, Clone)]
#[template(path = "succession.html")]
pub struct FormData {
    name: String,
    email: String,
    error: Option<FormError>,
}

#[derive(Deserialize, Clone, Debug)]
enum FormError {
    BadEmail,
    ConflictOrQueryBlewUp,
}

impl TryFrom<FormData> for NewSubscriber {
    type Error = String;

    fn try_from(value: FormData) -> Result<Self, Self::Error> {
        let name = SubscriberName::parse(value.name)?;
        let email = SubscriberEmail::parse(value.email)?;
        Ok(Self { email, name })
    }
}

#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(form, app_state),
    fields(
        subscriber_email = %form.email,
        subscriber_name = %form.name
    )
)]
pub async fn subscribe(
    State(app_state): State<AppState>,
    Form(form): Form<FormData>,
) -> impl IntoResponse {
    let new_subscriber: NewSubscriber = match form.clone().try_into() {
        Ok(form) => form,
        Err(_) => {
            return (
                axum::http::StatusCode::UNPROCESSABLE_ENTITY,
                FormData {
                    name: form.name,
                    email: form.email,
                    error: Some(FormError::BadEmail),
                },
            )
                .into_response();
        }
    };

    // returning a template
    if insert_subscriber(&app_state, new_subscriber.clone())
        .await
        .is_err()
    {
        return (
            // email already in db or could be that query didn't make it
            axum::http::StatusCode::CONFLICT,
            FormData {
                name: new_subscriber.name.into(),
                email: new_subscriber.email.into(),
                error: Some(FormError::ConflictOrQueryBlewUp),
            },
        )
            .into_response();
    }

    let email_client = app_state.email_client;

    if send_confirmation_email(&email_client, &new_subscriber)
        .await
        .is_err()
    {
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            FormData {
                name: new_subscriber.name.into(),
                email: new_subscriber.email.into(),
                error: Some(FormError::ConflictOrQueryBlewUp),
            },
        )
            .into_response();
    }

    // status code by default would be 200 OK
    FormData {
        name: new_subscriber.name.into(),
        email: new_subscriber.email.into(),
        error: None,
    }
    .into_response()
}

#[tracing::instrument(
    name = "Send a confirmation email to a new subscriber",
    skip(email_client, new_subscriber)
)]
pub async fn send_confirmation_email(
    email_client: &EmailClient,
    new_subscriber: &NewSubscriber,
) -> Result<(), reqwest::Error> {
    let confirmation_link = "https://there-is-no-such-domain.com/subscriptions/confirm";

    let plain_body = format!(
        "Welcome to our newsletter!\nVisit {} to confirm your subscription.",
        confirmation_link
    );
    let html_body = format!(
        "Welcome to our newsletter!<br />Click <a href=\"{}\">here</a> to confirm your subscription.",
        confirmation_link
    );
    email_client
        .send_email_mailgun(
            new_subscriber.clone().email,
            "Welcome!",
            &html_body,
            &plain_body,
        )
        .await
}

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(new_subscriber, app_state)
)]

pub async fn insert_subscriber(
    app_state: &AppState,
    new_subscriber: NewSubscriber,
) -> Result<(), sqlx::Error> {
    let pool = &app_state.pool;

    sqlx::query!(
        r#"
    INSERT INTO subscriptions (id, email, name, subscribed_at, status)
    VALUES ($1, $2, $3, $4, 'pending_confirmation')
            "#,
        Uuid::new_v4(),
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now()
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(())
}

// // FromRequest example, aka custom extractor
// use axum::extract::Request;
// use axum::RequestExt;
// use axum::response::Response;
// use axum::{async_trait, extract::FromRequest, Form};
// use axum::http::{header::CONTENT_TYPE, StatusCode};
// https://github.com/tokio-rs/axum/blob/main/examples/parse-body-based-on-content-type/src/main.rs
// #[derive(Debug)]
// pub struct CustomForm<T>(T);

// #[async_trait]
// impl<S, T> FromRequest<S> for CustomForm<T>
// where
//     S: Send + Sync,
//     // Json<T>: FromRequest<()>,
//     Form<T>: FromRequest<()>,
//     T: 'static,
// {
//     type Rejection = Response;

//     async fn from_request(req: Request, _state: &S) -> Result<Self, Self::Rejection> {
//         let content_type_header = req.headers().get(CONTENT_TYPE);
//         let content_type = content_type_header.and_then(|value| value.to_str().ok());

//         if let Some(content_type) = content_type {
//             // if content_type.starts_with("application/json") {
//             //     let Json(payload) = req.extract().await.map_err(IntoResponse::into_response)?;
//             //     return Ok(Self(payload));
//             // }

//             if content_type.starts_with("application/x-www-form-urlencoded") {
//                 let Form(payload) = req.extract().await.map_err(IntoResponse::into_response)?;
//                 return Ok(Self(payload));
//             }
//         }

//         Err(StatusCode::UNSUPPORTED_MEDIA_TYPE.into_response())
//     }
// }
// CustomExtractorError
// https://github.com/tokio-rs/axum/blob/main/examples/customize-extractor-error/src/custom_extractor.rs

//
// // we can also write a custom extractor that grabs a connection from the pool
// // which setup is appropriate depends on your application
// struct DatabaseConnection(sqlx::pool::PoolConnection<sqlx::Postgres>);

// #[async_trait]
// impl<S> FromRequestParts<S> for DatabaseConnection
// where
//     PgPool: FromRef<S>,
//     S: Send + Sync,
// {
//     type Rejection = (StatusCode, String);

//     async fn from_request_parts(_parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
//         let pool = PgPool::from_ref(state);

//         let conn = pool.acquire().await.map_err(internal_error)?;

//         Ok(Self(conn))
//     }
// }

// async fn using_connection_extractor(
//     DatabaseConnection(mut conn): DatabaseConnection,
// ) -> Result<String, (StatusCode, String)> {
//     sqlx::query_scalar("select 'hello world from pg'")
//         .fetch_one(&mut *conn)
//         .await
//         .map_err(internal_error)
// }

// /// Utility function for mapping any error into a `500 Internal Server Error`
// /// response.
// fn internal_error<E>(err: E) -> (StatusCode, String)
// where
//     E: std::error::Error,
// {
//     (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
// }
// https://github.com/tokio-rs/axum/blob/main/examples/sqlx-postgres/src/main.rs
