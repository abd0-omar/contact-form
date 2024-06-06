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
