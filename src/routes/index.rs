use askama::Template;
use askama_axum::IntoResponse;

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    title: String,
}

pub async fn index() -> impl IntoResponse {
    let template = IndexTemplate {
        title: String::from("mundo"),
    };
    template
}
