use crate::session_state::TypedSession;
use crate::utils::e500;
use axum::response::{IntoResponse, Redirect};
use axum_messages::Messages;

pub async fn log_out(
    session: TypedSession,
    messages: Messages,
) -> Result<axum::response::Response, axum::response::Response> {
    if session.get_user_id().await.map_err(e500)?.is_none() {
        Ok(Redirect::to("/login").into_response())
    } else {
        session.log_out().await.map_err(e500)?;
        messages.info("You have successfully logged out.");
        Ok(Redirect::to("/login").into_response())
    }
}
