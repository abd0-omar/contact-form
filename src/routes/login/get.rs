use askama::Template;
use askama_axum::IntoResponse;
use axum::response::Html;
use axum_extra::extract::{cookie::Cookie, CookieJar};

#[derive(Template)]
#[template(path = "login.html")]
pub struct ErrorTemplate {
    error: Option<String>,
}

pub async fn login_form(jar: CookieJar) -> impl IntoResponse {
    let error = jar.get("_flash").map(|cookie| cookie.value().to_string());
    let jar = jar.remove(Cookie::from("_flash"));
    // // same as
    // let mut headers = axum::http::HeaderMap::new();
    // // the only difference from the "jar.remove" method is it doesn't delete "_flash" just it's
    // // value
    // let mut cookie = axum_extra::extract::cookie::Cookie::new("_flash", "");
    // cookie.set_max_age(None);
    // headers.insert(
    //     axum::http::header::SET_COOKIE,
    //     cookie.to_string().parse().unwrap(),
    // );
    (jar, Html(ErrorTemplate { error }.render().unwrap()))
}
