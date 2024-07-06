use sqlx::PgPool;
use tokio::time::{sleep, Duration};

use askama::Template;
use askama_axum::IntoResponse;
use axum::{extract::State, Form};
use serde::Deserialize;

use chrono::Utc;
use uuid::Uuid;

use axum::http::StatusCode;

#[derive(Deserialize, Debug, Template, sqlx::FromRow)]
#[template(path = "succession.html")]
pub struct FormData {
    name: String,
    email: String,
}

#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(form, pool),
    fields(
        subscriber_email = %form.email,
        subscriber_name = %form.name
    )
)]
pub async fn subscribe(
    State(pool): State<PgPool>,
    Form(form): Form<FormData>,
) -> impl IntoResponse {
    match insert_subscriber(pool, &form).await {
        Ok(_) => FormData {
            name: form.name,
            email: form.email,
        }
        .into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }

    // let template = FormData {
    //     name: form.name,
    //     email: form.email,
    // };
    // template.into_response()
}

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(form, pool)
)]
pub async fn insert_subscriber(pool: PgPool, form: &FormData) -> Result<(), sqlx::Error> {
    sleep(Duration::from_secs(1)).await;
    println!("100 ms have elapsed");

    sqlx::query(
        "INSERT INTO subscriptions (id, name, email, subscribed_at) VALUES ($1, $2, $3, $4)",
    )
    .bind(Uuid::new_v4())
    .bind(&form.name)
    .bind(&form.email)
    .bind(Utc::now())
    .bind(3)
    .execute(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query {:?}", e);
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
