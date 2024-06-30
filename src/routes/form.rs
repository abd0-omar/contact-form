use tokio::time::{sleep, Duration};

use askama::Template;
use askama_axum::IntoResponse;
use axum::Form;
use serde::Deserialize;

#[derive(Deserialize, Debug, Template)]
#[template(path = "succession.html")]
pub struct Input {
    name: String,
    email: String,
}

pub async fn accept_form(Form(input): Form<Input>) -> impl IntoResponse {
    sleep(Duration::from_secs(1)).await;
    println!("100 ms have elapsed");
    dbg!(&input);
    let template = Input {
        name: input.name,
        email: input.email,
    };
    template
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
