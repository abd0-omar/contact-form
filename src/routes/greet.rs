pub async fn greet(name: Option<axum::extract::Path<String>>) -> impl axum::response::IntoResponse {
    if let Some(n) = name {
        let n = n.0;
        format!("hola {}!", n)
    } else {
        String::from("hola mundoz!")
    }
}
