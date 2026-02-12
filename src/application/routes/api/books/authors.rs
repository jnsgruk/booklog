use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use serde::Deserialize;

use crate::application::auth::AuthenticatedUser;
use crate::application::errors::{ApiError, AppError};
use crate::application::routes::api::images::save_deferred_image;
use crate::application::routes::api::macros::{
    define_delete_handler, define_get_handler, define_list_fragment_renderer,
};
use crate::application::routes::support::impl_has_changes;
use crate::application::routes::support::{
    FlexiblePayload, ListQuery, PayloadSource, is_datastar_request, render_redirect_script,
    update_response, validate_update,
};
use crate::application::state::AppState;
use crate::domain::authors::{Author, AuthorSortKey, NewAuthor, UpdateAuthor};
use crate::domain::ids::AuthorId;
use crate::domain::images::ImageData;
use crate::domain::listing::{ListRequest, SortDirection};
use crate::infrastructure::ai::{self, ExtractionInput};
use crate::presentation::web::templates::AuthorListTemplate;
use crate::presentation::web::views::{AuthorView, ListNavigator, Paginated};
use tracing::info;

const AUTHOR_PAGE_PATH: &str = "/data?type=authors";
const AUTHOR_FRAGMENT_PATH: &str = "/data?type=authors#author-list";

#[tracing::instrument(skip(state))]
pub(crate) async fn load_author_page(
    state: &AppState,
    request: ListRequest<AuthorSortKey>,
    search: Option<&str>,
) -> Result<(Paginated<AuthorView>, ListNavigator<AuthorSortKey>), AppError> {
    let page = state
        .author_repo
        .list(&request, search)
        .await
        .map_err(AppError::from)?;

    Ok(crate::application::routes::support::build_page_view(
        page,
        request,
        AuthorView::from,
        AUTHOR_PAGE_PATH,
        AUTHOR_FRAGMENT_PATH,
        search.map(String::from),
    ))
}

#[tracing::instrument(skip(state))]
pub(crate) async fn list_authors(
    State(state): State<AppState>,
) -> Result<Json<Vec<Author>>, ApiError> {
    let authors = state
        .author_repo
        .list_all_sorted(AuthorSortKey::Name, SortDirection::Asc)
        .await
        .map_err(AppError::from)?;
    Ok(Json(authors))
}

#[derive(Debug, Deserialize)]
pub(crate) struct NewAuthorSubmission {
    name: String,
    #[serde(default)]
    created_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(default)]
    image: ImageData,
}

impl NewAuthorSubmission {
    fn into_parts(self) -> (NewAuthor, Option<String>) {
        let author = NewAuthor {
            name: self.name,
            created_at: self.created_at,
        };
        (author, self.image.into_inner())
    }
}

#[tracing::instrument(skip(state, auth_user, headers, query))]
pub(crate) async fn create_author(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    headers: HeaderMap,
    Query(query): Query<ListQuery>,
    payload: FlexiblePayload<NewAuthorSubmission>,
) -> Result<Response, ApiError> {
    let (request, search) = query.into_request_and_search::<AuthorSortKey>();
    let (submission, source) = payload.into_parts();
    let (new_author, image_data_url) = submission.into_parts();
    let new_author = new_author.normalize();
    let user_id = auth_user.effective.id;
    let author = state
        .author_service
        .create(new_author, user_id)
        .await
        .map_err(AppError::from)?;

    info!(author_id = %author.id, name = %author.name, "author created");
    state.stats_invalidator.invalidate(user_id);

    save_deferred_image(
        &state,
        "author",
        i64::from(author.id),
        image_data_url.as_deref(),
    )
    .await;

    let detail_url = format!("/authors/{}", author.id);

    if is_datastar_request(&headers) {
        let from_data_page = headers
            .get("referer")
            .and_then(|v| v.to_str().ok())
            .is_some_and(|r| r.contains("type=authors"));

        if from_data_page {
            render_author_list_fragment(state, request, search, true)
                .await
                .map_err(ApiError::from)
        } else {
            render_redirect_script(&detail_url).map_err(ApiError::from)
        }
    } else if matches!(source, PayloadSource::Form) {
        Ok(Redirect::to(&detail_url).into_response())
    } else {
        Ok((StatusCode::CREATED, Json(author)).into_response())
    }
}

define_get_handler!(get_author, AuthorId, Author, author_repo);

#[derive(Debug, Deserialize)]
pub(crate) struct UpdateAuthorSubmission {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    created_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(default)]
    image: ImageData,
}

impl UpdateAuthorSubmission {
    fn into_parts(self) -> (UpdateAuthor, Option<String>) {
        let update = UpdateAuthor {
            name: self.name,
            created_at: self.created_at,
        };
        (update, self.image.into_inner())
    }
}

impl_has_changes!(UpdateAuthor, name, created_at);

#[tracing::instrument(skip(state, auth_user, headers))]
pub(crate) async fn update_author(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    headers: HeaderMap,
    Path(id): Path<AuthorId>,
    payload: FlexiblePayload<UpdateAuthorSubmission>,
) -> Result<Response, ApiError> {
    let (submission, source) = payload.into_parts();
    let (update, image_data_url) = submission.into_parts();

    validate_update(&update, image_data_url.as_ref())?;

    let author = state
        .author_repo
        .update(id, update)
        .await
        .map_err(AppError::from)?;
    info!(%id, "author updated");
    state.stats_invalidator.invalidate(auth_user.effective.id);

    save_deferred_image(
        &state,
        "author",
        i64::from(author.id),
        image_data_url.as_deref(),
    )
    .await;

    let detail_url = format!("/authors/{}", author.id);
    update_response(&headers, source, &detail_url, Json(author).into_response())
}

define_delete_handler!(
    delete_author,
    AuthorId,
    AuthorSortKey,
    author_repo,
    render_author_list_fragment,
    "type=authors",
    "/data?type=authors",
    image_type: "author"
);

#[tracing::instrument(skip(state, auth_user, headers, payload))]
pub(crate) async fn extract_author(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    headers: HeaderMap,
    payload: FlexiblePayload<ExtractionInput>,
) -> Result<Response, ApiError> {
    let (input, _) = payload.into_parts();
    let (result, usage) = ai::extract_author(
        &state.http_client,
        &state.openrouter_url,
        &state.openrouter_api_key,
        &state.openrouter_model,
        &input,
    )
    .await
    .map_err(ApiError::from)?;

    crate::application::routes::support::record_ai_usage(
        state.ai_usage_repo.clone(),
        auth_user.effective.id,
        &state.openrouter_model,
        "extract-author",
        usage,
    );

    if is_datastar_request(&headers) {
        use serde_json::Value;
        let signals = vec![
            (
                "_author-name",
                Value::String(result.name.unwrap_or_default()),
            ),
            ("_extracted", Value::Bool(true)),
        ];
        crate::application::routes::support::render_signals_json(&signals).map_err(ApiError::from)
    } else {
        Ok(Json(result).into_response())
    }
}

define_list_fragment_renderer!(
    render_author_list_fragment,
    AuthorSortKey,
    load_author_page,
    AuthorListTemplate { authors },
    "#author-list"
);
