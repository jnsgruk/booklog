use axum::Json;
use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use serde::Deserialize;

use crate::application::auth::AuthenticatedUser;
use crate::application::errors::{ApiError, AppError};
use crate::application::routes::api::macros::{
    define_delete_handler, define_get_handler, define_list_fragment_renderer,
};
use crate::application::routes::support::impl_has_changes;
use crate::application::routes::support::{
    FlexiblePayload, ListQuery, PayloadSource, is_datastar_request, render_redirect_script,
    update_response, validate_update,
};
use crate::application::state::AppState;
use crate::domain::genres::{Genre, GenreSortKey, NewGenre, UpdateGenre};
use crate::domain::ids::GenreId;
use crate::domain::listing::ListRequest;
use crate::presentation::web::templates::GenreListTemplate;
use crate::presentation::web::views::{GenreView, ListNavigator, Paginated};
use tracing::info;

pub(crate) const GENRE_PAGE_PATH: &str = "/data?type=genres";
pub(crate) const GENRE_FRAGMENT_PATH: &str = "/data?type=genres#genre-list";

#[tracing::instrument(skip(state))]
pub(crate) async fn load_genre_page(
    state: &AppState,
    request: ListRequest<GenreSortKey>,
    search: Option<&str>,
) -> Result<(Paginated<GenreView>, ListNavigator<GenreSortKey>), AppError> {
    let page = state
        .genre_repo
        .list(&request, search)
        .await
        .map_err(AppError::from)?;

    Ok(crate::application::routes::support::build_page_view(
        page,
        request,
        GenreView::from,
        GENRE_PAGE_PATH,
        GENRE_FRAGMENT_PATH,
        search.map(String::from),
    ))
}

#[tracing::instrument(skip(state))]
pub(crate) async fn list_genres(
    State(state): State<AppState>,
) -> Result<Json<Vec<Genre>>, ApiError> {
    use crate::domain::listing::SortDirection;
    let genres = state
        .genre_repo
        .list_all_sorted(GenreSortKey::Name, SortDirection::Asc)
        .await
        .map_err(AppError::from)?;
    Ok(Json(genres))
}

#[derive(Debug, Deserialize)]
pub(crate) struct NewGenreSubmission {
    name: String,
    #[serde(default)]
    created_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl NewGenreSubmission {
    fn into_new(self) -> NewGenre {
        NewGenre {
            name: self.name,
            created_at: self.created_at,
        }
    }
}

#[tracing::instrument(skip(state, auth_user, headers, query))]
pub(crate) async fn create_genre(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    headers: HeaderMap,
    Query(query): Query<ListQuery>,
    payload: FlexiblePayload<NewGenreSubmission>,
) -> Result<Response, ApiError> {
    let (request, search) = query.into_request_and_search::<GenreSortKey>();
    let (submission, source) = payload.into_parts();
    let new_genre = submission.into_new();
    let new_genre = new_genre.normalize();
    let user_id = auth_user.effective.id;
    let genre = state
        .genre_service
        .create(new_genre, user_id)
        .await
        .map_err(AppError::from)?;

    info!(genre_id = %genre.id, name = %genre.name, "genre created");
    state.stats_invalidator.invalidate(user_id);

    let detail_url = format!("/genres/{}", genre.id);

    if is_datastar_request(&headers) {
        let from_data_page = headers
            .get("referer")
            .and_then(|v| v.to_str().ok())
            .is_some_and(|r| r.contains("type=genres"));

        if from_data_page {
            render_genre_list_fragment(state, request, search, true)
                .await
                .map_err(ApiError::from)
        } else {
            render_redirect_script(&detail_url).map_err(ApiError::from)
        }
    } else if matches!(source, PayloadSource::Form) {
        Ok(Redirect::to(&detail_url).into_response())
    } else {
        Ok((StatusCode::CREATED, Json(genre)).into_response())
    }
}

define_get_handler!(get_genre, GenreId, Genre, genre_repo);

#[derive(Debug, Deserialize)]
pub(crate) struct UpdateGenreSubmission {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    created_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl UpdateGenreSubmission {
    fn into_update(self) -> UpdateGenre {
        UpdateGenre {
            name: self.name,
            created_at: self.created_at,
        }
    }
}

impl_has_changes!(UpdateGenre, name, created_at);

#[tracing::instrument(skip(state, auth_user, headers))]
pub(crate) async fn update_genre(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    headers: HeaderMap,
    axum::extract::Path(id): axum::extract::Path<GenreId>,
    payload: FlexiblePayload<UpdateGenreSubmission>,
) -> Result<Response, ApiError> {
    let (submission, source) = payload.into_parts();
    let update = submission.into_update();

    validate_update(&update, Option::<&String>::None)?;

    let genre = state
        .genre_repo
        .update(id, update)
        .await
        .map_err(AppError::from)?;
    info!(%id, "genre updated");
    state.stats_invalidator.invalidate(auth_user.effective.id);
    state
        .timeline_invalidator
        .invalidate("genre", i64::from(id));

    let detail_url = format!("/genres/{}", genre.id);
    update_response(&headers, source, &detail_url, Json(genre).into_response())
}

define_delete_handler!(
    delete_genre,
    GenreId,
    GenreSortKey,
    genre_repo,
    render_genre_list_fragment,
    "type=genres",
    "/data?type=genres",
    entity_type: "genre"
);

define_list_fragment_renderer!(
    render_genre_list_fragment,
    GenreSortKey,
    load_genre_page,
    GenreListTemplate { genres },
    "#genre-list"
);
