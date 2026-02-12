use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use tracing::info;

use crate::application::auth::AuthenticatedUser;
use crate::application::errors::{ApiError, AppError};
use crate::application::state::AppState;
use crate::domain::ids::{BookId, UserBookId};
use crate::domain::user_books::{NewUserBook, Shelf, UserBook};

#[derive(Debug, Deserialize)]
pub(crate) struct NewUserBookSubmission {
    book_id: BookId,
    #[serde(default)]
    shelf: Option<String>,
    #[serde(default)]
    book_club: Option<bool>,
}

#[tracing::instrument(skip(state, auth_user))]
pub(crate) async fn create_user_book(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Json(submission): Json<NewUserBookSubmission>,
) -> Result<Response, ApiError> {
    let user_id = auth_user.effective.id;
    let shelf = submission
        .shelf
        .and_then(|s| s.parse::<Shelf>().ok())
        .unwrap_or_default();

    let new_user_book = NewUserBook {
        user_id,
        book_id: submission.book_id,
        shelf,
        book_club: submission.book_club.unwrap_or(false),
    };

    let user_book = state
        .user_book_repo
        .insert(new_user_book)
        .await
        .map_err(AppError::from)?;

    info!(user_book_id = %user_book.id, book_id = %submission.book_id, shelf = %shelf.as_str(), "user book created");
    state.stats_invalidator.invalidate(user_id);

    Ok((StatusCode::CREATED, Json(user_book)).into_response())
}

#[derive(Debug, Deserialize)]
pub(crate) struct MoveShelfSubmission {
    shelf: String,
}

#[tracing::instrument(skip(state, auth_user))]
pub(crate) async fn move_user_book(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Path(id): Path<UserBookId>,
    Json(submission): Json<MoveShelfSubmission>,
) -> Result<Json<UserBook>, ApiError> {
    let existing = state.user_book_repo.get(id).await.map_err(AppError::from)?;
    if existing.user_id != auth_user.effective.id {
        return Err(AppError::NotFound.into());
    }

    let shelf: Shelf = submission
        .shelf
        .parse()
        .map_err(|()| AppError::validation("invalid shelf value"))?;

    let user_book = state
        .user_book_repo
        .move_shelf(id, shelf)
        .await
        .map_err(AppError::from)?;

    info!(%id, shelf = %shelf.as_str(), "user book moved");
    state.stats_invalidator.invalidate(auth_user.effective.id);

    Ok(Json(user_book))
}

#[derive(Debug, Deserialize)]
pub(crate) struct SetBookClubSubmission {
    book_club: bool,
}

#[tracing::instrument(skip(state, auth_user))]
pub(crate) async fn set_book_club_user_book(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Path(id): Path<UserBookId>,
    Json(submission): Json<SetBookClubSubmission>,
) -> Result<Json<UserBook>, ApiError> {
    let existing = state.user_book_repo.get(id).await.map_err(AppError::from)?;
    if existing.user_id != auth_user.effective.id {
        return Err(AppError::NotFound.into());
    }

    let user_book = state
        .user_book_repo
        .set_book_club(id, submission.book_club)
        .await
        .map_err(AppError::from)?;

    info!(%id, book_club = submission.book_club, "user book book_club updated");
    state.stats_invalidator.invalidate(auth_user.effective.id);

    Ok(Json(user_book))
}

#[tracing::instrument(skip(state, auth_user))]
pub(crate) async fn delete_user_book(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Path(id): Path<UserBookId>,
) -> Result<StatusCode, ApiError> {
    let existing = state.user_book_repo.get(id).await.map_err(AppError::from)?;
    if existing.user_id != auth_user.effective.id {
        return Err(AppError::NotFound.into());
    }

    state
        .user_book_repo
        .delete(id)
        .await
        .map_err(AppError::from)?;

    info!(%id, "user book deleted");
    state.stats_invalidator.invalidate(auth_user.effective.id);

    Ok(StatusCode::NO_CONTENT)
}

#[tracing::instrument(skip(state))]
pub(crate) async fn list_user_books(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
) -> Result<Json<Vec<UserBook>>, ApiError> {
    let user_id = auth_user.effective.id;
    let book_ids = state
        .user_book_repo
        .book_ids_for_user(user_id, None)
        .await
        .map_err(AppError::from)?;

    // Return simple list of user-book associations
    // For full details, use the list_by_user endpoint with pagination
    let request = crate::domain::listing::ListRequest::show_all(
        crate::domain::user_books::UserBookSortKey::CreatedAt,
        crate::domain::listing::SortDirection::Desc,
    );
    let page = state
        .user_book_repo
        .list_by_user(user_id, None, &request, None)
        .await
        .map_err(AppError::from)?;

    let user_books: Vec<UserBook> = page.items.into_iter().map(|d| d.user_book).collect();
    let _ = book_ids; // used only for potential future filtering
    Ok(Json(user_books))
}
