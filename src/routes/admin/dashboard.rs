use std::sync::Arc;

use crate::session_state::TypedSession;
use crate::startup::AppState;
use crate::utils::e500;
use anyhow::Context;
use axum::extract::State;
use axum::response::{Html, IntoResponse, Redirect};
use rinja_axum::Template;
use sqlx::SqlitePool;
use uuid::Uuid;

#[derive(Template)]
#[template(path = "dashboard/index.html")]
struct DashboardTemplate<'a> {
    username: &'a str,
}

pub async fn admin_dashboard(
    State(app_state): State<Arc<AppState>>,
    session: TypedSession,
    // TODO:
    // do proper error handling
) -> Result<axum::response::Response, axum::response::Response> {
    let username = if let Some(user_id) = session.get_user_id().await.map_err(e500)? {
        get_username(user_id, &app_state.pool).await.map_err(e500)?
    } else {
        return Ok(Redirect::to("/login").into_response());
    };

    Ok(Html(
        DashboardTemplate {
            username: &username,
        }
        .render()
        .unwrap(),
    )
    .into_response())
}

#[tracing::instrument(name = "Get username", skip(pool))]
pub async fn get_username(user_id: Uuid, pool: &SqlitePool) -> Result<String, anyhow::Error> {
    let user_id = user_id.to_string();
    let row = sqlx::query!(
        r#"
        SELECT username
        FROM users
        WHERE uuid = $1
        "#,
        user_id,
    )
    .fetch_one(pool)
    .await
    .context("Failed to perform a query to retrieve a username.")?;
    Ok(row.username)
}
