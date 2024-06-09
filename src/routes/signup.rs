use askama_axum::IntoResponse;
use axum::{extract::State, http::StatusCode, Form};
use serde::Deserialize;
use sqlx::{PgPool, Pool, Postgres};

#[derive(Deserialize)]
struct FormDetails {
    username: String,
    password: String,
}

pub async fn signup(
    Form(form): Form<FormDetails>,
    State(pool): State<PgPool>,
) -> impl IntoResponse {
    // change the Option later and parse don't validate
    let user_id = create_user_id(form.username, form.password, pool);
    todo!()
}

async fn create_user_id(
    username: String,
    password: String,
    pool: Pool<Postgres>,
) -> Result<i32, String> {
    let new_id = sqlx::query!(
        "INSERT INTO users (username, password) VALUES($1, $2) RETURNING id",
        username,
        password
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    todo!()
}
