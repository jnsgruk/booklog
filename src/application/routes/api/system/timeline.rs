use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;

use crate::application::auth::AuthenticatedUser;
use crate::application::state::AppState;

/// Trigger a full timeline rebuild via the background task.
#[tracing::instrument(skip(state, _auth_user))]
pub(crate) async fn rebuild_timeline(
    State(state): State<AppState>,
    _auth_user: AuthenticatedUser,
) -> impl IntoResponse {
    state.timeline_invalidator.invalidate_full();
    StatusCode::NO_CONTENT
}
